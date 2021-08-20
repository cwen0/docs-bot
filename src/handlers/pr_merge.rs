use crate::github::{Event, PullRequestAction};
use crate::handlers::Context;

pub async fn handle(_ctx: &Context, event: &Event) -> anyhow::Result<()>{
    let event = if let Event::PullRequest(e) = event{
        if !matches!(e.action, PullRequestAction::Closed) {
            log::debug!("skipping event, pr was {:?}", e.action);
            return Ok(());
        }
        if !e.pull_request.merged {
            log::debug!("skipping event, pr was not merged");
            return Ok(());
        }
        e
    } else {
        return Ok(());
    };

    let labels = event.pull_request.labels();

    for label in labels.iter() {
        log::info!("label: {:?}", label.name)
    }

    Ok(())
}