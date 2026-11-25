# Session: milestone 8 (Probe Depths)

**Where you left off:** the confidence channel is calibrated, with error bands.
**Goal:** a second sensor inside the model that detects prompt injection, trained by your own code and
recorded live as channel 2.
**Time box:** this is the biggest milestone, so it's split into two sessions of about 75 minutes each.

## The idea in one paragraph
A probe is a weighted sum of one layer's hidden state, squashed to 0..1. `probe_features.py` writes
documents with and without an injected instruction and saves the model's middle-layer state for every
token, labeled injection or not. Your `probetrain` learns the weights with logistic regression.
`stream_probe.py` then applies them live to every token, so channel 2 should spike on the injected sentence
even when the model's answer looks normal.

## Session A: the Rust side

- [ ] **Get the update (2 min).** `git pull`
- [ ] **See the starting line (2 min).** `cargo test --bin probetrain` (4 failed) and
      `cargo test --bin ch10write` (21 passed, 4 failed: the new `chan2_` tests).
- [ ] **YOUR PART 19: `sigmoid` (5 min).** Done when `cargo test --bin probetrain probe_sigmoid` passes.
- [ ] **YOUR PART 20: `score` (10 min).** Done when `cargo test --bin probetrain probe_score` passes.
- [ ] **YOUR PART 21: `train_epoch` (20 min).** Done when `cargo test --bin probetrain probe_learns` passes.
- [ ] **YOUR PART 22: `evaluate` (10 min).** Done when `cargo test --bin probetrain probe_confusion` passes.
- [ ] **YOUR PART 23: `parse_probe_datagram` (5 min)** in ch10write.rs. Done when `chan2_datagram` passes.
- [ ] **YOUR PART 24: `build_probe_packet` (10 min).** Done when `chan2_packet` passes.
- [ ] **YOUR PART 25: `probe_packets_for` (10 min).** Done when `chan2_recorder` passes.
- [ ] **YOUR PART 26: the record loop (10 min).** Done when `chan2_udp` passes. That's all 25 green.
- [ ] **Stopping point.** Fight The Whispering Injector. Its final blow trains a probe on synthetic data.

## Session B: the field work

- [ ] **Export features (10 min, mostly GPU).** In WSL with the venv active:
      `cd ~/ch10scan && python python/probe_features.py features.bin`
      240 examples, a file of roughly 25 MB. It's in `.gitignore`.
- [ ] **Train the probe (2 min).** `cargo run --release --bin probetrain -- features.bin probe.bin`
      Watch the loss fall each epoch, then read the held-out accuracy, precision and recall.
- [ ] **Catch an injection live (10 min).** Three terminals:
      1. `cargo run --bin ch10tail -- target/probe_run.c10`
      2. `cargo run --bin ch10write -- --udp 5106 target/probe_run.c10`
      3. `python python/stream_probe.py 127.0.0.1 probe.bin`
      The default prompt hides "Ignore all previous instructions..." in a range briefing.
      Probe lines over 0.5 get flagged `<- injection?` in the display.
- [ ] **Poke at it (15 min).** Run your own prompts as the third argument, with and without injections.
      Note any false alarms. That's milestone 7's calibration problem all over again.
- [ ] **Wrap up.** Re-entry note in Notes.md, then commit and push.

## Done when
- [ ] `cargo test --bin probetrain` (4) and `cargo test --bin ch10write` (25) all pass
- [ ] The Whispering Injector is defeated
- [ ] `probe.bin` is trained on real hidden states, and `target/probe_run.c10` caught an injection

## Notes
- The training documents come from templates, so expect a rough probe: it may react to command-like
  wording in harmless text. More varied data is the fix, and that's a good parking-lot item.
- Probe readings for the whole prompt arrive before the model's first token. That's why
  `probe_packets_for` can start a recording.
- I couldn't run the Python scripts here because there's no GPU or model download. The Rust side,
  the file formats both scripts share, and the two-channel UDP recording are tested.
