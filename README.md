# mcp-schema-compat

Deterministic, local-first compatibility checks for agent tool schemas. Validate a JSON tool definition against OpenAI, Gemini, or OpenAPI 3 conventions and emit text, JSON, or SARIF diagnostics.

```bash
cargo run -- tool.json --profile openai --output text
```

Exit status is non-zero when an error diagnostic is found. No model or network access is used.

中文说明见 [README.zh-CN.md](README.zh-CN.md)。
