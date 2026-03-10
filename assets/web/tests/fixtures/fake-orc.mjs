#!/usr/bin/env node

import { spawn } from "node:child_process";

const args = process.argv.slice(2);
const command = args[0] ?? "";

if (command === "auto_add_function") {
  const lines = ["stage: analyze", "stage: plan", "stage: implement", "stage: verify"];
  let index = 0;
  const timer = setInterval(() => {
    if (index >= lines.length) {
      clearInterval(timer);
      process.stdout.write("auto completed\n");
      process.exit(0);
      return;
    }
    process.stdout.write(`${lines[index]}\n`);
    index += 1;
  }, 450);
} else {
  const realBin = (process.env.ORC_REAL_BIN ?? "").trim();
  if (!realBin) {
    process.stderr.write(`unsupported fake orc command: ${command}\n`);
    process.exit(1);
  }
  const child = spawn(realBin, args, {
    stdio: "inherit"
  });
  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? 1);
  });
  child.on("error", (error) => {
    process.stderr.write(`${String(error)}\n`);
    process.exit(1);
  });
}
