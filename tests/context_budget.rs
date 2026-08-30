use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn reports_each_tool_and_total_for_tools_list_result() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("tools.json");
    fs::write(
        &input,
        r#"{"tools":[{"name":"search","description":"Find records","inputSchema":{"type":"object"}},{"name":"read","description":"Read one record","inputSchema":{"type":"object","properties":{"id":{"type":"string"}}}}]}"#,
    )
    .unwrap();

    let output = Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--context-budget",
            "--output",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["estimator"], "utf8_bytes_upper_bound_v1");
    assert_eq!(report["tools"].as_array().unwrap().len(), 2);
    assert_eq!(report["tools"][0]["name"], "search");
    assert!(report["tools"][0]["utf8_bytes"].as_u64().unwrap() > 0);
    assert_eq!(
        report["tools"][0]["estimated_tokens"],
        report["tools"][0]["utf8_bytes"]
    );
    assert_eq!(report["total_estimated_tokens"], report["total_utf8_bytes"]);
}

#[test]
fn accepts_a_tool_array_and_emits_all_budget_rule_codes() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("tools.json");
    fs::write(
        &input,
        r#"[{"name":"large","description":"long description","inputSchema":{"type":"object","properties":{"query":{"type":"string"}}}}]"#,
    )
    .unwrap();

    let output = Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--context-budget",
            "--output",
            "json",
            "--max-tool-bytes",
            "1",
            "--max-total-bytes",
            "1",
            "--max-description-bytes",
            "1",
            "--max-schema-bytes",
            "1",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let serialized = serde_json::to_string(&report).unwrap();
    for rule_id in [
        "CONTEXT_TOOL_BUDGET",
        "CONTEXT_TOTAL_BUDGET",
        "CONTEXT_DESCRIPTION_LENGTH",
        "CONTEXT_SCHEMA_LENGTH",
    ] {
        assert!(serialized.contains(rule_id), "missing {rule_id}");
    }
}

#[test]
fn preserves_single_tool_input_without_requiring_a_profile() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("tool.json");
    fs::write(
        &input,
        r#"{"name":"search","description":"Find records","parameters":{"type":"object"}}"#,
    )
    .unwrap();

    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([input.to_str().unwrap(), "--context-budget"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Estimator: utf8_bytes_upper_bound_v1",
        ))
        .stdout(predicates::str::contains("search"));
}
