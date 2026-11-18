# Original512-tick compact workload trajectory

The original 512-attempt world uses seed 827364 and 7000 by 7000 dimensions.
Source-a, source-b and bundled-JAR observations match through 512 complete World.time calls.
The fixture froze before Rust edits or comparison.

Its 29,914 rows contain 513 snapshots and 31,293 obligations.
These comprise 30,267 runtime values and 1,026 labeled source Gaussian invariants.
Each snapshot records 9 World scalars, 37 statistics scalars, 9 getters and 4 actual RNG observations.
Doubles retain raw bits; integers retain their Java widths.
Only a separately deserialized RNG clone supplies the suffix.
Q rows assert live scalar/RNG observer purity; X rows retain explicit nonprimitive omissions.

Ticks 256 and 512 cross two statistics rollovers.
Snapshot labels remain absolute; actual nFrames wraps modulo 256 and statistics time reaches 2.
Original P/U inputs and the entire old 512-cohort observation prefix 0..80 match unchanged bytes.
A midrun serialization matches the original tick 80 graph before continuation and checks scalar/RNG purity.

Fixture SHA256: b90255c9a2ff9ad25736b7f34d6b168042de6ea9243b477bca62414e9e7dd84c
Fixture bytes: 952239
Tick 80 graph SHA256: ee9e144c5ad61e81ee9b8e43eea2e6484f80cbe6607ec36ed561549f845e597f
Tick 512 graph SHA256: 066c660ffd335cac7bdccfe8d05d9f14f907b6aa1ffae69f5e6ca8e35e7d35ba

Serialization hashes are custody witnesses; observations never initialize Rust.

## Reproduction

The existing helper accepts ATTEMPTS NEW_OUTPUT_DIRECTORY [--evolving-512].
The optional literal requires 512 attempts and selects 512 ticks.
Two arguments retain the original 80-tick observations and bootstrap contract.
The frozen fixture used the private capture helper identified in provenance.
Canonical-helper compatibility passed six supervised commands.
All three 512-tick outputs match this fixture.
Default 80-tick output matches the old 512-cohort bytes exactly.
All three compiled helper classes match between source and JAR.
Owned groups exited gracefully; resource and subreaper state were restored.

Actual receipts: [compatibility result](../../../../.workflow-local/evolving-world-512-a/source-controls/canonical-helper-compat-a/result.json)
and [command receipts](../../../../.workflow-local/evolving-world-512-a/source-controls/canonical-helper-compat-a/commands.json).

Result SHA256: c3152c13a7a967f87bdc1d064e798e20fe400d797d61cc2938961056ea219967
Commands SHA256: 7dd4276d67ac344c31ba012436c70b9fafa4142234bbfb55ef77059e18c91c95

Source-b emitted "Timer already cancelled" from the original application's Timer-1 thread.
The deliberate presentation-timer cancellation and retained diagnostic do not establish UI or background-thread health.

Use the reviewed run-b run_command supervisor and command_entry.py wrapper for all child commands below.
They record actual host priority and provide subreaping, graceful termination and owned-group absence checks.
Retain CPU0/SCHED_IDLE/nice19/idleIO, 8 GiB AS, 1 GiB FSIZE, CORE0 and 512 MiB heaps.
Keep 60-second compilation, 90-second capture, 2 MiB trajectory and 16 GiB MemAvailable bounds.
Process-control elapsed values are not simulation timings.
The supervisor is .workflow-local/evolving-world-512-a/source-controls/capture-run-b.py.
Bind fresh controls to the new canonical helper hash; preserve historical manifests.

From the Biogenesis-R root, choose a new private RUN directory.
Set these paths:

    ROOT="$PWD"
    RUN="$ROOT/.workflow-local/evolving512-helper-compat-NEW"
    SOURCE="$ROOT/.workflow-local/reference-build/source-classes"
    JAR="$ROOT/../Colormod/ColorMod_2_01/biogenesis.jar"
    JTS="$ROOT/.workflow-local/reference-build/jts-core-1.15.0.jar"
    JAVA=/usr/lib/jvm/java-21-openjdk/bin/java
    JAVAC=/usr/lib/jvm/java-21-openjdk/bin/javac

