// Native launch acceptance for the built desktop host.
//
// The built host is launched three times, each against a throwaway directory:
//   1. a fresh location must create its window, load the bundled frontend, show
//      the missing-database chooser and create an empty database when asked to;
//   2. the same location must reopen that database without the chooser; and
//   3. a corrupt database must fail explicitly instead of becoming empty.
// The host records the evidence in `launch.json` when MONEY_TRACKER_LAUNCH_SMOKE
// is set, so this asserts on what a real windowed launch achieved rather than
// re-implementing startup.
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const binary = resolve(
  process.env.MONEY_TRACKER_DESKTOP_BIN ??
    `target/release/money-tracker-desktop${process.platform === "win32" ? ".exe" : ""}`,
);
if (!existsSync(binary)) {
  throw new Error(`build the desktop host first: ${binary}`);
}

function check(condition, message) {
  if (!condition) throw new Error(message);
}

function temporaryDirectory(label) {
  return mkdtempSync(join(tmpdir(), `money-tracker-launch-${label}-`));
}

function launch(directory, { action, status = 0 } = {}) {
  const report = join(directory, "launch.json");
  rmSync(report, { force: true });
  const env = { ...process.env, MONEY_TRACKER_LAUNCH_SMOKE: directory };
  if (action) env.MONEY_TRACKER_LAUNCH_SMOKE_ACTION = action;
  else delete env.MONEY_TRACKER_LAUNCH_SMOKE_ACTION;
  const result = spawnSync(binary, [], {
    env,
    stdio: ["ignore", "inherit", "inherit"],
    timeout: 120_000,
  });
  if (result.error) throw result.error;
  check(
    result.status === status,
    `launch exited with status ${result.status}, expected ${status}`,
  );
  check(existsSync(report), `the launch at ${directory} wrote no evidence`);
  const evidence = JSON.parse(readFileSync(report, "utf8"));
  console.log(
    `${action ?? "reopen"}:${directory} → ${JSON.stringify(evidence)}`,
  );
  return evidence;
}

const created = temporaryDirectory("create");
const corrupt = temporaryDirectory("corrupt");
let passed = false;
try {
  const first = launch(created, { action: "create" });
  check(first.ok === true, "the first launch did not report success");
  check(first.exit_code === 0, "the first launch reported a failure status");
  check(first.window?.created === true, "the first launch created no window");
  check(
    first.window.label === "main",
    `unexpected window label ${first.window.label}`,
  );
  check(
    first.window.title === "Money Tracker",
    `unexpected window title ${first.window.title}`,
  );
  check(first.window.visible === true, "the window was not visible");
  check(
    first.frontend_ready === true,
    "the bundled frontend never reached the host",
  );
  check(
    first.initial?.status === "missing",
    "a fresh location did not offer the chooser",
  );
  check(
    first.initial.legacy_database === null,
    "a fresh location reported a legacy database",
  );
  check(
    first.created?.status === "ready",
    "explicit creation did not open a database",
  );
  check(first.created.contract_version === 1, "unexpected contract version");
  check(
    first.created.holding_count === 0,
    "a new database did not start empty",
  );
  check(
    first.created.refresh_sources === 0,
    "an empty portfolio configured a refresh source",
  );
  check(
    first.created.reporting_currency === "USD",
    "unexpected default reporting currency",
  );
  check(
    first.database.exists === true,
    "explicit creation left no database file",
  );

  const second = launch(created);
  check(second.ok === true, "reopening the database did not report success");
  check(second.window?.created === true, "reopening created no window");
  check(second.frontend_ready === true, "reopening loaded no frontend");
  check(
    second.initial?.status === "ready",
    "an existing database did not open directly",
  );
  check(
    second.initial.contract_version === 1,
    "reopening changed the contract version",
  );
  check(second.initial.holding_count === 0, "reopening changed the holdings");
  check(second.created === null, "reopening ran the creation path");
  check(second.initial.snapshot_id >= 1, "reopening produced no snapshot");

  const blob = "not a money tracker database\n";
  const path = join(corrupt, "portfolio.duckdb");
  writeFileSync(path, blob);
  const third = launch(corrupt, { status: 1 });
  check(third.ok === false, "a corrupt database was not reported as a failure");
  check(third.exit_code === 1, "a corrupt database did not fail the launch");
  check(third.initial === null, "a corrupt database produced a startup state");
  check(third.database.exists === true, "the corrupt database disappeared");
  check(
    readFileSync(path, "utf8") === blob,
    "a corrupt database was overwritten or emptied",
  );
  check(third.failure !== null, "a corrupt database reported no reason");

  passed = true;
  console.log(
    `Native launch acceptance passed: window + frontend → chooser → empty database → reopen → corrupt rejected (contract ${second.initial.contract_version})`,
  );
} finally {
  if (passed) {
    rmSync(created, { recursive: true, force: true });
    rmSync(corrupt, { recursive: true, force: true });
  } else {
    console.error(
      `kept the failing launch evidence in ${created} and ${corrupt}`,
    );
  }
}
