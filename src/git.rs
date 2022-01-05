use chrono::Local;
use log::*;
use std::str;
use std::path::Path;
use std::collections::HashMap;

use git2::{Commit, Diff, DiffDelta, DiffFormat, DiffHunk, DiffLine, DiffOptions, Oid, ObjectType, Repository};

static DEFAULT_COMMIT_MSG: &str = "."; // what should be the default message???
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
fn repo_has_uncommitted_changes(repo: &Repository) -> Result<bool, git2::Error> {
    let last_commit = find_last_commit(&repo).unwrap();
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

    if &current_branch != &format!("refs/heads/{}", today_branch_name) {
        if repo_has_uncommitted_changes(&repo)? {
            commit_all(None)?;
        }
        // what if current branch is main? shouldn't be ever yea?
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

pub fn get_ancestor_with_main_branch<'repo>(
    repo: &'repo Repository,
) -> Result<Commit<'repo>, git2::Error> {
    // Ok i should make this module have a 
    // Repo struct and some helper functions
    // any func that takes repo should go on 
    // the Repo struct which will wrap
    // git2::Repository
    let last_commit = find_last_commit(&repo)?;
    let main_commit = get_last_commit_of_branch(&repo, "main")?;
    // do i need to find annotated commits?
    let main = repo.find_annotated_commit(main_commit.id())?;
    let other = repo.find_annotated_commit(last_commit.id())?;
    Ok(repo.find_commit(repo.merge_base(main.id(), other.id())?)?)
}

#[derive(Debug)]
pub struct DiffWords {
    words: HashMap<(Oid, Oid), (Vec<String>, Vec<String>)>
} 

impl DiffWords {
    pub fn new() ->Self {
        DiffWords { words: HashMap::new() }
    }
    pub fn insert(&mut self, delta: DiffDelta, line: DiffLine) {
        let key = (delta.old_file().id(), delta.new_file().id());
        match line.origin() {
            '+' => {
                for word in str::from_utf8(line.content()).unwrap().split_whitespace() {
                    self.words.entry(key)
                        .or_insert((vec![], vec![]))
                        .1.push(String::from(word));
                }
            }
            '-' => {
                for word in str::from_utf8(line.content()).unwrap().split_whitespace() {
                    self.words.entry(key)
                        .or_insert((vec![], vec![]))
                        .0.push(String::from(word));
                }
            }
            _ => {}
        }
    }
    pub fn diff(&mut self) {
        for (left, right) in self.words.values() {
            for result in lcs_diff::diff(left, right) {
                match result {
                    lcs_diff::DiffResult::Added(w) => {
                        debug!("[+] {:?}", w.data);
                    }
                    _ => {}
                }
            }

        }
    }
}

pub fn capture_diff_line(
    delta: DiffDelta,
    hunk: Option<DiffHunk>,
    line: DiffLine,
    // lines: &mut Vec<String>,
    diff: &mut DiffWords,
    print: bool,
) -> bool {
    let content = String::from(str::from_utf8(line.content()).unwrap());
    if print {
        // debug!("delta: {:?}", delta);
        // debug!("hunk: {:?}", hunk);
        // match hunk {
        //     Some(h) => {
        //         debug!("Hunk:\n{:?}",String::from(std::str::from_utf8(h.header()).unwrap()) );
        //     }
        //     None => {}
        // }
        match line.origin() {
            '+'|'-' => {
                debug!("line: [{}] {:?}", line.origin(), content);
            }
            _ => {}
        }
    }

    // lines.push(content);
    diff.insert(delta, line);

    true
}

pub fn get_branch_diff() -> String {
    todo!();
}

pub fn get_diff_word_count() -> u64 {
    todo!();
}
pub fn diff_w_last_commit() -> Result<(), git2::Error> {
    let repo = repo()?;
    let commit = find_last_commit(&repo).unwrap();
    diff_w_commit(&repo, &commit)?;

    Ok(())
}

pub fn diff_w_commit(repo: &Repository, commit: &Commit) -> Result<(), git2::Error> {
    // let commit = get_ancestor_with_main_branch(&repo).unwrap();
    let mut opts = DiffOptions::new();
    opts.patience(true);
    let diffs = repo
        .diff_tree_to_workdir(Some(&commit.tree().unwrap()), Some(&mut opts))
        .unwrap();
    debug!("n deltas: {}", diffs.deltas().len());
    // let mut lines: Vec<String> = vec![];
    let mut lines = DiffWords::new();
    diffs
        .print(DiffFormat::Patch, |d, h, l| {
            capture_diff_line(d, h, l, &mut lines, true)
        })
        .unwrap();
    debug!("/difflines/ {:?}", lines);
    lines.diff();

    Ok(())
}
