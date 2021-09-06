use crate::github::{Event, PullRequestAction, PullRequest};
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

pub async fn handle(ctx: &Context, config: Arc<RepoConfig>, event: &Event) -> anyhow::Result<()> {
    let pr = if let Event::PullRequest(e) = event{
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

    let labels = pr.pull_request.labels();

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
                    event.repo_name().to_string(),
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

async fn handle_docs_label(
    _ctx: &Context,
    config: &LabelConfig,
    pr_request: &PullRequest,
    repo_name: String,
) -> anyhow::Result<()> {
    let file_diff = parse_files_diff(&pr_request.diff_url).await.unwrap();
    cherry_pick(pr_request, repo_name, config, file_diff).unwrap();

    Ok(())
}

fn cherry_pick(
    pr_request: &PullRequest,
    repo_name: String,
    config: &LabelConfig,
    file_diff: HashMap<String, String>,
) -> anyhow::Result<()> {
    let current_dir = env::current_dir().unwrap();
    // let commit = pr_request.merge_commit_sha.unwrap();

    let commit = if let Some(s) = &pr_request.merge_commit_sha {
        s
    } else {
        log::error!("no merge_commit_sha in pr_request");
        return Ok(());
    };

    let target = &(commit.clone()[0..12]);
    let repo = format!("https://github.com/{}", repo_name);

    let cred = GitCredential::new(
        env::var("GITHUB_USERNAME").unwrap(),
        env::var("GITHUB_PASSWORD").unwrap(),
    );
    let gt = Git::new(current_dir, cred).unwrap();

    let repo = gt.clone_repo(target, repo.as_str()).unwrap();

    gt.create_branch(&repo, target, "main").unwrap();

    gt.checkout(&repo, target).unwrap();

    for (file, _diff) in file_diff.iter() {
        let path = Path::new(file);
        log::info!("file: {:?}", file);
        if path.starts_with(&config.source_directory) {
            let base_file = path.strip_prefix(&config.source_directory).unwrap();
            log::info!("base_file: {:?}", base_file.to_str());
            let target_file_path = Path::new(target)
                .join(&config.target_directory)
                .join(base_file);
            log::info!("target_file_path: {:?}", target_file_path.as_path().to_str());

            // let mut file = fs::OpenOptions::new()
            //     .read(true)
            //     .open(&target_file_path.as_path())
            //     .unwrap();
            // // let mut file_content = fs::read_to_string(target_file_path).unwrap();
            //
            // let mut file_content = String::new();
            // file.read_to_string(file_content.as_mut_string()).unwrap();
            //
            // // log::info!("{}", file_content);
            // log::info!("{}", diff);
            //
            // let path_str = diffy::Patch::from_str(diff).unwrap();
            // let patch_content = diffy::apply(file_content.as_str(), &path_str);
            //
            // let patch_content = match patch_content {
            //     Ok(p) => p,
            //     Err(e) => panic!("Problem creating the file: {:?}", e),
            // };
            //
            // log::info!("patch_content: {}", patch_content);

            // let mut n_file = fs::OpenOptions::new()
            //     .write(true)
            //     .open(&target_file_path.as_path())
            //     .unwrap();
            //
            // n_file.write_all(patch_content.as_bytes()).unwrap();

            let source_file_path = Path::new(target).join(path);

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

    gt.push_branch(&repo, target, "origin").unwrap();

    fs::remove_dir_all(target).unwrap();

    Ok(())
}

// fn default_username_from_env() -> String {
//     match env::var("GITHUB_USERNAME") {
//         Ok(v) => return v,
//         Err(_) => (),
//     }
//
//     panic!("could not find token in GITHUB_USERNAME or .gitconfig/github.oath-token")
// }

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

    // log::info!("{:?}", file_diff_map);
    // log::info!("{:?}", file_start_map);

    Ok(file_diff_map)
}

