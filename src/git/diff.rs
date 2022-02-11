use crate::git::{find_last_commit, get_ancestor_with_main_branch, repo};
use git2::{Commit, Diff, DiffDelta, DiffFormat, DiffHunk, DiffLine, DiffOptions, Oid, Repository};
use log::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::str;
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
