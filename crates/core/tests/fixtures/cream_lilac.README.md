# Cream and Lilac contact observations

This oracle invokes the original touchEffects1 dispatcher on real placed and added actors.
The producer verifies each current actor segment's exact ARGB before selecting Cream or Lilac.
Expected values come from actual unchanged Java execution.

The frozen dataset contains 265 cases, 585 snapshots, 1,755 actor views, and 320 calls.
Cream contributes 200 calls; Lilac contributes 120 calls.
Source runs twice, and the bundled JAR runs once.
All three outputs and four compiled helper classes match exactly.

The 688,600 rows contain 537,085 derived comparisons and 90,420 supplied genotype checks.
Named exclusions cover 3,510 collaborator observations and 265 selection inputs.
The remaining 57,320 rows define settings, actors, controls, calls, and boundaries.

## Frozen source inventory

Cream occupies Organism.java lines 10527-11005.
Lilac occupies lines 11006-11280.
The common peer death guard occupies lines 13106-13108.
This inventory defines the intended observations before generation and Rust replay.

| Family | Distinct source outcomes |
| --- | --- |
| Classification | All 46 base and 18 internal peer colors; unknown and translucent full ARGB; actual source defaults |
| Cream specialization | Modifiers 1/2/3; enhanced and inactive variants; classified White; Spike gates; Silver history; attacker-dependent Sky gates |
| Cream shields | Paid/partial/failed dodge; depleted Lavender recheck; Blue bypass and retaliation; Sky cooldown; Fruit and specialized Cream bypass |
| Cream reactions | Lavender growth despite failed payment; integer wrapping; ordered double death; Mint injury and retained caches; inactive Brown |
| Cream energy | Cost before transfer; explicit byproduct respiration; partial atmosphere; zero and nonfinite state boundaries |
| Lilac eligibility | Modifier; consumer/fungus/plague/fallow/coral flags; actor classification exclusions; Grass half strength |
| Lilac reactions | Own and peer recharge; Blue retaliation; Spike/SpikePoint gates; alive Brown; Bark aging; altruist exemptions |
| Lilac energy | Cost before oxygen-limited peer damage; failed and partial dodge; unchanged attacker energy during peer damage |
| Common tail | Alive guard; strict tolerance; blocked branches; repeated corpse checks; statistics and genotype reference order |

## Capture schema

The producer is [CreamLilacOracle.java](../../../../tools/reference/CreamLilacOracle.java).
Exact hashes, argument arrays, receipts, and ledgers appear in [cream_lilac.provenance.json](cream_lilac.provenance.json).

Each C starts a case with seed 827364.
U records all 147 mutable primitive Utils settings after actual defaults and case overrides.
Input G/N rows supply five explicit or cloned genotypes before actor creation.
Each genotype records 28 metadata values, including synthetic reference and gene-count values.
Each gene records 40 original instance fields.
g0, g1, and g2 use explicit construction; g3 and g4 use actual clones of g0.

Actual pasteOrganism and World.addOrganism create peer, victim, and killer, in that order.
Their initial legacy IDs are 0, 1, and 2.
The actual VisibleWorld selects peer before victim or killer can die.
This selection avoids requiring a nonexistent MainWindow for those callbacks.
It does not establish UI callback parity.

I records every supplied scalar, phenotype, cache, genotype, and atmospheric control in execution order.
Some cases explicitly control physical length caches, classification flags, and corpse counters.
Those labelled inputs do not claim normal frames generated every supplied combination.
All actors first use real constructors, successful placement, and actual World insertion.
No fake World, statistics holder, or GUI object supplies expected values.

K uses `touchEffects1.CREAM` or `touchEffects1.LILAC`, target victim, and argument `killer,seg,oseg`.
The producer checks the exact current actor ARGB before invoking touchEffects1 reflectively.
Most calls use indices 0,0; two cases use indices 1,1.
Later Lilac calls must still select Lilac after prior recharge behavior.
The source branch assertion protects that requirement.

P introduces a snapshot.
O records all 154 organism fields for each original actor, including rectangle bounds.
H records genotype, infection, and saved-code references by exact identity labels.
G/N records all five genotypes after each operation.
W records ten actual World values, including ordered member IDs, population, corpses, gases, and frame/time counters.
S/T records all 52 statistics fields and 49 public getters.
R peeks nextLong, nextInt(10000), and raw nextDouble bits through a serialized Random copy.
The original Random remains unchanged.
All 585 observed suffixes match, confirming these dispatcher calls consumed no additional RNG.

