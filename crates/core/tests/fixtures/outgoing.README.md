# Original outgoing transfer and paint observations

Date: 2026-10-26. Scope: original Java observations only. Publication: private, pending lead review.

Both corpora use real pasted actors, explicit genomes, and the original outgoing methods.
The real paused MainWindow supplies its real VisibleWorld and InfoToolbar collaborators.
Each case starts with three independently pasted and added actors: selector, victim, and killer.
The selector remains selected to avoid the unrelated null-selection callback defect.
No expected actor is synthesized from observation rows.

## Frozen corpora

| Corpus | Cases | Snapshots | Actor views | K operations | Original method operations | Rows |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| outgoing.tsv | 32 | 175 | 525 | 129 | 95 | 185514 |
| outgoing_frames.tsv | 3 | 27 | 81 | 19 | 15 | 27438 |

The core corpus contains 34 recording-boundary allocations and 34 real OutCorridor constructor calls.
The frame corpus contains four recording-boundary allocations and four real OutCorridor constructor calls.
K counts include those explicitly controlled boundary allocations.
No original Connection constructor executes.

Core ledger: 166432 eligible observations, 11200 supplied genotype rows, 1400 collaborator exclusions, and 6482 metadata rows.
Frame ledger: 25498 eligible observations, 1020 supplied genotype rows, 216 collaborator exclusions, and 704 metadata rows.
Eligible rows include named logical ownership projections and recording-boundary observations.
They are a pre-comparison denominator, not evidence of Rust parity.

## Input and observation protocol

C names the case, mode, and seed.
U records all 147 primitive Utils settings before actor construction.
I records explicit settings, actor controls, corridor bounds, list membership, connection state, and boundary failure controls.
USER_NAME is explicitly "oracle" in every case.
No real username supplies source expectations.

Five G/N input blocks supply g0 through g4.
The first four use explicit construction; g4 is an actual clone of g0.
A records real Organism construction, paste coordinates, and genotype identity.
Only supplied input blocks, A, U, I, and K define execution inputs.
Later O/H/G/N/W/V/S/T/F/CC rows are observations.

P names a snapshot.
O records all 154 organism fields, including Rectangle bounds.
H records primary, infected, and saved genotype identities.
W records ten world getters and counters.
V records all 15 World instance fields.
S records all 52 statistics fields; T records all 49 public zero-argument statistics getters.
Q records actual JTS query results using actor identities in returned order.
J records actual stored envelopes after operations without rebuilding the tree.
Y records genotype reachability; G/N record actual genome and gene state.
R records nextLong, nextInt(10000), and nextDouble bits from a serialized copy of the shared Random.
The original Random does not supply those suffix draws.
B records actual send, move, capacity, or corridor-selection outcomes.
X records actual exception class and the configured JVM's message.

F has six fields: _reference, bounds, connection, travellingOrganism, world, and canSendOrganism.
Identity labels use actual object identity maps; numeric organism IDs never determine identity.
OutCorridor and Organism dispatch precede Rectangle dispatch.
Repeated corridor list entries retain the same object identity.

CC has three fields: _reference, state, and recordedGenotypes.
state comes from the original Connection.getState method.
recordedGenotypes contains exact attempted send arguments, including arguments preceding the deliberate callback exception.
It does not mean queued, delivered, acknowledged, serialized, or remotely placed genomes.
F.world, F.connection, F._reference, CC._reference, and V.worldStatistics have explicit logical ownership interpretations.

L has nine columns: tag, stage, ordinal, operation, x, y, x2_or_width, y2_or_height, and signed ARGB.
Operations are drawLine, drawRect, and fillRect.
A real BufferedImage Graphics delegate starts WHITE before each paint.
The recording wrapper records calls and the delegate's current getColor().getRGB value.
It delegates every abstract Graphics operation and the recorded drawing operations.
It does not implement organism colors, coordinate rules, corridor transitions, or drawing selection.
Null setColor calls retain the real delegate's inherited color.
Paints receive additional before-step snapshots after explicit controls.

## Boundary allocation and operation meanings

connection.new allocates the recording subclass without executing any constructor.
The installed JDK's sun.misc.Unsafe.allocateInstance performs this narrowly scoped test allocation.
The subclass overrides only send(GeneticCode).
Its private constructor is never called.
Explicit setup initializes its recording list and original state field to 51.
Every snapshot verifies absent Connection timers, UI owner, and constructor-created corridor fields.
No setState, keepalive, NetSend, socket, or protocol method runs.

