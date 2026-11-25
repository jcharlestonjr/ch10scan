# Session: milestone 7 (Calibration Caverns)

**Where you left off:** a live ground-station display, and a recording made on the Fedora box.
**Today's goal:** calibrate the confidence channel. When entropy reads high, how often is the model actually
wrong, and how sure can you be of that number?
**Time box:** about 75 minutes, plus a few minutes of GPU time.

## The idea in one paragraph
Every instrument gets a calibration and an error band. Here the "known inputs" are questions with computed
answers (multiplication, addition) and a short list of capitals. The model answers each one, your sensor
records how unsure it was, and a grader marks it right or wrong. AUROC says how well a channel separates
wrong answers from right ones. The reliability table says what error rate to expect at each reading, with a
95% band that's honest about small samples.

## Steps

- [ ] **Get the update (2 min).** `git pull`
- [ ] **See the starting line (2 min).** `cargo test --bin ch10cal`, expect 5 failed.
- [ ] **YOUR PART 16: `auroc` (20 min).** Done when `cargo test --bin ch10cal cal_auroc` passes (2 tests).
- [ ] **YOUR PART 17: `reliability` (25 min).** Done when `cargo test --bin ch10cal cal_reliability` passes (2 tests).
- [ ] **YOUR PART 18: `wilson` (15 min).** Done when `cargo test --bin ch10cal cal_wilson` passes.
- [ ] **Try the report (2 min).** `cargo run --bin ch10cal -- samples/calib_synthetic.csv`
      That file is made-up data, just to see the report's shape.
- [ ] **Stopping point.** Fight The Overconfident Sage in the game.
- [ ] **The real calibration run (15 min, mostly waiting).** In WSL with the venv active:
      ```
      cd ~/ch10scan
      python python/calibrate.py calib_results.csv
      cargo run --bin ch10cal -- calib_results.csv
      ```
      120 questions by default. The script prints each one as ok or WRONG as it goes.
- [ ] **Read your report.** Which channel has the best AUROC? Does the error rate climb with the reading?
      Where are the bands wide? Rescan the game for Calibrated, and Bluff detector if the best AUROC is 0.70 or more.
- [ ] **Wrap up.** Re-entry note in Notes.md, then commit and push.

## Done when
- [ ] `cargo test --bin ch10cal`: 5 passed
- [ ] The Overconfident Sage is defeated
- [ ] `calib_results.csv` has 100+ real graded answers

## Reading the numbers
- AUROC 0.5 is a coin flip, 0.7 or more is useful, and 0.8 or more is strong.
- For short answers, first-token entropy often wins, because the first token is where the model commits.
- A bin with 4 answers and a band from 5% to 70% is telling you to collect more data, not that the channel is bad.
