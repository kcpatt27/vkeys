#!/usr/bin/env bash
# Install the hardened Raven broker: dedicated vkeys system user + systemd service.
# Run with sudo. Binaries default to ./rust/target/release (or pass a dir).
#
#   sudo ./deploy/raven/install.sh /path/to/dir/with/vkeys-and-vkeys-broker
#
# After it creates the user/dirs, seed the two keys as the vkeys user (see the
# printed commands), then enable the service. The agent user (kcpat) cannot read
# /etc/vkeys or /var/lib/vkeys.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${1:-$PWD/rust/target/release}"

if [[ $EUID -ne 0 ]]; then
  echo "run with sudo" >&2
  exit 1
fi

if ! id -u vkeys >/dev/null 2>&1; then
  useradd --system --home /var/lib/vkeys --shell /usr/sbin/nologin vkeys
fi

install -m 0755 "$BIN_DIR/vkeys" /usr/local/bin/vkeys
install -m 0755 "$BIN_DIR/vkeys-broker" /usr/local/bin/vkeys-broker

install -d -o vkeys -g vkeys -m 0700 /var/lib/vkeys /etc/vkeys
if [[ ! -f /etc/vkeys/profiles.toml ]]; then
  install -o vkeys -g vkeys -m 0600 "$HERE/profiles.toml" /etc/vkeys/profiles.toml
fi
install -m 0644 "$HERE/vkeys-broker.service" /etc/systemd/system/vkeys-broker.service
systemctl daemon-reload

cat <<'EOF'
installed.

Seed the two keys as the vkeys user (hidden prompt; do NOT paste them in chat):

  sudo -u vkeys env VKEYS_VAULT=/var/lib/vkeys/vault.bin \
    VKEYS_MASTER_KEY_FILE=/etc/vkeys/master.key \
    /usr/local/bin/vkeys set openrouter/broker

  sudo -u vkeys env VKEYS_VAULT=/var/lib/vkeys/vault.bin \
    VKEYS_MASTER_KEY_FILE=/etc/vkeys/master.key \
    /usr/local/bin/vkeys set broker/token

Then:

  sudo systemctl enable --now vkeys-broker
  systemctl status vkeys-broker --no-pager
  curl -s http://127.0.0.1:8787/healthz

The agent user (kcpat) cannot read /etc/vkeys or /var/lib/vkeys; it only gets
the broker token for use. Bind the broker to loopback or the tailnet only.
EOF
