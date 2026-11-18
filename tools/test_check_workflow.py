"""Exercise rejection paths without executing imported tracker callbacks."""

from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from check_workflow import check_records, check_safety, STATUSES


class WorkflowChecks(unittest.TestCase):
    def setUp(self):
        self.mapping = {"remote": None, "entries": [
            {"task_id": "TASK-1", "work_key": "key-1", "definition_source": "backlog_task", "requirements": ["BGR-001"]},
            {"task_id": "TASK-2", "work_key": "key-2", "definition_source": "backlog_task", "requirements": ["BGR-002"]},
        ]}
        self.tasks = {}
        for number in [1, 2]:
            task_id = f"TASK-{number}"
            self.tasks[task_id] = {
                "id": task_id, "description": f"Work key: key-{number}\nDefinition authority: backlog_task\n",
                "status": "Backlog", "priority": "p1", "milestone": "m-0", "dependencies": [],
                "acceptanceCriteriaCount": 1, "acceptanceCriteriaCompleted": 0,
            }

    def check(self, mapping=None, tasks=None):
        check_records(mapping or self.mapping, tasks or self.tasks, {"BGR-001", "BGR-002"}, {"m-0"})

    def test_valid_mapping(self):
        self.check()

    def test_reject_lost_requirement_and_duplicate_identity(self):
        for field, value, message in [
            ("requirements", ["BGR-001"], "Uncovered requirement"),
            ("task_id", "TASK-1", "Duplicate task mapping"),
            ("work_key", "key-1", "Duplicate work key"),
        ]:
            with self.subTest(field=field):
                mapping = deepcopy(self.mapping)
                mapping["entries"][1][field] = value
                with self.assertRaisesRegex(ValueError, message):
                    self.check(mapping=mapping)

    def test_reject_missing_dependency_and_cycle(self):
        self.tasks["TASK-1"]["dependencies"] = ["TASK-99"]
        with self.assertRaisesRegex(ValueError, "Missing dependency"):
            self.check()
        self.tasks["TASK-1"]["dependencies"] = ["TASK-2"]
        self.tasks["TASK-2"]["dependencies"] = ["TASK-1"]
        with self.assertRaisesRegex(ValueError, "Dependency cycle"):
            self.check()

    def test_cancelled_dependency_remains_blocking(self):
        self.tasks["TASK-1"]["status"] = "Cancelled"
        self.tasks["TASK-2"].update(status="Ready", dependencies=["TASK-1"])
        with self.assertRaisesRegex(ValueError, "Unsatisfied dependency"):
            self.check()
        self.tasks["TASK-1"].update(status="Complete", acceptanceCriteriaCompleted=1, finalSummary="Fixture evidence")
        self.check()

    def test_reject_false_completion_and_unclaimed_work(self):
        self.tasks["TASK-1"].update(status="Complete")
        with self.assertRaisesRegex(ValueError, "Unchecked acceptance"):
            self.check()
        self.tasks["TASK-1"].update(status="Implementing", assignees=[], implementationPlan=None)
        with self.assertRaisesRegex(ValueError, "Unclaimed active work"):
            self.check()

    def test_config_and_callback_rejection(self):
        with tempfile.TemporaryDirectory(prefix="bgr-workflow-check-") as directory:
            root = Path(directory)
            (root / "backlog/tasks").mkdir(parents=True)
            config = root / "backlog/config.yml"
            safe = '\n'.join([
                'filesystem_only: true', 'remote_operations: false', 'check_active_branches: false',
                'auto_commit: false', 'bypass_git_hooks: false', 'auto_open_browser: false',
                'default_status: "Backlog"', 'statuses: ' + json.dumps(STATUSES),
                'priorities: ["P0", "P1", "P2"]', '',
            ])
            config.write_text(safe)
            check_safety(root)
            config.write_text(safe.replace('remote_operations: false', 'remote_operations: true'))
            with self.assertRaisesRegex(ValueError, "Unsafe config"):
                check_safety(root)
            config.write_text(safe + 'onStatusChange: example-command\n')
            with self.assertRaisesRegex(ValueError, "Status callback"):
                check_safety(root)
            config.write_text(safe)
            (root / "backlog/tasks/fixture.md").write_text('---\nid: TASK-1\nonStatusChange: example-command\n---\n')
            with self.assertRaisesRegex(ValueError, "Status callback"):
                check_safety(root)

    def test_path_shadow_cannot_replace_the_adopted_cli(self):
        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory(prefix="bgr-cli-shadow-") as directory:
            shadow = Path(directory) / "backlog"
            marker = Path(directory) / "unexpected-cli"
            descriptor = os.open(shadow, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o700)
            with os.fdopen(descriptor, "w") as stream:
                stream.write("#!/usr/bin/python3\nfrom pathlib import Path\n"
                             + f"Path({str(marker)!r}).write_text('executed')\nprint('99.0.0')\n")
            env = os.environ.copy()
            env.pop("BACKLOG_CWD", None)
            env["PATH"] = directory + os.pathsep + env["PATH"]
            result = subprocess.run([sys.executable, str(root / "tools/check_workflow.py")], cwd=root,
                                    env=env, text=True, capture_output=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(marker.exists())


if __name__ == "__main__":
    unittest.main()
