import { h } from "preact";
import cx from "../style";
import { Link } from "preact-router";

export default function Nav() {
  return (
    <div class={cx("top-bar")} style="margin-bottom: 10px">
      <div class={cx("top-bar-left")}>
        <ul class={cx("menu")}>
          <li class={cx("menu-text")}>Starfish</li>
          <li>
            <Link href="/">Home</Link>
          </li>
        </ul>
      </div>
    </div>
  );
}
