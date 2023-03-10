import styles from "../css/main.scss";
import classNames from "classnames/bind";

export type Classname = keyof typeof styles;
export type Argument = Classname | { [K in Classname]?: boolean } | Argument[];

const cx: (...args: Argument[]) => string = classNames.bind(styles);

export default cx;
