# Parity harness: Python vkeys vs Rust vkeys.
# Build first: cargo build --release  (in rust/)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$py = if (Test-Path "$root\.venv\Scripts\python.exe") {
    "$root\.venv\Scripts\python.exe"
} else {
    "python"
}
& $py "$PSScriptRoot\parity.py" @args
exit $LASTEXITCODE
