#!/usr/bin/env python3
"""
Ambara Plugin System — Interactive Checkpoint Runner
=====================================================
Run after every completed TODO to allow human guidance between steps.

Usage:
    python3 checkpoint.py "<todo_id>" "<completed_description>"
"""

import sys
import json
import datetime
import os
import textwrap

CHECKPOINT_LOG = "checkpoint_log.json"


def load_log():
    if os.path.exists(CHECKPOINT_LOG):
        with open(CHECKPOINT_LOG, "r") as f:
            return json.load(f)
    return {"checkpoints": []}


def save_log(data):
    with open(CHECKPOINT_LOG, "w") as f:
        json.dump(data, f, indent=2)


def print_banner(todo_id, description):
    width = 72
    print("\n" + "=" * width)
    print(f"  CHECKPOINT REACHED: {todo_id}".center(width))
    print("=" * width)
    print(f"\n  Completed: {description}\n")
    print("-" * width)


def print_menu():
    print(textwrap.dedent("""
    What would you like to do next?

      [1] Continue to the next TODO automatically
      [2] Re-run the current TODO (redo with improvements)
      [3] Skip to a specific TODO by ID
      [4] Print all remaining TODOs
      [5] Print the full checkpoint log
      [6] Run cargo check + clippy now
      [7] Run cargo test now
      [8] Run cargo doc now
      [9] Abort and save session state
      [0] Enter a free-form instruction for the model

    Enter choice (default=1):
    """))


def run_cargo(command):
    import subprocess
    print(f"\n[RUNNING] cargo {command}\n")
    result = subprocess.run(
        ["cargo"] + command.split(),
        capture_output=False,
        text=True
    )
    if result.returncode != 0:
        print(f"\n[CARGO FAILED] Exit code {result.returncode}")
    else:
        print(f"\n[CARGO OK] {command} passed")


def main():
    if len(sys.argv) < 3:
        print("Usage: python3 checkpoint.py <todo_id> <description>")
        sys.exit(1)

    todo_id = sys.argv[1]
    description = " ".join(sys.argv[2:])
    timestamp = datetime.datetime.now().isoformat()

    log = load_log()
    log["checkpoints"].append({
        "todo_id": todo_id,
        "description": description,
        "timestamp": timestamp,
        "decision": None
    })

    print_banner(todo_id, description)
    print_menu()

    try:
        choice = input("  > ").strip() or "1"
    except (EOFError, KeyboardInterrupt):
        choice = "1"

    decision = ""

    if choice == "1":
        decision = "CONTINUE"
        print("\n[CHECKPOINT] Continuing to next TODO...\n")

    elif choice == "2":
        decision = "REDO"
        print(f"\n[CHECKPOINT] Re-running TODO {todo_id}...\n")

    elif choice == "3":
        target = input("  Enter target TODO ID: ").strip()
        decision = f"SKIP_TO:{target}"
        print(f"\n[CHECKPOINT] Skipping to {target}...\n")

    elif choice == "4":
        decision = "LIST"
        print("\n[CHECKPOINT] Printing remaining TODOs...\n")
        # Model should print the TODO list from its context

    elif choice == "5":
        decision = "LOG"
        print("\n[CHECKPOINT] Full checkpoint log:")
        for cp in log["checkpoints"]:
            print(f"  {cp['timestamp']} | {cp['todo_id']} | {cp['description']}")
        print()

    elif choice == "6":
        decision = "CARGO_CHECK"
        run_cargo("check --all-features 2>&1")
        run_cargo("clippy --all-features -- -D warnings 2>&1")

    elif choice == "7":
        decision = "CARGO_TEST"
        run_cargo("test --all-features 2>&1")

    elif choice == "8":
        decision = "CARGO_DOC"
        run_cargo("doc --all-features --no-deps 2>&1")

    elif choice == "9":
        decision = "ABORT"
        save_log(log)
        print(f"\n[CHECKPOINT] Session state saved to {CHECKPOINT_LOG}. Aborting.\n")
        sys.exit(0)

    elif choice == "0":
        try:
            instruction = input("  Enter free-form instruction:\n  > ").strip()
            decision = f"INSTRUCTION:{instruction}"
            print(f"\n[CHECKPOINT] Instruction recorded. Model should act on: '{instruction}'\n")
        except (EOFError, KeyboardInterrupt):
            decision = "CONTINUE"

    log["checkpoints"][-1]["decision"] = decision
    save_log(log)

    print(f"[CHECKPOINT LOGGED] {todo_id} → {decision}\n")


if __name__ == "__main__":
    main()
