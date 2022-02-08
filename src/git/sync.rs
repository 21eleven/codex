use crate::git::{checkout_branch, commit_any, repo};
use chrono::Local;
use git2::build::RepoBuilder;
use git2::{Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use log::*;
use std::env;
use std::path::Path;
pub fn push_to_git_remote() -> Result<(), git2::Error> {
    commit_any(None)?;
    let mut push_opts = PushOptions::default();
    push_opts.remote_callbacks(callback());
    let repo = repo()?;
    let current_branch = repo
        .head()?
        .name()
        .unwrap_or("/")
        .rsplit_once("/")
        .unwrap()
        .1
        .to_string();
    let latest_commit = repo
        .revparse_single(&current_branch)
        .unwrap()
        .peel_to_commit()
        .unwrap();
    repo.tag(
        "latest",
        latest_commit.as_object(),
        &repo.signature()?,
        &current_branch,
        true,
    )?;
    let mut remote = repo.find_remote("origin")?;
    // TODO need to get default branch from git2::Config::open_global()
    remote.push(
        &[
            format!(
                "refs/heads/{}:refs/heads/{}",
                current_branch, current_branch
            ),
            "refs/heads/main:refs/heads/main".to_owned(),
            "+refs/tags/latest:refs/tags/latest".to_owned(),
        ],
        Some(&mut push_opts),
    )?;
    debug!("branch pushed");
    Ok(())
}
fn callback() -> RemoteCallbacks<'static> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(|_url, username, _allowed_types| {
        debug!(
            "CB\nurl: {:?}\nusername: {:?}\nallowed types: {:?}",
            _url, &username, &_allowed_types
        );
        Cred::ssh_key(
            username.unwrap(),
            None,
            std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
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
    let repo = repo()?;
    let mut remote = repo.find_remote("origin")?;
    let today_branch_name = Local::now().format("%Y%m%d").to_string();
    // TODO need to get default branch from git2::Config::open_global()
    let main_commit = do_fetch(
        &repo,
        &[
            "+refs/heads/main:refs/heads/main",
            "+refs/tags/latest:refs/tags/latest",
        ],
        &mut remote,
    )?;
    let mut remote = repo.find_remote("origin")?;
    // let mut opts = git2::FetchOptions::new();
    // opts.remote_callbacks(callback());
    // opts.download_tags(git2::AutotagOption::All);
    // remote.download(&["latest"], Some(&mut opts))?;
    let latest_oid = repo.refname_to_id("refs/tags/latest")?;
    let latest = repo.find_tag(latest_oid)?;
    let most_recent_active_branch = latest.message().unwrap();
    // let today_branch_commit = do_fetch(&repo, &[most_recent_active_branch], &mut remote)?;
    let today_branch_commit = do_fetch(
        &repo,
        &[&format!(
            "+refs/heads/{}:refs/heads/{}",
            &most_recent_active_branch, &most_recent_active_branch
        )],
        &mut remote,
    )?;
    // probably need to wrtie the tree o ftshi commit to the repo
    // let commit_id = &today_branch_commit.id();

    do_merge(&repo, "main", main_commit)?;
    do_merge(&repo, most_recent_active_branch, today_branch_commit)?;
    // checkout_branch(&repo, most_recent_active_branch)?;
    // let tree = repo.head()?.peel_to_tree()?;
    let c = &repo.head()?.peel(git2::ObjectType::Commit)?;

    debug!("APPLYING {}", c.id());
    // repo.checkout_tree(&tree, None)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    Ok(())
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
    remote.fetch(refs, Some(&mut opts), None)?;

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
    Ok(repo.reference_to_annotated_commit(&fetch_head)?)
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
        &mut git2::build::CheckoutBuilder::default()
            // For some reason the force is required to make the working directory actually get updated
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
        normal_merge(&repo, &head_commit, &fetch_commit)?;
    } else {
        debug!("local in sync w remote, nothing to pull");
    }
    Ok(())
}
