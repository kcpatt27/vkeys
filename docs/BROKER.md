# vkeys-broker - Tier 2 OpenRouter broker

**Status:** tracer, 2026-10-28.
**Purpose:** keep the OpenRouter provider key out of every client/agent process.
Clients authenticate to the broker with a separate broker token; only the broker
holds the provider key.

This is a local HTTP proxy, not a general secret server.

## Run

```powershell
$env:OPENROUTER_API_KEY = "<injected by vkeys>"
$env:VKEYS_BROKER_TOKEN = "<separate broker token>"
vkeys-broker `
  --host 127.0.0.1 `
  --port 8787 `
  --upstream https://openrouter.ai/api/v1 `
  --budget-usd 5.00 `
  --price-in 0.14 `
  --price-out 0.28 `
  --audit C:\ProgramData\secrets\broker-audit.jsonl
```

The command is intentionally started through a vkeys profile so the provider
key is injected into this process, never into the client:

```toml
[profile.openrouter-broker]
cmd = ["vkeys-broker", "--host", "127.0.0.1", "--port", "8787",
       "--budget-usd", "5.00", "--price-in", "0.14", "--price-out", "0.28",
       "--audit", "C:\\ProgramData\\secrets\\broker-audit.jsonl"]
cwd = "C:\\Users\\<you>\\projects\\vkeys"
agent-ok = true

[profile.openrouter-broker.keys]
OPENROUTER_API_KEY = "openrouter/broker"
VKEYS_BROKER_TOKEN = "broker/token"
```

`openrouter/broker` and `broker/token` are dedicated names. Do not reuse the
Meridian or eyewright provider keys for the broker.

## Client

Point an OpenAI-compatible client at:

```text
base URL: http://127.0.0.1:8787/v1
API key:  <broker/token value, not the provider key>
```

`POST /v1/chat/completions` is the supported path. `GET /healthz` is public.

## Behaviour

- refuses to start without `OPENROUTER_API_KEY` or `VKEYS_BROKER_TOKEN`
- bearer-token auth on chat completions
- proxies non-streaming JSON to the upstream
- injects `Authorization: Bearer <provider key>` only upstream
- parses `usage.prompt_tokens` / `usage.completion_tokens`
- computes cost from `--price-in` / `--price-out` (USD per 1M tokens)
- refuses with 429 once the in-memory budget is exhausted
- appends an audit line per request: timestamp, status, model, token counts,
  cost; never prompt text, completion text, or keys
- no `vkeys get`, no generic command, no streaming, no tools

## Remote Ravens

The default is `127.0.0.1`. To serve a Pi or another machine, bind to a Tailscale
address and keep the broker off the public internet:

```powershell
vkeys-broker --host 100.x.y.z --port 8787 ...
```

The Pi gets the broker token, never the provider key. Use the tailnet ACL and
the broker token together; do not expose the port publicly.

## Limits

- spend is in memory and resets when the broker restarts
- one provider (OpenRouter) and one upstream
- no streaming; a client that sends `"stream": true` gets 400
- no per-client budgets yet; the token is shared until client identities exist
- no persistent request queue, retries, or model allowlist yet

## Tests

```powershell
uv run python -m unittest discover -s tests -q
```

The broker tests use a local fake upstream. They never read the real vault,
never use a live provider key, and never call OpenRouter.
