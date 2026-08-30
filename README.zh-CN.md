# mcp-schema-compat

本地、确定性的 Agent 工具 Schema 兼容性检查器，支持 OpenAI、Gemini 与
OpenAPI 3 profile，并可输出文本、JSON 或 SARIF。发现错误时返回非零退出码，
不访问模型或网络。

检查范围包括供应商 envelope 命名、OpenAI strict object 约束，以及递归嵌套的
`oneOf`、`anyOf`、`const` 和 `nullable` 关键字。

```bash
cargo run -- tool.json --profile openai --output text
```

## Context 预算报告

可静态检查单个工具、工具数组或 MCP `tools/list` 结果，不调用模型，也不执行工具：

```bash
cargo run -- tools.json --context-budget --output json
```

报告列出每个工具及其 description、input schema 的紧凑 JSON UTF-8 字节数，
并汇总总量。`utf8_bytes_upper_bound_v1` 估算器有意采用保守上界：估算 token
数等于 UTF-8 字节数。它只用于容量规划，不是精确 tokenizer 结果，也不代表
任何服务商的计费或上下文统计方式。

默认阈值为：单工具 32 KiB、总量 256 KiB、单 description 8 KiB、单 schema
24 KiB。可通过 `--max-tool-bytes`、`--max-total-bytes`、
`--max-description-bytes` 和 `--max-schema-bytes` 调整。超过阈值时输出稳定
规则 ID，并返回退出码 1。

English: [README.md](README.md)
如果它帮助你节省了调试时间，欢迎在 [Ko-fi](https://ko-fi.com/tinkora) 支持 Tinkora。
