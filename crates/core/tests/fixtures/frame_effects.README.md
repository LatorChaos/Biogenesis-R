# Frame-effect and touch-hatch observations

These fixtures observe ten actual Java helpers on real pasted and added organisms.
The producer invokes unchanged source classes twice and the bundled JAR once.
All three outputs match exactly.
Four independently compiled helper classes also match between source and JAR builds.
This evidence covers method calls, rather than complete simulation frames.

The frozen dataset contains 222 cases, 537 snapshots, 1,611 actor views, and 315 calls.
Its 774,373 rows contain 599,709 derived comparisons and 129,120 supplied genotype checks.
Named exclusions cover 3,222 collaborator rows and 222 selection inputs.
The remaining 42,100 rows define settings, actors, inputs, calls, and case boundaries.

| Actual helper | Calls | Pinned Organism.java lines |
| --- | ---: | --- |
| segmentsFrameEffects | 8 | 17936-17956 |
| leafExtraEffects | 26 | 17961-17991 |
| jadeExtraEffects | 24 | 17996-18226 |
| segmentsHealingEffects | 14 | 18231-18317 |
| segmentsExtraEffects | 3 | 18322-18386 |
| segmentsExtraPlantsEffects | 9 | 18391-18440 |
| plantsUpdateEffects | 23 | 18446-18878 |
| segmentsUpdateEffects | 7 | 18884-19078 |
| darkfireRestoreEffects | 7 | 19083-19121 |
| touchHatch | 194 | 5047-5193 |

Source paths and hashes appear in [frame_effects.provenance.json](frame_effects.provenance.json).
The producer is [FrameEffectsOracle.java](../../../../tools/reference/FrameEffectsOracle.java).

## Inputs and observations

Each case resets actual Utils defaults before applying its recorded U settings.
Every case supplies 147 mutable primitive settings.
The producer seeds the real Random before constructing actors.
Constructor draws therefore remain part of every observed RNG suffix.

Input G/N rows supply five genotypes before actor construction.
Each genotype contains 28 metadata values, including two synthetic identity/count values.
Each gene contains 40 original instance fields.
Genotypes g0, g1, and g2 use explicit construction.
Genotypes g3 and g4 use actual clones of g0.

Actual pasteOrganism and World.addOrganism create peer, victim, and killer, in that order.
Their initial legacy IDs are 0, 1, and 2.
The actual VisibleWorld selects peer, keeping victim death callbacks separate from selection.
The fixture records that selection input as excluded UI metadata.

I rows apply labelled genotype, phenotype, cache, scalar, atmosphere, and statistics inputs.
These inputs expose method boundaries without claiming a complete lifecycle produced every supplied state.
Only input G/N and explicit construction recipes construct actors and genotypes.
Later expected observations never supply constructor inputs.

P rows introduce snapshots.
O records every 154-field organism schema, including inherited rectangle bounds.
H records genotype, infection, and saved-cache holder identities.
G/N records all five live genotype states after each operation.
W records ten actual World getter values, including ordered member IDs.
S/T records all 52 statistics fields and 49 public getters.
R records nextLong, nextInt(10000), and raw nextDouble bits from a serialized Random copy.
This copy does not advance the original stream.

K has five tab-separated columns: stage, operation, target, and argument follow its marker.
Every target is victim.
The nine frame helpers use argument `~`.
Touch-hatch calls use argument `killer,0,0` and invoke the actual three-argument private method.
The source ignores the actor segment argument.
Organism identity serialization precedes Rectangle serialization because Organism extends Rectangle.
No V, Q, J, newborn, rejected-candidate, or raw-uninitialized diagnostic records occur here.

## Distinct outcomes

The case inventory targets these independent source outcomes.
It avoids Cartesian combinations of unrelated flags.

