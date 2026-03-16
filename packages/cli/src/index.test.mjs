import { spawnSync, spawn } from "node:child_process";
import { strict as assert } from "node:assert";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createServer } from "node:net";

const __dirname = dirname(fileURLToPath(import.meta.url));
const CLI = join(__dirname, "index.mjs");

let passed = 0;
let failed = 0;
const testQueue = [];

function test(name, fn) {
  testQueue.push({ name, fn });
}

// --- mock named-pipe server ---

function startMockServer(response) {
  return new Promise((resolve, reject) => {
    const tag = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const pipeName = `\\\\.\\pipe\\cmux-test-${tag}`;
    const received = [];

    const server = createServer((socket) => {
      let buf = "";
      socket.on("data", (chunk) => {
        buf += chunk.toString("utf8");
        const nl = buf.indexOf("\n");
        if (nl !== -1) {
          try {
            received.push(JSON.parse(buf.slice(0, nl)));
          } catch (error) {
            received.push({ __parseError: error.message, raw: buf.slice(0, nl) });
          }
          const next = typeof response === "function"
            ? response(received[received.length - 1])
            : response;
          const resp = next?.body ?? next;
          const keepOpen = next?.keepOpen ?? false;
          socket.write(JSON.stringify(resp) + "\n");
          if (!keepOpen) {
            socket.end();
          }
        }
      });
    });

    server.on("error", reject);
    server.listen(pipeName, () => resolve({
      pipeName,
      received,
      close: () => new Promise((r) => server.close(r)),
    }));
  });
}

// Async spawn — required for transport tests so the mock server's event loop
// can accept connections while the child process is running.
function runAsync(args, env) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [CLI, ...args], { env });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (d) => { stdout += d.toString("utf8"); });
    child.stderr.on("data", (d) => { stderr += d.toString("utf8"); });
    child.on("close", (code) => resolve({ status: code, stdout, stderr }));
  });
}

async function runViaPipe(args, serverResponse = { ok: true }) {
  const mock = await startMockServer(serverResponse);
  try {
    const result = await runAsync(args, { ...process.env, CMUX_PIPE: mock.pipeName });
    return { result, received: mock.received };
  } finally {
    await mock.close();
  }
}

// Synchronous run for validation-error tests — these call die() before
// touching the pipe, so blocking the event loop is fine.
function run(args) {
  const env = { ...process.env };
  delete env.CMUX_PIPE;
  return spawnSync(process.execPath, [CLI, ...args], { encoding: "utf8", env, timeout: 5000 });
}

// --- happy paths (transport actually exercised) ---

