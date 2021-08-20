#![allow(unused)]
use crate::github::PullRequest;

pub struct ErrorComment<'a> {
    pull_request: &'a PullRequest,
    message: String,
}

impl<'a> ErrorComment<'a> {
    pub fn new<T>(pull_request: &'a PullRequest, message: T) -> ErrorComment<'a>
        where
            T: Into<String>,
    {
        ErrorComment {
            pull_request,
            message: message.into(),
        }
    }

    // pub async fn post(&self, client: &GithubClient) -> anyhow::Result<()> {
    //     let mut body = String::new();
    //     writeln!(body, "**Error**: {}", self.message)?;
    //     writeln!(body)?;
    //     writeln!(
    //         body,
    //         "Please let **`@chaos-mesh/maintainers`** know if you're having trouble with this bot."
    //     )?;
    //     self.pull_request.post_comment(client, &body).await
    // }
}
