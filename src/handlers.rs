use std::fmt;
use std::sync::{mpsc, Arc};
use crate::github::{Event, GithubClient, PullRequestEvent};
use crate::config;

mod cherry_pick;

#[derive(Debug)]
pub enum HandlerError {
    Message(String),
    Other(anyhow::Error),
}

impl std::error::Error for HandlerError {}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HandlerError::Message(msg) => write!(f, "{}", msg),
            HandlerError::Other(_) => write!(f, "An internal error occurred."),
        }
    }
}

#[warn(unused_mut)]
pub async fn handle(
    _ctx: &Context,
    event: &Event,
    sender: mpsc::Sender<PullRequestEvent>,
) -> Vec<HandlerError> {
    let config = config::get_repo_config(event.repo_name()).await;
    let mut errors = Vec::new();

    match config {
        Ok(_c)   => {
            match event {
                Event::PullRequest( e) => {
                    log::info!("send event {:?}", e);
                    if e.is_closed_and_merged() {
                        sender.send(e.clone()).unwrap();
                    }
                }
                _ => {
                    log::debug!("skipping event");
                }
            }

        },
        Err(err) => {
            errors.push(HandlerError::Message(err.to_string()));
            log::error!("failed to get repo config, {}", err);
        },
    };
    errors
}

pub struct Context {
    pub github: GithubClient,
    // pub db_conn: Connection,
    pub username: String,
}

pub async fn handle_pr_task(
    ctx: Arc<Context>,
    receiver: mpsc::Receiver<PullRequestEvent> ,
) -> anyhow::Result<()> {
    for pr in receiver {
        log::info!("receiver event");
        let config = config::get_repo_config(
            pr.repository.full_name.as_str()).await;

        match config {
            Ok(c)   => {
                if let Err(err) = cherry_pick::handle(ctx.clone(), c, &pr).await {
                    log::error!("failed to process event {:?} with pr_merge handler: {:?}", pr, err);
                };
            },
            Err(err) => {
                log::error!("failed to get repo config, {}", err);
            },
        };
    }

    Ok(())
}
