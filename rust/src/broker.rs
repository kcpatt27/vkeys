//! Tier 2 OpenRouter broker.
//!
//! The provider key is injected into this process by vkeys. Clients never
//! receive it; they authenticate with a separate broker token. Only the
//! OpenAI-compatible chat-completions path is exposed.
//!
//! Rust port of `src/vkeys/broker.py`; behaviour is kept in lockstep.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub const DEFAULT_UPSTREAM: &str = "https://openrouter.ai/api/v1";
pub const MAX_BODY: usize = 10_000_000;
pub const TIMEOUT_SECS: u64 = 120;

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub upstream: String,
    pub provider_key: String,
    pub broker_token: String,
    pub audit: PathBuf,
    pub budget_usd: f64,
    pub price_in: f64,
    pub price_out: f64,
}

pub struct State {
    spent_usd: Mutex<f64>,
}

impl State {
    pub fn new() -> Self {
        Self {
            spent_usd: Mutex::new(0.0),
        }
    }

    pub fn spent(&self) -> f64 {
        *self.spent_usd.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn add(&self, cost: f64) {
        let mut spent = self.spent_usd.lock().unwrap_or_else(|e| e.into_inner());
        *spent += cost;
    }
}

pub fn default_audit() -> PathBuf {
    if let Ok(v) = std::env::var("VKEYS_DATA_DIR") {
        return PathBuf::from(v).join("broker-audit.jsonl");
    }
    #[cfg(windows)]
    {
        let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string());
        return PathBuf::from(base)
            .join("secrets")
            .join("broker-audit.jsonl");
    }
    #[cfg(target_os = "macos")]
    {
        return home_dir()
            .join("Library")
            .join("Application Support")
            .join("vkeys")
            .join("broker-audit.jsonl");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return PathBuf::from(xdg).join("vkeys").join("broker-audit.jsonl");
        }
        return home_dir()
            .join(".local")
            .join("share")
            .join("vkeys")
            .join("broker-audit.jsonl");
    }
    #[allow(unreachable_code)]
    PathBuf::from("broker-audit.jsonl")
}

