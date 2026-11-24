#!/usr/bin/env python3
"""Range Quest: turns your real test results into a game.

Run from the repo root, in WSL:
    python3 game/serve.py
then open http://localhost:8642 in your browser.

The server listens on this machine only. Each scan runs cargo test, cargo build,
a few fixed checks and git log. It never runs anything the page sends it.
"""
import argparse
import csv
import datetime as dt
import json
import os
import re
import select
import socket
import struct
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

GAME = Path(__file__).resolve().parent
REPO = GAME.parent
SRC = REPO / "src" / "bin" / "ch10write.rs"
SAMPLE_CSV = REPO / "confidence_sample.csv"
MODEL_FILE = REPO / "target" / "model.c10"
CLAUDE_AUTHOR = "Claude"

scan_lock = threading.Lock()
last_facts = None


def run(cmd, cwd=REPO, timeout=900):
    env = dict(os.environ, RUST_BACKTRACE="0")
    try:
        return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout, env=env)
    except FileNotFoundError:
        return subprocess.CompletedProcess(cmd, 127, "", f"{cmd[0]}: command not found")
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(cmd, 124, "", f"{' '.join(cmd)} timed out")


def first_error(text, max_lines=18):
    """The first error diagnostic, up to the blank line that ends it."""
    lines = text.splitlines()
    for i, line in enumerate(lines):
        if line.startswith("error"):
            block = []
            for l in lines[i:i + max_lines]:
                if block and not l.strip():
                    break
                block.append(l)
            return "\n".join(block)
    return "\n".join(lines[-max_lines:])


def run_tests():
    """Run ch10write's tests. Returns (tests, compile_error)."""
    r = run(["cargo", "test", "--bin", "ch10write", "--no-fail-fast", "--color", "never"])
    out = r.stdout + "\n" + r.stderr
    tests = {}
    for m in re.finditer(r"^test tests::(\w+) \.\.\. (ok|FAILED|ignored)", out, re.M):
        tests[m.group(1)] = {"status": "pass" if m.group(2) == "ok" else "fail", "message": ""}
    for m in re.finditer(r"^---- tests::(\w+) stdout ----\n(.*?)(?=^---- |^failures:|\Z)", out, re.M | re.S):
        lines = m.group(2).strip().split("stack backtrace:")[0].strip().splitlines()
        lines = [l for l in lines if "RUST_BACKTRACE" not in l]
        if m.group(1) in tests:
            tests[m.group(1)]["message"] = "\n".join(lines[:8])
    compile_error = None
    if not tests and r.returncode != 0:
        compile_error = first_error(r.stderr)
    return tests, compile_error


def count_warnings():
    r = run(["cargo", "build", "--bin", "ch10write", "--color", "never"])
    if r.returncode != 0:
        return None
    m = re.search(r"generated (\d+) warnings?", r.stderr)
    return int(m.group(1)) if m else 0


def scan_c10(path):
    """Run ch10scan on a file. It writes tmats.txt to its working folder, so use a temp one."""
    exe = REPO / "target" / "debug" / "ch10scan"
    with tempfile.TemporaryDirectory() as tmp:
        r = run([str(exe), str(path)], cwd=tmp, timeout=120)
    out = r.stdout + "\n" + r.stderr
    ok = re.search(r"OK: all (\d+) bytes accounted for", out)
    f0 = re.search(r"type 0x00 .*?packets\s+(\d+)", out)
    return {
        "ok": bool(ok),
        "bytes": int(ok.group(1)) if ok else None,
        "readings": int(f0.group(1)) if f0 else 0,
    }


def free_udp_port():
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


