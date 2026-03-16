import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";

const cargoCandidates = [];

if (process.env.CARGO) {
  cargoCandidates.push(process.env.CARGO);
}

if (process.platform === "win32" && process.env.USERPROFILE) {
  cargoCandidates.push(path.join(process.env.USERPROFILE, ".cargo", "bin", "cargo.exe"));
}

cargoCandidates.push("cargo");

const args = process.argv.slice(2);

for (const candidate of cargoCandidates) {
  if (candidate !== "cargo" && !existsSync(candidate)) {
    continue;
  }

  const result = spawnSync(candidate, args, {
    stdio: "inherit",
    shell: false
  });

  if (!result.error) {
    process.exit(result.status ?? 0);
  }
}

console.error("Unable to find cargo. Install Rust or reopen the terminal so cargo is on PATH.");
process.exit(1);
