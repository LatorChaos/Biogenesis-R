# Organism initialization observations

`organism_state.tsv` records actual unchanged Java source and bundled JAR observations.
`organism_state.provenance.json` records exact commands, hashes, accepted comparison rules, and coverage limits.
The helper is [OrganismStateOracle.java](../../../../tools/reference/OrganismStateOracle.java).
It contains input setup and reflection-based observation; the original methods calculate every expected biological result.

The helper uses real `World(null)` and `Organism` instances.
It supplies the configured initial gases through actual world gas methods.
It does not invoke `World.genesis`, which requires its visual collaborator.
It does not register organisms through `World.addOrganism`, which also requires UI collaborators.
World population and stored-organism counts therefore remain zero in these isolated placement observations.
Identification allocation and gas changes remain observed through the real placement methods.

The fixture contains 167 cases:

- One uninitialized real Organism constructor.
- All 46 selectable colors through actual one-hot Utils probabilities and `randomCreate()`.
- Three bounded-searched random worlds producing symmetry 1, 3, and 8.
- Six explicit symmetry/mirror combinations through actual `pasteOrganism(3500,3500)`.
- Twelve spore modifiers with lavender, including two direct age-one effect reapplications.
- Twelve C4/spore combinations covering modifiers 1 through 12.
- Sixty-two mixed-color cases with paired false/true modifier groups.
- Three summer activity cases and one explicit eye branch-reference case.
- Eighteen controlled internal phenotype colors, each followed by two direct effect calls.
- Separate DARKGREEN cases with original SPRING and JADE genotypes.
- One controlled age-zero spore reapplication case.

Mixed colors include SILVER with CORAL/PINK/GRAY/PLAGUE, SPIKE with DARKGRAY, and pure/mixed/protected C4 organisms.
They also include ROSE/BLOND/LAVENDER/WHITE, flower, gold, purple, teal, eye, and attached-eye combinations.
Paired modifier groups vary altruist, passive, modified pink/sky, plague, and clockwise together.
A full modifier Cartesian matrix remains untested.

Each case emits tab-separated records:

- `C`: case name, operation, and seed.
- `U`: effective Utils setting name, Java type, and supplied value.
- `I`: explicit controlled input name, type, and value.
- `G`: stage, raw genome field name, and value.
- `N`: stage, gene index, raw gene field name, and value.
- `O`: stage, raw organism field name, Java type, and value.
- `W`: stage, selected actual world field, Java type, and value.
- `A`: observed initialization-success assertion.
- `R`: following `nextLong()`, `nextInt(10000)`, and raw `nextDouble()` bits, in that order.
- `E`: case terminator.

Double values contain 16 lowercase hexadecimal raw-bit digits.
Colors contain eight hexadecimal ARGB digits.
Arrays contain comma-separated scalar values in source order.
Null values use `~`; nonnull World, VisibleWorld, and GeneticCode references use `present`.
GeneticCode fields and genes expand separately whenever the primary genotype exists.
Rectangles contain x, y, width, and height, in that order.
The additional organism key `bounds` captures inherited Rectangle state.

Observation stages separate these operations:

- `constructor`: actual constructor state before initialization.
- `initial`: actual successful `randomCreate()` or `pasteOrganism()` result.
- `controlled_input`: actual initialized state after the declared age/phenotype inputs.
- `reapplied`: one direct call to the original `segmentsCreateEffects()`.
- `reapplied_twice`: a second direct call without an intervening restoration or frame.

The `controlledCreateEffects` mode first performs a real explicit paste.
Reapplication stages also occur in selected paste cases when `I reapply_count` is positive.
The controlled age input remains unused when reapply_count is zero.
Controlled phenotype replacements change every current segment color while preserving the recorded genotype.
Most internal-color cases start from a real GREEN paste.
The separate DARKGREEN cases start from SPRING or JADE.
Age-one and repeated calls retain all other actual pre-call state.

Explicit input genes use length `6.25+3.5*i`, theta `0.25+0.37*i`, and the listed colors.
Branches default to -1; the explicit eye-reference case supplies -1, 0, and 1.
Reaction channel `j` receives `(i+j)%6`, in the source's 36-channel order.
Explicit genotypes use mutation rate 50, cream 2, fallow 1, adaptation 30, and the case's spore/activity values.
Their generation, clade, and updateClade fields retain actual constructor defaults before paste.
Complete constructor-stage genotype records preserve every supplied parameter.

Every declared organism instance field is observed: 153 fields, plus inherited bounds.
This includes primitive flags/counters, nullable state, phenotype colors, all 36 reaction arrays, and photo/geometry arrays.
Object references record presence; the helper does not serialize unrelated UI or world object graphs.
All effective configurable Utils defaults and case overrides are recorded.
One-hot random cases use initial complexity 1 and exactly one positive color weight of 7.
Symmetry searches execute real randomCreate calls with increasing seeds below 256.

Source/source/JAR comparisons require byte-identical output.
The Rust comparison contract was declared before comparison:

- Organism scalar floats, gas values, photo coefficients, angle, and random suffix values require exact bits.
- Integers, booleans, nullable state, ARGB, endpoints, bounds, and ordering require exact equality.
- Only `_m`, `_m1`, `_m2`, `_mass`, and `_I` may use `abs(actual-expected)<=1e-10*max(1,abs(expected))`.

The geometry allowance does not apply to `_mphoto` or other effect calculations.
The fixture retains raw geometry bits even when a consumer uses the declared tolerance.

The helper never invokes organism frames, touch interactions, rendering, networking, restoration, or birth inheritance.
Direct effect reapplication establishes the called function's behavior under labelled inputs.
It does not establish that those inputs occur through a complete frame or biological transformation journey.
Positive saved/infected genotypes, collision placement, and broader lifecycle behavior remain later checks.

Compile the helper separately against the pinned source classes and bundled JAR with Java 21.
Use a new output directory and explicit isolated home, temporary, and both preferences roots.
The helper rejects existing outputs and missing isolation flags before creating fixture files.
Compare source/source/JAR bytes before adopting regeneration results.
Exact executed argument arrays and receipt paths are recorded in the provenance file.
