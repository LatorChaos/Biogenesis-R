# Movement primitive observations

`movement.tsv` records actual Java offset, cached bounds, growth, and friction behavior.
Unchanged source twice and the bundled JAR produced identical bytes.
The fixture contains 97 cases, 326 primitive calls, and 423 complete organism snapshots.
It does not execute complete game frames.

## Comparison contract

The lead specified the comparison contract before Rust movement comparisons.
Only `_m`, `_m1`, `_m2`, `_mass`, and `_I` permit the existing derived-geometry allowance.
For finite expected values, require finite actual values and `abs(actual-expected) <= 1e-10 * max(1,abs(expected))`.
Expected geometry NaN requires actual NaN; expected infinity requires identical signed infinity.
Geometry arrays retain equal lengths and element order.

All other floating values require exact raw bits, including velocities, angles, energy, photo coefficients, effects, and RNG observations.
Scalar NaN signs and payloads remain exact outside the five designated geometry fields.
Integer endpoints, centers, bounds, reactions, ARGB values, booleans, references, lengths, and ordering require exact equality.
The geometry allowance does not permit different growth decisions, cache branches, or endpoint casts.

## Execution and construction

Compile `tools/reference/MovementOracle.java` against each pinned backend.
Run `biogenesis.MovementOracle NEW_OUTPUT_DIRECTORY` with the isolated profile arguments recorded in the provenance.
The helper rejects omitted profile roots and existing output directories.
The existing runner bounds subprocesses and cleans owned process groups.

Each case resets effective `Utils.DEF_*` values before construction.
It sets mutation bounds `0..10000`, meta mutation `0`, clade complexity `25`, and the case's friction value.
Mutation colors use `GREEN_PROB=7`, with the other 45 selectable probabilities set to `0`.
`U` rows record all 147 mutable primitive settings.

Create a real `World(null)` and add configured gases through its public methods.
Use ordinary `Random` seed `827364` for the real constructor and paste at `(3500,3500)`.
The helper registers the successfully pasted object directly in the world list.
It does not call the UI-dependent `addOrganism` path.

Default genes are GREEN, BLUE, and CYAN, with lengths `12`, `14`, and `16`.
Their theta values are `0`, `0.375`, and `0.75`; branches are `-1`; all 36 reactions are `0`.
Default symmetry is `3`, with mirror `1`.
Mutation rate and activity are `0`; cream, fallow, spore, and adaptive-spore modifiers are `1`.
All fifteen constructor booleans are `false`.
Generation is `41`, clade is `root`, and update-clade remains `false`.
The supplied `input` genotype rows record each case's exact variations.

Single-segment cache cases use one GREEN gene, symmetry `1`, and mirror `0`.
Zero-mass cases supply a GREEN gene with length `0`.
Sporification and spore-birth cases use GREEN/SPORE/EYE, symmetry `1`, mirror `0`, and the recorded modifier.

## Actual actions and labelled inputs

`I` rows apply typed controls immediately before their matching `K` action.
Controls specify the exact original field name and literal value.
`gene0._length` calls the real gene setter.
Genotype controls assign the primary reference or call actual `GeneticCode.clone`, as explicitly recorded.
They do not simulate contact infection.

Energy-boundary cases derive a supplied energy from the actual source `getMass()/10` or `getMass()/12`.
They use `Math.nextDown` or `Math.nextUp` for adjacent floating inputs.
The helper records the resulting literal energy before calling `grow`.
This constructs boundary inputs without computing expected growth decisions or velocity formulas.

Several growth cases supply `hasGrown=7` as a labelled reset sentinel.
The actual method replaces it with the observed `-1`, `0`, or `1` result.
This sentinel does not claim a normal frame produces that initial value.

The helper invokes actual private `grow`, private `offset`, and private `rubbingFramesEffects` through reflection.
It calls actual public `symmetric` and `calculateBounds` directly.
After every action, it records all 153 declared instance fields plus Rectangle bounds.
It also records every genome/gene field and stable mutable-genome identity labels.
No expected movement, growth, bounds, or friction formula is implemented by the helper.

`offset_velocity` supplies the actual current `dx`, `dy`, and `dtheta` to original `offset`.
It preserves the data flow from zero-mass growth through friction, offset, and bound calculation.
It does not replace generated nonfinite values with fixture constants.

## Real spore-child cache cases

Six cases use actual `inherit` after a real pasted GREEN/SPORE/EYE parent.
They use modifiers `1`, `2`, `3`, `6`, `7`, and `10`.
Recorded parent inputs match the earlier independent birth oracle.
They supply energy `1000`, threshold `101`, early threshold `0`, one child, and no existing virus children.
Parent velocity is `(1.25,-0.5)` and spore time is `30`.

The helper builds an actual JTS tree containing the parent and resets the birth seed before the real child constructor.
The tree's parent bounds and reset seed are recorded under `F`.
After actual inheritance succeeds, the focus becomes that child.
No copied spore or inheritance logic initializes it.
All six initial 154-field states match the corresponding earlier `birth_variants.tsv` EYE cases exactly.

Each actual child retains its cached endpoints and bounds under `calculateBounds(false)`.
Each changes all four endpoint arrays and its bounds under the subsequent `calculateBounds(true)`.
These observations establish stale-cache behavior after real spore geometry rebuilding.

## Covered boundaries

