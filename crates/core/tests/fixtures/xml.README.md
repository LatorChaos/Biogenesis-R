# Legacy genotype XML observations

`xml.tsv` records actual `BioXMLParser.parseGeneticCode` and `BioXMLParser.writeGeneticCode` calls.
`xml/cases/` contains every referenced input, output, partial output, and owned security marker.
The fixed corpus has 261 cases, 587 method calls, 443 successes, and 144 failures.
It contains 327 parses, 260 writes, 184 parsed-genome snapshots, and 76 writer-input snapshots.
Every snapshot records all 28 genome observation fields and all 40 fields per gene.

Two source runs and the bundled JAR produce identical TSV and case-file bytes.
All three compiled helper classes match across backends.
German and Turkish profiles preserve every XML file and all non-diagnostic rows.
The Turkish profile also changes the process default charset to ISO-8859-1.
German localizes 32 exception messages; those actual diagnostics remain preserved separately.

## Source and execution boundaries

Authority is `BioXMLParser.java:43-592`, `Gene.java:35-142`, and `GeneticCode.java:641-675`.
The existing IO-009..015 contract and its attribute/reaction tables remain authoritative.
No Rust formula, parser, expected-state constructor, GUI, world, or socket supplies observations.

Each case resets every mutable Utils field with a matching DEF field.
It then disables networking, backups, and initial organisms before recording all 147 primitive globals.
Two explicit mutation-preference overrides occur in one labelled case.
The per-case seed is 827364.
R observations copy the actual Random without advancing it.
Every method leaves its case RNG suffix unchanged.

The helper checks four isolated profile roots before original simulation or parser access.
The runner removes Java option and classpath injection variables.
Headless positive processes have 512 MiB limits and 120-second bounds.
Compiles use sixty seconds; six isolation guards use thirty seconds.
Only owned process groups are cleaned.
No always-on process or shared client activation is involved.

## Protocol and input authority

| Row | Meaning |
| --- | --- |
| C | Case name, parse/write operation family, seed |
| U | Original mutable primitive setting, Java type, actual value |
| F | Stage, supplied/owned/written/partial role, relative path, SHA256, byte count, encoding label |
| K | Stage, actual parse/write operation, relative path |
| O | Actual method success or failure |
| X | Stage, actual exception class, Base64 UTF-8 diagnostic |
| G | Stage, genotype holder, genome field, observed value |
| N | Stage, genotype holder, gene ordinal, gene field, observed value |
| A | Stage, gene_alias_roots, first-object-slot indices |
| T | Stage, Base64 actual document-element text |
| R | Stage, copied-generator long, bounded integer, raw double bits |
| V | Stage, owned external-resolution event, owned filename |
| E | Case end |

F paths resolve relative to `xml/`, rather than the TSV's containing directory.
`DEFAULT` means the actual process default charset; environment receipts identify it.
Double fields use sixteen raw hexadecimal bits; Color uses eight complete ARGB digits or `~`.
Strings use `b64:` followed by Base64 UTF-8; `~` denotes null.
Empty alias vectors represent zero genes.

Only writer cases have `input` G/N/A records.
Those records describe actual supplied recipe objects before any write.
Parse cases consume their supplied F bytes, without using observed genomes as inputs.
Later parse calls consume the preceding actual writer output.
A future bidirectional Rust replay must consume its own writer output at those steps.
Expected Java output files must not replace failed or missing Rust writes.
An independently declared reader-only replay can instead treat immutable Java documents as its decoder corpus.

## Observed source-accepted subset

The source validates all 23 root attributes and parses all 40 gene attributes.
All 46 lowercase base colors round-trip.
Boolean one-hot cases distinguish every permission, including mirror and generation battle.
Numeric root ranges retain their exact accepted endpoints.
Mutation rate accepts 10000 even when explicit preferences are 2222..3333.
No clamp applies to branch or reaction integers within signed i32.
All 36 reaction fields carry distinct signed values, including values outside laboratory bounds.
Signed i32 extrema survive; overflow, decimal integers, hexadecimal integers, and surrounding integer spaces fail.
A leading plus on an integer succeeds.

Lengths and theta accept negative values, signed zero, NaN, infinity, and tested hexadecimal/suffixed double syntax.
Double parsing accepts the tested surrounding spaces and overflow/underflow forms.
Lowercase `nan`, lowercase `inf`, comma decimal syntax, and empty doubles fail.
The corpus records exact canonical Java writer decimals and parsed bits.
It does not establish every Java numeric lexical form.

Comments and whitespace succeed.
Declared UTF-16 input and ISO-8859-1 input with a non-ASCII comment succeed.
Ordinary exported attributes remain ASCII under all tested process locales and charsets.
Writing UTF-16 bytes beneath the writer's fixed UTF-8 declaration succeeds as a write, then fails parsing.
The writer uses platform line endings; this Linux capture records LF.

