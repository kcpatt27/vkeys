# vkeys-broker - Tier 2 OpenRouter broker

**Status:** Tier 2, Rust default since 2026-10-28 (Python oracle retained).
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
- rate limits each client (peer IP + presented token) with 429; default 60
  requests per 60 seconds
- atomically reserves the configured per-request ceiling before calling upstream;
  refuses with 429 once the in-memory budget is exhausted
- parses the request JSON before forwarding: a `max_tokens` above
  `VKEYS_BROKER_MAX_REQUEST_TOKENS` is refused with 400, a zero `max_tokens` is
  refused with 400, and a missing `max_tokens` is set to that ceiling
- proxies non-streaming JSON to the upstream
- injects `Authorization: Bearer <provider key>` only upstream
- parses `usage.prompt_tokens` / `usage.completion_tokens`
- computes cost from `--price-in` / `--price-out` (USD per 1M tokens)
- caps the ledger at the reservation: if the upstream reports more than the
  reserved worst case, the ledger is charged only the reservation and the
  audit line carries `overage_usd` (actual - reserved)
- appends an audit line per request: timestamp, status, model, token counts,
  cost; refusals (401, 400, 429) also carry a reason. Never prompt text,
  completion text, or keys
- no `vkeys get`, no generic command, no streaming, no tools

## Environment

| Variable | Default | Meaning |
|---|---|---|
| `VKEYS_BROKER_TOKEN` | required | current broker token |
| `VKEYS_BROKER_TOKEN_PREVIOUS` | unset | old token still accepted during rotation |
| `VKEYS_BROKER_RATE_LIMIT` | `60` | requests per client per window; `0` disables |
| `VKEYS_BROKER_RATE_WINDOW_SECS` | `60` | limiter window in seconds; `0` disables |
| `VKEYS_BROKER_MAX_REQUEST_TOKENS` | `200000` | per-request `max_tokens` ceiling and reservation unit for the budget |

Limits apply to the peer IP and the presented token; both buckets must allow a
request. This caps unauthenticated floods per IP while giving each token its own
budget. Rate and spend state live in memory and reset when the broker restarts.

Because the IP bucket is shared, clients behind one address (loopback, or a
tailnet exit/NAT) share that bucket: one busy client can exhaust the per-IP
allowance and starve the others with 429s even when their token buckets are
idle. The token bucket still gives each token its own allowance, so the fix is
to raise `VKEYS_BROKER_RATE_LIMIT` or give each client its own token. The
ordering is deliberate: the IP bucket is checked first so an unauthenticated
flood is capped before any token work. Do not remove it without an equivalent
unauthenticated flood cap.

## Token rotation

Two tokens can be valid at once so rotation needs no downtime for clients:

1. Generate a new broker token and store it in vkeys (for example
   `vkeys set broker/token-next`).
2. Set `VKEYS_BROKER_TOKEN` to the new token and
   `VKEYS_BROKER_TOKEN_PREVIOUS` to the current token, then restart the broker.
3. Move clients to the new token.
4. Unset `VKEYS_BROKER_TOKEN_PREVIOUS` and restart the broker. The old token
   is now rejected with 401.

The provider key is not rotated by this procedure; rotate it separately in
OpenRouter and update `openrouter/broker`.

## Remote Ravens

The default is `127.0.0.1`: only local processes can connect. To serve a Pi or
another machine, bind to a Tailscale address and keep the broker off the public
internet:

```powershell
vkeys-broker --host 100.x.y.z --port 8787 ...
```

The Pi gets the broker token, never the provider key.

Binding is only half the control. Also:

- **Firewall:** allow the broker port on the Tailscale interface only (for
  example a Windows inbound rule scoped to the Tailscale adapter, or
  `ufw allow in on tailscale0 to any port 8787` on Linux). Do not create a
  router port-forward or a public listener.
- **Tailnet ACL:** restrict the port to the tags/users that need it (Pi, laptop)
  so other tailnet devices cannot reach it even with the port open.
- **Never bind `0.0.0.0`** on a host with a public interface. Loopback plus a
  tailnet address is the intended deployment.
- The broker token is still required on the tailnet. Treat any device that can
  reach the port as untrusted until the token check passes.

## Limits

- spend and rate state are in memory and reset when the broker restarts
- budget reservation uses `VKEYS_BROKER_MAX_REQUEST_TOKENS` as the per-request
  ceiling. If the ceiling is larger than the whole budget, requests are
  admitted one at a time; raise the budget or lower the ceiling for parallelism
- the broker ledger is capped at the reservation. The provider may still bill
  overage if the upstream ignores `max_tokens`; that overage is recorded in the
  audit line as `overage_usd` but is not charged to the in-memory ledger
- one provider (OpenRouter) and one upstream
- no streaming; a client that sends `"stream": true` gets 400
- per-client tokens and per-client budgets are deferred to ticket 011
- no persistent request queue, retries, or model allowlist yet

## Tests

```powershell
cd rust
cargo test --release          # 18 broker tests, local fake upstream
```

The broker tests use a local fake upstream. They never read the real vault,
never use a live provider key, and never call OpenRouter.
