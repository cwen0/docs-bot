use std::path::PathBuf;
use dialoguer::PasswordInput;
use git2::Error;
use git2::{Repository, Branch, BranchType};
use git2_credentials::CredentialHandler;
use git2_credentials::CredentialUI;

// #[derive(Debug)]
// pub enum Error {
//     CloneError(String),
//     CheckoutError(String),
//     PullError(),
//     ShowError(),
//     PushError(String),
//     HashError(),
//     ApplyError(),
//     Fatal(String),
//     // LoginError(String),
// }
//
// impl Display for Error {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         use self::Error::*;
//         match self {
//             CloneError(s) => write!(f, "unable to git clone project {}", s),
//             CheckoutError(s) => write!(f, "unable to checkout branch {}", s),
//             PullError() => write!(f, "unable to pull last changes"),
//             ShowError() => write!(f, "unable to show last commit"),
//             HashError() => write!(f, "unable to generate hash based on last commit"),
//             PushError(s) => write!(f, "unable to push changes to {}", s),
//             ApplyError() => write!(f, "unable to apply patch"),
//             // LoginError(s) => write!(f, "unable to login github, username: {}", s),
//             Fatal(s) => write!(f, "unexpected error {}", s),
//         }
//     }
// }
//
// impl From<std::io::Error> for Error {
//     fn from(error: std::io::Error) -> Self {
//         Error::Fatal(error.to_string())
//     }
// }

#[derive(thiserror::Error, Debug)]
#[error("Cannot clone repository with {remote_url} because of {source}")]
pub struct CloneError {
    pub source: git2::Error,
    pub remote_url: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct GitCredential {
    username: String,
    password: String,
}

impl GitCredential {
    pub fn new(username: String, password: String) -> GitCredential {
        GitCredential { username, password }
    }
}

impl CredentialUI for GitCredential {
    fn ask_user_password(&self, _: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
        Ok((self.username.clone(), self.password.clone()))
    }

    fn ask_ssh_passphrase(
        &self,
        passphrase_prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let passphrase: String = PasswordInput::new()
            .with_prompt(passphrase_prompt)
            .allow_empty_password(true)
            .interact()?;

        Ok(passphrase)
    }
}

#[derive(Clone, Debug)]
pub struct Git {
    pub workdir: PathBuf,
    pub cred: GitCredential,
}

impl Git {
    pub fn new(workdir: PathBuf, cred: GitCredential) -> anyhow::Result<Git> {
        Ok(Git {
            workdir,
            cred,
        })
    }

    pub fn clone_repo(&self, name: &str, remote_url: &str) -> anyhow::Result<git2::Repository, CloneError> {
        // let root_dir = self.workdir.clone();
        let mut local_path = self.workdir.clone();
        local_path.push(name);

        let remote_callbacks = self.create_remote_callback().map_err(|s| CloneError {
            source: s,
            remote_url: remote_url.to_string(),
        })?;

        let mut fo = git2::FetchOptions::new();

        fo.remote_callbacks(remote_callbacks)
            .download_tags(git2::AutotagOption::All)
            .update_fetchhead(true);

        git2::build::RepoBuilder::new()
            .fetch_options(fo)
            .clone(remote_url, local_path.as_path())
            .map_err(|s| CloneError {
                source: s,
                remote_url: remote_url.to_string(),
            })
    }

    pub fn exists(&self) -> bool {
        self.workdir.exists()
    }

    pub fn checkout(&self, repo: &Repository, branch: &str) -> anyhow::Result<()> {
        let obj = repo.revparse_single(&("refs/heads/".to_owned() + branch))?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head(&("refs/heads/".to_owned() + branch))?;

        Ok(())
    }

    pub fn create_branch<'a>(
        &self,
        repo: &'a Repository,
        new_branch: &str,
        base_branch: &str,
    ) -> anyhow::Result<Branch<'a>, Error> {
        let base_branch = repo.find_branch(base_branch, BranchType::Local)?;

        let oid = base_branch.get().target().unwrap();
        let commit = repo.find_commit(oid)?;

        repo.branch(new_branch, &commit, false)
    }

    pub fn push_branch(
        &self,
        repo: &Repository,
        branch: &str,
        remote_name: &str,
    ) -> anyhow::Result<(), Error> {
        let mut origin = repo.find_remote(remote_name).unwrap();

        let remote_callbacks = self.create_remote_callback().unwrap();

        let mut po = git2::PushOptions::new();
        po.remote_callbacks(remote_callbacks);

        origin.push(&[&ref_by_branch(branch)], Some(&mut po)).unwrap();

        Ok(())
    }

    pub fn create_remote_callback(&self) -> anyhow::Result<git2::RemoteCallbacks, Error> {
        let mut cb = git2::RemoteCallbacks::new();
        let git_config = git2::Config::open_default()?;
        let credential_ui: Box<dyn CredentialUI> = Box::new(self.cred.clone());

        let mut ch = CredentialHandler::new_with_ui(git_config, credential_ui);

        cb.credentials(move |url, username, allowed| ch.try_next_credential(url, username, allowed));

        Ok(cb)
    }
}

pub fn ref_by_branch(branch: &str) -> String {
    format!("refs/heads/{}:refs/heads/{}", branch, branch)
}