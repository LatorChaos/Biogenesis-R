# Death and world-boundary observations

`death.tsv` records actual `Organism.die` and private `Organism.isInsideWorld` calls.
Unchanged source twice and the bundled JAR produced identical bytes.
The fixture contains 36 cases, 84 snapshots, and 48 explicit method calls.
Each snapshot records three real actors, their genome references, world counters, and complete statistics.

## Comparison contract

The contract was declared before Rust comparison.
Compare scalar, effect, gas, statistics, velocity, and RNG values using exact raw bits.
Compare endpoints, bounds, colors, reactions, identities, and ordering exactly.
Only organism geometry fields `_m`, `_m1`, `_m2`, `_mass`, and `_I` use the existing geometry allowance.
That allowance is `1e-10 * max(1, abs(expected))` for finite values.
NaN classification and signed infinities remain explicit.
No other floating tolerance or NaN substitution is permitted.

The Java `World` and `VisibleWorld` object observations are named collaborator exclusions.
They account for 504 `O` rows across 252 actor views.
The 36 `visible:selected` inputs describe source callback routing and remain explicit source-only metadata.
These exclusions do not exclude world effects, statistics, actor fields, or genome references.

## Execution and supplied inputs

Compile `tools/reference/DeathOracle.java` against each pinned backend.
Run `biogenesis.DeathOracle NEW_OUTPUT_DIRECTORY` with the private profile arguments recorded in the provenance.
The helper rejects omitted profile roots and existing output directories before accessing simulation state.
Four negative invocations cover both guards in canonical and diagnostic modes.
The existing runner bounds subprocesses and cleans their owned process groups.

Each case resets actual `Utils.DEF_*` values and supplies all 147 mutable primitive settings through `U` rows.
`INITIAL_ORGANISMS` is explicitly zero; no genesis or world frame runs.
Mutation settings are minimum `0`, maximum `10000`, and meta rate `0`.
GREEN weight is `7`; the other 45 selectable weights are zero.
The random seed is `827364`.
Real world gas-add methods receive the recorded initial gases.

Five labelled genomes provide constructor inputs and later identity observations.
`g0`, `g1`, and `g2` are explicit; `g3` and `g4` use actual `GeneticCode.clone` on `g0`.
All genomes have symmetry `1`, mirror `0`, mutation rate `0`, activity `0`, and four modifiers of `1`.
All fifteen constructor booleans are false; generation is `41` and clade is `root`.
Ordinary genes have length `12`, theta `0`, branch `0`, and default reactions.
Ordinary `g0` has GREEN, CYAN, and EYE genes; `g1` and `g2` each have one GREEN gene.
The color cases give `g0` 66 GREEN genes before the labelled current-color override.
The zero-mass case gives `g0` one zero-length GREEN gene.
Full `input` G/N blocks preserve every supplied field.

The helper constructs an actual headless `VisibleWorld(null)` and its real `World`.
It pastes selector `g2` at `(1000,1000)`, victim `g0` at `(3500,3500)`, and killer at `(2000,2000)`.
The killer usually holds `g1`; the shared-genotype case supplies `g0` instead.
Each successful paste uses actual `World.addOrganism(actor,null)`.
The helper selects the separate valid selector before the first add.
This avoids MainWindow callback branches without replacing any collaborator.
It preserves actual creation events, population events, IDs, gases, actor storage, and statistics.
The initial IDs are selector `0`, victim `1`, and killer `2`.
Initial storage and population both contain three actors.

`I` rows label raw-state controls before each actual operation.
They supply effects, velocities, references, counters, rectangles, and overflow seeds explicitly.
They do not supply computed expectations.
Full snapshots follow each actual call; no snapshot silently applies a control.
`R` peeks through a serialized random copy without consuming the live stream.
The fixture retains constructor consumption and following long, bounded integer, and double bits.

## Covered behavior

| Group | Actual observations |
| --- | --- |
| Killers | Null killer, distinct killer, and the victim itself. |
| Repetition | Three actual deaths for each killer mode, without an alive guard. |
| Current colors | All 46 selectable colors, 18 internal colors, unknown ARGB, and translucent EYE-colored ARGB. |
| Retention | Populated flags, coefficients, energy, geometry, lineage counters, thresholds, display state, and metadata. |
| References | Infection/saved clearing, equal-valued clones, shared genotype, and live statistics metadata propagation. |
| Wrapping | Distinct/self killer counters and world population cross signed integer limits. |
| Boundaries | Interior, exact walls, corners, outside positions, degenerate dimensions, and wrapped endpoint sums. |
| Motion | Exact velocity negation, signed zeros, NaN payloads, and boundary rotation reset. |
| Nested death | Alive origin rectangles with dimensions at most one, versus larger or already-dead actors. |
| Real zero mass | Actual grow, velocity offset, forced bounds, and boundary death on a valid placed zero-length genome. |

