# CI repair and validation — 2026-11-18

## Scope and original failures

Reviewed all three workflows, their build inputs, dependency manifests and lockfiles, integration-test setup, container entry points, and GitOps configuration at commit `8b6df1a36d09305a19b085206c90c48dae5c8543`.

The [latest original CI run](https://github.com/Heyykrishnna/auth-rust-svelte/actions/runs/44285894004) passed Rust and web checks but failed the API image build: Cargo 1.80 could not parse an edition-2024 dependency. The [matching security run](https://github.com/Heyykrishnna/auth-rust-svelte/actions/runs/44285893966) failed both dependency audits, secret detection, and container building. Deployment was consequently skipped.

## Repairs

- Updated the Rust builder to 1.96 and a minimal Alpine runtime using static musl binaries. Builds use the committed lockfile, compile both executables, and retain Cargo caches without suppressing errors. Both Dockerfile copies stay synchronized. The Compose health check uses wget from the runtime.
- Added a development image with cargo-watch, selected the API as the default Cargo executable, and corrected the frontend workspace mount and working directory in the Compose development override.
- Migrated to patched SvelteKit 3, Vite 8, Vitest 5, TypeScript 6, SQLx 0.9, Validator, and compatible telemetry dependencies. Removed unused OAuth dependencies that brought obsolete TLS and RSA implementations. Updated SQLx's optional SQLite dependency in the lockfile to its patched compatible release.
- Disabled stripping for build-time Rust dependencies to avoid the macOS proc-macro loader failure tracked in [rust-lang/rust#157750](https://github.com/rust-lang/rust/issues/157750). Production executable stripping stays enabled.
- Enabled all 19 ignored database tests in CI with their required database variable, Redis port, and sequential execution. Fixed the control-plane fixture's signing-key cleanup so a second test run does not inherit a different test encryption key.
- Added workflow and infrastructure validation, a frontend production build, independent container jobs, executable/startup checks, and GHCR publication for successful pushes to main.
- Enforced HIGH/CRITICAL container findings, retained scan reports on failure, generated SBOMs for both images, and scoped security-event permissions and fork uploads appropriately. Gitleaks exceptions identify only reviewed synthetic fixtures by exact fingerprint.
- Validated manual tags through environment variables, prevented shell interpolation of user input, lowercased registry names, checked image existence, gated automatic releases on successful CI and security runs for the same current-main commit, and serialized manifest updates.
- Corrected Argo CD repository URLs, allowed its child Application resources, bootstrapped the project first, stopped ignoring deployment image changes, and selected the actual webhook-worker executable.

## Verification

| Check | Outcome |
| --- | --- |
| `npm ci` | Passed |
| `npm run check` | Passed; zero Svelte errors/warnings in diagnostics |
| `npm run lint` | Passed |
| `npm test` | Passed: 37 tests, 8 files |
| `npm run build` | Passed |
| Isolated web Dockerfile install/build/runtime stages reproduced on host | Passed, including production-only install with zero audit findings |
| Built web HTTP smoke test | Passed: five public routes return 200; unauthenticated dashboard returns 303 |
| `cargo fmt --manifest-path apps/auth-api/Cargo.toml --check` | Passed |
| `cargo clippy --locked --manifest-path apps/auth-api/Cargo.toml --all-targets --all-features -- -D warnings` | Passed |
| `cargo test --locked --manifest-path apps/auth-api/Cargo.toml --all-targets --all-features` | Passed: 44 tests; database cases run separately |
| Database tests with `TEST_DATABASE_URL`, `--ignored --test-threads=1` | Passed: all 19, twice consecutively on the same database after fixture repair |
| `cargo build --locked --manifest-path apps/auth-api/Cargo.toml --release --bins` | Passed after the build-time stripping fix; both executables built |
| `cargo audit --deny unsound` | Passed; Chrono unmaintained warning remains visible |
| `npm audit --workspaces --audit-level=high` | Fails on the unpatched Picomatch advisory; 15 high findings including affected dependents, zero critical/moderate findings |
| `actionlint` 1.7.12 | Passed for all three workflows |
| Prettier checks of changed workflow/package/Compose configuration | Passed |
| Gitleaks 8.30.1, full Git history | Passed: 92 commits scanned, no unexcluded findings |
| Helm lint: default, development, production | Passed |
| Helm production template rendering | Passed |
| Kustomize: development and production | Passed |
| Production image rewrite in temporary overlay | Passed: API and worker share the API tag; frontend uses the web tag |
| Release script scenarios | Passed: 14 cases covering valid/invalid/injected tags, full SHA, registry casing, success/failure/pending gates, and stale commits |
| Docker Compose configuration | Passed with `config --quiet` |
| Dockerfile pair synchronization and `git diff --check` | Passed |

Database tests used isolated host PostgreSQL 18 and Redis because local Docker storage is read-only. CI retains PostgreSQL 16. Full Linux container builds and Trivy scans could not execute locally; the new GitHub runs are being checked for those results. The isolated host web build verifies the Dockerfile's dependency/layout/lifecycle sequence, not its Alpine image.

A later GitHub run successfully built both initial container images and passed Rust tests, web checks, workflow/infrastructure validation, secret detection, and the Rust audit. It exposed a TCP-reset race in the new web readiness probe, now fixed with bounded retries for all connection errors. It also exposed vulnerabilities in bundled global npm tooling and the Debian API runtime. Follow-up changes remove unused npm/Yarn from the web runtime and switch the API to static musl binaries in Alpine; these changes require a new scan.

SvelteKit currently prints a deprecation notice for the preserved `$lib` and other configured import aliases. They continue to pass type checking, builds, and runtime smoke tests. Kustomize also prints existing `commonLabels` deprecation notices; rendering succeeds.

## Remaining blockers and external prerequisites

The [Picomatch advisory](https://github.com/advisories/GHSA-fc5q-39xc-vjpf) lists no patched version as of this validation. The complete Node audit remains blocking, so automatic production promotion stays blocked. No audit thresholds or scanner failures were weakened to produce green checks. See [security.md](security.md) for the advisory and fixture-exclusion policy.

The README also has a pre-existing link to a missing `LICENSE` file. No legal license text was invented or changed as part of the workflow repair.

The implementation was committed as `7a2a52fa03a55ddb0536c636ec6cde3127e5668c` with subsequent follow-up commits on `main`. GitHub verification is being checked separately; registry publication, manifest push, and cluster rollout have not yet been verified. GHCR access, main-branch bot-write policy, real production domains/secrets, and Argo CD cluster health require the configured repository and deployment environment; see [deployment.md](deployment.md).
