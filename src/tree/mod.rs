use log::*;
use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

pub struct Tree {
    chk: bool,
}

pub struct TreeError {
    err_text: String,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.err_text)
    }
}

impl fmt::Debug for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TreeError( {} )", self.err_text)
    }
}

impl error::Error for TreeError {}

pub struct NodeFilesMissingError {
    content_file_exists: bool,
    metadata_file_exists: bool,
    node: String,
}

impl NodeFilesMissingError {
    fn err_text(&self) -> String {
        format!(
            "{} {}",
            match (self.content_file_exists, self.metadata_file_exists) {
                (false, true) => "Missing `_.md` for ",
                (true, false) => "Missing `meta.toml` for ",
                _ => "Missing `_.md` and `meta.toml` for ",
            }
            .to_string(),
            self.node
        )
    }
}

impl fmt::Display for NodeFilesMissingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.err_text())
    }
}

impl fmt::Debug for NodeFilesMissingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NodeFilesMissingError( {} )", self.err_text())
    }
}

impl error::Error for NodeFilesMissingError {}

impl Tree {
    pub fn build(root: String) -> Result<Tree> {
        let mut file_check: HashSet<PathBuf> = HashSet::new();
        for fs_node in WalkDir::new(root.as_str())
            .sort_by_file_name()
            .contents_first(true)
        {
            debug!("{:?}", fs_node);
            match fs_node {
                Ok(node_path) => {
                    // `Path.to_str()` returns option bc
                    // some operating system allow paths
                    // that are not valid UTF-8... 🙄
                    if node_path.path().to_str() == Some(root.as_str()) {
                        debug!("skipping root dir {:?} in tree build", &root);
                        continue;
                    } else if !node_path.path().is_dir() {
                        // should *always* encounter node files fites
                        // when dir is encounter will check in set to
                        // verify dir struct not corrupt
                        file_check.insert(node_path.into_path());
                    } else {
                        match (
                            file_check.contains(&node_path.path().join("_.md")),
                            file_check.contains(&node_path.path().join("meta.toml")),
                        ) {
                            (true, true) => {}
                            (c1, c2) => {
                                return Err(NodeFilesMissingError {
                                    content_file_exists: c1,
                                    metadata_file_exists: c2,
                                    node: {
                                        match node_path.path().to_str() {
                                            Some(path) => String::from(path),
                                            _ => "".to_owned(),
                                        }
                                    },
                                }
                                .into())
                            }
                        }
                    }
                }
                Err(e) => return Err(Box::new(e)),
            }
        }
        Ok(Tree { chk: true })
    }
}
pub fn discover_tree(root: String) -> Result<Tree> {
    Ok(Tree { chk: true })
}

pub fn new_sibling_id(path: String) -> Result<i64> {
    todo!()
}
