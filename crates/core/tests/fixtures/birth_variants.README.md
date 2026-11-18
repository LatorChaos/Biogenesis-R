# Initial birth spore variants

`birth_variants.tsv` extends the separate 63-case birth fixture without changing its bytes.
The unchanged source twice and bundled JAR produced identical variant outputs.
It contains 80 successful actual `inherit` calls and 480 complete organism snapshots.
This fixture targets initial spore overrides, without claiming a complete spore lifecycle.

## Comparison contract

The lead specified this contract before the first Rust variant comparison.
Only `_m`, `_m1`, `_m2`, `_mass`, and `_I` permit the existing derived-geometry tolerance.
For finite expected values, require finite actual values and `abs(actual-expected) <= 1e-10 * max(1,abs(expected))`.
Expected NaN requires actual NaN.
Expected infinity requires the same signed infinity.
Geometry arrays must retain equal lengths and element order.

Every other floating value requires exact raw bits.
This includes energy, effect coefficients, photo coefficients, gases, angles, and RNG observations.
Endpoints, bounds, reactions, ARGB values, integers, booleans, array lengths, and ordering require exact equality.
No broader visual, relative-error, or gameplay tolerance applies.
Rust comparison results remain separate evidence.

## Execution and inputs

Compile the shared `tools/reference/BirthOracle.java` against each pinned backend.
Run `biogenesis.BirthOracle NEW_OUTPUT_DIRECTORY variants` with the explicit isolated profile arguments recorded in the provenance.
The default mode still emits the unchanged `birth.tsv` fixture.
Both modes were rerun against source twice and JAR after the helper extension.

Reuse the explicit construction, typed controls, RNG peeks, and frozen-tree ordering in [birth.README.md](birth.README.md).
Every variant creates an actual pasted parent and calls actual `inherit` once.
All phenotype classifications come from supplied real genes and actual source effect calculations.
No forced child phenotype assignment or replacement collaborator supplies an expected result.

`O` records all 153 declared nonstatic organism fields plus inherited Rectangle bounds.
This matches the complete 154-field schema of the earlier organism-state fixture.
World, visual-world, and genome references use `present` or `~`.
Genome identity remains separately recorded through `G._reference` labels.
All other row formats remain identical to the default birth fixture.

Most parent genes retain lengths `12 + 2*index` and theta `0.375*index`.
The tiny-geometry case supplies length `0.25` for every gene.
The two long-spore cases supply length `100` for every gene.
Initial `pasted` genotype rows record these supplied values.

The ordinary controls supply parent energy `1000`, threshold `101`, one requested child, and spore time `30`.
The respiration fallback cases instead supply parent energy `2`.
Version-three movement cases supply the real genotype's dispersal flag.
Other explicitly recorded inputs follow the shared helper recipe.

Respiration cases set `Utils.SPORE_ENERGY_CONSUMPTION=5` through a recorded effective `U` setting.
They supply oxygen through `I(parent-controls,world.O2_after_paste,double,raw_bits)`.
The helper calls actual `World.decreaseO2` and `World.addO2` after the parent controls, before the `prepared` snapshot.
The supplied oxygen values are `10`, `1`, and `0`.
These cases exercise actual `useEnergy` calls, including partial oxygen consumption and remaining-energy fallback.

## Observed variants

| Group | Supplied genes or conditions |
| --- | --- |
| Version three | Plant/consumer, plant/consumer/moving, nonplant consumer, pure plant, moving plant, unclassified, and moving unclassified. |
| Version four | Consumer cases with both observed duration draws, plus pure-plant duration. |
| Version six mixed plants | Plant/consumer methanotroph, moving/still JADE consumers, pure-plant methanotroph, and plant/fungus. |
| Version six pure plants | DARK pretouch, GRAY killer, FALLOW, LIME, FOREST, DARKGRAY enhancement, JADE, JADE/BLOND, and ordinary green. |
| Version six fallback | Nonplant consumer, unclassified, GOLD, symmetry-three mirrors, and no sufficiently large segment. |
| Versions one and two | Pure plant, mixed consumer, consumer, unclassified, positive respiration, scarce oxygen, fallback, and long spores. |
| Eye rebuilds | GREEN/SPORE/EYE with every modifier from `1` through `12`. |
| Adaptive blue/cyan | Modifiers `7` and `10`, each with adaptive inputs `20`, `30`, `40`, `50`, `60`, and `70`. |

All 80 inheritance calls succeed.
Adaptive-gate rejection means conversion is skipped; it does not mean the birth failed.
At adaptive input `20`, conversion is unconditional, so only the applied case exists.
At input `70`, no conversion branch executes, so only the unapplied case exists.
Inputs `30`, `40`, `50`, and `60` each include actual applied and unapplied cases.
Seed searches remain bounded below `1024` and retain ordinary `Random` behavior.

The version-four consumer cases use seeds `0` and `1`.
Their observed reproduction times are `100` and `99`, respectively.
Their following RNG suffixes preserve the extra source duration draw.

With a five-unit respiration request and oxygen `10`, versions one and two retain energy `25` and fruit colors.
With oxygen `1`, they retain energy `29` and unconverted colors.
With oxygen `0`, they retain energy `30` and unconverted colors.
The two-unit fallback case consumes that remaining energy and produces fruit colors.
These outcomes come from source observations, without reproducing respiration formulas in the helper.

Version-six JADE/BLOND retains energy `30` through the parent's blond-specific selection branch.
The ordinary version-six examples retain `75.75000000000001` after their initial birth overrides.
The fixture preserves the exact bits, including that floating arithmetic result.
Complete snapshots expose geometry rebuilds, per-segment coefficients, and retained flags after each override.

## Counts and boundaries

| Record | Count |
| --- | ---: |
| Cases | 80 |
| Actual recorded inheritance attempts | 80 |
| Successful attempts | 80 |
| Organism snapshots | 480 |
| `O` observations | 73,920 |
| Total rows | 142,368 |
| Fixture bytes | 6,723,647 |

Source anchors remain those in the shared birth README.
Initial spore overrides execute within `Organism.java:2068-2495`.
`segmentsCreateEffects` supplies the real phenotype conditions before those overrides.
`blueSpore` and `cyanSpore` execute through the actual adaptive conversion branches.

This fixture excludes spore aging, hatching, death, restoration, contact effects, transformation, public reproduction loops, and simulation frames.
It does not assert exhaustive coverage of every phenotype combination or internal branch.
Caller-owned public birth statistics and UI integration require separate evidence.
The provenance reports Java observations and preservation checks, without claiming Rust parity.
