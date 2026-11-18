# Physics observations

`physics.tsv` records unchanged Java geometry and impulse behavior.
Source twice and the bundled JAR produced identical bytes.
The fixture contains 91 cases and 3,768 rows.
It includes 48 actual reflective calls to private `Organism.touchMove`.

## Comparison contract

Before Rust comparison, the declared contract required exact raw floating bits and exact booleans.
No geometry, impulse, clamping, or distance tolerance applies.
NaN signs and raw infinity values remain recorded and require exact bits.
Integer inputs, endpoint additions, rectangle dimensions, field order, and case order remain exact.
Nonfinite classification alone does not satisfy this contract.

## Execution and independent inputs

Compile `tools/reference/PhysicsOracle.java` against each pinned backend.
Run `biogenesis.PhysicsOracle NEW_OUTPUT_DIRECTORY` with the isolated profile arguments recorded in the provenance.
The helper rejects omitted profile roots and existing output directories.
The existing subprocess runner bounds execution and cleans owned process groups.

Geometry cases call actual `ExLine2DDouble.getIntersection`, `Line2D.intersectsLine`, `Rectangle.intersectsLine`, and `Rectangle.intersects`.
They also call actual `Point2D.distanceSq` for each segment endpoint.
No expected intersection, distance, or impulse formula is implemented by the helper.

Offset cases supply signed integer segment offsets and centers.
The helper constructs endpoints with ordinary Java integer addition before `setLine`, matching the source caller's expression.
The `X` rows preserve those supplied integer operands.
The `L` rows then record actual Java wrapping results before executing the geometry APIs.
This observes the language boundary without claiming a full `contact` call.

Impulse cases create a real `World(null)` and two real `Organism(world)` instances.
The bodies remain unplaced; the tested method reads only the recorded physical fields.
Typed `I` rows explicitly supply mass, inertia, centers, velocities, growth indicator, and growth ratio.
`U` rows supply elasticity and velocity/rotation caps.
The helper invokes the unchanged private method through reflection on those real objects.
It records the same fields before and after the call.
No replacement collaborator, mock method, copied impulse formula, or complete simulation frame supplies the results.

Each impulse case starts ordinary `Random` seed `506172`.
The two real constructors consume their initial orientation draws.
After the collision call, the following RNG outputs match an independent ordinary `Random` after exactly two `nextDouble` calls.
The suffix is `1779108803982288030`, `8634`, and double bits `3fd7bc11b90e4b96`.

## Coverage

| Cases | Executed observations |
| --- | --- |
| Signed determinants | Asymmetric positive/negative crossings and values below, at, and above the source tolerance. |
| Parallel/collinear | Offset parallels, disjoint collinear segments, overlaps, and shared endpoints. |
| Degenerate points | On-line, off-line, equal, and distinct zero-length segments. |
| Endpoint distances | Equal-distance ties and unequal nearest-endpoint distances through actual `distanceSq`. |
| Rectangles | Positive, zero, and negative dimensions; edges, corners, points, and intersecting overflow boundaries. |
| Integer offsets | Ordinary coordinates, positive wrap, negative wrap, and both integer extremes. |
| Normal orientation | Both supplied line-owner flags, reversed lines, horizontal/skew lines, ties, and same-side centers. |
| Relative velocity | Approaching, separating, stationary, rotational, and asymmetric physical values. |
| Growth | Actor-only, peer-only, both actors, and negative growth indicators. |
| Clamping | Default, custom, and zero caps; elasticity zero/one and signed-zero velocities. |
| Degenerate physical states | Zero mass, zero inertia, zero normal, subnormal mass, infinity, and NaN inputs. |
| Unbounded stress | Explicit negative inertias and infinite caps expose raw infinite outputs. |

The negative-inertia and nonfinite cases are labelled controlled numeric stress inputs.
They do not claim that normal source gameplay constructs those bodies.
The fixture records behavior of the raw method on those inputs.

## Observed source quirks

