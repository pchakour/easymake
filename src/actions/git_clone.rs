use config_macros::ActionDoc;
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    Cred, Error, FetchOptions, ObjectType, Progress, RemoteCallbacks, Repository,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

use crate::{
    console::log,
    emake::{self, loader::TargetType, InFile, PluginAction},
    CREDENTIALS_STORE,
};

use super::Action;
pub static ID: &str = "git_clone";

#[derive(ActionDoc, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[action_doc(
    id = "git_clone",
    short_desc = "Clone a git repository",
    example = "
targets:
    git_clone:
        steps:
            - description: Cloning a repository
              git_clone:
                url: https://github.com/githubtraining/training-manual.git
                destination: \"{{ EMAKE_OUT_DIR }}/training\"
"
)]
pub struct GitCloneAction {
    #[action_prop(description = "Url of the repository to clone", required = true)]
    pub url: String,

    #[action_prop(description = "Clone destination", required = true)]
    pub destination: String,

    #[action_prop(
        description = "Commit to checkout. Could be a sha, a tag or a branch",
        required = false,
        default = "main"
    )]
    pub commit: Option<String>,

    #[action_prop(
        description = "Auth username when cloning with https",
        required = false
    )]
    pub username: Option<String>,

    #[action_prop(
        description = "Auth password when cloning with https",
        required = false
    )]
    pub password: Option<String>,

    #[action_prop(
        description = "Path to ssh key when cloning with ssh",
        required = false
    )]
    pub ssh_key: Option<String>,

    #[action_prop(description = "Clone inside the specify directory", required = false)]
    pub clone_inside: Option<bool>,
}

pub struct GitClone;

fn compile_secret(
    secret: &str,
    emakefile_cwd: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) -> Result<String, Box<dyn std::error::Error>> {
    // TODO externalize this code to be reused anywhere and when donwloading files
    let mut compiled_secret =
        emake::compiler::compile(secret, &emakefile_cwd.to_string(), maybe_replacements, None);
    let result_secret = emake::loader::get_target_on_path(
        &compiled_secret,
        emakefile_cwd,
        Some(TargetType::Secrets),
    );

    if result_secret.is_ok() {
        match result_secret.unwrap() {
            emake::loader::Target::SecretEntry(secret_config) => {
                if !secret_config.contains_key("type") {
                    return Err(format!("The secret {} must contains a type", secret).into());
                }
                let secret_type =
                    String::from(secret_config.get("type").unwrap().as_str().unwrap());
                let maybe_secret_plugin = CREDENTIALS_STORE.get(&secret_type);
                if let Some(secret_plugin) = maybe_secret_plugin {
                    compiled_secret = secret_plugin.extract(&secret_config);
                } else {
                    return Err(
                        format!("The credential type {} does not exist", secret_type).into(),
                    );
                }
            }
            _ => {}
        };
    }

    Ok(compiled_secret)
}

fn clone_and_checkout(
    url: &str,
    path: &PathBuf,
    fetch_opts: FetchOptions,
    reference: String, // branch | tag | sha
) -> Result<(), Error> {
    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_opts);
    let repo = builder.clone(url, path)?;

    // Resolution candidates (order matters)
    let candidates = [
        &reference,
        &format!("refs/heads/{}", reference),
        &format!("origin/{}", reference),
        &format!("refs/tags/{}", reference),
    ];

    let mut target = None;

    for spec in &candidates {
        if let Ok(obj) = repo.revparse_single(spec) {
            target = Some(obj);
            break;
        }
    }

    // Fallback: let libgit2 try harder (commit SHA, HEAD~1, etc.)
    let target = match target {
        Some(obj) => obj,
        None => repo.revparse_single(&reference)?,
    };

    // Checkout
    repo.checkout_tree(&target, Some(CheckoutBuilder::new().force()))?;

    // Set HEAD correctly
    match target.kind() {
        Some(ObjectType::Commit) => {
            repo.set_head_detached(target.id())?;
        }
        _ => {
            // branch or tag
            repo.set_head(&target.id().to_string())?;
        }
    }

    Ok(())
}

fn resolve_remote_ref(repo_url: &String, name: &str) -> Result<Option<String>, git2::Error> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, user, _| Cred::ssh_key_from_agent(user.unwrap()));

    let repo = Repository::init_bare("git-probe")?;
    let mut remote = repo.remote("origin", repo_url)?;
    remote.connect(git2::Direction::Fetch)?;

    let refs = remote.list()?;

    return Ok(Some(format!("tags/{}", name)));
    // for r in refs {
    //     if r.name() == format!("refs/heads/{}", name)
    //         || r.name() == format!("refs/tags/{}", name)
    //     {
    //         return Ok(Some(r.name().to_string()));
    //     }
    // }

    // Ok(None)
}