The supervisor creates isolated build-profile directories and compiles both read-only backends:

    "$JAVAC" -J-Xmx512m -encoding UTF-8 -cp "$SOURCE:$JTS" -d "$RUN/helpers-source" "$ROOT/tools/reference/WorkloadTrajectoryOracle.java"
    "$JAVAC" -J-Xmx512m -encoding UTF-8 -cp "$JAR:$JTS" -d "$RUN/helpers-jar" "$ROOT/tools/reference/WorkloadTrajectoryOracle.java"
    diff -r "$RUN/helpers-source" "$RUN/helpers-jar"

Require exactly three identical helper classes.
Run these three new-mode cases with the shared command:

| CASE beneath RUN | CP |
| --- | --- |
| source-a-512 | RUN/helpers-source:SOURCE:JTS |
| source-b-512 | RUN/helpers-source:SOURCE:JTS |
| jar-512 | RUN/helpers-jar:JAR:JTS |

Set CASE and CP to the expanded absolute paths for each row.
Create private home,tmp,prefs-user,prefs-system,config,cache and data directories under each case.
Use PATH=/usr/bin:/bin, LANG=C.UTF-8 and PYTHONDONTWRITEBYTECODE=1.
Use this complete child command with the reviewed supervisor:

    env BGR_ISOLATED_XVFB=1 HOME="$CASE/home" TMPDIR="$CASE/tmp" XDG_CONFIG_HOME="$CASE/config" XDG_CACHE_HOME="$CASE/cache" XDG_DATA_HOME="$CASE/data" /usr/bin/xvfb-run -n 410 -a -e "$CASE/xvfb.log" -s '-screen 0 1280x960x24 -nolisten tcp' "$JAVA" -Xms16m -Xmx512m -Djava.awt.headless=false "-Duser.home=$CASE/home" "-Djava.io.tmpdir=$CASE/tmp" "-Djava.util.prefs.userRoot=$CASE/prefs-user" "-Djava.util.prefs.systemRoot=$CASE/prefs-system" -cp "$CP" biogenesis.WorkloadTrajectoryOracle 512 "$CASE/observations" --evolving-512

Run the default compatibility case with CASE=RUN/default80-source-512 and CP=RUN/helpers-source:SOURCE:JTS.
Use its own fresh profile directories and the same command without the final --evolving-512 literal.

Keep -n 410 before -a; the installed wrapper processes options in order.
Allow graceful owned Xvfb termination and reap descendants before conditional KILL.
Do not delete another process's lock or socket.
Run-a failed X11 startup and remains failure evidence.
Run-b completed all five commands without KILL and restored subreaper state.

Compare all three new-mode outputs with the frozen fixture:

    cmp "$RUN/source-a-512/observations/trajectory.tsv" "$ROOT/crates/core/tests/fixtures/workload_trajectory_512_ticks.tsv"
    cmp "$RUN/source-a-512/observations/trajectory.tsv" "$RUN/source-b-512/observations/trajectory.tsv"
    cmp "$RUN/source-a-512/observations/trajectory.tsv" "$RUN/jar-512/observations/trajectory.tsv"

Extract the old 512 case without changing its bytes:

    python3 - "$ROOT/crates/core/tests/fixtures/workload_trajectory.tsv" "$RUN/old-512.tsv" <<'PYCODE'
    from pathlib import Path
    import sys
    rows = Path(sys.argv[1]).read_bytes().splitlines(keepends=True)
    start = next(i for i, row in enumerate(rows) if row.startswith(b"C\tworkload-512\t"))
    stop = rows.index(b"E\n", start)
    Path(sys.argv[2]).write_bytes(b"".join(rows[start:stop + 1]))
    PYCODE
    cmp "$RUN/default80-source-512/observations/trajectory.tsv" "$RUN/old-512.tsv"

Check both bootstrap contracts.
Default mode retains only main_window,info_toolbar,paused,timer_cancelled,owned_display,endpoint80_sha256,endpoint80_bytes and seed_cache_extraction.
Long mode adds tick 512 custody, 512 ticks, 513 snapshots, final frame 0/time 2 and witness purity.
Check the actual owned display rather than requiring 410.
The preparation receipt provides fully expanded argv and environments for all six compatibility commands.

## Limits

Compact scalar/RNG fidelity does not establish complete actor geometry, identity, order, trees or history contents.
It establishes no performance, steady-state, Pi/ARM, renderer, 24-hour endurance or complete product acceptance.
The original 80-frame fixture, benchmarks and budgets remain separate and unchanged.
