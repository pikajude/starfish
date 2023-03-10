import { h } from "preact";
import { route } from "preact-router";
import { useCallback, useState } from "preact/hooks";
import { Build, BuildNew, put } from "../api";
import cx from "../style";

export default function NewBuild() {
  const [build, setBuild] = useState(BuildNew());

  const submit = useCallback(() => {
    async function dothething() {
      const resp = await put("/api/build", JSON.stringify(build.toJSON()));
      const build_json: Build = await resp.json();
      route(`/build/${build_json.id}`, false);
    }

    dothething();
  }, [build]);

  return (
    <form>
      <div class={cx("grid-x", "grid-margin-x")}>
        <div class={cx("cell", "medium-6")}>
          <label>
            URL:{" "}
            <input
              name="origin"
              type="text"
              value={build.origin}
              onInput={(e) =>
                setBuild((old) =>
                  old.set("origin", (e.target as HTMLInputElement).value)
                )
              }
            />
          </label>
        </div>
        <div class={cx("cell", "medium-6")}>
          <label>
            Commit SHA, branch, tag, etc:{" "}
            <input
              name="rev"
              type="text"
              value={build.rev}
              onInput={(e) =>
                setBuild((old) =>
                  old.set("rev", (e.target as HTMLInputElement).value)
                )
              }
            />
          </label>
        </div>
        <div class={cx("cell")}>
          <label>
            Extra paths to build:{" "}
            <input
              name="paths"
              type="text"
              value={build.paths}
              onInput={(e) =>
                setBuild((old) =>
                  old.set("paths", (e.target as HTMLInputElement).value)
                )
              }
            />
          </label>
        </div>
        <div class={cx("cell")}>
          <button
            class={cx("button")}
            onClick={(e) => {
              e.preventDefault();
              submit();
            }}
          >
            Trigger a build
          </button>
        </div>
      </div>
    </form>
  );
}
