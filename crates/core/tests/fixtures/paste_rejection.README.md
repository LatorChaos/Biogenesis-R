# Paste rejection observations

`paste_rejection.tsv` preserves four actual source/source/JAR-matched paste cases.
The helper is [PasteBoundaryOracle.java](../../../../tools/reference/PasteBoundaryOracle.java).
`paste_rejection.provenance.json` records exact commands, hashes, outcomes, and comparison limits.
The separate 167-case organism-state fixture remains unchanged.

Each case uses a real fresh `World(null)` with its configured initial gases.
Each genotype contains one GREEN gene with length 12, theta 0, and the case's first branch.
All 36 reactions remain their real zero-constructor defaults.
Genome inputs use symmetry 1, mirror 0, mutation/activity 0, and cream/fallow/spore/adaptation values 1.
All 15 boolean modifiers are false.
Generation, clade, and updateClade retain real constructor defaults before the candidate paste.

The valid control pastes at 3500,3500 with branch 0.
The first-branch case supplies branch 999 at the same location.
The outside case uses -100,3500, avoiding the source's zero-origin death special case.
The overlap case first places a real blocker through `pasteOrganism()` at 3500,3500.
It appends that successful blocker to the actual world's organism list before constructing the candidate.
It does not call UI-dependent `World.addOrganism()` or substitute a collaborator.

The records use tab-separated fields:

- `C`: case name, seed, world width/height, candidate x/y, first branch, and overlap flag.
- `K`: blocker x/y/width/height, then CO2/O2/CH4 and NEXT_ID after its real placement.
- `B`: CO2/O2/CH4, NEXT_ID, candidate generation, and candidate clade before its constructor.
- `A`: success, CO2/O2/CH4, NEXT_ID, candidate generation/clade, and inherited x/y/width/height after paste.
- `R`: following `nextLong()`, `nextInt(10000)`, and raw `nextDouble()` bits, in that order.
- `E`: case terminator.

Double values contain 16 hexadecimal raw-bit digits; comparisons require exact values.
Null clade IDs use `~`; other scalar values use Java decimal integers or booleans.
The blocker consumes its own actual constructor draw before the overlap candidate.
The candidate constructor consumes an orientation draw in every case, including rejections.

The valid paste consumes 40 CO2, adds 40 O2, and allocates one identification number.
All three rejected candidates preserve their world's gases and NEXT_ID.
Every candidate's genotype generation changes from 0 to 1 during the real paste attempt.
The valid genotype receives clade `0`; rejected candidate clades remain null.
Source observations retain rejected candidate metadata and bounds even when discarded by the Rust factory.

The Rust factory comparison requires rejection/success, placed-object state, world gas effects, ID allocation, and RNG suffixes.
Discarded candidate metadata/bounds are unavailable through that factory and have an explicit comparison disposition.
They remain recorded here; their equality is not claimed.
Editor/clipboard input retention remains a caller contract, without a UI journey claim here.

The source guard follows geometry initialization in `Organism.java:3013-3019`.
Geometry intentionally ignores a first gene's branch, while paste rejects any branch greater than its index.
The source's discarded outside bounds are retained without correcting them.
The fixture does not exercise the zero-origin death special case, full frames, UI controls, or birth inheritance.

Compile the helper separately against unchanged source classes and the bundled JAR using Java 21.
Use a new output directory and explicit isolated home, temporary, and both preferences roots.
The helper rejects existing outputs and missing isolation flags before creating fixture files.
Compare source/source/JAR bytes before adopting regenerated observations.
