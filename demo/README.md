# Demo assets

- `teaser/` — Remotion project with two compositions:
  - **`TerminalDemo`** (24 s, 1280×720): a code-rendered recreation of **real**
    vkeys output, using a temp vault and a fake key. It shows the hidden set, the
    names-only listing, the `[REDACTED]` output, and the audit line.
  - **`Teaser`** (15 s, 1920×1080): key → vault → profile → app → a redacted
    agent pane.
- `demo-profiles.toml` — fake profile used by the terminal demo
  (`cmd /c echo %DEMO_KEY%`, Windows).
- `demo-profiles-linux.toml` — the same demo for Linux/macOS
  (`sh -c 'echo $DEMO_KEY'`).

VHS was tried first and hangs without a PTY on this machine, so the terminal
demo is rendered from the same real output instead of a live cast.

## Render

```powershell
cd demo/teaser
npm install
npx remotion render src/index.ts TerminalDemo out/terminal.mp4 --browser-executable "C:\Program Files\Google\Chrome\Application\chrome.exe"
npx remotion render src/index.ts Teaser out/teaser.mp4 --browser-executable "C:\Program Files\Google\Chrome\Application\chrome.exe"
```

GIF for the README:

```powershell
ffmpeg -y -i demo/out/terminal.mp4 -vf "fps=20,scale=1200:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse" docs/assets/demo.gif
```

The fake key and demo vault live only under `demo/` and are git-ignored.
