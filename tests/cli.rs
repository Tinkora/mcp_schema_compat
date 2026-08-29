use assert_cmd::Command;
use std::path::Path;
#[test]
fn reports_json_and_nonzero_for_incompatible_fixture() {
    let mut c = Command::cargo_bin("mcp-schema-compat").unwrap();
    c.args([
        "tests/fixtures/openai_input_schema.json",
        "--profile",
        "openai",
        "--output",
        "json",
    ])
    .assert()
    .failure()
    .stdout(predicates::str::contains("OPENAI001"));
}
fn _exists(_: &Path) {}
