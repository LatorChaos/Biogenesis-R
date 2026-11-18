# Geometry reference observations

These fixtures observe actual Java geometry routines through explicit input cases.
The helper calls `Organism.create`, `symmetric`, `calculateBounds`, `Vector2D`, and `GeneticCode.draw`.
It does not advance organism frames, contacts, reproduction, or atmosphere.
Direct state assignments provide scenario inputs and preserve explicit phenotype/classification combinations.
Graphics tracing delegates real drawing to `BufferedImage` while recording production draw arguments.

Declared comparison rules precede fixture generation.

- Compare integer geometry, bounds, colors, reactions, segment order, and flags exactly.
- Compare physical lengths, rounded mass, radii, and polar values with `1e-12` absolute plus `1e-12` relative tolerance.
- Compare inertia with `1e-9` absolute plus `1e-12` relative tolerance.
- Compare preview scale with `1e-15` absolute plus `1e-12` relative tolerance.
- Compare NaN and signed infinity classifications explicitly.
- Investigate changed endpoint or branch outcomes regardless of floating tolerance.

Imported genomes can exceed 100 genes.
Finite signed and zero lengths retain source semantics.
Geometry rejects nonfinite input explicitly; legacy input files remain untouched.
The source's unweighted final inertia term and signed-int overflow remain intentional compatibility behavior.
Source classification reaches eye geometry through explicit `EyeState` values.
Opaque EYE matching must preserve full ARGB identity; translucent matches remain distinct colors.

The generated manifest records source/JAR commands, hashes, fixture counts, and remaining evidence limits.
