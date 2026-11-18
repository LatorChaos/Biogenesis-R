# Reproduction caller observations

`reproduction.tsv` records actual calls to all eight source reproduction methods.
Source twice and the bundled JAR produce identical bytes.
The five compiled helper classes also match across both backends.
The fixture contains 73 cases, 222 snapshots, 149 explicit operations, and 94 discovered newborns.

## Comparison contract

The comparison contract predates Rust replay.
Compare scalar, effect, photo, gas, statistics, velocity, endpoint, color, reaction, identity, and RNG observations exactly.
Only organism fields `_m`, `_m1`, `_m2`, `_mass`, and `_I` retain the existing geometry allowance.
For finite geometry values, use `1e-10 * max(1, abs(expected))`.
Check geometric NaN classification and signed infinity explicitly.
No other floating tolerance or NaN substitution applies.

Original constructor inputs remain separate from later observations.
Only `input` G/N records and labelled setup construction recipes supply original genomes.
Never use newborn or candidate observations as Rust constructor inputs.
Discover newborns through actual object identity, not numeric ID uniqueness.
Preserve genotype references through original actors, newborns, saved codes, and statistics.

## Execution and setup

Compile `tools/reference/ReproductionOracle.java` against each pinned backend.
Run `biogenesis.ReproductionOracle NEW_OUTPUT_DIRECTORY reproduction` with the isolated profile arguments recorded in provenance.
The helper rejects missing profile roots and existing output directories before simulation access.
It writes its new TSV with `CREATE_NEW`.
The existing runner bounds processes and cleans their owned process groups.

Each case resets source defaults and records all 147 mutable primitive settings.
`INITIAL_ORGANISMS` is zero; no genesis or complete world frame runs.
Initial oxygen is `1000000`; other initial gases retain their source defaults.
Mutation bounds are `0..10000`; meta mutation is zero.
GREEN weight is seven; the other selectable weights are zero.
The setup seed is `827364`.
Labelled settings and controls override those defaults explicitly.

Five original genomes receive stable identities `g0` through `g4`.
`g0` is the victim's supplied code; `g1` and `g2` supply killer and selector codes.
`g3` supplies an independent infection code; `g4` is an actual clone of `g0`.
Full `input` G/N blocks record every supplied property and gene field.
Construction recipes distinguish explicit construction from actual `GeneticCode.clone`.

Three real organisms are pasted and added through actual source methods.
The selector occupies `(1000,1000)`, victim `(3500,3500)`, and killer `(2000,2000)`.
A real separate selector avoids MainWindow callback branches.
Selection metadata remains a named source-only exclusion.
The killer also supplies the real peer or blocker rectangle for placement tests.

`build_tree` creates a fresh real JTS tree from the explicit actor list.
It records each actual inserted envelope, then explicitly builds the tree.
Several cases replace the peer's rectangle through labelled inputs.
Newborns append to the actual world list without entering that frozen tree.
Queries preserve source JTS result order.

## Covered behavior

| Group | Actual source outcomes |
| --- | --- |
| Callers | All eight methods execute successful and blocked attempts. |
| Litter placement | Four looping callers create eight children or stop after partial placement. |
| Silver thresholds | Both threshold families, strict ties, negative totals, maximum totals, fungus, and infectious flags. |
| Accounting | Wrapping lifetime children update before synchronous statistics. |
| Payments | Yellow, auburn, and blond partial oxygen payments; ordered front costs; one blond payment across eight births. |
| Plant blond | Ordinary, C4, and late branches; wrapped integer energy differences before multiplication. |
| Virus conversion | Fractional compound conversion and saturation to the Java integer limit. |
| Early virus | Exact inserted-child caching, countdown reset, and version-five cache clearing. |
| Spores | Actual blue/cyan preparation, infection clearing, saved-code retention, and forced dormancy after adaptive rejection. |
| Foreign virus | Once-only auburn cost across two real attempts. |
| Dynamic limits | Inheritance reduces the caller's child limit before its loop finishes. |
| Aliases | Shared whole-parent genotype and ordinary spore guards for versions one through six. |

The dynamic-limit case uses an explicitly empty frozen tree.
This prevents placement failure from hiding the loop-bound behavior.
The source caller sets seven children; first self-inheritance reduces that count to six.
The actual caller completes six births and stops at the changed limit.
The helper does not calculate an expected child count.
A bounded search selects the actual self-inheritance outcome.

Spore caller construction precedes infection and saved-reference clearing in the source.
The fixture records surrounding references, generated children, and exact subsequent RNG state.
It does not inject a callback inside the opaque caller.
Intermediate ordering also relies on the pinned source inspection.

