use std::fmt;
use std::sync::mpsc;
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
pub async fn handle<'a>(
    ctx: &Context<'a>,
    event: &Event,
) -> Vec<HandlerError> {
    let config = config::get_repo_config(event.repo_name()).await;
    let mut errors = Vec::new();

    match config {
        Ok(_c)   => {
            let sender = ctx.pr_task_sender.clone();

            match event {
                Event::PullRequest( e) => {
                    if e.is_closed_and_merged() {
                        sender.send(e).unwrap();
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

pub struct Context<'a> {
    pub github: GithubClient,
    // pub db_conn: Connection,
    pub username: String,
    pub pr_task_sender: mpsc::Sender<&'a PullRequestEvent>,
    pub pr_task_receiver: mpsc::Receiver<&'a PullRequestEvent>,
}

pub async fn handle_pr_task<'a>(ctx: &Context<'a>) -> anyhow::Result<()> {
    let receiver = &ctx.pr_task_receiver;
    for pr in receiver {
        let config = config::get_repo_config(
            pr.repository.full_name.as_str()).await;

        match config {
            Ok(c)   => {
                if let Err(err) = cherry_pick::handle(ctx, c, &pr).await {
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
