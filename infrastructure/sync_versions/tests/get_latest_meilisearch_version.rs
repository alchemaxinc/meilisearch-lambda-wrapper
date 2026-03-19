use std::process::Command;

#[test]
fn test_get_latest_meilisearch_version_returns_tag() {
    let output = Command::new(env!("CARGO_BIN_EXE_sync_versions"))
        .arg("get-latest-meilisearch-version")
        .output()
        .expect("failed to execute sync_versions binary");

    assert!(output.status.success(), "process exited with error");

    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");
    let tag = stdout.trim();

    assert!(
        tag.starts_with("v"),
        "expected tag starting with 'v', got: {tag}"
    );
}
