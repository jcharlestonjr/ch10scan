# Range Quest

Your ch10write progress as a game. XP only comes from things the computer checks:
tests passing, files existing, your own commits, and byte-for-byte comparisons.
You can't fake it, and it can't drift out of date.

## Start it

In WSL, from the repo root:

```
python3 game/serve.py
```

Then open http://localhost:8642 in your Windows browser. The first scan compiles and
runs your tests, so it takes a few seconds. Leave the terminal running while you play,
and press Ctrl+C to stop it.

If the page won't load from Windows, restart it with `python3 game/serve.py --host 0.0.0.0`
and try the same address again. If VS Code offers "Open in Browser" for port 8642, that works too.

## Playing

- **Map:** click a zone to see its quests and boss. Each zone is a milestone.
- **Briefing:** the YOUR PART hints, pulled straight from the comments in your code.
- **Fight:** runs your tests live. Each passing check hits the boss and drops its
  signal on the strip chart. Each failing check blocks you and shows the real failure message.
- **Rescan:** after you change code, to update XP without a fight.
- **Next quest:** always the single next thing to do, with the command to check it.

## How XP works

- **Quests:** each YOUR PART, earned when its tests pass. A few older ones count your commit.
- **Bosses:** earned when every check in the fight passes, including any final end-to-end check.
- **Achievements:** 50 XP each.
- **Streak:** weeknights in a row with a commit of yours. Weekends don't break it.

## What a scan does

It runs `cargo test --bin ch10write` and `cargo build`. Once Live Wire Pass is ready, it also
replays `confidence_sample.csv` over UDP into `--udp` mode in a temp folder and compares the result
with CSV mode. It runs ch10scan on `target/model.c10` (if you've recorded one) in a temp folder,
and reads `git log`. It never changes your repo, and it never runs anything the page sends it.

Level-up and achievement pop-ups compare against your last visit, which your browser remembers.

When a new milestone is set up, its zone gets added to `quests.json` and opens on the map.
