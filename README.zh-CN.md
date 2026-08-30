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

## MCP 工具 annotation 报告

可检查单个工具、工具数组或 MCP `tools/list` 结果是否显式声明安全 annotation：

```bash
cargo run -- tools.json --tool-annotations --output sarif
```

选择启用的 `explicit_safety_hints_v1` 策略要求 `readOnlyHint`、
`destructiveHint` 和 `openWorldHint` 显式声明为 boolean，同时检查可选
`idempotentHint` 与 `title` 的类型，并把 `readOnlyHint=true,
destructiveHint=true` 作为自定义策略 normalization 报告：该策略要求只读工具的
`destructiveHint=false`。稳定的 `ANNOTATION001` 到 `ANNOTATION004` 诊断支持
文本、JSON 和 SARIF。这是部署就绪策略，并不声称 MCP 协议要求声明所有可选
annotation，也不把该组合称为协议无效。

该模式只做结构检查，不推断工具行为，也不能证明声明与实际实现相符；每个值仍需
根据实现人工审阅。
每个输入项必须包含非空 string `name` 与 object `inputSchema`；非法 collection
项会令分析失败，而不是被静默跳过。

## MCP 聚合名称冲突报告

当宿主聚合多个 MCP 服务器时，可检查调用方提供的最终名称：

```bash
cargo run -- inventory.json --name-collisions --output json
```

输入是数组或包含 `tools` 数组的对象；每项必须明确提供 `origin_id`、
`server_id`、原始 `tool_name` 和最终 `server_tool`。这是显式部署策略检查，
并不声称 MCP 原始名称必须全局唯一。工具不会重命名、截断、散列名称或选择
胜者。稳定的 `NAME001` 到 `NAME005` 规则覆盖同服务器原始名称重复、最终名称
重复、显式规范化冲突、显式长度限制及原始名称可疑空白。

默认不规范化。`--normalize ascii_lower_sep` 只小写 ASCII 字母，并把每个非
字母数字字符映射为 `-`。`--max-server-tool-bytes N` 以字节衡量 ASCII 最终
名称；首版策略刻意不衡量非 ASCII 名称，避免暗中假设 Unicode 长度单位。

English: [README.md](README.md)
如果它帮助你节省了调试时间，欢迎在 [Ko-fi](https://ko-fi.com/tinkora) 支持 Tinkora。
