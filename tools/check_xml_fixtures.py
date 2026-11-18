"""Verify the frozen reader-only XML corpus without invoking a target parser"""

import argparse
import base64
from collections import Counter
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "crates/core/tests/fixtures"
PINS = {
    "xml.tsv": "ba5546e89bde74a6b277483e557d4ce3f1c4251e79b29937f44cc647adcffac7",
    "xml.README.md": "151dde9a4f1dc8409b3fe7cd7b6cdc7fbaad083e3ada9de07a3f766236842dab",
    "xml.provenance.json": "05e8d75d0ce47d198b962ccba1e065cb147f759681c8a75f0b30b75d9959f706",
}
SHAPES = {"C": 4, "U": 4, "F": 7, "K": 4, "O": 3, "X": 4,
          "G": 5, "N": 6, "A": 4, "T": 3, "R": 5, "V": 4, "E": 1}
COUNTS = {"C": 261, "U": 38367, "R": 848, "F": 448, "K": 587,
          "O": 587, "G": 7280, "N": 20640, "A": 260, "T": 184,
          "E": 261, "X": 144, "V": 3}
LEDGER = {"settings_schema_commands": 39476, "rng_observations": 848,
          "supplied_asset_checks": 188, "actual_method_outcomes": 587,
          "actual_parsed_model_observations": 17816, "source_diagnostic_metadata": 331,
          "actual_output_file_checks": 260, "supplied_writer_model_checks": 10364}
