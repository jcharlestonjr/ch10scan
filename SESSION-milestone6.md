# Session: milestone 6 (Ground Station Ridge)

**Where you left off:** live recording works, and the live file is byte-identical to CSV mode.
**Today's goal:** a live ground-station display (`ch10tail`), then a range day with the recorder on the Fedora box.
**Time box:** about 90 minutes. There's a natural stopping point before the Fedora part.

## Steps

- [ ] **Get the update (2 min).** `git pull`
- [ ] **Quick fix first (2 min).** In `record()` in ch10write.rs, `expected = r.index + 1;` sits inside the
      `if r.index != expected` block, so every other reading prints a false "gap". Move that line
      below the `if`. A replay should then show no `gap` lines.
- [ ] **See the starting line (2 min).** `cargo test --bin ch10tail`
      Expect 6 failed ("not yet implemented"). The later milestones' binaries fail too. That's expected,
      and the game shows them as zones further up the map.
- [ ] **YOUR PART 12: `next_packet` (25 min).** The heart of a live reader: only hand back whole packets.
      Done when `cargo test --bin ch10tail tail_packet` passes (3 tests).
- [ ] **YOUR PART 13: `decode_confidence` (10 min).** Your packet builder, backwards.
      Done when `cargo test --bin ch10tail tail_confidence` passes.
- [ ] **YOUR PART 14: `decode_time` (15 min).** Your BCD, backwards.
      Done when `cargo test --bin ch10tail tail_time` passes.
- [ ] **YOUR PART 15: `strip_line` (10 min).** Done when `cargo test --bin ch10tail tail_strip` passes.
- [ ] **Read your own recording (2 min).** `cargo run --bin ch10tail -- samples/ai_sample.c10 --once`
      Expect a strip chart of 58 readings, each stamped with a clock time from 06:17:01.998 on.
- [ ] **Watch it live in WSL (5 min).** Three terminals, started in this order:
      1. `cargo run --bin ch10tail -- target/live.c10` (it waits for the file to appear)
      2. `cargo run --bin ch10write -- --udp 5106 target/live.c10`
      3. `python3 python/replay_csv.py 127.0.0.1 confidence_sample.csv`
- [ ] **Stopping point.** Everything above is a complete win on its own. Fight The Dropout in the game.
- [ ] **Range day on the Fedora box (30 min).**
      1. On Fedora, install Rust with rustup (`curl https://sh.rustup.rs -sSf | sh`), then
         `git clone https://github.com/jcharlestonjr/ch10scan.git`, `cd ch10scan`, `git switch ch10write-setup`
      2. Open the port until the next reboot: `sudo firewall-cmd --add-port=5106/udp`
      3. Note its address: `hostname -I`
      4. Fedora terminal A: `cargo run --bin ch10write -- --udp 5106 target/station.c10`
      5. Fedora terminal B: `cargo run --bin ch10tail -- target/station.c10`
      6. WSL (venv active, in ~/ai-telemetry): `python ~/ch10scan/python/stream_confidence.py <FEDORA-IP> "your prompt"`
      7. Watch the strip chart scroll on the Fedora box while the 4060 generates.
- [ ] **Bring the tape home (5 min).** Fedora needs sshd running (`sudo systemctl enable --now sshd`). Then in WSL:
      `scp <you>@<FEDORA-IP>:ch10scan/target/station.c10 ~/ch10scan/target/station.c10`
      Press Rescan in the game for the Range day achievement.
- [ ] **Wrap up.** Re-entry note in Notes.md, then commit and push.

## Done when
- [ ] `cargo test --bin ch10tail`: 6 passed
- [ ] The Dropout is defeated. The final blow runs `ch10tail --once` on your sample recording.
- [ ] `target/station.c10` came home from the Fedora box

## If nothing reaches the Fedora box
- Start the receiver before the sender. WSL can send out to your LAN, but the Fedora firewall has to allow 5106/udp.
- Check both machines are on the same network (`ping <FEDORA-IP>` from WSL).

## Notes
- Why `next_packet` waits: the recorder writes a packet in one go, but a reader polling the file can still
  catch it halfway. Returning None and trying again next poll is how real quick-look displays stay sane.
- After milestone 8, `ch10tail` also shows probe readings on channel 2. That part is already pre-filled.
