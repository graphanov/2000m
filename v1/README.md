# 2000m v1

`2000m.driver.v1` expands the benchmark to 28 ACs with quality scoring and stress/probe commands (`profile`, `replay`, `challenge`).

v1 is a harder benchmark track than v0, but it is still a credibility-hardening track until real contender evidence exists. Treat the conformance suite as the source of truth for scoring semantics.

## Local v1 smoke

Run the intentionally weak v1 stub through the v1 conformance suite:

```bash
cargo test -q --manifest-path v1/conformance/Cargo.toml
cargo run -q --manifest-path v1/conformance/Cargo.toml -- \
  v1/stub-driver \
  --json-out /tmp/2000m-v1-stub.json
```

Expected shape:

- `protocolVersion` is `2000m.driver.v1`.
- `totalAcs` is `28`.
- determinism passes for the stub.
- `passCount` is partial, never `28/28`.
- skipped ACs count as zero in the ranked composite denominator.

## Standalone composite

The canonical standalone v1 composite is implemented in `v1/conformance/src/main.rs`:

```text
pass_rate = passed_non_skipped_acs / total_acs
quality_avg = sum(ac.quality for all acs, including skipped as zero) / total_acs
composite_score = pass_rate * 70.0 + quality_avg * 0.3
```

The standalone scorer does **not** include LOC, convergence speed, generation count, human feel, or external result-repo trajectory fields. Those may appear as separate result columns, but they are not part of `compositeScore`.

## Performance and quality provenance

v1 has mixed evidence sources and should say so in result evidence:

- AC20 and AC26 use suite-recomputed deterministic evidence and are the strongest signals.
- AC18, AC23, and AC25 currently include host wall-clock timing around `step` calls. Those fields measure the scorer host plus JSON/IPC overhead and are host-bound.
- AC27 uses driver-reported full-window `profile` telemetry for the ranked performance gate. It is reproducible if the driver reports honestly, but it is not independently recomputed by the scorer.
- AC28 visual-polish event strings are probe/quality telemetry only; they do not create a mechanical visual-polish pass.
- Some quality subscore fields are constant rubric defaults rather than measured polish.

Do not present v1 timing or polish telemetry as portable, independently verified proof unless a later scorer version replaces those fields with suite-recomputed evidence or a canonical-host policy.
