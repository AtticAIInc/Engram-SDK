#!/usr/bin/env node
"use strict";

const https = require("https");
const http = require("http");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const REPO = "AtticAIInc/Engram-SDK";

const PLATFORM_MAP = {
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-musl",
  "linux-arm64": "aarch64-unknown-linux-musl",
};

function getTarget() {
  const key = `${process.platform}-${process.arch}`;
  const target = PLATFORM_MAP[key];
  if (!target) {
    console.error(`Unsupported platform: ${key}`);
    console.error(
      `Engram supports: ${Object.keys(PLATFORM_MAP).join(", ")}`
    );
    process.exit(1);
  }
  return target;
}

function getVersion() {
  const pkg = JSON.parse(
    fs.readFileSync(path.join(__dirname, "package.json"), "utf-8")
  );
  return `v${pkg.version}`;
}

function download(url, redirects) {
  if (redirects === undefined) redirects = 0;
  if (redirects > 5) {
    return Promise.reject(new Error("Too many redirects"));
  }
  return new Promise((resolve, reject) => {
    const proto = url.startsWith("https") ? https : http;
    proto
      .get(
        url,
        { headers: { "User-Agent": "engram-npm-installer" } },
        (res) => {
          if (
            res.statusCode >= 300 &&
            res.statusCode < 400 &&
            res.headers.location
          ) {
            return download(res.headers.location, redirects + 1).then(
              resolve,
              reject
            );
          }
          if (res.statusCode !== 200) {
            reject(
              new Error(`Download failed: HTTP ${res.statusCode} from ${url}`)
            );
            return;
          }
          const chunks = [];
          res.on("data", (chunk) => chunks.push(chunk));
          res.on("end", () => resolve(Buffer.concat(chunks)));
          res.on("error", reject);
        }
      )
      .on("error", reject);
  });
}

async function main() {
  const target = getTarget();
  const version = getVersion();
  const binDir = path.join(__dirname, "bin");
  const binPath = path.join(binDir, "engram");

  // Skip if binary already exists and works
  if (fs.existsSync(binPath)) {
    try {
      execSync(`"${binPath}" version`, { stdio: "ignore" });
      return;
    } catch {
      // Binary exists but broken -- re-download
    }
  }

  const url = `https://github.com/${REPO}/releases/download/${version}/engram-${target}.tar.gz`;
  console.log(`Downloading engram ${version} for ${target}...`);

  try {
    const tarball = await download(url);
    fs.mkdirSync(binDir, { recursive: true });

    const tmpPath = path.join(binDir, "engram.tar.gz");
    fs.writeFileSync(tmpPath, tarball);
    execSync(`tar xzf "${tmpPath}" -C "${binDir}"`, { stdio: "inherit" });
    fs.unlinkSync(tmpPath);
    fs.chmodSync(binPath, 0o755);

    console.log(`Installed engram ${version} to ${binPath}`);
  } catch (err) {
    console.error(`\nFailed to install engram: ${err.message}`);
    console.error(`\nYou can install manually:`);
    console.error(
      `  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh`
    );
    console.error(`\nOr build from source:`);
    console.error(
      `  cargo install --git https://github.com/${REPO}.git engram-cli`
    );
    process.exit(1);
  }
}

main();
