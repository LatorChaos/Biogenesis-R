# SPDX-License-Identifier: GPL-2.0-or-later

"""Verify Rust XML writer bytes through the unchanged source and bundled Java parsers"""

import argparse
import base64
from collections import Counter
import hashlib
import json
import math
import os
from pathlib import Path
import resource
import shutil
import signal
import struct
import sys
import tempfile

sys.dont_write_bytecode = True
from build import environment, executable, local_path, neutral_directory, require, seed_cache
from run_reference_oracles import run_command

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE = ROOT.parent
FIXTURE = "ba5546e89bde74a6b277483e557d4ce3f1c4251e79b29937f44cc647adcffac7"
CODEC = "986f33caa59b95a29da32c1c3d9f9bef9d147d1aa9b2a2bdb175cb94b0a2860c"
DTD = "510c6457b146482b767ba0474676bcc38f691a14e8cb6dd3685e69b2b3158b8d"
FIELDS = {"C": 4, "U": 4, "F": 7, "K": 4, "O": 3, "X": 4, "G": 5,
          "N": 6, "A": 4, "T": 3, "R": 5, "V": 4, "E": 1}
COUNTS = {"C": 261, "U": 38367, "R": 848, "F": 448, "K": 587, "O": 587,
          "G": 7280, "N": 20640, "A": 260, "T": 184, "E": 261, "X": 144, "V": 3}
RANGES = {"_symmetry": (1, 8), "_mutationrate": (0, 10000), "_activity": (0, 2),
          "_modifiescream": (1, 3), "_modifiesfallow": (1, 4), "_modifiesspore": (1, 12), "_adaptspore": (1, 70)}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def pin_reference():
    manifest = json.loads((ROOT / "docs/evidence/reference-manifest.json").read_text())["files"]
    for relative, expected in manifest.items():
        path = WORKSPACE / relative
        stat = path.stat()
        require(not path.is_symlink() and digest(path) == expected["sha256"] and stat.st_size == expected["size"]
                and oct(stat.st_mode & 0o777) == expected["mode"], "Original reference changed: " + relative)
    require(len(manifest) == 74, "Original reference denominator changed")
    return manifest


def recipes():
    fixture = ROOT / "crates/core/tests/fixtures/xml.tsv"
    require(digest(fixture) == FIXTURE, "Frozen original XML fixture changed")
    cases, counts, active = [], Counter(), None
    for line in fixture.read_text().splitlines():
        row = line.split("\t")
        require(row[0] in FIELDS and len(row) == FIELDS[row[0]], "Original XML row shape changed")
        counts[row[0]] += 1
        if row[0] == "C":
            require(active is None, "Nested original XML case")
            active = {"name": row[1], "mode": row[2], "states": {}, "commands": [], "files": {}, "settings": {}}
            cases.append(active)
        require(active is not None, "XML row outside a case")
        if row[0] in ("G", "N", "A"):
            state = active["states"].setdefault(row[1], {"g": {}, "n": {}, "a": None})
            if row[0] == "G":
                require(row[3] not in state["g"], "Duplicate original genome field")
                state["g"][row[3]] = row[4]
            elif row[0] == "N":
                gene = state["n"].setdefault(row[3], {})
                require(row[4] not in gene, "Duplicate original gene field")
                gene[row[4]] = row[5]
            else:
                require(state["a"] is None, "Duplicate original alias observation")
                state["a"] = row[3]
        elif row[0] == "K":
            active["commands"].append(row[1:])
        elif row[0] == "U":
            active["settings"][row[1]] = row[2:]
        elif row[0] == "F":
            require(row[3] not in active["files"], "Duplicate original asset")
            active["files"][row[3]] = row
        elif row[0] == "E":
            require(len(active["settings"]) == 147, "Original settings count changed")
            active = None
    require(active is None and dict(counts) == COUNTS, "Original XML fixed row ledger changed")
    base_colors = {case["states"]["input"]["n"]["0"]["_color"] for case in cases
                   if case["name"].startswith("writer-color-") and case["name"][13:] not in ("UNKNOWN", "TRANSLUCENT", "NULL")}
    require(len(base_colors) == 46, "Original base palette changed")
    writes = []
    for case in cases:
        previous = "input" if case["mode"] == "write" else None
        for stage, operation, relative in case["commands"]:
            if operation == "parse":
                if stage in case["states"]:
                    previous = stage
                continue
            require(operation == "write" and previous is not None, "Writer lacks an actual supplied/parsed input")
            state = case["states"][previous]
            g, n = state["g"], state["n"]
            require(len(g) == 28 and int(g["_nGenes"]) == len(n) and state["a"] is not None, "Original genotype schema changed")
            require(set(n) == {str(i) for i in range(len(n))} and all(len(gene) == 40 for gene in n.values()), "Original gene schema changed")
            aliases = list(map(int, state["a"].split(","))) if state["a"] else []
            require(len(aliases) == len(n), "Original alias vector size changed")
            for i, first in enumerate(aliases):
                require(0 <= first <= i and n[str(i)] == n[str(first)], "Original gene alias recipe is invalid")
            disposition = "success"
            if any(not low <= int(g[key]) <= high for key, (low, high) in RANGES.items()):
                disposition = "InvalidAttribute"
            elif not n:
                disposition = "InvalidGenome"
            elif any(gene["_color"] not in base_colors for gene in n.values()):
                disposition = "UnsupportedColor"
            require(relative in case["files"], "Original writer output evidence missing")
            file_row = case["files"][relative]
            path = ROOT / "crates/core/tests/fixtures/xml" / relative
            local_path(ROOT, path)
            require(path.is_file() and digest(path) == file_row[4] and path.stat().st_size == int(file_row[5]), "Original writer asset changed")
            writes.append({"id": f"{len(writes):04d}", "case": case["name"], "stage": stage,
                           "input_stage": previous, "state": state, "disposition": disposition,
                           "source_file": relative, "source_sha256": file_row[4], "source_role": file_row[2],
                           "source_encoding": file_row[6]})
    require(len(writes) == 260 and Counter(item["disposition"] for item in writes) ==
            {"success": 235, "UnsupportedColor": 22, "InvalidGenome": 2, "InvalidAttribute": 1}, "Writer disposition denominator changed")
    return writes, counts


