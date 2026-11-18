# Birth observations

`birth.tsv` records actual `Organism.inherit` calls from unchanged Java source and the bundled JAR.
Two source executions and one JAR execution produced identical bytes.
The fixture contains 63 cases and 92,161 rows.
It covers genotype selection, energy allocation, placement, saved retries, initial spore effects, and successful clade integration.
It does not establish a complete reproduction or simulation-frame implementation.

## Reproduction

Compile `tools/reference/BirthOracle.java` against each pinned backend.
Use the exact compiler and runtime arguments in `birth.provenance.json`.
The helper requires a new output directory and four explicit isolated profile properties.
The existing runner bounds subprocesses and cleans their owned process groups.
Each successful run searches seeds below `1024` and records its selected concrete seeds.
Searches call actual `inherit`; they do not reproduce its selection or placement formulas.

## Explicit inputs

Each case resets effective `Utils.DEF_*` settings before construction.
The helper then sets mutation bounds to `0..10000`, meta mutation to `0`, and the recorded clade limit.
It sets `GREEN_PROB=7` and the other 45 selectable probabilities to `0`.
`U` rows record all effective mutable primitive settings, including unchanged defaults.
The one-hot distribution controls mutation colors without controlling random draws.

Construct a real `World(null)` and add the configured initial gases through its public methods.
Use setup seed `827364` for the real parent constructor and paste at `(3500,3500)`.
Register the pasted parent directly in the real organism list.
This avoids the unrelated UI-dependent `World.addOrganism` path.

Each supplied gene has length `12 + 2*index`, theta `0.375*index`, branch `-1`, and all 36 reactions `0`.
Parent colors, symmetry, mirror, mutation rate, and spore modifier are recorded in the initial `G`/`N` rows.
Parent activity is `0`; cream and fallow modifiers are `1`.
Adaptive-spore input is `1`.
Only selfish and dispersal constructor booleans vary; the other thirteen boolean inputs remain `false`.
Parent generation is `41`, clade is the case input, and update-clade initially remains `false`.

The explicit infection genome contains one white gene, generation `80`, and clade `virus`.
Its symmetry and mirror vary for the INDIGO threshold cases.
The explicit saved genome contains one blue gene, generation `91`, and clade `saved`.
Both use mutation rate `0`, ordinary modifiers, and false constructor booleans.
These reference assignments are controlled inputs, without claiming contact infection or a previous real failed birth.
Separate retry cases obtain their saved genome through an actual failed `inherit` call.

`pasted` genotype rows describe these supplied inputs.
`pasted` organism rows record actual construction effects.
Apply only recorded `I` parent-control rows after that snapshot.
The controls supply energy, reproduction thresholds, requested counts, velocity, and specified phenotype flags.
Do not derive supplied inputs from later child observations.

## Ordering and spatial state

The helper builds an actual JTS `STRtree` from the recorded live parent and blockers.
`T` rows record each inserted object's bounds and exact envelope coordinates.
No `World.time` call builds or updates the tree.
The explicit tree construction matches its source insertion boundary, without executing movement or contacts.

Immediately before child construction, reset the ordinary `Random` to the case seed.
Construct each child with the real `Organism(world)` constructor.
Record its pre-inherit state, then call actual `inherit(parent, first)`.
Record the result and post-inherit state before direct list registration.
On success, `L` appends that real child without updating the tree.
New children therefore remain absent from the frozen spatial index during later litter attempts.

The child's initial Rectangle has zero width and height.
The first candidate offset therefore uses those zero dimensions.
After a failed candidate, later candidates use the updated child dimensions.
Eight blocked-direction cases capture this distinction through final centers and bounds.
The helper does not instrument intermediate `calculateBounds` calls.

Blockers use real constructors and successful paste operations at `(1000,1000)`.
Their Rectangle bounds are then explicit collision inputs, recorded under `I` and `T`.
Their unchanged segment geometry is irrelevant to this rectangle-only placement query.
Blocked-direction bounds come from an actual unblocked source birth with the same case seed.
An independent final birth repeats that seed against the recorded blocker.
No placement formula constructs the blocker bounds.

Structural retry cases initially use a blocker covering `(3400,3400,200,200)`.
The first actual birth fails and supplies the parent's saved genome.
Remove the blocker, rebuild the real tree with only the parent, and retry with the continuing RNG stream.
The saved genome remains the exact same object across those attempts.
The retry has a new constructor orientation but consumes no repeated genetic mutation draws.

`R` rows peek through a serialized copy of the current ordinary `Random`.
They record following `nextLong`, `nextInt(10000)`, and raw `nextDouble` bits.
These observations do not advance the actual retry stream.
Every recorded state therefore retains its actual constructor, mutation, delay, and placement draw ordering.

## Concrete coverage

