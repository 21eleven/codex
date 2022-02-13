use chrono::Local;
use log::*;
use std::path::Path;
use std::str;

use git2::{Commit, ObjectType, Repository};
pub mod diff;
pub mod sync;
pub use sync::{fetch_and_pull, git_clone, push_to_git_remote};

static DEFAULT_COMMIT_MSG: &str = "."; // what should be the default message???
static GLOB_ALL: &str = "*";

pub fn repo() -> Result<Repository, git2::Error> {
    Repository::open("./")
}

pub fn stage_paths(paths: Vec<&Path>) -> Result<(), git2::Error> {
    let repo = repo()?;
    let mut index = repo.index()?;
    index.add_all(&paths, git2::IndexAddOption::DEFAULT, None)?;
    index.update_all(paths, None)?;
    index.write()?;
    // what is the difference between write and write_tree
    index.write_tree()?;
    Ok(())
}

pub fn stage_all() -> Result<(), git2::Error> {
    stage_paths(vec![Path::new("*")])?;
    Ok(())
}

pub fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    // let head = repo.head().unwrap();
    // let oid = head.target().unwrap();
    // let commit = repo.find_commit(oid).unwrap();
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

pub fn make_branch_and_checkout(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
    let last_commit = find_last_commit(repo)?;
    repo.branch(branch_name, &last_commit, false)?;
    let obj = repo
        .revparse_single(&("refs/heads/".to_owned() + branch_name))
        .unwrap();

    repo.checkout_tree(&obj, None)?;

    repo.set_head(&("refs/heads/".to_owned() + branch_name))?;
    Ok(())
}

fn checkout_branch(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
    let obj = repo
        .revparse_single(&("refs/heads/".to_owned() + branch_name))
        .unwrap();

    repo.checkout_tree(&obj, None)?;

    repo.set_head(&("refs/heads/".to_owned() + branch_name))?;
    Ok(())
}

pub fn get_last_commit_of_branch<'repo>(
    repo: &'repo Repository,
    branch_name: &str,
) -> Result<Commit<'repo>, git2::Error> {
    repo.revparse_single(&("refs/heads/".to_owned() + branch_name))?
        .peel_to_commit()
}

pub fn commit_paths(
    repo: &Repository,
    paths: Vec<&Path>,
    message: &str,
) -> Result<(), git2::Error> {
    let sig = repo.signature()?;
    let mut index = repo.index()?;
    index.add_all(paths, git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;
    let parents = match find_last_commit(repo) {
        Ok(commit) => vec![commit],
        Err(_) => vec![],
    };
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        parents.iter().collect::<Vec<&Commit>>().as_slice(),
    )?;
    Ok(())
}

pub fn commit_all(message: Option<&str>) -> Result<(), git2::Error> {
    let message = match message {
        Some(msg) => msg,
        None => DEFAULT_COMMIT_MSG,
    };

    commit_paths(&repo()?, vec![Path::new(GLOB_ALL)], message)?;
    Ok(())
}
pub fn commit_any(message: Option<&str>) -> Result<(), git2::Error> {
    let message = match message {
        Some(msg) => msg,
        None => DEFAULT_COMMIT_MSG,
    };
    let repo = repo()?;
    if repo_has_uncommitted_changes(&repo)? {
        commit_all(Some(message))
    } else {
        Ok(())
    }
}

pub fn commit_staged(message: Option<&str>) -> Result<(), git2::Error> {
    let message = match message {
        Some(msg) => msg,
        None => DEFAULT_COMMIT_MSG,
    };
    let repo = repo()?;
    let sig = repo.signature()?;
    let mut index = repo.index()?;
    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;
    let parents = match find_last_commit(&repo) {
        Ok(commit) => vec![commit],
        Err(_) => vec![],
    };
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        parents.iter().collect::<Vec<&Commit>>().as_slice(),
    )?;
    Ok(())
}
fn repo_has_uncommitted_changes(repo: &Repository) -> Result<bool, git2::Error> {
    let last_commit = find_last_commit(repo).unwrap();
    Ok(repo
        .diff_tree_to_workdir(Some(&last_commit.tree()?), None)?
        .deltas()
        .len()
        != 0)
}

pub fn handle_git_branching() -> Result<(), git2::Error> {
    let repo = repo()?;
    let today_branch_name = Local::now().format("%Y%m%d").to_string();
    let current_branch = repo.head()?.name().unwrap_or("").to_string();

    if current_branch != format!("refs/heads/{}", today_branch_name) {
        commit_any(None)?;
        // what if current branch is main? shouldn't be ever yea?
        // TODO need to get default branch from git2::Config::open_global()
        let last_commit = find_last_commit(&repo)?;
        let main_commit = get_last_commit_of_branch(&repo, "main")?;

        if last_commit.id() != main_commit.id() {
            checkout_branch(&repo, "main")?;
            // do i need to find annotated commits?
            let main = repo.find_annotated_commit(main_commit.id())?;
            let other = repo.find_annotated_commit(last_commit.id())?;
            let main_tree = repo.find_commit(main.id())?.tree()?;
            let other_tree = repo.find_commit(other.id())?.tree()?;
            let ancestor = repo
                .find_commit(repo.merge_base(main.id(), other.id())?)?
                .tree()?;
            let mut idx = repo.merge_trees(&ancestor, &main_tree, &other_tree, None)?;
            // let mut idx = repo.merge_commits(&main_commit, &last_commit, None)?;
            let result_tree = repo.find_tree(idx.write_tree_to(&repo)?)?;
            repo.checkout_index(Some(&mut idx), None)?;
            let sig = repo.signature()?;
            let _merge_commit = repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("merge day branch {} into main", current_branch),
                &result_tree,
                &[&main_commit, &last_commit],
            )?;
        }
        make_branch_and_checkout(&repo, &today_branch_name)?;
    } else {
        debug!("staying on branch: {}", &current_branch);
    }
    Ok(())
}

pub fn get_ancestor_with_main_branch(repo: &Repository) -> Result<Commit, git2::Error> {
    // Ok i should make this module have a
    // Repo struct and some helper functions
    // any func that takes repo should go on
    // the Repo struct which will wrap
    // git2::Repository
    let last_commit = find_last_commit(repo)?;
    // TODO need to get default branch from git2::Config::open_global()
    let main_commit = get_last_commit_of_branch(repo, "main")?;
    // do i need to find annotated commits?
    let main = repo.find_annotated_commit(main_commit.id())?;
    let other = repo.find_annotated_commit(last_commit.id())?;
    Ok(repo.find_commit(repo.merge_base(main.id(), other.id())?)?)
}
