# REX configuration — operator guide

> Role: how-to | Status: active | Audience: operators | Read when: setting up daemon and inference
> Prefer: ## Operator quick start


## Summary

How-to bootstrap daemon, sidecar, Ollama, LiteLLM gateway, and oMLX managed inference.

How-to for daemon bootstrap, brokered HTTP, LiteLLM gateway, and oMLX. **JSON key catalog:** [CONFIGURATION.md](CONFIGURATION.md).

## Operator quick start (daemon + brokered HTTP)

The product path requires a **supervised sidecar** ([MVP_SPEC.md](MVP_SPEC.md)). Bootstrap JSON; **`rex`** ensures the daemon (**R071**):

```bash
rex config init
# Edit $REX_ROOT/config.json — set inference.openai_compat.base_url and model; enable sidecars.list[].enabled
rex config validate
rex
```

For **Anthropic, OpenAI, and local Ollama** via one broker URL, use the [Inference Gateway](#inference-gateway) (`managed` or `external`) or the [LiteLLM operator profile](#operator-profile-litellm-anthropic-and-other-providers) below.

Example HTTP backend (Ollama) in `$REX_ROOT/config.json`:

```json
"inference": {
 "runtime": "http-openai-compat",
 "openai_compat": {
 "base_url": "http://127.0.0.1:11434/v1",
 "model": "llama3.2",
 "native_tools": "auto"
 }
}
```

**`inference.openai_compat.native_tools`** (**R038**): tri-state `auto` \| `true` \| `false`; schema default **`auto`**. Controls whether daemon forwards OpenAI `tools[]` on `BrokerInference` vs interim JSON-in-text. **`auto`** probes Ollama `/api/show` for `tools` capability — [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md). `mock` / `cursor-cli` runtimes always use interim regardless of config.

### `inference.openai_compat` keys

| Key | Default | Purpose |
|-----|---------|---------|
| `base_url` | (none) | OpenAI-compat API root. Required when `runtime` is `http-openai-compat` unless a managed child injects URL — see [Effective base URL](#effective-openai_compatbase_url). |
| `api_key` | (none) | Optional `Authorization: Bearer` when `headers` does not set `Authorization`. |
| `model` | `gpt-4o-mini` | Default model id on chat/completions requests. |
| `timeout_secs` | `120` | Upper bound for a single HTTP completion request. |
| `native_tools` | `auto` | Tri-state native tool forwarding — see above. |
| `headers` | `{}` | Optional extra HTTP headers. Project `.rex/config.json` merges into global headers. |

Example with custom auth header:

```json
"openai_compat": {
 "base_url": "https://my-gateway.example/v1",
 "model": "my-model",
 "headers": {
 "X-Api-Key": "secret-token"
 }
}
```

CI and unit tests use `mock` runtime — see [CI.md](CI.md) and [CONFIGURATION.md — CI and harness](CONFIGURATION.md#ci-and-harness-not-operator-product-config).

## Inference Gateway

**Status:** **implemented** — [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md), [ADR 0019](architecture/decisions/0019-inference-gateway-opt-in-litellm.md). Bootstrap with `rex gateway init`; validate with `rex gateway doctor`.

Opt-in **`inference.gateway.mode: managed`** spawns LiteLLM locally; **`external`** uses an operator-run URL; **`disabled`** leaves gateway off.

Example (`managed`):

```json
{
 "inference": {
 "runtime": "http-openai-compat",
 "gateway": {
 "mode": "managed",
 "port": 4000,
 "ollama": { "enabled": true, "api_base": "http://127.0.0.1:11434" }
 },
 "openai_compat": {
 "model": "claude-sonnet-4-20250514",
 "timeout_secs": 120
 }
 }
}
```

Effective `openai_compat.base_url` becomes `http://127.0.0.1:4000/v1` when managed. Secrets: `$REX_ROOT/gateway/.env` (gitignored). See hub for full field table.

## Inference oMLX

**Status:** **implemented** — [OMLX_INFERENCE.md](OMLX_INFERENCE.md), [ADR 0033](architecture/decisions/0033-omlx-managed-local-inference.md).

Opt-in **`inference.omlx.mode: managed`** spawns oMLX on Apple Silicon; **`external`** uses operator URL; **`disabled`** leaves oMLX off.

Example (`managed`):

```json
{
 "inference": {
 "runtime": "http-openai-compat",
 "omlx": {
 "mode": "managed",
 "port": 8000,
 "model": "qwen2.5-coder-32b"
 },
 "openai_compat": {
 "native_tools": "auto"
 }
 }
}
```

Full key table: [CONFIGURATION.md — inference.omlx](CONFIGURATION.md#json-configuration-keys-implemented) and [OMLX_INFERENCE.md](OMLX_INFERENCE.md).

## Effective `openai_compat.base_url`

| Priority | Condition | Effective URL |
|----------|-----------|---------------|
| 1 | Non-empty `openai_compat.base_url` and managed `allow_url_override` | Configured URL |
| 2 | `inference.omlx.mode: managed` | `http://127.0.0.1:{omlx.port}/v1` |
| 3 | `inference.gateway.mode: managed` | `http://127.0.0.1:{gateway.port}/v1` |
| 4 | Otherwise | Configured `openai_compat.base_url` or broker error |

**Mutual exclusion:** `rex config validate` **fails** if both oMLX and gateway are `managed`.

## Operator profile: LiteLLM (Anthropic and other providers)

Run LiteLLM with provider keys in **LiteLLM’s** config. Rex only needs the OpenAI-compat surface LiteLLM exposes.

```json
{
 "inference": {
 "runtime": "http-openai-compat",
 "openai_compat": {
 "base_url": "http://127.0.0.1:4000/v1",
 "model": "claude-sonnet-4-20250514",
 "timeout_secs": 120
 }
 },
 "sidecars": {
 "active": "agent",
 "required": true,
 "list": [
 { "name": "agent", "binary": "rex-agent", "enabled": true, "socket": "/tmp/rex-sidecar.sock" }
 ]
 }
}
```

Verification:

1. Confirm LiteLLM responds on `{base_url}/chat/completions`.
2. Launch desktop with sidecar; send a chat turn with model `<litellm-model>` ([OPERATOR_UX.md](OPERATOR_UX.md)).
3. On failure, see [ADAPTERS.md](ADAPTERS.md#broker-provider-errors-intent).

## Layered prompts (design accepted)

**Status:** **design accepted** — not shipped. [ADR 0012](architecture/decisions/0012-layered-prompt-assemblies.md). Hub: [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md).

Versioned **system / project prompt assemblies** in the daemon — config section `prompts` when **R015**+ ships. See [CONFIGURATION.md](CONFIGURATION.md) and [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md).

## See also

- [CONFIGURATION.md](CONFIGURATION.md) — JSON key catalog
- [OPERATOR_UX.md](OPERATOR_UX.md)
- [DEPENDENCIES.md](DEPENDENCIES.md)
