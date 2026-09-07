# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 54 — external multi-response temporal file CLI

**Status: integration candidate complete.**

Milestone 54 exposes the sealed M53 multi-response property documents together with M19 declarative model files through one built-binary route:

```text
fvlab temporal multi-file <model-path> <property-path> [fairness / model-budget / product-budget options]
```

The command reads and parses the model and property independently, compiles the textual response clauses to the canonical `MultiResponseProperty`, and delegates verification to the existing multi-response orchestration. It therefore preserves the established no-fair, weak-only, strong-only, canonical combined weak/strong fairness, product-only budget, and staged model-before-product budget semantics without adding another verifier, fairness evaluator, limit engine, or traversal.

The external contract remains explicit and fail closed: satisfied analyses exit `0`, direct multi-response violations exit `7`, unresolved bounded analyses exit `3`, and file/parser/option errors exit `2`. Fairness remains an external execution assumption rather than part of the property grammar. Finite pending terminals retain precedence over infinite fairness filtering, and bounded reports retain model/product cutoff provenance.

Executable evidence on exact candidate `8e4a60bf82a7e6fceb06f137735ad96c95f4bcba` passed rustfmt, all-target build, Clippy with `-D warnings`, the full test suite, all historical CLI regression gates, and the bounded state-property workflow. `tests/multi_temporal_cli.rs` adds **6 built-binary regressions** covering exact-clause no-fair violation, combined-fair satisfaction, finite pending-terminal precedence, model/product cutoff provenance, malformed/missing input, and explicit compatibility for the historical `temporal check` and `temporal file` routes.

M54 intentionally remains a two-file invocation boundary. It does not introduce a project format, machine-readable report schema, embedded fairness grammar, wall-clock proof bound, performance claim, or new temporal semantics.

## Next frontier — Milestone 55: reproducible verification job manifests

M54 makes an external multi-response run executable, but a complete invocation is still split across two file paths plus command-line fairness and model/product limit flags. M55 should make that invocation reproducible as one portable verification-job artifact while reusing the sealed M19/M53/M54 semantics.

Acceptance criteria:

- add a small deterministic typed/textual job manifest that names one declarative model file and one multi-response property file and may declare the existing weak/strong exact-action fairness assumptions plus existing model/product state, transition, and depth budgets;
- resolve relative model/property paths against the manifest file's directory so a checked-in job remains relocatable as a unit; absolute paths may remain explicit inputs but must not be invented or silently rewritten;
- validate singleton directives, repeated fairness declarations, numeric limits, unsupported directives, missing required model/property entries, and quoted-string syntax with deterministic fail-closed diagnostics; do not invoke a shell or treat manifest text as command-line source;
- compile the manifest into the same canonical M54 multi-response orchestration/options rather than duplicating verification, fairness, cutoff, or reporting semantics;
- expose one explicit binary route such as `fvlab temporal job <manifest-path>` with the existing `0` / `7` / `3` / `2` outcome contract;
- add differential integration tests showing manifest execution is result/report equivalent to the corresponding explicit M54 `multi-file` invocation for no-fair, mixed-fair, product-bounded, and staged model-before-product cases;
- add built-binary regressions for relative-path relocation, malformed/duplicate directives, missing referenced files, finite pending-terminal precedence, cutoff provenance, and historical M54/M18/M19 CLI compatibility;
- keep fairness outside the M53 property grammar and make no new temporal, fairness, traversal, performance, security, or wall-clock-bound claim.
