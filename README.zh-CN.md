# mcp-schema-compat

本地、确定性的 Agent 工具 Schema 兼容性检查器，支持 OpenAI、Gemini 与 OpenAPI 3 profile，并可输出文本、JSON 或 SARIF。发现错误时返回非零退出码，不访问模型或网络。

检查范围包括供应商 envelope 命名、OpenAI strict object 约束，以及递归嵌套的 `oneOf`、`anyOf`、`const` 和 `nullable` 关键字。

```bash
cargo run -- tool.json --profile openai --output text
```

English: [README.md](README.md)
如果它帮助你节省了调试时间，欢迎在 [Ko-fi](https://ko-fi.com/tinkora) 支持 Tinkora。
