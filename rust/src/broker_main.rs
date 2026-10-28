//! `vkeys-broker` - Tier 2 OpenRouter broker (Rust port of `src/vkeys/broker.py`).

mod broker;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;

use broker::{bind, config_from_env, default_audit, serve, Config, State, DEFAULT_UPSTREAM};

#[derive(Parser)]
#[command(name = "vkeys-broker", about = "Tier 2 OpenRouter broker")]
struct Cli {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 8787)]
    port: u16,
    #[arg(long, default_value = DEFAULT_UPSTREAM)]
    upstream: String,
    #[arg(long)]
    audit: Option<PathBuf>,
    #[arg(long, default_value_t = 5.0)]
    budget_usd: f64,
    #[arg(long, default_value_t = 0.0)]
    price_in: f64,
    #[arg(long, default_value_t = 0.0)]
    price_out: f64,
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let cli = Cli::parse();
    let config: Config = match config_from_env(
        cli.host,
        cli.port,
        cli.upstream,
        cli.audit.unwrap_or_else(default_audit),
        cli.budget_usd,
        cli.price_in,
        cli.price_out,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("vkeys-broker: {e}");
            return 2;
        }
    };
    let server = match bind(&config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("vkeys-broker: {e}");
            return 1;
        }
    };
    eprintln!(
        "vkeys-broker listening on http://{}:{}",
        config.host, config.port
    );
    serve(server, Arc::new(config), Arc::new(State::new()));
    0
}