def expected_state(item):
    state = item["state"]
    g = {key: value for key, value in state["g"].items() if key != "_reference"}
    g.update(_generation="0", _cladeID="~", _updateClade="false", _mirror=str(int(g["_mirror"]) != 0 and 1 or 0))
    return {"g": g, "n": state["n"], "a": ",".join(str(i) for i in range(len(state["n"])))}


def observations(path):
    result, active, count = {}, None, Counter()
    for line in path.read_text().splitlines():
        row = line.split("\t")
        require(row[0] in ("C", "O", "G", "N", "A", "X", "R", "E"), "Unknown Java readback row")
        count[row[0]] += 1
        if row[0] == "C":
            require(active is None and len(row) == 2 and row[1] not in result, "Readback case shape")
            active = {"g": {}, "n": {}, "a": None}
            result[row[1]] = active
        require(active is not None, "Readback row outside a case")
        if row[0] == "O":
            require("outcome" not in active and len(row) == (2 if row[1] == "success" else 3), "Readback outcome shape")
            active["outcome"] = row[1]
            if row[1] != "success":
                active["exception"] = row[2]
        elif row[0] == "G":
            require(len(row) == 3 and row[1] not in active["g"], "Readback genome shape")
            active["g"][row[1]] = row[2]
        elif row[0] == "N":
            require(len(row) == 4, "Readback gene shape")
            gene = active["n"].setdefault(row[1], {})
            require(row[2] not in gene, "Duplicate readback gene field")
            gene[row[2]] = row[3]
        elif row[0] == "A":
            require(len(row) == 2 and active["a"] is None, "Readback alias shape")
            active["a"] = row[1]
        elif row[0] == "R":
            require(row == ["R", "unchanged"] and not active.get("rng"), "Readback RNG changed or duplicated")
            active["rng"] = True
        elif row[0] == "X":
            require(len(row) == 2, "Readback diagnostic shape")
            base64.b64decode(row[1], validate=True)
        elif row[0] == "E":
            require(len(row) == 1 and active.get("rng") and "outcome" in active, "Incomplete readback")
            active = None
    require(active is None, "Unclosed readback")
    return result, dict(count)


