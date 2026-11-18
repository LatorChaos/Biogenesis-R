"""Run the required ordered base-color fidelity matrix with private offline controls"""

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import subprocess
import sys
import tempfile
import time
import tomllib

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
MANIFEST = Path("crates/core/tests/fixtures/matrix-full.manifest.json")
MANIFEST_SHA256 = "fb3418e35294d345853f45026dc5c24f1730461d5d3ee51c48503c1bd51604f8"
COLORS = "RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW".split()
TEST = "world::organism_oracle_tests::reproduction_oracle_tests::color_matrix_full_oracle_tests::full_color_matrix_external_shard_matches_original_java"
FIELDS = "rows cases snapshots operations eligible input_GN collaborators metadata actor_views discoveries insertions owner_projections".split()
MAX_SHARD_BYTES = 25_000_000
CONTROLS = ("check_color_matrix.py", "build.py", "check_workload_endpoints.py", "run_reference_oracles.py")


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def local(path, base=ROOT):
    path = Path(path)
    if not path.is_absolute():
        path = base / path
    require(path.resolve().is_relative_to(base.resolve()), "Path escapes owned root: " + str(path))
    for part in [path, *path.parents]:
        if part == base:
            break
        require(not part.is_symlink(), "Path uses a symlink: " + str(part))
    return path


def terminate(number, _frame):
    raise SystemExit(128 + number)


def complete_rows(rows):
    require([row["actor"] for row in rows] == COLORS, "All46 ordered actor rows are required")


def load_manifest():
    path = local(MANIFEST)
    require(path.is_file() and digest(path) == MANIFEST_SHA256, "Frozen matrix manifest differs")
    data = json.loads(path.read_text())
    require(data["schema"] == 1, "Unsupported matrix manifest schema")
    complete_rows(data["shards"])
    require(data["limits"]["maximum_expanded_shard_bytes"] == MAX_SHARD_BYTES, "Expansion policy differs")
    require((data["expected_totals"]["cases"], data["expected_totals"]["snapshots"], data["expected_totals"]["operations"]) == (2116, 16928, 14812), "Fixed full protocol differs")
    table = local(data["table"]["path"])
    require(table.is_file() and digest(table) == data["table"]["sha256"], "Source-derived Rust ledger table differs")
    require(sum(row["compressed_bytes"] for row in data["shards"]) == data["limits"]["expected_gzip_bytes"], "Compressed corpus size differs")
    require(sum(row["expanded_bytes"] for row in data["shards"]) == data["limits"]["expected_expanded_bytes"], "Expanded corpus size differs")
    return data


def verify_compressed(row):
    path = local(row["gzip"])
    require(path.is_file() and path.stat().st_size == row["compressed_bytes"] and digest(path) == row["compressed_sha256"], "Frozen compressed input differs: " + row["actor"])
    require(0 < row["expanded_bytes"] <= MAX_SHARD_BYTES, "Frozen expansion size exceeds policy")
    return path


def expand(row, destination):
    encoded = verify_compressed(row)
    size = 0
    checksum = hashlib.sha256()
    with gzip.open(encoded, "rb") as source, destination.open("xb") as output:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            size += len(block)
            require(size <= row["expanded_bytes"] and size <= MAX_SHARD_BYTES, "Expansion exceeds frozen bound")
            checksum.update(block)
            output.write(block)
    require(size == row["expanded_bytes"] and checksum.hexdigest() == row["expanded_sha256"], "Expanded input differs")


def marker(output):
    rows = [line.partition("FULL_MATRIX_ROW_MATCH\t")[2] for line in output.splitlines() if "FULL_MATRIX_ROW_MATCH\t" in line]
    require(len(rows) == 1, "One complete row marker is required")
    entries = [item.split("=", 1) for item in rows[0].split("\t")]
    require(all(len(item) == 2 for item in entries), "Malformed row marker field")
    fields = dict(entries)
    require(len(fields) == len(entries) and set(fields) == {"actor", *FIELDS}, "Complete unique row marker fields are required")
    return {"actor": fields["actor"], **{key: int(fields[key]) for key in FIELDS}}


