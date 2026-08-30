use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn reports_missing_invalid_and_contradictory_annotations() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("tools.json");
    fs::write(
        &input,
        r#"{"tools":[
          {"name":"missing","inputSchema":{"type":"object"}},
          {"name":"invalid","inputSchema":{"type":"object"},"annotations":{"readOnlyHint":"yes","openWorldHint":false}},
          {"name":"contradictory","inputSchema":{"type":"object"},"annotations":{"readOnlyHint":true,"destructiveHint":true,"openWorldHint":false}}
        ]}"#,
    )
    .unwrap();

    let output = Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--tool-annotations",
            "--output",
            "json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["policy"], "explicit_safety_hints_v1");
    let serialized = serde_json::to_string(&report).unwrap();
    for rule_id in [
        "ANNOTATION001_MISSING_ANNOTATIONS",
        "ANNOTATION002_MISSING_REQUIRED_HINT",
        "ANNOTATION003_INVALID_FIELD_TYPE",
        "ANNOTATION004_READ_ONLY_DESTRUCTIVE",
    ] {
        assert!(serialized.contains(rule_id), "missing {rule_id}");
    }
}

#[test]
fn accepts_explicit_consistent_required_hints() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("tool.json");
    fs::write(
        &input,
        r#"{"name":"search","inputSchema":{"type":"object"},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":true,"idempotentHint":true,"title":"Search"}}"#,
    )
    .unwrap();

    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([input.to_str().unwrap(), "--tool-annotations"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Tool annotations are complete"));
}

#[test]
fn emits_annotation_findings_as_sarif() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("tool.json");
    fs::write(
        &input,
        r#"{"name":"search","inputSchema":{"type":"object"}}"#,
    )
    .unwrap();

    let output = Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--tool-annotations",
            "--output",
            "sarif",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let sarif: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sarif["version"], "2.1.0");
    assert_eq!(
        sarif["runs"][0]["results"][0]["ruleId"],
        "ANNOTATION001_MISSING_ANNOTATIONS"
    );
}

#[test]
fn rejects_non_tool_input_instead_of_reporting_success() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("invalid.json");
    fs::write(&input, r#"{"foo":1}"#).unwrap();

    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([input.to_str().unwrap(), "--tool-annotations"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("ToolAnnotations(InvalidInput)"));
}
