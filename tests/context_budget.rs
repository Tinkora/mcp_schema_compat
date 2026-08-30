use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
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

#[test]
fn rejects_an_arbitrary_object_instead_of_reporting_a_false_success() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("not-a-tool.json");
    fs::write(&input, r#"{"foo":1}"#).unwrap();

    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([input.to_str().unwrap(), "--context-budget"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("InvalidInput"));
}

#[test]
fn escapes_untrusted_tool_names_in_text_output() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("tool.json");
    fs::write(
        &input,
        r#"{"name":"bad\nERROR [FAKE]","inputSchema":{"type":"object"}}"#,
    )
    .unwrap();

    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([input.to_str().unwrap(), "--context-budget"])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""bad\nERROR [FAKE]""#))
        .stdout(predicates::str::contains("bad\nERROR [FAKE]:").not());
}

#[test]
fn reports_name_collisions_only_within_server_and_uses_explicit_fields() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("inventory.json");
    fs::write(
        &input,
        r#"{
      "tools": [
        {"origin_id":"a","server_id":"s1","tool_name":"search","server_tool":"search"},
        {"origin_id":"b","server_id":"s1","tool_name":"search","server_tool":"search"},
        {"origin_id":"c","server_id":"s2","tool_name":"search","server_tool":"search"}
      ]
    }"#,
    )
    .unwrap();
    let output = Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--name-collisions",
            "--output",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let ids: Vec<_> = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"NAME001_DUPLICATE_RAW_TOOL_NAME"));
    assert!(ids.contains(&"NAME002_DUPLICATE_SERVER_TOOL"));
    assert_eq!(
        ids.iter()
            .filter(|id| **id == "NAME001_DUPLICATE_RAW_TOOL_NAME")
            .count(),
        1
    );
}

#[test]
fn applies_explicit_normalization_and_ascii_byte_limit_without_mutating_names() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("inventory.json");
    fs::write(&input, r#"[{"origin_id":"a","server_id":"s","tool_name":"Read File","server_tool":"read-file"},{"origin_id":"b","server_id":"s","tool_name":"read_file","server_tool":"read_file"}]"#).unwrap();
    let output = Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--name-collisions",
            "--normalize",
            "ascii_lower_sep",
            "--max-server-tool-bytes",
            "8",
            "--output",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let ids: Vec<_> = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"NAME003_NORMALIZED_SERVER_TOOL_COLLISION"));
    assert!(ids.contains(&"NAME004_SERVER_TOOL_OVER_LIMIT"));
    assert_eq!(report["entries"][0]["server_tool"], "read-file");
}

#[test]
fn rejects_missing_explicit_identity_fields() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("inventory.json");
    fs::write(&input, r#"[{"server_tool":"x"}]"#).unwrap();
    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([input.to_str().unwrap(), "--name-collisions"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("InvalidEntry(0)"));
}

#[test]
fn rejects_empty_identity_fields_and_does_not_apply_ascii_limit_to_unicode() {
    let dir = tempdir().unwrap();
    let invalid = dir.path().join("invalid.json");
    fs::write(
        &invalid,
        r#"[{"origin_id":"","server_id":"s","tool_name":"x","server_tool":"x"}]"#,
    )
    .unwrap();
    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([invalid.to_str().unwrap(), "--name-collisions"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("InvalidEntry(0)"));

    let unicode = dir.path().join("unicode.json");
    fs::write(
        &unicode,
        r#"[{"origin_id":"a","server_id":"s","tool_name":"工具","server_tool":"工具"}]"#,
    )
    .unwrap();
    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([
            unicode.to_str().unwrap(),
            "--name-collisions",
            "--max-server-tool-bytes",
            "1",
        ])
        .assert()
        .success();
}

#[test]
fn does_not_echo_untrusted_names_into_text_diagnostics() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("inventory.json");
    fs::write(
        &input,
        r#"[{"origin_id":"a","server_id":"s","tool_name":"x","server_tool":"bad\nERROR [FAKE]"},{"origin_id":"b","server_id":"s","tool_name":"y","server_tool":"bad\nERROR [FAKE]"}]"#,
    )
    .unwrap();
    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([input.to_str().unwrap(), "--name-collisions"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("ERROR [FAKE]").not());
}

#[test]
fn reports_the_correct_error_when_sarif_is_requested_for_name_collisions() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("inventory.json");
    fs::write(
        &input,
        r#"[{"origin_id":"a","server_id":"s","tool_name":"x","server_tool":"x"}]"#,
    )
    .unwrap();

    Command::cargo_bin("mcp-schema-compat")
        .unwrap()
        .args([
            input.to_str().unwrap(),
            "--name-collisions",
            "--output",
            "sarif",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("NameCollisionSarif"));
}
