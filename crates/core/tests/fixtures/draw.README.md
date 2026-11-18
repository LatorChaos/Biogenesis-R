# Original draw-command observations

`draw.tsv` records actual original `World.draw` calls and their nested `Organism.draw` calls.
Two source runs and the bundled JAR produce identical bytes.
All six compiled helper classes match across those backends.
The fixed scope contains 27 cases, 97 snapshots, 99 actor views, and one dynamically discovered member.
There are 32 draw calls, four bounds preparations, one death preparation, and one genesis call.
The actual draws emit 142 line commands.

## Command authority

`RecordingGraphics` extends `Graphics` and delegates every abstract method to a real owned BufferedImage graphics context.
Each draw creates a fresh 64-by-64 ARGB image and sets its real delegate color to WHITE.
The observer records each actual `drawLine` call and the delegate's current `getColor().getRGB()` value.
It then forwards the original coordinates to the delegate.
It does not calculate coordinates, classify colors, select tints, or reproduce the source drawing branches.
It records commands even when the image clips their visible pixels.
The image size does not define a viewport acceptance test.
No pixel output supplies expected values.

L uses eight columns: `L,stage,ordinal,x1,y1,x2,y2,argb`.
Every coordinate and ARGB value is a signed Java i32.
The ordinal starts at zero for each draw.
L contains no organism identifier because World.draw supplies no actor callback.
Separate O/H/V observations preserve model identity and membership order.

Each draw applies its explicit I inputs before snapshot `before-step-N`.
K then calls actual World.draw, followed by L commands and snapshot `step-N`.
Every recorded model observation matches exactly between each draw's before and after snapshots.
The observer clones Random through serialization for R observations; it does not consume the source RNG.

K uses the existing five-column structure.
`draw,victim,~` calls actual World.draw, including the empty case without a victim actor.
`bounds,ROLE,true|false` calls the actual actor's calculateBounds method with that force argument.
`die,victim,~` calls actual victim.die(null).
`genesis,victim,~` calls actual World.genesis using the recorded settings and current RNG.
The victim placeholder for World operations does not fabricate an actor.
There are no B or J rows.

## Setup and complete observations

Each case resets source defaults and records all 147 mutable primitive Utils settings.
Networking, automatic backups, and hardware acceleration stay disabled.
Ordinary actor cases explicitly set world gases before actual paste and add operations.
The constructor's statistics remain based on the original recorded initial settings.
Empty cases retain the original new-World zero-gas distinction.

Initial g0, g1, g2, and g3 are explicit recipes.
g4 is an actual deep clone of g0.
All supplied genotype and gene fields appear in separate input G/N blocks.
No observed line, snapshot, or dynamic child becomes construction input.
Ordinary recipes use length twelve, symmetry one, mirror zero, mutation zero, generation 41, and clade `root`.
Targeted recipes explicitly supply their recorded colors, symmetry, and mirror values.

A rows construct actual pasted actors and append them through actual World.addOrganism.
The genesis case creates a real random organism through World.genesis and Organism.randomCreate.
D discovers that member through Java object identity and actual membership order.
All known actors remain available for observations.
Y declares genotype reachability; every recorded scope is observable.

Every actor view includes O154, three H references, and all current genotype/gene fields.
Each snapshot also includes W10, V15, S52, T49, actual Q results, and nonconsuming R suffix observations.
O uses the existing 153 instance fields plus the inherited Rectangle bounds.
Q queries the actual empty tree; this corpus performs no tree insertion or rebuild.
The observer handles Organism identity before Rectangle inheritance when encoding objects.
All role and genotype reference tokens pass independent readback checks.

## Distinct outcomes

Normal bodies cover pasted geometry, mirrored symmetry three, and all 66 current palette values.
Unknown and translucent colors preserve their complete ARGB values.
Changed angles retain cached endpoints until an actual forced bounds call.
An equal-angle nonforce preparation retains explicitly supplied cached endpoints.
Wrapped integer-center sums retain Java overflow.
A separate case distinguishes integer centers from fractional centers and supplied bounds.

Positive tint timers differ from zero and negative timers.
Exact EYE colors remain exempt only when the actual source eye flag enables that branch.
A translucent EYE lookalike and VISION do not receive that exemption.
Dead bodies distinguish eye retention, ordinary BROWN rendering, and active temporary tints.
A real death call supplies a separate naturally transitioned corpse case.

