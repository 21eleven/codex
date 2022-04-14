use codex::node::{NodeLink, NodeMeta};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn number_of_nodes<P: AsRef<Path>>(path: P) -> usize {
    WalkDir::new(path)
        .sort_by_file_name()
        .contents_first(true)
        .min_depth(0)
        .into_iter()
        .map(|e| e.unwrap().into_path())
        .filter(|p| p.is_file() && p.ends_with("meta.toml"))
        .count()
}

pub fn nodekeys_in_dir<P: AsRef<Path>>(path: P) -> Vec<String> {
    let prefix = path
        .as_ref()
        .to_str()
        .unwrap()
        .chars()
        .chain(['/'])
        .collect::<String>();
    WalkDir::new(path)
        .sort_by_file_name()
        .contents_first(true)
        .min_depth(0)
        .into_iter()
        .map(|e| e.unwrap().into_path())
        .filter(|p| p.is_file() && p.ends_with("meta.toml"))
        .map(|meta| meta.parent().unwrap().to_str().unwrap().to_string())
        .map(|string| string.strip_prefix(&prefix).unwrap().to_string())
        .collect()
}

pub fn meta_has_link<P: AsRef<Path>>(path: P, id: String, link: &NodeLink) -> bool {
    let meta = NodeMeta::from_toml(&path.as_ref());
    meta.links.contains(&link.to_toml(id))
}
pub fn meta_has_backlink<P: AsRef<Path>>(path: P, id: String, backlink: &NodeLink) -> bool {
    let meta = NodeMeta::from_toml(&path.as_ref());
    let backlink_id = NodeLink::serialize_backlink_id((id, backlink.timestamp));
    meta.backlinks.contains(&backlink.to_toml(backlink_id))
}
