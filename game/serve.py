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
SAMPLE_C10 = REPO / "samples" / "ai_sample.c10"
SAMPLE_CALIB = REPO / "samples" / "calib_synthetic.csv"
TARGET = REPO / "target"
MODEL_FILE = TARGET / "model.c10"
EXE = TARGET / "debug"
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
    """Run every binary's tests. Returns (tests, compile_error).
    Tests are keyed by name; the new milestones use prefixes so names never collide."""
    # stderr (the "Running unittests ..." lines) merged into stdout, so they stay in order
    try:
        r = subprocess.run(["cargo", "test", "--bins", "--no-fail-fast", "--color", "never"], cwd=REPO,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=900,
                           env=dict(os.environ, RUST_BACKTRACE="0"))
    except (FileNotFoundError, subprocess.TimeoutExpired) as e:
        return {}, f"cargo test didn't run: {e}"
    r.stderr = r.stdout
    tests = {}
    current = "?"
    lines = r.stdout.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        m = re.search(r"Running unittests src/(?:bin/)?(\w+)\.rs", line)
        if m:
            current = "ch10scan" if m.group(1) == "main" else m.group(1)
        m = re.match(r"test tests::(\w+) \.\.\. (ok|FAILED|ignored)", line)
        if m:
            key = m.group(1) if m.group(1) not in tests else f"{current}::{m.group(1)}"
            tests[key] = {"status": "pass" if m.group(2) == "ok" else "fail", "message": "", "bin": current}
        m = re.match(r"---- tests::(\w+) stdout ----", line)
        if m:
            body = []
            i += 1
            while i < len(lines) and not lines[i].startswith(("---- ", "failures:", "test result")):
                body.append(lines[i])
                i += 1
            text = "\n".join(body).split("stack backtrace:")[0].strip().splitlines()
            text = [l for l in text if "RUST_BACKTRACE" not in l]
            for key in (f"{current}::{m.group(1)}", m.group(1)):
                if key in tests and tests[key].get("bin") == current:
                    tests[key]["message"] = "\n".join(text[:8])
                    break
            continue
        i += 1
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
    with tempfile.TemporaryDirectory() as tmp:
        r = run([str(EXE / "ch10scan"), str(path)], cwd=tmp, timeout=120)
    out = r.stdout + "\n" + r.stderr
    ok = re.search(r"OK: all (\d+) bytes accounted for", out)
    counts = {}
    for m in re.finditer(r"ch\s+(\d+)\s+type 0x([0-9a-fA-F]{2}).*?packets\s+(\d+)", out):
        counts[f"{int(m.group(1))}:{m.group(2).lower()}"] = int(m.group(3))
    return {
        "ok": bool(ok),
        "bytes": int(ok.group(1)) if ok else None,
        "readings": counts.get("0:00", 0) if ok else 0,
        "probe_packets": counts.get("2:00", 0) if ok else 0,
        "pcm_packets": counts.get("3:09", 0) if ok else 0,
    }


def ch10_packets(data):
    """A small independent Ch10 reader: (channel, data_type, rtc, body) for each packet."""
    out, off = [], 0
    while off + 24 <= len(data):
        sync, ch, plen, dlen, _ver, _seq, _flags, dt = struct.unpack_from("<HHIIBBBB", data, off)
        if sync != 0xEB25 or plen < 24 or off + plen > len(data):
            break
        rtc = int.from_bytes(data[off + 16:off + 22], "little")
        out.append((ch, dt, rtc, data[off + 24:off + 24 + dlen]))
        off += plen
    return out


def check_tail_once():
    r = run([str(EXE / "ch10tail"), str(SAMPLE_C10), "--once"], timeout=60)
    n = sum(1 for l in r.stdout.splitlines() if " entropy " in l)
    if n == 58 and "time: day 324 06:17:01" in r.stdout:
        return {"pass": True, "message": "58 readings and the time anchor, read back from your recording."}
    return {"pass": False, "message": f"Expected 58 entropy lines and the day 324 time line, got {n} lines.\n" + r.stderr[-400:]}


def check_cal_report():
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "report.txt"
        r = run([str(EXE / "ch10cal"), str(SAMPLE_CALIB), str(out)], timeout=60)
        ok = r.returncode == 0 and out.exists() and "best AUROC" in out.read_text() and "95% band" in out.read_text()
    if ok:
        return {"pass": True, "message": "Full report with AUROC and error bands (synthetic sample data)."}
    return {"pass": False, "message": "ch10cal didn't produce a full report:\n" + first_error(r.stderr, 8)}


def check_probe_train():
    """Train on small synthetic features: two classes separated along one direction."""
    import random
    rng = random.Random(3)
    dim = 24
    direction = [rng.gauss(0, 1) for _ in range(dim)]
    with tempfile.TemporaryDirectory() as tmp:
        feats, probe = Path(tmp) / "f.bin", Path(tmp) / "p.bin"
        rows = []
        for ex in range(100):
            for k in range(8):
                label = 1 if (ex % 2 and k < 4) else 0
                rows.append((ex, label, [rng.gauss(0, 1) + (1.2 * direction[i] if label else 0) for i in range(dim)]))
        with open(feats, "wb") as f:
            f.write(b"PRB1" + struct.pack("<III", len(rows), dim, 7))
            for ex, label, v in rows:
                f.write(struct.pack("<II", ex, label) + struct.pack(f"<{dim}f", *v))
        r = run([str(EXE / "probetrain"), str(feats), str(probe)], timeout=120)
        m = re.search(r"test : accuracy ([\d.]+)%", r.stdout)
        valid = probe.exists() and probe.read_bytes()[:4] == b"PRW1" and len(probe.read_bytes()) == 16 + 12 * dim
    if m and float(m.group(1)) >= 90 and valid:
        return {"pass": True, "message": f"Held-out accuracy {m.group(1)}% on synthetic features, probe file valid."}
    return {"pass": False, "message": "probetrain didn't learn the synthetic probe:\n" + (r.stdout[-300:] or first_error(r.stderr, 8))}