| Cases | Observed boundary |
| --- | --- |
| Growth ratios `1..16` | Actual body reconstruction, mass/inertia changes, velocity rescaling, and maximum-size behavior. |
| Energy thresholds | One floating value below, equal to, and above mass/10 and mass/12. |
| Ages `7`, `8`, `15` | Eight-frame growth gating and separate shrink behavior. |
| Inactive ratios `15`, `16` | Newborn growth exception versus inactive older bodies. |
| Shrink ratios `1`, `14`, `15`, `16` | Shrink limits, unchanged cases, and inactive shrinking. |
| Spore versions `-1`, `-2`, `-3` | Growth conversion to `-1` and sporification eligibility. |
| Display frames `0`, `9`, `10` | Every infectious/blond combination for blue and cyan variants. |
| Saved/infected references | Saved-only retention, infected-plus-saved clearing, and shared-reference clearing. |
| Consecutive calls | A real shrink followed by sporification on the next explicit call. |
| Cache preservation | Changed gene geometry, unchanged angles, sentinel endpoints, and forced recomputation. |
| Floating centers | Fractional, negative, huge, NaN, and infinite offsets with actual Java casts. |
| Friction | Signed zero, strict tolerance boundaries, default/zero coefficients, subnormals, NaN, and infinity. |
| Zero mass/inertia | Generated NaN velocities continue through actual subsequent primitives. |

The sporification matrix includes blue modifier `7` and cyan modifier `10`.
Integer center sentinels remain distinct from supplied floating centers during bound calculation.
`offset` updates integer centers; `calculateBounds` does not normalize them.
Both positive/negative-zero angle pairs compare equal for cache selection.
NaN angles compare unequal and trigger recomputation.

The source initializes right and bottom bounds with positive `Double.MIN_VALUE`.
The negative-center cases preserve the resulting source rectangle behavior.
Offsets retain unnormalized angles and use Java's saturating floating-to-integer casts.

Zero-mass grow and shrink chains produce negative NaN bits `fff8000000000000` in velocities and angle.
Their final integer centers are zero and bounds are `(0,0,1,1)`.
The controlled NaN/infinite offset case produces bounds `(0,2147483647,1,-2147483648)`.
These are raw primitive observations, without claiming the subsequent border/death behavior ran.

## TSV schema

Rows are tab-separated ASCII, with one newline after each row.
Doubles use sixteen hexadecimal raw IEEE digits.
Null uses `~`; references use `present` or `~`; colors use eight ARGB digits.
Arrays use comma-separated scalar values; rectangles use `x,y,width,height`.

| Tag | Columns after the tag |
| --- | --- |
| `C` | Case name, setup `pasted`/`spore_birth`, seed. |
| `U` | Effective mutable primitive setting, Java type, value. |
| `I` | Action stage or `parent-input`, target field, Java type, supplied input. |
| `K` | Action stage, operation, optional arguments. |
| `O` | Stage, original organism field, Java type, observed value. |
| `G` | Stage, genome holder, original field or synthetic key, observed value. |
| `N` | Stage, genome holder, gene index, original field, observed value. |
| `R` | Stage, following long, bounded integer, and double bits. |
| `F` | Real-birth factory observation name and value. |
| `E` | Case terminator. |

Stage `input` contains the supplied primary genome.
Stage `initial` follows real construction, before the first movement action.
Actions use consecutive stages `step-0`, `step-1`, and onward.
Genome holders are `genotype`, `infected`, and `saved`.
Synthetic `_reference` labels preserve identity; synthetic `_nGenes` records length.

`K offset` has three raw double arguments.
`K bounds` has one boolean argument.
`symmetric`, `grow`, `rubbing`, and `offset_velocity` have no explicit arguments.
Array controls `x1`, `y1`, `x2`, and `y2` are supplied only in single-segment cache cases.
Reference controls use `shared` or `clone` as their explicit `GeneticCode` input mode.

`R` reads a serialized copy of the ordinary `Random`, without advancing the actual stream.
All 97 cases retain the same suffix across every movement primitive in their sequence.
Real inheritance can consume draws during setup; its resulting stream remains observable afterward.

## Counts and sources

| Record group | Count |
| --- | ---: |
| Cases | 97 |
| Complete organism snapshots | 423 |
| `symmetric` calls | 68 |
| `calculateBounds` calls | 166 |
| `grow` calls | 68 |
| Explicit `offset` calls | 8 |
| Velocity-driven `offset` calls | 2 |
| `rubbingFramesEffects` calls | 14 |
| Total rows | 157,137 |

Source anchors are relative to `../Colormod/src/biogenesis/` from the project root.

| Source | Behavior |
| --- | --- |
| `Organism.java:3071-3240` | Actual symmetric body reconstruction and geometry values. |
| `Organism.java:3577-3601` | Cached endpoint selection, bounds, and last-angle assignment. |
| `Organism.java:3607-3660` | Growth, shrink, velocity rescaling, and spore eligibility. |
| `Organism.java:5757-5760` | Floating offset, unnormalized angle, and integer-center casts. |
| `Organism.java:4932-5003` | Actual blue/cyan conversion and possible geometry rebuilding. |
| `Organism.java:17925-17932` | Multiplicative friction and strict tolerance comparison. |
| `Utils.java:112,719` | Default and effective friction values. |

The original 74 reference files retain hashes, sizes, and modes.
All seven prior TSV fixtures remain unchanged.
The provenance records exact compiler/runtime arguments, hashes, versions, and isolation checks.

This fixture excludes complete frames, activation, biological contacts, corridors, border/death handling, respiration, UI controls, and persistence.
It does not claim every internal branch or every spore genotype combination was exercised.
World scheduling and frame integration require separate evidence.
The provenance reports Java observations only; Rust comparison results remain separate.
