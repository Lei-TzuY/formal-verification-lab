# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 56 — machine-readable verification result envelope

**Status: integration candidate complete.**

Milestone 56 adds one explicitly versioned machine-readable result envelope to the reproducible M55 verification-job surface without introducing a second verifier or parsing the human report. `fvlab temporal job <manifest-path> --format json` loads the same job, declarative model, and textual multi-response property as the historical human path, builds the same canonical fairness/resource configuration, and dispatches through the existing unbounded, product-bounded, or staged multi-response engines.

Schema version 1 reports the overall outcome (`satisfied`, `violated`, `inconclusive`, or `error`), canonical verification status (`SATISFIED`, `VIOLATED`, or `INCONCLUSIVE`; input/execution errors have no verification status), model/property identity, canonicalized weak/strong fairness assumptions, configured model/product state/transition/depth budgets, model/product accounting, and stage-qualified cutoff reason when analysis is incomplete. Conclusive evidence is copied from the canonical result: violated clause plus either the exact finite trace or the exact stem/closed-cycle lasso, preserving state/action/pending-vector order without recomputing witnesses.

Structured execution preserves the established process contract: satisfied exits `0`, violated exits `7`, inconclusive exits `3`, and malformed manifest/model/property or verification setup errors produce a JSON error envelope and exit `2`. Omitting `--format json` preserves the M55 human report and historical exit behavior unchanged. The serializer emits deterministic compact JSON with centralized escaping for quotes, backslashes, control characters, and UTF-8 text and adds no runtime dependency.

Executable evidence on implementation candidate `4b53ac8ff83e6bb82a3db9651a10056e30d34dfa` passed CI #463: rustfmt, all-target build, Clippy with `-D warnings`, full tests, and all historical CLI regression gates. The same exact candidate passed Bounded state-property CLI workflow #313. M56 regressions cover typed-dispatch equivalence, exact lasso ordering, finite-terminal precedence, mixed weak/strong fairness, product and model cutoff provenance/accounting, canonical `INCONCLUSIVE` status, deterministic repeated JSON output, structured malformed-input errors, JSON escaping, and historical human-output compatibility.

M56 adds no new temporal/fairness semantics, fairness-by-default behavior, traversal engine, wall-clock proof bound, performance claim, security claim, or generic RPC/server subsystem.

## Next frontier — Milestone 57: deterministic verification job suites

M55 makes one verification invocation reproducible and M56 makes one result machine-readable. The next architectural gap is deterministic orchestration of a bounded collection of those already-sealed jobs so CI and regression consumers do not need an external script to reconstruct ordering, aggregate outcomes, or per-job result metadata.

Acceptance criteria:

- add a small deterministic suite manifest that contains an ordered list of verification-job manifest paths and resolves relative paths against the suite manifest directory; reject empty suites, duplicate singleton metadata, malformed quoting/escapes, and invalid paths fail closed;
- execute every suite entry through the existing M55/M56 job loader and typed execution dispatch rather than invoking a shell, reparsing human output, or creating a second verification engine;
- preserve each entry's complete M56 schema-v1 result envelope and input ordering in one explicitly versioned aggregate machine-readable result;
- define and test a deterministic aggregate outcome/exit policy suitable for CI: input/execution error takes precedence, otherwise any conclusive violation, otherwise any inconclusive job, otherwise all jobs satisfied; still retain every per-job result so aggregate status never erases evidence or cutoff provenance;
- expose one explicit opt-in CLI surface such as `fvlab temporal suite <suite-manifest> --format json`; do not silently reinterpret the historical single-job command;
- continue executing independent later suite entries after an individual semantic violation or inconclusive result so the aggregate is a complete deterministic batch report; fail-fast is reserved for suite-level syntax/loading errors that prevent defining the ordered job set;
- add built-binary differential tests comparing each suite entry with direct M56 single-job execution for satisfied, finite violation, lasso violation, product cutoff, model cutoff, mixed fairness, malformed job input, deterministic ordering, and repeated-output equality;
- place a reasonable explicit upper bound on suite entry count or otherwise fail closed before unbounded manifest-driven orchestration; this is an orchestration safety/resource guard, not a proof-performance claim;
- do not add concurrency, distributed execution, a daemon/RPC server, new verification semantics, fairness-by-default behavior, wall-clock proof bounds, performance claims, or hidden result suppression in this milestone.
