"""Run the required expensive endpoint gate in a private source snapshot and Cargo target"""

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import sys
import tempfile

sys.dont_write_bytecode = True
from build import environment, executable, local_path, neutral_directory, require, seed_cache
from run_reference_oracles import run_command

ROOT = Path(__file__).resolve().parents[1]
FIXTURE_SHA256 = "c518210e49c129de8073c483b1f3f2d5959cf1c14656660822e2b50e0b639b8d"
EXPANDED_SHA256 = "380b031be3a82f0461e85866c4b1b69005cf3bc84208794344ab72fe66e3ed54"
EXPANDED_BYTES = 188971386
TEST = "world::organism_oracle_tests::workload_endpoint_tests::workload_endpoints"
CASES = ((0, 290), (32, 10437), (512, 170687), (12500, 4117647))
DENSE_FIXTURE_SHA256 = "59f2ad54a2f7aae1098dcec0ddca38595b38c3d6b8590c5330bef65ccb12fe42"
DENSE_EXPANDED_SHA256 = "b97b2734db5d60d25e5ccce2c41991c51b77f64eb91bbc027348d366cecb5265"
DENSE_EXPANDED_BYTES = 182874988
DENSE_TEST = "world::organism_oracle_tests::workload_endpoint_tests::dense_world_first_rollover_matches_original_complete_state"


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            result.update(chunk)
    return result.hexdigest()


def inputs(root):
    paths = [root / "Cargo.toml", root / "Cargo.lock"]
    for crate in ("core", "godot"):
        paths.append(root / "crates" / crate / "Cargo.toml")
        for folder in ("src", "tests"):
            directory = root / "crates" / crate / folder
            if directory.exists():
                paths.extend(path for path in directory.rglob("*") if path.is_file())
    paths.extend(path for path in (root / "tests").rglob("*") if path.is_file())
    for path in paths:
        local_path(root, path)
    return sorted(paths)


def snapshot(root, destination):
    paths = inputs(root)
    before = {str(path.relative_to(root)): digest(path) for path in paths}
    for path in paths:
        output = destination / path.relative_to(root)
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, output)
        require(digest(output) == before[str(path.relative_to(root))], "Source changed while copying: " + str(path))
    after = {str(path.relative_to(root)): digest(path) for path in inputs(root)}
    require(before == after, "Source tree changed during snapshot; preserve receipts and obtain a stable snapshot")
    return before


