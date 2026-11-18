# Java threshold rounding observations

The fixture contains eleven actual `Math.round(int / 10)` results and nine real pasted FLOWER organisms.
The source and bundled Color Mod JAR produce identical bytes; both compiled helper classes also match.

`Math.round(int / 10)` first divides integers, then selects the float overload.
Converting the quotient through f32 changes some values above 16,777,216.
The narrow Rust helper preserves that conversion without changing division order.

FLOWER cases use one finite gene, symmetry one, and valid two-billion-unit worlds.
Actual paste performs construction, geometry, placement, gas spending, and initial effects.
Observed integers are reproduction delay, early reproduction threshold, and final reproduction threshold.
These output values are assertions only; the tagged gene length is the constructor input.
No post-call actor or genotype data constructs the Rust model.

Before correction, length 16,777,217 produced delay 16,777,217 instead of Java's 16,777,216.
The regression failed in `.workflow-local/build-runs/test-eo8b6l6z/commands.json`.
After correction, all 92 workspace tests passed in `test-jrzu3d0y`.
Both initial-effects sites share `java_math::round_integer_tenth` with the restoration implementation.

The producer is `tools/reference/ThresholdOracle.java`.
`thresholds.provenance.json` records helper/class/fixture hashes and the exact source/source/JAR receipt.
This fixture does not independently accept BLOND restoration, full contacts, rendered huge worlds, or frame scheduling.
