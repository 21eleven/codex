use tempfile;

use rstest::fixture;

pub type TempDir = tempfile::TempDir;

#[fixture]
pub fn tempdir() -> TempDir {
    TempDir::new().unwrap()
}
