# mcp-schema-compat

Deterministic, local-first compatibility checks for agent tool schemas. Validate a JSON tool definition against OpenAI, Gemini, or OpenAPI 3 conventions and emit text, JSON, or SARIF diagnostics.

Checks include provider envelope naming, strict OpenAI object constraints, and recursively nested keywords such as `oneOf`, `anyOf`, `const`, and `nullable`.

```bash
cargo run -- tool.json --profile openai --output text
```

Exit status is non-zero when an error diagnostic is found. No model or network access is used.

中文说明见 [README.zh-CN.md](README.zh-CN.md)。

If this saves you debugging time, support Tinkora on [Ko-fi](https://ko-fi.com/tinkora).
