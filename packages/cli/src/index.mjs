#!/usr/bin/env node
import { randomUUID } from "node:crypto";
import { createConnection } from "node:net";

const PROTOCOL_VERSION = 1;
const PIPE_PATH = process.env.CMUX_PIPE ?? "\\\\.\\pipe\\cmux-win-v1";

function die(msg) {
  process.stderr.write(`error: ${msg}\n`);
  process.exit(1);
}

function parseFlags(args) {
  const flags = {};
  for (let i = 0; i < args.length; i++) {
    if (!args[i].startsWith("--")) die(`unexpected argument: ${args[i]}`);
    const key = args[i].slice(2);
    if (i + 1 >= args.length || args[i + 1].startsWith("--")) die(`--${key} requires a value`);
    flags[key] = args[++i];
  }
  return flags;
}

function requireFlags(flags, ...keys) {
  for (const k of keys) {
    if (flags[k] === undefined) die(`--${k} is required`);
  }
}

function buildRequest(command, payload) {
  return {
    protocolVersion: PROTOCOL_VERSION,
    id: randomUUID(),
    type: "command",
    command,
    payload,
  };
}

function sendRequest(request) {
  return new Promise((resolve, reject) => {
    const socket = createConnection(PIPE_PATH);
    let data = "";
    let settled = false;

    function finishWithError(error) {
      if (settled) return;
      settled = true;
      reject(error);
    }

    function finishWithData(raw) {
      if (settled) return;
      settled = true;
      socket.destroy();
      resolve(raw);
    }

    socket.setTimeout(5000);

    socket.on("connect", () => {
      socket.end(JSON.stringify(request) + "\n");
    });

    socket.on("data", (chunk) => {
      data += chunk.toString("utf8");
      const newline = data.indexOf("\n");
      if (newline !== -1) {
        finishWithData(data.slice(0, newline).trim());
      }
    });

    socket.on("end", () => {
      finishWithData(data.trim());
    });

    socket.on("timeout", () => {
      socket.destroy();
      finishWithError(new Error("connection timed out"));
    });

    socket.on("error", (err) => {
      finishWithError(err);
    });
  });
}

const [,, cmd, ...rest] = process.argv;

if (!cmd) die("usage: cmux-win <workspace|pane|notify> [subcommand] [flags]");

let request;

if (cmd === "workspace") {
  const [sub, ...flagArgs] = rest;
  if (sub !== "create") die(`unknown subcommand: workspace ${sub ?? "(none)"}`);
  const f = parseFlags(flagArgs);
  requireFlags(f, "name", "root-dir", "shell-profile");
  request = buildRequest("workspace.create", {
    name: f["name"],
    rootDir: f["root-dir"],
    shellProfile: f["shell-profile"],
  });

} else if (cmd === "pane") {
  const [sub, ...flagArgs] = rest;
  if (sub !== "split") die(`unknown subcommand: pane ${sub ?? "(none)"}`);
  const f = parseFlags(flagArgs);
  requireFlags(f, "workspace-id", "pane-id", "new-pane-id", "orientation", "ratio");
  if (!["vertical", "horizontal"].includes(f["orientation"]))
    die("--orientation must be vertical or horizontal");
  const ratio = parseFloat(f["ratio"]);
  if (isNaN(ratio) || ratio <= 0 || ratio >= 1)
    die("--ratio must be a number in (0, 1)");
  request = buildRequest("pane.split", {
    workspaceId: f["workspace-id"],
    paneId: f["pane-id"],
    newPaneId: f["new-pane-id"],
    orientation: f["orientation"],
    ratio,
  });

} else if (cmd === "notify") {
  const f = parseFlags(rest);
  requireFlags(f, "title", "body", "level");
  if (!["info", "success", "warning", "error"].includes(f["level"]))
    die("--level must be info, success, warning, or error");
  const payload = { title: f["title"], body: f["body"], level: f["level"] };
  if (f["workspace-id"]) payload.workspaceId = f["workspace-id"];
  request = buildRequest("notify.send", payload);

} else {
  die(`unknown command: ${cmd}`);
}

sendRequest(request).then((raw) => {
  let resp;
  try {
    resp = JSON.parse(raw);
  } catch {
    die("invalid response from server");
  }
  if (resp.ok === false) {
    die(resp.error?.message ?? resp.error ?? resp.message ?? "server returned ok=false");
  }
  process.stdout.write(JSON.stringify(resp) + "\n");
}).catch((err) => {
  die(`pipe unreachable: ${err.message}`);
});
