# Private Picomatch security patch

This private fork applies the single-branch repeated-extglob ReDoS fix needed by [GHSA-fc5q-39xc-vjpf](https://github.com/advisories/GHSA-fc5q-39xc-vjpf). Upstream's latest published version is 4.0.7 and the advisory lists no patched release as of 2026-11-18. `4.0.8+secure-auth.1` identifies this repository's modified build; it is not a published upstream release.

The root development dependency and `$picomatch` override route all transitive consumers to this directory. Audit thresholds remain unchanged. The archive, patch, generated files, and license are committed so `npm ci` needs no patch-time lifecycle scripts or network fetch beyond ordinary dependency installation.

## Patch behavior

For a single, flat extglob branch containing an unescaped wildcard and no path separator or nested group, repeating the branch is redundant: its wildcard can consume intervening copies. The parser emits one branch for `+(branch)` and an optional branch for `*(branch)`, retaining the existing contextual regex guards. This removes the nested unbounded quantifier responsible for catastrophic backtracking in patterns such as `+(*a)` and `+(*)X`.

Escapes, quoted text, and bracket classes are respected when identifying the wildcard. Existing recursion limits, multi-branch handling, captures, platform options, and explicit opt-outs keep their existing code paths. The change does not claim to solve every possible regular-expression complexity issue in arbitrary user-supplied patterns.

## Source and reproducibility

- Canonical source: [official Picomatch 4.0.7 npm archive](https://registry.npmjs.org/picomatch/-/picomatch-4.0.7.tgz), retained unchanged under `upstream/`.
- SHA-512 integrity: `qcJu88Q2IWqJsDD529JKMdwGm/dvInW4HvQnRwiH9JtihJvzGOscDtHE3x1pBKeUOTysQ8kVmLnJ2kJu7yhcGA==`.
- Authored change: `security.patch`.
- Generated distribution: `index.js`, `posix.js`, and `lib/*.js`.
- Upstream MIT license: `LICENSE`, retained verbatim.

From the repository root, with dependencies installed:

```bash
node vendor/picomatch/scripts/generate.mjs
npm run check:vendor
npm run test:security
npm audit --workspaces --audit-level=high
```

Generation requires Node, the project's pinned TypeScript compiler, `tar`, and `patch`. It verifies the archive hash, extracts only the expected source files, applies the patch, emits JavaScript without explanatory comments, and verifies the upstream license. `--check` compares every generated file without modifying it. Edit the patch and regenerate; do not edit generated files directly.

## Validation and retirement

The security regression runs malicious patterns in a subprocess with a hard timeout. The same test times out against unmodified 4.0.7. Positive and negative match cases cover ordinary build-tool globs, repeated wildcards, escaped/class stars, and public helpers. The patched distribution also passed all 1,983 tests from the official 4.0.7 upstream test suite.

CI checks regeneration and the security regressions. The scheduled security workflow also fails when the upstream latest release changes from 4.0.7, requiring review rather than silently retaining a stale fork. When an upstream release resolves the advisory, remove the local dependency and override, regenerate the lockfile, remove the fork and its CI maintenance check, and rerun the full audit, regression tests, and application checks.
