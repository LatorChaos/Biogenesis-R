# Complete workload endpoint observations

`workload_endpoints.tsv.gz` contains complete canonical observations from four existing frame-80 Java World snapshots.
It contains no new Java simulation run.
The inputs are the previously captured workloads with attempts `0`, `32`, `512`, and `12500`.
Each original workload used seed `827364`, dimensions `7000x7000`, sixteen warmup frames, and sixty-four measured frames.
Endpoint statistics time is zero and frame is eighty.

Each trusted input was decoded twice through pinned source classes and once through the bundled JAR.
All twelve decodes succeeded.
For each workload, all three canonical outputs match byte for byte.
All four compiled helper classes also match across the backends.
No hash equality alone establishes Rust fidelity.
The canonical records retain individual fields and edges for first-field diagnostics.

## Inputs and settings authority

Original inputs remain under `.workflow-local/reference-run-dve5ksyn/workload-source-N/fixtures/world-after-80.bgw`.
The corresponding `workload-jar-N` payloads have identical verified bytes.
Provenance pins all eight input files and the four source preference files.
The helper accepts only the four exact input hashes and their matching preference hashes.
It never treats a final observed field as a construction input.

U147 records reconstructed inputs from original source defaults and each workload's ten pinned preference entries.
The original workload did not serialize global settings.
Its preferences explicitly set attempts, dimensions, disabled networking/backups, software graphics, English, delay twenty, and preference version 700.
The source defaults supply complexity two, CO2 500000, O2 zero, and CH4 zero.
Preference VERSION prevents source preference clearing; it does not replace the source VERSION constant.
LOCALE is presentation configuration outside U's primitive field schema.
No Preferences API or XML entity resolver runs during decoding.
The helper parses only the exact hashed flat preference bytes.

No supplied genotype, organism, intermediate snapshot, or expected output seeds a Rust World.
A future consumer must construct the actual seeded World and execute eighty complete ordinary frames.
The separate GUI startup snapshot is unsuitable as this workload's initial checkpoint.
That GUI journey used `512x400`, CO2 `100000`, and O2 `5000`.

## Serialization inspection and bounded decoding

A data-only serialization token parser inspected all descriptors before any new target deserialization.
It executed no target method and loaded no target class.
The four streams contain twenty distinct serialized descriptor classes in total.
The dense stream contains 773700 handles, maximum token depth 39, and maximum explicitly serialized array length sixteen.
ArrayList's later internal allocation is separately bounded during actual Java decoding.

The application-specific read hook is World.readObject.
It calls defaultReadObject and creates new empty transient corridor lists.
The observed JTS classes declare no custom serialization hooks.
Standard JDK collection hooks remain subject to the exact class and resource filters.
The strict resolveClass allowlist contains the twenty inspected stream classes.
Proxy class resolution is rejected.
The runtime filter additionally permits only Object[] collection allocation and SynchronizedRandomAccessList replacement.
These two runtime-only classes do not broaden accepted serialized descriptors.

Each input is capped at 36000000 bytes.
The ObjectInputFilter bounds depth at 64, references at 4000000, and array length at 20000.
Its stream-byte bound is 36000000.
Each process uses a 512 MiB Java heap limit and a 240-second timeout.
Canonical output is capped at 600 MiB per endpoint.
Output is streamed through a buffered ASCII writer.
The dense decode observed depth eighteen, 962313 references, and an internal array length of 13190.
Every positive decode completed in less than five seconds in the recorded run.
These decode timings are artifact-processing evidence, not simulation benchmarks.

## Canonical identity and complete fields

Actors receive `oN` labels from actual membership order, followed by tree-only identities in actual stored traversal order.
Numeric organism IDs never determine identity.
Genomes receive `gN` labels from actor holders, then statistics holders.
Genes receive `nN` labels from ordered genome slots.
Labels use actual Java reference identity, including equal-valued but distinct objects.

Each actor has O154 observations, including inherited Rectangle bounds, plus three H genotype holders.
O includes the serialized `_world` relation to root `w0`.
World has V15 observations and W10 getter-style observations.
Statistics has S52 fields and T49 actual getters.
Each genome has G28 observations; each genome slot has N40 Gene observations.
J connects every slot to its actual mutable Gene identity.
Shared Gene observations must agree across every recorded slot.
The independent readback verifies all field counts, types, identities, and alias consistency.

Rectangle values retain all four signed integer fields.
Primitive arrays retain their complete ordered values and raw floating bits.
Colors retain full nullable ARGB values rather than RGB-only classifications.
Arrays, lists, Rectangles, actors, genomes, Genes, World, statistics, and tree references retain explicit A edges.
Color and String reference identities also remain recorded, labelled immutable.
Their values remain exact obligations, while Java immutable allocation identity is explicit metadata.
Rust may represent logically independent containers through ownership without copying Java allocation layout.
Actual mutable sharing semantics still require a consumer check.
This producer claims no Rust alias parity.

M records actual serialized live-list membership, including corpses.
F records actual stored tree leaf order and each original Envelope's four raw double bounds.
The observer walks existing pending entries or packed children directly.
It never inserts, rebuilds, queries, or reconstructs the collision tree.
It verifies that the pending list, root reference, and built flag remain unchanged.
Tree-only actors remain fully observed with their own O/H/genotype fields.
Private JTS node allocation layout and synchronized-list mutex layout are not Rust representation requirements.
Their observable leaf order, envelopes, membership references, and sharing remain recorded.

