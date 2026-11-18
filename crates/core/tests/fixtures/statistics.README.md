# Statistics observations

`statistics.tsv` records actual `WorldStatistics` fields, getters, events, and reference identity.
Unchanged source twice and the bundled JAR produced identical bytes.
It contains 29 cases, 134 snapshots, and 255 actual event/scan calls.
Every snapshot records all 52 instance fields and all 49 public zero-argument getters.

## Comparison contract

Before Rust comparison, the declared contract required exact scalar bits, classification, references, and ordering.
There is no floating tolerance or NaN-category substitution.
History entries retain exact raw bits, including signed zero, signed NaNs, NaN payloads, and infinities.
Integer wrapping and strict comparison ties remain source behavior.
Nullable records and optional killers/infectors remain distinct from invalid actors.

Constructor RNG observations are explicit exclusions from the Rust statistics comparison.
Java creates four real unplaced actors; their constructors consume orientation draws outside the statistics API.
The helper verifies every recorded suffix against ordinary `Random` after exactly four constructor `nextDouble` calls.
All 29 cases retain that suffix across their statistics operations.
The 134 `R` rows do not claim Rust actor-constructor parity.

## Execution and input authority

Compile `tools/reference/StatisticsOracle.java` against each pinned backend.
Run `biogenesis.StatisticsOracle NEW_OUTPUT_DIRECTORY` with the private profile arguments recorded in the provenance.
The helper rejects omitted profile roots and existing output directories.
The existing runner bounds subprocesses and cleans their owned process groups.

`U` supplies `INITIAL_ORGANISMS`, `INITIAL_O2`, `INITIAL_CO2`, and `INITIAL_CH4` before actual statistics construction.
The target statistics object is independent of the real actors' unused world statistics.
It receives only the recorded actual event calls.
No copied event, average, extrema, history, or extinction formula supplies an expectation.

Three explicit recipe records construct `g0`, `g1`, and `g2`.
`g0` is explicit; `g1` uses actual `GeneticCode.clone`; `g2` is independently explicit.
All three initially have equal values and distinct mutable genome identities.
Each has one GREEN gene, length `12`, theta `0.25`, branch `-1`, and 36 zero reactions.
Symmetry is `1`; mirror, mutation rate, and activity are `0`.
All four numeric modifiers are `1`; all fifteen constructor booleans are `false`.
Generation is `41`, clade is `root`, and update-clade remains `false`.
The supplied `input` genotype rows record all constructor values.

Four real `Organism(world, geneticCode)` objects receive the labelled `A` inputs.
Actors `a` and `c` share `g0`; `b` holds clone `g1`; `d` holds independent `g2`.
All canonical actors have valid nonnull genotypes.
The helper controls their counters, raw age, alive flag, and active flag explicitly.
These are unplaced event inputs, without claiming body construction or a complete world frame.
Age is not an oldest-organism metric in this source class.
The alive scan reads the alive flag and counters, without filtering inactive living actors.

`I` rows provide named actor, statistics, and genome controls.
They include private-state seeds needed to observe integer overflow without billions of events.
List controls mutate the actual lists returned by Java getters.
A Rust consumer may seed private history state without claiming a mutable UI getter.
Shared genome edits call the real gene setter or assign explicitly labelled metadata inputs.
They preserve reference topology through historical and living records.

## Covered behavior

| Group | Actual observations |
| --- | --- |
| Defaults | All fields, nullable records, empty histories, zero-time averages, and all getter names/types. |
| Population/creation | Created counts, extrema, timestamps, strict ties, and independent time checkpoints. |
| Birth/death/infection | Last-record selection, donor records, strict counter ties, later replacement, and absent optional donors. |
| Living records | Stable list order, equal-valued distinct genomes, inactive living actors, dead filtering, empty scans, and nonpositive counters. |
| Shared metadata | Historical/living references observe changed clade, generation, and mutable gene values. |
| Extinction | Natural transitions plus every state with lower/upper threshold ties, moderate input, and severe input. |
| Histories | Actual checkpoints through 105 entries, with birth/death/infection ordering. |
| Oversized raw history | Controlled size `101` advances to `102`, preserving the exact equality-only rollover condition. |
| Numeric limits | Signed-zero gases, NaN payloads, infinities, constructor NaN extrema, negative sums, and integer/long wrapping. |