impl Action for GitClone {
    fn insert_in_files<'a>(
        &'a self,
        action: &'a PluginAction,
        in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::GitClone { git_clone } => {
                    in_files.push(InFile::Simple(git_clone.url.clone()));
                }
                _ => {}
            }
        })
    }

    fn insert_out_files<'a>(
        &'a self,
        action: &'a PluginAction,
        out_files: &'a mut Vec<String>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::GitClone { git_clone } => {
                    let mut destination = PathBuf::from(git_clone.destination.clone());
                    out_files.push(destination.to_string_lossy().to_string());
                }
                _ => {}
            }
        })
    }

    fn run<'a>(
        &'a self,
        _target_id: &'a str,
        step_id: &'a str,
        emakefile_cwd: &'a str,
        _silent: bool,
        action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_files: &'a Vec<String>,
        _working_dir: &'a String,
        maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>> {
        Box::pin(async move {
            let git_action = match action {
                PluginAction::GitClone { git_clone } => git_clone,
                _ => {
                    log::panic!("Error when using git_clone");
                }
            };

            for (index, repository) in in_files.iter().enumerate() {
                let mut commit = None;
                if git_action.commit.is_some() {
                    commit = Some(emake::compiler::compile(
                        git_action.commit.as_ref().unwrap(),
                        &emakefile_cwd.to_string(),
                        maybe_replacements,
                        None,
                    ));

                    if commit.is_some() {
                        let parsed_compiled_commit: Result<Vec<String>, _> =
                            serde_json::from_str(&commit.clone().unwrap());

                        match parsed_compiled_commit {
                            Ok(compiled) => commit = Some(compiled[index].clone()),
                            Err(_) => (),
                        }
                    }
                }

                let default_branch = commit.unwrap_or(String::from("main"));
                let t = resolve_remote_ref(&repository, &default_branch);
                log::action_info!(step_id, ID, "AAA {:?}", t);
                // let mut branches_candidates = Vec::from([
                //     format!("refs/tags/{}", default_branch),
                //     format!("refs/heads/{}", default_branch),
                //     format!("origin/{}", default_branch),
                //     default_branch,
                // ]);

                // loop {
                let mut destination = PathBuf::from(&out_files[0]);

                if git_action.clone_inside.unwrap_or(false) {
                    if !fs::exists(&destination).unwrap() {
                        fs::create_dir_all(&destination).unwrap();
                    }

                    let git_paths = repository.split("/");
                    let repository_name = git_paths.last().unwrap().strip_suffix(".git").unwrap();
                    destination = destination.join(repository_name);
                }

                log::action_info!(step_id, ID, "Cloning repository {}", repository);
                let mut callbacks = RemoteCallbacks::new();

                // Progress callback
                callbacks.transfer_progress(|stats: Progress| {
                    let total = stats.total_objects();
                    if total > 0 {
                        let received = stats.received_objects();
                        let indexed = stats.indexed_objects();

                        let download_percent = received * 100 / total;
                        let index_percent = indexed * 100 / total;
                        let percent = (download_percent + index_percent) / 2;

                        log::action_debug!(
                            step_id,
                            ID,
                            "Percent {}% | Cloning repository {}",
                            percent,
                            repository
                        );
                    }

                    true
                });

                if git_action.password.is_some() && git_action.ssh_key.is_some() {
                    return Err(format!("Error when cloning repository: You can't specify a password and an ssh_key at same time").into());
                }

                if git_action.password.is_some() {
                    callbacks.credentials(|_url, username_from_url, _allowed_types| {
                        let default_username =
                            String::from(username_from_url.unwrap_or("username"));
                        let username_secret =
                            git_action.username.as_ref().unwrap_or(&default_username);
                        let username =
                            compile_secret(username_secret, emakefile_cwd, maybe_replacements)
                                .map_err(|e| {
                                    git2::Error::from_str(&format!("username error: {}", e))
                                })?;

                        let password_secret = git_action.password.as_ref().unwrap();
                        let password =
                            compile_secret(password_secret, emakefile_cwd, maybe_replacements)
                                .map_err(|e| {
                                    git2::Error::from_str(&format!("password error: {}", e))
                                })?;
                        Cred::userpass_plaintext(&username, &password)
                    });
                }

                if git_action.ssh_key.is_some() {
                    callbacks.credentials(|_url, username_from_url, _allowed_types| {
                        Cred::ssh_key(
                            git_action
                                .username
                                .as_ref()
                                .unwrap_or(&String::from(username_from_url.unwrap())),
                            None,
                            Path::new(git_action.ssh_key.as_ref().unwrap()),
                            None,
                        )
                    });
                }

                let mut builder = git2::build::RepoBuilder::new();

                let mut fetch_opts = FetchOptions::new();
                fetch_opts.depth(1);
                fetch_opts.remote_callbacks(callbacks);
                let branch = t.unwrap_or(Some(default_branch)).unwrap();
                builder.fetch_options(fetch_opts).branch(&branch);
                log::action_info!(step_id, ID, "Clone branch {}", branch);
                match builder.clone(&repository, &destination) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(format!("Error when cloning repository: {}", e).into());
                    }
                }

                // match clone_and_checkout(
                //     &repository,
                //     &destination,
                //     fetch_opts,
                //     commit.unwrap_or(String::from("main")),
                // ) {
                //     Ok(_) => {},
                //     Err(e) => {
                //         return Err(format!("Error when cloning repository: {}", e).into());
                //     },
                // }
                // }
            }
            Ok(())
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
    fn get_checksum(&self) -> Option<String> {
        None
    }
}
