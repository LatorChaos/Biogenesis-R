# Compact original workload trajectories

This fixture records the actual original game at frames 0 through 80.
It covers initial creation attempts 0, 32, 512, and 12500.
Every cohort uses seed 827364 and a 7000 by 7000 world.

The corpus contains 324 snapshots and 320 actual `World.time` calls.
It has 19,432 rows and 19,764 checked obligations.
The current Rust gate compares 19,116 runtime values and checks 648 Gaussian source invariants separately.
Its initial consumer compared 18,792 runtime values and 972 invariants before the real corridor flag became observable.
The 324 corridor rows now compare that actual runtime field; observation bytes and the total denominator remain unchanged.
Source runs twice and the bundled JAR produces identical bytes.
All three compiled helper classes also match.

## Initialization and call order

The helper invokes actual `MainWindow.main` with the original seed argument.
It supplies the exact ten preferences from `ReferenceWorkload.java`.
The original application creates each initial world through natural genesis.
No observed state initializes a world, actor, statistic, or generator.

Each run uses a new profile, owned Xvfb display, and real `InfoToolbar`.
The application stays paused and offline.
The helper cancels its presentation timer before observation.
The EDT executes sixteen time calls, followed by sixty-four time calls.
Snapshots occur before the first call and after each call.

All 147 active primitive settings match the four earlier workload settings.
The final world serialization also matches each earlier hashed `world-after-80.bgw` payload.
This final hash check occurs after all observations.
It never supplies constructor inputs or expected scalar values.

## Protocol

| Tag | Columns after tag | Meaning |
| --- | --- | --- |
| C | name, attempts, seed, width, height, warmup, following | Supplied workload identity and call groups |
| P | name, Java type, value | Supplied original preference input |
| U | name, Java type, value | Actual active primitive Utils setting |
| X | owner, nonprimitive_fields, names | Explicitly omitted reference fields |
| K | frame, time | One actual time call producing that frame |
| V | frame, name, Java type, value | All nine primitive World fields |
| S | frame, name, Java type, value | All 37 primitive WorldStatistics fields |
| W | frame, name, Java type, value | Nine actual scalar getter or membership counts |
| R | frame, seed48, haveGaussian, cacheBits, nextLong, int10000, doubleBits | Actual generator state and copied suffix |
| Q | frame, random_serialization_pure, true, world_statistics_scalars_pure, true | Checked observer purity |
| E | none | Case end |

The provenance contains the exact V, S, and W schemas.
W records population, member count, corpses, gases, NEXT_ID, frame, and statistics time.
World frame fields retain their original int and long distinctions.
Every double uses sixteen raw hexadecimal bits.
Other integers retain their declared signed Java widths.
The actual generator seed contains 48 unsigned state bits within a Java long.

## Generator observation

The helper serializes the actual `Utils.random` object without drawing from it.
It accepts only the exact original `java.util.Random` class.
A bounded deserialization creates a separate Random object.
Its actual `Random.readObject` calls the observer's `readFields` override.
The override captures supplied seed, Gaussian flag, and Gaussian cache fields.
It rejects defaulted or missing state fields.

The copy must serialize identically before suffix observation.
Only that copy supplies nextLong, nextInt(10000), and nextDouble, in that order.
The original object and its serialized bytes must remain unchanged.
Every World and statistics scalar must also remain unchanged during observation.
No module overrides or private JDK field reflection are used.

All observed Gaussian flags are false and all caches contain positive zero.
This corpus does not exercise Gaussian cache transitions.
Java serialization canonicalizes NaN double payloads.
No observed cache contains NaN, so that behavior loses no observed state here.
Arbitrary unserialized NaN cache payload preservation is outside this fixture.

## Fixed ledger

| Tag | Rows |
| --- | ---: |
| C | 4 |
| P | 40 |
| U | 588 |
| X | 8 |
| K | 320 |
| V | 2916 |
| S | 11988 |
| W | 2916 |
| R | 324 |
| Q | 324 |
| E | 4 |

The source value count is 324 times 61 values.
Each snapshot supplies nine V, 37 S, nine W, and six R values.
Rust compares 58 actual runtime values per snapshot.
The source Gaussian flag, Gaussian cache, and ordinary-world corridor flag form three additional invariant checks.
These 972 invariant checks do not fabricate Rust Gaussian or corridor fields.
The 588 U values are configuration inputs and validation metadata.
Q records source observer assertions; it does not establish Rust behavior.

## Verification and limits

Eight profile, cohort, and display guards fail before accepted data generation.
Two compiled observer mutants also fail through actual original application calls.
One consumes the original RNG; the other advances the original World clock during observation.
Both actual failures preserve their diagnostics.

Native jobs use private homes, preferences, temporary directories, and cache directories.
Positive JVM heaps are capped at 512 MiB and each process has a ninety-second deadline.
Per-cohort text output is bounded to 2 MiB.
Random deserialization limits depth to four, references to sixteen, and bytes to 1024.

All 74 originals, 159 reference classes, and prior fixtures retain their pins.
Both native jobs verify their owned root process groups.
Their Python drivers also check child group absence in the same native context.
Unrelated inaccessible processes remain explicit unknowns.
Filesystem captures are snapshots, not syscall traces.
Selected digest-budget omissions do not replace the driver's complete preservation checks.

No actor state, geometry, genotypes, membership order, identity topology, or stored tree is recorded.
The fifteen nonprimitive statistics fields are also omitted explicitly.
Matching these signals does not establish complete world or trajectory fidelity.
The earlier complete endpoint fixtures remain a separate gate.
All steps precede the statistics clock rollover at frame 256.
No timing, hardware, presentation, control-journey, corridor, or networking acceptance is claimed.

## Future Rust comparison

Initialize from C, P, and U inputs through the existing real World constructor and genesis route.
Never initialize from V, W, S, or R observations.
Compare the initial state before advancing any frame.
Then execute one actual frame for every K record.
Compare available runtime scalars, field widths, actual RNG state, and clone suffixes exactly.
Check the three source-only invariants separately while retaining the complete denominator.
No geometry or floating tolerance applies to this compact corpus.
Inspect suffixes using a separate clone of the actual Rust generator.
Never reseed or restore the generator between frames.
Keep the broader endpoint, detailed frame, and performance gates separate.
