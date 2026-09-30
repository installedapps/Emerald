use super::*;

#[test]
fn lists_only_regular_supported_files() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = Workspace::open(dir.path()).unwrap();
    fs::write(dir.path().join("b.asc"), "").unwrap();
    fs::write(dir.path().join("a.adoc"), "").unwrap();
    fs::write(dir.path().join("ignore.txt"), "").unwrap();
    let files = workspace.list_asciidoc_files().unwrap();
    assert_eq!(files.len(), 2);
    assert!(files[0].ends_with("a.adoc"));
}

#[test]
fn rejects_path_components_when_creating_files() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = Workspace::open(dir.path()).unwrap();
    assert!(workspace.create_file("../outside").is_err());
    assert!(!dir.path().join("../outside.adoc").exists());
}

#[cfg(unix)]
#[test]
fn excludes_symlinks_from_startup_files() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    symlink(outside.path(), dir.path().join("linked.adoc")).unwrap();
    let workspace = Workspace::open(dir.path()).unwrap();
    assert!(!workspace
        .list_asciidoc_files()
        .unwrap()
        .iter()
        .any(|path| path.ends_with("linked.adoc")));
}
