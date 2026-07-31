import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const cli = fileURLToPath(new URL("../dist/evmole_cli.mjs", import.meta.url));
const { version: packageVersion } =
  createRequire(import.meta.url)("../package.json");
const fixturePath = (name) =>
  new URL(`../../agent/tests/fixtures/${name}`, import.meta.url);
const fixture = async (name) => (await readFile(fixturePath(name), "utf8")).trim();
const childEnvironment = { ...process.env };
delete childEnvironment.NODE_TEST_CONTEXT;

function run(args, options = {}) {
  return spawnSync(process.execPath, [cli, ...args], {
    encoding: "utf8",
    env: childEnvironment,
    ...options,
  });
}

function runWithOpenStdin(args, input) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [cli, ...args], {
      env: childEnvironment,
      stdio: ["pipe", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.stdin.on("error", () => {});
    child.stdin.write(input);

    const timeout = setTimeout(() => {
      child.kill();
      reject(new Error("CLI waited for stdin despite explicit input."));
    }, 2_000);
    child.once("error", (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    child.once("close", (status) => {
      clearTimeout(timeout);
      resolve({ status, stdout, stderr });
    });
  });
}

test("analyzes argument input with pure compact JSON stdout", async () => {
  const result = run([
    "analyze",
    "--bytecode",
    await fixture("minimal-runtime.hex"),
  ]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stderr, "");
  assert.doesNotMatch(result.stdout.trim(), /\n/);
  assert.equal(JSON.parse(result.stdout).schemaVersion, 1);
});

test("supports stdin and pretty JSON", async () => {
  const result = run(
    [
      "analyze",
      "--include",
      "selectors,arguments,stateMutability",
      "--pretty",
    ],
    { input: await fixture("functions-runtime.hex") },
  );
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /\n  "schemaVersion"/);
  assert.ok(JSON.parse(result.stdout).analysis.functions.length > 0);
});

test("supports file paths containing spaces", async () => {
  const directory = await mkdtemp(join(tmpdir(), "evmole cli "));
  const path = join(directory, "runtime code.hex");
  await writeFile(path, `${await fixture("minimal-runtime.hex")}\n`);
  const result = run(["analyze", "--file", path]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout).input.byteLength, 1);
});

test("explicit input does not wait for open stdin", async () => {
  const result = await runWithOpenStdin(
    ["analyze", "--bytecode", "0x00"],
    "unused stdin remains open",
  );
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout).input.byteLength, 1);
});

test("rejects conflicting options and invalid usage with exit 2", () => {
  const conflicting = run([
    "analyze",
    "--bytecode",
    "0x00",
    "--file",
    "runtime.hex",
  ]);
  assert.equal(conflicting.status, 2);
  assert.equal(JSON.parse(conflicting.stdout).error.code, "INVALID_REQUEST");
  assert.equal(conflicting.stderr, "");

  const unknown = run(["analyze", "--wat"]);
  assert.equal(unknown.status, 2);
  assert.equal(JSON.parse(unknown.stdout).error.code, "INVALID_REQUEST");
});

test("uses documented request and size exit codes", () => {
  const invalid = run(["analyze", "--bytecode", "0x0"]);
  assert.equal(invalid.status, 3);
  assert.equal(JSON.parse(invalid.stdout).error.code, "INVALID_BYTECODE");

  const tooLarge = run(
    ["analyze"],
    { input: `0x${"00".repeat(1024 * 1024 + 1)}` },
  );
  assert.equal(tooLarge.status, 4);
  assert.equal(JSON.parse(tooLarge.stdout).error.code, "BYTECODE_TOO_LARGE");
});

test("prints help, version, and schemas", () => {
  const help = run(["--help"]);
  assert.equal(help.status, 0);
  assert.match(help.stdout, /Usage: evmole/);

  const version = run(["--version"]);
  assert.equal(version.status, 0);
  assert.equal(version.stdout, `${packageVersion}\n`);

  const schemas = run(["schema"]);
  assert.equal(schemas.status, 0, schemas.stderr);
  const parsed = JSON.parse(schemas.stdout);
  assert.equal(parsed.schemaVersion, 1);
  assert.equal(parsed.request.title, "EVMole agent analysis request v1");
});
