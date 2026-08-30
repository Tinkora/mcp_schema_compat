# Product specification

## Problem

Agent tool schemas often pass local type checks but fail provider validation
because names, descriptions, parameter envelopes, or object constraints differ.
The CLI provides fast, reproducible preflight feedback before an MCP or tool call
is published.

## Scope

- Read one JSON tool definition.
- For context budgeting, accept one tool, a tool array, or an MCP `tools/list` result.
- For annotation checks, accept the same tool and `tools/list` input shapes.
- Check common required fields and provider-specific envelope rules.
- Emit stable rule IDs in text, JSON, or SARIF.
- Recursively flag provider-incompatible schema keywords and non-strict OpenAI objects.
- Report compact-JSON UTF-8 bytes per tool, description, schema, and combined input.
- Provide a named conservative token upper bound while explicitly distinguishing
  it from provider tokenizers.
- Enforce configurable per-tool, total, description, and schema byte limits with
  stable context rule IDs.
- Never call a model or send input over the network.
- Under the opt-in `explicit_safety_hints_v1` policy, require explicit boolean
  `readOnlyHint`, `destructiveHint`, and `openWorldHint` annotations, validate
  optional annotation field types, and reject the read-only/destructive
  contradiction.

## Non-goals

This is not a schema translator, remote validator, provider tokenizer, billing
estimator, runtime profiler, or replacement for provider SDKs. Context budget
analysis does not execute tools or prove that a payload fits a provider-specific
context window.

Name collision analysis does not automatically rename, truncate, hash, or pick
a winner, and does not treat equal raw names from different servers as an MCP
protocol error.

Annotation analysis does not infer side effects, idempotency, external access,
or destructive behavior from names, descriptions, or schemas. It validates the
declarations' completeness and structural consistency, not their truthfulness.

## Context budget contract

- Estimator: `utf8_bytes_upper_bound_v1`.
- Estimate: one token per UTF-8 byte of compact JSON, used as a portable
  upper-bound planning signal rather than an exact count.
- Inputs: a single tool object, an array of tool objects, or an object whose
  `tools` member is an array.
- Policy: `explicit_safety_hints_v1`, a deployment-readiness baseline rather
  than an MCP protocol-validity requirement.
- Rule IDs: `CONTEXT_TOOL_BUDGET`, `CONTEXT_TOTAL_BUDGET`,
  `CONTEXT_DESCRIPTION_LENGTH`, and `CONTEXT_SCHEMA_LENGTH`.
- Exit status: 1 when any configured context limit is exceeded; 0 otherwise.

## Aggregated name inventory contract

- Inputs: records with explicit `origin_id`, `server_id`, `tool_name`, and final
  `server_tool` fields.
- Rules: `NAME001_DUPLICATE_RAW_TOOL_NAME` (same server only),
  `NAME002_DUPLICATE_SERVER_TOOL`, `NAME003_NORMALIZED_SERVER_TOOL_COLLISION`,
  `NAME004_SERVER_TOOL_OVER_LIMIT`, and `NAME005_RAW_TOOL_NAME_ADVISORY`.
- Policies: normalization and length limits are opt-in; the initial length
  policy measures ASCII names in bytes and leaves non-ASCII names unmeasured.
- Behavior: deterministic, read-only, offline, and never mutates inventory.

## Tool annotation contract

- Inputs: a single tool object, an array of tool objects, or an object whose
  `tools` member is an array.
- Required explicit boolean hints: `readOnlyHint`, `destructiveHint`, and
  `openWorldHint`.
- Optional checked fields: boolean `idempotentHint` and string `title`.
- Rules: `ANNOTATION001_MISSING_ANNOTATIONS`,
  `ANNOTATION002_MISSING_REQUIRED_HINT`, `ANNOTATION003_INVALID_FIELD_TYPE`,
  and `ANNOTATION004_READ_ONLY_DESTRUCTIVE`.
- Outputs: text, JSON, or SARIF; any finding returns exit status 1.
