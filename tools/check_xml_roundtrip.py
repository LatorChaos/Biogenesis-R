"""Verify frozen Rust writer bytes through Rust decoding, re-encoding and actual Java parsing."""

import argparse
from collections import Counter
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from build import environment, executable, local_path, neutral_directory, seed_cache
from run_reference_oracles import run_command, require

CODEC = "986f33caa59b95a29da32c1c3d9f9bef9d147d1aa9b2a2bdb175cb94b0a2860c"
DTD = "510c6457b146482b767ba0474676bcc38f691a14e8cb6dd3685e69b2b3158b8d"
DRIVER = "acd09e6d2ad671d7148780f4a5be4de0b013bf8d2b815dec653aab794a839d1b"
READBACK_CLASS = "aac4403d5ebfcf721a718470b98631ef2b0aec53d50ba848d53403c623c05691"
JAVA_FIRST = "8c25157afce0f4fae0bda61c724eb25261342d2423229a1482e170dae328f1f8"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def inventory(root):
    return {str(path.relative_to(root)): digest(path) for path in sorted(root.rglob("*")) if path.is_file()}


def selected_manifest(candidate):
    paths = [candidate / name for name in ("Cargo.toml", "Cargo.lock", "crates/core/Cargo.toml", "crates/godot/Cargo.toml")]
    for crate in ("core", "godot"):
        for folder in ("src", "tests"):
            directory = candidate / "crates" / crate / folder
            if directory.is_dir():
                paths.extend(path for path in sorted(directory.rglob("*")) if path.is_file())
    for path in paths:
        local_path(candidate, path)
    return {str(path.relative_to(candidate)): digest(path) for path in paths}


def decoded_rows(path):
    result, active = {}, None
    for line in path.read_text().splitlines():
        row = line.split("\t")
        if row[0] == "C":
            require(active is None and len(row) == 2 and row[1] not in result, "Duplicate or malformed Rust identity")
            active = {"g": {}, "n": {}, "a": None}
            result[row[1]] = active
        require(active is not None, "Rust row outside a decoded genotype")
        if row[0] == "O":
            require(row == ["O", "success"] and "outcome" not in active, "Rust decode outcome changed")
            active["outcome"] = "success"
        elif row[0] == "G":
            require(len(row) == 3 and row[1] not in active["g"], "Rust genome schema changed")
            active["g"][row[1]] = row[2]
        elif row[0] == "N":
            require(len(row) == 4, "Rust gene row changed")
            fields = active["n"].setdefault(row[1], {})
            require(row[2] not in fields, "Duplicate Rust gene field")
            fields[row[2]] = row[3]
        elif row[0] == "A":
            require(len(row) == 2 and active["a"] is None, "Rust alias row changed")
            active["a"] = row[1]
        elif row[0] == "E":
            require(row == ["E"] and active.get("outcome") == "success" and active["a"] is not None, "Incomplete Rust state")
            active = None
        else:
            require(row[0] == "C", "Unknown Rust observation row")
    require(active is None, "Unclosed Rust state")
    return result


