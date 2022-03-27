use tempfile;

use codex::node::init_codex_repo;
use rstest::fixture;

pub type TempDir = tempfile::TempDir;

#[fixture]
pub fn tempdir() -> TempDir {
    TempDir::new().unwrap()
}

#[fixture]
pub fn initialdir(tempdir: TempDir) ->TempDir {
    init_codex_repo(Some(tempdir.path().to_str().unwrap()));
    tempdir
}
