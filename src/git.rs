use std::path::Path;

use git2::{Commit, ObjectType, Repository};

static DEFAULT_COMMIT_MSG: &str = ".";  // what should be the default message???
static GLOB_ALL: &str = "codex/*";

pub fn repo() -> Result<Repository, git2::Error> {
    Ok(Repository::open("./")?)
}

pub fn stage_paths(paths: Vec<&Path>) -> Result<(), git2::Error> {
    let repo = repo()?;
    let mut index = repo.index()?;
    index.add_all(paths, git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

pub fn stage_all() -> Result<(), git2::Error> {
    stage_paths(vec![Path::new("codex/*")])?;
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

pub fn commit_all(message: Option<&str>) -> Result<(), git2::Error> {
    let message = match message {
        Some(msg) => msg,
        None => DEFAULT_COMMIT_MSG,
    };

    commit_paths(&repo()?, vec![Path::new(GLOB_ALL)], message)?;
    Ok(())
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
