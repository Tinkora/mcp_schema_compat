use clap::{Parser, ValueEnum};
use mcp_schema_compat::{
    AnnotationReport, BudgetLimits, ContextBudgetReport, NameCollisionPolicy, NameNormalization,
    analyze_context_budget, analyze_name_collisions, analyze_tool_annotations,
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use thiserror::Error;

#[derive(Parser)]
#[command(
    name = "mcp-schema-compat",
    version,
    about = "Check agent tool schemas against provider profiles"
)]
struct Cli {
    /// JSON tool definition, tool array, or tools/list result.
    input: PathBuf,
    /// Provider compatibility profile.
    #[arg(short, long, value_enum, required_unless_present_any = ["context_budget", "name_collisions", "tool_annotations"])]
    profile: Option<Profile>,
    /// Report static context size without running any tools.
    #[arg(long, conflicts_with = "profile")]
    context_budget: bool,
    /// Check an explicitly supplied multi-server tool inventory for name collisions.
    #[arg(long, conflicts_with = "profile", conflicts_with = "context_budget")]
    name_collisions: bool,
    /// Check explicit MCP tool annotations without executing any tools.
    #[arg(long, conflicts_with_all = ["profile", "context_budget", "name_collisions"])]
    tool_annotations: bool,
    #[arg(long, requires = "name_collisions", default_value = "none")]
    normalize: String,
    #[arg(long, requires = "name_collisions")]
    max_server_tool_bytes: Option<usize>,
    /// Maximum compact JSON bytes allowed for one tool.
    #[arg(long, requires = "context_budget", default_value_t = 32 * 1024)]
    max_tool_bytes: usize,
    /// Maximum combined compact JSON bytes allowed for all tools.
    #[arg(long, requires = "context_budget", default_value_t = 256 * 1024)]
    max_total_bytes: usize,
    /// Maximum UTF-8 bytes allowed for one description.
    #[arg(long, requires = "context_budget", default_value_t = 8 * 1024)]
    max_description_bytes: usize,
    /// Maximum compact JSON bytes allowed for one input schema.
    #[arg(long, requires = "context_budget", default_value_t = 24 * 1024)]
    max_schema_bytes: usize,
    /// Report format. Context budgets support text and JSON.
    #[arg(short, long, value_enum, default_value_t = Output::Text)]
    output: Output,
}
#[derive(Clone, ValueEnum)]
enum Profile {
    Openai,
    Gemini,
    Openapi3,
}
#[derive(Clone, ValueEnum)]
enum Output {
    Text,
    Json,
    Sarif,
}
#[derive(Debug, Error)]
enum Error {
    #[error("read input: {0}")]
    Read(#[from] std::io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("context budget analysis: {0}")]
    ContextBudget(#[from] mcp_schema_compat::ContextBudgetError),
    #[error("SARIF output is not available for context budget reports; use text or JSON")]
    ContextBudgetSarif,
    #[error("name collision analysis: {0}")]
    NameCollisions(#[from] mcp_schema_compat::NameCollisionError),
    #[error("SARIF output is not available for name collision reports; use text or JSON")]
    NameCollisionSarif,
    #[error("tool annotation analysis: {0}")]
    ToolAnnotations(#[from] mcp_schema_compat::AnnotationError),
}
#[derive(serde::Serialize)]
struct Diagnostic {
    rule_id: String,
    level: String,
    message: String,
    path: String,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    let value: Value = serde_json::from_str(&fs::read_to_string(cli.input)?)?;
    if cli.tool_annotations {
        let report = analyze_tool_annotations(&value)?;
        print_annotation_report(&report, &cli.output)?;
        if report.has_errors() {
            std::process::exit(1);
        }
        return Ok(());
    }
    if cli.context_budget {
        if matches!(cli.output, Output::Sarif) {
            return Err(Error::ContextBudgetSarif);
        }
        let report = analyze_context_budget(
            &value,
            BudgetLimits {
                max_tool_bytes: cli.max_tool_bytes,
                max_total_bytes: cli.max_total_bytes,
                max_description_bytes: cli.max_description_bytes,
                max_schema_bytes: cli.max_schema_bytes,
            },
        )?;
        print_context_budget(&report, &cli.output)?;
        if report.has_errors() {
            std::process::exit(1);
        }
        return Ok(());
    }
    if cli.name_collisions {
        if matches!(cli.output, Output::Sarif) {
            return Err(Error::NameCollisionSarif);
        }
        let normalization = match cli.normalize.as_str() {
            "none" => NameNormalization::None,
            "ascii_lower_sep" => NameNormalization::AsciiLowerSep,
            other => {
                return Err(Error::NameCollisions(
                    mcp_schema_compat::NameCollisionError::UnsupportedNormalization(
                        other.to_owned(),
                    ),
                ));
            }
        };
        let report = analyze_name_collisions(
            &value,
            NameCollisionPolicy {
                normalization,
                max_server_tool_bytes: cli.max_server_tool_bytes,
            },
        )?;
        match cli.output {
            Output::Text => {
                for d in &report.diagnostics {
                    println!(
                        "{} [{}] {}: {}",
                        d.level.to_uppercase(),
                        d.rule_id,
                        d.path,
                        d.message
                    );
                }
                if report.diagnostics.is_empty() {
                    println!("No name collisions");
                }
            }
            Output::Json => println!("{}", serde_json::to_string_pretty(&report)?),
            Output::Sarif => unreachable!(),
        }
        if report.diagnostics.iter().any(|d| d.level == "error") {
            std::process::exit(1);
        }
        return Ok(());
    }

    let ds = check(
        &value,
        cli.profile.as_ref().expect("profile is required by clap"),
    );
    match cli.output {
        Output::Text => {
            for d in &ds {
                println!(
                    "{} [{}] {}: {}",
                    d.level.to_uppercase(),
                    d.rule_id,
                    d.path,
                    d.message
                );
            }
            if ds.is_empty() {
                println!("Compatible");
            }
        }
        Output::Json => println!("{}", serde_json::to_string_pretty(&ds)?),
        Output::Sarif => println!("{}", sarif(&ds)),
    }
    if ds.iter().any(|d| d.level == "error") {
        std::process::exit(1);
    }
    Ok(())
}

fn print_annotation_report(report: &AnnotationReport, output: &Output) -> Result<(), Error> {
    match output {
        Output::Text => {
            for diagnostic in &report.diagnostics {
                println!(
                    "{} [{}] {}: {}",
                    diagnostic.level.to_uppercase(),
                    diagnostic.rule_id,
                    diagnostic.path,
                    diagnostic.message
                );
            }
            if report.diagnostics.is_empty() {
                println!("Tool annotations are complete and structurally consistent");
            }
        }
        Output::Json => println!("{}", serde_json::to_string_pretty(report)?),
        Output::Sarif => println!(
            "{}",
            json!({"version":"2.1.0","runs":[{"tool":{"driver":{"name":"mcp-schema-compat","version":env!("CARGO_PKG_VERSION")}},"results":report.diagnostics.iter().map(|d| json!({"ruleId":d.rule_id,"level":d.level,"message":{"text":d.message},"locations":[{"logicalLocations":[{"fullyQualifiedName":d.path}]}]})).collect::<Vec<_>>() }]})
        ),
    }
    Ok(())
}

fn print_context_budget(report: &ContextBudgetReport, output: &Output) -> Result<(), Error> {
    match output {
        Output::Text => {
            println!("Estimator: {}", report.estimator);
            println!("Note: {}", report.estimator_note);
            for tool in &report.tools {
                println!(
                    "{}: {} UTF-8 bytes, <= {} estimated tokens (description {}, schema {})",
                    serde_json::to_string(&tool.name).expect("string serialization cannot fail"),
                    tool.utf8_bytes,
                    tool.estimated_tokens,
                    tool.description_bytes,
                    tool.schema_bytes
                );
                for diagnostic in &tool.diagnostics {
                    println!(
                        "{} [{}] {}: {}",
                        diagnostic.level.to_uppercase(),
                        diagnostic.rule_id,
                        diagnostic.path,
                        diagnostic.message
                    );
                }
            }
            println!(
                "Total: {} UTF-8 bytes, <= {} estimated tokens",
                report.total_utf8_bytes, report.total_estimated_tokens
            );
            for diagnostic in &report.diagnostics {
                println!(
                    "{} [{}] {}: {}",
                    diagnostic.level.to_uppercase(),
                    diagnostic.rule_id,
                    diagnostic.path,
                    diagnostic.message
                );
            }
        }
        Output::Json => println!("{}", serde_json::to_string_pretty(report)?),
        Output::Sarif => return Err(Error::ContextBudgetSarif),
    }
    Ok(())
}

fn check(v: &Value, p: &Profile) -> Vec<Diagnostic> {
    let mut d = Vec::new();
    let obj = match v.as_object() {
        Some(x) => x,
        None => {
            d.push(diag(
                "SCHEMA001",
                "error",
                "schema must be a JSON object",
                "$",
            ));
            return d;
        }
    };
    if !obj.contains_key("name") {
        d.push(diag(
            "SCHEMA002",
            "error",
            "tool name is required",
            "$.name",
        ));
    }
    if !obj.get("description").is_some_and(Value::is_string) {
        d.push(diag(
            "SCHEMA003",
            "error",
            "tool description must be a string",
            "$.description",
        ));
    }
    let params = obj.get("parameters").or_else(|| obj.get("inputSchema"));
    if params.is_none_or(|x| x.get("type").and_then(Value::as_str) != Some("object")) {
        d.push(diag(
            "SCHEMA004",
            "error",
            "parameters/inputSchema must declare type=object",
            "$.parameters",
        ));
    }
    if let Some(x) = params {
        if x.get("additionalProperties") == Some(&Value::Bool(true)) {
            d.push(diag(
                "SCHEMA005",
                "warning",
                "unbounded additionalProperties reduce provider compatibility",
                "$.parameters.additionalProperties",
            ));
        }
        scan_keywords(x, p, "$.parameters", &mut d);
        if matches!(p, Profile::Openai) && x.get("additionalProperties").is_none() {
            d.push(diag(
                "OPENAI002",
                "warning",
                "set additionalProperties=false for strict structured outputs",
                "$.parameters.additionalProperties",
            ));
        }
    }
    match p {
        Profile::Openai => {
            if obj.contains_key("inputSchema") && !obj.contains_key("parameters") {
                d.push(diag(
                    "OPENAI001",
                    "error",
                    "OpenAI tools use parameters, not inputSchema",
                    "$.inputSchema",
                ));
            }
        }
        Profile::Gemini => {
            if obj.contains_key("parameters") && !obj.contains_key("inputSchema") {
                d.push(diag("GEMINI001", "warning", "Gemini uses parametersJsonSchema/inputSchema naming; adapt parameters before sending", "$.parameters"));
            }
        }
        Profile::Openapi3 => {
            if obj.get("type").and_then(Value::as_str) != Some("function") {
                d.push(diag(
                    "OPENAPI001",
                    "error",
                    "OpenAPI operation schemas require an operation type",
                    "$.type",
                ));
            }
        }
    }
    d
}
fn scan_keywords(v: &Value, p: &Profile, path: &str, d: &mut Vec<Diagnostic>) {
    if let Some(obj) = v.as_object() {
        for key in ["oneOf", "anyOf", "const", "nullable"] {
            if obj.contains_key(key) {
                let unsupported = matches!(p, Profile::Openapi3) || key == "const";
                if unsupported {
                    d.push(diag(
                        "SCHEMA006",
                        "error",
                        "keyword is outside the selected provider compatibility subset",
                        &format!("{path}.{key}"),
                    ));
                }
            }
        }
        if let Some(props) = obj.get("properties").and_then(Value::as_object) {
            for (name, child) in props {
                scan_keywords(child, p, &format!("{path}.properties.{name}"), d);
            }
        }
        if let Some(items) = obj.get("items") {
            scan_keywords(items, p, &format!("{path}.items"), d);
        }
    }
}
fn diag(id: &str, level: &str, msg: &str, path: &str) -> Diagnostic {
    Diagnostic {
        rule_id: id.into(),
        level: level.into(),
        message: msg.into(),
        path: path.into(),
    }
}
fn sarif(ds: &[Diagnostic]) -> Value {
    json!({"version":"2.1.0","runs":[{"tool":{"driver":{"name":"mcp-schema-compat","version":env!("CARGO_PKG_VERSION")}},"results":ds.iter().map(|d| json!({"ruleId":d.rule_id,"level":d.level,"message":{"text":d.message},"locations":[{"logicalLocations":[{"fullyQualifiedName":d.path}]}]})).collect::<Vec<_>>() }]})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_openai() {
        let v: Value = serde_json::json!({"name":"search","description":"Find items","parameters":{"type":"object","properties":{},"additionalProperties":false}});
        assert!(check(&v, &Profile::Openai).is_empty());
    }
    #[test]
    fn rejects_missing_name() {
        let v = serde_json::json!({"description":"x","parameters":{"type":"object"}});
        assert!(
            check(&v, &Profile::Openai)
                .iter()
                .any(|d| d.rule_id == "SCHEMA002")
        );
    }
    #[test]
    fn openapi_requires_type() {
        let v = serde_json::json!({"name":"x","description":"x","parameters":{"type":"object"}});
        assert!(
            check(&v, &Profile::Openapi3)
                .iter()
                .any(|d| d.rule_id == "OPENAPI001")
        );
    }
    #[test]
    fn rejects_nested_const_for_openai() {
        let v = serde_json::json!({"name":"x","description":"x","parameters":{"type":"object","properties":{"mode":{"type":"string","const":"fast"}}}});
        assert!(
            check(&v, &Profile::Openai)
                .iter()
                .any(|d| d.rule_id == "SCHEMA006")
        );
    }
    #[test]
    fn warns_when_openai_object_is_not_strict() {
        let v = serde_json::json!({"name":"x","description":"x","parameters":{"type":"object","properties":{}}});
        assert!(
            check(&v, &Profile::Openai)
                .iter()
                .any(|d| d.rule_id == "OPENAI002")
        );
    }
}