Explicit operations comprise 74 caller invocations, 73 tree builds, and two source dormancy preparations.
The eight-child cases produce eight actual births each.
Partial looping cases produce four, four, four, and five births respectively.
These outcomes come from the original methods, without copied placement formulas.

## Shared TSV protocol

Rows are tab-separated ASCII with a final newline.
Doubles use sixteen raw IEEE hexadecimal digits.
Arrays and lists preserve comma-separated order.
Null references use `~`; empty lists use an empty value.

| Tag | Columns after the tag |
| --- | --- |
| `C` | Case name, mode, setup seed. |
| `U` | Original setting, primitive type, supplied value. |
| `I` | Stage, explicit target, Java type, supplied value. |
| `A` | Original role, supplied genome label, paste X, paste Y. |
| `K` | Stage, actual operation, target role, argument. |
| `J` | Stage, inserted role, actual X, Y, width, height. |
| `D` | Stage, discovered newborn role, actual ID, genotype identity. |
| `P` | Snapshot stage. |
| `O` | Stage, role, original organism field, Java simple type, observed value. |
| `H` | Stage, role, genotype/infected/saved holder, genotype identity or null. |
| `W` | Stage, derived world getter key, Java type, observed value. |
| `V` | Stage, original World instance field, Java simple type, observed value. |
| `Q` | Stage, queried role, actual hit-role list in source query order. |
| `S` | Stage, original statistics field, Java simple type, observed value. |
| `T` | Stage, actual statistics getter, return type, observed value. |
| `Y` | Stage, genotype label, `observable` or `rejected-only` scope. |
| `G` | Stage, genotype label, field, value. |
| `N` | Stage, genotype label, gene index, field, value. |
| `R` | Stage, following long, integer bounded by `10000`, double bits. |
| `E` | Case terminator. |

Each actor has all 154 organism fields and three explicit genotype-reference holders.
Each snapshot records ten getter-style world values, 15 World instance fields, and full statistics.
Statistics contain all 52 instance fields and 49 public zero-argument getters.
The source `nFrames` instance field is `int`; its getter-style W observation is widened to `long`.

`D` discovery walks the actual world list and compares organism object identities.
Original roles remain `selector`, `victim`, and `killer`.
New roles use `newbornN`; duplicate numeric IDs do not merge objects.
New genotype identities append to the observed pool after actual execution.
No newborn G/N block becomes an input recipe.

Handle `Organism` identity before `Rectangle` values because `Organism` extends `Rectangle`.
Consequently, Q hits and V `_organisms` contain actor labels, never rectangle strings.
J rows retain actual inserted rectangle data separately.

World fields `_visibleWorld`, `inCorridors`, `outCorridors`, and `_corridorexists` remain named exclusions.
A false source corridor flag does not prove implemented Rust transport.
O `_world` and `_visibleWorld` also remain named collaborator exclusions.
All remaining world values, statistics, RNG observations, available actors, and available genotypes remain comparisons.

Y records actual reachability from setup genomes, valid actors, world membership, and statistics.
[Transformation observations](transformation.README.md) explain rejected candidates and their distinct scope.
Y and Z records classify scope; they do not provide biological constructor inputs.

## Predeclared ledger

| Group | Rows |
| --- | ---: |
| Compared observations | 274,872 |
| Supplied G/N constructor checks | 36,980 |
| O collaborator exclusions | 1,522 |
| V collaborator exclusions | 888 |
| Source selection metadata | 73 |
| Scope metadata | 1,235 |
| Other input/schema/command rows | 12,388 |
| Total | 327,958 |

No rejected candidates occur in this fixture.
The provenance preserves exact row tags, operation counts, case names, commands, and hashes.

Source callers occupy `Organism.java:3667-4156`.
Inheritance occupies `Organism.java:1790-2531`; add-event ordering occupies `World.java:747-756`.
The helper's only outcome searches invoke actual source methods within a fixed 1024-seed bound.

The first serializer candidate used rectangle strings for organism identities.
Its helper, outputs, and receipts remain preserved privately.
The corrected generation changes only Q and V `_organisms` rows.
All other rows, including biology, statistics, and RNG, remain byte-identical.
A failed impossible combined-mutation search also remains preserved.
Gene-count and symmetry mutations are mutually exclusive at `GeneticCode.java:760-793`.

All 74 original files retain their hashes, sizes, and modes.
Seventeen prior TSVs remain unchanged.
One prior maintenance fixture has a separately authorized serializer correction, with its baseline and replacement evidence preserved.
The provenance attributes that supersession explicitly.

These observations do not execute full world frames, biological contacts, UI callbacks, persistence migration, or full ecological scheduling.
Rust comparison results remain separate from Java observation provenance.