Death clears the recorded movement/reaction controls, photosynthesis, infection reference, and saved reference.
It retains unrelated flags and coefficients, including methanotrophy and colony photosynthesis.
Only exact current EYE color survives; other current colors become BROWN.
Translucent EYE-colored ARGB also becomes BROWN.
Death decrements population before increasing a nonnull killer's counter and delivering the real statistics event.
Repeated death repeats those side effects.
Shared statistics records observe the later explicit clade edit on the same genome.

Boundary checks treat equality with a wall as outside.
Their signed integer endpoint sums can wrap and make controlled extreme coordinates appear inside.
Negative dimensions alone do not force rejection in the interior.
An alive origin rectangle dies when both dimensions are at most one, including zero or negative dimensions.
An already-dead origin actor does not receive another nested death.
Boundary velocity negation preserves the observed signed-zero and NaN bit changes.

A real pasted zero-length body initially has bounds `(3500,3500,2,2)`.
Its actual grow call produces NaN velocities; actual offset propagates NaN centers.
Forced bounds then produces `(0,0,1,1)`; the real boundary call invokes death.
This case uses no raw allocation or copied geometry formula.
Direct zero-length paste alone does not produce the degenerate one-by-one rectangle.

## TSV schema

Rows are tab-separated ASCII, with one newline after each row.
Doubles use sixteen hexadecimal raw IEEE digits; arrays retain comma-separated order.
Null references use `~`; mutable genome identities use `g0` through `g4`.

| Tag | Columns after the tag |
| --- | --- |
| `C` | Case name, mode, seed. |
| `U` | Original setting, Java primitive type, supplied value. |
| `I` | Control stage, target, Java type, supplied value. |
| `A` | Actor role, genome label, paste X, paste Y. |
| `P` | Snapshot stage. |
| `K` | Stage, actual operation, victim role, argument. |
| `B` | Stage, `inside`, actual boolean return. |
| `O` | Stage, role, original field, reflected Java type, observed value. |
| `H` | Stage, role, `genotype`/`infected`/`saved`, genome identity or null. |
| `W` | Stage, world key, Java type, observed value. |
| `S` | Stage, original statistics field, reflected type, observed value. |
| `T` | Stage, actual statistics getter, reflected return type, observed value. |
| `G` | Stage, genome label, field, value. |
| `N` | Stage, genome label, gene index, field, value. |
| `R` | Stage, following long, integer bounded by `10000`, double bits. |
| `E` | Case terminator. |

Every actor view uses the existing complete 154-field organism schema, including inherited `bounds`.
Every statistics view records all 52 instance fields and all 49 public zero-argument getters.
Genome blocks include synthetic `_reference` and `_nGenes` observations.
GeneticCode object `O` fields record presence; `H` records their actual identities.
World/VisibleWorld object `O` fields record presence.

World keys are `_population`, `organism_count`, `corpses`, `_O2`, `_CO2`, `_CH4`, `NEXT_ID`, `nFrames`, `time`, and `organism_ids`.
`corpses` uses the actual source getter; IDs follow actual world-list order.
Control targets use `actor:<role>:<original field>`, `world:<field>`, `statistics:<field>`, or `genome:<label>:<field>`.
Setup genome recipes use `genome:<label>:construction` and `explicit` or `clone:g0`.
GeneticCode assignments supply existing `gN` labels directly.
Rectangle inputs use `x,y,width,height` and actual `setBounds`.

`die` arguments are `~`, `killer`, or `self`.
`isInsideWorld`, `grow`, and `offset_velocity` use `~`.
`offset_velocity` supplies the actor's current velocities to the actual offset method.
`bounds` supplies `true` to actual `calculateBounds`.

## Counts and limits

| Record group | Count |
| --- | ---: |
| Derived compared observations | 141,370 |
| Source O collaborator exclusions | 504 |
| Source selection metadata exclusions | 36 |
| Supplied G/N constructor checks | 43,320 |
| Other input/schema/command rows | 5,959 |
| Total rows | 191,189 |

Explicit calls comprise 23 deaths, 22 boundary checks, one grow, one offset, and one forced bounds calculation.
Nested deaths occur within boundary calls and are observable through actual world/statistics effects.
The separate [invalid-state diagnostic](death_invalid.README.md) preserves exceptions and uninitialized allocation behavior.
It supplies no passed valid-Rust actor claim.

Source behavior is pinned at `Organism.java:5201-5223` and `Organism.java:5735-5748`.
World population routing occupies `World.java:223-236`; creation/death routing occupies `World.java:747-768`.
The provenance records helper, fixture, source, compiled backend, JAR, runtime, and command evidence.
All 74 original files and ten earlier TSV fixtures remain unchanged.

These observations do not execute complete world frames, biological contacts, corpse decomposition, or UI callbacks.
They do not accept every caller's border placement, infection lifecycle, or save compatibility.
Rust comparison evidence remains separate from this Java observation provenance.