#[allow(dead_code)]
fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn local_timestamp() -> String {
    chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

#[allow(clippy::too_many_arguments)]
pub fn config_with(
    provider_key: Option<String>,
    broker_token: Option<String>,
    host: String,
    port: u16,
    upstream: String,
    audit: PathBuf,
    budget_usd: f64,
    price_in: f64,
    price_out: f64,
) -> Result<Config, String> {
    let provider_key = provider_key.unwrap_or_default();
    let broker_token = broker_token.unwrap_or_default();
    if provider_key.is_empty() {
        return Err("OPENROUTER_API_KEY is required".to_string());
    }
    if broker_token.is_empty() {
        return Err("VKEYS_BROKER_TOKEN is required".to_string());
    }
    Ok(Config {
        host,
        port,
        upstream,
        provider_key,
        broker_token,
        audit,
        budget_usd,
        price_in,
        price_out,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn config_from_env(
    host: String,
    port: u16,
    upstream: String,
    audit: PathBuf,
    budget_usd: f64,
    price_in: f64,
    price_out: f64,
) -> Result<Config, String> {
    config_with(
        std::env::var("OPENROUTER_API_KEY").ok(),
        std::env::var("VKEYS_BROKER_TOKEN").ok(),
        host,
        port,
        upstream,
        audit,
        budget_usd,
        price_in,
        price_out,
    )
}

pub fn bind(config: &Config) -> io::Result<Server> {
    Server::http((config.host.as_str(), config.port))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

pub fn serve(server: Server, config: Arc<Config>, state: Arc<State>) {
    for request in server.incoming_requests() {
        let cfg = Arc::clone(&config);
        let st = Arc::clone(&state);
        thread::spawn(move || handle(request, &cfg, &st));
    }
}

fn handle(request: Request, config: &Config, state: &State) {
    let method = request.method().clone();
    let url = request.url().to_string();
    match (method, url.as_str()) {
        (Method::Get, "/healthz") => {
            let body = json!({
                "ok": true,
                "spent_usd": round(state.spent(), 6),
                "budget_usd": config.budget_usd,
            });
            respond_json(request, 200, body);
        }
        (Method::Post, "/v1/chat/completions") => post_chat(request, config, state),
        _ => respond_json(request, 404, json!({"error": "not found"})),
    }
}

fn post_chat(mut request: Request, config: &Config, state: &State) {
    if !authorized(&request, &config.broker_token) {
        audit(
            config,
            json!({
                "ts": local_timestamp(), "status": 401, "model": Value::Null,
                "prompt_tokens": 0, "completion_tokens": 0, "cost_usd": 0.0,
            }),
        );
        respond_json(request, 401, json!({"error": "invalid broker token"}));
        return;
    }

    if config.budget_usd > 0.0 && state.spent() >= config.budget_usd {
        audit(
            config,
            json!({
                "ts": local_timestamp(), "status": 429, "model": Value::Null,
                "prompt_tokens": 0, "completion_tokens": 0, "cost_usd": 0.0,
            }),
        );
        respond_json(request, 429, json!({"error": "budget exhausted"}));
        return;
    }

    let length = content_length(&request);
    if length == 0 || length > MAX_BODY {
        respond_json(request, 400, json!({"error": "invalid body"}));
        return;
    }
    let mut body = Vec::new();
    let read = request
        .as_reader()
        .take((MAX_BODY as u64) + 1)
        .read_to_end(&mut body);
    if read.is_err() || body.len() > MAX_BODY {
        respond_json(request, 400, json!({"error": "invalid body"}));
        return;
    }

    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            respond_json(request, 400, json!({"error": "invalid JSON"}));
            return;
        }
    };
    if is_truthy(payload.get("stream")) {
        respond_json(request, 400, json!({"error": "streaming is not supported"}));
        return;
    }

    let upstream = format!("{}/chat/completions", config.upstream.trim_end_matches('/'));
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build();
    let result = agent
        .post(&upstream)
        .set("Authorization", &format!("Bearer {}", config.provider_key))
        .set("Content-Type", "application/json")
        .send_bytes(&body);

    let (status, response_body, content_type) = match result {
        Ok(response) => drain(response),
        Err(ureq::Error::Status(code, response)) => {
            let (_, body, content_type) = drain(response);
            (code, body, content_type)
        }
        Err(_) => (
            502,
            br#"{"error":"upstream unavailable"}"#.to_vec(),
            "application/json".to_string(),
        ),
    };

    let mut model = payload
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut prompt_tokens = 0i64;
    let mut completion_tokens = 0i64;
    let mut cost = 0.0f64;
    if let Ok(parsed) = serde_json::from_slice::<Value>(&response_body) {
        let (p, c) = tokens(parsed.get("usage"));
        prompt_tokens = p;
        completion_tokens = c;
        if let Some(m) = parsed.get("model").and_then(|v| v.as_str()) {
            model = Some(m.to_string());
        }
        cost = (prompt_tokens as f64 * config.price_in
            + completion_tokens as f64 * config.price_out)
            / 1_000_000.0;
    }
    state.add(cost);

    audit(
        config,
        json!({
            "ts": local_timestamp(),
            "status": status,
            "model": model,
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "cost_usd": round(cost, 8),
        }),
    );
    respond(request, status, response_body, &content_type);
}

fn authorized(request: &Request, token: &str) -> bool {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Authorization"))
        .map(|h| h.value.as_str() == format!("Bearer {token}"))
        .unwrap_or(false)
}

fn content_length(request: &Request) -> usize {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Content-Length"))
        .and_then(|h| h.value.as_str().trim().parse::<usize>().ok())
        .unwrap_or(0)
}

fn is_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => true,
    }
}

fn tokens(usage: Option<&Value>) -> (i64, i64) {
    let get = |key: &str| {
        usage
            .and_then(|u| u.get(key))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64
    };
    (get("prompt_tokens"), get("completion_tokens"))
}

fn round(value: f64, places: i32) -> f64 {
    let factor = 10f64.powi(places);
    (value * factor).round() / factor
}

fn drain(response: ureq::Response) -> (u16, Vec<u8>, String) {
    let status = response.status();
    let content_type = response
        .header("Content-Type")
        .unwrap_or("application/json")
        .to_string();
    let mut body = Vec::new();
    let _ = response.into_reader().read_to_end(&mut body);
    (status, body, content_type)
}

fn audit(config: &Config, event: Value) {
    if let Some(parent) = config.audit.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.audit)
    {
        let _ = writeln!(file, "{event}");
    }
}

