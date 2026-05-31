# 2000m v1

`2000m.driver.v1` expands the benchmark to 28 ACs with quality scoring and stress/probe commands (`profile`, `replay`, `challenge`).

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

The standalone v1 composite is intentionally machine-reproducible: it does not use host wall-clock, LOC, OS state, or a synthetic convergence bonus. Visual-polish event strings remain probe/quality telemetry, not a mechanical pass by themselves.