| Cases | Observed behavior |
| --- | --- |
| Ordinary, selfish, capped energy, early, infected | Distinct genotype sources and exact energy partitioning. |
| JADE and consumer exception | Infection energy caps apply only to the eligible parent phenotype. |
| INDIGO and JADE/INDIGO | Scale-dependent infection allocation and source arithmetic ordering. |
| INDIGO mirror boundaries | The same `3.49264705882353` energy succeeds without the special mirror and fails with it. |
| Saved override | The saved genotype is reused while energy follows the selected infection threshold. |
| Infected dead birth | Placement succeeds and initializes the source's aged brown-child state. |
| Single-child velocity and dispersal | Children inherit velocity or use their selected direction. |
| Eight ordinary direction cases | Every neighboring direction is selected by an actual successful birth. |
| Eight blocked-direction cases | A recorded initial placement is blocked, causing actual cyclic candidate advancement. |
| Litter reservations | Eight children reserve distinct directions; the ninth explicit attempt fails. |
| Five structural retry cases | Gene increase/decrease, symmetry increase/decrease, and equal randomized symmetry. |
| Clade limits `0`, `1`, `2` | Pending clade updates clear on success; marker limits control suffixes. |
| Spore modifiers `1..12` | Every modifier produces an actual successful birth and initial spore phenotype. |
| Negative spore time | First call wakes the parent and fails; the next actual call succeeds. |
| Infected spore ownership | Versions `1` and `3` each select own or infection genomes through real seeded gates. |
| Later infected litter member | The nonfirst branch uses the source's existing-virus-child path. |
| Negative infected energy | Both the actual second-gate abort and successful infection selection occur. |
| Existing virus delay abort | Failure resets source child counts before creating a genotype. |

The litter's ninth call is a deliberate exhaustion probe beyond the public reproduction loop's eight-child cap.
It does not claim that normal play requests nine offspring.
All twelve spore cases use adaptive-spore input `1`.
The adaptive-spore random gates at larger values remain outside this fixture.
The separate [variant fixture](birth_variants.README.md) records those gates and additional initial spore overrides.
The complex spore phenotype combinations remain covered only where these concrete cases exercise them.

For structural retries, failed children retain clade `root` and update-clade `true`.
Successful reuse appends `+2`, `-2`, `>2`, or `<2`, matching the actual allocated child identity.
Equal randomized symmetry clears the update flag without adding a marker.
With initial clade `root+3`, limits `0` and `1` suppress another marker.
Limit `2` permits the observed `root+3+2` result.

## TSV schema

Rows are tab-separated ASCII, with one newline after each row.
Null uses `~`; booleans use `true` or `false`.
Doubles use sixteen hexadecimal digits containing raw IEEE bits.
Colors use eight hexadecimal ARGB digits.
Arrays use comma-separated scalar values; empty arrays produce an empty final field.
Rectangles use `x,y,width,height`.

| Tag | Columns after the tag |
| --- | --- |
| `C` | Case name, operation, birth seed. |
| `U` | Effective Utils field, Java type, value. |
| `I` | Stage, supplied target or control, Java type, input value. |
| `O` | Stage, `parent` or `child`, original field name, Java type, observed value. |
| `G` | Stage, genome holder, original field or synthetic key, observed value. |
| `N` | Stage, genome holder, gene index, original gene field, observed value. |
| `A` | Stage, result or identity name, observed value. |
| `W` | Stage, world field, observed value. |
| `R` | Stage, following long, bounded integer, double bits. |
| `T` | Stage, `rebuild`, membership count. |
| `T` member | Stage, index, object label, bounds, minimum-X, maximum-X, minimum-Y, maximum-Y bits. |
| `L` | Stage, list operation, object label. |
| `E` | Case terminator. |

`O` records 58 selected birth, energy, geometry, phenotype, and bookkeeping fields per present organism.
`G`/`N` record every declared nonstatic genome and gene field, excluding the genome's array container.
Synthetic `_nGenes` records its length.
Synthetic `_reference` assigns stable local identity labels such as `g0`; null uses `~`.
Repeated labels identify the same mutable genome object within that case.
Holder names distinguish primary, infected, saved, child, and failed-child genomes.
No runtime identity hash or address enters the fixture.

`A` center-direction values classify observed center differences.
They identify a selected placement direction only when inheritance succeeds.
Early rejected children can retain zero centers and no genotype.
Retain those observations without claiming that an unexposed Rust discarded object matches them.

## Source anchors and limits

Paths below are relative to `../Colormod/src/biogenesis/` from the project root.

| Source | Relevant boundary |
| --- | --- |
| `Organism.java:756-774` | Child constructor orientation and supplied-genome reference behavior. |
| `Organism.java:1790-1978` | Inherited genotype selection, energy, early aborts, saved reuse, and mutation. |
| `Organism.java:1980-1993` | Geometry initialization and single/litter placement choice. |
| `Organism.java:1994-2495` | Successful child initialization and initial spore behavior. |
| `Organism.java:2497-2526` | Success-only clade updates and failed-placement saved references. |
| `Organism.java:3013-3063` | Real parent and blocker paste initialization. |
| `Organism.java:3336-3459` | Actual neighboring placement, reservation, velocity, identity allocation, and parent debit. |
| `World.java:539-566` | Pre-frame tree insertion and fixed iteration length. |
| `World.java:674-689` | Rectangle clearance queries through the actual tree. |
| `GeneticCode.java:764-788` | Structural mutations mark pending clade changes. |

All observed energy, gas, RNG, identity, metadata, and phenotype values require exact comparisons.
Integer centers and rectangles also require exact comparisons.
Only `_mass` and `_I` retain the previously declared geometry tolerance when a Rust consumer uses it.
That tolerance is `1e-10 * max(1, abs(expected))`.
It does not apply to birth energy or initial photo coefficients.

Direct list registration leaves source population and public reproduction statistics unchanged.
Those caller-owned effects require separate integration evidence.
This fixture does not execute `World.time`, public reproduction loops, contacts, transformation, networking, UI controls, or persistence.
It does not verify every internal branch or every spore phenotype combination.
The provenance records Java execution only; Rust comparison results remain separate evidence.
All four prior TSV fixtures remain unchanged.
