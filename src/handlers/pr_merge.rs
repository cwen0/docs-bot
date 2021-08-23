use crate::github::{Event, PullRequestAction, PullRequest};
use crate::handlers::Context;
use crate::config::{RepoConfig, LabelConfig};
use std::sync::Arc;

pub async fn handle(ctx: &Context, config: Arc<RepoConfig>, event: &Event) -> anyhow::Result<()>{
    let event = if let Event::PullRequest(e) = event{
        if !matches!(e.action, PullRequestAction::Closed) {
            log::debug!("skipping event, pr was {:?}", e.action);
            return Ok(());
        }
        // if !e.pull_request.merged {
        //     log::debug!("skipping event, pr was not merged");
        //     return Ok(());
        // }
        e
    } else {
        return Ok(());
    };

    let labels = event.pull_request.labels();

    for config_label in config.labels.iter() {
        let label = labels.iter().find(|&l| l.name == config_label.label);
        let pull_request = &event.pull_request;
        match label {
            Some(_l) => {
                handle_docs_label(ctx, config_label, pull_request)
            },
            None => return Ok(()),
        }
    }

    Ok(())
}

fn handle_docs_label(ctx: &Context,config: &LabelConfig, pr_request: &PullRequest) -> anyhow::Result<()>{

    Ok(())
}
