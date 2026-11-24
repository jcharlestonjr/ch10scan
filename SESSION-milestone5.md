# Session: milestone 5 (live streaming)

**Where you left off:** ch10write writes TMATS, a time packet and confidence packets from a CSV.
**Today's goal:** the model on Windows streams each reading over UDP as it generates, and
ch10write in WSL records them live into a Ch10 file.
**Time box:** about 90 minutes. There's a natural stopping point after the WSL replay if you need one.

Progress: milestones 1-4 done, **milestone 5 today**.

## First: the milestone 4 bug (read this, 2 min)
Your milestone 4 file passed ch10scan, but every reading's clock time came out **1.996 s early**.
The time packet was stamped at RTC 9,980,100 (0.998 s), and the readings counted from the first
token. Decoded, the file said "at 0.998 s the clock read 06:17:01.000." But 0.998 s after the
first token, the clock really read 06:17:02.996.

The fix is now built into `Recorder` (YOUR PART 10): the time packet's RTC is 0, and readings
count from the anchor. The `recorder_` tests pin both, so it can't come back quietly.
The lesson: byte counts prove a file is well-formed, not that it's right.

## Steps

- [ ] **Get the update (2 min).** `git pull`
- [ ] **See the starting line (2 min).** `cargo test --bin ch10write`
      Expect 16 passed and 5 failed. The 6 warnings (unused imports, `mut`) go away as you
      fill in the parts.
- [ ] **YOUR PART 9: `parse_datagram` (10 min).** Bytes to numbers, the reverse of your packets.
      Done when `cargo test --bin ch10write datagram_` passes.
- [ ] **YOUR PART 10: `Recorder::packets_for` (20 min).** Your milestone 4 logic, one reading
      at a time, with the timing fix. Done when `cargo test --bin ch10write recorder_` passes.
- [ ] **YOUR PART 11: the loop in `record` (15 min).**
      Done when `cargo test --bin ch10write udp_` passes. That's all 21 green.
- [ ] **CSV mode still works (2 min).**
      `cargo run --bin ch10write -- confidence_sample.csv target/csv_mode.c10`
      Expect `wrote 2580 bytes`.
- [ ] **Live replay inside WSL (5 min).** Two terminals:
      - Terminal 1: `cargo run --bin ch10write -- --udp 5106 target/live.c10`
      - Terminal 2: `python3 python/replay_csv.py 127.0.0.1 confidence_sample.csv`

      Watch 58 `rx` lines scroll by in about a second, then:
      `cmp target/live.c10 target/csv_mode.c10`
      **No output means byte-identical.** The live recording matches the CSV conversion exactly.
- [ ] **Stopping point.** Everything above is a complete win on its own.
- [ ] **Windows to WSL (10 min).**
      1. In WSL, run `hostname -I` and note the first address (something like 172.x.x.x).
      2. Copy the scripts to your Windows Python folder (swap in your real path):
         `cp python/*.py confidence_sample.csv /mnt/c/Users/<you>/ai-telemetry/`
      3. Terminal 1 (WSL): `cargo run --bin ch10write -- --udp 5106 target/win.c10`
      4. Windows PowerShell, venv active, in ai-telemetry: `python replay_csv.py <WSL-IP>`
      5. The `rx` lines should appear in WSL.
- [ ] **The real thing (10 min).**
      1. Terminal 1 (WSL): `cargo run --bin ch10write -- --udp 5106 target/model.c10`
      2. Windows: `python stream_confidence.py <WSL-IP> "your own prompt"`
      3. Watch readings arrive in WSL while the model generates, up to 200 tokens.
      4. `cargo run -- target/model.c10`, then `git restore tmats.txt`.
- [ ] **Wrap up.** Write the re-entry note in Notes.md, then commit and push.

## Done when
- [ ] `cargo test --bin ch10write`: 21 passed
- [ ] `cmp target/live.c10 target/csv_mode.c10` prints nothing
- [ ] A live model run from Windows is recorded in WSL and ch10scan reads it cleanly

## If nothing arrives from Windows
- Start the receiver **before** the sender. UDP doesn't wait for a receiver.
- The WSL address changes when WSL restarts, so run `hostname -I` again.
- On Windows 11 you can switch WSL to mirrored networking and use `127.0.0.1` instead. Add
  `[wsl2]` and `networkingMode=mirrored` to `C:\Users\<you>\.wslconfig`, then run `wsl --shutdown`.
- Still stuck? Send me what each terminal shows.

## How the receiver behaves
- It waits as long as needed for the first reading, because the model takes a while to load.
- After that, it stops on the end-of-run datagram, or after 5 s of silence if that got lost.
- Each packet is written the moment it arrives, so a run cut off by Ctrl+C still leaves a valid file.
- `gap: expected reading N, got M` means UDP dropped a datagram. A recorder should say so.

## What I checked
- With a finished version: a replay over UDP was byte-identical to CSV mode, and both matched
  the correct-timing milestone 4 file.
- A run with a skipped reading, a junk datagram and no end marker printed the gap, ignored the
  junk and stopped after 5 s of silence.
- I couldn't run `stream_confidence.py` here because there's no GPU or model download. It uses
  the same sensor code you ran and the same packing as `replay_csv.py`, which I did test.

## Next session (don't start it today)
Pick one: a time packet every second like a real recorder, or the Fedora box as a separate
ground station over the LAN.

## Parking lot
ch10scan rtc column bug, a proper TMATS data channel instead of F0, TMATS entries for the time
channel, merging the branch into main, independent reader check (irig106.org tools), the agent
harness, envelope expansion, the SPARK kill switch, the PCM ground-station stretch goal, a bigger model.
