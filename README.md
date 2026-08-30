# mcp-schema-compat

Deterministic, local-first compatibility checks for agent tool schemas. Validate
a JSON tool definition against OpenAI, Gemini, or OpenAPI 3 conventions and emit
text, JSON, or SARIF diagnostics.

Checks include provider envelope naming, strict OpenAI object constraints, and
recursively nested keywords such as `oneOf`, `anyOf`, `const`, and `nullable`.

```bash
cargo run -- tool.json --profile openai --output text
```

## Context budget report

Inspect the static context cost of one tool, a tool array, or an MCP `tools/list`
result without calling or executing anything:

```bash
cargo run -- tools.json --context-budget --output json
```

The report includes compact-JSON UTF-8 bytes for every tool and its description
and input schema, plus combined totals. It uses the deliberately conservative
`utf8_bytes_upper_bound_v1` estimator: estimated tokens equal UTF-8 bytes. This
is an upper-bound planning signal, not an exact tokenizer result and not a claim
about any provider's billing or context accounting.

Default limits are 32 KiB per tool, 256 KiB total, 8 KiB per description, and
24 KiB per schema. Override them with `--max-tool-bytes`, `--max-total-bytes`,
`--max-description-bytes`, and `--max-schema-bytes`. Exceeded limits emit stable
rule IDs and return exit status 1.

Exit status is non-zero when an error diagnostic is found. No model or network
access is used.

中文说明见 [README.zh-CN.md](README.zh-CN.md)。

If this saves you debugging time, support Tinkora on [Ko-fi](https://ko-fi.com/tinkora).
