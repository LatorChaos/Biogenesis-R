# Frozen spatial query observations

`spatial.tsv` records actual queries from the embedded JTS 1.15.0 library.
Two unchanged-source runs and one bundled-game-JAR run produced identical bytes.
The fixture has 113 cases, 20,282 insertion records, and 3,740 queries.
The largest case contains 12,500 entries.

`tools/reference/SpatialOracle.java` creates a real `World(null)` for each case.
It assigns the real JTS tree to `World.colDetTree` and queries that field.
It executes no organism frame, contact, frontend callback, or network operation.
Source and JAR runs use their own compiled helper and game classes.
Both use the exact JTS bytes extracted from the bundled game JAR.

## Scope and input semantics

Each input uses four Java `Rectangle` integer fields.
The helper constructs `Envelope(getX(),getMaxX(),getY(),getMaxY())`, matching World spatial methods.
`getMaxX()` and `getMaxY()` widen before addition.
These additions therefore retain values beyond the signed integer range.
Negative dimensions normalize into reversed endpoint envelopes.
Zero dimensions remain point or line envelopes.
Envelope intersections include touching edges and corners.

These candidate rules differ from subsequent live `Rectangle.intersects` checks.
The Rust tree only returns tokens.
Its caller must retain live objects and apply the correct live-body filters.
The tree stores frozen envelopes and does not observe later rectangle mutations.
It retains duplicate token occurrences and their positions.
It does not manage object lifetime, energy, parent exclusions, or newborn scheduling.

## Ordering

The port uses capacity 10, stable center-X sorting, vertical slices, and stable center-Y sorting.
Each slice forms groups of at most ten children.
Parent envelopes repeat the same packing until one root remains.
Queries traverse children depth first in their stored order.
Envelope centers use `(minimum + maximum) / 2`.

The source JTS tree builds lazily upon its first query.
The Rust tree builds eagerly after receiving the complete insertion sequence.
This representation change preserves the frozen frame's query results.
Neither build process consumes simulation randomness.
No post-build insertion or removal API exists in the Rust tree.

## Cases and checks

Entry counts include 0, 1, 9, 10, 11, 20, 21, 99, 100, 101, 257, 1001, and 12500.
Generated cases exercise tied X centers, tied Y centers, and mixed centers.
Additional cases tie both centers while retaining different extents.
Forward, reversed, and seeded shuffled insertion orders protect stable tie behavior.
Explicit cases cover closed boundaries, negative dimensions, zero area, integer extremes, and duplicate tokens.

Every query records the actual ordered JTS result.
Java also checks its result multiset against a linear Envelope-intersection scan.
This invariant checks membership without deriving expected query order.
Every Java query repeats and checks immutable results.
Rust compares exact normalized envelope bits and exact token sequences.
Rust repeats each query and separately checks frozen ownership and duplicate tokens.

The focused Rust tests pass in debug and optimized builds.
A private capacity-16 mutation fails the 11-entry ordering case.
Its actual order differs from the frozen capacity-10 observation.
Oracle negatives reject missing isolated profiles and an existing output directory.
The existing output remains byte-identical after rejection.

## Schema

Rows use tab-separated ASCII and one newline per record.
Empty result lists use `~`; nonempty results use comma-separated integer tokens.
Envelope doubles use sixteen hexadecimal raw-bit digits.

| Tag | Columns after the tag |
| --- | --- |
| `M` | `JTSVersion`, actual library version. |
| `C` | Case name. |
| `I` | Token, x, y, width, height, minXbits, maxXbits, minYbits, maxYbits. |
| `Q` | Query name, x, y, width, height, minXbits, maxXbits, minYbits, maxYbits, ordered tokens. |
| `E` | Case terminator. |

## Provenance and licensing

`spatial.provenance.json` records hashes, commands, raw receipts, and source preservation checks.
The authoritative algorithm evidence is the exact embedded JTS 1.15.0 bytecode.
Read-only `javap` output covers STRtree, its comparators, AbstractSTRtree, and Envelope.
Current official upstream source provides an additional implementation and attribution cross-check.
Attempts to fetch tagged 1.15.0 source returned cache misses.
Current upstream source is not presented as the pinned 1.15.0 source.

The embedded manifest declares EPLv1 and EDLv1 license alternatives.
The official 1.15.0 release confirms that license choice.
The port selects EDLv1, the BSD three-clause alternative.
`spatial.rs` retains Vivid Solutions and Eclipse notices, conditions, and the disclaimer.
Binary distributions must retain this notice in their attribution materials.
No JTS runtime dependency enters the Rust product.

Primary references:

- [JTS 1.15.0 release](https://github.com/locationtech/jts/releases/tag/jts-1.15.0).
- [Official EDLv1 text](https://raw.githubusercontent.com/locationtech/jts/master/LICENSE_EDLv1.txt).
- [Official STRtree source](https://raw.githubusercontent.com/locationtech/jts/master/modules/core/src/main/java/org/locationtech/jts/index/strtree/STRtree.java).
- [Official AbstractSTRtree source](https://raw.githubusercontent.com/locationtech/jts/master/modules/core/src/main/java/org/locationtech/jts/index/strtree/AbstractSTRtree.java).
- [Official Envelope source](https://raw.githubusercontent.com/locationtech/jts/master/modules/core/src/main/java/org/locationtech/jts/geom/Envelope.java).

## Limits

The API accepts integer rectangles; arbitrary floating, null, infinite, and NaN envelopes remain outside its contract.
The fixture does not accept complete World frames, contact behavior, or same-frame object-lifetime integration.
The 12,500-entry case proves query parity for that input, without establishing a performance budget.
Nearest-neighbor search and dynamic JTS insertion or removal are outside this frozen-tree port.
