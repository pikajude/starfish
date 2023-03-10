import { h } from "preact";
import cx from "../style";

interface StarfishWindow extends Window {
  STARFISH_VERSION: string;
  STARFISH_SHA: string;
}

declare let window: StarfishWindow;

export default function Footer() {
  return (
    <footer class={cx("cell")}>
      <p>
        Starfish {window.STARFISH_VERSION}.{window.STARFISH_SHA}
      </p>
    </footer>
  );
}
