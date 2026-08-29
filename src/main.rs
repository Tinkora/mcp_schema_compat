use clap::{Parser, ValueEnum};
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
    input: PathBuf,
    #[arg(short, long, value_enum)]
    profile: Profile,
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
    let ds = check(&value, &cli.profile);
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
        let v: Value = serde_json::json!({"name":"search","description":"Find items","parameters":{"type":"object","properties":{}}});
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
}