Null tint uses actual Graphics.setColor behavior.
Cases distinguish initial WHITE, a preceding EYE segment, and a preceding actor's final color.
One explicitly controlled nullable phenotype case records the same real delegate behavior without a tint.
Those nullable phenotype inputs describe a method boundary, rather than ordinary natural lifecycle reachability.
The array itself is nonnull; entries zero and two are null.
No inspected normal gameplay or valid placed-construction path creates those null entries.
Random generation, XML parsing, and the color selector produce nonnull colors.
A null gene color fails placement during the original immediate color equality check.
Public Gene.setColor permits later external null mutation, but its subsequent valid gameplay reachability remains unestablished.
The current Rust replay includes this case in the complete 27-case ledger and retains its dedicated diagnostic check.
See [the November 5 reconciliation](../../../../docs/evidence/colors-2026-11-05.md) for implementation history and execution evidence.
Two-actor cases preserve actual forward World membership and segment order.
A positive-zero-negative timer sequence uses successive real draws on one body.

The random-created body initially retains actual zero endpoint caches.
Its first draw records those caches without an observer repair.
A later real forced bounds call changes the next draw's source coordinates.
The empty world emits no line commands.

## Comparison contract and fixed ledger

The fixture, helper, ledger, typed inputs, and exclusions freeze before Rust replay.
Compare every L coordinate, signed ARGB, ordinal, and command order exactly.
Compare nongeometric floating bits, gases, statistics, RNG, integers, reactions, endpoints, bounds, identities, and ordering exactly.
Only O fields `_m`, `_m1`, `_m2`, `_mass`, and `_I` retain the established geometry allowance.
For finite geometry, use `1e-10 * max(1, abs(expected))`.
Classify geometric NaN and signed infinity explicitly.
There is no additional drawing, color, integer, cache, or endpoint allowance.

| Disposition | Rows |
| --- | ---: |
| Derived comparisons | 84,626 |
| Supplied genotype checks | 16,740 |
| Organism collaborators | 198 |
| World collaborators | 388 |
| Selection metadata | 0 |
| Genotype availability metadata | 491 |
| Inputs, schema, and commands | 4,461 |
| Total | 106,904 |

L142 and D1 are derived observations.
P97 snapshot markers are ordering metadata.
Organism collaborator exclusions are exactly `_world` and `_visibleWorld`.
World collaborator exclusions are exactly `_visibleWorld`, `inCorridors`, `outCorridors`, and `_corridorexists`.
There are no private candidate exclusions or unnamed observation exclusions.
Provenance includes the complete 27-target typed input inventory.

## Real UI and isolation

The helper validates four explicit private profile roots before Preferences access.
It requires the owned Xvfb environment marker and an isolated display number with at least three digits.
It invokes actual MainWindow.main on the Swing EDT.
It requires a paused real MainWindow, its actual InfoToolbar, and no server thread.
It cancels the real simulation timer before case execution.
Each new World uses the real VisibleWorld and becomes the actual MainWindow's World.
Setup, operations, and snapshots run on the EDT.

`ui.tsv` records actual selection and InfoToolbar energy, age, and mass labels after each snapshot.
`bootstrap.json` records the real collaborator classes, paused state, timer cancellation, and owned display.
All three captures produce identical sidecars.
These sidecars remain outside the core comparison ledger and do not establish Rust UI parity.

Compile `tools/reference/DrawOracle.java` against each pinned backend.
Run `biogenesis.DrawOracle NEW_OUTPUT_DIRECTORY` with the exact isolated arguments recorded in provenance.
The raw run is `.workflow-local/draw-oracle-9efpussl/`.
The native capture is `.workflow-local/draw-native-rea.json`.
Positive calls have a 120-second bound; compilation uses sixty seconds; negative calls use thirty seconds.
Six actual guards reject four omitted roots, an escaped preferences symlink, and an existing output directory.
The existing-output fixture remains unchanged after rejection.

Cleanup cancels remaining MainWindow timers and disposes every Window on the EDT.
Secondary cleanup failures remain attached to the original error, and the helper exits with the actual status.
The runner cleans every owned Java/Xvfb process group.
Native capture verifies its owned group cleanup; it does not prove full-host process absence.
All 74 original hash/size/mode pins, 159 compiled reference classes, and 37 prior TSV files remain unchanged.
The accepted Frame helper and artifacts remain unchanged.
No failed producer compilation or scenario required supersession for this corpus.

These artifacts establish actual original draw-command observations.
Rust renderer parity requires separate replay and review.
Corridor drawing, pixels, antialiasing, clipping fidelity, hardware, full presentation, and player journeys remain separate acceptance gates.
