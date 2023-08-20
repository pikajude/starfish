import { List } from "immutable";
import { Component, Fragment, RenderableProps, h } from "preact";
import Helmet from "preact-helmet";
import { useCallback, useEffect, useState } from "preact/hooks";
import * as api from "../api";
import cx from "../style";
import { ifn } from "../util";

type BuildState =
  | { is: "ok"; s: api.GetBuild }
  | { is: "error"; s: { error: api.Error } }
  | { is: "loading" };

export default function Build({ ...props }: RenderableProps<{ id: string }>) {
  const [buildState, setBuildState] = useState<BuildState>({ is: "loading" });

  useEffect(() => {
    async function foo() {
      const build_json = await api.get(`/api/build/${props.id}`);
      if (build_json.ok) {
        setBuildState({ is: "ok", s: await build_json.json() });
      } else {
        setBuildState({ is: "error", s: await build_json.json() });
      }
    }

    foo();
  }, [setBuildState, props.id]);

  const restart = useCallback(() => {
    async function foo() {
      const r = await api.put(`/api/build/${props.id}/restart`, "");
      const status: { success: boolean } = await r.json();
      if (status.success) {
        window.location.reload();
      } else {
        alert("Failed to restart build");
      }
    }

    foo();
  }, [props.id]);

  if (buildState.is == "loading") {
    return <div>Loading...</div>;
  } else if (buildState.is == "error") {
    return <div>{buildState.s.error.description}</div>;
  }

  const { build, inputs } = buildState.s;

  return (
    <>
      <Helmet title={`Build #${build.id} - Starfish`} />
      <div class={cx("cell")}>
        <h4>
          Build #{build.id}{" "}
          <span class={cx("label", labelclass(build.status))}>
            {build.status}
          </span>
        </h4>
        <p>
          {build.origin} @ {build.rev}
        </p>
        {ifn(build.error_msg, (msg) => (
          <div class={cx("callout", "alert")}>
            <p>{msg}</p>
          </div>
        ))}
        {api.isRunning(build.status) ? null : (
          <p>
            <button class={cx("button", "small")} onClick={restart}>
              Restart build
            </button>
          </p>
        )}
        <Tailer id={build.id} size={20} />
        <Outputs data={inputs} />
      </div>
    </>
  );
}

type TailerProps = { id: number; size: number };

class Tailer extends Component<
  TailerProps,
  { tailHead: List<string>; tailTail: string; loadIndicator: null | string }
> {
  src: EventSource;

  constructor(props: RenderableProps<TailerProps>) {
    super(props);

    this.state = {
      tailHead: List(),
      tailTail: "",
      loadIndicator: "loading log...",
    };

    this.src = new EventSource(`/api/build/${props.id}/tail?len=${props.size}`);
    this.src.onmessage = (msg) => this.handleEvent(JSON.parse(msg.data));
    this.src.onerror = (err) => this.handleError(err);
  }

  componentWillUnmount() {
    this.src.close();
  }

  handleError(_ev: Event) {
    this.setState({
      tailTail:
        "unable to communicate with logger backend. try using the raw link (see above).",
      tailHead: List(),
    });
  }

  handleEvent(m: api.TailEvent) {
    switch (m.t) {
      case "Text":
        this.setState((oldState) => {
          if (/\n/.test(m.c)) {
            const addedLines = m.c.split("\n");
            const newHead = oldState.tailHead.withMutations((newHead) => {
              newHead.push(oldState.tailTail + addedLines.shift());
              if (newHead.size > this.props.size) {
                newHead.shift();
              }
              while (addedLines.length > 1) {
                newHead.push(addedLines.shift()!);
                if (newHead.size > this.props.size) {
                  newHead.shift();
                }
              }
            });
            const newTail = addedLines.shift() ?? "";
            return {
              tailHead: newHead,
              tailTail: newTail,
              loadIndicator: null,
            };
          }
          return {
            tailHead: oldState.tailHead,
            tailTail: oldState.tailTail + m.c,
            loadIndicator: null,
          };
        });
        break;
      case "Error":
        console.warn(m.c);
        break;
      case "Lines": {
        this.setState({ tailTail: "", tailHead: List(m.c) });
        break;
      }
      case "Reset":
        this.setState({ tailTail: "", tailHead: List() });
        break;
    }
  }

  render() {
    return (
      <>
        <p>
          Last {this.props.size} lines of log:{" "}
          <a href={`/build/${this.props.id}/raw`} target="_top">
            (view full log)
          </a>
        </p>
        <pre class={cx("pre-tail")}>
          {this.state.loadIndicator}
          {this.state.tailHead.toArray().map((x) => `${x}\n`)}
          {this.state.tailTail}
        </pre>
      </>
    );
  }
}

const Outputs = ({ data }: { data: api.InputOutputs[] }) => {
  if (data.length == 0) {
    return null;
  }
  return (
    <>
      <h5>Outputs</h5>
      <table>
        <thead>
          <tr>
            <th>Nix file</th>
            <th>Paths</th>
          </tr>
        </thead>
        <tbody>
          {data.map((input, i) => (
            <tr key={i}>
              <td>
                <code>{input.path}</code>
              </td>
              {input.outputs.length == 0 ? (
                <td>(not built)</td>
              ) : (
                <td>
                  <ul>
                    {input.outputs.map((output, i) => (
                      <li key={i}>
                        {output.store_path} - {output.system}
                      </li>
                    ))}
                  </ul>
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
};

const labelclass = (stat: api.BuildStatus) => {
  switch (stat) {
    case "building":
    case "uploading":
      return "primary";
    case "succeeded":
      return "success";
    case "queued":
      return "secondary";
    case "canceled":
      return "warning";
    case "failed":
      return "alert";
    default:
      throw new Error("unreachable");
  }
};
