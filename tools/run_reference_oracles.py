"""Compile pinned Java inputs and compare bounded source/JAR reference scenarios."""

import argparse
from functools import partial
import hashlib
import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import signal
import subprocess
import tempfile
import time
import zipfile


ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT.parent / "Colormod"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run_command(arguments, run, receipts, directory=None, env=None, timeout=40):
    directory = run if directory is None else directory
    begin = time.monotonic()
    process = subprocess.Popen(arguments, cwd=directory, env=env, text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    process_group = os.getpgid(process.pid)
    timed_out = False
    interruption = None

    def signal_group(signum):
        try:
            os.killpg(process_group, signum)
        except ProcessLookupError:
            pass

    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
    except BaseException as error:
        interruption = error
    finally:
        signal_group(signal.SIGTERM)
        try:
            stdout, stderr = process.communicate(timeout=3)
        except subprocess.TimeoutExpired:
            signal_group(signal.SIGKILL)
            stdout, stderr = process.communicate(timeout=3)
        finally:
            # An exited wrapper can leave redirected descendants outside communicate's wait
            signal_group(signal.SIGKILL)
    receipt = {"argv": list(map(str, arguments)), "cwd": str(directory), "pid": process.pid,
               "process_group": process_group, "exit_code": process.returncode,
               "timed_out": timed_out, "interrupted": interruption is not None,
               "elapsed_seconds": time.monotonic() - begin, "stdout": stdout, "stderr": stderr}
    receipts.append(receipt)
    (run / "commands.json").write_text(json.dumps(receipts, indent=2) + "\n")
    with (run / "progress.log").open("a") as stream:
        stream.write(time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()) + " "
                     + json.dumps({"phase": directory.name, "exit_code": process.returncode,
                                   "interrupted": interruption is not None, "timed_out": timed_out}) + "\n")
    if interruption is not None:
        raise interruption
    require(not timed_out and process.returncode == 0, "Reference command failed; inspect " + str(run / "commands.json"))
    print(json.dumps({"phase": directory.name, "command": Path(arguments[0]).name, "exit_code": 0}), flush=True)
    return stdout


