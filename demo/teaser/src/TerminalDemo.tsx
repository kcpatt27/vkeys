import React from "react";
import {AbsoluteFill, interpolate, useCurrentFrame} from "remotion";

const MONO = "Consolas, 'Cascadia Mono', 'Courier New', monospace";
const SANS = "'Segoe UI', system-ui, sans-serif";
const BG = "#0b0f14";
const PANEL = "#0f1620";
const BORDER = "#243040";
const TEXT = "#d7e0ea";
const DIM = "#8b949e";
const GREEN = "#3fb950";
const CYAN = "#56d4dd";
const RED = "#ff7b72";
const AMBER = "#d29922";

const fade = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

const Cursor: React.FC<{frame: number}> = ({frame}) => (
  <span style={{color: CYAN, opacity: Math.floor(frame / 8) % 2 === 0 ? 1 : 0}}>▋</span>
);

const Type: React.FC<{text: string; start: number; cps?: number}> = ({text, start, cps = 1.8}) => {
  const frame = useCurrentFrame();
  if (frame < start) {
    return null;
  }
  const n = Math.max(0, Math.min(text.length, Math.floor((frame - start) * cps)));
  return (
    <div style={{color: TEXT, whiteSpace: "pre"}}>
      <span style={{color: GREEN}}>$ </span>
      {text.slice(0, n)}
      {n < text.length ? <Cursor frame={frame} /> : null}
    </div>
  );
};

const Out: React.FC<{start: number; color?: string; children: React.ReactNode}> = ({
  start,
  color = TEXT,
  children,
}) => {
  const frame = useCurrentFrame();
  if (frame < start) {
    return null;
  }
  return <div style={{color, opacity: fade(frame, start, start + 8), whiteSpace: "pre"}}>{children}</div>;
};

export const TerminalDemo: React.FC = () => {
  const frame = useCurrentFrame();
  const title = fade(frame, 0, 15);
  const caption = Math.min(fade(frame, 470, 500), 1 - fade(frame, 560, 590));
  const end = fade(frame, 590, 620);

  return (
    <AbsoluteFill style={{backgroundColor: BG, fontFamily: MONO, color: TEXT}}>
      <div
        style={{
          position: "absolute",
          inset: 40,
          background: PANEL,
          borderRadius: 16,
          border: `1px solid ${BORDER}`,
          overflow: "hidden",
          boxShadow: "0 30px 80px rgba(0,0,0,.55)",
          opacity: title,
        }}
      >
        <div
          style={{
            height: 44,
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "0 16px",
            borderBottom: `1px solid ${BORDER}`,
            background: "#0d141d",
          }}
        >
          <span style={{width: 11, height: 11, borderRadius: 99, background: "#ff5f56"}} />
          <span style={{width: 11, height: 11, borderRadius: 99, background: "#ffbd2e"}} />
          <span style={{width: 11, height: 11, borderRadius: 99, background: "#27c93f"}} />
          <span style={{marginLeft: 12, color: DIM, fontSize: 14}}>
            vkeys — demo (temp vault, fake key)
          </span>
        </div>

        <div style={{padding: "26px 30px", fontSize: 21, lineHeight: 1.55}}>
          <Type text="vkeys set demo/key" start={20} />
          <Out start={70} color={GREEN}>
            stored demo/key
          </Out>

          <div style={{height: 14}} />
          <Type text="vkeys ls" start={100} />
          <Out start={140} color={DIM}>
            {"NAME                             SET                  LAST USED            USES"}
          </Out>
          <Out start={155}>
            {"demo/key                         2026-10-30T14:23:36  -                       0"}
          </Out>

          <div style={{height: 14}} />
          <Type text="vkeys run demo" start={215} />
          <Out start={255} color={RED}>
            [REDACTED]
          </Out>
          <Out start={275} color={AMBER}>
            vkeys: redacted 1 secret occurrence(s) from output
          </Out>

          <div style={{height: 14}} />
          <Type text="vkeys audit --tail 1" start={335} />
          <Out start={375} color={DIM}>
            <span style={{fontSize: 16}}>
              {`{"ts": "2026-10-30T14:23:36", "cmd": "run", "profile": "demo",\n "keys": ["DEMO_KEY"], "scrub_hits": 1, "exit": 0, "ppid": 40704}`}
            </span>
          </Out>
        </div>
      </div>

      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          bottom: 78,
          textAlign: "center",
          fontFamily: SANS,
          fontSize: 34,
          fontWeight: 600,
          color: TEXT,
          opacity: caption,
        }}
      >
        the agent never sees the value
      </div>

      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 18,
          background: BG,
          opacity: end,
        }}
      >
        <div style={{fontFamily: SANS, fontSize: 92, fontWeight: 800, letterSpacing: -2}}>vkeys</div>
        <div style={{fontFamily: MONO, fontSize: 28, color: GREEN}}>cargo install vkeys</div>
        <div style={{fontFamily: MONO, fontSize: 18, color: DIM}}>github.com/kcpatt27/vkeys</div>
      </div>
    </AbsoluteFill>
  );
};
