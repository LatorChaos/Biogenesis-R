# Nonopaque cropped-image raster derivation

This source map accompanies the cropped-image Rust port.
The comparison records are maintained separately from this source distribution.
The public image API, membership/alive guards, byte budget, and opaque raster contract remain unchanged.

## Actual pipeline authority

The frozen original Java method creates a fresh default Graphics2D over TYPE_INT_RGB.
Its installed JDK21 classes select the following nonopaque route:

| Seam | Actual installed instruction contract |
| --- | --- |
| SurfaceData.validatePipe | Nonopaque composite selects paintViaShape without antialiasing |
| PixelToShapeConverter.drawLine | Convert integer endpoints into Line2D.Float |
| SpanShapeRenderer.draw | Use LoopPipe.getStrokeSpans for BasicStroke |
| LoopPipe.getStrokeSpans | Nonzero winding, thin stroke, normalization on, antialiasing off |
| BasicStroke() | Width1, square caps, miter joins, miter limit10 |
| DMarlinRenderingEngine | Thin non-antialiased width1; select ON_NO_AA normalization |
| NearestPixelQuarter.normCoord | floor(coordinate+0.25)+0.25 |
| Stroker.lineTo/finish | Offset both sides by half-width; extend square caps |
| PathConsumer2DAdapter | Convert emitted double vertices back to float |
| ShapeSpanIterator | Scan pixel centers, preserve directed winding, and quantize edge stepping |

Primary official source references also support the derivation:

- OpenJDK21u DMarlinRenderingEngine.java, normalization modes and default thin-width path
- OpenJDK21u ShapeSpanIterator.c, appendSegment, subdivision rejection, and span stepping
- OpenJDK mainline Stroker.java, offset and square-cap structure

The [raster notice](LICENSES/OpenJDK-raster/NOTICE.md) lists the source URLs and their version limits.
The mainline Stroker reference is not labelled as the installed21 source revision.
Installed21 bytecode determines the ported statement order and clip defaults.
The native disassembly confirms the floating operations and integer scale in the official21u reference.

## Bounded line-only construction

A source integer coordinate first passes through f32, as Line2D.Float requires.
Normalize the resulting double coordinate to the nearest quarter-pixel rule.
Compute the default width1 stroke normal using ordered square, sum, square root, multiply, and divide operations.
Emit both offset sides and square caps in the source order.
Each emitted vertex passes through f32 before edge setup.
Zero-length input uses the source horizontal unit direction before stroking.

Clipping precedes float outline conversion when the source subdivider applies.
The default clip begins with the original quarter-pixel and0.001 offsets.
The miter limit expands each side by5, exceeding the square-cap margin.
Crossing lines longer than100 Manhattan units split at the additionally quarter-padded clip boundaries.
Roots use the source coefficient expression, range [0,1), and ascending order.
Successive line subdivisions use relative parameters, preserving the original operation order.
Common-outcode pieces are discarded; artificial joins retain side endpoints without extra join decoration.
Caps apply only to retained endpoints inside the stroke-expanded clip.
There are at most five line pieces and24 outline vertices.
No curve, dash, transform, custom stroke, custom rendering hint, or generic path framework is introduced.
Those features are absent from the actual getImage graphics construction.

## Native span rules

Each float edge is directed upward for setup while retaining its original winding sign.
First and exclusive-last rows use ceil(y-0.5) after float subtraction.
The first intersection uses the original float multiply/divide/add order.
Its integer X uses ceil(x-0.5).
The lower X increment is floor(floatSlope).
Fractional error uses scale2147483647; row carry uses bit31 and a0x7fffffff remainder.
The implementation distinguishes the installed one-row update from bulk clipped-row advancement.
It retains x86_64 invalid float-to-int conversion as INT_MIN.

Edges wholly left preserve winding at the clip boundary.
Edges wholly right are omitted, with any nonzero final winding filled through the right clip boundary.
Equal-X crossing order affects zero-width spans only.
Unstable in-place sorting avoids hidden heap scratch.
All polygon, edge, and crossing storage uses fixed stack arrays.
The only payload heap allocations remain the preflighted command and RGB8 vectors.
Row iteration is bounded by image height; span iteration is bounded by image width.

The candidate uses ordinary nonzero-winding contour coverage for every nonopaque line.
It contains no case names, expected pixels, tie-mask exceptions, or per-scenario branches.
The opaque Bresenham route remains separate because actual Java selects a different path.

## Source observations and strict gate

The immutable original fixture retains39cases,39475rows,29771 proposed runtime rows, and2962pixels.
The separate approved alpha fixture adds5cases,4965rows,3760 proposed runtime rows, and405pixels.
Combined canonical obligations are44cases,44440rows,33531runtime rows, and3367 exact pixels.
The nine raw diagnostics retain8360unpassed rows; they do not enter either success replay.

The consumer reuses its complete model, holder, World, statistics, genotype, gene, and RNG observers.
Only the fixed mode and tag/row ledgers differ between the two fixture entry points.
The original fixture and observer comparisons are not replaced by the supplement.
All source expectations remain immutable files; no output supplies construction inputs.
No pixel tolerance or scalar allowance changes.
The existing five geometry allowances remain unchanged.

## License and limits

The new alpha module retains Oracle copyright notices and the GPLv2/Classpath header.
Its SPDX tag is GPL-2.0-only WITH Classpath-exception-2.0.
The combined package remains GPL-2.0-only because its existing numeric port requires that license.
No dependency or manifest changed.
The source map does not grant universal JDK, renderer-property, architecture, or platform parity.
Default installed21 behavior and the two actual original corpora are the current compatibility authority.
PNG/file/UI integration and combined fidelity have separately attributed evidence in the linked image report.
All raw nullable-state diagnostics remain separate and unpassed.
