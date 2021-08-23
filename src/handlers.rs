use std::fmt;
use crate::github::{Event, GithubClient};
use crate::config;

mod pr_merge;

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
pub async fn handle(ctx: &Context, event: &Event) -> Vec<HandlerError> {
    let config = config::get_repo_config(event.repo_name()).await;
    let mut errors = Vec::new();

    let repo_config = match config {
        Ok(c)   => c,
        Err(err) => {
            errors.push(HandlerError::Message(err.to_string()));
            return errors;
        },
    };

    match event {
        Event::PullRequest(_event) => {
            if let Err(e) = pr_merge::handle(ctx, repo_config, event).await {
                log::error!(
                    "failed to process event {:?} with pr_merge handler: {:?}",
                    event,
                    e
                );
            }
        }
        _ => {
            log::debug!("skipping event");
        }
    }

    errors
}

pub struct Context {
    pub github: GithubClient,
    // pub db_conn: Connection,
    pub username: String,
}
