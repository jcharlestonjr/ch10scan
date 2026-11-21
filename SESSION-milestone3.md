# Session: milestone 3 (AI confidence readings into Ch10)

**Where you left off:** ch10write writes a TMATS-only file, and ch10scan reads it back.
**Today's goal:** one Ch10 file with TMATS plus one packet per token from the 4060 run, read back cleanly by ch10scan.
**Time box:** about 75 minutes.

Progress: milestone 1 (confidence channel) done, milestone 2 (TMATS writer) done, **milestone 3 today**.

## Steps

- [ ] **Get the update (5 min).** Commit your finished step-1 work first, then:
      `git pull --rebase`
      If git reports a conflict, stop there and send me the message.
- [ ] **Build (3 min).** `cargo build --bin ch10write`
      The first build downloads the `csv` crate. That's the only new dependency.
- [ ] **See the starting line (2 min).** `cargo test --bin ch10write`
      Expect 6 passed and 3 failed ("not yet implemented"). Your five step-1 tests pass,
      `reads_sample_csv` passes (that part is pre-filled), and the 3 new ones fail.
- [ ] **Skim the new pieces (5 min).** The `Reading` struct, `read_readings`, and the
      Milestone 3 section in `src/bin/ch10write.rs`.
- [ ] **YOUR PART 3: `rtc_from_ns` (5 min).** This is the warm-up.
      Done when `cargo test --bin ch10write rtc_` passes.
- [ ] **YOUR PART 4: `build_confidence_packet` (20 min).**
      Done when `cargo test --bin ch10write confidence_packet_layout` passes.
- [ ] **YOUR PART 5: the loop in `main` (15 min).**
- [ ] **Write the file (2 min).**
      `cargo run --bin ch10write -- confidence_sample.csv target/ai_conf.c10`
      Expect: `wrote 2516 bytes to target/ai_conf.c10`
- [ ] **Read it back (5 min).** `cargo run -- target/ai_conf.c10`
      Expect:
      ```
      TMATS: 167 bytes -> tmats.txt
      ch   0  type 0x00  Computer Generated F0 rtc 0.000001  packets  58  bytes 2320
      ch   0  type 0x01  Computer Generated F1 rtc 0.000000  packets   1  bytes 196
      OK: all 2516 bytes accounted for (0 skipped)
      ```
      Then `git restore tmats.txt`.
- [ ] **Wrap up.** Write the re-entry note in Notes.md, then commit and push.

## Done when
- [ ] `cargo test --bin ch10write`: 9 passed
- [ ] ch10scan shows 58 F0 packets and "OK: all 2516 bytes accounted for"

## Notes
- `confidence_sample.csv` is the CSV from your first 4060 run (the IRIG one), so the
  numbers above are exact. Your own new runs will give different counts.
- **Known ch10scan quirk (parked):** the `rtc` column turns each packet's time into whole
  seconds and adds them up, so it shows nonsense like `0.000001`. That isn't your writer.
  The tests check your RTC values directly. The last reading should land around 1.137 s.
- Why `seq` starts at 1: sequence numbers count per channel, and the TMATS packet
  already used 0 on channel 0.

## Next session (don't start it today)
Pick one: add a time packet so RTC maps to real clock time, or have the Python script
stream straight into ch10write instead of going through a CSV.

## Parking lot
Agent harness, envelope expansion, SPARK kill switch, PCM ground-station stretch goal, bigger model,
independent reader check (irig106.org tools), fixing ch10scan's rtc column, moving from user-defined
F0 to a proper data channel defined in TMATS.
