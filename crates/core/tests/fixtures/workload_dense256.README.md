# Complete original dense-256 workload evidence

This case starts 12,500 attempts with seed 827364 in a 7000 by 7000 world.
The original application executes 256 complete World.time calls from natural genesis.
All 257 scalar/RNG snapshots and the complete final serialized World match across source-a, source-b and bundled JAR.
The full endpoint decoders also produce identical bytes.
Expectations froze before Rust edits.

## Frozen obligations

| Evidence | Value |
| --- | --- |
| Compact rows / obligations | 15,066 / 15,677 |
| Actual runtime values / labeled Gaussian invariants | 15,163 / 514 |
| Full rows / derived comparisons | 4,150,610 / 4,000,244 |
| Required boxed-history A aliases / L records / D records | 6 / 6 / 6 |
| Membership / stored leaves / union actors | 13,279 / 13,278 / 13,281 |
| Genomes / gene slots / unique Genes | 12,851 / 19,319 / 18,540 |

The full ledger retains 123,646 Color/String allocation metadata rows.
It separately retains 13,284 absent transient fields, 13,287 named absence records and 149 input/boundary rows.
All categories sum to the full row count.
Two actors occur only in the stored tree; three members are absent from that tree.
Population totals cannot replace the complete actor/order/alias checks.

Histories contain six actual ArrayLists, six ordered slots and six distinct boxed Double owners.
L records retain list owner, runtime class and size.
D records retain list owner, slot index, box owner and raw bits.
Existing S/T/A observations remain.
Double A edges say immutable because their values are immutable; their owner identities remain required derived checks.
Only the pre-existing Color/String allocation identity allowance remains metadata.

The tree contract retains stored leaf order, actor references, raw Envelope bounds and World-to-tree identity.
Existing query/order obligations remain unchanged.
Private JTS node allocation was already outside the endpoint representation contract.

## Artifact pins

| Artifact | SHA256 | Bytes |
| --- | --- | ---: |
| Compact TSV | ec07fb3fd03c5bad29edad69715f4b42d5a85510adce48d9d2c24d7612ee6b18 | See provenance |
| Final BGW | f912ec8932a482283696fd5265878b96bc8439fc8d395dbcf73064639e8a5598 | 36,074,003 |
| Endpoint gzip | 59f2ad54a2f7aae1098dcec0ddca38595b38c3d6b8590c5330bef65ccb12fe42 | 22,296,207 |
| Expanded endpoint | b97b2734db5d60d25e5ccce2c41991c51b77f64eb91bbc027348d366cecb5265 | 182,874,988 |

The old 12,500-cohort P/U inputs and all K/V/S/W/R/Q observations through tick 80 match exactly.
C declares the new 16+240 functional call groups.
The tick-80 serialization witness remains ad5f4e3b9257591c6a5055c3de1d472c4c15da3858018b706ed8125d767c383d, 35,915,346 bytes.
Final nFrames is 0 and statistics time is 1.

Provenance links the complete raw receipts, schema maps, ledger, descriptor preflight and resource/cleanup results.
It distinguishes the private helpers that generated observations from canonical reproduction helpers.
Canonical helper compatibility passed all 16 serial commands.
Both helper class maps match across source and JAR builds.
Three canonical dense captures reproduce the exact compact TSV and BGW bytes.
Three fresh canonical dense decodes reproduce the exact full endpoint, ledger and schemas.
Default-80 dense, evolving-512 moderate and all four old State modes also reproduce their original bytes.

Direct Rust debug and release each pass 757 ordinary tests, with 8 ignored and 765 registered.
Each profile passes three focused tests, the dense-256 heavy gate and all four old full-80 cases.
Direct dependency custody joins 637 captured paths against the recorded core inputs.
Five additional fixture paths join current files and baseline Source 0d6e9716.
This split custody establishes no precompile five-fixture inventory or single captured dependency closure.
The actual debug CLI gate also passes all 4,000,244 derived endpoint comparisons.
Its nominal build bound is 12 GiB; the successful outer command imposes an 8 GiB hard cap.
Executed limit-setting code implies 8 GiB build and 4 GiB test soft caps.
Live per-command proc limits were not observed.
The CLI receipt does not claim an ordinary-suite pass.

