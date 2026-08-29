# Product specification

## Problem

Agent tool schemas often pass local type checks but fail provider validation because names, descriptions, parameter envelopes, or object constraints differ. The CLI provides fast, reproducible preflight feedback before an MCP or tool call is published.

## Scope

- Read one JSON tool definition.
- Check common required fields and provider-specific envelope rules.
- Emit stable rule IDs in text, JSON, or SARIF.
- Recursively flag provider-incompatible schema keywords and non-strict OpenAI objects.
- Never call a model or send input over the network.

## Non-goals

This is not a schema translator, remote validator, or replacement for provider SDKs.
