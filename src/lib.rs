use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

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
