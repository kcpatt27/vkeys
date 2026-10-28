"""Tier 2 OpenRouter broker.

The provider key is injected into this process by vkeys. Clients never receive
it; they authenticate with a separate broker token. The broker exposes only the
OpenAI-compatible chat-completions path needed by local agents.
"""

from __future__ import annotations

import argparse
import hmac
import json
import os
import sys
import time
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from threading import Lock
from typing import Any
from urllib import error, request

DEFAULT_UPSTREAM = "https://openrouter.ai/api/v1"
DEFAULT_AUDIT = Path(os.environ.get("PROGRAMDATA", ".")) / "secrets" / "broker-audit.jsonl"


@dataclass
class BrokerConfig:
    host: str
    port: int
    upstream: str
    provider_key: str
    broker_token: str
    audit: Path
    budget_usd: float
    price_in: float
    price_out: float


class BrokerState:
    def __init__(self) -> None:
        self.spent_usd = 0.0
        self.lock = Lock()


def _tokens(usage: Any) -> tuple[int, int]:
    if not isinstance(usage, dict):
        return 0, 0
    prompt = usage.get("prompt_tokens")
    completion = usage.get("completion_tokens")
    return (
        int(prompt) if isinstance(prompt, (int, float)) else 0,
        int(completion) if isinstance(completion, (int, float)) else 0,
    )


def _cost(prompt: int, completion: int, config: BrokerConfig) -> float:
    return (prompt * config.price_in + completion * config.price_out) / 1_000_000


def _audit(config: BrokerConfig, event: dict[str, Any]) -> None:
    try:
        config.audit.parent.mkdir(parents=True, exist_ok=True)
        with config.audit.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(event, ensure_ascii=False) + "\n")
    except OSError:
        pass


class BrokerServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, config: BrokerConfig, state: BrokerState) -> None:
        self.config = config
        self.state = state
        super().__init__((config.host, config.port), BrokerHandler)


class BrokerHandler(BaseHTTPRequestHandler):
    server: BrokerServer

    def log_message(self, format: str, *args: Any) -> None:
        # Keep request lines out of logs; the audit has the useful fields.
        return

    def _send(self, status: int, body: bytes, content_type: str = "application/json") -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _json(self, status: int, payload: dict[str, Any]) -> None:
        self._send(status, json.dumps(payload).encode("utf-8"))

    def _authorized(self) -> bool:
        raw = self.headers.get("Authorization", "")
        expected = f"Bearer {self.server.config.broker_token}"
        return hmac.compare_digest(raw, expected)

    def do_GET(self) -> None:  # noqa: N802 - http.server API
        if self.path != "/healthz":
            self._json(404, {"error": "not found"})
            return
        self._json(
            200,
            {
                "ok": True,
                "spent_usd": round(self.server.state.spent_usd, 6),
                "budget_usd": self.server.config.budget_usd,
            },
        )

    def do_POST(self) -> None:  # noqa: N802 - http.server API
        if self.path != "/v1/chat/completions":
            self._json(404, {"error": "not found"})
            return

        if not self._authorized():
            _audit(self.server.config, {
                "ts": time.strftime("%Y-%m-%dT%H:%M:%S"),
                "status": 401,
                "model": None,
                "prompt_tokens": 0,
                "completion_tokens": 0,
                "cost_usd": 0.0,
            })
            self._json(401, {"error": "invalid broker token"})
            return

        if self.server.config.budget_usd > 0 and self.server.state.spent_usd >= self.server.config.budget_usd:
            _audit(self.server.config, {
                "ts": time.strftime("%Y-%m-%dT%H:%M:%S"),
                "status": 429,
                "model": None,
                "prompt_tokens": 0,
                "completion_tokens": 0,
                "cost_usd": 0.0,
            })
            self._json(429, {"error": "budget exhausted"})
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            length = 0
        if length <= 0 or length > 10_000_000:
            self._json(400, {"error": "invalid body"})
            return
        body = self.rfile.read(length)

        try:
            payload = json.loads(body)
        except json.JSONDecodeError:
            self._json(400, {"error": "invalid JSON"})
            return
        if payload.get("stream"):
            self._json(400, {"error": "streaming is not supported"})
            return

        upstream = self.server.config.upstream.rstrip("/") + "/chat/completions"
        upstream_request = request.Request(
            upstream,
            data=body,
            method="POST",
            headers={
                "Authorization": f"Bearer {self.server.config.provider_key}",
                "Content-Type": "application/json",
            },
        )

        try:
            with request.urlopen(upstream_request, timeout=120) as response:
                status = response.status
                response_body = response.read()
                content_type = response.headers.get("Content-Type", "application/json")
        except error.HTTPError as exc:
            status = exc.code
            response_body = exc.read()
            content_type = exc.headers.get("Content-Type", "application/json")
        except (error.URLError, TimeoutError, OSError):
            status = 502
            response_body = json.dumps({"error": "upstream unavailable"}).encode("utf-8")
            content_type = "application/json"

        model = payload.get("model")
        prompt_tokens = 0
        completion_tokens = 0
        cost = 0.0
        try:
            parsed = json.loads(response_body)
            prompt_tokens, completion_tokens = _tokens(parsed.get("usage"))
            model = parsed.get("model") or model
            cost = _cost(prompt_tokens, completion_tokens, self.server.config)
        except (json.JSONDecodeError, TypeError):
            parsed = None

        with self.server.state.lock:
            self.server.state.spent_usd += cost

        _audit(self.server.config, {
            "ts": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "status": status,
            "model": model,
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "cost_usd": round(cost, 8),
        })
        self._send(status, response_body, content_type)


def build_config(args: argparse.Namespace, env: dict[str, str] | None = None) -> BrokerConfig:
    environment = env if env is not None else os.environ
    provider_key = environment.get("OPENROUTER_API_KEY", "")
    broker_token = environment.get("VKEYS_BROKER_TOKEN", "")
    if not provider_key:
        raise ValueError("OPENROUTER_API_KEY is required")
    if not broker_token:
        raise ValueError("VKEYS_BROKER_TOKEN is required")
    return BrokerConfig(
        host=args.host,
        port=args.port,
        upstream=args.upstream,
        provider_key=provider_key,
        broker_token=broker_token,
        audit=Path(args.audit),
        budget_usd=args.budget_usd,
        price_in=args.price_in,
        price_out=args.price_out,
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="vkeys-broker", description="Tier 2 OpenRouter broker")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8787)
    parser.add_argument("--upstream", default=DEFAULT_UPSTREAM)
    parser.add_argument("--audit", default=str(DEFAULT_AUDIT))
    parser.add_argument("--budget-usd", type=float, default=5.0)
    parser.add_argument("--price-in", type=float, default=0.0, help="USD per 1M input tokens")
    parser.add_argument("--price-out", type=float, default=0.0, help="USD per 1M output tokens")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        config = build_config(args)
    except ValueError as exc:
        print(f"vkeys-broker: {exc}", file=sys.stderr)
        return 2

    state = BrokerState()
    server = BrokerServer(config, state)
    print(f"vkeys-broker listening on http://{config.host}:{config.port}", file=sys.stderr)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