Internal DTD defaults can supply one attribute or all 63 root/gene attributes.
A bounded internal entity can supply a double attribute.
Omitting attributes with #IMPLIED declarations instead exposes the source's missing-value failures.
Canonical #REQUIRED omission fails validation earlier.
An absent DOCTYPE, duplicate attribute, wrong root, and wrong child fail.
Canonical gene+ rejects zero genes.
A supplied gene* DTD permits zero genes, but canonical re-export fails its later reparse.
The source accepts 129 genes without an editor-derived limit.

Each XML parse creates fresh genome and gene objects.
A shared two-slot input gene becomes two independent parsed genes.
Lineage generation 91, a NUL/non-ASCII clade string, and update-clade true disappear from XML.
Parsed lineage becomes generation zero, null clade, and update-clade false.
Raw mirror -7 writes as yes and parses as 1.
These are actual source format losses, rather than lossless persistence guarantees.

## Actual failures and security diagnostics

All eighteen internal colors, unknown ARGB, translucent ARGB, and alpha-zero ARGB write an empty color name.
Their subsequent parses fail.
A null color instead throws NullPointerException after writing a partial document.
The partial bytes remain explicit F evidence; they are not valid save artifacts.
The corpus records 143 SAXException outcomes and one NullPointerException outcome.

The first actual candidate preserved helpful-NPE messages from both backends.
Source says `<parameter1>`; the older bundled class says `c`.
Canonical profiles disable only JVM helpful-NPE detail using `-XX:-ShowCodeDetailsInExceptionMessages`.
The actual failure class and failed write remain unchanged.
This process setting avoids treating compiled local-variable names as a format contract.
Original detailed diagnostics and all prior candidate artifacts remain preserved.

Three security cases reference only owned local DTD or nonsecret marker files.
The resolver verifies exact canonical parent and allowlisted filename, then returns null for original file resolution.
Every other resource is denied before resolution.
The process allows the file DTD protocol explicitly; external schema access is disabled.
These runs do not establish the unmodified JVM's default access policy.
No external network URL, arbitrary host file, or network request is supplied.

An owned external DTD supplies mutation rate 321.
An owned external parameter entity supplies length 22.5.
An owned external general entity appears in actual DOM text.
These are exposure diagnostics, not Rust compatibility acceptance.
Do not reproduce external file resolution in the Rust decoder.

## Frozen row ledger

This complete ledger precedes any Rust replay.
It counts source observations without claiming that Rust has compared them.

| Disposition | Rows |
| --- | ---: |
| Settings, schema, and commands | 39,476 |
| Supplied writer model checks | 10,364 |
| Supplied asset checks | 188 |
| Actual parsed-model observations | 17,816 |
| Actual method outcomes | 587 |
| Actual output-file checks | 260 |
| RNG observations | 848 |
| Source diagnostic metadata X/T/V | 331 |
| Total | 69,870 |

O acceptance/rejection and parsed fields are mandatory compatibility evidence within the selected safe subset.
X messages, T DOM text, and V resolution traces remain source diagnostic metadata.
External-resource cases require an explicit security-correction disposition before a Rust replay ledger is adopted.
Unsupported-color and partial-write behavior require explicit writer-boundary dispositions.
No source failure can become a silent successful Rust parse or save.

## Proposed safe Rust decoder contract

Parse complete owned input bytes before changing editor or world state.
Preserve all metadata, ordered genes, signed integers, accepted doubles, and fresh XML object identities.
Keep placement and geometry validation separate from genotype decoding.
Recognize supported encodings and validate declarations against actual bytes.
Bound input bytes, nesting, attributes, genes, text, and internal expansion work.
Do not introduce a silent 100-gene cap.
Reject external DTDs and external entities without filesystem or network resolution.
Preserve supported safe internal defaults/entities through an audited parser route.
Do not mistake the canonical writer's gene+ declaration for every source-accepted content model.
Return typed malformed, unsupported, resource-limit, or encoding errors without replacing the previous valid state.

For export, preflight unsupported/null colors and invalid metadata before committing a file.
Use explicit UTF-8 and a matching declaration.
Close, flush, and report failures; never commit the source's partial null-color output.
Record these safety corrections separately from exact legacy parsing behavior.
Parser selection, exact resource budgets, writer error policy, and full compatibility acceptance remain lead-owned decisions.

## Evidence and limits

Final receipts are under `.workflow-local/xml-oracle-final-ur2q6pwj/`.
Source/source/JAR outputs, helper classes, six guards, 74 originals, 159 source classes, and 38 prior TSVs passed preservation checks.
The prior 260-case TSV is an exact prefix; all its case files remain unchanged.
The initial compile failure and the earlier backend-specific diagnostic difference remain preserved.

This corpus covers the listed concrete XML forms, not every possible DTD, encoding, or numeric spelling.
It does not test file-dialog commits, disk failures, laboratory UI, organism placement, or world persistence.
It does not establish Java object-stream, socket, corridor, performance, or player-journey acceptance.
No Rust implementation, dependency adoption, product code, shared manifest, or tracker changed.
