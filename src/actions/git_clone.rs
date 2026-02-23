use config_macros::ActionDoc;
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    AutotagOption, Cred, Error, FetchOptions, ObjectType, Progress, RemoteCallbacks, Repository,
};
use keyring::default;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

use crate::{
    cache::get_working_dir_path,
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

    #[action_prop(
        description = "Overwrite if the directory already exists",
        required = false
    )]
    pub overwrite: Option<bool>,
}

pub struct GitClone;

fn compile_secret(
    secret: &str,
    emakefile_cwd: &str,
    maybe_replacements: Option<HashMap<String, String>>,
) -> Result<String, Box<dyn std::error::Error>> {
    // TODO externalize this code to be reused anywhere and when donwloading files
    let mut compiled_secret =
        emake::compiler::compile(secret, &emakefile_cwd.to_string(), maybe_replacements.as_ref(), None);
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

fn add_credentials(
    mut callbacks: RemoteCallbacks,
    git_action: GitCloneAction,
    emakefile_cwd: String,
    maybe_replacements: Option<HashMap<String, String>>,
) -> RemoteCallbacks {

    // --------------------------------------------------
    // HTTPS credentials
    // --------------------------------------------------
    if git_action.password.is_some() {
        callbacks.credentials(move |_url, username_from_url, _allowed_types| {
            let default_username =
                username_from_url.unwrap_or("username").to_string();

            let username_secret =
                git_action.username.as_ref().unwrap_or(&default_username);

            let username = compile_secret(
                username_secret,
                &emakefile_cwd,
                maybe_replacements.clone(),
            )
            .map_err(|e| git2::Error::from_str(&format!("username error: {}", e)))?;

            let password_secret = git_action.password.as_ref().unwrap();

            let password = compile_secret(
                password_secret,
                &emakefile_cwd,
                maybe_replacements.clone(),
            )
            .map_err(|e| git2::Error::from_str(&format!("password error: {}", e)))?;

            Cred::userpass_plaintext(&username, &password)
        });

        return callbacks;
    }

    // --------------------------------------------------
    // SSH credentials
    // --------------------------------------------------
    if git_action.ssh_key.is_some() {
        callbacks.credentials(move |_url, username_from_url, _allowed_types| {
            Cred::ssh_key(
                git_action
                    .username
                    .as_deref()
                    .unwrap_or(username_from_url.unwrap_or("git")),
                None,
                Path::new(git_action.ssh_key.as_ref().unwrap()),
                None,
            )
        });
    }

    callbacks
}

fn clone_and_checkout(
    repository: &str,
    destination: &Path,
    default_branch: &str,
    fetch_callbacks: RemoteCallbacks<'_>,
    connect_callbacks: RemoteCallbacks<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.depth(1);
    fetch_opts.download_tags(AutotagOption::None);
    fetch_opts.remote_callbacks(fetch_callbacks);

    // Init repository
    let repo = Repository::init(destination).map_err(|e| e.to_string())?;
    repo.remote("origin", repository)
        .map_err(|e| e.to_string())?;

    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;

    // Connect with auth
    remote
        .connect_auth(
            git2::Direction::Fetch,
            Some(connect_callbacks),
            None,
        )
        .map_err(|e| e.to_string())?;

    // Detect branch or tag
    let refs = remote.list().map_err(|e| e.to_string())?;
    let mut maybe_branch = None;

    for r in refs {
        if r.name() == format!("refs/heads/{}", default_branch)
            || r.name() == format!("refs/tags/{}", default_branch)
        {
            maybe_branch = Some(r.name().to_string());
            break;
        }
    }

    let branch = maybe_branch.unwrap_or_else(|| "refs/heads/main".to_string());
    let refs_branch = format!("{}:{}", branch, branch);

    // Fetch
    remote
        .fetch(&[&refs_branch], Some(&mut fetch_opts), None)
        .map_err(|e| e.to_string())?;

    // Checkout
    let obj = repo.revparse_single(&branch).map_err(|e| e.to_string())?;
    let commit = obj.peel_to_commit().map_err(|e| e.to_string())?;

    if repo.find_reference(&branch).is_err() {
        repo.branch(default_branch, &commit, true)
            .map_err(|e| e.to_string())?;
    }

    repo.set_head(&branch).map_err(|e| e.to_string())?;

    repo.checkout_head(Some(CheckoutBuilder::new().force()))
        .map_err(|e| e.to_string())?;

    Ok(())
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
                    let destination = PathBuf::from(git_clone.destination.clone());
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
                PluginAction::GitClone { git_clone } => git_clone.clone(),
                _ => {
                    log::panic!("Error when using git_clone");
                }
            };

            let step_id = step_id.to_string();
            let in_files = in_files.clone();
            let out_files = out_files.clone();
            let maybe_replacements_clone: Option<HashMap<String, String>> =
                maybe_replacements.map(|h| h.clone());
            let emakefile_cwd = emakefile_cwd.to_string();

            let spawn_result = tokio::task::spawn_blocking(move || {
                for (index, repository) in in_files.iter().enumerate() {
                    let mut default_branch = git_action
                        .commit
                        .clone()
                        .unwrap_or_else(|| "main".to_string());

                    default_branch = emake::compiler::compile(
                        &default_branch,
                        &emakefile_cwd.to_string(),
                        maybe_replacements_clone.as_ref(),
                        None,
                    );

                    let parsed_default_branch_result: Result<Vec<String>, _> = serde_json::from_str(&default_branch);

                    match parsed_default_branch_result {
                        Ok(parsed_default_branch) => {
                            if parsed_default_branch.len() == 1  {
                                default_branch = parsed_default_branch[0].clone();
                            } else {
                                default_branch = parsed_default_branch[index].clone();
                            }
                        },
                        Err(_) => (),
                    }
                    

                    let mut destination = PathBuf::from(&out_files[0]);

                    if git_action.clone_inside.unwrap_or(false) {
                        if !destination.exists() {
                            fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
                        }

                        let repo_name = repository
                            .split('/')
                            .last()
                            .unwrap()
                            .strip_suffix(".git")
                            .unwrap();

                        destination = destination.join(repo_name);
                    }
                    
                    if destination.exists() {
                        if git_action.overwrite.unwrap_or(false) {
                            fs::remove_dir_all(&destination).map_err(|e| e.to_string())?;
                        } else {
                            return Err(format!(
                                "Directory {} already exists",
                                destination.display()
                            ));
                        }
                    }

                    log::action_info!(&step_id, ID, "Cloning repository {}", repository);

                    if git_action.password.is_some() && git_action.ssh_key.is_some() {
                        return Err(format!("Error when cloning repository: You can't specify a password and an ssh_key at same time").into());
                    }


                    let mut fetch_callbacks = RemoteCallbacks::new();

                    // Progress callback
                    fetch_callbacks.transfer_progress(|stats: Progress| {
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

                    fetch_callbacks = add_credentials(
                        fetch_callbacks,
                        git_action.clone(),
                        emakefile_cwd.clone(),
                        maybe_replacements_clone.clone(),
                    );

                    let connect_callbacks = add_credentials(
                        RemoteCallbacks::new(),
                        git_action.clone(),
                        emakefile_cwd.clone(),
                        maybe_replacements_clone.clone(),
                    );
                    
                    clone_and_checkout(
                        repository,
                        &destination,
                        &default_branch,
                        fetch_callbacks,
                        connect_callbacks,
                    ).map_err(|e| e.to_string())?
                }

                Ok(())
            })
            .await
            .unwrap();

            if spawn_result.is_err() {
                let error_message = spawn_result.err().unwrap().to_string();
                let error: Result<(), Box<dyn std::error::Error>> = Err(error_message.into());
                return error;
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
