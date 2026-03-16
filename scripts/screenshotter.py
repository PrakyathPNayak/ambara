#!/usr/bin/env python3
"""Ambara autonomous screenshot capture and verification utility."""

import argparse
import datetime
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

SCREENSHOTS_DIR = Path("screenshots")
VERDICTS_DIR = Path("build/verdicts")
LOGS_DIR = Path("logs")
BUILD_DIR = Path("build")

SCREENSHOTS_DIR.mkdir(parents=True, exist_ok=True)
VERDICTS_DIR.mkdir(parents=True, exist_ok=True)
LOGS_DIR.mkdir(parents=True, exist_ok=True)
BUILD_DIR.mkdir(parents=True, exist_ok=True)


def _which(binary: str) -> bool:
    """Return True if a binary exists in PATH.

    Args:
        binary: Executable name.

    Returns:
        True when executable is available, False otherwise.

    Raises:
        OSError: If subprocess invocation fails.
    """
    return subprocess.run(["which", binary], capture_output=True, check=False).returncode == 0


def capture_screen(tag: str, url: str | None = None) -> Path:
    """Capture screenshot using URL rendering or display capture.

    Args:
        tag: Logical tag for output filename.
        url: Optional URL to render in headless browser.

    Returns:
        Path to screenshot file, or headless JSON report path.

    Raises:
        RuntimeError: If capture command crashes unexpectedly.
    """
    ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    out = SCREENSHOTS_DIR / f"{tag}_{ts}.png"

    if url:
        for chrome_bin in ["chromium-browser", "chromium", "google-chrome", "google-chrome-stable"]:
            if _which(chrome_bin):
                subprocess.run(
                    [
                        chrome_bin,
                        "--headless",
                        "--disable-gpu",
                        "--window-size=1400,900",
                        f"--screenshot={out}",
                        url,
                    ],
                    capture_output=True,
                    check=False,
                    timeout=30,
                )
                if out.exists():
                    print(f"[SCREENSHOT] Captured {url} -> {out}")
                    return out

    if _which("scrot"):
        subprocess.run(["scrot", str(out)], capture_output=True, check=False)
        if out.exists():
            print(f"[SCREENSHOT] scrot -> {out}")
            return out

    if _which("gnome-screenshot"):
        subprocess.run(["gnome-screenshot", "-f", str(out)], capture_output=True, check=False)
        if out.exists():
            print(f"[SCREENSHOT] gnome-screenshot -> {out}")
            return out

    if _which("import"):
        display = os.environ.get("DISPLAY", ":0")
        subprocess.run(["import", "-window", "root", "-display", display, str(out)], capture_output=True, check=False)
        if out.exists():
            print(f"[SCREENSHOT] import -> {out}")
            return out

    placeholder = out.with_suffix(".headless.json")
    placeholder.write_text(
        json.dumps(
            {
                "mode": "headless",
                "tag": tag,
                "url": url,
                "timestamp": ts,
                "note": "No visual capture backend available; using log/API verification.",
            },
            indent=2,
        )
    )
    print(f"[SCREENSHOT] Headless fallback -> {placeholder}")
    return placeholder


def _scan_logs_for_keywords(keywords: list[str]) -> str:
    """Scan logs and JSON artifacts for expected keywords.

    Args:
        keywords: Keywords to look for.

    Returns:
        Concatenated text blob from logs and JSON artifacts.

    Raises:
        OSError: If file reading fails due to permission errors.
    """
    chunks: list[str] = []
    for log_file in LOGS_DIR.glob("*.log"):
        try:
            chunks.append(log_file.read_text(errors="replace")[-12000:])
        except (OSError, UnicodeDecodeError):
            continue
    for artifact in BUILD_DIR.glob("*.json"):
        try:
            chunks.append(artifact.read_text(errors="replace"))
        except (OSError, UnicodeDecodeError):
            continue
    return "\n".join(chunks)


def _save_verdict(tag: str, verdict: dict[str, Any]) -> None:
    """Persist latest verdict for a tag.

    Args:
        tag: Tag identifier.
        verdict: Verdict payload.

    Returns:
        None.

    Raises:
        OSError: If file cannot be written.
    """
    out = VERDICTS_DIR / f"{tag}_latest.json"
    out.write_text(json.dumps(verdict, indent=2))


