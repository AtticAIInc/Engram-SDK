#!/usr/bin/env node
"use strict";

const { execFileSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const binPath = path.join(__dirname, "bin", "engram");

if (!fs.existsSync(binPath)) {
  console.error("engram binary not found. Running install...");
  require("./install");
  if (!fs.existsSync(binPath)) {
    console.error("Installation failed. See above for details.");
    process.exit(1);
  }
}

try {
  execFileSync(binPath, process.argv.slice(2), { stdio: "inherit" });
} catch (err) {
  process.exit(err.status || 1);
}
