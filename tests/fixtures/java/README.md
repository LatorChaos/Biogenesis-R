# Curated Java oracle fixtures

These files are intentional test inputs for the Rust conversion, generated from the pinned original source.
Repeated source execution and bundled-JAR execution produced identical bytes.
[manifest.json](manifest.json) records their hashes and generation recipe.
[Reference execution](../../../docs/reference_oracles.md) records the measured scope and remaining gaps.

`core.jsonl` contains observations, exact hexadecimal floating-point strings, scenario inputs, and explicit unexercised paths.
The `.ser` files are trusted generated objects for compatibility verification, not arbitrary deserialization samples.
`genome.bgg` exercises the real genetic XML format; `invalid-symmetry.bgg` is its deliberate rejection case.

Do not edit expected values to match a Rust failure.
Regenerate only through the documented pinned reference recipe and inspect any changed hash before adoption.
These fixtures do not replace real controls, complete color-pair coverage, or final runtime acceptance.