def main(gui):
    manifest = json.loads((ROOT / "docs/evidence/reference-manifest.json").read_text())["files"]
    for name, item in manifest.items():
        require(digest(ROOT.parent / name) == item["sha256"], "Reference drift: " + name)
    run = Path(tempfile.mkdtemp(prefix="reference-run-", dir=ROOT / ".workflow-local"))
    (run / "progress.log").write_text(time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()) + " started\n")
    print(json.dumps({"run_root": str(run), "monitor": shlex.join(["tail", "-f", str(run / "progress.log")])}), flush=True)
    receipts = []

    def terminate(signum, _frame):
        raise SystemExit(128 + signum)

    signal.signal(signal.SIGTERM, terminate)

    command = partial(run_command, run=run, receipts=receipts)

    jar = REFERENCE / "ColorMod_2_01/biogenesis.jar"
    jts = run / "jts-core-1.15.0.jar"
    with zipfile.ZipFile(jar) as archive:
        jts.write_bytes(archive.read("jts-core-1.15.0.jar"))
    classes = run / "source-classes"
    classes.mkdir()
    sources = sorted((REFERENCE / "src").rglob("*.java"))
    require(len(sources) == 32, "Unexpected source inventory")
    command(["javac", "-encoding", "UTF-8", "-cp", str(jts), "-d", str(classes), *map(str, sources)])
    for name in ["images", "messages"]:
        shutil.copytree(REFERENCE / "src/biogenesis" / name, classes / "biogenesis" / name)
    helpers = run / "helpers"
    helpers.mkdir()
    harness_sources = [ROOT / "tools/reference/ReferenceOracle.java"]
    if gui:
        harness_sources += [ROOT / "tools/reference/ReferenceGui.java", ROOT / "tools/reference/ReferenceWorkload.java"]
    command(["javac", "-encoding", "UTF-8", "-cp", os.pathsep.join([str(classes), str(jts)]),
             "-d", str(helpers), *map(str, harness_sources)])

    def invoke(name, backend, entry, extra, headless):
        directory = run / name
        directory.mkdir()
        for folder in ["home", "tmp", "prefs-user", "prefs-system"]:
            (directory / folder).mkdir()
        argv = ["java", "-Xms16m", "-Xmx512m", "-Djava.awt.headless=" + str(headless).lower(),
                "-Duser.home=" + str(directory / "home"), "-Djava.io.tmpdir=" + str(directory / "tmp"),
                "-Djava.util.prefs.userRoot=" + str(directory / "prefs-user"),
                "-Djava.util.prefs.systemRoot=" + str(directory / "prefs-system"),
                "-cp", os.pathsep.join([str(helpers), str(backend), str(jts)]), entry,
                *extra, str(directory / "fixtures")]
        env = os.environ.copy()
        if not headless:
            env["BGR_ISOLATED_XVFB"] = "1"
            env.pop("WAYLAND_DISPLAY", None)
            argv = ["xvfb-run", "-a", "-n", "120", "-e", str(directory / "xvfb.log"),
                    "-s", "-screen 0 1280x960x24 -nolisten tcp", *argv]
        stdout = command(argv, directory=directory, env=env, timeout=45)
        (directory / "stdout.txt").write_text(stdout)
        return directory, stdout

    core = []
    for name, backend in [("core-source-a", classes), ("core-source-b", classes), ("core-jar", jar)]:
        directory, stdout = invoke(name, backend, "biogenesis.ReferenceOracle", [], True)
        rows = [json.loads(line) for line in stdout.splitlines()]
        require(len(rows) == 30 and rows[-1]["observations"]["result"] == "PASS", "Incomplete core oracle output")
        core.append((directory, stdout))
    require(all(output == core[0][1] for _, output in core), "Source/JAR core observations differ")
    fixtures = {path.name: digest(path) for path in (core[0][0] / "fixtures").iterdir()}
    require(len(fixtures) == 6, "Incomplete serialization/XML fixture set")
    for directory, _ in core[1:]:
        require({path.name: digest(path) for path in (directory / "fixtures").iterdir()} == fixtures,
                "Source/JAR serialized fixtures differ")
    result = {"result": "PASS", "core_assertions": rows[-1]["observations"]["checks"], "core_rows": len(rows),
              "core_stdout_sha256": hashlib.sha256(core[0][1].encode()).hexdigest(),
              "reference_sha256": digest(jar), "jts_sha256": digest(jts), "fixtures": fixtures,
              "system": {"platform": platform.platform(), "machine": platform.machine()}, "gui": "not_run"}
    if gui:
        captures = []
        for name, backend in [("gui-source", classes), ("gui-jar", jar)]:
            directory, stdout = invoke(name, backend, "biogenesis.ReferenceGui", [], False)
            rows = [json.loads(line) for line in stdout.splitlines() if line.startswith("{")]
            require(rows[-1]["event"] == "quit-without-save-requested", "Incomplete real-control journey")
            require(len(list((directory / "fixtures").glob("*.png"))) == 6, "Incomplete rendered journey")
            captures.append(directory)
        require(digest(captures[0] / "fixtures/startup-world.bgw") == digest(captures[1] / "fixtures/startup-world.bgw"),
                "Seeded source/JAR genesis differs")
        workloads = []
        for count in [0, 32, 512, 12500]:
            pair = []
            for backend_name, backend in [("source", classes), ("jar", jar)]:
                _, stdout = invoke(f"workload-{backend_name}-{count}", backend, "biogenesis.ReferenceWorkload", [str(count)], False)
                observation = json.loads(stdout)
                pair.append(observation)
                workloads.append({"backend": backend_name, **observation})
            require(pair[0]["snapshot_sha256"] == pair[1]["snapshot_sha256"], "World-frame source/JAR divergence")
        result.update(gui="PASS", workloads=workloads, startup_sha256=digest(captures[0] / "fixtures/startup-world.bgw"))
    for name, item in manifest.items():
        require(digest(ROOT.parent / name) == item["sha256"], "Reference changed during verification: " + name)
    (run / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"run_root": str(run), **result}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gui", action="store_true", help="Also run real controls and world workloads on owned Xvfb displays")
    main(parser.parse_args().gui)