corridor.new invokes the real OutCorridor constructor with the actual World and known recording connection.
send invokes its original sendOrganism method.
canSend invokes its original canSendOrganism method.
checkHitCorridor invokes the original World.checkHitCorridor method.
move invokes the original Organism.move method.
die invokes the original Organism.die(null) method.
draw invokes the original OutCorridor.draw method with the real delegate.
world.draw invokes the original World.draw method with the real delegate.
time invokes one complete original World.time call in the separate frame corpus.

## Covered outcomes

Core cases cover constructor RNG, empty capacity, disconnected state, state 52, full capacity, and first-containing shadowing.
Containment covers lower edges, exclusive upper edges, zero width, and wrapping positive-width integer edges.
Motion covers stationary, ordinary movement, zero-velocity eyes, inactive actors, and already-dead active actors.
Border response occurs before successful transfer and prevents subsequent rollback.

Send cases cover ordinary, existing-colon, empty, and null clades.
They cover sufficient, scarce, and empty oxygen, zero energy, repeated dead-actor accounting, and population wrapping.
The recorded genotype remains the exact primary genotype after subsequent clade, generation, and gene-length mutation.
The synchronous recording exception preserves the earlier clade prefix but prevents later slot, energy, death, and population changes.
It is a controlled transport-boundary exception, not an observation of TCP failure.

Paint cases cover ratios 13 through 17, repeat paints without ticks, and exact release timing.
A paint starting at 15 reaches 16 and remains occupied.
The next paint clears capacity without another rebuild, rotation, or actor draw.
Cases cover temporary tint, exact eye exemptions, unknown/translucent ARGB, corpse brown, and inherited null-tint colors.
World.draw observes repeated corridor references sequentially, followed by still-present world members.
Direct send cases deliberately retain their original live-list membership until an actual frame removes actors.

Frame cases observe real synchronous transfer removal, later ticks without paint, two outgoing slots, and paused World paints.
The removed actor remains the exact traveller and frozen-tree object after its transfer frame.
The following frame rebuild removes its old tree entry while retaining the traveller.
One nearby later actor also queries the removed traveller through the same frozen tree.
These are three concrete corridor-frame observations, not exhaustive full-frame or contact coverage.

## Comparison contract and exclusions

Before Rust comparison, retain only the established geometry allowance for O _m, _m1, _m2, _mass, and _I.
All other scalar bits, gas values, counters, endpoints, ARGB, order, aliases, and RNG suffixes remain exact.
No numerical tolerance was used for source/source/JAR equality.
The ledger excludes O._world, O._visibleWorld, V._visibleWorld, and V.inCorridors.
The original World and UI references remain recorded for provenance.
Incoming corridors, Connection construction, network concurrency, keepalive, and transport delivery are outside this corpus.
The UI sidecar records selected identity and toolbar text separately; it establishes no Rust UI comparison.
Recorded drawing commands establish no pixel, hardware renderer, presentation-rate, or performance acceptance.

## Executed evidence

driver.py --compile-only compiled against unchanged source and the bundled JAR.
All seven generated helper classes matched byte-for-byte.
driver.py then ran isolated source, repeated source, and bundled-JAR captures with bounded process groups.
Its first strict comparison failed only on an enriched null-pointer diagnostic message.
The unchanged source class named "<parameter1>"; the bundled JAR named "org".
native and rea-capture.json preserve that failed comparison and every original output.
npe-diagnostic-difference.json records its only differing row.

The repeated capture used the explicit supported JVM flag -XX:-ShowCodeDetailsInExceptionMessages for every backend.
The helper stayed unchanged.
The original NPE still occurs before sending or mutation beyond supplied inputs.
native-v2 records exact source/source/JAR equality for both complete corpora.
No enriched message was rewritten or normalized by the observer.
Six guards rejected missing profile roots, escaped profile storage, and an existing output directory.
Every owned command group was absent in its own controller context.
The native capture independently verified its owned root group cleanup.
One unrelated process environment was unreadable and remained untouched.
No full-host absence claim is made.

The driver preserved 74 original hash/size/mode pins, 159 reference classes, and 585 pre-existing fixture files.
It also preserved original JAR, JTS, Java, javac, helper, and runner hashes.
No product files, original sources, canonical fixtures, instructions, tracker records, or tool configuration changed.
No Git operation occurred.

Read ledgers.json for exact schemas, typed targets, exclusions, and operation counts.
Read provenance.json for hashes, actual argv, receipts, guards, preservation, and native evidence IDs.
Read observed-outcomes.json for indexed source results without deriving new expected values.

Rust comparison, independent generator review, real transport, and full network player journeys remain pending.
