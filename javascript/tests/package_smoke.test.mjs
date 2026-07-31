import assert from "node:assert/strict";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const packageRoot = fileURLToPath(new URL("..", import.meta.url));

test("npm package contains and executes the JSON CLI", { timeout: 120_000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), "evmole-package-"));
  const environment = {
    ...process.env,
    npm_config_cache: join(directory, "npm-cache"),
  };
  delete environment.NODE_TEST_CONTEXT;
  const packed = spawnSync("npm", ["pack", "--json"], {
    cwd: packageRoot,
    encoding: "utf8",
    env: environment,
  });
  assert.equal(packed.status, 0, packed.stderr);
  const packJson = JSON.parse(packed.stdout);
  const { filename, files } = Array.isArray(packJson)
    ? packJson[0]
    : Object.values(packJson)[0];
  const paths = new Set(files.map((file) => file.path));
  for (const required of [
    "dist/agent_api.mjs",
    "dist/evmole_cli.mjs",
    "dist/schemas/request-v1.schema.json",
  ]) {
    assert.ok(paths.has(required), `missing ${required}`);
  }
  assert.ok(![...paths].some((path) => path.startsWith("tests/")));

  const tarball = join(packageRoot, basename(filename));
  const install = spawnSync(
    "npm",
    ["install", "--ignore-scripts", "--no-audit", "--no-fund", tarball],
    { cwd: directory, encoding: "utf8", env: environment },
  );
  assert.equal(install.status, 0, install.stderr);
  const executable = process.platform === "win32"
    ? join(directory, "node_modules", ".bin", "evmole.cmd")
    : join(directory, "node_modules", ".bin", "evmole");
  const result = spawnSync(executable, ["analyze", "--bytecode", "0x00"], {
    cwd: directory,
    encoding: "utf8",
    env: environment,
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout).input.byteLength, 1);

  const packageJson = JSON.parse(
    await readFile(join(directory, "node_modules", "evmole", "package.json"), "utf8"),
  );
  assert.equal(packageJson.exports["./agent"], "./dist/agent_api.mjs");
});
