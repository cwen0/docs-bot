use std::fmt;
use crate::github::{Event, GithubClient};

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
pub async fn handle(_ctx: &Context, event: &Event) -> Vec<HandlerError> {
    // let config = config::get(&ctx.github, event.repo_name()).await;
    let errors = Vec::new();

    match event {
        Event::Issue(_event) => {
            if let Err(e) = pr_merge::handle(_ctx, event).await {
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

// macro_rules! issue_handles {
//     ($($name:ident,)*) => {
//         async fn handle_issue(
//             ctx: &Context,
//             event: &IssuesEvent,
//             config: &Arc<Config>,
//             errors: &mut Vec<HandlerError>,
//         ) {
//             $(
//             match $name::parse_input(ctx, event, config.$name.as_ref()) {
//                 Err(err) => errors.push(HandlerError::Message(err)),
//                 Ok(Some(input)) => {
//                     if let Some(config) = &config.$name {
//                         $name::handle_input(ctx, config, event, input).await.unwrap_or_else(|err| errors.push(HandlerError::Other(err)));
//                     } else {
//                         errors.push(HandlerError::Message(format!(
//                             "The feature `{}` is not enabled in this repository.\n\
//                             To enable it add its section in the `triagebot.toml` \
//                             in the root of the repository.",
//                             stringify!($name)
//                         )));
//                     }
//                 }
//                 Ok(None) => {}
//             })*
//         }
//     }
// }
//

pub struct Context {
    pub github: GithubClient,
    // pub db_conn: Connection,
    pub username: String,
}
