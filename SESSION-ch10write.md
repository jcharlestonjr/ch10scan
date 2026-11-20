# Session: ch10write step 1 (TMATS-only file)

**Where you left off:** the AI confidence channel works on the 4060 (CSV and plot). It caught a made-up IRIG expansion.
**Today's goal:** write a Chapter 10 file that holds only a TMATS packet, and have ch10scan read it back cleanly.
**Time box:** about 60-75 minutes.

## Steps

- [ ] **Get the branch (5 min).** Commit anything unpushed on your machine first, then:
      `git fetch` and `git switch ch10write-setup`
- [ ] **See the starting line (2 min).** `cargo test --bin ch10write`
      All 5 tests fail with "not yet implemented". That's expected.
- [ ] **Read the byte map (5 min).** It's the comment at the top of `src/bin/ch10write.rs`.
      It's the same layout `parse_header` reads, just written instead of read.
- [ ] **YOUR PART 1: `build_header` (20 min).**
      Done when `cargo test --bin ch10write builds_known_header` passes.
      That test rebuilds the real header from discrete.c10 byte for byte.
- [ ] **YOUR PART 2: `build_tmats_packet` (20 min).**
      Done when `cargo test --bin ch10write` shows all 5 passing.
- [ ] **Delete the `#![allow(...)]` line** near the top of ch10write.rs. `cargo build --bin ch10write` should show no warnings.
- [ ] **Write a real file (2 min).** `cargo run --bin ch10write -- target/ai_tmats.c10`
      Expect: `wrote 156 bytes to target/ai_tmats.c10`
- [ ] **Read it back with ch10scan (5 min).** `cargo run -- target/ai_tmats.c10`
      Expect:
      ```
      TMATS: 128 bytes -> tmats.txt
      ch   0  type 0x01  Computer Generated F1 ...  packets   1  bytes 156
      OK: all 156 bytes accounted for (0 skipped)
      ```
- [ ] **Check the TMATS (3 min).** Open `tmats.txt`: it should match the `TMATS` const.
      Then run `git restore tmats.txt`, because ch10scan overwrites the sample one.
- [ ] **Write the re-entry note** in Notes.md.

## Done when
- [ ] `cargo test --bin ch10write`: 5 passed
- [ ] ch10scan reports 1 packet and "OK: all 156 bytes accounted for"

## Gotchas
- Fill bytes 0..22 **before** computing the checksum. The checksum covers them.
- `data_len` doesn't include the header or the filler. `packet_len` includes everything.
- `cargo run` with no `--bin` still runs ch10scan (`default-run` is set in Cargo.toml).

## Next session (don't start it today)
Add a time packet and the confidence channel's packets, so the CSV from the 4060 lands in the same file.

## Parking lot
Agent harness, envelope expansion, SPARK kill switch, PCM ground-station stretch goal, bigger model,
checking output with an independent reader (irig106.org tools).