EXTERNAL = {"owned-external-dtd-default": "owned.dtd",
            "owned-external-parameter-entity": "parameter.dtd",
            "owned-external-general-entity": "owned-marker.txt"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def checked_path(base, relative):
    """Reject links and traversal before opening any fixture asset"""
    parts = PurePosixPath(relative).parts
    require(len(parts) == 3 and parts[0] == "cases" and re.fullmatch(r"[0-9]{4}", parts[1])
            and re.fullmatch(r"[a-z0-9_-]+\.(?:bgg|dtd|txt)", parts[2])
            and str(PurePosixPath(relative)) == relative and "\\" not in relative,
            "Invalid XML asset path: " + relative)
    require(stat.S_ISDIR(base.lstat().st_mode) and not base.is_symlink(), "Linked XML asset root")
    target = base
    for index, part in enumerate(parts):
        target = target / part
        mode = target.lstat().st_mode
        require(not stat.S_ISLNK(mode), "Linked XML asset path: " + relative)
        require(stat.S_ISREG(mode) if index == len(parts) - 1 else stat.S_ISDIR(mode),
                "Unexpected XML asset type: " + relative)
    return target


def verify_assets(base, inventory):
    require(len(inventory) <= 448, "XML asset inventory exceeds its fixed bound")
    for relative, expected in inventory.items():
        target = checked_path(base, relative)
        flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(target, flags)
        with os.fdopen(descriptor, "rb") as stream:
            info = os.fstat(stream.fileno())
            require(stat.S_ISREG(info.st_mode) and info.st_size == expected["bytes"],
                    "XML asset size differs: " + relative)
            digest = hashlib.sha256()
            remaining = expected["bytes"]
            while remaining:
                block = stream.read(min(65536, remaining))
                require(block, "Truncated XML asset: " + relative)
                remaining -= len(block)
                digest.update(block)
            require(not stream.read(1), "XML asset grew during read: " + relative)
        require(digest.hexdigest() == expected["sha256"], "XML asset hash differs: " + relative)
    actual = set()
    for directory, directories, files in os.walk(base, followlinks=False):
        for name in directories:
            require(not (Path(directory) / name).is_symlink(), "Linked XML asset directory")
        for name in files:
            path = Path(directory) / name
            relative = path.relative_to(base).as_posix()
            checked_path(base, relative)
            actual.add(relative)
    require(actual == set(inventory), "Missing or additional XML assets")


def settings_schema():
    result = {}
    with (FIXTURES / "organism_state.tsv").open(encoding="ascii") as stream:
        for line in stream:
            row = line.rstrip("\n").split("\t")
            if row[0] == "E":
                break
            if row[0] == "U" and row[2] != "java.lang.String":
                require(row[1] not in result, "Duplicate primitive settings schema")
                result[row[1]] = row[2]
    require(len(result) == 147, "Primitive settings schema differs")
    return result


def inspect_rows(text, schema):
    counts, ledger, inventory, names = Counter(), Counter(), {}, set()
    external, methods, source_outcomes = [], Counter(), Counter()
    active = None
    for line_number, line in enumerate(text.splitlines(), 1):
        row = line.split("\t")
        tag = row[0]
        require(tag in SHAPES and len(row) == SHAPES[tag], f"Malformed XML row {line_number}")
        counts[tag] += 1
        if tag == "C":
            require(active is None and row[1] not in names and row[2] in ("parse", "write")
                    and row[3] == "827364", "Invalid XML case boundary")
            names.add(row[1])
            active = {"name": row[1], "settings": {}, "commands": {}, "outcomes": {}, "rng": {}}
        require(active is not None, "XML row outside a case")
        if tag in ("G", "N", "A"):
            ledger["supplied_writer_model_checks" if row[1] == "input"
                   else "actual_parsed_model_observations"] += 1
        elif tag == "F":
            ledger["supplied_asset_checks" if row[2] in ("supplied", "owned")
                   else "actual_output_file_checks"] += 1
        elif tag == "O":
            ledger["actual_method_outcomes"] += 1
        elif tag == "R":
            ledger["rng_observations"] += 1
        elif tag in ("X", "T", "V"):
            ledger["source_diagnostic_metadata"] += 1
        else:
            ledger["settings_schema_commands"] += 1
        if tag == "U":
            require(row[1] in schema and row[2] == schema[row[1]]
                    and row[1] not in active["settings"], "XML U schema differs")
            active["settings"][row[1]] = row[3]
            if row[2] == "double":
                require(re.fullmatch(r"[0-9a-f]{16}", row[3]), "Invalid U double")
            elif row[2] == "boolean":
                require(row[3] in ("true", "false"), "Invalid U Boolean")
            else:
                require(-(1 << 31) <= int(row[3]) < (1 << 31), "Invalid U integer")
        elif tag == "F":
            require(row[3] not in inventory and row[2] in ("supplied", "owned", "written", "partial")
                    and re.fullmatch(r"[0-9a-f]{64}", row[4]), "Invalid F asset record")
            size = int(row[5])
            require(0 <= size <= 104762, "XML fixture exceeds frozen asset bound")
            inventory[row[3]] = {"role": row[2], "sha256": row[4], "bytes": size, "encoding": row[6]}
        elif tag == "K":
            require(row[1] == f"step-{len(active['commands'])}" and row[2] in ("parse", "write"),
                    "XML command order differs")
            active["commands"][row[1]] = row[2:]
            methods[row[2]] += 1
        elif tag == "O":
            require(row[1] in active["commands"] and row[1] not in active["outcomes"]
                    and row[2] in ("success", "failure"), "Invalid XML outcome")
            active["outcomes"][row[1]] = row[2]
            source_outcomes[row[2]] += 1
        elif tag == "R":
            require(row[1] not in active["rng"] and re.fullmatch(r"[0-9a-f]{16}", row[4]),
                    "Invalid copied-source RNG observation")
            require(-(1 << 63) <= int(row[2]) < (1 << 63) and 0 <= int(row[3]) < 10000,
                    "Invalid copied-source RNG scalar")
            active["rng"][row[1]] = tuple(row[2:])
        elif tag in ("X", "T"):
            base64.b64decode(row[-1], validate=True).decode("utf-8")
        elif tag == "V":
            require(active["name"] in EXTERNAL and row[1] == "step-0"
                    and row[2] == "owned_external_resolution" and row[3] == EXTERNAL[active["name"]],
                    "Unspecified external-resource correction")
            external.append((active["name"], row[1]))
        elif tag == "E":
            require(active["settings"].keys() == schema.keys(), "Missing XML settings")
            require(active["commands"].keys() == active["outcomes"].keys(), "Missing XML method outcome")
            require(active["rng"].keys() == {"initial", *active["commands"]}
                    and len(set(active["rng"].values())) == 1, "Copied source RNG changed")
            active = None
    require(active is None and counts == Counter(COUNTS) and ledger == Counter(LEDGER),
            "Frozen XML row ledger differs")
    require(methods == {"parse": 327, "write": 260} and source_outcomes == {"success": 443, "failure": 144},
            "Frozen XML method ledger differs")
    require(set(external) == {(name, "step-0") for name in EXTERNAL} and len(external) == 3,
            "External-resource correction scope differs")
    return inventory, counts, ledger


def check():
    directory = ROOT
    for part in ("crates", "core", "tests", "fixtures"):
        directory /= part
        require(not directory.is_symlink() and stat.S_ISDIR(directory.lstat().st_mode),
                "Linked XML fixture parent directory")
    for name, digest in PINS.items():
        path = FIXTURES / name
        require(not path.is_symlink() and path.stat().st_size <= 4 * 1024 * 1024,
                "Invalid XML metadata artifact")
        require(hashlib.sha256(path.read_bytes()).hexdigest() == digest, "XML artifact hash differs: " + name)
    inventory, counts, ledger = inspect_rows((FIXTURES / "xml.tsv").read_text(encoding="ascii"), settings_schema())
    expected = json.loads((FIXTURES / "xml.provenance.json").read_text())["fixture"]["case_file_inventory"]
    require(inventory == expected and len(inventory) == 448, "XML asset inventory differs")
    verify_assets(FIXTURES / "xml", inventory)
    return {"result": "PASS", "scope": "reader-only XML asset preflight; no target parser execution",
            "cases": 261, "rows": sum(counts.values()), "assets": 448,
            "source_ledger": dict(ledger), "reader_parse_calls": 327, "security_rejections": 3}


class AssetBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="biogenesis-xml-fixture-")
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.relative = "cases/0000/input.bgg"
        path = self.base / self.relative
        path.parent.mkdir(parents=True)
        path.write_bytes(b"safe")
        self.inventory = {self.relative: {"bytes": 4, "sha256": hashlib.sha256(b"safe").hexdigest()}}

    def test_valid_inventory_and_corrupt_same_size_payload(self):
        verify_assets(self.base, self.inventory)
        (self.base / self.relative).write_bytes(b"evil")
        with self.assertRaisesRegex(ValueError, "hash differs"):
            verify_assets(self.base, self.inventory)

    def test_missing_extra_and_size_changed_assets(self):
        path = self.base / self.relative
        path.write_bytes(b"longer")
        with self.assertRaisesRegex(ValueError, "size differs"):
            verify_assets(self.base, self.inventory)
        path.write_bytes(b"safe")
        (path.parent / "extra.bgg").write_bytes(b"extra")
        with self.assertRaisesRegex(ValueError, "additional"):
            verify_assets(self.base, self.inventory)
        path.unlink()
        with self.assertRaises(FileNotFoundError):
            verify_assets(self.base, self.inventory)

    def test_traversal_absolute_and_linked_assets(self):
        for value in ("../input.bgg", "/cases/0000/input.bgg", "cases/0000/../input.bgg", "cases\\0000\\input.bgg"):
            with self.subTest(path=value), self.assertRaises(ValueError):
                checked_path(self.base, value)
        path = self.base / self.relative
        path.unlink()
        path.symlink_to(__file__)
        with self.assertRaisesRegex(ValueError, "Linked"):
            verify_assets(self.base, self.inventory)
        path.unlink()
        path.parent.rmdir()
        path.parent.symlink_to(self.base / "cases", target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "Linked"):
            verify_assets(self.base, self.inventory)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true", help="Check disposable asset-boundary failures")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(AssetBoundaryTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    print(json.dumps(check(), sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError) as error:
        print("XML fixture preflight failed: " + str(error), file=sys.stderr)
        raise SystemExit(1)