| Area | Recorded outcomes |
| --- | --- |
| Cached movement | Reverse order; Teal versus Cyan coefficients; excluded coefficients; successful and failed rolls; full, partial, and empty payments |
| Movement arithmetic | Wrapped integer endpoint subtraction; velocity clamps; real zero-length bodies; zero mass/inertia; nonfinite velocity inputs |
| Leaf energy | Age 7/8/9 methane gates; partial atmosphere; five tint conversions; retained unrelated tints and countdowns |
| Photosynthesis | Positive, zero, negative, and NaN colony caches; successful and failed low-carbon gates; negative and nonfinite quantities |
| Jade restoration | Success/failure bounds; Summer time cycles; negative and extreme time; original Forest/Leaf/Lime/default colors; crowding and tints |
| Healing | Unconditional zero-threshold draws; negative thresholds; later healing gates changed by restoration; failed and partial payments |
| DarkFire | Paid healing followed by another natural draw; successful and failed natural restoration; mirrored symmetry-three thresholds |
| Ability restoration | Olive, Lilac, and Sky success/failure; negative coefficient gate; signed-zero and NaN gate exclusions |
| Maintenance | Full base/internal/unknown ARGB classification; plant/nonplant distinctions; consumer/fungus/gray/killer/eyes/spore flags; cached effects retained |
| Cost parameters | Flower, Eye, Rose, Dark, crowded Lime, movement, and healing costs through actual source calls |
| Scaling | Positive Leaf and methane coefficients underflowing after their original positivity guards; signed-zero energy and atmosphere |
| Integer behavior | Symmetry-three integer healing factors; large valid Magenta genes; wrapping healing totals; age wrapping during actual hatch |
| Touch adaptation | All 46 base and 18 internal colors, plus unknown `12345678` and translucent `12ff00ff` ARGB |
| Touch precedence | Consumer before fungus before plant; infectious plants; active versus inactive peers; Silver and Coral distinctions |
| Touch gates | Healing; modified Lilac; actor infection/Coral flags; fungus spin; Fruit peer genotype; negative and extreme adaptation integers |
| Actual hatch | Successful, failed, partial, zero-cost, and NaN payments; fallback death; Eye rebuilding; modifiers 9/10; age 0/MAX |

All 66 palette cases show an actual miss followed by an actual successful hatch.
Additional precedence cases include actual blocked outcomes.
The adaptation table supplies chosen inputs; it never generates expected answers.

The signed-zero methane case observes energy and oxygen becoming positive zero after scaling a positive coefficient to zero.
The Leaf case observes its tint conversion even when its positive cached coefficient later scales to zero.
These are actual Java observations.

Large healing cases use real 1,000,000,000-square worlds and valid 200,000,000-length genes.
Successful actual paste calls establish their placement.
No fabricated geometry or UI collaborator creates those bodies.

## Equality contract

This contract was declared before Rust comparison.
Compare every non-geometric scalar, cache, photo coefficient, velocity, gas, statistic, and RNG value by exact raw bits.
Compare integer endpoints, bounds, ARGB, reactions, identity, and order exactly.

Only O fields `_m`, `_m1`, `_m2`, `_mass`, and `_I` use the existing geometry allowance.
For finite values, use `1e-10 * max(1, abs(expected))`.
For those five fields, check NaN classification and signed infinity explicitly.
No other float field receives a tolerance or NaN normalization.

Exclude only O `_world` and `_visibleWorld` collaborator references.
Exclude the `visible:selected` input from Rust model-state comparison.
Actual World values, synchronous statistics, genotype references, member order, and constructor RNG remain required.
Separate genetics fixtures cover per-Gene mutable-object alias topology.

## Execution and recovery

Exact compile and JVM argument arrays appear in the provenance file.
Positive JVMs use 120-second bounds and private heap limits.
Each JVM has explicit private home, temporary, user-preferences, and system-preferences roots.
The producer validates resolved roots before accessing Utils or preferences.
The existing process-group runner bounds jobs and cleans owned descendants.
No GUI, network server, installation, or global configuration change occurs.

Six actual negative invocations reject omitted profile properties, existing output, and an escaping preferences symlink.
Each rejection preserves the requested output boundary.
All 74 original hash/size/mode pins and 26 previously adopted TSV hashes pass.
The frozen Rust production file remains unchanged during generation.

The first coverage run remains preserved with its helper, outputs, and receipts.
Its audit found seven movement scenarios selected failed rolls after constructor draws.
Corrected searches execute fresh explicit inputs and actual helper calls for every trial.
Their 2,048-trial bounds found movement seed 263, paid/natural seed 47, and natural-restoration seed 32.
Source/source/JAR search outputs and compiled search classes match.

An earlier diagnostic reused mutable phenotype arrays between trials.
Its results were discarded.
A stricter attempt failed before that input reuse was corrected.
Both receipts remain preserved; neither result supplies adopted expectations.

The final dataset retains 209 initial cases byte-for-byte, corrects seven seeds, and adds six observed restoration successes.
All changes precede fixture freeze and Rust replay.
The validator also corrected an overstrict diagnostic that expected a NaN horizontal velocity to change.
Its replacement checks actual changes across velocity components without relaxing the fixture's equality contract.

## Limits

No World.time scheduling, physical contact eligibility, complete contact matrix, or full ecology acceptance is claimed.
No UI, networking, persistence migration, performance, or distributable acceptance is claimed.
The fixture does not exhaust unrelated flag combinations or malformed settings.
It contains no raw-null genotype or uninitialized actor cases.
This provenance describes Java evidence; Rust comparison receipts remain separate.
