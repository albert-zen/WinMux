import { spawnSync } from "node:child_process";
import { strict as assert } from "node:assert";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const CLI = join(__dirname, "index.mjs");

let passed = 0;
let failed = 0;

function run(args) {
  return spawnSync(process.execPath, [CLI, ...args], { encoding: "utf8" });
}

function test(name, fn) {
  try {
    fn();
    console.log(`  ok  ${name}`);
    passed++;
  } catch (e) {
    console.error(`  FAIL  ${name}: ${e.message}`);
    failed++;
  }
}

// --- happy paths ---

test("workspace create emits valid envelope", () => {
  const r = run(["workspace", "create", "--name", "inbox", "--root-dir", "/tmp", "--shell-profile", "bash"]);
  assert.equal(r.status, 0, `exited ${r.status}: ${r.stderr}`);
  const out = JSON.parse(r.stdout.trim());
  assert.equal(out.protocolVersion, 1);
  assert.equal(out.type, "command");
  assert.equal(out.command, "workspace.create");
  assert.equal(out.payload.name, "inbox");
  assert.equal(out.payload.rootDir, "/tmp");
  assert.equal(out.payload.shellProfile, "bash");
  assert.ok(typeof out.id === "string" && out.id.length > 0);
});

test("pane split emits valid envelope", () => {
  const r = run(["pane", "split", "--workspace-id", "ws-1", "--pane-id", "p-1", "--new-pane-id", "p-2", "--orientation", "vertical", "--ratio", "0.5"]);
  assert.equal(r.status, 0, `exited ${r.status}: ${r.stderr}`);
  const out = JSON.parse(r.stdout.trim());
  assert.equal(out.command, "pane.split");
  assert.equal(out.payload.workspaceId, "ws-1");
  assert.equal(out.payload.paneId, "p-1");
  assert.equal(out.payload.newPaneId, "p-2");
  assert.equal(out.payload.orientation, "vertical");
  assert.equal(out.payload.ratio, 0.5);
  assert.equal(out.type, "command");
  assert.equal(out.protocolVersion, 1);
});

test("notify emits valid envelope without workspace-id", () => {
  const r = run(["notify", "--title", "Hi", "--body", "msg", "--level", "info"]);
  assert.equal(r.status, 0, `exited ${r.status}: ${r.stderr}`);
  const out = JSON.parse(r.stdout.trim());
  assert.equal(out.command, "notify.send");
  assert.equal(out.payload.title, "Hi");
  assert.equal(out.payload.body, "msg");
  assert.equal(out.payload.level, "info");
  assert.ok(!("workspaceId" in out.payload));
});

test("notify with optional --workspace-id", () => {
  const r = run(["notify", "--title", "Hi", "--body", "msg", "--level", "success", "--workspace-id", "ws-2"]);
  assert.equal(r.status, 0, `exited ${r.status}: ${r.stderr}`);
  const out = JSON.parse(r.stdout.trim());
  assert.equal(out.payload.workspaceId, "ws-2");
});

test("pane split accepts horizontal orientation", () => {
  const r = run(["pane", "split", "--workspace-id", "w", "--pane-id", "p", "--new-pane-id", "p-2", "--orientation", "horizontal", "--ratio", "0.3"]);
  assert.equal(r.status, 0, `exited ${r.status}: ${r.stderr}`);
  const out = JSON.parse(r.stdout.trim());
  assert.equal(out.payload.orientation, "horizontal");
  assert.equal(out.payload.ratio, 0.3);
});

// --- error paths ---

test("missing required flag exits non-zero with message", () => {
  const r = run(["workspace", "create", "--name", "x"]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("--root-dir"), `stderr: ${r.stderr}`);
});

test("invalid orientation exits non-zero", () => {
  const r = run(["pane", "split", "--workspace-id", "w", "--pane-id", "p", "--new-pane-id", "p-2", "--orientation", "diagonal", "--ratio", "0.5"]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("orientation"), `stderr: ${r.stderr}`);
});

test("ratio=0 exits non-zero (exclusive lower bound)", () => {
  const r = run(["pane", "split", "--workspace-id", "w", "--pane-id", "p", "--new-pane-id", "p-2", "--orientation", "vertical", "--ratio", "0"]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("ratio"), `stderr: ${r.stderr}`);
});

test("ratio=1 exits non-zero (exclusive upper bound)", () => {
  const r = run(["pane", "split", "--workspace-id", "w", "--pane-id", "p", "--new-pane-id", "p-2", "--orientation", "vertical", "--ratio", "1"]);
  assert.notEqual(r.status, 0);
});

test("ratio=1.5 exits non-zero", () => {
  const r = run(["pane", "split", "--workspace-id", "w", "--pane-id", "p", "--new-pane-id", "p-2", "--orientation", "vertical", "--ratio", "1.5"]);
  assert.notEqual(r.status, 0);
});

test("invalid level exits non-zero", () => {
  const r = run(["notify", "--title", "t", "--body", "b", "--level", "critical"]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("level"), `stderr: ${r.stderr}`);
});

test("unknown top-level command exits non-zero", () => {
  const r = run(["noop"]);
  assert.notEqual(r.status, 0);
});

test("no arguments exits non-zero", () => {
  const r = run([]);
  assert.notEqual(r.status, 0);
});

// --- output sanity: id is unique per invocation ---

test("each invocation produces a unique id", () => {
  const args = ["workspace", "create", "--name", "x", "--root-dir", "/", "--shell-profile", "sh"];
  const a = JSON.parse(run(args).stdout.trim());
  const b = JSON.parse(run(args).stdout.trim());
  assert.notEqual(a.id, b.id);
});

console.log(`\n${passed} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
