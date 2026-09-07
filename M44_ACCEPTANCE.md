# Milestone 44 acceptance criteria

Milestone 44 closes the remaining M43 backend-to-CLI integration gap for exact-action strong fairness on direct finite-monitor verification.

Acceptance requires:

- `monitor ... --strong-fair-action <ACTION>` routes to the M43 strong-fair monitor backend for unbounded, product-bounded, and staged model/product analyses;
- no-fairness and weak-fairness monitor behavior remains unchanged;
- fairness constrains only infinite progress cycles and never excuses rejecting states or finite progress terminals;
- product/model cutoffs preserve stage-qualified `INCONCLUSIVE` semantics and exit 3;
- monitor violations retain exit 8 and satisfied analyses retain exit 0;
- duplicate, empty, missing, or mixed weak-plus-strong fairness declarations fail closed with exit 2;
- reports render strong-fair assumptions explicitly and separately from monitor evidence;
- built-binary regressions exercise lasso filtering, unrelated/taken fair actions, rejection/terminal precedence, cutoff provenance, and option validation;
- formatter, build, Clippy `-D warnings`, full tests, prior regression gates, exact-head mergeability, and post-merge main checks must all be green before the milestone is sealed.

This milestone adds no new fairness semantics, model checker traversal, wall-clock bound, performance claim, or combined weak-plus-strong fairness semantics.
