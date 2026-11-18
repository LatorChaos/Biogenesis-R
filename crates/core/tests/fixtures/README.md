# Construction observations

`construction.tsv` contains matching observations from the pinned Java source and bundled JAR.
`manifest.json` records helper and fixture hashes.
The helper uses real `World` and `Organism` objects without Swing or substituted collaborators.
It appends successful organisms directly because `World.addOrganism` requires its live UI collaborators.
It rotates bodies after all placement attempts, preserving the reference's initial placement rectangles.
It does not invoke `World.genesis`, `World.time`, rendering controls, or organism frames.

Records use tab-separated fields.

- `R`: seed, bound, twenty Java `Random.nextInt(bound)` draws, and the following `nextLong`.
- `Q`: raw double bits and Java's rounded coordinate, including adjacent half-boundary values.
- `W`: seed, dimensions, attempt count, placed population, oxygen, and carbon dioxide.
- `S`: organism identity, four rendered endpoint coordinates, and exact RGB.
- `E`: world record terminator.

The five worlds include zero attempts, crowded failed placements, and 512 organisms across the reference dimensions.
The random cases exercise power-of-two bounds, rejection sampling, signed overflow, and maximal Java bounds.
The construction comparisons require exact integer coordinates, RGB, ordering, identity, population, and initial gases.
The mixed RNG and genetic observations come from `tests/fixtures/java/core.jsonl` at the project root.
The generated Rust oracle table records that file's SHA256.

Run the helper against each reference classpath using Java 21.
Use project-local output directories and `-Djava.awt.headless=true`.
The actual compilation and execution receipts are `.workflow-local/core-oracles/commands.json`.
Never replace expected values with Rust output.

## Combined World rules observations

`world_rules.tsv` freezes four actual-Java cases from original Source twice and the bundled JAR once.
All three TSV files match byte-for-byte before Rust comparison.
The fixture has 2,114,138 bytes and 54,800 rows.
Its SHA256 is `50ab3eff780afdabeb1485d5c1e65301988bed0f0ddfddf48d39ebd45793d545`.
`world_rules.provenance.json` records custody, recipes, row dispositions, and verification limits.

The active-pair case retains a blocked incoming candidate, aliased queue payloads, and an actual outgoing traveller across genesis.
It uses seed 827364, dimensions 7000x7000, and two configured initial attempts.
Three global cases start with one corpse, actor energies 8/13/21, and oxygen 64/5/0.
Each case calls killAll twice, disperseAll twice, and actual World.time once.

The corpus has 49 complete snapshots and 49 copied-RNG suffix observations.
It contains 168 actor views with 154 fields and three genotype holders each.
Each snapshot contains 15 World fields, ten World getters, 52 statistics fields, and 49 statistics getters.
IN contains 91 rows: seven fields across 13 incoming corridor observations.
Both ordered corridor registries and the sticky corridor flag receive direct comparisons.

| Frozen ledger disposition | Rows | Meaning |
| --- | ---: | --- |
| eligible | 51,968 | Expected state, identity, geometry, paint, query, and RNG rows |
| input_genotypes | 1,440 | Declared input-stage genotype and gene fields |
| collaborators | 385 | Actor World/UI presence and World UI presence |
| other | 1,007 | Recipes, settings, controls, placement, commands, discoveries, snapshot boundaries, and scope |
| total | 54,800 | Complete corpus, with no excluded rows |

The other category contains C=4, U=588, I=60, A=12, K=31, D=4, P=49, Y=255, and E=4.
Input rows alone construct Rust state. Observed output rows supply expected values only.
D rows bind actual discovered identities, then validate IDs and genotype labels.
They never select actors by numeric ID or construct actors from observations.
P/E rows delimit snapshots. Y rows compare observable genotype roots.

Exactly 217 collaborator rows describe Java UI presence without a Rust UI object.
These rows contain O._visibleWorld=168 and V._visibleWorld=49.
Four I visible:selected controls also describe the actual Java UI collaborator.
The remaining 168 O._world presence rows have explicit actor-World identity links in ID rows.

Explicit ownership projections preserve the existing Rust representation.
They cover 195 actor/corridor World presence rows and 40 pending-container/corridor-World ID rows.
Rust projects the Java pending container onto its stable incoming corridor owner.
Incoming/outgoing World links use the owning World supplied to their actual Rust calls.
The two absent incoming inherited fields compare exact null across 26 rows.
The frozen corpus has zero null array observations requiring the existing incoming allocation projection.
These annotations overlap the frozen ledger classes. They never reduce the denominator.

Actor holders, queue payload order/aliases, candidate, traveller, actor World links, and outgoing connection links compare actual identities.
World members, both corridor registries, genotypes, statistics, frozen envelopes, query order, paint, and RNG retain complete comparisons.
The shared comparer retains exactly five existing geometry allowances: _m, _m1, _m2, _mass, and _I.
All other values stay exact.

The consumer uses the existing Frame empty constructor pattern for setup.
It performs no setup genesis before the declared genesis call.
Actual World.genesis, kill_all, disperse_all, incoming methods, and paired-corridor attachment drive replay.

One new family test includes positive replay and eleven corruption checks.
They cover O2/CO2, queue order/alias, candidate, traveller, death statistics/order, final RNG, and malformed row sets.
The malformed sets contain a missing row, duplicate row, or unknown tag.

Consumer compilation and debug/release execution remain pending in this draft.
The lead owns integration and updates provenance after execution.
RecordingConnection supplies an inert recording boundary; this corpus does not exercise transport interoperability.
These method observations do not establish rendered controls, performance, Pi, or complete product acceptance.
