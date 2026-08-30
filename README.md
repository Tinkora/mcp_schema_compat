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

## MCP tool annotation report

Check one tool, a tool array, or an MCP `tools/list` result for explicit safety
annotations:

```bash
cargo run -- tools.json --tool-annotations --output sarif
```

The opt-in `explicit_safety_hints_v1` policy requires `readOnlyHint`,
`destructiveHint`, and `openWorldHint` to be declared as booleans. It also
checks the optional `idempotentHint` and `title` types, and rejects the
deterministic contradiction where a tool is both read-only and destructive.
Stable `ANNOTATION001` through `ANNOTATION004` diagnostics are available in
text, JSON, and SARIF. This is a deployment-readiness policy, not a claim that
the MCP protocol requires every optional annotation.

This structural check does not infer a tool's behavior or prove that declared
annotations are truthful. Review each value against the implementation.

## Aggregated MCP name collision report

Check caller-supplied final names when a host aggregates multiple MCP servers:

```bash
cargo run -- inventory.json --name-collisions --output json
```

The input is an array, or an object with a `tools` array. Every entry must
provide `origin_id`, `server_id`, raw `tool_name`, and final `server_tool`.
This is an explicit deployment-policy check, not a claim that raw MCP names
must be globally unique. It never renames, truncates, hashes, or chooses a
winner. Stable `NAME001` through `NAME005` diagnostics cover same-server raw
duplicates, final-name duplicates, explicit normalization collisions, explicit
length limits, and suspicious raw-name whitespace.

Normalization is off by default. `--normalize ascii_lower_sep` lowercases ASCII
letters and maps each non-alphanumeric character to `-`.
`--max-server-tool-bytes N` measures ASCII final names in bytes; non-ASCII names
are deliberately not measured by this initial policy, avoiding an unstated
Unicode length assumption.

中文说明见 [README.zh-CN.md](README.zh-CN.md)。

If this saves you debugging time, support Tinkora on [Ko-fi](https://ko-fi.com/tinkora).