The natural extinction case observes states `0,0,0,1,1,1,2,3,0` across its snapshots.
The controlled matrix exercises all four source extinction states.
It records pending counters before the real checkpoint and observes the resulting state afterward.

The normal history case observes lengths `0,1,2,99,100,100,100`.
The source removes an oldest history item only when its size equals `100`.
The controlled oversized case observes `0,101,102`; it does not assert that ordinary UI use creates oversized histories.

`eventTime` updates sums, gas extrema, checkpoint maxima, histories, and pending counters.
Population extrema change only through the separate population event methods.
Historical record ties retain the earlier reference and timestamp.
Living scans reset their records before applying list-order ties.
The fixture preserves the source's `Extintion` spellings and singular `getMaxBirth` method name.

## TSV schema

Rows are tab-separated ASCII, with one newline after each row.
Doubles use sixteen hexadecimal raw IEEE digits; lists use ordered comma-separated double bits.
An empty list has an empty final value, not `~`.
Null references and null strings use `~`; mutable genome references use `g0`, `g1`, or `g2`.

| Tag | Columns after the tag |
| --- | --- |
| `C` | Case name, mode, constructor seed. |
| `U` | Constructor setting, Java primitive type, supplied value. |
| `A` | Actor label, genome label, children, kills, infections, raw age, alive, active. |
| `I` | Control stage, target, Java type, supplied value. |
| `K` | Event stage, exact Java method name, arguments. |
| `P` | Snapshot stage. |
| `S` | Snapshot stage, original instance field, reflected Java type, observed value. |
| `T` | Snapshot stage, actual getter name, reflected return type, observed value. |
| `G` | Stage, genome label, field, value. |
| `N` | Stage, genome label, gene index, field, value. |
| `R` | Snapshot stage, following long, bounded integer, double bits. |
| `E` | Case terminator. |

`S` and `T` use reflected simple type names, including `GeneticCode` and `List`.
`input` G/N blocks provide constructor data; later G/N blocks observe actual mutable code objects.
Synthetic `_reference` records identity and `_nGenes` records gene count.
Input recipes use `I(setup,genome:gN:construction,String,explicit|clone:g0)`.

Control targets use `actor:<label>:<original field>`, `statistics:<original field>`, or `genome:<gN>:<field>`.
`genome:g0:gene0._length` supplies a real gene-setter input.
History controls use Java type `List` and the original list field name.
Controls and events retain their exact row order.
Several events may occur before the next explicit `P` snapshot.

Event argument encodings are:

| Method | Arguments |
| --- | --- |
| `eventPopulationIncrease` / `eventPopulationDecrease` | Signed integer population. |
| `eventOrganismCreated` | None. |
| `eventOrganismBorn` | New actor, parent actor. |
| `eventOrganismDie` | Victim actor, killer actor or `~`. |
| `eventOrganismInfects` | Receiver actor, donor actor or `~`. |
| `eventTime` | Population, oxygen bits, carbon-dioxide bits, methane bits. |
| `findBestAliveBeings` | Comma-separated actor order, or `~` for an empty list. |

## Counts and scope

| Record group | Count |
| --- | ---: |
| Instance-field observations | 6,968 |
| Getter observations | 6,566 |
| Derived genome/gene observations | 27,336 |
| Supplied constructor genotype rows | 5,916 |
| Constructor RNG rows | 134 |
| Other input/schema/command rows | 982 |
| Total rows | 47,902 |

S/T/G/N total `46,786` when constructor genotype checks are included.
Statistics-derived and subsequent genotype observations total `40,870`.
Keep constructor inputs, constructor checks, and statistics results attributed to their actual scope.

The separate [raw-null diagnostic](statistics_null_genome.README.md) preserves invalid Java actor observations.
Do not substitute dummy Rust genomes or describe those diagnostic calls as passed valid-actor comparisons.

Source behavior is pinned in `WorldStatistics.java:26-516`.
Defaults and fields occupy lines `29-140`; getters occupy lines `142-344`.
Events occupy lines `346-486`; alive scans occupy lines `488-515`.
The provenance records source, backend class, JAR, helper, and fixture hashes.

This fixture does not execute UI, complete world frames, actor reproduction, biological contacts, movement, or serialization migration.
World event routing and statistics UI remain separate acceptance boundaries.
The original 74 reference files and all eight prior TSV fixtures remain unchanged.
The provenance reports Java observations only; Rust comparison results remain separate.
