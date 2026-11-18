# Simulation core

The core has no engine imports.
Its XML parser dependency is pinned `quick-xml`, with encoding support.
One `World` owns its seed, ordered organisms, random stream, gases, clock, and pause state.
The adapter consumes owned ordered `RenderSegment` snapshots.
Snapshot reads do not consume randomness or advance the world.
The separate `advance_presentation()` command reproduces source paint-driven traveller transitions and returns ordered line and rectangle commands.

This foundation implements these reference behaviors.

- Java-compatible 48-bit seeded random draws, bounded rejection sampling, and signed integer composition.
- Complete gene/genome fields, configurable weighted generation, cloning, inheritance, mutation, and meta-mutation.
- Shared gene and genotype identity, isolated world copies, detached inspections, and lineage metadata updates.
- All 36 reaction channels and exact full-ARGB classification, including unknown and transparent colors.
- Body geometry across growth scales, polar vectors, integer rounding, branches, symmetry, mirroring, and eye geometry.
- Initial organism state, segment aggregate effects, photosynthesis coefficients, and reproduction thresholds.
- Child selection, energy allocation, saved-genotype retries, neighboring placement, stillbirth, birth spores, and successful clade updates.
- Frozen spatial packing and query order matching the embedded JTS implementation.
- World statistics events, record genotype aliases, population counters, and the 256-frame sampling clock.
- Geometry rebuilds that preserve current segment colors and reactions.
- Initial gas debits, including failed placement, twelve placement trials, and ordered organism identities.
- Reference polar rotation calculations for segment snapshots.
- Ordered atmospheric CO2/CH4 conversion through explicit discrete ticks.
- Source-ordered actor movement, contacts, active effects, reproduction, maintenance, death, and removal.
- Positional inspection with live/corpse priority, guarded global death, and ordered corpse respiration.
- Opaque identity-preserving organism handles, guarded player actions, and read-only tracking centers.
- Source-ordered genome previews with distinct forward-branch and organism-placement rules.
- Ordered incoming retries and outgoing admission, shared genotype queues, isolated clone graphs, and explicit presentation-driven corridor capacity.
- Legacy genotype XML parsing, semantic writing, and bounded atomic file operations, with documented remaining format gaps.
- Pause, resume, paused single-step, and deterministic reset.

`WorldConfig::default()` uses seed zero and the reference dimensions, initial attempts, and initial gases.
New worlds and resets start paused as an explicit frontend integration policy.
`step()` advances only running worlds.
`single_step()` advances only paused worlds and preserves the paused state.
Both methods return `Result<bool, WorldError>` and retain partial simulation state on failure.
A running world pauses when frame execution fails.

This foundation restricts startup to 12,500 attempts to bound the current integration checkpoint.
That restriction must be removed or parameterized before full configurable-settings acceptance.
Invalid dimensions and bodies that exceed placement bounds produce explicit errors.
The Java reference instead rejects invalid random bounds through an exception.
No failed world construction replaces an existing world.

Death and boundary methods match dedicated Java comparisons.
Reproduction, transformation, and maintenance methods match dedicated source/source/JAR comparisons.
Contact-state restoration and support methods match dedicated source/source/JAR comparisons.
Reaction, brake, active segment, and contact-hatching methods match dedicated source/source/JAR comparisons.
Coral contact methods match dedicated source/source/JAR comparisons, including actual newborns and failed-transformation death.
ORANGE, FIRE, RED, PINK, MAROON, CREAM, and LILAC methods also match dedicated Java comparisons.
SPIKEPOINT, OCHRE, and SILVER also match dedicated source/source/JAR comparisons.
The complete C1 dispatcher runs all eleven draining-contact cases through their original phenotype selection.
BLUE, YELLOW, ROSE, MINT, LAVENDER, and MAGENTA match dedicated comparisons, including synchronous Yellow births.
WHITE and FRUIT also match dedicated Java observations, including direct source-global numeric boundary states.
The complete C2 dispatcher selects all fourteen original cases.
PLAGUE, FALLOW, VIOLET, SKY, OLIVE, and GRAY now pass actual Java comparisons.
The complete physical contact and frozen-tree world query also pass actual Java comparisons.
The 255-test contact checkpoint preserves 180,816 derived contact observations across 61 scenarios.
Ordinary actor frames pass dedicated source/source/JAR comparisons, including frozen-tree retention and same-frame newborn deferral.
Both required dense endpoint profiles pass 4,144,017 derived comparisons after actual eighty-frame workloads.
Compact scalar and RNG histories pass 19,116 runtime comparisons plus 648 explicit Gaussian source invariants.
Corridor controls and transport, statistics UI, complete persistence, and networking remain incomplete.
The full Color Mod conversion remains the product contract.
This checkpoint does not satisfy TASK-7, TASK-8, or final product acceptance.
See [genetics evidence](../../docs/evidence/genetics-2026-10-25.md) for TASK-6 comparisons and exclusions.
See [contact evidence](../../docs/evidence/contacts-2026-10-26.md) for subsequent method checks and remaining integration gates.
Read [numeric integration](../../docs/evidence/numeric-2026-10-26.md), [trajectory evidence](../../docs/evidence/trajectory-2026-10-26.md), and [XML coverage](../../docs/xml_codec.md).
Read [incoming integration](../../docs/evidence/incoming-2026-10-26.md) for queue identity, placement retries, and frame ordering.
Read [outgoing integration](../../docs/evidence/outgoing-2026-10-26.md) for bounded admission and source-ordered presentation.
Read [world actions](../../docs/evidence/world-actions-2026-10-26.md) for positional identity and global-action integration boundaries.
Read [player controls](../../docs/evidence/player-controls-2026-10-27.md) for selected actions, preview corrections, and actual native integration.
Read [ordered color routing](../../docs/evidence/color-matrix-2026-10-26.md) for all 2116 base-color pairs and the required external gate.

Source authority:

- `../Colormod/src/biogenesis/Gene.java`: fields, constructors, setters, cloning, random reactions, and color selection.
- `../Colormod/src/biogenesis/GeneticCode.java`: fields, constructors, inheritance, mutation, cloning, and clade updates.
- `../Colormod/src/biogenesis/Organism.java`: constructors, geometry, placement, initial effects, and identity observations.
- `../Colormod/src/biogenesis/World.java`: initial gas state, `genesisCheckHit`, and the two ordered reactions in `time`.
- `../Colormod/src/biogenesis/Utils.java`: selected constructor defaults and exact colors.

These source paths are relative to the project root.
Copyright remains with the original authors named in the translated source files.
The combined core package uses GPL-2.0-only.
Original files retain their individual license notices.
Read `../../NOTICE.numeric` for the included Intel LIBM translation and its source provenance.

Run `python3 tools/build.py test` from the project root through the verified isolated build configuration.
Tests compare fixed Java observations and independently checked invariants.
`tests/fixtures/README.md` records construction fixture scope and provenance.
Exact floating RNG and genome fields compare by bits.
Construction coordinates, reactions, bounds, and color identities compare exactly.
Some derived geometry floats use explicitly recorded tolerances; other fixture scalar comparisons preserve exact bits.
The independent atmospheric closed form uses absolute tolerance `1e-7` after 1,024 ticks.
Other-platform floating behavior and complete game runtime remain unverified.
