"""Check the real JDK parser against source text that defeats regex parsing."""

from pathlib import Path
import subprocess
import tempfile
import unittest

from reference_catalog import parse_sources


class ReferenceCatalogTests(unittest.TestCase):
    def test_actual_declarations_and_decisions(self):
        with tempfile.TemporaryDirectory(prefix="bgr-parser-fixture-") as directory:
            root = Path(directory)
            path = root / "Fixture.java"
            path.write_text('''class Fixture {
                String message = "if (pretend) { false; }";
                // void false_method() { if (false) {} }
                int value(int x) {
                    if (x > 0) return x;
                    return x < 0 ? -x : 0;
                }
                void run() {
                    Runnable worker = new Runnable() { public void run() { while (false) {} } };
                    switch (message) { case "a": break; default: break; }
                }
            }''')
            records = parse_sources([path], root)
            self.assertEqual([r["name"] for r in records if r["kind"] == "method"],
                             ["value(int)", "run()", "run()"])
            self.assertEqual(len([r for r in records if r["kind"] == "if"]), 1)
            self.assertEqual(len([r for r in records if r["kind"] == "conditional"]), 1)
            self.assertEqual(len([r for r in records if r["kind"] == "case"]), 2)
            self.assertEqual([r["start_line"] for r in records if r["kind"] == "while"], [9])
            self.assertTrue(any(r["class"] == "Fixture.anonymous" for r in records))
            self.assertTrue(all(len(r["source_sha256"]) == 64 for r in records))
            method = next(r for r in records if r["kind"] == "method" and r["name"] == "value(int)")
            self.assertEqual((method["start_line"], method["end_line"]), (4, 7))
            self.assertEqual(method["source_sha256"],
                             "e84931568f4f06337fd63f29393da97d6a11311332f183c8b462ccbe10ae78dd")
            self.assertFalse(list(root.glob("*.class")))

    def test_reject_broken_source(self):
        with tempfile.TemporaryDirectory(prefix="bgr-parser-invalid-") as directory:
            root = Path(directory)
            path = root / "Broken.java"
            path.write_text("class Broken { void broken( }")
            with self.assertRaises(subprocess.CalledProcessError):
                parse_sources([path], root)


if __name__ == "__main__":
    unittest.main()
