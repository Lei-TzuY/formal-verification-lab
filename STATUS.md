# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 55 — reproducible verification job manifests

**Status: integration candidate complete.**

Milestone 55 packages one external multi-response verification invocation as a portable deterministic job manifest while reusing the sealed M19/M53/M54 ingestion, fairness, bounded/staged verification, reporting, and exit semantics.

A job names one declarative model file and one textual multi-response property file and may carry the existing repeated weak/strong exact-action fairness assumptions plus model/product state, transition, and depth budgets. `fvlab temporal job <manifest-path>` resolves relative model/property paths against the manifest directory, compiles the manifest to the canonical M54 `temporal multi-file` option surface, and does not invoke a shell, evaluate environment variables, or create a second verification engine.

The parser is deterministic and fail closed for missing required model/property directives, duplicate singleton directives, unsupported directives, malformed quoted strings/escapes, empty paths, invalid or overflowing non-negative integer limits, and trailing input. Canonical rendering preserves one stable manifest representation. Fairness semantics and resource-limit semantics remain owned by the existing canonical orchestration rather than being reimplemented in the manifest layer.

Executable evidence on exact candidate `564bece874f523444ab7686a7f326a16f5312721` passed CI #447: rustfmt, all-target build, Clippy with `-D warnings`, full tests, and all historical CLI regression gates. The same exact candidate passed Bounded state-property CLI workflow #297. `tests/verification_job_cli.rs` differentially compares job execution with the corresponding M54 `temporal multi-file` invocation for relative-path relocation, no-fair behavior, mixed weak/strong fairness, product and staged model-before-product cutoffs, finite pending-terminal precedence, malformed/missing inputs, and historical M54 compatibility. The temporary write-capable formatter workflow used during construction was removed before the candidate was validated.

M55 adds no new temporal or fairness semantics, fairness-by-default behavior, traversal engine, shell execution, wall-clock proof bound, performance claim, security claim, or machine-readable result schema.

## Next frontier — Milestone 56: machine-readable verification result envelope

M55 makes verification inputs reproducible, but automated consumers still have to parse the human-oriented line report and infer execution metadata from process context. M56 should add one stable machine-readable result envelope for verification-job execution while preserving the existing verifier/report authorities and exit semantics.

Acceptance criteria:

- add an explicitly versioned structured result type for `temporal job` execution containing at least schema version, overall outcome (`satisfied`, `violated`, `inconclusive`, `error` where appropriate), canonical verification status, model/property identity, applied weak/strong fairness assumptions, configured model/product budgets, model/product accounting, and bounded cutoff stage/reason when present;
- represent conclusive multi-response evidence structurally: violated clause plus either finite trace or stem/closed-cycle lasso, preserving the exact canonical state/action ordering already produced by M54 rather than recomputing witnesses;
- expose an explicit opt-in binary surface such as `fvlab temporal job <manifest-path> --format json`; the historical human report and `0` / `7` / `3` / `2` exits must remain unchanged when no structured format is requested;
- serialize deterministically with correct escaping and a documented schema version; prefer a small well-audited serialization dependency or a narrowly scoped serializer rather than ad-hoc invalid JSON construction;
- make malformed manifest/model/property input and verification errors machine-readable on the structured path without converting semantic failures into successful exits;
- add built-binary differential tests proving the structured envelope describes the same status, accounting, cutoff provenance, clause identity, and evidence as the canonical human/M54 execution for satisfied, finite violation, lasso violation, product cutoff, model cutoff, and malformed-input cases;
- add round-trip/schema-shape regressions and deterministic repeated-output checks suitable for CI consumers;
- do not add a second verifier, new temporal/fairness semantics, fairness-by-default behavior, wall-clock proof bounds, performance claims, or a generic RPC/server subsystem in this milestone.
