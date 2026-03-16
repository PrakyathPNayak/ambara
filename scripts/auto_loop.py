#!/usr/bin/env python3
"""Autonomous task state tracker for the Ambara chatbot integration loop."""

import datetime
import json
import sys
from enum import Enum
from pathlib import Path

STATE_FILE = Path("build/loop_state.json")
MAX_RETRIES = 3


class TaskStatus(str, Enum):
    """State machine states for autonomous tasks."""

    PENDING = "PENDING"
    RUNNING = "RUNNING"
    VERIFYING = "VERIFYING"
    PASS = "PASS"
    FAIL = "FAIL"
    BLOCKED = "BLOCKED"
    SKIPPED = "SKIPPED"


def load_state() -> dict:
    """Load loop state from disk.

    Args:
        None.

    Returns:
        State dictionary.

    Raises:
        OSError: If file cannot be read.
        json.JSONDecodeError: If existing state file is malformed.
    """
    if STATE_FILE.exists():
        return json.loads(STATE_FILE.read_text())
    return {
        "tasks": {},
        "current_task": None,
        "start_time": datetime.datetime.now().isoformat(),
        "completed": False,
    }


def save_state(state: dict) -> None:
    """Persist loop state.

    Args:
        state: State dictionary.

    Returns:
        None.

    Raises:
        OSError: If file cannot be written.
    """
    STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
    STATE_FILE.write_text(json.dumps(state, indent=2))


def mark_task(task_id: str, status: TaskStatus, notes: str = "") -> None:
    """Mark task with a new status and optional notes.

    Args:
        task_id: Task identifier.
        status: New status enum.
        notes: Optional status notes.

    Returns:
        None.

    Raises:
        OSError: If state file operations fail.
    """
    state = load_state()
    ts = datetime.datetime.now().isoformat()

    if task_id not in state["tasks"]:
        state["tasks"][task_id] = {"retries": 0}

    state["tasks"][task_id].update({
        "status": status.value,
        "updated_at": ts,
        "notes": notes,
    })

    if status == TaskStatus.FAIL:
        retries = state["tasks"][task_id].get("retries", 0) + 1
        state["tasks"][task_id]["retries"] = retries
        if retries >= MAX_RETRIES:
            state["tasks"][task_id]["status"] = TaskStatus.BLOCKED.value
            print(f"[LOOP] Task {task_id} BLOCKED after {MAX_RETRIES} retries")

    state["current_task"] = task_id
    save_state(state)
    print(f"[LOOP] {task_id} -> {state['tasks'][task_id]['status']} | {notes}")


def get_status(task_id: str) -> str:
    """Fetch current status for task.

    Args:
        task_id: Task identifier.

    Returns:
        Status string.

    Raises:
        OSError: If state file cannot be read.
    """
    state = load_state()
    return state["tasks"].get(task_id, {}).get("status", TaskStatus.PENDING.value)


def print_summary() -> None:
    """Print aggregate status counts.

    Args:
        None.

    Returns:
        None.

    Raises:
        OSError: If state file cannot be read.
    """
    state = load_state()
    counts: dict[str, int] = {s.value: 0 for s in TaskStatus}
    for task in state["tasks"].values():
        status = task.get("status", TaskStatus.PENDING.value)
        counts[status] = counts.get(status, 0) + 1

    print("\n[LOOP SUMMARY]")
    for status, count in counts.items():
        if count > 0:
            print(f"  {status:12s}: {count}")
    print()


def main() -> int:
    """Command line interface for state updates.

    Args:
        None.

    Returns:
        Exit code.

    Raises:
        ValueError: If invalid status string is provided.
    """
    cmd = sys.argv[1] if len(sys.argv) > 1 else "summary"

    if cmd == "mark":
        if len(sys.argv) < 4:
            print("Usage: auto_loop.py mark <task_id> <status> [notes]")
            return 1
        task_id = sys.argv[2]
        status = TaskStatus(sys.argv[3])
        notes = " ".join(sys.argv[4:])
        mark_task(task_id, status, notes)
        return 0

    if cmd == "status":
        if len(sys.argv) < 3:
            print("Usage: auto_loop.py status <task_id>")
            return 1
        print(get_status(sys.argv[2]))
        return 0

    print_summary()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