`ExLine2DDouble` compares the signed determinant with `Utils.tol`; it does not compare its absolute value.
Reversing one asymmetric crossing therefore changes the reported point from `(2.4,0.6000000000000001)` to `(3.25,1.25)`.
The latter is the source's endpoint average, despite the segments remaining nonparallel.

Collinear shared-endpoint segments can report an endpoint average instead of their shared endpoint.
The recorded example reports `(3.5,0)` instead of `(4,0)`.
Java `Line2D` reports intersection between the supplied distinct zero-length point segments.
Those observations remain exact source behavior.

`touchMove` applies impulses to the separating-velocity cases as well as approaching cases.
Only the actor's growth indicator contributes to its contact-point velocity.
The peer's supplied growth indicator remains unused by that calculation.
The source's peer angular update uses `ny` in both lever terms.
Its impulse denominator instead uses the distinct normal components.
The recorded rotational cases preserve that asymmetry.

## TSV schema

Rows are tab-separated ASCII, with one newline after each row.
Doubles use sixteen hexadecimal digits containing raw IEEE bits.
Booleans use `true` or `false`; signed integers use decimal notation.

| Tag | Columns after the tag |
| --- | --- |
| `C` | Case name; operation `geometry`, `offsets`, or `impulse`. |
| `L` | Line label; start-X, start-Y, end-X, end-Y bits. |
| `Q` | Rectangle label; X, Y, width, height integers. |
| `X` | Line label; start/end integer offsets, then integer center X/Y. |
| `P` | Point label; X/Y bits. |
| `U` | Source parameter name; raw double bits. |
| `I` | Object or call owner; field; Java type; supplied input. |
| `O` | Stage `before`/`after`; object owner; field; Java type; observed value. |
| `B` | Boolean observation name; observed value. |
| `D` | Endpoint label; observed squared-distance bits. |
| `R` | Following long, bounded integer, and double bits. |
| `E` | Case terminator. |

Geometry `L` and `Q` rows are supplied inputs.
Offset `L` rows are Java-added coordinate observations derived from supplied `X` inputs.
Geometry point `intersection`, booleans, and distances are expected API outputs.
Impulse line `impact` and point `contact` are supplied inputs.
Its `I(call,thisOrganism,boolean,value)` selects the real method's line-owner argument.

Each impulse object records `_mass`, `_I`, `_dCenterX`, `_dCenterY`, `dx`, `dy`, `dtheta`, `hasGrown`, and `_growthRatio`.
The first seven use Java `double`; the final two use Java `int`.
`hasGrown` and `_growthRatio` are distinct source inputs.
Unchanged physical fields remain observations rather than implicit assumptions.

## Counts and source anchors

| Record group | Count |
| --- | ---: |
| Geometry cases | 39 |
| Integer-offset cases | 4 |
| Impulse cases | 48 |
| Boolean observations | 172 |
| Distance observations | 172 |
| Physical-state observations | 1,728 |
| RNG suffixes | 48 |

Source anchors are relative to `../Colormod/src/biogenesis/` from the project root.

| Source | Behavior |
| --- | --- |
| `ExLine2DDouble.java:37-52` | Signed determinant, parallel average, and interpolation. |
| `Organism.java:756-763` | Real constructor orientation draw. |
| `Organism.java:5673-5725` | Original impulse, actor growth, normal selection, clamping, and peer angular asymmetry. |
| `Organism.java:5872-5896` | Integer endpoint addition, endpoint distances, and line-owner dispatch. |
| `Utils.java:595-605` | Default caps and elasticity. |
| `Utils.java:1619-1620` | Floating clamp through Java `Math.min` and `Math.max`. |

The JDK supplies `Line2D`, `Point2D`, and `Rectangle`; the provenance records its executed version.
Both source/JAR helper compilations produced identical helper class bytes.
The original 74 reference files retain hashes, sizes, and modes.
All six prior TSV fixtures remain unchanged.

The helper does not execute complete contacts, collision scanning, movement, borders, death, rendering, or game frames.
Endpoint-distance observations do not claim that the full contact dispatcher executed.
World ownership and frame integration require separate evidence.
The provenance reports Java observations only; Rust comparisons remain separately recorded.
