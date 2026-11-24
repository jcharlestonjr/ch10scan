# Session: milestone 4 (time packet)

**Where you left off:** ch10write turns confidence.csv into TMATS plus 58 F0 packets, and ch10scan reads all 2516 bytes.
**Today's goal:** add a Time Data Format 1 packet so the file's RTC maps to real clock time.
**Time box:** about 75 minutes.

Progress: milestones 1-3 done, **milestone 4 today**.

## The idea in one paragraph
The RTC is a free-running 10 MHz counter that only says how far apart packets are.
A time packet pins it to the clock: "at RTC X, the time was T." From that one packet,
any other packet's RTC converts to real clock time. That's how your sample files work too:
in discrete.c10, time packets land on whole seconds, about 10,000,000 ticks apart.

## Steps

- [ ] **Get the update (2 min).** `git pull`
      If it complains about local changes, run `git restore tmats.txt` or commit, then pull again.
- [ ] **See the starting line (2 min).** `cargo test --bin ch10write`
      Expect 11 passed and 5 failed. Your 9 pass, the 2 pre-filled ones (`utc_parts`, leap years)
      pass, and the 5 new ones fail with "not yet implemented".
- [ ] **Skim the new pieces (5 min).** `UtcParts`, `utc_parts` (pre-filled calendar math),
      and the Milestone 4 section in `src/bin/ch10write.rs`.
- [ ] **YOUR PART 6: `bcd_doy_words` (25 min).**
      Done when `cargo test --bin ch10write bcd_` passes (3 tests).
      Two of them are copied from real recorder time packets in discrete.c10 and event.c10.
- [ ] **YOUR PART 7: `build_time_packet` (20 min).**
      Done when `cargo test --bin ch10write time_packet` passes (2 tests: layout and leap year).
- [ ] **YOUR PART 8: anchor the clock in `main` (10 min).**
- [ ] **Write the file (2 min).**
      `cargo run --bin ch10write -- confidence_sample.csv target/ai_time.c10`
      Expect:
      ```
      time anchor: 2026 day 324 06:17:01 UTC
      wrote 2580 bytes to target/ai_time.c10
      ```
      06:17 UTC is 10:17 PM Pacific, the night you ran it.
- [ ] **Read it back (5 min).** `cargo run -- target/ai_time.c10`
      Expect:
      ```
      TMATS: 196 bytes -> tmats.txt
      ch   0  type 0x00  Computer Generated F0 rtc 0.000006  packets  58  bytes 2320
      ch   0  type 0x01  Computer Generated F1 rtc 0.000000  packets   1  bytes 224
      ch   1  type 0x11  Time               F1 rtc 0.000000  packets   1  bytes 36
      OK: all 2580 bytes accounted for (0 skipped)
      ```
      Then `git restore tmats.txt`.
- [ ] **Wrap up.** Write the re-entry note in Notes.md, then commit and push.

## Done when
- [ ] `cargo test --bin ch10write`: 16 passed
- [ ] ch10scan shows the ch 1 Time F1 packet and "OK: all 2580 bytes accounted for"

## Notes
- **Why whole seconds:** real recorders put time packets on whole seconds. So the anchor is the
  first token's time rounded down, and the first reading lands at about RTC 0.998 s.
- **Day-of-year format has no year in it.** The year comes from context, which is why the
  leap-year bit (CSDW bit 8) exists.
- **Time source flags:** FMT = internal real-time clock, SRC = internal. That's the honest
  description of a PC clock kept in sync by NTP, and it's the same FMT/SRC as ethernet.c10.
- The F0 `rtc` column changes from 0.000001 to 0.000006. That's still the parked ch10scan bug.
- **What I checked:** using only the finished file's time packet and RTCs, I rebuilt every token's
  clock time. All 58 came out within 98 ns of the CSV timestamps, under one 100 ns tick.

## Next session (don't start it today)
Pick one: stream live from the Python script instead of going through a CSV, or emit a time
packet every second like a real recorder.

## Parking lot
Agent harness, envelope expansion, SPARK kill switch, PCM ground-station stretch goal, bigger model,
independent reader check (irig106.org tools), fixing ch10scan's rtc column, a proper TMATS data
channel instead of F0, TMATS entries for the time channel, merging the branch into main.
