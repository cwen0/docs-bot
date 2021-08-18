use crate::github::{Event, IssuesAction};
use crate::handlers::Context;

pub async fn handle(_ctx: &Context, event: &Event) -> anyhow::Result<()>{
    let event = if let Event::Issue(e) = event{
        if !e.issue.is_pr() {
            log::debug!("skipping event, event {:?} was an issue", e.issue.number);
            return Ok(());
        }
        if !matches!(e.action, IssuesAction::Closed) {
            log::debug!("skipping event, pr was {:?}", e.action);
            return Ok(());
        }
        if !e.issue.merged {
            log::debug!("skipping event, pr was not merged");
            return Ok(());
        }
        e
    } else {
        return Ok(());
    };

    let labels = event.issue.labels();

    for label in labels.iter() {
        log::info!("label: {:?}", label.name)
    }

    Ok(())
}