"""Replay a confidence CSV over UDP as if the model were running.

Tests the receiver and the network path without the GPU.
usage: python replay_csv.py <receiver-ip> [confidence.csv]
"""
import csv
import socket
import struct
import sys
import time

PORT = 5106
host = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1"
path = sys.argv[2] if len(sys.argv) > 2 else "confidence_sample.csv"

with open(path, newline="", encoding="utf-8") as f:
    rows = list(csv.DictReader(f))

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
prev_t = None
for row in rows:
    t = int(row["timestamp_ns"])
    if prev_t is not None:
        time.sleep((t - prev_t) / 1e9)  # keep the original pacing between tokens
    prev_t = t
    # 20 bytes, little-endian: u32 index, u64 t_ns, f32 entropy, f32 top_prob
    payload = struct.pack("<IQff", int(row["index"]), t, float(row["entropy"]), float(row["top_prob"]))
    sock.sendto(payload, (host, PORT))

sock.sendto(b"", (host, PORT))  # empty datagram = end of run
print(f"sent {len(rows)} readings to {host}:{PORT}")