Only input G/N and explicit construction recipes construct the original objects.
Later expected observations never supply constructor inputs.
Organism identity serialization precedes Rectangle serialization because Organism extends Rectangle.
No newborn, rejected-candidate, V/Q/J, raw-null-genotype, or uninitialized-actor records occur.

## Observed boundaries

Each branch observes all 66 peer palette entries separately.
The palette includes unknown ARGB 12345678 and translucent Magenta ARGB 12ff00ff.
Those values retain source default behavior and differ from opaque Magenta's altruist exemption.

The two Blue double-death cases observe actor death before peer death.
Both killer counters become one; population falls from three to one.
The final statistics death record retains g1, the peer's original genotype reference.

The Mint sequences observe partial payment without actor phenotype mutation.
A later complete payment produces DarkFire, Broken, or LightBrown according to the supplied healing and plant flags.
Photo caches, update markers, and the actor's temporary tint remain observed through both calls.

Lavender wrapping cases use the valid maximum integer shield setting.
Failed actor payment still produces source shield growth and tint changes.
Normal and enhanced growth wrap to -2147483499 and -2147482699 respectively.

Lilac Grass and Green cases use a labelled physical length cache of one.
Actual peer energies become 90 and 80 from 100, while actor energy remains 20.
Separate cases exhaust oxygen during the attack cost before attempted peer damage.
Successful attack payment still sets recharge state when no oxygen remains for damage.

Incomplete eligibility blocks dodge payment for Grass and Green.
Positive Spike cases cover plant and enhanced peers.
Repeated paid-dodge sequences retain has_dodged without treating it as an immunity gate.

Strict tolerance cases use original Utils.tol and half that value.
They observe the common death guard after source no-op peer colors.
Already dead peers do not produce duplicate death events.
Insufficient actor energy does not invent an extra actor death check in this dispatcher.

Physical cache cases include zero, negative, NaN, and infinite values.
They are controlled state boundaries, rather than claims about normal construction producing negative physical lengths.
All biological results remain actual source observations.

## Equality contract

This contract was declared before Rust replay.
Compare every non-geometric scalar, cache, velocity, gas, statistic, and RNG value by exact raw bits.
Compare endpoints, bounds, ARGB, reactions, references, and order exactly.

Only O fields `_m`, `_m1`, `_m2`, `_mass`, and `_I` use the existing geometry allowance.
For finite values, use `1e-10 * max(1, abs(expected))`.
For those five fields, check NaN classification and signed infinity explicitly.
No other float field receives a tolerance or NaN normalization.

Exclude only O `_world` and `_visibleWorld` collaborator references.
Exclude I `visible:selected` from Rust model-state comparison.
All independently observable World values, statistics, genotype holders, member order, and constructor RNG remain required.
Separate genetics fixtures cover mutable Gene-object alias topology.

## Execution and preservation

Each positive JVM uses a 120-second bound, private heap limits, and explicit isolated profile properties.
Resolved home, temporary, user-preferences, and system-preferences roots must remain inside the fresh run root.
The producer checks those roots before accessing Utils or preferences.
The existing runner bounds process groups and cleans owned descendants.
No GUI session, network server, installation, or global configuration change occurs.

Six actual negative invocations reject omitted profile properties, existing output, and an escaping preference-root symlink.
Every failure preserves the output boundary.
All 74 original hash/size/mode pins and 28 prior TSV hashes pass.
The frozen Cream/Lilac Rust production file remains unchanged during generation.

The initial 253-case run remains preserved with its helper, outputs, and receipts.
Coverage review added twelve cases before freeze and Rust replay.
They cover positive Cream costs, eligibility before dodge, positive Spike outcomes, empty actor energy, and repeated paid dodges.
Every original case remains a byte-identical prefix of the final output.
No expected observation was replaced after a Rust comparison.

## Limits

This fixture verifies method-level Java observations.
It does not establish physical contact eligibility, complete matrix integration, World.time ordering, or full ecology.
It does not establish UI, networking, persistence migration, performance, or distributable acceptance.
The cases avoid exhaustive Cartesian combinations of unrelated flags and malformed settings.
Java observation provenance does not claim Rust replay results.
