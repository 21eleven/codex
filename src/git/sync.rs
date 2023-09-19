use crate::git::{checkout_branch, commit_all, repo};
use git2::build::RepoBuilder;
use git2::{Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use log::*;
use regex::Regex;
use std::path::Path;
use std::process::{Command, ExitStatusError};
use std::{env, fs};

pub async fn push_all_to_git_remote_cmd() {
    commit_all(None).unwrap();
    let push_all = Command::new("git")
        .arg("push")
        .arg("--all")
        .output()
        .expect("push all command fail");
    debug!("GIT PUSH: {}", String::from_utf8(push_all.stdout).unwrap());
    debug!(
        "GIT PUSH STDERR: {}",
        String::from_utf8(push_all.stderr).unwrap()
    );
    push_all
        .status
        .exit_ok()
        .expect("push all non zero exit code");
}

pub async fn push_to_git_remote() -> Result<(), git2::Error> {
    // commit_any(None)?; -- not currently working
    commit_all(None).unwrap();
    let mut push_opts = PushOptions::default();
    push_opts.remote_callbacks(callback());
    let repo = repo()?;
    let mut remote = repo.find_remote("origin")?;
    // TODO need to get default branch from git2::Config::open_global()
    remote.push(
        &[
            "refs/heads/main:refs/heads/main".to_owned(),
        ],
        Some(&mut push_opts),
    )?;
    debug!("main branch pushed");
    Ok(())
}

fn callback() -> RemoteCallbacks<'static> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(|_url, username, _allowed_types| {
        debug!(
            "CB\nurl: {:?}\nusername: {:?}\nallowed types: {:?}",
            _url, &username, &_allowed_types
        );
        // Cred::ssh_key(
        //     username.unwrap(),
        //     None,
        //     std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
        //     None,
        // )
        Cred::ssh_key_from_memory(
            username.unwrap(),
            None,
            &fs::read_to_string(std::path::Path::new(&format!(
                "{}/.ssh/id_ghub",
                env::var("HOME").unwrap()
            )))
            .unwrap(),
            None,
        )
    });
    cb
}

pub fn git_clone(url: &str) -> Result<(), git2::Error> {
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callback());
    opts.download_tags(git2::AutotagOption::All);
    let mut builder = RepoBuilder::new();
    builder.fetch_options(opts);
    let repo = builder.clone(url, Path::new("./"))?;
    let latest_oid = repo.refname_to_id("refs/tags/latest")?;
    let latest = &repo.find_tag(latest_oid)?;
    let most_recent_active_branch = latest.message().unwrap().trim().to_string();
    let mut remote = repo.find_remote("origin")?;
    do_fetch(
        &repo,
        &[&format!(
            "refs/heads/{}:refs/heads/{}",
            &most_recent_active_branch, &most_recent_active_branch
        )],
        &mut remote,
    )?;
    checkout_branch(&repo, &most_recent_active_branch)?;
    Ok(())
}

pub fn fetch_and_pull() -> Result<(), git2::Error> {
    let repo = repo().unwrap();
    let mut remote = repo.find_remote("origin").unwrap();
    // let today_branch_name = Local::now().format("%Y%m%d").to_string();
    // TODO need to get default branch from git2::Config::open_global()
    // let gox_repo = Repository::open("./").expect("unable to open repo | gitoxide");
    // gox_repo.fe

    let main_commit = do_fetch(
        &repo,
        &[
            "+refs/heads/main:refs/heads/main",
            "+refs/tags/latest:refs/tags/latest",
        ],
        &mut remote,
    )
    .unwrap();
    let mut remote = repo.find_remote("origin").unwrap();
    // let mut opts = git2::FetchOptions::new();
    // opts.remote_callbacks(callback());
    // opts.download_tags(git2::AutotagOption::All);
    // remote.download(&["latest"], Some(&mut opts)).unwrap();
    let latest_oid = repo.refname_to_id("refs/tags/latest").unwrap();
    let latest = repo.find_tag(latest_oid).unwrap();
    let most_recent_active_branch = latest.message().unwrap();
    // let today_branch_commit = do_fetch(&repo, &[most_recent_active_branch], &mut remote).unwrap();
    let today_branch_commit = do_fetch(
        &repo,
        &[&format!(
            "+refs/heads/{}:refs/heads/{}",
            &most_recent_active_branch.trim(),
            &most_recent_active_branch.trim()
        )],
        &mut remote,
    )
    .unwrap();
    // probably need to wrtie the tree o ftshi commit to the repo
    // let commit_id = &today_branch_commit.id();

    do_merge(&repo, "main", main_commit).unwrap();
    do_merge(&repo, most_recent_active_branch, today_branch_commit).unwrap();
    // checkout_branch(&repo, most_recent_active_branch)?;
    // let tree = repo.head()?.peel_to_tree()?;
    let c = &repo.head()?.peel(git2::ObjectType::Commit)?;

    debug!("APPLYING {}", c.id());
    // repo.checkout_tree(&tree, None)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    Ok(())
}

