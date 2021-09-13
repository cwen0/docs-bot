#![allow(clippy::new_without_default)]

use std::fmt;
use crate::handlers;
use crate::github;
use anyhow::Context;

#[derive(Debug)]
pub enum EventName {
    PullRequest,
    PullRequestReview,
    PullRequestReviewComment,
    IssueComment,
    Issue,
    Push,
    Create,
    Other,
}

impl std::str::FromStr for EventName {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<EventName, Self::Err> {
        Ok(match s {
            "pull_request_review" => EventName::PullRequestReview,
            "pull_request_review_comment" => EventName::PullRequestReviewComment,
            "issue_comment" => EventName::IssueComment,
            "pull_request" => EventName::PullRequest,
            "issues" => EventName::Issue,
            "push" => EventName::Push,
            "create" => EventName::Create,
            _ => EventName::Other,
        })
    }
}

impl fmt::Display for EventName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                EventName::PullRequestReview => "pull_request_review",
                EventName::PullRequestReviewComment => "pull_request_review_comment",
                EventName::IssueComment => "issue_comment",
                EventName::Issue => "issues",
                EventName::PullRequest => "pull_request",
                EventName::Push => "push",
                EventName::Create => "create",
                EventName::Other => "other",
            }
        )
    }
}

#[derive(Debug)]
pub struct WebhookError(anyhow::Error);

impl From<anyhow::Error> for WebhookError {
    fn from(e: anyhow::Error) -> WebhookError {
        WebhookError(e)
    }
}

pub fn deserialize_payload<T: serde::de::DeserializeOwned>(v: &str) -> anyhow::Result<T> {
    let mut deserializer = serde_json::de::Deserializer::from_str(&v);
    let res: Result<T, _> = serde_path_to_error::deserialize(&mut deserializer);

    match res {
        Ok(r) => Ok(r),
        Err(e) => {
            let ctx = format!("at {:?}", e.path());
            Err(e.into_inner()).context(ctx)
        }
    }
}

pub async fn webhook<'a>(
    event: EventName,
    payload: String,
    ctx: &handlers::Context<'a>,
) -> Result<bool, WebhookError> {
    let event = match event {
        EventName::PullRequest => {
            // log::info!("payload={:?}", &payload);
            let payload = deserialize_payload::<github::PullRequestEvent>(&payload)
                .with_context(|| format!("{:?} failed to deserialize", event))
                .map_err(anyhow::Error::from)?;

            github::Event::PullRequest(payload)
        }
        _ => {
            return Ok(false);
        }
    };

    let errors = handlers::handle(&ctx, &event).await;
    let mut other_error = false;
    let mut message = String::new();

    for err in errors {
        match err {
            handlers::HandlerError::Message(msg) => {
                if !message.is_empty() {
                    message.push_str("\n\n");
                }
                message.push_str(&msg);
            }
            handlers::HandlerError::Other(err) => {
                log::error!("handling event failed: {:?}", err);
                other_error = true;
            }
        }
    }

    if !message.is_empty() {
        // TODO
    }

    if other_error {
        Err(WebhookError(anyhow::anyhow!(
            "handling failed, error logged",
        )))
    } else {
        Ok(true)
    }
}
