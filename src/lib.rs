use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

const REQUIRED_ANNOTATION_HINTS: [&str; 3] = ["readOnlyHint", "destructiveHint", "openWorldHint"];

#[derive(Debug, Error)]
pub enum AnnotationError {
    #[error("input must be one tool object, a tool array, or a tools/list result")]
    InvalidInput,
    #[error("tools/list result must contain an array of tool objects")]
    InvalidToolsList,
}

#[derive(Debug, Serialize)]
pub struct AnnotationDiagnostic {
    pub rule_id: &'static str,
    pub level: &'static str,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct AnnotationReport {
    pub policy: &'static str,
    pub required_hints: [&'static str; 3],
    pub tools_checked: usize,
    pub diagnostics: Vec<AnnotationDiagnostic>,
}

impl AnnotationReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == "error")
    }
}

pub fn analyze_tool_annotations(input: &Value) -> Result<AnnotationReport, AnnotationError> {
    let tools = extract_annotation_tools(input)?;
    let tools_checked = tools.len();
    let mut diagnostics = Vec::new();

    for (tool, path) in tools {
        let Some(annotations) = tool.get("annotations") else {
            diagnostics.push(AnnotationDiagnostic {
                rule_id: "ANNOTATION001_MISSING_ANNOTATIONS",
                level: "error",
                message: "tool must declare an annotations object with explicit required hints"
                    .to_owned(),
                path: format!("{path}.annotations"),
            });
            continue;
        };
        let Some(annotations) = annotations.as_object() else {
            diagnostics.push(AnnotationDiagnostic {
                rule_id: "ANNOTATION003_INVALID_FIELD_TYPE",
                level: "error",
                message: "annotations must be an object".to_owned(),
                path: format!("{path}.annotations"),
            });
            continue;
        };

        for hint in REQUIRED_ANNOTATION_HINTS {
            if !annotations.contains_key(hint) {
                diagnostics.push(AnnotationDiagnostic {
                    rule_id: "ANNOTATION002_MISSING_REQUIRED_HINT",
                    level: "error",
                    message: format!("required annotation hint {hint} must be declared explicitly"),
                    path: format!("{path}.annotations.{hint}"),
                });
            }
        }
        for hint in [
            "readOnlyHint",
            "destructiveHint",
            "openWorldHint",
            "idempotentHint",
        ] {
            if annotations
                .get(hint)
                .is_some_and(|value| !value.is_boolean())
            {
                diagnostics.push(AnnotationDiagnostic {
                    rule_id: "ANNOTATION003_INVALID_FIELD_TYPE",
                    level: "error",
                    message: format!("annotation hint {hint} must be a boolean"),
                    path: format!("{path}.annotations.{hint}"),
                });
            }
        }
        if annotations
            .get("title")
            .is_some_and(|value| !value.is_string())
        {
            diagnostics.push(AnnotationDiagnostic {
                rule_id: "ANNOTATION003_INVALID_FIELD_TYPE",
                level: "error",
                message: "annotation title must be a string".to_owned(),
                path: format!("{path}.annotations.title"),
            });
        }
        if annotations.get("readOnlyHint") == Some(&Value::Bool(true))
            && annotations.get("destructiveHint") == Some(&Value::Bool(true))
        {
            diagnostics.push(AnnotationDiagnostic {
                rule_id: "ANNOTATION004_READ_ONLY_DESTRUCTIVE",
                level: "error",
                message: "a read-only tool cannot also be marked destructive".to_owned(),
                path: format!("{path}.annotations"),
            });
        }
    }

    Ok(AnnotationReport {
        policy: "explicit_safety_hints_v1",
        required_hints: REQUIRED_ANNOTATION_HINTS,
        tools_checked,
        diagnostics,
    })
}

