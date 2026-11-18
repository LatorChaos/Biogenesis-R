# Empty-world atmosphere frame observations

This private corpus observes actual unchanged `World.time()` calls.
It contains 39 successful raw-state cases and 9,906 rows.
The separate UI-hook diagnostic corpus contains two cases and 510 rows.
Neither corpus establishes Rust fidelity or public settings acceptance.
Canonical adoption remains lead-owned and pending.

## Actual execution

Each case constructs a real `new World(null)`.
Each case contains no organisms or corridors.
The helper directly supplies four World fields and two divisor settings.
It calls the actual `world.time()` once.
It does not copy the atmosphere algorithm or substitute callback behavior.

Ordinary cases start with `nFrames=1`.
Four clock cases instead use 254, 255, -1, and `Integer.MAX_VALUE`.
These starting values avoid the source's twenty-frame UI callback.
The 255, -1, and maximum-value cases reach the actual statistics clock transition.
The helper captures every World instance field and every statistics field/getter before and after execution.

The first headless source probe passed at `probe-6iv693_5`.
The first earlier probe failed its observer schema guard before any getters or `time()` call.
Its getter filter incorrectly included `eventOrganismCreated()`.
The corrected filter selects only public zero-argument `get*` methods.
The failed helper and receipt remain preserved at `probe-88s5f9hi`.

## Input authority

Each C row names the case, mode, and seed 827364.
Eight U rows record actual constructor or frame settings.
They include the four statistics defaults, dimensions, and both atmosphere divisors.
Four I rows supply `_CO2`, `_CH4`, `_O2`, and `nFrames`.
All oxygen inputs use exact negative zero.
Every expected W/S/T/G/R value comes from real source objects after construction.
No expected after-state supplies any input.

The helper resets source DEF values before each case.
It sets initial organisms to zero and disables connection and backup settings.
The World constructor does not call `genesis()`.
Statistics constructor defaults therefore remain distinct from supplied atmospheric gas fields.

## Grammar

C has four columns: tag, case name, mode, seed.
U and I have four columns: tag, field, Java type, value.
W/S/T have five columns: tag, stage, field or getter, Java type, value.
Stages are `before` and `after`.
W contains all 15 World instance fields.
S contains all 52 statistics instance fields.
T contains all 49 public zero-argument statistics getters.
G has five columns and records the actual `World.getTime` result.
R has five columns: tag, stage, nextLong, nextInt10000, nextDoubleBits.
R samples a serialized copy of the actual source Random.
K has two columns and names the actual `time` call.
B has three columns and records completed, tree_replaced, or random_unchanged.
E closes the case.

Double values use 16 lowercase hexadecimal raw-bit digits.
Lists contain ordered comma-separated double bits, with empty values for empty lists.
Null references use `~`.
The World statistics relation uses `s0`.
The collision tree uses `present`; no tree query or build occurs in the observer.
World membership and corridor lists remain empty.

## Fixed comparison ledger

| Disposition | Rows |
| --- | ---: |
| Exact method-level comparison observations | 8,970 |
| Named UI/corridor collaborator dispositions | 312 |
| Source tree allocation metadata | 39 |
| Input and command rows | 585 |
| Total canonical rows | 9,906 |

Each case has 230 exact comparison observations.
They include both scalar/list snapshots, actual clocks, completed status, copied RNG suffixes, and actual RNG purity.
No geometry or numeric tolerance applies.
NaN and signed-zero raw bits remain recorded exactly.

The eight collaborator rows per case cover `_visibleWorld`, `inCorridors`, `outCorridors`, and `_corridorexists`.
These records remain source evidence without invented Rust UI or corridor fields.
The allocation-only `tree_replaced` row requires a named disposition for Rust's value-owned spatial tree.
It cannot establish Java allocation-layout parity.

## Coverage and source behavior

Cases cover positive, negative, mixed, zero, and signed-zero gas inputs.
Each divisor independently receives 0, 1, -1, integer minimum, and integer maximum.
Additional cases cover both-zero divisors and sequential use of updated methane.
Maximum finite gas inputs expose source-generated nonfinite outputs.
Four cases capture signed clock wrap and statistics scheduling.

With gases -10/-3 and default divisors, source finishes with CO2 -13 and methane positive zero.
With positive gas and a zero carbon divisor, source caps the first transfer at available carbon.
Zero divisors use floating division; they do not throw `ArithmeticException`.
Zero-over-zero can propagate negative NaN bits through the actual frame.

Both dialog divisor setters require positive integers.
Preferences read integer values without applying those dialog guards.
Negative gases and nonpositive divisors are explicit raw-state/Task-10 compatibility observations.
A future private Rust replay may install those states directly with explicit attribution.
Such replay does not accept them through the public settings API.

## Separate partial-state diagnostics

`atmosphere_frame_ui_diagnostic.tsv` starts both cases at frame zero with no VisibleWorld.
The actual source performs gas updates and frame postincrement before its InfoPanel callback throws.
The helper catches only the expected `NullPointerException` for these labelled diagnostic cases.
It records the exception class and all after-state fields.
It does not fabricate a successful callback or require equivalent Rust null-callback failure.
All 510 diagnostic rows remain outside canonical Rust comparison acceptance.

## Custody and limits

Raw run: `.workflow-local/atmosphere-frame-oracle/canonical-lve9lq1l`.
Two source runs and the bundled JAR produce identical canonical bytes and diagnostic bytes.
Both compiled helper classes match between source and JAR builds.
Six rejecting guards cover missing/outside/symlink profiles, headless mode, existing output, and invalid mode.
All 74 original files, 159 compiled reference classes, and 569 prior fixture files remain unchanged.
Positive target executions use isolated home, temporary, and preference roots, bounded heap, and owned process-group cleanup.
These observations make no host-global process-absence claim.
No GUI, native display, networking, actor behavior, performance, or complete frame acceptance follows.
