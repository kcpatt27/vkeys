import {Composition} from "remotion";
import {Teaser} from "./Teaser";
import {TerminalDemo} from "./TerminalDemo";

export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Composition
        id="TerminalDemo"
        component={TerminalDemo}
        durationInFrames={720}
        fps={30}
        width={1280}
        height={720}
      />
      <Composition
        id="Teaser"
        component={Teaser}
        durationInFrames={450}
        fps={30}
        width={1920}
        height={1080}
      />
    </>
  );
};
