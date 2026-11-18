# Genotype identity observations

`genotype_identity.tsv` records actual Java reference identity and shared-state observations.
The unchanged source and bundled JAR produced identical bytes.
The fixture contains one scenario and six observation stages.
It proves reference identity, without claiming the complete infection lifecycle.

## Execution

Compile `tools/reference/GenotypeIdentityOracle.java` against each pinned backend.
Use the exact compiler and runtime arguments in `genotype_identity.provenance.json`.
The source runtime also loads unchanged resources from `../Colormod/src/`.
The helper requires a new output directory and four explicit isolated profile properties.
All subprocesses use the existing bounded process-group runner.

The helper creates a real headless `VisibleWorld(null)` and real `World`.
It assigns the real recipient as the selected-organism input.
This prevents the infection notification from accessing the absent `MainWindow`.
No mocked collaborator or subclass replaces source behavior.
No top-level window, contact calculation, transport operation, or simulation frame executes.

## Independent setup

Use seed `506172` with an ordinary `java.util.Random`.
Create one explicit gene: length `12`, theta `0`, green, branch `0`, and all reactions `0`.
Create its genome with symmetry `1`, mirror `0`, mutation rate `0`, and activity `0`.
Set cream, fallow, spore, and adaptive-spore modifiers to `1`.
Set all fifteen boolean constructor arguments to `false`.
Supply generation `7`, clade `root`, and update-clade `true` as explicit metadata inputs.

Clone that genome through the real `GeneticCode.clone()` method.
Construct organisms `a` and `b` with the original genome.
Construct organism `c` with the cloned genome.
Each organism uses the real `Organism(World, GeneticCode)` constructor.
Set `a._reproduceEnergy=123`, `c._reproduceEnergy=-41`, and `b._infectedReproduceEnergy=-17`.
These energy values are explicit inputs, without claiming a phenotype calculation.

## Stages and observations

| Stage | Operation |
| --- | --- |
| `constructed` | Observe constructor aliases and initially equal clone values. |
| `infection-organism-shared` | Preload `b`'s saved genome, then call actual `b.infectedBy(a)`. |
| `shared-edit` | Apply recorded metadata/gene inputs and explicitly assign `c`'s saved alias. |
| `infection-organism-clone` | Preload `b`'s saved genome, then call actual `b.infectedBy(c)`. |
| `infection-code-shared` | Preload `b`'s saved genome, then call actual `b.infectedBy(shared)`. |
| `shared-second-edit` | Apply the second recorded shared metadata input. |

The constructors retain the same genome reference for `a` and `b`.
The real clone has an independent genome, gene array, and mutable gene.
Reflection compares every declared nonstatic genome and gene field for initial value equality.
Immutable color references remain shared in Java.
Rust may preserve these colors as exact nullable ARGB values without allocating equivalent Java objects.

Both actual infection overloads retain the exact supplied genome reference and clear the saved reference.
The organism overload increments its donor's infection count and copies its reproduction energy.
The code overload leaves reproduction energy unchanged at `-41`.
Three real infection notifications increment statistics to `3`.
Equal donor infection counts retain the first statistics winner.

Explicit shared edits propagate through primary, infected, saved, and statistics references.
The independent cloned genome retains generation `7`, clade `root`, length `12`, and red reaction `0`.
The saved-alias assignment is a controlled input, without exercising the reproduction path that supplies saved genomes.
The clade edits are controlled inputs, without executing `OutCorridor.sendOrganism`.

## TSV schema

Rows are tab-separated ASCII, with one newline after each row.
Booleans use `true` or `false`; null uses `~`.
Double values use sixteen hexadecimal digits containing raw IEEE bits.

| Row | Columns after the tag |
| --- | --- |
| `C` | Scenario name, seed. |
| `I` | Stage, target field, explicitly supplied input value. |
| `B` | Stage, identity or initial-value comparison name, observed boolean. |
| `S` | Stage, counter or energy name, observed integer. |
| `V` | Stage, reference-holder name, observed field, observed value. |
| `R` | Following `nextLong`, `nextInt(10000)`, and raw `nextDouble` bits. |
| `E` | Scenario terminator. |

For `V`, absent holders emit only `present=false`.
Present holders emit generation, clade, update-clade, gene length, and red reaction.
Holder names identify primary, infected, saved, and statistics references.
The RNG suffix matches an independent ordinary `Random` after exactly three constructor `nextDouble` calls.
Infection, cloning, and controlled edits consume no additional random draws in this scenario.

## Source anchors and limits

Paths below are relative to `../Colormod/src/biogenesis/` from the project root.

| Source | Relevant behavior |
| --- | --- |
| `Organism.java:756-774` | Constructors retain the supplied genome and draw initial orientation. |
| `GeneticCode.java:641-670` | Explicit construction copies its container and retains supplied gene references. |
| `GeneticCode.java:1233-1244` | Cloning copies the genome, gene array, and each mutable gene. |
| `Organism.java:5611-5630` | Infection overloads assign references and update bookkeeping. |
| `World.java:776-780` | Infection updates statistics before the selected-organism callback. |
| `WorldStatistics.java:388-397` | Statistics retain genotype references and use a strict infection-count comparison. |
| `Organism.java:2519-2520` | A reproduction path retains a saved genome reference; this path remains unexecuted. |
| `Organism.java:14689-14700` | Contacts compare genotype object identity; contact behavior remains unexecuted. |
| `OutCorridor.java:36-55` | Transport can modify shared clade metadata; transport remains unexecuted. |

This fixture does not verify infection eligibility, contact effects, reproduction, recovery, transport, or persistence.
It does not verify deep-world cloning or Rust behavior.
Separate Rust consumers must identify compared observations and explicit representation differences.
The prior genetics, organism-state, and paste-rejection fixtures remain unchanged.