test("workspace create sends request over pipe and prints response", async () => {
  const { result, received } = await runViaPipe(
    ["workspace", "create", "--name", "inbox", "--root-dir", "/tmp", "--shell-profile", "bash"],
    { ok: true, workspaceId: "ws-123" },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  const resp = JSON.parse(result.stdout.trim());
  assert.equal(resp.ok, true);
  assert.equal(resp.workspaceId, "ws-123");
  assert.equal(received.length, 1);
  const req = received[0];
  assert.equal(req.protocolVersion, 1);
  assert.equal(req.type, "command");
  assert.equal(req.command, "workspace.create");
  assert.equal(req.payload.name, "inbox");
  assert.equal(req.payload.rootDir, "/tmp");
  assert.equal(req.payload.shellProfile, "bash");
  assert.ok(typeof req.id === "string" && req.id.length > 0);
});

test("pane split sends request over pipe and prints response", async () => {
  const { result, received } = await runViaPipe(
    ["pane", "split", "--workspace-id", "ws-1", "--pane-id", "p-1",
     "--new-pane-id", "p-2", "--orientation", "vertical", "--ratio", "0.5"],
    { ok: true },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  const req = received[0];
  assert.equal(req.command, "pane.split");
  assert.equal(req.payload.workspaceId, "ws-1");
  assert.equal(req.payload.paneId, "p-1");
  assert.equal(req.payload.newPaneId, "p-2");
  assert.equal(req.payload.orientation, "vertical");
  assert.equal(req.payload.ratio, 0.5);
  assert.equal(req.type, "command");
  assert.equal(req.protocolVersion, 1);
});

test("notify sends request over pipe without workspace-id", async () => {
  const { result, received } = await runViaPipe(
    ["notify", "--title", "Hi", "--body", "msg", "--level", "info"],
    { ok: true },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  const req = received[0];
  assert.equal(req.command, "notify.send");
  assert.equal(req.payload.title, "Hi");
  assert.equal(req.payload.body, "msg");
  assert.equal(req.payload.level, "info");
  assert.ok(!("workspaceId" in req.payload));
});

test("notify with optional --workspace-id", async () => {
  const { result, received } = await runViaPipe(
    ["notify", "--title", "Hi", "--body", "msg", "--level", "success", "--workspace-id", "ws-2"],
    { ok: true },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  assert.equal(received[0].payload.workspaceId, "ws-2");
});

test("pane split accepts horizontal orientation", async () => {
  const { result, received } = await runViaPipe(
    ["pane", "split", "--workspace-id", "w", "--pane-id", "p",
     "--new-pane-id", "p-2", "--orientation", "horizontal", "--ratio", "0.3"],
    { ok: true },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  assert.equal(received[0].payload.orientation, "horizontal");
  assert.equal(received[0].payload.ratio, 0.3);
});

test("session start sends request over pipe and prints response", async () => {
  const { result, received } = await runViaPipe(
    ["session", "start", "--workspace-id", "ws-1", "--pane-id", "pane-1",
     "--shell-profile", "cmd.exe", "--cols", "120", "--rows", "40"],
    { ok: true, result: { sessionId: "session:1" } },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  assert.equal(received[0].command, "session.start");
  assert.equal(received[0].payload.workspaceId, "ws-1");
  assert.equal(received[0].payload.paneId, "pane-1");
  assert.equal(received[0].payload.shellProfile, "cmd.exe");
  assert.equal(received[0].payload.cols, 120);
  assert.equal(received[0].payload.rows, 40);
});

test("session send-input sends request over pipe", async () => {
  const { result, received } = await runViaPipe(
    ["session", "send-input", "--session-id", "session:1", "--data", "echo hi\r\n"],
    { ok: true, result: { delivered: true } },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  assert.equal(received[0].command, "session.sendInput");
  assert.equal(received[0].payload.sessionId, "session:1");
  assert.equal(received[0].payload.data, "echo hi\r\n");
});

test("session status sends request over pipe", async () => {
  const { result, received } = await runViaPipe(
    ["session", "status", "--session-id", "session:1"],
    { ok: true, result: { sessionId: "session:1", status: "running" } },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  assert.equal(received[0].command, "session.getStatus");
  assert.equal(received[0].payload.sessionId, "session:1");
});

test("session resize sends request over pipe", async () => {
  const { result, received } = await runViaPipe(
    ["session", "resize", "--session-id", "session:1", "--cols", "132", "--rows", "48"],
    { ok: true, result: { resized: true } },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  assert.equal(received[0].command, "session.resize");
  assert.equal(received[0].payload.sessionId, "session:1");
  assert.equal(received[0].payload.cols, 132);
  assert.equal(received[0].payload.rows, 48);
});

// --- transport error paths ---

test("server ok=false exits non-zero with error message", async () => {
  const { result } = await runViaPipe(
    ["workspace", "create", "--name", "x", "--root-dir", "/", "--shell-profile", "sh"],
    { ok: false, error: { code: "conflict", message: "workspace already exists" } },
  );
  assert.notEqual(result.status, 0);
  assert.ok(result.stderr.includes("workspace already exists"), `stderr: ${result.stderr}`);
});

test("client resolves on first response line even if server keeps socket open", async () => {
  const { result } = await runViaPipe(
    ["notify", "--title", "Hi", "--body", "msg", "--level", "info"],
    { body: { ok: true, result: { queued: true } }, keepOpen: true },
  );
  assert.equal(result.status, 0, `exited ${result.status}: ${result.stderr}`);
  const resp = JSON.parse(result.stdout.trim());
  assert.equal(resp.ok, true);
  assert.equal(resp.result.queued, true);
});

test("pipe unreachable exits non-zero", async () => {
  const result = await runAsync(
    ["workspace", "create", "--name", "x", "--root-dir", "/", "--shell-profile", "sh"],
    { ...process.env, CMUX_PIPE: "\\\\.\\pipe\\cmux-no-such-pipe-xyz-99999" },
  );
  assert.notEqual(result.status, 0);
  assert.ok(result.stderr.includes("pipe unreachable") || result.stderr.includes("error"),
    `stderr: ${result.stderr}`);
});

// --- unique id per invocation (verified at transport level) ---

test("each invocation produces a unique id", async () => {
  const mock = await startMockServer({ ok: true });
  try {
    const args = ["workspace", "create", "--name", "x", "--root-dir", "/", "--shell-profile", "sh"];
    const env = { ...process.env, CMUX_PIPE: mock.pipeName };
    await runAsync(args, env);
    await runAsync(args, env);
    assert.equal(mock.received.length, 2, `expected 2 requests, got ${mock.received.length}`);
    assert.notEqual(mock.received[0].id, mock.received[1].id);
  } finally {
    await mock.close();
  }
});

// --- validation error paths (exit before connecting — no pipe required) ---

test("missing required flag exits non-zero with message", () => {
  const r = run(["workspace", "create", "--name", "x"]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("--root-dir"), `stderr: ${r.stderr}`);
});

test("invalid orientation exits non-zero", () => {
  const r = run(["pane", "split", "--workspace-id", "w", "--pane-id", "p",
    "--new-pane-id", "p-2", "--orientation", "diagonal", "--ratio", "0.5"]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("orientation"), `stderr: ${r.stderr}`);
});

test("ratio=0 exits non-zero (exclusive lower bound)", () => {
  const r = run(["pane", "split", "--workspace-id", "w", "--pane-id", "p",
    "--new-pane-id", "p-2", "--orientation", "vertical", "--ratio", "0"]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("ratio"), `stderr: ${r.stderr}`);
});

test("ratio=1 exits non-zero (exclusive upper bound)", () => {
  const r = run(["pane", "split", "--workspace-id", "w", "--pane-id", "p",
    "--new-pane-id", "p-2", "--orientation", "vertical", "--ratio", "1"]);
  assert.notEqual(r.status, 0);
});

test("ratio=1.5 exits non-zero", () => {
  const r = run(["pane", "split", "--workspace-id", "w", "--pane-id", "p",
    "--new-pane-id", "p-2", "--orientation", "vertical", "--ratio", "1.5"]);
  assert.notEqual(r.status, 0);
});

test("invalid level exits non-zero", () => {
  const r = run(["notify", "--title", "t", "--body", "b", "--level", "critical"]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("level"), `stderr: ${r.stderr}`);
});

test("session start rejects zero cols", () => {
  const r = run([
    "session", "start",
    "--workspace-id", "ws-1",
    "--pane-id", "pane-1",
    "--shell-profile", "cmd.exe",
    "--cols", "0",
    "--rows", "24",
  ]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("cols"), `stderr: ${r.stderr}`);
});

test("session resize rejects zero rows", () => {
  const r = run([
    "session", "resize",
    "--session-id", "session:1",
    "--cols", "80",
    "--rows", "0",
  ]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("rows"), `stderr: ${r.stderr}`);
});

test("session send-input requires data", () => {
  const r = run([
    "session", "send-input",
    "--session-id", "session:1",
    "--data",
  ]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("--data"), `stderr: ${r.stderr}`);
});

test("unknown session subcommand wins over flag parsing", () => {
  const r = run(["session", "bogus", "--flag"]);
  assert.notEqual(r.status, 0);
  assert.ok(r.stderr.includes("unknown subcommand"), `stderr: ${r.stderr}`);
});

test("unknown top-level command exits non-zero", () => {
  const r = run(["noop"]);
  assert.notEqual(r.status, 0);
});

test("no arguments exits non-zero", () => {
  const r = run([]);
  assert.notEqual(r.status, 0);
});

// --- run all tests sequentially ---

(async () => {
  for (const { name, fn } of testQueue) {
    try {
      await fn();
      console.log(`  ok  ${name}`);
      passed++;
    } catch (e) {
      console.error(`  FAIL  ${name}: ${e.message}`);
      failed++;
    }
  }
  console.log(`\n${passed} passed, ${failed} failed`);
  if (failed > 0) process.exit(1);
})();
