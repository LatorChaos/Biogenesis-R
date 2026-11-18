"""Check local tracker safety, requirement coverage, identities, and readiness."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys


STATUSES = ["Backlog", "Ready", "Implementing", "Reviewing", "Verifying", "Cancelled", "Complete"]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def check_safety(root):
    config = (root / "backlog/config.yml").read_text()
    expected = {
        "filesystem_only": "true",
        "remote_operations": "false",
        "check_active_branches": "false",
        "auto_commit": "false",
        "bypass_git_hooks": "false",
        "auto_open_browser": "false",
    }
    for key, value in expected.items():
        require(re.findall(rf"^{key}:\s*(\S+)\s*$", config, re.M) == [value], f"Unsafe config: {key}")
    for key, value in [("statuses", STATUSES), ("priorities", ["P0", "P1", "P2"]), ("default_status", "Backlog")]:
        matches = re.findall(rf"^{key}:\s*(.+)$", config, re.M)
        require(len(matches) == 1 and json.loads(matches[0]) == value, f"Unexpected config: {key}")
    for path in [root / "backlog/config.yml", *sorted((root / "backlog").rglob("*.md"))]:
        body = path.read_text()
        if path.suffix == ".md":
            body = body.split("---", 2)[1] if body.startswith("---\n") else ""
        require(not re.search(r"^[ \t]*[\"']?(?:onStatusChange|on_status_change)[\"']?\s*:", body, re.M),
                f"Status callback requires reconciliation: {path}")


def check_records(mapping, tasks, requirements, milestones):
    entries = mapping["entries"]
    ids = [entry["task_id"] for entry in entries]
    keys = [entry["work_key"] for entry in entries]
    require(len(ids) == len(set(ids)), "Duplicate task mapping")
    require(len(keys) == len(set(keys)), "Duplicate work key")
    require(set(ids) == set(tasks), "Unmapped or missing task")
    require(mapping["remote"] is None, "Remote authority requires explicit reconciliation")
    covered = set()
    for entry in entries:
        task = tasks[entry["task_id"]]
        require(entry["definition_source"] == "backlog_task", "Unexpected definition authority")
        require("Work key: " + entry["work_key"] + "\n" in (task["description"] or ""), "Work key mismatch")
        require("Definition authority: backlog_task" in task["description"], "Task authority mismatch")
        require(set(entry["requirements"]) <= requirements, "Unknown requirement")
        covered.update(entry["requirements"])
        require(task["status"] in STATUSES, "Unknown lifecycle state")
        require(task["priority"] in {"p0", "p1", "p2"}, "Unknown priority")
        require(task["milestone"] in milestones, "Missing milestone")
        require(task["acceptanceCriteriaCount"] > 0, "Missing acceptance criteria")
        deps = task["dependencies"]
        require(set(deps) <= set(tasks), "Missing dependency")
        require(task["id"] not in deps, "Self dependency")
        if task["status"] in {"Ready", "Implementing", "Reviewing", "Verifying", "Complete"}:
            require(all(tasks[dep]["status"] == "Complete" for dep in deps), "Unsatisfied dependency")
        if task["status"] in {"Implementing", "Reviewing", "Verifying"}:
            require(task["assignees"] and task["implementationPlan"], "Unclaimed active work")
            require("Workspace:" in (task["implementationNotes"] or ""), "Missing claim workspace")
        if task["status"] == "Complete":
            require(task["acceptanceCriteriaCompleted"] == task["acceptanceCriteriaCount"], "Unchecked acceptance")
            require(task["finalSummary"], "Missing completion evidence summary")
    require(covered == requirements, "Uncovered requirement")
    visited, active = set(), set()

    def visit(task_id):
        require(task_id not in active, "Dependency cycle")
        if task_id in visited:
            return
        active.add(task_id)
        for dep in tasks[task_id]["dependencies"]:
            visit(dep)
        active.remove(task_id)
        visited.add(task_id)

    for task_id in tasks:
        visit(task_id)


def check(root):
    require("BACKLOG_CWD" not in os.environ, "Unset BACKLOG_CWD before verification")
    require(not (root / "backlog.config.yml").exists(), "Unexpected root config override")
    check_safety(root)

    def cli(*args):
        return subprocess.run(["/usr/bin/backlog", *args], cwd=root, check=True, text=True,
                              capture_output=True, timeout=30).stdout

    require(cli("--version").strip() == "1.53.0", "Reverify workflow after a Backlog.md version change")
    listing = json.loads(cli("task", "list", "--json"))
    require(listing["schemaVersion"] == 1, "Unsupported CLI schema")
    tasks = {item["id"]: json.loads(cli("task", "view", item["id"], "--json"))["task"]
             for item in listing["tasks"]}
    stored = []
    for path in (root / "backlog").rglob("*.md"):
        stored.extend(re.findall(r"^id: (TASK-[0-9]+)\s*$", path.read_text(), re.M))
    require(len(stored) == len(set(stored)) and set(stored) == set(tasks), "Hidden or duplicate stored tasks")
    milestones = set()
    for path in (root / "backlog/milestones").glob("*.md"):
        milestones.update(re.findall(r"^id: (m-[0-9]+)\s*$", path.read_text(), re.M))
    requirements = set(re.findall(r"^\| ((?:BGR|SET)-[0-9]{3}) \|", (root / "docs/requirements.md").read_text(), re.M))
    mapping = json.loads((root / "docs/tracking_index.json").read_text())
    check_records(mapping, tasks, requirements, milestones)
    for path in [root / "AGENTS.md", *sorted((root / "docs").rglob("*.md"))]:
        for target in re.findall(r"\]\(([^)]+)\)", path.read_text()):
            if "://" not in target and not target.startswith("#"):
                require((path.parent / target.split("#")[0]).exists(), f"Broken link: {path}: {target}")
    for task in tasks.values():
        for target in task["references"] + task["documentation"]:
            if "://" not in target:
                require((root / target.split("#")[0]).exists(), f"Broken task reference: {target}")
    print(json.dumps({"result": "PASS", "tasks": len(tasks), "requirements": len(requirements),
                      "milestones": len(milestones), "selected_route": "local"}))


if __name__ == "__main__":
    check(Path(sys.argv[1]).resolve() if len(sys.argv) == 2 else Path(__file__).resolve().parents[1])
