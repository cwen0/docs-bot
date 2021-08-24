use crate::github::{Event, PullRequestAction, PullRequest};
use crate::handlers::Context;
use crate::config::{RepoConfig, LabelConfig};
use std::sync::Arc;
use std::collections::HashMap;
use futures::{AsyncReadExt, StreamExt};
use std::convert::TryFrom;

pub async fn handle(ctx: &Context, config: Arc<RepoConfig>, event: &Event) -> anyhow::Result<()>{
    let event = if let Event::PullRequest(e) = event{
        if !matches!(e.action, PullRequestAction::Closed) {
            log::debug!("skipping event, pr was {:?}", e.action);
            // return Ok(());
        }
        if !e.pull_request.merged {
            log::debug!("skipping event, pr was not merged");
            // return Ok(());
        }
        e
    } else {
        return Ok(());
    };

    let labels = event.pull_request.labels();

    for config_label in config.labels.iter() {
        log::info!("config label: {}", config_label.label);
        let label = labels.iter().find(|&l| l.name == config_label.label);
        let pull_request = &event.pull_request;
        match label {
            Some(_l) => {
                handle_docs_label(ctx, config_label, pull_request).await
            },
            None => return Ok(()),
        };
    }

    Ok(())
}

async fn handle_docs_label(_ctx: &Context,config: &LabelConfig, pr_request: &PullRequest) -> anyhow::Result<()>{
    let con = parse_files_diff(&pr_request.diff_url).await?;

    Ok(())
}

async fn parse_files_diff(url: &String) -> anyhow::Result<HashMap<String, String>, reqwest::Error>{
    let file_content = reqwest::get(url).await?
        .text().await?;

    // log::info!("{}", file_content);
    let mut file_diff_map: HashMap<String, String> = HashMap::new();
    let mut file_name = "".to_string();
    let mut file_start_map: HashMap<String, u64> = HashMap::new();

    for (num, line) in file_content.lines().enumerate() {
        let num_u64 = u64::try_from(num).unwrap();
        if line.starts_with("diff --git a") {
            file_name = line
                .split(' ')
                .find(|&n| n.starts_with("a/"))
                .unwrap()
                .trim_start_matches("a/")
                .to_string();

            // let new_file_name = file_name.clone();
            file_start_map.insert(file_name.clone(), num_u64+2);
            let start_diff = "".to_string();
            file_diff_map.insert(file_name.clone(), start_diff);
        }

        if !file_name.is_empty() {
            let mut diffs = file_diff_map.get(file_name.as_str()).unwrap().clone();
            let start_index = file_start_map.get(file_name.as_str()).unwrap();

            if num_u64 >= *start_index {
                if !diffs.is_empty() {
                    diffs.push('\n')
                }
                diffs.push_str(line);

                file_diff_map.insert(file_name.clone(), diffs);
            }
        }
    }

    // log::info!("{:?}", file_diff_map);
    // log::info!("{:?}", file_start_map);

    Ok(file_diff_map)
}