pub fn pull_origin_main() -> std::result::Result<(), ExitStatusError> {
    let pull_main = Command::new("git")
        .arg("pull")
        .arg("origin")
        .arg("main")
        .arg("--ff")
        .output()
        .expect("pull main command failed");
    pull_main.status.exit_ok()
}

pub fn cmdline_fetch_and_pull() {
    let fetch = Command::new("git")
        .arg("fetch")
        .arg("--all")
        .output()
        .expect("fetch failed");
    debug!("GIT FETCH: {}", String::from_utf8(fetch.stdout).unwrap());
    debug!(
        "GIT FETCH STDERR: {}",
        String::from_utf8(fetch.stderr).unwrap()
    );
    fetch.status.exit_ok().expect("fetch non zero exit code");
    let chkout_main = Command::new("git")
        .arg("checkout")
        .arg("main")
        .output()
        .expect("checkout main command failed");
    debug!(
        "GIT CHKOUT_MAIN: {}",
        String::from_utf8(chkout_main.stdout).unwrap()
    );
    debug!(
        "GIT CHKOUT_MAIN STDERR: {}",
        String::from_utf8(chkout_main.stderr).unwrap()
    );
    chkout_main
        .status
        .exit_ok()
        .expect("check out of main failed");
    let pull_main = Command::new("git")
        .arg("pull")
        .arg("origin")
        .arg("main")
        .arg("--ff")
        .output()
        .expect("pull main command failed");
    pull_main.status.exit_ok().expect("pull of main failed");
    let ls_branches = Command::new("git")
        .arg("branch")
        .arg("-a")
        .output()
        .expect("branch -a command failed");
    ls_branches.status.exit_ok().expect("branch ls failed");
    let remote_yyyymmdd_branch_patter = Regex::new(r"^\s*remotes/origin/(\d{8})").unwrap();
    let latest = String::from_utf8(ls_branches.stdout)
        .unwrap()
        .lines()
        .rev()
        .find_map(|ln| {
            remote_yyyymmdd_branch_patter
                .captures(ln)
                .map(|cap| cap[1].to_string())
        })
        .expect("no remote branches matching yyyymmdd regex");
    debug!("LATEST BRANCH: {}", latest);
    let chkout_latest = Command::new("git")
        .arg("checkout")
        .arg(latest.clone())
        .output()
        .expect("checkout latest command failed");
    debug!(
        "GIT CHKOUT_LATEST: {}",
        String::from_utf8(chkout_latest.stdout).unwrap()
    );
    debug!(
        "GIT CHKOUT_LATEST STDERR: {}",
        String::from_utf8(chkout_latest.stderr).unwrap()
    );
    chkout_latest
        .status
        .exit_ok()
        .expect("check out of latest failed");
    let pull_latest = Command::new("git")
        .arg("pull")
        .arg("origin")
        .arg(latest)
        .arg("--ff")
        .output()
        .expect("pull latest command failed");
    pull_latest.status.exit_ok().expect("pull of latest failed");
}

fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(callback());
    opts.download_tags(git2::AutotagOption::All);
    debug!("Fetching {:?} for repo", refs);
    remote.fetch(refs, Some(&mut opts), None).unwrap();

    // If there are local objects (we got a thin pack), then tell the user
    // how many objects we saved from having to cross the network.
    let stats = remote.stats();
    if stats.local_objects() > 0 {
        debug!(
            "\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes(),
            stats.local_objects()
        );
    } else {
        debug!(
            "\rReceived {}/{} objects in {} bytes",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes()
        );
    }

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    repo.reference_to_annotated_commit(&fetch_head)
}

fn fast_forward(
    repo: &Repository,
    lb: &mut git2::Reference,
    rc: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };
    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    debug!("{}", msg);
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(
        &mut git2::build::CheckoutBuilder::default(), // For some reason the force is required to make the working directory actually get updated
                                                      // I suspect we should be adding some logic to handle dirty working directory states
                                                      // but this is just an example so maybe not.
                                                      // .force(),
    ))?;
    Ok(())
}

fn normal_merge(
    repo: &Repository,
    local: &git2::AnnotatedCommit,
    remote: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

    if idx.has_conflicts() {
        error!("Merge conficts detected...");
        repo.checkout_index(Some(&mut idx), None)?;
        return Ok(());
    }
    let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
    // now create the merge commit
    let msg = format!("Merge: {} into {}", remote.id(), local.id());
    let sig = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;
    // Do our merge commit and set current branch head to that commit.
    let _merge_commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &msg,
        &result_tree,
        &[&local_commit, &remote_commit],
    )?;
    // Set working tree to match head.
    repo.checkout_head(None)?;
    Ok(())
}

fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> Result<(), git2::Error> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appopriate merge
    if analysis.0.is_fast_forward() {
        debug!("Doing a fast forward");
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true), // .force(),
                ))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(repo, &head_commit, &fetch_commit)?;
    } else {
        debug!("local in sync w remote, nothing to pull");
    }
    Ok(())
}