The provenance pins actual result, summary and command receipts for these checks.
Historical private capture receipts remain unchanged.
Canonical checks retain cleanup, restoration and resource evidence.
These checks establish selected functional fidelity and helper reproduction only.

## Reproduction commands

Use the approved graceful per-command supervisor and command-entry resource wrapper.
They record real host PID/PGID/priority, reap owned descendants and restore resource/subreaper state.
Create fresh controls that bind current helper hashes; preserve historical input manifests.
The supervisor is .workflow-local/evolving-world-512-a/source-controls/capture-run-b.py.
The entry wrapper is command_entry.py in the same directory.

Keep CPU0/SCHED_IDLE/nice19/idleIO, 8 GiB AS, 1 GiB FSIZE and CORE0.
Use 512 MiB javac/Java heaps and 16 GiB MemAvailable checks before and after commands.
Compile deadlines are 60 seconds; live capture deadlines are 90 seconds; decoder deadlines are 240 seconds.
Compact text remains capped at 2 MiB; captured BGWs remain capped at 1 GiB.
Decoder references/depth/arrays/full text remain 4,000,000 / 64 / 20,000 / 600 MiB.
These are functional controls without timing qualification.

Set these paths from Biogenesis-R:

    ROOT="$PWD"
    RUN="$ROOT/.workflow-local/dense256-reproduce-NEW"
    SOURCE="$ROOT/.workflow-local/reference-build/source-classes"
    JAR="$ROOT/../Colormod/ColorMod_2_01/biogenesis.jar"
    JTS="$ROOT/.workflow-local/reference-build/jts-core-1.15.0.jar"
    PREFS="$ROOT/.workflow-local/reference-run-dve5ksyn/workload-source-12500/prefs-user/.java/.userPrefs/biogenesis/prefs.xml"
    JAVA=/usr/lib/jvm/java-21-openjdk/bin/java
    JAVAC=/usr/lib/jvm/java-21-openjdk/bin/javac

The supervisor creates a new RUN and isolated build profiles.
Compile each helper against both read-only backends:

    "$JAVAC" -J-Xmx512m -encoding UTF-8 -cp "$SOURCE:$JTS" -d "$RUN/trajectory-source" "$ROOT/tools/reference/WorkloadTrajectoryOracle.java"
    "$JAVAC" -J-Xmx512m -encoding UTF-8 -cp "$JAR:$JTS" -d "$RUN/trajectory-jar" "$ROOT/tools/reference/WorkloadTrajectoryOracle.java"
    "$JAVAC" -J-Xmx512m -encoding UTF-8 -cp "$SOURCE:$JTS" -d "$RUN/state-source" "$ROOT/tools/reference/WorkloadStateOracle.java"
    "$JAVAC" -J-Xmx512m -encoding UTF-8 -cp "$JAR:$JTS" -d "$RUN/state-jar" "$ROOT/tools/reference/WorkloadStateOracle.java"
    diff -r "$RUN/trajectory-source" "$RUN/trajectory-jar"
    diff -r "$RUN/state-source" "$RUN/state-jar"

Require three matching trajectory helper classes and four matching state helper classes.
Run source-a, source-b and JAR captures serially.
For each case, create fresh home,tmp,prefs-user,prefs-system,config,cache and data directories.
Set PATH=/usr/bin:/bin, LANG=C.UTF-8 and PYTHONDONTWRITEBYTECODE=1.
CASE is the absolute case directory.
CP is trajectory-source:SOURCE:JTS for source-a/source-b, or trajectory-jar:JAR:JTS for JAR.
Expand all classpath components to absolute paths.

    env BGR_ISOLATED_XVFB=1 HOME="$CASE/home" TMPDIR="$CASE/tmp" XDG_CONFIG_HOME="$CASE/config" XDG_CACHE_HOME="$CASE/cache" XDG_DATA_HOME="$CASE/data" /usr/bin/xvfb-run -n 410 -a -e "$CASE/xvfb.log" -s '-screen 0 1280x960x24 -nolisten tcp' "$JAVA" -Xms16m -Xmx512m -Djava.awt.headless=false "-Duser.home=$CASE/home" "-Djava.io.tmpdir=$CASE/tmp" "-Djava.util.prefs.userRoot=$CASE/prefs-user" "-Djava.util.prefs.systemRoot=$CASE/prefs-system" -cp "$CP" biogenesis.WorkloadTrajectoryOracle 12500 "$CASE/observations" --dense-256

