import { render } from "solid-js/web";
import { App } from "./App";
import "@money-tracker/design-system/styles.css";
import "./desktop.css";
render(() => <App />, document.getElementById("root")!);
