#!/usr/bin/env bash
# One-shot hardened Raven install.
#
#   sudo ./deploy/raven/install.sh /path/to/dir/with/vkeys-and-vkeys-broker
#
# Creates a dedicated vkeys system user, installs both binaries, seeds the
# provider key (hidden prompt), generates a broker token for the agent, enables
# the systemd service, and verifies /healthz.
#
# The agent user cannot read /etc/vkeys or /var/lib/vkeys. It only gets
# ~/.config/vkeys/broker.token (use-only).
#
# Non-interactive: set VKEYS_PROVIDER_KEY_FILE to a root-only file containing
# the provider key; the broker token is always generated.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${1:-$PWD/rust/target/release}"
SERVICE_NAME="vkeys-broker"
SERVICE_USER="vkeys"
STATE_DIR="/var/lib/vkeys"
CONF_DIR="/etc/vkeys"
VAULT="$STATE_DIR/vault.bin"
KEY_FILE="$CONF_DIR/master.key"
PROFILES="$CONF_DIR/profiles.toml"

AGENT_USER="${SUDO_USER:-$(logname 2>/dev/null || echo kcpat)}"
AGENT_HOME="$(getent passwd "$AGENT_USER" | cut -d: -f6 || true)"
AGENT_HOME="${AGENT_HOME:-/home/$AGENT_USER}"
AGENT_TOKEN="$AGENT_HOME/.config/vkeys/broker.token"

die() { echo "error: $*" >&2; exit 1; }

[[ $EUID -eq 0 ]] || die "run with sudo"
[[ -x "$BIN_DIR/vkeys" && -x "$BIN_DIR/vkeys-broker" ]] || die "vkeys binaries not found in $BIN_DIR"

vkeys_as_service() {
  runuser -u "$SERVICE_USER" -- env \
    VKEYS_VAULT="$VAULT" \
    VKEYS_MASTER_KEY_FILE="$KEY_FILE" \
    VKEYS_PROFILES="$PROFILES" \
    /usr/local/bin/vkeys "$@"
}

seed() {
  # The value goes in on stdin, never argv.
  printf '%s\n' "$2" | vkeys_as_service set "$1" --stdin >/dev/null
}

has_key() {
  vkeys_as_service ls 2>/dev/null | grep -q "^$1[[:space:]]"
}

if ! id -u "$SERVICE_USER" >/dev/null 2>&1; then
  useradd --system --home "$STATE_DIR" --shell /usr/sbin/nologin "$SERVICE_USER"
fi

install -m 0755 "$BIN_DIR/vkeys" /usr/local/bin/vkeys
install -m 0755 "$BIN_DIR/vkeys-broker" /usr/local/bin/vkeys-broker
install -d -o "$SERVICE_USER" -g "$SERVICE_USER" -m 0700 "$STATE_DIR" "$CONF_DIR"
[[ -f "$PROFILES" ]] || install -o "$SERVICE_USER" -g "$SERVICE_USER" -m 0600 "$HERE/profiles.toml" "$PROFILES"
install -m 0644 "$HERE/vkeys-broker.service" "/etc/systemd/system/$SERVICE_NAME.service"

if has_key "openrouter/broker"; then
  echo "provider key already stored"
else
  if [[ -n "${VKEYS_PROVIDER_KEY_FILE:-}" ]]; then
    [[ -r "$VKEYS_PROVIDER_KEY_FILE" ]] || die "cannot read $VKEYS_PROVIDER_KEY_FILE"
    PROVIDER="$(cat "$VKEYS_PROVIDER_KEY_FILE")"
  else
    read -rsp "OpenRouter provider key (hidden): " PROVIDER
    echo
  fi
  [[ -n "$PROVIDER" ]] || die "no provider key entered"
  seed "openrouter/broker" "$PROVIDER"
  unset PROVIDER
fi

if [[ -s "$AGENT_TOKEN" ]] && has_key "broker/token"; then
  echo "broker token already present at $AGENT_TOKEN"
else
  TOKEN="$(head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
  seed "broker/token" "$TOKEN"
  install -d -o "$AGENT_USER" -g "$AGENT_USER" -m 0700 "$AGENT_HOME/.config/vkeys"
  printf '%s\n' "$TOKEN" > "$AGENT_TOKEN"
  chown "$AGENT_USER:$AGENT_USER" "$AGENT_TOKEN"
  chmod 0600 "$AGENT_TOKEN"
  unset TOKEN
  echo "broker token written to $AGENT_TOKEN"
fi

systemctl daemon-reload
systemctl enable --now "$SERVICE_NAME"

for _ in $(seq 1 15); do
  if curl -fsS http://127.0.0.1:8787/healthz >/dev/null 2>&1; then
    echo "broker healthy: $(curl -s http://127.0.0.1:8787/healthz)"
    echo "agent token: $AGENT_TOKEN (use-only, 0600)"
    exit 0
  fi
  sleep 1
done

echo "health check failed; recent logs:" >&2
journalctl -u "$SERVICE_NAME" -n 50 --no-pager >&2 || true
exit 1