def analyze_screenshot(tag: str, expect_keywords: list[str]) -> dict[str, Any]:
    """Analyze latest screenshot/headless report for expected keywords.

    Args:
        tag: Screenshot tag.
        expect_keywords: Required keywords.

    Returns:
        Verdict dictionary with pass/fail and notes.

    Raises:
        RuntimeError: If OCR pipeline fails unexpectedly.
    """
    candidates = sorted(SCREENSHOTS_DIR.glob(f"{tag}_*.png"), reverse=True)
    headless = sorted(SCREENSHOTS_DIR.glob(f"{tag}_*.headless.json"), reverse=True)

    verdict: dict[str, Any] = {
        "tag": tag,
        "timestamp": datetime.datetime.now().isoformat(),
        "expected_keywords": expect_keywords,
        "found_keywords": [],
        "missing_keywords": [],
        "ocr_text": "",
        "passed": False,
        "mode": "unknown",
        "screenshot_path": None,
        "notes": [],
    }

    latest_png_mtime = candidates[0].stat().st_mtime if candidates else 0
    latest_headless_mtime = headless[0].stat().st_mtime if headless else 0

    if headless and (not candidates or latest_headless_mtime > latest_png_mtime):
        text = _scan_logs_for_keywords(expect_keywords)
        verdict["mode"] = "headless_log_analysis"
        verdict["screenshot_path"] = str(headless[0])
        verdict["ocr_text"] = text
        verdict["found_keywords"] = [k for k in expect_keywords if k.lower() in text.lower()]
        verdict["missing_keywords"] = [k for k in expect_keywords if k not in verdict["found_keywords"]]
        verdict["passed"] = len(verdict["missing_keywords"]) == 0
        verdict["notes"].append("Headless analysis completed")
        _save_verdict(tag, verdict)
        return verdict

    if not candidates:
        text = _scan_logs_for_keywords(expect_keywords)
        verdict["mode"] = "log_only_analysis"
        verdict["ocr_text"] = text
        verdict["found_keywords"] = [k for k in expect_keywords if k.lower() in text.lower()]
        verdict["missing_keywords"] = [k for k in expect_keywords if k not in verdict["found_keywords"]]
        verdict["passed"] = len(verdict["missing_keywords"]) == 0
        verdict["notes"].append("No screenshot found; used log and artifact scan")
        _save_verdict(tag, verdict)
        return verdict

    shot = candidates[0]
    verdict["mode"] = "visual_ocr"
    verdict["screenshot_path"] = str(shot)

    try:
        import pytesseract  # type: ignore
        from PIL import Image  # type: ignore

        text = pytesseract.image_to_string(Image.open(shot))
        verdict["ocr_text"] = text
        verdict["found_keywords"] = [k for k in expect_keywords if k.lower() in text.lower()]
        verdict["missing_keywords"] = [k for k in expect_keywords if k not in verdict["found_keywords"]]
        verdict["passed"] = len(verdict["missing_keywords"]) == 0
        verdict["notes"].append(f"OCR extracted {len(text)} chars")
    except ImportError:
        text = _scan_logs_for_keywords(expect_keywords)
        verdict["ocr_text"] = text
        verdict["found_keywords"] = [k for k in expect_keywords if k.lower() in text.lower()]
        verdict["missing_keywords"] = [k for k in expect_keywords if k not in verdict["found_keywords"]]
        verdict["passed"] = len(verdict["missing_keywords"]) == 0
        verdict["notes"].append("pytesseract unavailable, used log scan")
    except (OSError, RuntimeError, ValueError) as err:
        text = _scan_logs_for_keywords(expect_keywords)
        verdict["ocr_text"] = text
        verdict["found_keywords"] = [k for k in expect_keywords if k.lower() in text.lower()]
        verdict["missing_keywords"] = [k for k in expect_keywords if k not in verdict["found_keywords"]]
        verdict["passed"] = len(verdict["missing_keywords"]) == 0
        verdict["notes"].append(f"OCR error, used log scan instead: {err}")

    _save_verdict(tag, verdict)
    status = "PASS" if verdict["passed"] else "FAIL"
    print(f"[SCAN {status}] {tag} | found={verdict['found_keywords']} | missing={verdict['missing_keywords']}")
    return verdict


def main() -> int:
    """CLI entrypoint.

    Args:
        None.

    Returns:
        Process exit code.

    Raises:
        SystemExit: Via argparse when invalid arguments are provided.
    """
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="cmd")

    cap = sub.add_parser("capture")
    cap.add_argument("--tag", required=True)
    cap.add_argument("--url", default=None)

    ana = sub.add_parser("analyze")
    ana.add_argument("--tag", required=True)
    ana.add_argument("--expect", required=True)

    ver = sub.add_parser("verdict")
    ver.add_argument("--tag", required=True)

    args = parser.parse_args()

    if args.cmd == "capture":
        capture_screen(args.tag, args.url)
        return 0

    if args.cmd == "analyze":
        keywords = [part.strip() for part in args.expect.split(",") if part.strip()]
        result = analyze_screenshot(args.tag, keywords)
        print(json.dumps(result, indent=2))
        return 0 if result.get("passed") else 1

    if args.cmd == "verdict":
        path = VERDICTS_DIR / f"{args.tag}_latest.json"
        if not path.exists():
            print(f"[VERDICT] No verdict found for tag: {args.tag}")
            return 1
        verdict = json.loads(path.read_text())
        print(json.dumps(verdict, indent=2))
        return 0 if verdict.get("passed") else 1

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