def strict_compare(writer, expected, actual):
    require(set(expected) == set(actual), "Decoded identity denominator differs")
    count = writer.compare(expected, actual)
    # The independent Java parse already records the format's canonical NaN bits
    for label, state in expected.items():
        for index, gene in state["n"].items():
            for name in ("_length", "_theta"):
                require(actual[label]["n"][index][name] == gene[name],
                        f"{label}.N.{index}.{name}: exact canonical bits differ")
    return count


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("cargo", "rustc", "rustdoc", "java"):
        parser.add_argument("--" + name)
    parser.add_argument("--cache-source", type=Path, default=ROOT / ".workflow-local/cargo-home")
    parser.add_argument("--candidate-source", type=Path, default=ROOT,
                        help="Checkout or immutable project-local source snapshot; copy only manifests and crate sources/tests")
    parser.add_argument("--writer-run", type=Path, required=True,
                        help="Completed project-local writer gate with the pinned codec, driver and Java observations")
    args = parser.parse_args(argv)
    candidate = local_path(ROOT, args.candidate_source.absolute()).resolve()
    writer_run = local_path(ROOT, args.writer_run.absolute()).resolve()
    require(candidate.is_dir() and writer_run.is_dir(), "Candidate source and writer run directories are required")
    tools = {name: executable(getattr(args, name), name) for name in ("cargo", "rustc", "rustdoc", "java")}
    frozen_driver = writer_run / "source/tools/check_xml_writer.py"
    require(digest(frozen_driver) == DRIVER, "Frozen writer driver changed")
    spec = importlib.util.spec_from_file_location("_frozen_xml_writer", frozen_driver)
    writer = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(writer)
    writer.ROOT, writer.WORKSPACE = ROOT, ROOT.parent
    writes, counts = writer.recipes()
    original_pins = writer.pin_reference()
    require(digest(candidate / "crates/core/src/xml.rs") == CODEC
            and digest(candidate / "crates/core/src/xml_dtd.rs") == DTD, "Selected codec changed")
    writer_summary = digest(writer_run / "summary.json")
    require(digest(writer_run / "source-a.tsv") == JAVA_FIRST, "Pinned original Java observations changed")
    old = json.loads((writer_run / "summary.json").read_text())
    require(old["result"] == "PASS" and len(old["rust_documents"]) == 235
            and old["original_writer_calls"] == 260 and old["representable"] == 235
            and old["codec_sha256"] == CODEC and old["dtd_sha256"] == DTD,
            "Selected writer proof does not cover this codec and denominator")
    require(old["helper_class_hashes"] == {"biogenesis/XmlWriterReadback.class": READBACK_CLASS},
            "Writer readback class differs from the independently reviewed helper")
    writer_manifest = json.loads((writer_run / "source-manifest.json").read_text())
    require(writer_manifest["crates/core/src/xml.rs"] == CODEC
            and writer_manifest["crates/core/src/xml_dtd.rs"] == DTD
            and writer_manifest["tools/check_xml_writer.py"] == DRIVER,
            "Writer source custody does not match the pinned codec and driver")
    for relative, checksum in writer_manifest.items():
        path = local_path(writer_run, writer_run / "source" / relative)
        require(digest(path) == checksum, "Writer source snapshot changed: " + relative)
    previous = writer.observations(writer_run / "source-a.tsv")[0]
    require((writer_run / "source-a.tsv").read_bytes() == (writer_run / "source-b.tsv").read_bytes()
            == (writer_run / "jar.tsv").read_bytes(), "Frozen Java first-document observations differ")
    expected = {"rust-" + item["id"]: {key: previous["rust-" + item["id"]][key] for key in ("g", "n", "a")}
                for item in writes if item["disposition"] == "success"}
    normalized = {"rust-" + item["id"]: writer.expected_state(item) for item in writes if item["disposition"] == "success"}
    require(writer.compare(normalized, previous) == 26300, "Independent source normalization differs")
    run_parent = local_path(ROOT, ROOT / ".workflow-local/xml-roundtrip-runs")
    run_parent.mkdir(exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="chain-", dir=run_parent))
    print(json.dumps({"run_root": str(run)}), flush=True)
    receipts = []
    for signum in (signal.SIGTERM, signal.SIGINT):
        signal.signal(signum, lambda sig, _frame: sys.exit(128 + sig))
    prior = inventory(ROOT / "crates/core/tests/fixtures")
    baseline = selected_manifest(candidate)
    source = run / "source"
    for relative, checksum in baseline.items():
        target = source / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(candidate / relative, target)
        require(digest(target) == checksum, "Selected source changed during snapshot: " + relative)
    require(selected_manifest(candidate) == baseline, "Selected source changed during snapshot")
    helper = ROOT / "tools/reference/XmlRoundtrip.rs"
    parent = source / "crates/core/src/organism_oracle_tests.rs"
    parent.write_text(parent.read_text() + '\n#[path = "xml_roundtrip_probe.rs"]\nmod xml_roundtrip_probe;\n')
    shutil.copy2(helper, source / "crates/core/src/xml_roundtrip_probe.rs")
    source_manifest = inventory(source)
    inputs = run / "inputs"
    inputs.mkdir()
    manifest = []
    retained_outcomes = (writer_run / "rust/outcomes.tsv").read_text().splitlines()
    require(len(retained_outcomes) == 260, "Retained writer outcomes are incomplete")
    for item, line in zip(writes, retained_outcomes):
        row = line.split("\t")
        require(row[:2] == ["O", item["id"]], "Retained writer identity differs")
        document = writer_run / "rust" / (item["id"] + ".bgg")
        if item["disposition"] == "success":
            require(row == ["O", item["id"], "success"] and digest(document) == old["rust_documents"][document.name]["sha256"], "Retained document differs")
            shutil.copy2(document, inputs / document.name)
            manifest.append(f"rust-{item['id']}\tinputs/{document.name}")
        else:
            require(not document.exists() and len(row) == 4 and row[2] == "error"
                    and row[3].startswith(item["disposition"]), "Retained rejection differs")
    require(len(manifest) == 235, "Roundtrip success denominator differs")
    manifest_path = run / "manifest.tsv"
    manifest_path.write_text("\n".join(manifest) + "\n")
    save(run / "expected-semantics.json", expected)
    gate_sources = [Path(__file__), helper, ROOT / "tools/build.py", ROOT / "tools/run_reference_oracles.py"]
    gate_hashes = {str(path.relative_to(ROOT)): digest(path) for path in gate_sources}
    for path in gate_sources:
        target = run / "gate-used" / path.relative_to(ROOT)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, target)
    save(run / "input-manifest.json", {"codec": CODEC, "dtd": DTD, "baseline": str(candidate), "baseline_sha256": baseline,
         "private_source_sha256": source_manifest, "writer_driver": DRIVER, "helper": digest(helper),
         "gate_sources": gate_hashes, "tool_executables": {name: {"path": str(path), "sha256": digest(path)} for name, path in tools.items()},
         "selected_writer_run": str(writer_run), "frozen_writer_summary": writer_summary,
         "frozen_first_java_readback": JAVA_FIRST, "writer_source_manifest": writer_manifest,
         "first_documents": inventory(inputs), "retained260stage_dispositions": {item["id"]: item["disposition"] for item in writes},
         "expected_authority": "Independent original Kwrite normalizations, then exact actual source/source/JAR first-document parse bits",
         "constructor_authority": "Only actual first-document bytes; no expected G/N data enters Rust construction"})
    env = environment(os.environ, run / "profile", run / "cargo-home", run / "target", tools)
    seed_cache(ROOT, args.cache_source.resolve(), run / "cargo-home", source / "Cargo.lock")
    env.update(BGR_XML_CHAIN_ROOT=str(run), BGR_XML_CHAIN_MANIFEST=str(manifest_path), BGR_XML_CHAIN_COUNT="235")
    with tempfile.TemporaryDirectory(prefix="bgr-xml-chain-") as directory:
        working = Path(directory)
        neutral_directory(working)
        def command(argv, process_env=env, timeout=180):
            return run_command(list(map(str, argv)), run, receipts, working, process_env, timeout)
        base = [tools["cargo"], "test", "--offline", "--locked", "--manifest-path", source / "Cargo.toml", "-p", "biogenesis-core", "--lib", "xml_roundtrip_actual_bytes"]
        rust_results = {}
        for profile, extra in (("debug", []), ("release", ["--release"])):
            output = run / ("rust-" + profile)
            env["BGR_XML_CHAIN_OUTPUT"] = str(output)
            command(base + extra + ["--", "--nocapture"])
            actual = decoded_rows(output / "decoded.tsv")
            rust_results[profile] = strict_compare(writer, expected, actual)
            require(rust_results[profile] == 26300, "Rust semantic ledger changed")
        require(inventory(run / "rust-debug") == inventory(run / "rust-release"), "Rust profile outputs differ")
        java_manifest = run / "java-manifest.tsv"
        java_manifest.write_text("\n".join(f"{label}\trust-debug/{label[5:]}.bgg" for label in expected) + "\n")
        jts = ROOT / ".workflow-local/reference-build/jts-core-1.15.0.jar"
        backends = {"source": ROOT / ".workflow-local/reference-build/source-classes", "jar": ROOT.parent / "Colormod/ColorMod_2_01/biogenesis.jar"}
        backend_pins = {str(path): digest(Path(path)) for path in json.loads((writer_run / "backend-manifest.json").read_text())}
        java_rows = []
        def java(label, backend, input_manifest):
            directory = run / label
            directory.mkdir()
            flags = []
            for key, folder in (("user.home", "home"), ("java.io.tmpdir", "tmp"), ("java.util.prefs.userRoot", "prefs-user"), ("java.util.prefs.systemRoot", "prefs-system")):
                path = directory / folder
                path.mkdir()
                flags.append(f"-D{key}={path}")
            classes = writer_run / (backend + "-classes")
            require(digest(classes / "biogenesis/XmlWriterReadback.class") == READBACK_CLASS, "Frozen Java helper changed")
            target = directory / "observations.tsv"
            command([tools["java"], "-Xmx512m", "-Djava.awt.headless=true", "-Dfile.encoding=UTF-8", "-Duser.language=en", "-Duser.country=US", "-XX:-ShowCodeDetailsInExceptionMessages", *flags, "-cp", os.pathsep.join(map(str, [classes, backends[backend], jts])), "biogenesis.XmlWriterReadback", run, input_manifest, target])
            return writer.observations(target)[0], target
        for label, backend in (("source-a", "source"), ("source-b", "source"), ("jar", "jar")):
            actual, path = java(label, backend, java_manifest)
            require(strict_compare(writer, expected, actual) == 26300, "Second-generation Java semantic ledger changed")
            java_rows.append(path.read_bytes())
        require(java_rows[0] == java_rows[1] == java_rows[2], "Second-generation source/source/JAR differs")
        first = (inputs / "0000.bgg").read_bytes()
        require(b'branch="-1"' in first, "Negative source input changed")
        (run / "negative.bgg").write_bytes(first.replace(b'branch="-1"', b'branch="2"', 1))
        negative_manifest = run / "negative-manifest.tsv"
        negative_manifest.write_text("rust-0000\tnegative.bgg\n")
        env.update(BGR_XML_CHAIN_MANIFEST=str(negative_manifest), BGR_XML_CHAIN_COUNT="1", BGR_XML_CHAIN_OUTPUT=str(run / "rust-negative"))
        command(base + ["--", "--nocapture"])
        try:
            strict_compare(writer, {"rust-0000": expected["rust-0000"]}, decoded_rows(run / "rust-negative/decoded.tsv"))
        except ValueError as error:
            require("._branch" in str(error), "Rust negative failed on an unrelated field")
            negative_rust = str(error)
        else:
            raise ValueError("Rust parsed-field negative control passed")
        negative_java_manifest = run / "negative-java-manifest.tsv"
        negative_java_manifest.write_text("rust-0000\trust-negative/0000.bgg\n")
        actual, _ = java("java-negative", "source", negative_java_manifest)
        try:
            strict_compare(writer, {"rust-0000": expected["rust-0000"]}, actual)
        except ValueError as error:
            require("._branch" in str(error), "Java negative failed on an unrelated field")
            negative_java = str(error)
        else:
            raise ValueError("Java second-generation negative control passed")
    require(selected_manifest(candidate) == baseline and inventory(source) == source_manifest, "Immutable source changed")
    require(digest(writer_run / "summary.json") == writer_summary
            and digest(writer_run / "source-a.tsv") == JAVA_FIRST, "Selected writer evidence changed")
    require(writer_manifest == {relative: digest(writer_run / "source" / relative) for relative in writer_manifest},
            "Writer source snapshot changed during the chain gate")
    require(gate_hashes == {path: digest(ROOT / path) for path in gate_hashes}, "Gate sources changed during execution")
    require(inventory(ROOT / "crates/core/tests/fixtures") == prior and writer.pin_reference() == original_pins, "Original data changed")
    require(backend_pins == {path: digest(Path(path)) for path in backend_pins}, "Original Java backend changed")
    result = {"result": "PASS", "writer_stages": 260, "chain_documents": 235, "retained_typed_rejections": 25,
              "codec": CODEC, "dtd": DTD, "selected_candidate_source": str(candidate),
              "selected_writer_run": str(writer_run), "writer_summary_sha256": writer_summary,
              "rust_comparisons_per_profile": 26300, "java_comparisons_per_backend": 26300,
              "all_fields_and_canonical_nan_bits_exact": True, "rust_debug_release_byte_equal": True,
              "source_source_jar_byte_equal": True, "java_parse_calls": 706,
              "negative_controls": {"rust_decoded_branch": negative_rust, "java_reencoded_branch": negative_java},
              "original_fixture_rows": dict(counts), "source_and_fixtures_preserved": True,
              "scope": "Actual frozen Rust bytes -> Rust parse -> Rust encode -> original Java parse; no atomic file or UI acceptance",
              "copied_rng_attribution": "No Rust RNG observation is fabricated; original Java helper checks its generator purity"}
    save(run / "summary.json", result)
    print(json.dumps({"result": "PASS", "run_root": str(run), "chain_documents": 235, "retained_typed_rejections": 25}), flush=True)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
