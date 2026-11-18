# Genetics observations

`genetics.tsv` records actual observations from the pinned Java source classes and bundled Color Mod JAR.
`genetics.provenance.json` records hashes, exact commands, comparison results, coverage, and limitations.
The helper is [GeneticsOracle.java](../../../../tools/reference/GeneticsOracle.java).
Existing construction fixtures remain separate.

The helper uses real `Gene`, `GeneticCode`, and `java.util.Random` implementations.
It records every nonstatic genome field and every nonstatic gene field.
The field sets contain 26 genome fields and 40 gene fields, including all 36 reactions.
Each selected case executes with recording and plain Random instances.
Both executions must produce identical states, aliases, editing observations, and following random values.
The recorder delegates every draw to `super`; it supplies no scripted values.
Discarded search candidates supply no expected state records.
Source and JAR runs must produce byte-identical states, draw traces, and coverage tags.

The fixture uses tab-separated records:

- `C`: case name, operation, seed, initial complexity, minimum rate, maximum rate, meta rate, and clade complexity.
- `W`: color index, color name, and weight. Each case includes all 46 weights, with YELLOW last.
- `G`: state label, raw Java genome field name, and value.
- `N`: state label, gene index, raw Java gene field name, and value.
- `A`: observed assertion name and value, or a supplied edit input.
- `R`: following `nextLong()`, `nextInt(10000)`, and raw bits from `nextDouble()`, in that order.
- `X`: observed Java exception class and message for an explicit invalid-setting case.
- `E`: case terminator.

Raw double values use 16 lowercase hexadecimal digits.
Colors use eight hexadecimal ARGB digits.
Null colors and clade IDs use `~`.
Other fields use Java decimal integers, lowercase booleans, or the controlled ASCII clade string.

The state labels are `input`, `output`, `input_after`, and `output_after`.
After states follow the documented edit operations, or verify parent preservation after inheritance.
Two additional `N` fields record mutable gene identity:

- `_shared_parent_index`: first matching gene slot in the input genome, or -1.
- `_alias_index`: first matching earlier slot within the same genome, or -1.

Java `Color` instances are immutable.
The Rust comparison preserves their exact nullable ARGB values without requiring allocated Color-object identity.
Mutable gene sharing remains observable through identity comparisons and subsequent edits.

The controlled template supplies these constructor inputs:

- Gene index `i` has length `20.25+i` and theta `-0.375-i/8`.
- Its ARGB is `0x12000000 | (((i+1)*0x12345) & 0xffffff)`.
- Its branch equals `i`, except index zero uses -1.
- Reaction index `j` contains `1000+100*i+j`, in `REACTION_NAMES` order.
- Genome defaults are symmetry 4, mirror 1, mutation rate 0, activity 2, and cream/fallow/spore/adaptation values 7/4/12/70.
- The 15 boolean modifiers alternate true/false, starting with plague=true and ending with selfish=true.
- Ordinary templates set generation 41, clade `root+3-5>8<9`, and updateClade=true after construction.

Every inheritance input has its complete supplied fields recorded.
Special cases record the exact alternative metadata, branches, generation, or null clade.
The constructor-default case leaves generation, clade, and updateClade untouched after the real constructor.
The raw metadata case supplies symmetry -3, mirror -7, mutation -17, activity -4, and cream `Integer.MIN_VALUE`.
It also supplies fallow -9, spore 99, and adaptation -30.

The operations have these controlled meanings:

- `gene-blank` creates the real zero-argument Gene.
- `gene-explicit` passes the index-zero template parameters into the full Gene constructor.
- `gene-setters` starts with a blank Gene and exercises every scalar and reaction setter.
- `gene-random` invokes the real `Gene.randomize()` with supplied weights.
- `gene-color` changes only an explicit gene's color through `randomizeColor()`.
- `gene-clone` clones an explicit gene, then edits the clone.
- `genome-empty` constructs an explicit genome from an empty list.
- `genome-explicit` constructs distinct containers from the same three supplied references: gene 0, gene 0, gene 2.
- `genome-random` invokes the real random GeneticCode constructor with supplied settings.
- `genome-clone` deep-clones a three-slot genome containing repeated gene references, then edits clone slot zero.
- `inherit` invokes the real parent-based constructor and records parent/child fields and shared references.

The explicit-list test replaces caller-list slot zero after construction, then edits the original supplied gene.
The list replacement must leave both copied containers unchanged.
The supplied-gene edit remains visible through shared references.
Clone edits and the designated inheritance edit set gene-zero length to -1234.5 and red reaction to -777.
The `A` records also supply these edit values.
The all-setters operation supplies length bits `7ff8000000000042`, theta bits `8000000000000000`, null color, and branch `Integer.MIN_VALUE`.
Reaction index `j` receives `Integer.MIN_VALUE+j` through its actual setter.

Coverage includes blank/explicit construction, cloning, sharing, every color selector, custom weights, configurable complexity, and rate bounds.
Inheritance cases cover zero/full mutation, symmetry precedence, unchanged symmetry selections, insertion, deletion, branch repair, and all reaction gates.
Insertion cases include fresh genes and clones before, after, first, and last.
Meta cases cover large/small increase/decrease, boundary no-ops, clamping, and signed integer wrapping.
Lineage cases cover generation wrapping, clade copying, and structural update marking.
Imported 101-gene cases preserve or decrease their count; mutation cannot add at the 100-gene threshold.
A separately searched random constructor produces 101 genes with initial complexity 101.

Searches are bounded to seeds 0 through 2047.
Trace sites identify observed draws and decisions; coverage tags do not measure every Java control-flow edge.
The two malformed-setting cases retain actual Java failure observations and following random values.
Rust rejects malformed settings before drawing randomness; those cases use an explicit boundary-validation expectation.
Valid-setting cases require exact following random values.

Birth placement, spore paths, and actual clade-suffix appending remain dependencies for later lifecycle work.
The helper does not call `Organism.inherit`, `inheritTransformation`, UI controls, rendering, or network peers.
The recorded constructor and mutation cases establish genetics observations, without a complete game-parity claim.

Compile the helper separately against unchanged source classes and the bundled JAR using Java 21.
Use the reference classpath and JTS artifact identified in `docs/reference_oracles.md`.
Each invocation requires a new output directory and explicit isolated home, temporary, and both preferences roots.
The helper rejects existing output directories and missing isolation flags before opening fixture files.
Compare source/source/JAR bytes before adopting regenerated fixtures.
The provenance file contains the exact executed argument arrays and receipt locations.
