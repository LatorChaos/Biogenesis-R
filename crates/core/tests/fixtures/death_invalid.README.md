# Invalid death and boundary diagnostics

`death_invalid.tsv` preserves two explicitly invalid source-state cases.
Unchanged source twice and the bundled JAR produced identical bytes.
The fixture contains two cases, four complete snapshots, and two actual calls.
It uses the same helper and observation schema as [canonical death observations](death.README.md).
Run `biogenesis.DeathOracle NEW_OUTPUT_DIRECTORY raw-invalid` with the isolated profile arguments recorded in provenance.

The first case pastes valid actors, then supplies a current-color array beginning with null.
Actual `die(killer)` clears its early flags and references, then throws `java.lang.NullPointerException` during color processing.
Population, killer counter, and death statistics remain unchanged because execution fails before their updates.
`X` records the exception class without a runtime-dependent message.
The helper catches that diagnostic exception only; it does not report successful death.

The second case first creates and registers the same three valid actors as the canonical setup.
It then replaces only the observed victim role with actual `new Organism(world)`.
`F(replace_victim_with_unregistered_raw_allocation,~)` labels this deliberate input change.
The original valid victim remains registered in the world list.
The replacement has a null genotype, zero segments, and a zero-size origin rectangle.
Actual private `isInsideWorld` invokes `die(null)`, decrements population, and delivers real death statistics.
The world storage remains the original three actors.
The operation returns false and negates both zero velocities.
`X(none)` records that this actual boundary operation did not throw.

These invalid allocations and null current colors are outside the valid Rust organism representation.
Do not construct dummy genomes, suppress exceptions, or describe these calls as passed Rust parity cases.
All 6,139 rows remain source diagnostics with zero canonical Rust comparisons claimed.
The two unsupported calls retain all observed state for future compatibility review.
Nullable statistics records and cleared infection/saved references remain canonical supported requirements elsewhere.

The fixture adds `X(stage,exception_class_or_none)` and the single `F` allocation-replacement marker.
It otherwise retains all 154 organism fields, 52 statistics fields, 49 getters, genome identities, world values, and RNG suffixes.
There are 1,848 `O`, 208 `S`, 196 `T`, 840 `G`, and 2,640 `N` rows.
The remaining 407 rows record inputs, references, world state, calls, returns, RNG, and framing.
The provenance records the full tag ledger and source/source/JAR equality.

No complete world frame, contact, UI callback, corpse decomposition, or Rust acceptance occurs in this diagnostic mode.
The original reference files and all earlier fixtures remain unchanged.
