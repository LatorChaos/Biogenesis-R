# Selected-donor callback ordering diagnostics

`infection_callback.tsv` records three actual source failures through a real selected donor and a headless VisibleWorld.
Unchanged source twice and the bundled JAR produced identical bytes.
The fixture contains three cases, six snapshots, and three actual calls.
Every actor is actually pasted and registered with a nonnull primary genotype.
No invalid actor allocation, fake World, or substituted statistics object is used.

Run `biogenesis.InfectionOracle NEW_OUTPUT_DIRECTORY callback-diagnostic` with the isolated arguments recorded in the provenance.
The fixture retains the [canonical infection schema](infection.README.md).
It adds `X(stage,exception_class)` observations.
Each actual call records `java.lang.NullPointerException` without a runtime-dependent exception message.

The helper's real VisibleWorld has no MainWindow.
Peer is selected and becomes the actual donor for each diagnostic call.
The methods are infectedBy, infectedByFruit, and infectedByFruitWeak.
World first updates its real statistics, then tries the selected-donor MainWindow callback.
That actual callback throws before the infection method assigns its copied or fruit threshold.
The helper catches only this explicitly diagnostic NullPointerException and records the resulting partial state.

Every case observes donor infections increase from zero to one.
The victim receives the exact donor genome reference and clears its saved code.
Actual infection statistics increase from zero to one and retain donor genome `g2` as their record.
The victim's infected reproduction threshold remains the explicitly supplied `314`.
Those observations independently expose notification-before-threshold ordering without a fake callback probe.
The Java source also specifies that order directly.

The missing MainWindow is an unsupported source UI collaborator in this diagnostic setup.
Do not describe these three calls as successful infections or passed Rust UI behavior.
All 9,219 rows remain source diagnostics with zero canonical Rust comparisons claimed.
Preserve the exceptions and partial state for future compatibility review.
Canonical infection calls avoid this callback branch using the separate selected peer.

There are 2,772 O rows, 312 statistics-field rows, 294 getter rows, and 5,220 G/N rows.
The remaining 621 rows preserve controls, settings, actor recipes, references, world state, RNG, commands, and framing.
The provenance records the complete tag ledger and source/source/JAR equality.

This diagnostic does not test a real MainWindow, rendered UI, contact eligibility, a full simulation frame, or networking.
It does not require Rust to emulate an absent-UI NullPointerException.
The original files and all earlier fixtures remain unchanged.
