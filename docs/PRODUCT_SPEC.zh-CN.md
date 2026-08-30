# 产品规格

## 问题

Agent 工具 Schema 可能通过本地类型检查，却因名称、描述、参数封装或对象约束差异被服务商拒绝。本工具在发布 MCP 或工具调用前提供快速、可复现的预检。

## 范围

- 读取一个 JSON 工具定义；
- Context 预算模式支持单工具、工具数组或 MCP `tools/list` 结果；
- Annotation 检查支持相同的单工具、工具数组与 MCP `tools/list` 输入形式；
- 检查通用必填字段和服务商特定封装；
- 输出稳定规则 ID，支持文本、JSON、SARIF；
- 递归检查供应商不兼容关键字和非 strict 的 OpenAI object；
- 报告每个工具、description、schema 及整体输入的紧凑 JSON UTF-8 字节数；
- 提供有名称的保守 token 上界，并明确区分于服务商 tokenizer；
- 通过稳定规则 ID 检查可配置的单工具、总量、description 和 schema 字节阈值；
- 不调用模型，不发送网络请求。
- 在选择启用的 `explicit_safety_hints_v1` 策略下，要求显式声明 boolean 类型的
  `readOnlyHint`、`destructiveHint` 与 `openWorldHint`，检查可选 annotation
  字段类型，并按该自定义策略 normalization 处理只读与破坏性同时为真的组合。

## 非目标

不做 Schema 转换、远程验证、服务商 tokenizer、计费估算或运行时分析，也不替代
服务商 SDK。Context 预算分析不会执行工具，也不能证明 payload 一定适合某个
服务商的上下文窗口。

名称冲突分析不会自动重命名、截断、散列名称或选择胜者，也不会把来自不同
服务器的相同原始名称判定为 MCP 协议错误。

Annotation 分析不会根据名称、描述或 schema 推断副作用、幂等性、外部访问或
破坏性。它只验证声明是否完整且结构一致，不能证明声明真实反映实现。

## Context 预算契约

- 估算器：`utf8_bytes_upper_bound_v1`；
- 估算方式：紧凑 JSON 的每个 UTF-8 字节按一个 token 计，作为跨服务商的容量规划上界，而非精确 token 数；
- 输入：单个工具对象、工具对象数组，或 `tools` 成员为数组的对象；
- 规则 ID：`CONTEXT_TOOL_BUDGET`、`CONTEXT_TOTAL_BUDGET`、`CONTEXT_DESCRIPTION_LENGTH`、`CONTEXT_SCHEMA_LENGTH`；
- 退出码：任一配置阈值被超过时为 1，否则为 0。

## 聚合名称清单契约

- 输入：明确包含 `origin_id`、`server_id`、`tool_name` 与最终
  `server_tool` 的记录；
- 规则：`NAME001_DUPLICATE_RAW_TOOL_NAME`（仅同服务器）、
  `NAME002_DUPLICATE_SERVER_TOOL`、`NAME003_NORMALIZED_SERVER_TOOL_COLLISION`、
  `NAME004_SERVER_TOOL_OVER_LIMIT`、`NAME005_RAW_TOOL_NAME_ADVISORY`；
- 策略：规范化与长度限制均需显式启用；首版长度策略以字节衡量 ASCII 名称，
  不衡量非 ASCII 名称；
- 行为：确定、只读、离线，不修改清单。

## 工具 annotation 契约

- 输入：单个工具对象、工具对象数组，或 `tools` 成员为数组的对象；
- 身份验证：每项必须包含非空 string `name` 与 object `inputSchema`；任一非法项
  都会令完整分析失败；
- 策略：`explicit_safety_hints_v1`，它是部署就绪基线，不是 MCP 协议有效性要求；
- 必须显式声明的 boolean hint：`readOnlyHint`、`destructiveHint` 和
  `openWorldHint`；
- 检查的可选字段：boolean 类型的 `idempotentHint` 与 string 类型的 `title`；
- 自定义 normalization：`readOnlyHint` 为 true 时，该策略要求
  `destructiveHint` 为 false；该诊断并不声称 MCP 协议无效；
- 规则：`ANNOTATION001_MISSING_ANNOTATIONS`、
  `ANNOTATION002_MISSING_REQUIRED_HINT`、`ANNOTATION003_INVALID_FIELD_TYPE`、
  `ANNOTATION004_READ_ONLY_DESTRUCTIVE`；
- 输出：文本、JSON 或 SARIF；存在任一诊断时返回退出码 1。
