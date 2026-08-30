# Product specification

## Problem

Agent tool schemas often pass local type checks but fail provider validation
because names, descriptions, parameter envelopes, or object constraints differ.
The CLI provides fast, reproducible preflight feedback before an MCP or tool call
is published.

## Scope

- Read one JSON tool definition.
- For context budgeting, accept one tool, a tool array, or an MCP `tools/list` result.
- Check common required fields and provider-specific envelope rules.
- Emit stable rule IDs in text, JSON, or SARIF.
- Recursively flag provider-incompatible schema keywords and non-strict OpenAI objects.
- Report compact-JSON UTF-8 bytes per tool, description, schema, and combined input.
- Provide a named conservative token upper bound while explicitly distinguishing
  it from provider tokenizers.
- Enforce configurable per-tool, total, description, and schema byte limits with
  stable context rule IDs.
- Never call a model or send input over the network.

## Non-goals

This is not a schema translator, remote validator, provider tokenizer, billing
estimator, runtime profiler, or replacement for provider SDKs. Context budget
analysis does not execute tools or prove that a payload fits a provider-specific
context window.

## Context budget contract

- Estimator: `utf8_bytes_upper_bound_v1`.
- Estimate: one token per UTF-8 byte of compact JSON, used as a portable
  upper-bound planning signal rather than an exact count.
- Inputs: a single tool object, an array of tool objects, or an object whose
  `tools` member is an array.
- Rule IDs: `CONTEXT_TOOL_BUDGET`, `CONTEXT_TOTAL_BUDGET`,
  `CONTEXT_DESCRIPTION_LENGTH`, and `CONTEXT_SCHEMA_LENGTH`.
- Exit status: 1 when any configured context limit is exceeded; 0 otherwise.