fn extract_annotation_tools(input: &Value) -> Result<Vec<(&Value, String)>, AnnotationError> {
    if let Some(items) = input.as_array() {
        if items.iter().any(|item| !item.is_object()) {
            return Err(AnnotationError::InvalidInput);
        }
        return Ok(items
            .iter()
            .enumerate()
            .map(|(index, tool)| (tool, format!("$[{index}]")))
            .collect());
    }

    let object = input.as_object().ok_or(AnnotationError::InvalidInput)?;
    if let Some(tools) = object.get("tools") {
        let items = tools.as_array().ok_or(AnnotationError::InvalidToolsList)?;
        if items.iter().any(|item| !item.is_object()) {
            return Err(AnnotationError::InvalidToolsList);
        }
        return Ok(items
            .iter()
            .enumerate()
            .map(|(index, tool)| (tool, format!("$.tools[{index}]")))
            .collect());
    }

    let has_name = object.get("name").and_then(Value::as_str).is_some();
    let has_schema = object.get("inputSchema").is_some() || object.get("parameters").is_some();
    if has_name && has_schema {
        Ok(vec![(input, "$".to_owned())])
    } else {
        Err(AnnotationError::InvalidInput)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct NameInventoryEntry {
    pub origin_id: String,
    pub server_id: String,
    pub tool_name: String,
    pub server_tool: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum NameNormalization {
    #[default]
    None,
    AsciiLowerSep,
}

#[derive(Debug, Clone, Copy)]
pub struct NameCollisionPolicy {
    pub normalization: NameNormalization,
    pub max_server_tool_bytes: Option<usize>,
}

#[derive(Debug, Error)]
pub enum NameCollisionError {
    #[error("name collision inventory must be an array or an object containing a tools array")]
    InvalidInventory,
    #[error(
        "inventory entry {0} must contain string origin_id, server_id, tool_name, and server_tool"
    )]
    InvalidEntry(usize),
    #[error("unsupported normalization policy: {0}")]
    UnsupportedNormalization(String),
    #[error("failed to parse inventory: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Serialize)]
pub struct NameCollisionDiagnostic {
    pub rule_id: &'static str,
    pub level: &'static str,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct NameCollisionReport {
    pub normalization: &'static str,
    pub length_unit: &'static str,
    pub entries: Vec<NameInventoryEntry>,
    pub diagnostics: Vec<NameCollisionDiagnostic>,
}

pub fn analyze_name_collisions(
    input: &Value,
    policy: NameCollisionPolicy,
) -> Result<NameCollisionReport, NameCollisionError> {
    let (raw, root) = if let Some(entries) = input.as_array() {
        (entries, "$")
    } else if let Some(entries) = input.get("tools").and_then(Value::as_array) {
        (entries, "$.tools")
    } else {
        return Err(NameCollisionError::InvalidInventory);
    };
    let mut entries = Vec::with_capacity(raw.len());
    for (i, value) in raw.iter().enumerate() {
        let entry: NameInventoryEntry = serde_json::from_value(value.clone())
            .map_err(|_| NameCollisionError::InvalidEntry(i))?;
        if entry.origin_id.is_empty()
            || entry.server_id.is_empty()
            || entry.tool_name.is_empty()
            || entry.server_tool.is_empty()
        {
            return Err(NameCollisionError::InvalidEntry(i));
        }
        entries.push(entry);
    }
    let mut diagnostics = Vec::new();
    let path = |i: usize| format!("{root}[{i}]");
    let mut raw_names = HashMap::new();
    let mut final_names = HashMap::new();
    let mut normalized_names = HashMap::new();
    for i in 0..entries.len() {
        let entry = &entries[i];
        if raw_names
            .insert((entry.server_id.as_str(), entry.tool_name.as_str()), i)
            .is_some()
        {
            diagnostics.push(NameCollisionDiagnostic {
                rule_id: "NAME001_DUPLICATE_RAW_TOOL_NAME",
                level: "error",
                message: "raw tool name is duplicated within this server_id".into(),
                path: path(i),
            });
        }
        if final_names.insert(entry.server_tool.as_str(), i).is_some() {
            diagnostics.push(NameCollisionDiagnostic {
                rule_id: "NAME002_DUPLICATE_SERVER_TOOL",
                level: "error",
                message: "server_tool is duplicated".into(),
                path: path(i),
            });
        }
        let normalized = normalize(&entry.server_tool, policy.normalization);
        if let Some(previous) = normalized_names.insert(normalized.clone(), i) {
            if entries[previous].server_tool != entry.server_tool {
                diagnostics.push(NameCollisionDiagnostic {
                    rule_id: "NAME003_NORMALIZED_SERVER_TOOL_COLLISION",
                    level: "error",
                    message: "normalized server_tool collides".into(),
                    path: path(i),
                });
            }
        }
        if let Some(limit) = policy.max_server_tool_bytes {
            if entries[i].server_tool.is_ascii() && entries[i].server_tool.len() > limit {
                diagnostics.push(NameCollisionDiagnostic {
                    rule_id: "NAME004_SERVER_TOOL_OVER_LIMIT",
                    level: "error",
                    message: format!(
                        "server_tool uses {} ASCII bytes, exceeding {limit}-byte limit",
                        entries[i].server_tool.len()
                    ),
                    path: path(i),
                });
            }
        }
        if entries[i].tool_name.trim() != entries[i].tool_name {
            diagnostics.push(NameCollisionDiagnostic {
                rule_id: "NAME005_RAW_TOOL_NAME_ADVISORY",
                level: "warning",
                message: "raw tool name has leading or trailing whitespace".into(),
                path: format!("{}.tool_name", path(i)),
            });
        }
    }
    Ok(NameCollisionReport {
        normalization: match policy.normalization {
            NameNormalization::None => "none",
            NameNormalization::AsciiLowerSep => "ascii_lower_sep",
        },
        length_unit: "ASCII bytes (non-ASCII names are not measured)",
        entries,
        diagnostics,
    })
}

fn normalize(value: &str, policy: NameNormalization) -> String {
    match policy {
        NameNormalization::None => value.to_owned(),
        NameNormalization::AsciiLowerSep => value
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect(),
    }
}

pub const TOKEN_ESTIMATOR: &str = "utf8_bytes_upper_bound_v1";
pub const TOKEN_ESTIMATOR_NOTE: &str =
    "Upper bound equal to compact JSON UTF-8 bytes; not a provider tokenizer or exact token count.";

#[derive(Clone, Copy, Debug)]
pub struct BudgetLimits {
    pub max_tool_bytes: usize,
    pub max_total_bytes: usize,
    pub max_description_bytes: usize,
    pub max_schema_bytes: usize,
}

impl Default for BudgetLimits {
    fn default() -> Self {
        Self {
            max_tool_bytes: 32 * 1024,
            max_total_bytes: 256 * 1024,
            max_description_bytes: 8 * 1024,
            max_schema_bytes: 24 * 1024,
        }
    }
}

#[derive(Debug, Error)]
pub enum ContextBudgetError {
    #[error("tools/list result must contain a tools array")]
    InvalidToolsList,
    #[error("input must be one tool object, a tool array, or a tools/list result")]
    InvalidInput,
    #[error("failed to serialize tool: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Serialize)]
pub struct BudgetDiagnostic {
    pub rule_id: &'static str,
    pub level: &'static str,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ToolBudget {
    pub name: String,
    pub path: String,
    pub utf8_bytes: usize,
    pub estimated_tokens: usize,
    pub description_bytes: usize,
    pub schema_bytes: usize,
    pub diagnostics: Vec<BudgetDiagnostic>,
}

#[derive(Debug, Serialize)]
pub struct ContextBudgetReport {
    pub estimator: &'static str,
    pub estimator_note: &'static str,
    pub total_utf8_bytes: usize,
    pub total_estimated_tokens: usize,
    pub tools: Vec<ToolBudget>,
    pub diagnostics: Vec<BudgetDiagnostic>,
}

impl ContextBudgetReport {
    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty() || self.tools.iter().any(|tool| !tool.diagnostics.is_empty())
    }
}

pub fn analyze_context_budget(
    input: &Value,
    limits: BudgetLimits,
) -> Result<ContextBudgetReport, ContextBudgetError> {
    let tools = extract_tools(input)?;
    let mut reports = Vec::with_capacity(tools.len());
    let mut total_utf8_bytes = 0;

    for (tool, path) in tools {
        let compact = serde_json::to_vec(tool)?;
        let utf8_bytes = compact.len();
        let description_bytes = tool
            .get("description")
            .and_then(Value::as_str)
            .map_or(0, |description| description.len());
        let (schema_key, schema) = if let Some(schema) = tool.get("inputSchema") {
            ("inputSchema", Some(schema))
        } else {
            ("parameters", tool.get("parameters"))
        };
        let schema_bytes = schema
            .map(serde_json::to_vec)
            .transpose()?
            .map_or(0, |schema| schema.len());
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("<unnamed>")
            .to_owned();
        let mut diagnostics = Vec::new();

        push_if_exceeded(
            &mut diagnostics,
            "CONTEXT_TOOL_BUDGET",
            utf8_bytes,
            limits.max_tool_bytes,
            &path,
            "tool definition",
        );
        push_if_exceeded(
            &mut diagnostics,
            "CONTEXT_DESCRIPTION_LENGTH",
            description_bytes,
            limits.max_description_bytes,
            &format!("{path}.description"),
            "description",
        );
        push_if_exceeded(
            &mut diagnostics,
            "CONTEXT_SCHEMA_LENGTH",
            schema_bytes,
            limits.max_schema_bytes,
            &format!("{path}.{schema_key}"),
            "input schema",
        );

        total_utf8_bytes += utf8_bytes;
        reports.push(ToolBudget {
            name,
            path,
            utf8_bytes,
            estimated_tokens: utf8_bytes,
            description_bytes,
            schema_bytes,
            diagnostics,
        });
    }

    let mut diagnostics = Vec::new();
    push_if_exceeded(
        &mut diagnostics,
        "CONTEXT_TOTAL_BUDGET",
        total_utf8_bytes,
        limits.max_total_bytes,
        "$",
        "combined tool definitions",
    );

    Ok(ContextBudgetReport {
        estimator: TOKEN_ESTIMATOR,
        estimator_note: TOKEN_ESTIMATOR_NOTE,
        total_utf8_bytes,
        total_estimated_tokens: total_utf8_bytes,
        tools: reports,
        diagnostics,
    })
}

fn extract_tools(input: &Value) -> Result<Vec<(&Value, String)>, ContextBudgetError> {
    if let Some(items) = input.as_array() {
        if items.iter().any(|item| !item.is_object()) {
            return Err(ContextBudgetError::InvalidInput);
        }
        return Ok(items
            .iter()
            .enumerate()
            .map(|(index, tool)| (tool, format!("$[{index}]")))
            .collect());
    }

    let object = input.as_object().ok_or(ContextBudgetError::InvalidInput)?;
    if let Some(tools) = object.get("tools") {
        let items = tools
            .as_array()
            .ok_or(ContextBudgetError::InvalidToolsList)?;
        if items.iter().any(|item| !item.is_object()) {
            return Err(ContextBudgetError::InvalidToolsList);
        }
        return Ok(items
            .iter()
            .enumerate()
            .map(|(index, tool)| (tool, format!("$.tools[{index}]")))
            .collect());
    }

    let has_name = object.get("name").and_then(Value::as_str).is_some();
    let has_schema = object.get("inputSchema").is_some() || object.get("parameters").is_some();
    if has_name && has_schema {
        Ok(vec![(input, "$".to_owned())])
    } else {
        Err(ContextBudgetError::InvalidInput)
    }
}

fn push_if_exceeded(
    diagnostics: &mut Vec<BudgetDiagnostic>,
    rule_id: &'static str,
    actual: usize,
    limit: usize,
    path: &str,
    subject: &str,
) {
    if actual > limit {
        diagnostics.push(BudgetDiagnostic {
            rule_id,
            level: "error",
            message: format!(
                "{subject} uses {actual} UTF-8 bytes, exceeding the {limit}-byte limit"
            ),
            path: path.to_owned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_non_array_tools_list_member() {
        let error =
            analyze_context_budget(&json!({"tools": {}}), BudgetLimits::default()).unwrap_err();
        assert!(matches!(error, ContextBudgetError::InvalidToolsList));
    }

    #[test]
    fn measures_multibyte_text_as_utf8_bytes() {
        let report = analyze_context_budget(
            &json!({"name":"x","description":"工具","inputSchema":{"type":"object"}}),
            BudgetLimits::default(),
        )
        .unwrap();
        assert_eq!(report.tools[0].description_bytes, 6);
    }
}
