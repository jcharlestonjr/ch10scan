# Session: milestone 9 (The PCM Citadel)

**Where you left off:** two live AI channels, confidence and the injection probe, recorded in Ch10.
**Today's goal:** put them into PCM, the format every telemetry ground station already speaks.
**Time box:** about 90 minutes.

## The idea in one paragraph
Each confidence reading becomes one PCM minor frame: the 32-bit sync `FE6B2840`, then 16-bit words for a
frame counter, the token index, entropy, top probability and the probe score. The probe channel is
asynchronous, so each frame carries its latest value (sample-and-hold), the way a PCM encoder samples any
input. Frames go into PCM Data Format 1 packets in packed mode, laid out the same way as the packed channel
in `pcm.c10` from a real recorder. One of the tests checks your bytes against that file.

## Steps

- [ ] **Get the update (2 min).** `git pull`
- [ ] **See the starting line (2 min).** `cargo test --bin ch10pcm`, expect 5 failed.
- [ ] **YOUR PART 27: `to_counts` (10 min).** Done when `cargo test --bin ch10pcm pcm_counts` passes.
- [ ] **YOUR PART 28: `minor_frame` (15 min).** Done when `cargo test --bin ch10pcm pcm_minor_frame` passes.
- [ ] **YOUR PART 29: `build_pcm_packet` (30 min).** Done when `cargo test --bin ch10pcm pcm_packet` passes (2 tests).
- [ ] **YOUR PART 30: `decom` (20 min).** Done when `cargo test --bin ch10pcm pcm_decom` passes.
- [ ] **Convert and inspect (5 min).**
      ```
      cargo run --bin ch10pcm -- samples/ai_sample.c10 target/sample_pcm.c10
      cargo run -- target/sample_pcm.c10
      cargo run --bin ch10pcm -- --decom target/sample_pcm.c10
      git restore tmats.txt
      ```
      ch10scan should list `ch 3 type 0x09 PCM F1`, and decom should print your 58 readings back.
- [ ] **Stopping point.** Fight The PCM Warden. Its final blow decommutates your file and compares
      every value with the original.
- [ ] **Your own recording (5 min).** `cargo run --bin ch10pcm -- target/model.c10 target/model_pcm.c10`
      Use `target/probe_run.c10` as input instead to carry all three channels. Rescan for Citadel.
- [ ] **Wrap up.** Re-entry note in Notes.md, then commit and push.

## Done when
- [ ] `cargo test --bin ch10pcm`: 5 passed
- [ ] The PCM Warden is defeated
- [ ] `target/model_pcm.c10` exists and ch10scan reads it cleanly

## The legendary step (not in the game, because only you can check it)
Load `model_pcm.c10` into real ground-station software. The TMATS has R and P groups set up like `pcm.c10`,
so the station should find the PCM stream and frame sync. It doesn't have D and C groups yet (measurement
names and scaling), so define the measurements in the station by hand. The word map and scale factors are
in the TMATS `G\COM` line: entropy is 1 count = 0.001 nats, and probabilities and probe are counts / 65535.
Adding D and C groups is a parking-lot item.

## What I checked
Your frames' first 16 bytes match `pcm.c10`'s packed channel byte for byte, ch10scan accepts the file, and
decommutation returns every reading within one count. I couldn't test it in real ground-station software.