def test_result(output):
    rows = re.findall(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", output, re.M)
    require(len(rows) == 1, "One test result is required")
    return tuple(map(int, rows[0]))


def freeze_controls(run):
    records = {}
    for name in CONTROLS:
        origin = local(Path("tools") / name)
        checksum = digest(origin)
        target = local(run / "controls/tools" / name, run)
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open("xb") as output:
            output.write(origin.read_bytes())
        require(digest(target) == checksum == digest(origin), "Python control changed while copying")
        records[name] = {"origin": str(origin), "copy": str(target), "sha256": checksum}
    (run / "controls.json").write_text(json.dumps(records, indent=2) + "\n")
    return records


def verify_controls(records):
    for record in records.values():
        require(digest(Path(record["origin"])) == record["sha256"] == digest(Path(record["copy"])), "Original or retained Python control changed")


def load_helpers(records):
    verify_controls(records)
    directory = Path(records["build.py"]["copy"]).parent
    sys.path.insert(0, str(directory))
    import build
    import check_workload_endpoints
    import run_reference_oracles
    for name, module in [("build.py", build), ("check_workload_endpoints.py", check_workload_endpoints), ("run_reference_oracles.py", run_reference_oracles)]:
        require(Path(module.__file__).resolve() == Path(records[name]["copy"]).resolve(), "Unverified helper import")
    return build, check_workload_endpoints, run_reference_oracles


def manifest_of(root, inputs):
    return {str(path.relative_to(root)): digest(path) for path in inputs(root)}


def absent(group):
    try:
        os.killpg(group, 0)
    except ProcessLookupError:
        return True
    except PermissionError:
        return False
    return False


def _cancellation_worker(run):
    records = freeze_controls(run)
    _, _, runner = load_helpers(records)
    previous = signal.signal(signal.SIGTERM, terminate)
    try:
        code = 'import json,os,pathlib,sys,time; pathlib.Path(sys.argv[1]).write_text(json.dumps({"pid":os.getpid(),"group":os.getpgrp()})); time.sleep(60)'
        runner.run_command([sys.executable, "-B", "-c", code, str(run / "ready.json")], run, [], run, os.environ.copy(), 90)
        raise ValueError("Cancellation was not requested")
    finally:
        signal.signal(signal.SIGTERM, previous)


def cancellation_check(run):
    worker = local(run / "cancel-worker", run)
    worker.mkdir()
    code = 'import importlib.util,pathlib,sys; spec=importlib.util.spec_from_file_location("matrix_gate",sys.argv[1]); gate=importlib.util.module_from_spec(spec); spec.loader.exec_module(gate); gate._cancellation_worker(pathlib.Path(sys.argv[2]))'
    process = subprocess.Popen([sys.executable, "-B", "-c", code, str(Path(__file__).resolve()), str(worker)], cwd=worker, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
    child_group = None
    try:
        deadline = time.monotonic() + 10
        while not (worker / "ready.json").exists() and process.poll() is None and time.monotonic() < deadline:
            time.sleep(0.02)
        require((worker / "ready.json").is_file(), "Owned cancellation child did not start")
        ready = json.loads((worker / "ready.json").read_text())
        child_group = ready["group"]
        require(ready["pid"] == child_group, "Cancellation child lacks its own group")
        process.send_signal(signal.SIGTERM)
        stdout, stderr = process.communicate(timeout=12)
        receipts = json.loads((worker / "commands.json").read_text())
        require(process.returncode == 143 and len(receipts) == 1 and receipts[0]["interrupted"] and not receipts[0]["timed_out"], "Interruption receipt differs")
        require(receipts[0]["process_group"] == child_group and absent(child_group) and absent(process.pid), "Owned cancellation group remains")
        return {"worker_exit": 143, "child_exit": receipts[0]["exit_code"], "interrupted": True, "owned_groups_absent": True, "stdout": stdout, "stderr": stderr, "scope": "Owned Python processes only"}
    finally:
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.communicate(timeout=12)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.communicate(timeout=3)
        if child_group is not None and not absent(child_group):
            try:
                os.killpg(child_group, signal.SIGKILL)
            except ProcessLookupError:
                pass


def self_check(run):
    results = []
    def rejects(action, category):
        try:
            action()
        except ValueError as error:
            require(category in str(error), "Unexpected guard rejection: " + str(error))
            results.append({"category": category, "result": "rejected"})
        else:
            raise ValueError("Invalid metadata was accepted")
    rows = [{"actor": color} for color in COLORS]
    complete_rows(rows)
    results.append({"category": "complete actor order", "result": "accepted"})
    rejects(lambda: complete_rows(rows[:-1]), "All46 ordered actor rows")
    changed = list(rows)
    changed[0], changed[1] = changed[1], changed[0]
    rejects(lambda: complete_rows(changed), "All46 ordered actor rows")
    data = b"owned fixture\n"
    compressed = run / "owned.gz"
    compressed.write_bytes(gzip.compress(data, mtime=0))
    row = {"actor": "owned", "gzip": str(compressed), "compressed_bytes": compressed.stat().st_size, "compressed_sha256": digest(compressed), "expanded_bytes": len(data), "expanded_sha256": hashlib.sha256(data).hexdigest()}
    expand(row, run / "positive.tsv")
    require((run / "positive.tsv").read_bytes() == data, "Positive owned expansion differs")
    results.append({"category": "exact owned expansion", "result": "accepted"})
    bounded = {**row, "expanded_bytes": 3}
    rejects(lambda: expand(bounded, run / "bounded.tsv"), "Expansion exceeds frozen bound")
    compressed.write_bytes(compressed.read_bytes() + b"x")
    rejects(lambda: expand(row, run / "changed.tsv"), "Frozen compressed input differs")
    text = "FULL_MATRIX_ROW_MATCH\tactor=RED" + "".join("\t" + field + "=1" for field in FIELDS)
    marker(text)
    results.append({"category": "complete marker", "result": "accepted"})
    rejects(lambda: marker(text.rsplit("\t", 1)[0]), "Complete unique row marker")
    rejects(lambda: marker(text + "\n" + text), "One complete row marker")
    rejects(lambda: marker(text + "\trows=1"), "Complete unique row marker")
    cancellation = cancellation_check(run)
    return {"status": "PASS_METADATA_AND_OWNED_CANCELLATION", "metadata_checks": results, "cancellation": cancellation, "Rust_or_Java_processes": 0}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["cargo", "rustc", "rustdoc"]:
        parser.add_argument("--" + name, help="Installed executable path or discoverable command name")
    cache_default = ROOT / ".workflow-local/cargo-home"
    parser.add_argument("--cache-source", type=Path, default=cache_default if cache_default.is_dir() else Path.home() / ".cargo")
    parser.add_argument("--timeout", type=float, default=600)
    parser.add_argument("--self-check", action="store_true", help="Run bounded metadata and owned Python cancellation checks only")
    args = parser.parse_args(argv)
    require(sys.platform.startswith("linux"), "This gate currently supports native Linux")
    require(0 < args.timeout <= 3600, "Timeout must be positive and at most3600seconds")
    runs = local(Path(".workflow-local/color-matrix-runs"))
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="self-check-" if args.self_check else "matrix-", dir=runs))
    print(json.dumps({"run_root": str(run)}), flush=True)
    summary = {"status": "FAIL", "mode": "self-check" if args.self_check else "full-matrix", "phase": "preflight"}
    (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    try:
        return execute(args, run, summary)
    except BaseException as error:
        summary["status"] = "FAIL"
        summary["error_type"] = type(error).__name__
        summary["error"] = str(error)
        (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        raise


def execute(args, run, summary):
    controls = freeze_controls(run)
    build, endpoints, runner = load_helpers(controls)
    if args.self_check:
        summary["phase"] = "metadata-and-cancellation"
        result = self_check(run)
        verify_controls(controls)
        summary.clear()
        summary.update(result)
        (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps(summary), flush=True)
        return 0
    data = load_manifest()
    for row in data["shards"]:
        verify_compressed(row)
    source = run / "source"
    before = endpoints.snapshot(ROOT, source)
    (run / "source-manifest.json").write_text(json.dumps(before, indent=2) + "\n")
    tools = {name: build.executable(getattr(args, name), name) for name in ["cargo", "rustc", "rustdoc"]}
    tool_hashes = {name: digest(path) for name, path in tools.items()}
    cargo_home, target = run / "cargo-home", run / "target"
    cargo_home.mkdir()
    env = build.environment(os.environ, run / "profile", cargo_home, target, tools)
    registry_count = sum("source" in package for package in tomllib.loads((source / "Cargo.lock").read_text())["package"])
    copied_packages = build.seed_cache(ROOT, args.cache_source.resolve(), cargo_home, source / "Cargo.lock")
    require(copied_packages == registry_count, "Current locked dependency closure differs")
    requirement = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]["rust-version"]
    minimum = tuple(map(int, requirement.split(".")[:2]))
    receipts, binaries = [], {}
    summary.update(fixture_manifest_sha256=MANIFEST_SHA256, source_manifest_sha256=digest(run / "source-manifest.json"), tool_sha256=tool_hashes, registry_packages=copied_packages, profiles={name: {"rows": []} for name in ["debug", "release"]})
    (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    old_limit = resource.getrlimit(resource.RLIMIT_AS)
    def limit(gib):
        cap = gib * 1024 ** 3
        resource.setrlimit(resource.RLIMIT_AS, (min(cap, old_limit[0]) if old_limit[0] != resource.RLIM_INFINITY else cap, old_limit[1]))
    try:
        with tempfile.TemporaryDirectory(prefix="bgr-color-matrix-cargo-") as temporary:
            working = Path(temporary)
            build.neutral_directory(working)
            def command(arguments, command_env=env):
                return runner.run_command(list(map(str, arguments)), run, receipts, working, command_env, args.timeout)
            versions = {}
            summary["phase"] = "installed-tool-versions"
            for name, path in tools.items():
                versions[name] = command([path, "--version"]).strip()
                version = re.match(name + r" (\d+)\.(\d+)\.", versions[name])
                require(version and tuple(map(int, version.groups())) >= minimum, "Installed tool does not meet current Rust requirement")
            summary["versions"] = versions
            for profile in ["debug", "release"]:
                summary["phase"] = "compile-and-normal-" + profile
                (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
                limit(12 if profile == "debug" else 24)
                arguments = [tools["cargo"], "test", "--package", "biogenesis-core", "--lib", "--no-run", "--offline", "--locked", "--manifest-path", source / "Cargo.toml", "--target-dir", target, "--message-format=json-render-diagnostics"]
                if profile == "release":
                    arguments.append("--release")
                output = command(arguments)
                artifacts = [Path(item["executable"]) for item in map(json.loads, output.splitlines()) if item.get("reason") == "compiler-artifact" and item.get("executable") and item["target"]["name"] == "biogenesis_core"]
                require(len(artifacts) == 1, "One fresh core executable is required")
                binary = local(artifacts[0], target)
                require(binary.is_file(), "Fresh core executable is missing")
                binaries[profile] = binary
                limit(4)
                listing = command([binary, "--list", "--ignored", "--exact", TEST])
                require(TEST + ": test" in listing and "1 test" in listing, "The required ignored matrix entry is missing")
                normal = command([binary, "--test-threads=1"])
                result = test_result(normal)
                require(result[0] > 0 and result[1] == 0, "Ordinary core tests failed or did not execute")
                summary["profiles"][profile].update(binary_sha256=digest(binary), normal_test_result=result)
                (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
            expanded_root = run / "expanded"
            expanded_root.mkdir()
            for row in data["shards"]:
                directory = expanded_root / row["actor"]
                directory.mkdir()
                expanded = directory / "source.tsv"
                expand(row, expanded)
                row_env = {**env, "BGR_COLOR_MATRIX_ACTOR": row["actor"], "BGR_COLOR_MATRIX_SHARD": str(expanded)}
                limit(4)
                for profile in ["debug", "release"]:
                    summary["phase"] = row["actor"] + "-" + profile
                    (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
                    output = command([binaries[profile], "--exact", TEST, "--ignored", "--nocapture", "--test-threads=1"], row_env)
                    require(test_result(output) == (1, 0, 0), "Required ignored row did not execute once")
                    actual = marker(output)
                    require(actual == {"actor": row["actor"], **row["expected_marker"]}, "Full row counters differ")
                    summary["profiles"][profile]["rows"].append(actual)
                    (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
                expanded.unlink()
            for profile in ["debug", "release"]:
                rows = summary["profiles"][profile]["rows"]
                complete_rows(rows)
                totals = {key: sum(row[key] for row in rows) for key in FIELDS}
                require(totals == data["expected_totals"], "Complete actual matrix denominator differs")
                summary["profiles"][profile]["totals"] = totals
        require(before == manifest_of(ROOT, endpoints.inputs) == manifest_of(source, endpoints.inputs), "Source or private snapshot changed")
        require(digest(local(MANIFEST)) == MANIFEST_SHA256, "Matrix manifest changed during proof")
        for row in data["shards"]:
            verify_compressed(row)
        require(tool_hashes == {name: digest(path) for name, path in tools.items()}, "Selected installed tools changed")
        verify_controls(controls)
        summary["status"] = "PASS_ALL46_ROWS_BOTH_PROFILES"
        summary["phase"] = "complete"
        summary["source_snapshot_controls_and_inputs_preserved"] = True
        (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps(summary), flush=True)
        return 0
    finally:
        resource.setrlimit(resource.RLIMIT_AS, old_limit)


if __name__ == "__main__":
    previous_signal = signal.signal(signal.SIGTERM, terminate)
    try:
        raise SystemExit(main())
    finally:
        signal.signal(signal.SIGTERM, previous_signal)
