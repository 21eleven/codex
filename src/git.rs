use chrono::Local;
use git2::build::RepoBuilder;
use log::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::str;

use git2::{
    Commit, Diff, DiffDelta, DiffFormat, DiffHunk, DiffLine, DiffOptions, FetchOptions, ObjectType,
    Oid, Repository,
};
use git2::{Cred, PushOptions, RemoteCallbacks};
use std::env;

static DEFAULT_COMMIT_MSG: &str = "."; // what should be the default message???
static GLOB_ALL: &str = "*";

pub fn repo() -> Result<Repository, git2::Error> {
    Repository::open("./")
}

pub fn stage_paths(paths: Vec<&Path>) -> Result<(), git2::Error> {
    let repo = repo()?;
    let mut index = repo.index()?;
    index.add_all(paths, git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
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

pub fn get_ancestor_with_main_branch(repo: &Repository) -> Result<Commit, git2::Error> {
    // Ok i should make this module have a
    // Repo struct and some helper functions
    // any func that takes repo should go on
    // the Repo struct which will wrap
    // git2::Repository
    let last_commit = find_last_commit(repo)?;
    let main_commit = get_last_commit_of_branch(repo, "main")?;
    // do i need to find annotated commits?
    let main = repo.find_annotated_commit(main_commit.id())?;
    let other = repo.find_annotated_commit(last_commit.id())?;
    Ok(repo.find_commit(repo.merge_base(main.id(), other.id())?)?)
}

#[derive(Debug)]
pub struct DiffWords {
    words: HashMap<(Oid, Oid), (Vec<String>, Vec<String>)>,
}

impl DiffWords {
    pub fn new() -> Self {
        DiffWords {
            words: HashMap::new(),
        }
    }
    pub fn insert(&mut self, delta: DiffDelta, line: DiffLine) {
        if delta
            .new_file()
            .path()
            .unwrap_or_else(|| Path::new(""))
            .file_name()
            .unwrap_or_default()
            != "meta.toml"
        {
            let key = (delta.old_file().id(), delta.new_file().id());
            match line.origin() {
                '+' => {
                    for word in str::from_utf8(line.content()).unwrap().split_whitespace() {
                        self.words
                            .entry(key)
                            .or_insert((vec![], vec![]))
                            .1
                            .push(String::from(word));
                    }
                }
                '-' => {
                    for word in str::from_utf8(line.content()).unwrap().split_whitespace() {
                        self.words
                            .entry(key)
                            .or_insert((vec![], vec![]))
                            .0
                            .push(String::from(word));
                    }
                }
                _ => {}
            }
        }
    }
    pub fn diff_words_added(&mut self) -> u64 {
        let mut added = 0;
        for (left, right) in self.words.values() {
            for result in lcs_diff::diff(left, right) {
                if let lcs_diff::DiffResult::Added(w) = result {
                    added += 1;
                }
            }
        }
        added
    }
}

pub fn capture_diff_line(
    delta: DiffDelta,
    _hunk: Option<DiffHunk>,
    line: DiffLine,
    diff: &mut DiffWords,
    print: bool,
) -> bool {
    let content = String::from(str::from_utf8(line.content()).unwrap());
    if print {
        match line.origin() {
            '+' | '-' => {
                debug!("line: [{}] {:?}", line.origin(), content);
            }
            _ => {}
        }
    }

    diff.insert(delta, line);

    true
}
fn diff<'a>(repo: &'a Repository, commit: &'a Commit) -> Result<Diff<'a>, git2::Error> {
    let mut opts = DiffOptions::new();
    opts.patience(true);
    repo.diff_tree_to_workdir_with_index(Some(&commit.tree().unwrap()), Some(&mut opts))
}

pub fn diff_w_main() -> Result<u64, git2::Error> {
    let repo = repo()?;
    let commit = get_ancestor_with_main_branch(&repo).unwrap();
    debug!("ancestor w main sha1 {:?}", &commit);
    diff_w_commit(&repo, &commit)
}

pub fn diff_w_last_commit() -> Result<u64, git2::Error> {
    let repo = repo()?;
    let commit = find_last_commit(&repo).unwrap();
    diff_w_commit(&repo, &commit)
}

pub fn diff_w_commit(repo: &Repository, commit: &Commit) -> Result<u64, git2::Error> {
    let diffs = diff(repo, commit)?;
    let mut word_diff = DiffWords::new();
    diffs
        .print(DiffFormat::Patch, |d, h, l| {
            capture_diff_line(d, h, l, &mut word_diff, false)
        })
        .unwrap();
    debug!("/difflines/ {:?}", word_diff);
    Ok(word_diff.diff_words_added())
}

struct DiffReport {
    lines: BTreeMap<String, Vec<String>>,
}

impl DiffReport {
    pub fn new() -> Self {
        Self {
            lines: BTreeMap::new(),
        }
    }
    pub fn insert(&mut self, delta: DiffDelta, line: DiffLine) {
        if delta
            .new_file()
            .path()
            .unwrap_or_else(|| Path::new(""))
            .file_name()
            .unwrap_or_default()
            != "meta.toml"
        {
            if let '+' = line.origin() {
                let content = String::from(str::from_utf8(line.content()).unwrap());
                let key = String::from(delta.new_file().path().unwrap().to_str().unwrap());
                self.lines.entry(key).or_insert_with(Vec::new).push(content);
            }
        }
    }
    pub fn report(&self) -> String {
        let mut output = vec![];
        for (filepath, content) in self.lines.iter() {
            output.push(format!("/// {} ///", filepath.clone()));
            output.push(content.join(""));
        }
        output.join("\n")
    }
}

pub fn diff_report(repo: &Repository, commit: &Commit) -> Result<String, git2::Error> {
    let diffs = diff(repo, commit)?;
    let mut report = DiffReport::new();
    diffs
        .print(DiffFormat::Patch, |d, _, l| {
            report.insert(d, l);
            true
        })
        .unwrap();
    let output = report.report();
    debug!("diff report output String: {:?}", &output);

    Ok(output)
}

pub fn diff_w_main_report() -> Result<String, git2::Error> {
    let repo = repo()?;
    let commit = get_ancestor_with_main_branch(&repo).unwrap();
    diff_report(&repo, &commit)
}

pub fn diff_w_last_commit_report() -> Result<String, git2::Error> {
    let repo = repo()?;
    let commit = find_last_commit(&repo).unwrap();
    diff_report(&repo, &commit)
}

pub fn push_to_git_remote() -> Result<(), git2::Error> {
    commit_all(None)?;
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
    let main_commit = do_fetch(&repo, &["+refs/heads/main:refs/heads/main", "+refs/tags/latest:refs/tags/latest"], &mut remote)?;
    let mut remote = repo.find_remote("origin")?;
    // let mut opts = git2::FetchOptions::new();
    // opts.remote_callbacks(callback());
    // opts.download_tags(git2::AutotagOption::All);
    // remote.download(&["latest"], Some(&mut opts))?;
    let latest_oid = repo.refname_to_id("refs/tags/latest")?;
    let latest = repo.find_tag(latest_oid)?;
    let most_recent_active_branch = latest.message().unwrap();
    // let today_branch_commit = do_fetch(&repo, &[most_recent_active_branch], &mut remote)?;
    let today_branch_commit = do_fetch(&repo, &[&format!("+refs/heads/{}:refs/heads/{}", &most_recent_active_branch, &most_recent_active_branch)], &mut remote)?;
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
                        .conflict_style_merge(true)
                        // .force(),
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
