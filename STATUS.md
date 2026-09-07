# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 57 — deterministic verification job suites

**Status: integration candidate complete.**

Milestone 57 promotes the reproducible M55 job manifest and M56 machine-readable result envelope into one deterministic bounded batch-orchestration surface. A suite manifest contains one named suite plus an ordered list of verification-job manifest paths. Relative job paths resolve against the suite manifest directory, input order is preserved, duplicate job entries fail closed, quoted strings reuse deterministic escaping/position-aware diagnostics, and suite size is capped at `128` entries before execution.

Every suite entry is executed through the existing M56 `run_verification_job_json` path. The suite layer does not invoke a shell, parse human reports, rebuild fairness/resource options, or introduce a verifier. Each entry embeds the complete schema-v1 M56 result envelope unchanged, including status, fairness assumptions, model/product budgets and accounting, cutoff provenance, and finite/lasso evidence. The aggregate schema is explicitly versioned and deterministic.

Aggregate outcome precedence is `error > violated > inconclusive > satisfied`, with matching process exits `2 / 7 / 3 / 0`. A per-job violation, inconclusive result, or malformed job does not suppress independent later entries; only a suite-level load/parse failure that prevents defining the ordered job set produces a fail-fast aggregate error with no job results. The new `fvlab-suite <suite-manifest> --format json` binary is opt-in and does not reinterpret the historical `fvlab temporal job` surface.

Adding the second binary exposed a real integration regression: historical repository CI commands such as `cargo run -- run mutex-bug` became ambiguous once Cargo could choose between `fvlab` and `fvlab-suite`. M57 fixes that at the package boundary with `default-run = "fvlab"`, preserving the existing `cargo run -- ...` contract while keeping suite execution explicitly named.

Executable evidence on implementation candidate `0859b03c55acf007c13ceb922a0bfa634f3de246` passed CI #472: rustfmt, all-target build, Clippy with `-D warnings`, the complete test suite, and every historical CLI regression gate. The same exact candidate passed Bounded state-property CLI workflow #322. M57 built-binary differential coverage executes both `fvlab-suite` and the sealed M56 `fvlab temporal job ... --format json` path and verifies exact nested-envelope preservation for malformed jobs, satisfied jobs, finite pending-terminal violations, lasso violations, product-stage cutoffs, model-stage cutoffs, and mixed weak/strong fairness; it also locks deterministic input ordering, repeated-output equality, aggregate precedence, later-entry continuation after per-job errors, suite-level structured errors, parser round trips, duplicate rejection, and the 128-entry guard.

M57 adds no new verification, temporal, or fairness semantics; no fairness-by-default behavior; no concurrency or distributed execution; no shell orchestration; no wall-clock proof bound; no performance/security claim; and no daemon/RPC subsystem.

## Next frontier — Milestone 58: expectation-aware regression suites

M57 can deterministically report a heterogeneous batch, but its process status still reflects the jobs' **observed** verification outcomes. That is correct for analysis, yet it leaves a CI/regression gap: a deliberately negative job whose expected behavior is `VIOLATED`, `INCONCLUSIVE`, or even a structured job-level `ERROR` still makes the raw suite exit nonzero, so an external script must decide whether the observed result is actually the expected regression baseline.

Milestone 58 should make that expectation contract declarative and machine-checkable while preserving M57 raw aggregation unchanged.

Acceptance criteria:

- extend the suite specification with an explicit expected outcome per regression entry for the existing four M56 outcomes: `satisfied`, `violated`, `inconclusive`, or `error`; preserve deterministic ordering, quoted path handling, the 128-entry guard, and fail-closed duplicate/malformed expectation validation;
- keep the current M57 raw suite execution/result/exit semantics available unchanged; expectation checking must be an explicit opt-in mode rather than silently redefining what an ordinary suite means;
- execute every job through the same M56 structured runner and retain the complete **observed** schema-v1 job envelope verbatim; expectations classify results but never replace, edit, or recompute verification evidence, fairness, accounting, or cutoff provenance;
- produce a versioned deterministic regression result that records expected outcome, observed outcome, and match/mismatch for every entry in suite order, so a mismatch cannot be hidden by another matching job;
- define an unambiguous CI contract: all declared expectations matched exits `0`; at least one observed-vs-expected mismatch exits a dedicated nonzero regression-mismatch code; suite-level syntax/loading failures remain exit `2`; no expected negative verification outcome should require an external shell wrapper to count as a passing regression;
- require complete expectations in expectation-check mode and fail closed on missing/unknown expectations rather than inferring intent from file names, job exit codes, or previous runs;
- add built-binary differential regressions covering expected satisfied/finite violation/lasso violation/product cutoff/model cutoff/mixed-fairness/malformed-job matches, each corresponding mismatch class, multiple simultaneous mismatches, deterministic ordering, and repeated JSON equality, while proving the historical M57 raw suite output and exit behavior remain unchanged;
- do not introduce golden witness text matching, mutable baseline files, automatic expectation rewriting, concurrency/distributed execution, new verifier semantics, fairness-by-default behavior, wall-clock proof bounds, or performance claims in this milestone.