def check_byte_perfect():
    """Replay the sample CSV over UDP into --udp mode and compare with CSV mode."""
    exe = REPO / "target" / "debug" / "ch10write"
    with tempfile.TemporaryDirectory() as tmp:
        csv_out = Path(tmp) / "csv.c10"
        live_out = Path(tmp) / "live.c10"
        r = run([str(exe), str(SAMPLE_CSV), str(csv_out)], timeout=120)
        if r.returncode != 0 or not csv_out.exists():
            return {"pass": False, "message": "CSV mode didn't finish:\n" + first_error(r.stderr, 8)}

        port = free_udp_port()
        p = subprocess.Popen([str(exe), "--udp", str(port), str(live_out)], cwd=REPO,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        # wait for "listening on UDP port ..." so no datagram is sent before it binds
        ready, _, _ = select.select([p.stderr], [], [], 10)
        first_line = p.stderr.readline() if ready else ""
        if "listening" not in first_line:
            p.kill()
            return {"pass": False, "message": "The receiver didn't start:\n" + first_line}

        with open(SAMPLE_CSV, newline="", encoding="utf-8") as f:
            rows = list(csv.DictReader(f))
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        for row in rows:
            payload = struct.pack("<IQff", int(row["index"]), int(row["timestamp_ns"]),
                                  float(row["entropy"]), float(row["top_prob"]))
            sock.sendto(payload, ("127.0.0.1", port))
            time.sleep(0.001)
        sock.sendto(b"", ("127.0.0.1", port))
        sock.close()
        try:
            _, err = p.communicate(timeout=20)
        except subprocess.TimeoutExpired:
            p.kill()
            return {"pass": False, "message": "The receiver didn't stop after the end-of-run datagram."}

        if not live_out.exists():
            return {"pass": False, "message": "The receiver didn't write a file."}
        a, b = live_out.read_bytes(), csv_out.read_bytes()
        if a == b:
            return {"pass": True, "message": f"{len(a)} bytes, identical."}
        diff = next((i for i in range(min(len(a), len(b))) if a[i] != b[i]), min(len(a), len(b)))
        return {"pass": False,
                "message": f"Live file is {len(a)} bytes, CSV mode is {len(b)}. First difference at byte {diff}."}


def git_facts():
    r = run(["git", "log", "--format=%an%x1f%ad%x1f%s", "--date=iso-strict", "-n", "300"])
    mine = []
    for line in r.stdout.splitlines():
        parts = line.split("\x1f")
        if len(parts) == 3 and parts[0] != CLAUDE_AUTHOR:
            mine.append({"author": parts[0], "date": parts[1], "subject": parts[2]})

    head = run(["git", "rev-parse", "HEAD"]).stdout.strip()
    up = run(["git", "rev-parse", "@{u}"])
    pushed = up.returncode == 0 and up.stdout.strip() == head

    days = {dt.date.fromisoformat(c["date"][:10]) for c in mine}
    return {"mine": mine[:6], "subjects": [c["subject"] for c in mine],
            "pushed": pushed, "streak": weeknight_streak(days)}


def weeknight_streak(days):
    """Weekdays in a row with a commit. Weekends don't count or break it,
    and today doesn't break it before you've committed."""
    def prev_weekday(d):
        d -= dt.timedelta(days=1)
        while d.weekday() >= 5:
            d -= dt.timedelta(days=1)
        return d

    d = dt.date.today()
    while d.weekday() >= 5:
        d -= dt.timedelta(days=1)
    if d not in days:
        d = prev_weekday(d)
    streak = 0
    while d in days:
        streak += 1
        d = prev_weekday(d)
    return streak


def briefings():
    """Pull each YOUR PART comment block out of ch10write.rs."""
    if not SRC.exists():
        return {}
    text = SRC.read_text(encoding="utf-8")
    out = {}
    for m in re.finditer(r"// YOUR PART (\d+)([^\n]*)\n((?:[ \t]*//[^\n]*\n)*)", text):
        head = m.group(2).lstrip(" :").strip()
        body = [re.sub(r"^[ \t]*// ?", "", l) for l in m.group(3).splitlines()]
        out[m.group(1)] = "\n".join([head] + body).strip()
    return out


def quest_files():
    data = json.loads((GAME / "quests.json").read_text(encoding="utf-8"))
    names = set()

    def walk(v):
        if isinstance(v, dict):
            for k, x in v.items():
                if k == "files":
                    names.update(x)
                else:
                    walk(x)
        elif isinstance(v, list):
            for x in v:
                walk(x)

    walk(data)
    return sorted(names)


def scan():
    started = time.time()
    tests, compile_error = run_tests()
    warnings = count_warnings()
    run(["cargo", "build", "-q", "--bin", "ch10scan"])

    e2e = {}
    live_ready = all(tests.get(t, {}).get("status") == "pass" for t in [
        "datagram_parses", "recorder_first_reading_brings_headers",
        "recorder_later_readings_are_one_packet", "udp_records_live_readings"])
    if live_ready and warnings is not None:
        e2e["byte_perfect"] = check_byte_perfect()

    model = {"exists": MODEL_FILE.exists()}
    if model["exists"]:
        model.update(scan_c10(MODEL_FILE))

    return {
        "scanned_at": dt.datetime.now().isoformat(timespec="seconds"),
        "seconds": round(time.time() - started, 1),
        "compile_error": compile_error,
        "tests": tests,
        "files": {name: (REPO / name).exists() for name in quest_files()},
        "warnings": warnings,
        "e2e": e2e,
        "model_file": model,
        "git": git_facts(),
        "briefings": briefings(),
    }


class Handler(BaseHTTPRequestHandler):
    def send(self, code, body, ctype):
        data = body if isinstance(body, bytes) else body.encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        global last_facts
        if self.path in ("/", "/index.html"):
            self.send(200, (GAME / "index.html").read_bytes(), "text/html; charset=utf-8")
        elif self.path == "/quests.json":
            self.send(200, (GAME / "quests.json").read_bytes(), "application/json")
        elif self.path == "/api/facts":
            with scan_lock:
                if last_facts is None:
                    last_facts = scan()
                facts = last_facts
            self.send(200, json.dumps(facts), "application/json")
        else:
            self.send(404, "not found", "text/plain")

    def do_POST(self):
        global last_facts
        if self.path == "/api/scan":
            with scan_lock:
                last_facts = scan()
                facts = last_facts
            self.send(200, json.dumps(facts), "application/json")
        else:
            self.send(404, "not found", "text/plain")

    def log_message(self, fmt, *args):
        pass  # keep the terminal quiet


def main():
    ap = argparse.ArgumentParser(description="Range Quest game server")
    ap.add_argument("--port", type=int, default=8642)
    ap.add_argument("--host", default="127.0.0.1",
                    help="use 0.0.0.0 if your Windows browser can't reach 127.0.0.1")
    args = ap.parse_args()
    server = ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"Range Quest is running. Open http://localhost:{args.port} in your browser.")
    print("The first scan compiles and runs your tests, so give it a few seconds. Ctrl+C to stop.")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nRange Quest stopped.")


if __name__ == "__main__":
    main()