fn respond_json(request: Request, status: u16, value: Value) {
    respond(
        request,
        status,
        value.to_string().into_bytes(),
        "application/json",
    );
}

fn respond(request: Request, status: u16, body: Vec<u8>, content_type: &str) {
    let mut response = Response::from_data(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()) {
        response.add_header(header);
    }
    let _ = request.respond(response);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn find_headers_end(bytes: &[u8]) -> Option<usize> {
        bytes
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|i| i + 4)
    }

    fn fake_upstream(mode: &'static str) -> (u16, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&calls);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut socket = match stream {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    match socket.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => {
                            buf.extend_from_slice(&chunk[..n]);
                            if find_headers_end(&buf).is_some() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let header_end = find_headers_end(&buf).unwrap_or(buf.len());
                let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
                if let Some(line) = head
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("authorization:"))
                {
                    seen.lock().unwrap().push(line.trim().to_string());
                }
                let len = head
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                    .and_then(|l| l.split(':').nth(1))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                while buf.len() < header_end + len {
                    match socket.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        Err(_) => break,
                    }
                }
                let (status, body) = if mode == "error" {
                    (500, r#"{"error":"upstream failed"}"#.to_string())
                } else {
                    (
                        200,
                        r#"{"model":"test/model","choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":100,"completion_tokens":50}}"#
                            .to_string(),
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes());
            }
        });
        (port, calls)
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "vkeys-broker-{tag}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn start_broker(
        upstream_port: u16,
        audit: PathBuf,
        budget: f64,
        price_in: f64,
        price_out: f64,
    ) -> (u16, Arc<Config>) {
        let config = Arc::new(Config {
            host: "127.0.0.1".to_string(),
            port: 0,
            upstream: format!("http://127.0.0.1:{upstream_port}/v1"),
            provider_key: "provider-secret".to_string(),
            broker_token: "broker-secret".to_string(),
            audit,
            budget_usd: budget,
            price_in,
            price_out,
        });
        let server = bind(&config).unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let state = Arc::new(State::new());
        let cfg = Arc::clone(&config);
        let st = Arc::clone(&state);
        thread::spawn(move || serve(server, cfg, st));
        (port, config)
    }

    fn post(port: u16, token: &str, body: &str) -> (u16, String) {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .build();
        let result = agent
            .post(&format!("http://127.0.0.1:{port}/v1/chat/completions"))
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_string(body);
        match result {
            Ok(response) => (response.status(), response.into_string().unwrap_or_default()),
            Err(ureq::Error::Status(code, response)) => {
                (code, response.into_string().unwrap_or_default())
            }
            Err(e) => panic!("transport error: {e}"),
        }
    }

    fn body() -> &'static str {
        r#"{"model":"test/model","messages":[{"role":"user","content":"hello"}]}"#
    }

    #[test]
    fn health_is_public() {
        let (upstream, _) = fake_upstream("ok");
        let dir = temp_dir("health");
        let (port, _) = start_broker(upstream, dir.join("audit.jsonl"), 5.0, 0.0, 0.0);
        let response = ureq::get(&format!("http://127.0.0.1:{port}/healthz"))
            .call()
            .unwrap();
        assert_eq!(response.status(), 200);
        let value: Value = serde_json::from_str(&response.into_string().unwrap()).unwrap();
        assert_eq!(value["ok"], true);
    }

    #[test]
    fn bad_token_is_rejected_without_upstream() {
        let (upstream, calls) = fake_upstream("ok");
        let dir = temp_dir("badtoken");
        let (port, _) = start_broker(upstream, dir.join("audit.jsonl"), 5.0, 0.0, 0.0);
        let (status, body) = post(port, "wrong", body());
        assert_eq!(status, 401);
        assert!(body.contains("invalid broker token"));
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn proxy_injects_provider_key_and_audits() {
        let (upstream, calls) = fake_upstream("ok");
        let dir = temp_dir("proxy");
        let audit = dir.join("audit.jsonl");
        let (port, _) = start_broker(upstream, audit.clone(), 5.0, 1.0, 1.0);
        let (status, body) = post(port, "broker-secret", body());
        assert_eq!(status, 200);
        assert!(body.contains("test/model"));
        assert_eq!(
            calls.lock().unwrap().last().map(|s| s.as_str()),
            Some("Authorization: Bearer provider-secret")
        );
        let text = fs::read_to_string(&audit).unwrap();
        let event: Value = serde_json::from_str(text.lines().last().unwrap()).unwrap();
        assert_eq!(event["status"], 200);
        assert_eq!(event["prompt_tokens"], 100);
        assert_eq!(event["completion_tokens"], 50);
        assert!(!text.contains("provider-secret"));
        assert!(!text.contains("hello"));
    }

    #[test]
    fn budget_refuses_next_call() {
        let (upstream, calls) = fake_upstream("ok");
        let dir = temp_dir("budget");
        let (port, _) = start_broker(upstream, dir.join("audit.jsonl"), 0.0001, 1.0, 1.0);
        assert_eq!(post(port, "broker-secret", body()).0, 200);
        let (status, body) = post(port, "broker-secret", body());
        assert_eq!(status, 429);
        assert!(body.contains("budget exhausted"));
        assert_eq!(calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn upstream_error_is_proxied_and_audited() {
        let (upstream, _) = fake_upstream("error");
        let dir = temp_dir("upstream-error");
        let audit = dir.join("audit.jsonl");
        let (port, _) = start_broker(upstream, audit.clone(), 5.0, 0.0, 0.0);
        let (status, body) = post(port, "broker-secret", body());
        assert_eq!(status, 500);
        assert!(body.contains("upstream failed"));
        let text = fs::read_to_string(&audit).unwrap();
        let event: Value = serde_json::from_str(text.lines().last().unwrap()).unwrap();
        assert_eq!(event["status"], 500);
    }

    #[test]
    fn streaming_is_rejected() {
        let (upstream, _) = fake_upstream("ok");
        let dir = temp_dir("stream");
        let (port, _) = start_broker(upstream, dir.join("audit.jsonl"), 5.0, 0.0, 0.0);
        let (status, body) = post(
            port,
            "broker-secret",
            r#"{"model":"test/model","stream":true,"messages":[]}"#,
        );
        assert_eq!(status, 400);
        assert!(body.contains("streaming is not supported"));
    }

    #[test]
    fn config_requires_both_secrets() {
        let err = config_with(None, None, "127.0.0.1".into(), 0, DEFAULT_UPSTREAM.into(), PathBuf::from("a"), 5.0, 0.0, 0.0)
            .err()
            .unwrap();
        assert!(err.contains("OPENROUTER_API_KEY"));
        let err = config_with(
            Some("provider".into()),
            None,
            "127.0.0.1".into(),
            0,
            DEFAULT_UPSTREAM.into(),
            PathBuf::from("a"),
            5.0,
            0.0,
            0.0,
        )
        .err()
        .unwrap();
        assert!(err.contains("VKEYS_BROKER_TOKEN"));
    }
}
