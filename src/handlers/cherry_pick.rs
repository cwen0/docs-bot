use crate::github::{PullRequest, PullRequestEvent};
use crate::handlers::Context;
use crate::config::{RepoConfig, LabelConfig};
use crate::git::{Git, GitCredential};
use std::sync::Arc;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::{env, fs, writeln};
use std::path::Path;
use std::fmt::Write as FmtWrite;
use git2::IndexAddOption;
use serde_json::json;
use std::time::Duration;
use std::thread::sleep;

pub async fn handle<'a>(
    ctx: &Context<'a>,
    config: Arc<RepoConfig>,
    pr: &PullRequestEvent,
) -> anyhow::Result<()> {
    let labels = pr.pull_request.labels();
    let repo_name = &pr.repository.full_name;

    for config_label in config.labels.iter() {
        log::info!("config label: {}", config_label.label);
        let label = labels.iter().find(|&l| l.name == config_label.label);
        let pull_request = &pr.pull_request;
        match label {
            Some(_l) => {
                match handle_docs_label(
                    ctx,
                    config_label,
                    pull_request,
                    repo_name.to_string(),
                ).await {
                    Ok(()) => {
                        log::info!("handle docs label successfully!")
                    },
                    Err(_e) => {},
                }
            },
            _ => return Ok(()),
        };
    }

    Ok(())
}

async fn handle_docs_label<'a>(
    ctx: &Context<'a>,
    config: &LabelConfig,
    pr_request: &PullRequest,
    repo_name: String,
) -> anyhow::Result<()> {
    let file_diff = parse_files_diff(&pr_request.diff_url).await.unwrap();
    let commit = if let Some(s) = &pr_request.merge_commit_sha {
        s
    } else {
        log::error!("no merge_commit_sha in pr_request");
        return Ok(());
    };

    let target = &(commit.clone()[0..12]);

    cherry_pick(repo_name.as_str(), config, file_diff, target).unwrap();

    log::info!("sleep 2 s");
    sleep(Duration::from_secs(2));
    log::info!("sleep end");

    let body = json!({
        "title": format!("sync docs to {}", &config.label),
        "head": target,
        "base": config.base_branch,
        "maintainer_can_modify": true,
    });

    let gh = ctx.github.clone();

    gh.create_pull_request(repo_name.as_str(), body.to_string()).await.unwrap();

    Ok(())
}

fn cherry_pick(
    repo_name: &str,
    config: &LabelConfig,
    file_diff: HashMap<String, String>,
    target_branch: &str,
) -> anyhow::Result<()> {
    let current_dir = env::current_dir().unwrap();

    let repo = format!("https://github.com/{}", repo_name);

    let cred = GitCredential::new(
        env::var("GITHUB_USERNAME").unwrap(),
        env::var("GITHUB_PASSWORD").unwrap(),
    );
    let gt = Git::new(current_dir, cred).unwrap();

    let repo_dir = target_branch.clone();
    let base_branch = config.base_branch.as_str();

    let repo = gt.clone_repo(repo_dir,  base_branch, repo.as_str()).unwrap();

    gt.create_branch(&repo, target_branch, base_branch).unwrap();

    gt.checkout(&repo, target_branch).unwrap();

    for (file, _diff) in file_diff.iter() {
        let path = Path::new(file);
        log::info!("file: {:?}", file);
        if path.starts_with(&config.source_directory) {
            let source_file_path = Path::new(repo_dir).join(path);

            let base_file = path.strip_prefix(&config.source_directory).unwrap();
            let target_file_path = Path::new(repo_dir)
                .join(&config.target_directory)
                .join(base_file);

            log::info!("copy {:?} to {:?}", source_file_path, target_file_path);

            fs::copy(source_file_path, target_file_path).unwrap();
        }
    }

    let mut index = repo.index().expect("cannot get the Index file");
    index.add_all(["."].iter(), IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();

    gt.commit_index(
        &repo,
        &mut index,
        format!("sync to {}", config.target_directory)
            .as_str())
        .unwrap();

    gt.push_branch(&repo, target_branch, "origin").unwrap();

    fs::remove_dir_all(target_branch).unwrap();

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
                // if !diffs.is_empty() {
                //     diffs.push('\n')
                // }
                // diffs.push_str(line);

                match writeln!(&mut diffs, "{}", line) {
                    Ok(_) => {},
                    Err(e) => panic!("failed to run writeln: {:?}", e),
                };

                file_diff_map.insert(file_name.clone(), diffs);
            }
        }
    }

    Ok(file_diff_map)
}

