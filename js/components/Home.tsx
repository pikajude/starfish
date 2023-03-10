import { h, Fragment } from "preact";
import { useEffect, useState } from "preact/hooks";
import { Build, get } from "../api";
import { Link } from "preact-router";

import cx from "../style";
import NewBuild from "./NewBuild";

export default function Home() {
  const [builds, setBuilds] = useState<Build[]>([]);

  useEffect(() => {
    async function dothingy() {
      const all_builds = await get("/api/builds");
      const js = await all_builds.json();
      setBuilds(js);
    }

    dothingy();
  }, [setBuilds]);

  return (
    <>
      <div id="all-builds" class={cx("cell")}>
        <table>
          <thead>
            <tr>
              <th>-</th>
              <th>URL</th>
              <th>sha256</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {builds.map((build, i) => (
              <tr key={i}>
                <td>
                  <Link href={`/build/${build.id}`}>{build.id}</Link>
                </td>
                <td>{build.origin}</td>
                <td>{build.rev}</td>
                <td>{build.status}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div class={cx("cell")}>
        <NewBuild />
      </div>
    </>
  );
}