def nan(text):
    return math.isnan(struct.unpack(">d", bytes.fromhex(text))[0])


def compare(expected, actual):
    comparisons = 0
    for label, state in expected.items():
        require(label in actual and actual[label]["outcome"] == "success", "Original parser rejected Rust XML: " + label)
        got = actual[label]
        require(set(got["g"]) == set(state["g"]) and set(got["n"]) == set(state["n"]), "Parsed schema differs: " + label)
        for name, value in state["g"].items():
            require(got["g"][name] == value, f"{label}.G.{name}: expected={value} actual={got['g'][name]}")
            comparisons += 1
        for index, gene in state["n"].items():
            require(set(got["n"][index]) == set(gene), "Parsed gene schema differs: " + label)
            for name, value in gene.items():
                found = got["n"][index][name]
                equal = value == found or (name in ("_length", "_theta") and nan(value) and nan(found))
                require(equal, f"{label}.N.{index}.{name}: expected={value} actual={found}")
                comparisons += 1
        require(got["a"] == state["a"], "Parsed genes unexpectedly retain input aliases: " + label)
        comparisons += 1
    return comparisons


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("cargo", "rustc", "rustdoc", "java", "javac"):
        parser.add_argument("--" + name)
    parser.add_argument("--cache-source", type=Path, default=ROOT / ".workflow-local/cargo-home")
    parser.add_argument("--timeout", type=float, default=180)
    args = parser.parse_args(argv)
    require(sys.platform.startswith("linux") and 0 < args.timeout <= 600, "Linux and a deadline of at most600seconds are required")
    tools = {name: executable(getattr(args, name), name) for name in ("cargo", "rustc", "rustdoc", "java", "javac")}
    writes, row_counts = recipes()
    original = pin_reference()
    prior = {str(path.relative_to(ROOT)): digest(path) for path in (ROOT / "crates/core/tests/fixtures").rglob("*") if path.is_file()}
    require(digest(ROOT / "crates/core/src/xml.rs") == CODEC and digest(ROOT / "crates/core/src/xml_dtd.rs") == DTD, "Frozen codec changed")
    runs = local_path(ROOT, ROOT / ".workflow-local/xml-writer-runs")
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="writer-", dir=runs))
    print(json.dumps({"run_root": str(run)}), flush=True)
    receipts = []
    status = {"result": "FAIL", "original_writer_calls": 260, "representable": 235,
              "typed_rejections": {"UnsupportedColor": 22, "InvalidGenome": 2, "InvalidAttribute": 1},
              "source_fixture_sha256": FIXTURE, "source_row_counts": dict(row_counts),
              "codec_sha256": CODEC, "dtd_sha256": DTD,
              "codec_reviewed_predecessor": "09a6c13385ade02e0d01f3175e7c43ed429d2e981bd96eba1a13acc4691b4958",
              "codec_delta": "Reviewed charset decoder extension; parser, numeric and writer bodies remain unchanged",
              "codec_review": "docs-independent lead receipt .workflow-local/reviews/xml-charset-extension-2026-10-26.json",
              "scope": "Rust writer bytes through actual unchanged Java parsers; no external IO, atomic save, UI or full persistence acceptance",
              "equality": "All semantic fields and fresh-gene identities exact; length/theta NaN classification only; no numerical tolerance",
              "format_losses": ["generation=0", "clade=null", "updateClade=false", "nonzero mirror=1", "fresh parsed genes"],
              "source_incompatible_encoding": "Original UTF16 writer bytes retain their failed parse; Rust emits UTF8"}
    save(run / "summary.json", status)
    save(run / "writer-input-ledger.json", writes)
    output = run / "originals"
    output.mkdir()
    prepared = []
    expected = {}
    original_manifest = []
    original_success = set()
    for item in writes:
        name = item["id"]
        source = ROOT / "crates/core/tests/fixtures/xml" / item["source_file"]
        shutil.copy2(source, output / (name + ".bgg"))
        original_manifest.append(f"original-{name}\toriginals/{name}.bgg")
        if item["disposition"] == "success" and item["source_encoding"] != "UTF-16":
            original_success.add("original-" + name)
        prepared.append("C\t" + name)
        for key, value in item["state"]["g"].items():
            if key == "_cladeID" and value != "~":
                require(value.startswith("b64:"), "Original clade encoding changed")
                value = base64.b64decode(value[4:], validate=True).hex()
            prepared.append(f"G\t{key}\t{value}")
        for index, gene in item["state"]["n"].items():
            prepared.extend(f"N\t{index}\t{key}\t{value}" for key, value in gene.items())
        prepared.extend(["A\t" + item["state"]["a"], "E"])
        if item["disposition"] == "success":
            expected["rust-" + name] = expected_state(item)
    (run / "inputs.tsv").write_text("\n".join(prepared) + "\n")
    save(run / "expected-semantics.json", expected)
    status["predeclared_semantic_comparisons"] = sum(28 + 40 * len(state["n"]) for state in expected.values())
    status["predeclared_original_semantic_comparisons"] = sum(
        28 + 40 * len(item["state"]["n"]) for item in writes if "original-" + item["id"] in original_success)
    require(status["predeclared_semantic_comparisons"] == 26300 and
            status["predeclared_original_semantic_comparisons"] == 26232, "Fixed semantic comparison ledger changed")
    status["predeclared_java_calls_per_backend"] = 495
    save(run / "summary.json", status)

    snapshot = run / "source"
    copied = []
    for relative in ("Cargo.toml", "Cargo.lock", "crates/core/Cargo.toml", "crates/godot/Cargo.toml"):
        copied.append(ROOT / relative)
    for crate in ("core", "godot"):
        copied.extend(sorted((ROOT / "crates" / crate / "src").rglob("*.rs")))
    copied.extend(ROOT / "tools/reference" / name for name in ("XmlWriterInput.rs", "XmlWriterReadback.java"))
    copied.extend(ROOT / "tools" / name for name in ("check_xml_writer.py", "build.py", "run_reference_oracles.py"))
    manifest = {str(path.relative_to(ROOT)): digest(path) for path in copied}
    for path in copied:
        target_path = snapshot / path.relative_to(ROOT)
        target_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, target_path)
    require(manifest == {str(path.relative_to(ROOT)): digest(path) for path in copied}, "Canonical source changed during snapshot")
    require(manifest == {relative: digest(snapshot / relative) for relative in manifest}, "Private snapshot copy differs")
    save(run / "source-manifest.json", manifest)
    library = snapshot / "crates/core/src/lib.rs"
    require("pub use xml::{encode_genome_xml, parse_genome_xml, XmlError};" in library.read_text(),
            "Public codec exports changed; inspect before using this gate")
    cargo_home = run / "cargo-home"
    cargo_home.mkdir()
    target = run / "target"
    env = environment(os.environ, run / "profile", cargo_home, target, tools)
    for name in ("JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "CLASSPATH"):
        require(not os.environ.get(name), "Remove Java invocation override: " + name)
    status["registry_packages"] = seed_cache(ROOT, args.cache_source.resolve(), cargo_home, snapshot / "Cargo.lock")
    profiles = []
    for name in ("home", "tmp", "preferences-user", "preferences-system"):
        path = local_path(run, run / "java-profile" / name)
        path.mkdir(parents=True)
        profiles.append(path)
    flags = [f"-D{name}={path}" for name, path in zip(("user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot"), profiles)]
    flags.extend(["-Djava.awt.headless=true", "-Duser.language=en", "-Duser.country=US", "-Dfile.encoding=UTF-8", "-XX:-ShowCodeDetailsInExceptionMessages", "-Xmx512m"])
    backend = {"source": ROOT / ".workflow-local/reference-build/source-classes", "jar": WORKSPACE / "Colormod/ColorMod_2_01/biogenesis.jar"}
    jts = ROOT / ".workflow-local/reference-build/jts-core-1.15.0.jar"
    backend_pins = {str(path): digest(path) for folder in backend.values() for path in ([folder] if folder.is_file() else sorted(folder.rglob("*.class")))}
    backend_pins[str(jts)] = digest(jts)
    save(run / "backend-manifest.json", backend_pins)
    old_limit = resource.getrlimit(resource.RLIMIT_AS)
    cap = 12 * 1024 ** 3
    resource.setrlimit(resource.RLIMIT_AS, (min(old_limit[0], cap) if old_limit[0] != resource.RLIM_INFINITY else cap, old_limit[1]))
    previous = signal.signal(signal.SIGTERM, lambda signum, frame: sys.exit(128 + signum))
    try:
        with tempfile.TemporaryDirectory(prefix="bgr-xml-writer-") as directory:
            working = Path(directory)
            neutral_directory(working)
            def command(argv):
                return run_command(list(map(str, argv)), run, receipts, working, env, args.timeout)
            versions = {}
            for name, path in tools.items():
                output_text = command([path, "--version"] if name not in ("java", "javac") else [path, "-version"])
                versions[name] = (output_text or receipts[-1]["stderr"]).strip()
                require(versions[name], "Tool version output is empty: " + name)
            status["versions"] = versions
            status["executable_hashes"] = {name: {"path": str(path), "sha256": digest(path)} for name, path in tools.items()}
            command([tools["cargo"], "build", "--package", "biogenesis-core", "--lib", "--offline", "--locked", "--manifest-path", snapshot / "Cargo.toml", "--target-dir", target])
            binary = run / "xml-writer-input"
            command([tools["rustc"], "--edition=2024", snapshot / "tools/reference/XmlWriterInput.rs", "--extern", "biogenesis_core=" + str(target / "debug/libbiogenesis_core.rlib"), "-L", "dependency=" + str(target / "debug/deps"), "-o", binary])
            command([binary, run / "inputs.tsv", run / "rust"])
            outcomes = (run / "rust/outcomes.tsv").read_text().splitlines()
            require(len(outcomes) == 260, "Rust writer omitted an input")
            rust_manifest = []
            for item, line in zip(writes, outcomes):
                row = line.split("\t")
                require(row[:2] == ["O", item["id"]], "Rust output identity changed")
                document = run / "rust" / (item["id"] + ".bgg")
                if item["disposition"] == "success":
                    require(row == ["O", item["id"], "success"] and document.is_file(), "Rust writer rejected a representable input")
                    require(document.stat().st_size <= 16 * 1024 ** 2, "Rust output exceeds codec bound")
                    rust_manifest.append(f"rust-{item['id']}\trust/{item['id']}.bgg")
                else:
                    require(len(row) == 4 and row[2] == "error" and row[3].startswith(item["disposition"]), "Typed writer rejection differs: " + line)
                    require(not document.exists(), "Rejected writer input produced a document")
            require(len(rust_manifest) == 235 and len(list((run / "rust").glob("*.bgg"))) == 235, "Rust document denominator changed")
            manifest_path = run / "parse-manifest.tsv"
            manifest_path.write_text("\n".join(original_manifest + rust_manifest) + "\n")
            classes = {}
            for name, path in backend.items():
                classes[name] = run / (name + "-classes")
                classes[name].mkdir()
                command([tools["javac"], "-encoding", "UTF-8", "-cp", str(path) + os.pathsep + str(jts), "-d", classes[name], snapshot / "tools/reference/XmlWriterReadback.java"])
            hashes = [{str(path.relative_to(classes[name])): digest(path) for path in classes[name].rglob("*.class")} for name in backend]
            require(hashes[0] == hashes[1] and len(hashes[0]) == 1, "Source/JAR helper classes differ")
            status["helper_class_hashes"] = hashes[0]
            def java(name, file, output_path, arguments=flags):
                return command([tools["java"], *arguments, "-cp", str(classes[name]) + os.pathsep + str(backend[name]) + os.pathsep + str(jts), "biogenesis.XmlWriterReadback", run, file, output_path])
            decoded = []
            for name, engine in (("source-a", "source"), ("source-b", "source"), ("jar", "jar")):
                output_path = run / (name + ".tsv")
                java(engine, manifest_path, output_path)
                actual, ledger = observations(output_path)
                require(len(actual) == 495 and set(actual) == {"original-" + item["id"] for item in writes} | set(expected), "Java parse denominator differs")
                require({label for label, state in actual.items() if label.startswith("original-") and state["outcome"] == "success"} == original_success, "Original writer parse dispositions differ")
                comparisons = compare(expected, actual)
                require(comparisons == status["predeclared_semantic_comparisons"], "Semantic ledger changed")
                original_comparisons = 0
                for item in writes:
                    label = "original-" + item["id"]
                    if label in original_success:
                        original_comparisons += compare({label: expected_state(item)}, actual)
                require(original_comparisons == status["predeclared_original_semantic_comparisons"], "Original semantic ledger changed")
                decoded.append(actual)
                status.setdefault("backends", []).append({"name": name, "calls": 495, "rust_semantic_comparisons": comparisons,
                                                          "original_semantic_comparisons": original_comparisons,
                                                          "original_successes": len(original_success), "original_failures": 260 - len(original_success),
                                                          "sha256": digest(output_path), "rows": ledger})
                save(run / "summary.json", status)
            require(decoded[0] == decoded[1] == decoded[2], "Source/source/JAR readback differs")
            require(digest(run / "source-a.tsv") == digest(run / "source-b.tsv") == digest(run / "jar.tsv"), "Java readback byte identity differs")
            negative = run / "negative.bgg"
            first = (run / "rust/0000.bgg").read_bytes()
            require(b'branch="-1"' in first, "Negative-control source changed")
            negative.write_bytes(first.replace(b'branch="-1"', b'branch="2"', 1))
            (run / "negative-manifest.tsv").write_text("rust-0000\tnegative.bgg\n")
            java("source", run / "negative-manifest.tsv", run / "negative.tsv")
            try:
                compare({"rust-0000": expected["rust-0000"]}, observations(run / "negative.tsv")[0])
            except ValueError as error:
                require("._branch" in str(error), "Negative control failed for an unrelated field")
                status["negative_semantic_control"] = {"result": "REJECTED", "diagnostic": str(error), "actual_java_parse": "success", "modified_input": "first gene branch -1->2"}
            else:
                raise ValueError("Semantic negative control passed unexpectedly")
            guards = []
            for index in range(6):
                guard_flags = flags.copy()
                output_path = run / (f"guard-{index}.tsv")
                if index < 4:
                    del guard_flags[index]
                elif index == 4:
                    guard_flags[0] = "-Duser.home=/tmp"
                else:
                    output_path.write_text("owned existing output\n")
                before = output_path.read_bytes() if output_path.exists() else None
                try:
                    java("source", manifest_path, output_path, guard_flags)
                except ValueError:
                    require(receipts[-1]["exit_code"] != 0 and not receipts[-1]["timed_out"], "Isolation guard did not fail cleanly")
                    require(output_path.read_bytes() == before if before is not None else not output_path.exists(), "Guard failure changed output")
                    guards.append({"index": index, "result": "REJECTED", "receipt": len(receipts) - 1})
                else:
                    raise ValueError("Isolation guard passed unexpectedly")
            status["isolation_guards"] = guards
            require(original == pin_reference(), "Original pins changed")
            require(prior == {str(path.relative_to(ROOT)): digest(path) for path in (ROOT / "crates/core/tests/fixtures").rglob("*") if path.is_file()}, "Prior fixtures changed")
            require(backend_pins == {path: digest(Path(path)) for path in backend_pins}, "Original compiled backend changed")
            require(digest(ROOT / "crates/core/src/xml.rs") == CODEC and digest(ROOT / "crates/core/src/xml_dtd.rs") == DTD, "Codec changed during gate")
            require(manifest == {relative: digest(snapshot / relative) for relative in manifest}, "Private source snapshot changed during execution")
            status["owned_process_cleanup"] = "Every command uses bounded run_command process-group TERM/KILL cleanup, including expected failures"
            status.update(result="PASS", reference_files_preserved=74, prior_fixture_files_preserved=len(prior),
                          source_source_jar_identical=True, complete_java_calls=1486,
                          rust_documents={path.name: {"sha256": digest(path), "bytes": path.stat().st_size} for path in (run / "rust").glob("*.bgg")})
            save(run / "summary.json", status)
            print(json.dumps({"result": "PASS", "run_root": str(run), "writer_inputs": 260,
                              "rust_documents": 235, "typed_rejections": 25, "semantic_comparisons_per_backend": status["predeclared_semantic_comparisons"]}), flush=True)
            return 0
    except (ValueError, OSError, SystemExit) as error:
        status["error"] = str(error)
        save(run / "summary.json", status)
        print(str(error), file=sys.stderr)
        return 1
    finally:
        signal.signal(signal.SIGTERM, previous)
        resource.setrlimit(resource.RLIMIT_AS, old_limit)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