Keep -n 410 before -a.
Allow graceful owned Xvfb exit and reap descendants before conditional KILL.
Do not remove another process's locks or sockets.
Each capture writes trajectory.tsv, bootstrap.json and complete world-after-256.bgw.
Compare all three TSVs and BGWs byte-for-byte, then check the hashes above and the tick-80 witness.

The current StateOracle admits this BGW only with --dense-256, exact f912 hash and exact 36,074,003-byte size.
The default four-argument modes retain their four old hashes, twenty classes and 36,000,000-byte limit.
Only the dense flag admits the two preflight-observed Double/Number schemas.
Stream/local SUIDs, fields and bootstrap runtime classes/superclasses remain checked.
Unobserved decoder classes remain rejected.

Create fresh headless decode profiles for each backend/input.
Set CASE to that decode directory and BGW to its matching captured file.
CP is state-source:SOURCE:JTS or state-jar:JAR:JTS.

    env HOME="$CASE/home" TMPDIR="$CASE/tmp" XDG_CONFIG_HOME="$CASE/config" XDG_CACHE_HOME="$CASE/cache" XDG_DATA_HOME="$CASE/data" "$JAVA" -Xms16m -Xmx512m -Djava.awt.headless=true -Duser.language=en -Duser.country=US "-Duser.home=$CASE/home" "-Djava.io.tmpdir=$CASE/tmp" "-Djava.util.prefs.userRoot=$CASE/prefs-user" "-Djava.util.prefs.systemRoot=$CASE/prefs-system" -cp "$CP" biogenesis.WorkloadStateOracle 12500 "$BGW" "$PREFS" "$CASE/observations" --dense-256

Compare all three state.tsv files byte-for-byte and require expanded b97b2734 above.
Run the reviewed complete ledger validator; it checks every declared history identity and row disposition.
The raw producer ledger and schema hashes are pinned in provenance.
Package one verified state.tsv deterministically:

    python3 - "$STATE" "$OUT_GZIP" <<'PYCODE'
    import gzip, shutil, sys
    with open(sys.argv[1], "rb") as source, open(sys.argv[2], "xb") as target:
        with gzip.GzipFile(fileobj=target, mode="wb", filename="", compresslevel=6, mtime=0) as encoded:
            shutil.copyfileobj(source, encoded, 1024 * 1024)
    PYCODE

Check compressed/expanded hashes and full-byte decompression roundtrip.

## Preserved modes and scope

Trajectory without a flag still observes 80 ticks for the four original cohorts.
The --evolving-512 flag still permits only 512 attempts and 512 ticks.
The --dense-256 flag permits only 12,500 attempts and 256 ticks.
Keep old mode/bootstrap compatibility checks separate from new-mode reproduction.

Only the original Java observation decoder reads the BGW.
Rust constructs declared seeded genesis and compares both datasets on the same actual 256-tick World.
No observed scalar, RNG, actor or graph becomes a Rust constructor input.
Only existing O._m/_m1/_m2/_mass/_I geometry allowances remain.
History values/owners, gas, energy, RNG, integer, envelope and ordering checks gain no tolerance.

This selected trajectory and final endpoint establish no universal trajectory equivalence.
They establish no performance, budget, Pi/ARM, renderer, 24-hour endurance or full-product acceptance.
Original process diagnostics remain retained; exact data do not establish UI or background-thread health.
