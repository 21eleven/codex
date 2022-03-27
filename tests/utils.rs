use walkdir::WalkDir;
use std::path::{Path, PathBuf};

pub fn number_of_nodes<P: AsRef<Path>>(path: P) ->usize {
    WalkDir::new(path)
        .sort_by_file_name()
        .contents_first(true)
        .min_depth(0)
        .into_iter()
        .map(|e| e.unwrap().into_path())
        .filter(|p| p.is_file() && p.ends_with("meta.toml"))
        .count()
    // let metas = WalkDir::new(path)
    //     .sort_by_file_name()
    //     .contents_first(true)
    //     .into_iter()
    //     .map(|e| e.unwrap().into_path())
    //     // .filter(|p| p.is_file() && p.ends_with("meta.toml"))
    //     .collect::<Vec<PathBuf>>();
    // dbg!(&metas);
    // metas.len()
}
