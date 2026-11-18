"""Build a source coverage index using the installed JDK parser."""

from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT.parent / "Colormod/src"
CATALOG = ROOT / "docs/parity/source-catalog.json"
ORACLES = {
    "genetics": "Compare seeded Java/Rust construction, inheritance, mutation, full fields, and RNG draw order",
    "world": "Compare seeded world and organism state before/after each operation, including ordered events and gas balances",
    "colors": "Compare directed color/state contacts and per-frame updates with full organism state, energies, gases, and RNG traces",
    "interface": "Exercise each real control; compare visible state, simulation changes, validation, cancellation, and failure feedback",
    "configuration": "Compare defaults, boundaries, preference restart, invalid settings, locale fallbacks, and effective live values",
    "statistics": "Compare organism/world records, ordered histories, extinction/reset, and rendered inspection values",
    "persistence": "Compare legacy fixtures and fresh-process round trips; exercise malformed inputs, write failures, backup recovery, and images",
    "network": "Compare protocol bytes and state transitions with isolated legacy/Rust peers, failure paths, cancellation, and transfers",
    "foundation": "Compare helper results and boundaries against Java; verify actual entry point and reference-derived effects",
}


def owners(record):
    name = Path(record["file"]).stem
    method = record["method"].split("(")[0]
    if name in {"Gene", "GeneticCode"}:
        return ["TASK-6"], "genetics"
    if name == "Organism":
        if method in {"create", "symmetric", "randomCreate"}:
            return ["TASK-6"], "genetics"
        if method in {"inherit", "inheritTransformation"}:
            return ["TASK-6", "TASK-8"], "genetics"
        color_methods = {"segmentsCheckPlant", "segmentsRestoreEffects", "segmentsCreateEffects", "transform",
                         "decay", "blueSpore", "cyanSpore", "hatch", "die", "survive", "inhibited", "frozen",
                         "paralyzed", "neutralized", "virusneutralized", "lavenderinhibition", "lavendershield",
                         "weaklavendershield", "rubbingFramesEffects", "plantsUpdateEffects", "darkfireRestoreEffects",
                         "getTypeColor", "standstill", "standochre"}
        if (method.startswith(("touch", "pretouch", "infectedBy", "segments", "leafExtra", "jadeExtra"))
                or method in color_methods) and method != "touchMove":
            return ["TASK-8"], "colors"
        if method in {"draw", "setColor", "getImage"}:
            return ["TASK-9", "TASK-12"], "interface"
        return ["TASK-7"], "world"
    if name in {"World", "Vector2D", "ExLine2DDouble"}:
        return ["TASK-7"], "world"
    if name in {"WorldStatistics", "StatisticsWindow", "InfoToolbar", "GeneticCodePanel"}:
        return ["TASK-11"], "statistics"
    if name in {"NetConfigWindow", "NetConnectionsWindow", "NetSend", "NetServerThread", "Connection", "InCorridor", "OutCorridor", "Corridor"}:
        return ["TASK-13"], "network"
    if name in {"BioXMLParser", "BioFileFilter"}:
        return ["TASK-12"], "persistence"
    if name in {"ParamDialog", "Messages", "CheckVersionThread"}:
        return ["TASK-10"], "configuration"
    if name == "ColorComboBox":
        return ["TASK-8", "TASK-9"], "colors"
    if name == "Utils":
        if record["name"].startswith("Color") or method == "getColorMessage":
            return ["TASK-8", "TASK-10"], "colors"
        if method in {"randomSign", "randomMutation"}:
            return ["TASK-6"], "genetics"
        return ["TASK-10"], "configuration"
    if name in {"VisibleWorld", "LabWindow", "MainWindow", "MainApplet", "StdAction", "BareBonesBrowserLaunch", "Cleaner"}:
        if method in {"saveWorld", "readWorld", "loadWorld", "saveObject", "readObject", "saveGeneticCode", "readGeneticCode"}:
            return ["TASK-9", "TASK-12"], "persistence"
        return ["TASK-9"], "interface"
    raise ValueError("Unmapped source file: " + name)


def parse_sources(paths, source_root=SOURCE):
    with tempfile.TemporaryDirectory(prefix="bgr-source-catalog-") as directory:
        subprocess.run(["javac", "-d", directory, str(ROOT / "tools/SourceCatalog.java")],
                       check=True, capture_output=True, text=True, timeout=30)
        result = subprocess.run(["java", "-cp", directory, "SourceCatalog", str(source_root), *map(str, paths)],
                                check=True, capture_output=True, text=True, timeout=30)
        return [json.loads(line) for line in result.stdout.splitlines()]


def build():
    manifest = json.loads((ROOT / "docs/evidence/reference-manifest.json").read_text())["files"]
    for name, expected in manifest.items():
        path = ROOT.parent / name
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected["sha256"]:
            raise ValueError("Reference drift: " + name)
    paths = sorted(SOURCE.rglob("*.java"))
    if len(paths) != 32:
        raise ValueError("Reference Java file inventory changed")
    records = parse_sources(paths)
    ordinals = Counter()
    for record in records:
        prefix = "JAVA-" + Path(record["file"]).stem + "-" + record["kind"].upper() + "-" + str(record["start_line"])
        ordinals[prefix] += 1
        record["id"] = prefix + "-" + str(ordinals[prefix])
        record["owners"], record["oracle_id"] = owners(record)
    colors = []
    text = (SOURCE / "biogenesis/ColorComboBox.java").read_text()
    labels = re.findall(r'Messages.getString\("T_([A-Z0-9_]+)"\)', text.split("public ColorComboBox", 1)[0])
    for match in re.finditer(r"case (\d+): return ((?:Color|Utils)\.[A-Za-z_0-9]+);", text):
        index = int(match[1])
        colors.append({"index": index, "name": labels[index] if index < len(labels) else match[2].split(".")[1].removeprefix("Color"),
                       "expression": match[2], "selectable": index < len(labels),
                       "source_line": text[:match.start()].count("\n") + 1, "owner": "TASK-8", "oracle_id": "colors"})
    if len(labels) != 46 or [row["index"] for row in colors] != list(range(63)):
        raise ValueError("Unexpected color selector schema")
    default = re.search(r"default: return Utils\.ColorBROWN;", text)
    if default is None:
        raise ValueError("Color selector fallback changed")
    colors.append({"index": None, "name": "BROWN", "expression": "Utils.ColorBROWN", "selectable": False,
                   "source_line": text[:default.start()].count("\n") + 1, "owner": "TASK-8", "oracle_id": "colors"})
    return {"schema_version": 1, "reference_manifest": "../evidence/reference-manifest.json",
            "scope": "Source coverage obligations; no product implementation or behavior acceptance is claimed",
            "evidence_gap": "Every oracle remains unexercised until the linked reference and Rust task evidence proves it",
            "oracles": ORACLES,
            "files": [{"path": str(path.relative_to(SOURCE)), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                      for path in paths],
            "counts": dict(sorted(Counter(record["kind"] for record in records).items())),
            "colors": colors, "records": records}


if __name__ == "__main__":
    if sys.argv[1:] not in [["--write"], ["--check"]]:
        raise SystemExit("Usage: python3 tools/reference_catalog.py --write|--check")
    content = json.dumps(build(), indent=2, ensure_ascii=True) + "\n"
    if sys.argv[1] == "--write":
        CATALOG.write_text(content)
    elif not CATALOG.exists() or CATALOG.read_text() != content:
        raise SystemExit("Source catalog drift; inspect the reference before regenerating")
    print(json.dumps({"result": "PASS", "mode": sys.argv[1], "records": len(json.loads(content)["records"])}))
