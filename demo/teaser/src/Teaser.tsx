import React from "react";
import {
  AbsoluteFill,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";

const SANS = "'Segoe UI', system-ui, sans-serif";
const MONO = "Consolas, 'Cascadia Mono', monospace";
const BG = "#070b10";
const GREEN = "#3fb950";
const CYAN = "#56d4dd";
const RED = "#ff7b72";
const TEXT = "#e6edf3";
const DIM = "#7d8590";
const PANEL = "rgba(15,22,32,0.72)";
const BORDER = "rgba(86,212,221,0.22)";

const clamp = {extrapolateLeft: "clamp" as const, extrapolateRight: "clamp" as const};
const mix = (frame: number, a: number, b: number) => interpolate(frame, [a, b], [0, 1], clamp);

const KeyGlyph: React.FC<{size?: number; color?: string; glow?: boolean; blur?: number}> = ({
  size = 120,
  color = GREEN,
  glow = true,
  blur = 0,
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 100 100"
    style={{
      filter: `${glow ? `drop-shadow(0 0 22px ${color}88)` : ""} ${blur ? `blur(${blur}px)` : ""}`.trim(),
    }}
  >
    <circle cx="30" cy="50" r="16" fill="none" stroke={color} strokeWidth="6" />
    <rect x="44" y="46" width="40" height="8" rx="3" fill={color} />
    <rect x="68" y="54" width="7" height="11" rx="2" fill={color} />
    <rect x="80" y="54" width="7" height="8" rx="2" fill={color} />
  </svg>
);

const Box: React.FC<{
  x: number;
  y: number;
  w: number;
  h: number;
  label: string;
  opacity: number;
  accent?: string;
  children?: React.ReactNode;
}> = ({x, y, w, h, label, opacity, accent = CYAN, children}) => (
  <div
    style={{
      position: "absolute",
      left: x,
      top: y,
      width: w,
      height: h,
      borderRadius: 20,
      border: `1px solid ${accent}66`,
      background: PANEL,
      boxShadow: `0 0 60px ${accent}14`,
      opacity,
    }}
  >
    <div style={{position: "absolute", top: -30, left: 2, fontFamily: MONO, fontSize: 18, color: accent}}>
      {label}
    </div>
    {children}
  </div>
);

export const Teaser: React.FC = () => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();

  const keyIn = spring({frame, fps, config: {damping: 14, stiffness: 90}});
  const keyX = interpolate(frame, [40, 140], [960, 450], clamp);
  const keyY = interpolate(frame, [40, 140], [470, 500], clamp);
  const keyScale = interpolate(frame, [40, 140], [1, 0.45], clamp);

  const vault = mix(frame, 55, 100);
  const pipe = mix(frame, 135, 190);
  const refX = interpolate(frame, [150, 220], [640, 760], clamp);
  const app = mix(frame, 175, 225);
  const agent = mix(frame, 250, 300);
  const redacted = mix(frame, 290, 330);
  const audit = mix(frame, 330, 370);
  const end = mix(frame, 400, 430);

  const phase =
    frame < 120
      ? "one key"
      : frame < 220
        ? "encrypted at rest"
        : frame < 300
          ? "profiles use it"
          : "the agent never sees it";

  return (
    <AbsoluteFill style={{backgroundColor: BG, fontFamily: SANS, color: TEXT, overflow: "hidden"}}>
      <div
        style={{
          position: "absolute",
          inset: 0,
          backgroundImage: `linear-gradient(${BORDER} 1px, transparent 1px), linear-gradient(90deg, ${BORDER} 1px, transparent 1px)`,
          backgroundSize: "80px 80px",
          opacity: 0.18,
        }}
      />

      <div style={{position: "absolute", top: 64, left: 80, fontFamily: MONO, fontSize: 22, color: DIM}}>
        vkeys <span style={{color: GREEN}}>//</span> virtual keys
      </div>

      {/* vault */}
      <Box x={280} y={380} w={340} h={240} label="vault" opacity={vault} accent={CYAN}>
        <div
          style={{
            position: "absolute",
            inset: 28,
            borderRadius: 14,
            border: `1px dashed ${CYAN}44`,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontFamily: MONO,
            fontSize: 20,
            color: DIM,
          }}
        >
          encrypted at rest
        </div>
      </Box>

      {/* profile pipe */}
      <div
        style={{
          position: "absolute",
          left: 620,
          top: 496,
          height: 3,
          width: 140 * pipe,
          background: `linear-gradient(90deg, ${CYAN}00, ${CYAN}, ${GREEN})`,
        }}
      />
      <div
        style={{
          position: "absolute",
          left: refX,
          top: 486,
          width: 18,
          height: 18,
          borderRadius: 5,
          border: `2px dashed ${GREEN}`,
          background: "#0b1118",
          opacity: pipe * (1 - app),
        }}
      />

      {/* app */}
      <Box x={760} y={380} w={400} h={240} label="profile → app" opacity={app} accent={GREEN}>
        <div
          style={{
            position: "absolute",
            inset: 28,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontFamily: MONO,
            fontSize: 22,
            color: GREEN,
          }}
        >
          OPENROUTER_API_KEY=&lt;ref&gt;
        </div>
      </Box>

      {/* agent */}
      <Box x={1300} y={380} w={420} h={240} label="agent" opacity={agent} accent={RED}>
        <div
          style={{
            position: "absolute",
            inset: 28,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            gap: 18,
          }}
        >
          <div style={{opacity: redacted * 0.5}}>
            <KeyGlyph size={74} color={RED} blur={7} />
          </div>
          <div style={{fontFamily: MONO, fontSize: 30, color: RED, opacity: redacted}}>[REDACTED]</div>
        </div>
        <div
          style={{
            position: "absolute",
            left: 28,
            bottom: 18,
            fontFamily: MONO,
            fontSize: 15,
            color: DIM,
            opacity: audit,
            lineHeight: 1.5,
          }}
        >
          scrub_hits: 1
          <br />
          keys: [&quot;DEMO_KEY&quot;]
        </div>
      </Box>

      {/* key */}
      <div
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          transform: `translate(${keyX - 60}px, ${keyY - 60}px) scale(${keyScale * keyIn})`,
          opacity: frame < 140 ? 1 : 0,
        }}
      >
        <KeyGlyph size={120} />
      </div>

      {/* phase line */}
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          bottom: 110,
          textAlign: "center",
          fontSize: 40,
          fontWeight: 600,
          letterSpacing: -0.5,
          color: TEXT,
          opacity: 1 - end,
        }}
      >
        {phase}
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          bottom: 62,
          textAlign: "center",
          fontFamily: MONO,
          fontSize: 18,
          color: DIM,
          opacity: 1 - end,
        }}
      >
        a key can be used without being seen
      </div>

      {/* end card */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 20,
          background: BG,
          opacity: end,
        }}
      >
        <KeyGlyph size={110} />
        <div style={{fontSize: 120, fontWeight: 800, letterSpacing: -4}}>vkeys</div>
        <div style={{fontFamily: MONO, fontSize: 30, color: GREEN}}>cargo install vkeys</div>
        <div style={{fontFamily: MONO, fontSize: 18, color: DIM}}>Apache-2.0 · Windows · macOS · Linux</div>
      </div>
    </AbsoluteFill>
  );
};