def check_pcm_round_trip():
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "pcm.c10"
        r = run([str(EXE / "ch10pcm"), str(SAMPLE_C10), str(out)], timeout=60)
        if r.returncode != 0 or not out.exists():
            return {"pass": False, "message": "ch10pcm didn't convert the sample:\n" + first_error(r.stderr, 8)}
        scan = scan_c10(out)
        pcm = ch10_packets(out.read_bytes())
    original = [(struct.unpack_from("<I", b, 4)[0], struct.unpack_from("<f", b, 8)[0], struct.unpack_from("<f", b, 12)[0])
                for ch, dt, _rtc, b in ch10_packets(SAMPLE_C10.read_bytes()) if ch == 0 and dt == 0 and len(b) >= 16]
    frames = []
    for ch, dt, _rtc, body in pcm:
        if ch == 3 and dt == 0x09:
            off = 4
            while off + 24 <= len(body):
                words = struct.unpack_from("<7H", body, off + 10)
                if words[:2] != (0xFE6B, 0x2840):
                    break
                frames.append(words)
                off += 24
    if not scan["ok"] or scan["pcm_packets"] == 0:
        return {"pass": False, "message": "ch10scan doesn't accept the PCM file, or found no PCM packets."}
    if len(frames) != len(original):
        return {"pass": False, "message": f"{len(frames)} PCM frames for {len(original)} readings."}
    worst = max(max(abs(w[4] * 0.001 - e), abs(w[5] / 65535 - t)) for w, (_i, e, t) in zip(frames, original))
    if worst > 0.001:
        return {"pass": False, "message": f"Decommutated values are off by up to {worst:.4f}."}
    return {"pass": True, "message": f"{len(frames)} frames decommutate back to the original readings (worst error {worst:.5f})."}


def field_facts():
    """The files you produce on real runs, checked independently."""
    facts = {}
    station = TARGET / "station.c10"
    facts["station"] = scan_c10(station) if station.exists() else {"ok": False, "readings": 0}

    calib = REPO / "calib_results.csv"
    facts["calib"] = {"rows": 0, "best_auroc": None}
    if calib.exists():
        with open(calib, newline="", encoding="utf-8") as f:
            facts["calib"]["rows"] = sum(1 for _ in csv.DictReader(f))
        if (EXE / "ch10cal").exists():
            with tempfile.TemporaryDirectory() as tmp:
                r = run([str(EXE / "ch10cal"), str(calib), str(Path(tmp) / "r.txt")], timeout=60)
            m = re.search(r"best AUROC ([\d.]+)", r.stdout)
            facts["calib"]["best_auroc"] = float(m.group(1)) if m else None

    probe = REPO / "probe.bin"
    data = probe.read_bytes() if probe.exists() else b""
    facts["probe"] = {"valid": len(data) >= 16 and data[:4] == b"PRW1"
                      and len(data) == 16 + 12 * struct.unpack_from("<I", data, 4)[0]}

    run_file = TARGET / "probe_run.c10"
    scores = []
    if run_file.exists():
        scores = [struct.unpack_from("<f", b, 8)[0] for ch, dt, _r, b in ch10_packets(run_file.read_bytes())
                  if ch == 2 and dt == 0 and len(b) >= 12]
    facts["probe_run"] = {"probe_packets": len(scores), "max_score": max(scores) if scores else 0.0}

    pcm = TARGET / "model_pcm.c10"
    facts["model_pcm"] = scan_c10(pcm) if pcm.exists() else {"ok": False, "pcm_packets": 0}
    return facts


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
    """Pull each YOUR PART comment block out of every binary."""
    text = "\n".join(p.read_text(encoding="utf-8") for p in sorted((REPO / "src" / "bin").glob("*.rs")))
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

    if not compile_error:
        run(["cargo", "build", "-q", "--bins"])

    def ready(*names):
        return not compile_error and all(tests.get(t, {}).get("status") == "pass" for t in names)

    e2e = {}
    if ready("datagram_parses", "recorder_first_reading_brings_headers",
             "recorder_later_readings_are_one_packet", "udp_records_live_readings"):
        e2e["byte_perfect"] = check_byte_perfect()
    if ready("tail_packet_reads_whole_file", "tail_confidence_decodes_reading_6", "tail_time_decodes_anchor",
             "tail_strip_line_draws_bars"):
        e2e["tail_once"] = check_tail_once()
    if ready("cal_auroc_textbook_example", "cal_reliability_bins_scores", "cal_wilson_matches_reference"):
        e2e["cal_report"] = check_cal_report()
    if ready("probe_sigmoid_basics", "probe_score_known_value", "probe_learns_two_blobs", "probe_confusion_counts"):
        e2e["probe_train"] = check_probe_train()
    if ready("pcm_counts_scale_and_clamp", "pcm_minor_frame_layout", "pcm_packet_header_and_layout", "pcm_decom_round_trip"):
        e2e["pcm_round_trip"] = check_pcm_round_trip()

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
        **field_facts(),
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
