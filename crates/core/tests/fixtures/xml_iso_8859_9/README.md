# Original ISO-8859-9 observations

These files copy actual original parser inputs and observations exactly.
They are separate from the unchanged 261-case XML corpus and 327-parse replay.
The finite capture used fresh Source/source/JAR processes under the installed JDK21 profile.
All three streams are byte-identical: 260 cases, 226 successes, 34 errors, and 55634 rows.
Successful cases record 28 genome observations, 40 gene fields, aliases, and actual DOM carrier codepoints.
Error rows retain original SAX/IO classes and diagnostics. RNG rows observe copied Source state only.

`cases/` retains every original input byte. `observations.tsv` is the actual Source-a observation stream.
The provenance manifest pins all input/output copies and original locations.
Only fixture paths and file modes change. No Rust output supplies an expected value.
The 260-input replay and decoded-carrier regression remain unexecuted during preparation.
This decoder family does not complete XML AC1 or Task16.

The casing supplement retains two additional exact inputs and its actual 100-byte observation stream.
Both observed names succeed across Source/source/JAR. No distinct alias is inferred.
Casing model comparisons reuse uppercase Source observations for identical document bodies.
