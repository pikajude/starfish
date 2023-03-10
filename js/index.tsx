import { Fragment, h, render } from "preact";
import Router, { Route } from "preact-router";
import "preact/debug";
import "preact/compat";

import Nav from "./components/Nav";
import Home from "./components/Home";
import Build from "./components/Build";
import Footer from "./components/Footer";
import cx from "./style";

const NotFound = () => {
  return <div>Page not found</div>;
};

const App = () => {
  return (
    <>
      <Router>
        <Route default component={Nav} />
      </Router>
      <div class={cx("grid-container")}>
        <div class={cx("grid-x")}>
          <Router>
            <Route path="/" component={Home} />
            <Route path="/build/:id" component={Build} />
            <Route default component={NotFound} />
          </Router>
          <Router>
            <Route default component={Footer} />
          </Router>
        </div>
      </div>
    </>
  );
};

render(<App />, document.body);