def expand_fixture(source, target, dense_rollover=False):
    compressed_hash, expanded_hash, expanded_bytes = (
        (DENSE_FIXTURE_SHA256, DENSE_EXPANDED_SHA256, DENSE_EXPANDED_BYTES) if dense_rollover
        else (FIXTURE_SHA256, EXPANDED_SHA256, EXPANDED_BYTES))
    require(digest(source) == compressed_hash, "Endpoint gzip hash differs from the frozen fixture")
    size = 0
    checksum = hashlib.sha256()
    with gzip.open(source, "rb") as encoded, target.open("xb") as output:
        while chunk := encoded.read(1024 * 1024):
            size += len(chunk)
            require(size <= expanded_bytes, "Endpoint expansion exceeds its frozen byte bound")
            checksum.update(chunk)
            output.write(chunk)
    require(size == expanded_bytes and checksum.hexdigest() == expanded_hash,
            "Endpoint expansion does not match its frozen canonical stream")
    return {"compressed_sha256": compressed_hash, "expanded_sha256": checksum.hexdigest(), "expanded_bytes": size}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for tool in ("cargo", "rustc", "rustdoc"):
        parser.add_argument("--" + tool)
    parser.add_argument("--cache-source", type=Path, default=ROOT / ".workflow-local/cargo-home")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--dense-rollover", action="store_true", help="Run the required fixed12500-attempt256-tick gate")
    parser.add_argument("--timeout", type=float, default=600, help="Per-command deadline, at most 3600 seconds")
    args = parser.parse_args(argv)
    test = DENSE_TEST if args.dense_rollover else TEST
    cases = ((12500, 4150610),) if args.dense_rollover else CASES
    require(sys.platform.startswith("linux"), "The bounded endpoint runner currently supports Linux")
    require(0 < args.timeout <= 3600, "Timeout must be positive and at most 3600 seconds")
    tools = {name: executable(getattr(args, name), name) for name in ("cargo", "rustc", "rustdoc")}
    runs = local_path(ROOT, ROOT / ".workflow-local/workload-runs")
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="endpoint-", dir=runs))
    print(json.dumps({"run_root": str(run)}), flush=True)
    receipts = []
    source = run / "source"
    manifest = snapshot(ROOT, source)
    (run / "source-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    cargo_home, target = run / "cargo-home", run / "target"
    cargo_home.mkdir()
    env = environment(os.environ, run / "profile", cargo_home, target, tools)
    count = seed_cache(ROOT, args.cache_source.resolve(), cargo_home, source / "Cargo.lock")
    expanded = run / ("workload_endpoint_12500_256.tsv" if args.dense_rollover else "workload_endpoints.tsv")
    filename = "workload_endpoint_12500_256.tsv.gz" if args.dense_rollover else "workload_endpoints.tsv.gz"
    package = expand_fixture(source / "crates/core/tests/fixtures" / filename, expanded, args.dense_rollover)
    env["BGR_ENDPOINT_FILE"] = str(expanded)
    env["BGR_ENDPOINT_DIAGNOSTIC_FILE"] = str(run / "first-mismatch.tsv")
    status = {"result": "FAIL", "required_gate": "workload endpoints", "normal_suite_pass": False,
              "source_manifest_sha256": digest(run / "source-manifest.json"), "fixture": package,
              "registry_packages": count, "cases": [], "profile": "release" if args.release else "debug",
              "ownership_projections": ["Root World ownership", "Serialized transient UI/corridor absence; actual sticky flag compared",
                                         "Independent owned array columns, Rectangles, histories, and Gene containers"],
              "sharing_checks": "Actual Rc actor/Genome/Gene identity, including current and retained tree objects",
              "max_build_address_space_bytes": (24 if args.release else 12) * 1024 ** 3,
              "max_test_address_space_bytes": 4 * 1024 ** 3,
              "limits": "Endpoint fidelity only; no final RNG, trajectory, observer-performance, UI, or hardware claim"}
    if args.dense_rollover:
        status.update(required_gate="dense256 complete trajectory and endpoint", expanded_fixture=str(expanded),
                      source_root=str(source), limits="Dense256 scalar/RNG and full endpoint fidelity only; no performance, Pi or intermediate actor-trajectory claim")
    (run / "summary.json").write_text(json.dumps(status, indent=2) + "\n")

    def terminate(signum, _frame):
        raise SystemExit(128 + signum)

    previous = signal.signal(signal.SIGTERM, terminate)
    # Bound each descendant's virtual address space; the source fixture remains an external streamed file
    original_limit = resource.getrlimit(resource.RLIMIT_AS)
    # Thin-LTO compilation needs more virtual address space than the debug build
    cap = (24 if args.release else 12) * 1024 ** 3
    resource.setrlimit(resource.RLIMIT_AS, (min(cap, original_limit[0]) if original_limit[0] != resource.RLIM_INFINITY else cap,
                                         original_limit[1]))
    try:
        with tempfile.TemporaryDirectory(prefix="bgr-endpoint-cargo-") as temporary:
            working = Path(temporary)
            neutral_directory(working)

            def command(arguments, command_env=env):
                return run_command(arguments, run, receipts, working, command_env, args.timeout)

            versions = {}
            for name, path in tools.items():
                versions[name] = command([str(path), "--version"]).strip()
                match = re.match(name + r" (\d+)\.(\d+)\.", versions[name])
                require(match and tuple(map(int, match.groups())) >= (1, 94), "Rust/Cargo 1.94 or newer is required")
            status["versions"] = versions
            arguments = [str(tools["cargo"]), "test", "--package", "biogenesis-core", "--lib", "--no-run",
                         "--offline", "--locked", "--manifest-path", str(source / "Cargo.toml"),
                         "--target-dir", str(target), "--message-format=json-render-diagnostics"]
            if args.release:
                arguments.append("--release")
            output = command(arguments)
            artifacts = []
            for line in output.splitlines():
                message = json.loads(line)
                if message.get("reason") == "compiler-artifact" and message.get("executable") and message["target"]["name"] == "biogenesis_core":
                    artifacts.append(Path(message["executable"]))
            require(len(artifacts) == 1, "Cargo must produce exactly one core test executable")
            binary = local_path(run, artifacts[0])
            require(binary.is_file(), "Core test executable is missing")
            status["test_binary_sha256"] = digest(binary)
            if args.dense_rollover:
                status["test_binary"] = str(binary)
            test_cap = 4 * 1024 ** 3
            resource.setrlimit(resource.RLIMIT_AS, (min(test_cap, original_limit[0]) if original_limit[0] != resource.RLIM_INFINITY else test_cap, original_limit[1]))
            listing = command([str(binary), "--list", "--ignored", "--exact", test])
            require(test + ": test" in listing and "1 test" in listing,
                    "The required ignored endpoint entry is absent; no normal-suite result substitutes for it")
            for attempts, rows in cases:
                case_env = {**env, "BGR_ENDPOINT_ATTEMPTS": str(attempts)}
                output = command([str(binary), "--ignored", "--exact", test, "--nocapture", "--test-threads=1"], case_env)
                require(re.search(r"test result: ok\. 1 passed; 0 failed;", output), "The required endpoint test did not execute once")
                marker = ("DENSE_ROLLOVER_ENDPOINT PASS attempts=12500 ticks=256 rows=4150610 "
                          if args.dense_rollover else f"WORKLOAD_ENDPOINT PASS attempts={attempts} rows={rows} ")
                require(output.count(marker) == 1, "Endpoint case marker or fixed ledger differs")
                if args.dense_rollover:
                    compact = "DENSE_ROLLOVER_TRAJECTORY PASS rows=15066 obligations=15677 runtime=15163 source_invariants=514 snapshots=257 calls=256"
                    require(output.count(compact) == 1, "Complete dense trajectory ledger differs")
                    require(output.count("boxed_aliases=6") == 1, "Required boxed history alias checks differ")
                    details = re.search(r"DENSE_ROLLOVER_ENDPOINT PASS attempts=12500 ticks=256 rows=4150610 derived_comparisons=4000244 mutable_aliases=(\d+) immutable_alias_metadata=(\d+) boxed_aliases=6 absent_fields=(\d+) absent_metadata=(\d+) input_settings=149", output)
                else:
                    details = re.search(r"WORKLOAD_ENDPOINT PASS attempts=\d+ rows=\d+ mutable_aliases=(\d+) immutable_alias_metadata=(\d+) absent_fields=(\d+) absent_metadata=(\d+) input_settings=149", output)
                require(details is not None, "The endpoint disposition ledger is missing")
                mutable, immutable, absent, metadata = map(int, details.groups())
                derived = rows - immutable - absent - metadata - 149
                required = {0: (132, 9, 0, 3, 6), 32: (9902, 1818, 313, 35, 38),
                            512: (164245, 30572, 5202, 544, 547),
                            12500: (3969738, 744499, 121367, 13195, 13198)}
                if args.dense_rollover:
                    required = {12500: (4000244, 749803, 123646, 13284, 13287)}
                require((derived, mutable, immutable, absent, metadata) == required[attempts],
                        "The complete endpoint disposition ledger differs")
                status["cases"].append({"attempts": attempts, "rows": rows, "result": "PASS",
                                        "receipt": len(receipts) - 1, "derived_comparisons": derived,
                                        "mutable_alias_rows": mutable, "immutable_identity_metadata": immutable,
                                        "absent_transient_fields": absent, "named_absence_metadata": metadata,
                                        "input_settings_and_boundaries": 149})
                if args.dense_rollover:
                    status["cases"][-1].update(ticks=256, compact_rows=15066, compact_obligations=15677,
                                              runtime_scalar_rng_comparisons=15163, source_Gaussian_invariants=514,
                                              snapshots=257, boxed_alias_rows=6)
                (run / "summary.json").write_text(json.dumps(status, indent=2) + "\n")
            require(manifest == {str(path.relative_to(ROOT)): digest(path) for path in inputs(ROOT)},
                    "Canonical source changed during the endpoint gate")
            require(manifest == {str(path.relative_to(source)): digest(path) for path in inputs(source)},
                    "The private source snapshot changed during execution")
            status["result"] = "PASS"
            status["source_and_snapshot_preserved"] = True
            (run / "summary.json").write_text(json.dumps(status, indent=2) + "\n")
            print(json.dumps(status), flush=True)
            return 0
    except ValueError as error:
        status["error"] = str(error)
        (run / "summary.json").write_text(json.dumps(status, indent=2) + "\n")
        print(str(error), file=sys.stderr)
        result = receipts[-1]["exit_code"] if receipts else 1
        return result if result and result > 0 else 1
    finally:
        resource.setrlimit(resource.RLIMIT_AS, original_limit)
        signal.signal(signal.SIGTERM, previous)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