## Record grammar

C has four columns: `C,workload-N,endpoint80,827364`.
U retains the existing primitive settings grammar.
O has six columns: `O,endpoint,oN,field,type,value`.
V has six columns: `V,endpoint,w0,field,type,value`.
W, S, and T retain their five-column endpoint field grammar.
G and N retain the existing five-column and six-column genome/slot grammars.
H retains the existing five-column holder grammar.
M has three columns: `M,ordinal,oN`.
F has seven columns: `F,leafOrdinal,oN,minXbits,maxXbits,minYbits,maxYbits`.
J has four columns: `J,gN,geneIndex,nN`.
A has seven columns: `A,ownerKind,ownerLabel,field,referenceKind,referenceLabel,mutability`.
Mutability is exactly `mutable` or `immutable`.
Container element edges use their zero-based index as field.
X has four columns and names each missing or reconstructed field category.
E closes each workload.
There are no constructor A rows, mutation I rows, K operations, Q queries, or R RNG suffixes.

## Absent state and comparison contract

World `_visibleWorld`, `inCorridors`, and `outCorridors` were transient and absent from the streams.
Organism `_visibleWorld` was also absent.
Their typed field rows contain `absent`, with matching X records.
Fresh empty lists created by World.readObject do not become claimed serialized corridor observations.
Static Utils.random, static settings, and Organism's static geometry scratch were not serialized.
X explicitly records these limitations.

The helper, canonical data, fixed ledger, and allowances freeze before Rust replay.
Compare all available scalar bits, integer bounds, cached endpoints, ARGB, reactions, membership, envelopes, order, and mutable sharing exactly.
Only O `_m`, `_m1`, `_m2`, `_mass`, and `_I` retain the existing geometry allowance.
For finite geometry, use `1e-10 * max(1, abs(expected))`.
Classify geometric NaN and signed infinity explicitly.
No new energy, gas, velocity, integer, tree, or statistical allowance applies.
Immutable Color/String identity edges remain explicit metadata; their values remain exact.
All other unavailable fields and metadata have named ledger dispositions.

| Attempts | Members | Tree entries | Union actors | Genomes | Unique Genes | Rows |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 0 | 0 | 0 | 0 | 0 | 290 |
| 32 | 32 | 32 | 32 | 32 | 49 | 10,437 |
| 512 | 541 | 541 | 541 | 522 | 801 | 170,687 |
| 12500 | 13,190 | 13,191 | 13,192 | 12,729 | 18,809 | 4,117,647 |

The dense population is 13170, while current membership also contains twenty corpses.
Two additional actors survive only through the frozen tree.
One current member is absent from that tree.
These differences demonstrate why aggregates cannot replace complete state and identity checks.

| Disposition | Rows |
| --- | ---: |
| Derived comparisons, including mutable sharing semantics | 4,144,017 |
| Immutable Color/String identity metadata | 126,882 |
| Absent transient field observations | 13,777 |
| Named absence and reconstructed-settings metadata | 13,789 |
| Input settings and record boundaries | 596 |
| Total | 4,299,061 |

## Compression, isolation, and evidence

The deterministic gzip uses level six, timestamp zero, no filename, and workload order `0/32/512/12500`.
Its compressed size is 22637114 bytes; the complete uncompressed stream is 188971386 bytes.
Independent streaming decompression verifies the full uncompressed hash and byte count.
This packaging adds no Rust dependency and avoids embedding 180 MiB in every test binary.
A runner can extract privately using Python's standard gzip module before a Rust std::io comparison.
Canonical uncompressed per-workload files remain available in raw receipts for immediate diagnostics.

Compile `tools/reference/WorkloadStateOracle.java` against each pinned backend.
Run `biogenesis.WorkloadStateOracle ATTEMPTS INPUT_WORLD PREFS_XML NEW_OUTPUT_DIRECTORY` with the exact arguments in provenance.
The helper requires four explicit isolated home, temporary, and preference roots before target decoding.
Eight actual rejection checks cover omitted roots, an escaping preferences symlink, existing output, unapproved class, and unapproved input bytes.
The class diagnostic serializes a local empty HashMap and exercises the same rejecting resolveClass boundary.
No unapproved endpoint reaches target deserialization.

The raw run is `.workflow-local/workload-state-cuj11xxh/`.
Each command uses the existing bounded owned-process-group runner.
That runner terminates its owned group on success, failure, timeout, or cancellation.
No GUI, Xvfb, timer, network service, or persistent process is started by this decoder.
There is no full-host absence claim.
All 74 original hash/size/mode pins, 159 compiled reference classes, and 38 prior TSV files remain unchanged.
All eight original binary snapshots and four supplied preference files remain unchanged.
No positive decoder or compilation failed in this capture.
Negative rejection receipts remain preserved.

These artifacts establish complete Java endpoint observations and source/JAR equivalence.
They do not establish intermediate trajectory, final RNG state, Rust fidelity, simulation speed, memory budgets, or presentation acceptance.
Those gates require separate actual execution and comparison.
