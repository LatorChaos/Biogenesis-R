# Uninitialized Java actor diagnostics

`statistics_null_genome.tsv` is a separate diagnostic, outside the canonical valid-actor statistics comparison.
Source twice and the bundled JAR produced identical bytes.
It contains one case, six snapshots, and five actual event/scan calls.

Run `biogenesis.StatisticsOracle NEW_OUTPUT_DIRECTORY raw-null-genome` with the private profiles recorded in its provenance.
The schema remains identical to [statistics.tsv](statistics.README.md).

The helper constructs four actual Java `Organism(world, null)` objects.
Their named `A` inputs therefore record genome `~`.
The three explicit genome recipe objects remain recorded, but the actors do not reference them.
The helper supplies positive counters to one actor before calling actual statistics methods.
No dummy genotype or replacement collaborator changes Java behavior.

The diagnostic calls are:

- `eventOrganismBorn` with an uninitialized receiver and parent.
- `eventOrganismDie` with an uninitialized victim and killer.
- `eventOrganismInfects` with uninitialized receiver and donor.
- `findBestAliveBeings` over those actual actors.
- `eventTime` after the four unsupported actor-consuming calls.

The first four calls cannot be represented by Rust's valid-organism API, which requires nonnull genotypes.
The time event itself is representable, but this diagnostic's prior state comes from unsupported actor calls.
All five calls and all 2,067 rows remain diagnostic observations without a Rust pass claim.
Do not introduce nullable wrappers solely to reproduce an uninitialized Java allocation.
Do not replace null genotypes with dummy genomes to obtain passing comparisons.

Nullable statistics records remain representable as data.
This disposition concerns the actor input boundary, not permission to discard source nullability during future save migration.
Canonical optional killers/infectors and default null records remain strictly compared separately.

The six `R` rows record Java constructor draws and remain explicit constructor-RNG exclusions.
The helper verifies that statistics calls consume no additional draws.
All 52 instance fields, 49 getters, histories, and recorded genome objects remain preserved after each call.
The provenance records the exact commands, hashes, original-file preservation, and output-isolation checks.
