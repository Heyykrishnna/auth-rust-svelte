const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const { test } = require("node:test");
const { join } = require("node:path");
const picomatch = require("..");

test("repeated wildcard extglobs reject adversarial input within an isolated process", () => {
  const modulePath = join(__dirname, "..");
  const source = `
    const assert = require('node:assert/strict');
    const picomatch = require(${JSON.stringify(modulePath)});
    for (const pattern of ['+(*a)', '*(*a)', '+(*)X', '*([ab]*)X', '+([[]*a)']) {
      assert.equal(picomatch(pattern)('a'.repeat(256) + '!'), false);
    }
  `;
  const result = spawnSync(process.execPath, ["-e", source], {
    timeout: 5000,
    encoding: "utf8",
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
});

test("repeated wildcard extglobs preserve valid matching behavior", () => {
  const cases = [
    ["+(*a)", "baaa", true],
    ["+(*a)", "baaab", false],
    ["*(*a)", "babaab", false],
    ["+(*)X", "anythingX", true],
    ["*([ab]*)X", "abbbX", true],
    ["*([ab]*)X", "X", true],
    ["*([ab]*)X", "cX", false],
  ];
  for (const [pattern, input, expected] of cases) {
    assert.equal(picomatch(pattern)(input), expected, `${pattern}: ${input}`);
  }
});

test("ordinary build-tool globs preserve their matching behavior", () => {
  const cases = [
    ["src/**/*.svelte", "src/routes/login/+page.svelte", true],
    ["src/**/*.{test,spec}.{js,ts}", "src/lib/api/auth.test.ts", true],
    ["src/**/*.{test,spec}.{js,ts}", "src/lib/api/auth.ts", false],
    ["**/*.ts", ".hidden/file.ts", false],
    ["**/node_modules/**", "apps/web/node_modules/vite/index.js", true],
    ["+(ab|cd)", "abcdab", true],
    ["+(ab|cd)", "abce", false],
    ["+([ab])", "abba", true],
    ["+([*])", "***", true],
    ["+(\\*)", "**", true],
    ["+(*(a)|*(b))", "abba", true],
  ];
  for (const [pattern, input, expected] of cases) {
    assert.equal(picomatch(pattern)(input), expected, `${pattern}: ${input}`);
  }
});

test("the public matcher helpers remain available", () => {
  assert.equal(picomatch.isMatch("auth.test.ts", "*.test.ts"), true);
  assert.equal(picomatch.makeRe("*.ts").test("auth.ts"), true);
  assert.equal(picomatch.scan("src/**/*.ts").base, "src");
  assert.equal(typeof picomatch.parse("*.ts").output, "string");
  assert.equal(require("../posix")("*.ts")("auth.ts"), true);
});
