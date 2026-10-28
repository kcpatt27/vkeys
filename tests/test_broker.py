import json
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib import error, request

from vkeys.broker import BrokerConfig, BrokerServer, BrokerState, build_config, build_parser


class FakeUpstream(BaseHTTPRequestHandler):
    mode = "ok"
    calls = 0
    auth = ""

    def log_message(self, format, *args):
        return

    def do_POST(self):
        type(self).calls += 1
        type(self).auth = self.headers.get("Authorization", "")
        length = int(self.headers.get("Content-Length", "0"))
        self.rfile.read(length)
        if type(self).mode == "error":
            body = b'{"error":"upstream failed"}'
            self.send_response(500)
        else:
            body = json.dumps({
                "model": "test/model",
                "choices": [{"message": {"content": "ok"}}],
                "usage": {"prompt_tokens": 100, "completion_tokens": 50},
            }).encode()
            self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def start_server(handler, **attrs):
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    for key, value in attrs.items():
        setattr(server, key, value)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


class BrokerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        FakeUpstream.mode = "ok"
        FakeUpstream.calls = 0
        FakeUpstream.auth = ""

        self.upstream = start_server(FakeUpstream)
        self.upstream_url = f"http://127.0.0.1:{self.upstream.server_address[1]}/v1"

    def tearDown(self):
        self.upstream.shutdown()
        self.upstream.server_close()
        self.tmp.cleanup()

    def start_broker(self, **overrides):
        config = BrokerConfig(
            host="127.0.0.1",
            port=0,
            upstream=self.upstream_url,
            provider_key="provider-secret",
            broker_token="broker-secret",
            audit=self.root / "audit.jsonl",
            budget_usd=5.0,
            price_in=1.0,
            price_out=1.0,
        )
        for key, value in overrides.items():
            setattr(config, key, value)
        state = BrokerState()
        server = BrokerServer(config, state)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        return server, state

    def call(self, server, token="broker-secret", payload=None):
        body = json.dumps(payload or {
            "model": "test/model",
            "messages": [{"role": "user", "content": "hello"}],
        }).encode()
        req = request.Request(
            f"http://127.0.0.1:{server.server_address[1]}/v1/chat/completions",
            data=body,
            method="POST",
            headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
        )
        try:
            with request.urlopen(req) as response:
                return response.status, json.loads(response.read())
        except error.HTTPError as exc:
            return exc.code, json.loads(exc.read())

    def test_health_is_public(self):
        server, _ = self.start_broker()
        with request.urlopen(f"http://127.0.0.1:{server.server_address[1]}/healthz") as response:
            self.assertEqual(response.status, 200)
            self.assertTrue(json.loads(response.read())["ok"])

    def test_missing_or_bad_token_is_rejected(self):
        server, _ = self.start_broker()
        status, body = self.call(server, token="wrong")
        self.assertEqual((status, body["error"]), (401, "invalid broker token"))
        self.assertEqual(FakeUpstream.calls, 0)

    def test_proxy_injects_provider_key_and_audits(self):
        server, _ = self.start_broker()
        status, body = self.call(server)
        self.assertEqual(status, 200)
        self.assertEqual(body["model"], "test/model")
        self.assertEqual(FakeUpstream.auth, "Bearer provider-secret")
        audit = (self.root / "audit.jsonl").read_text(encoding="utf-8")
        event = json.loads(audit.splitlines()[-1])
        self.assertEqual(event["status"], 200)
        self.assertEqual(event["prompt_tokens"], 100)
        self.assertEqual(event["completion_tokens"], 50)
        self.assertNotIn("provider-secret", audit)
        self.assertNotIn("hello", audit)

    def test_budget_refuses_next_call(self):
        server, _ = self.start_broker(budget_usd=0.0001, price_in=1.0, price_out=1.0)
        status, _ = self.call(server)
        self.assertEqual(status, 200)
        status, body = self.call(server)
        self.assertEqual((status, body["error"]), (429, "budget exhausted"))
        self.assertEqual(FakeUpstream.calls, 1)

    def test_upstream_error_is_proxied_and_audited(self):
        FakeUpstream.mode = "error"
        server, _ = self.start_broker()
        status, body = self.call(server)
        self.assertEqual((status, body["error"]), (500, "upstream failed"))
        event = json.loads((self.root / "audit.jsonl").read_text(encoding="utf-8").splitlines()[-1])
        self.assertEqual(event["status"], 500)

    def test_streaming_is_rejected(self):
        server, _ = self.start_broker()
        status, body = self.call(server, payload={
            "model": "test/model", "stream": True,
            "messages": [{"role": "user", "content": "hello"}],
        })
        self.assertEqual((status, body["error"]), (400, "streaming is not supported"))

    def test_config_requires_both_secrets(self):
        args = build_parser().parse_args([])
        with self.assertRaisesRegex(ValueError, "OPENROUTER_API_KEY"):
            build_config(args, env={})
        with self.assertRaisesRegex(ValueError, "VKEYS_BROKER_TOKEN"):
            build_config(args, env={"OPENROUTER_API_KEY": "provider"})


if __name__ == "__main__":
    unittest.main()
