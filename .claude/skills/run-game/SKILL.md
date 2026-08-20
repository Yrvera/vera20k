---
name: run-game
description: "Build and launch the VERA20k game binary for a live look — a normal play session, a quickplay map, or a scripted verify. Use whenever the user asks to run, start, play, or screenshot the game, or wants a change confirmed in the running engine rather than in tests. Covers the release-profile rule, the RA2_QUICKPLAY modes, and how to observe a run without ruining it."
---

# Run VERA20k

Launching the engine so a human can judge it. Tests answer "is it correct";
this answers "does it look and feel right", which is the only check that
catches frame pacing, sprite placement, and audio.

## The rule that matters most

**Build `--release`.** `[profile.dev]` in `Cargo.toml` is `opt-level = 1` — a
deliberate build-speed compromise, not a play profile. A debug binary is
visibly slow even sitting on the menu, and it makes the judgement the user
wanted worthless: nobody can tell whether a tracer leaves the barrel correctly
at the wrong frame rate.

Never hand over a debug binary for a live look. If there is a reason to run
one, say so before launching.

## Launch

```bash
powershell -NoProfile -Command "(Get-Process cargo,rustc,vera20k -ErrorAction SilentlyContinue | Measure-Object).Count"
cargo build --release -p vera20k --bin vera20k
./target/release/vera20k.exe
```

- The process check is not optional — ENGINE.md forbids running cargo in
  parallel from one session, and a second game instance fights for the audio
  device.
- Run from the **primary checkout**. Retail assets resolve through the
  configured install path, which is machine-local and recorded in `LOCAL.md`;
  a worktree without that config will fail to find them.
- Both builds take roughly two minutes on this machine. Start it in the
  background and wait on the process count rather than polling the log.

## Launch modes — state which one you are using

`RA2_QUICKPLAY` selects what the engine boots into. Pick deliberately and tell
the user, because the mode decides what the run can prove.

| Mode | Command | Proves |
|---|---|---|
| Normal | no env var | Boot, assets, shell UI, and a real play session. The default for "run the game". |
| Map | `RA2_QUICKPLAY=<name>.map\|.mpr\|.mmx` | Straight into that map, no menu clicks. |
| Random map | `RA2_QUICKPLAY=<name>.sed` | Exercises the map generator without the skirmish UI. |

`minerloop.map` is the zero-interaction harvester fixture — a pre-placed
refinery and miner, useful for confirming the economy loop without touching the
mouse. It proves nothing about combat.

`RA2_QUICKPLAY` must be **unset** for shell capture; `app/diagnostics/
shell_capture.rs` asserts on it, and `tactical_capture` carries a wider
environment denylist. Check that file before setting any `RA2_*` variable
alongside a capture.

## Confirm it actually came up

```bash
powershell -NoProfile -Command "Get-Process vera20k -ErrorAction SilentlyContinue | Select-Object Id,@{n='RSS_MB';e={[math]::Round($_.WorkingSet64/1MB)}},@{n='Title';e={$_.MainWindowTitle}},Responding | Format-List"
```

A healthy launch reports `Title : RA2 Engine`, `Responding : True`, and a few
hundred MB resident. A process that exists but reports an empty title has not
opened its window yet — wait rather than concluding it works.

**Stdout is nearly empty by design.** Real logging goes to `logs/ra2.log`. Two
rodio `DeviceSink` drop notices at shutdown are cosmetic. Grep the log file,
not the console, when something looks wrong.

## Observing without ruining the run

The window takes focus. A live observation needs the machine left alone —
Windows `ForegroundLockTimeout` means roughly 200 seconds of idle before
foreground behaviour settles, and typing during that window makes captured
frames worthless. Say so before launching so the user is not surprised.

Screenshot, verified working:

```bash
powershell -NoProfile -Command "Add-Type -AssemblyName System.Windows.Forms,System.Drawing; \$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \$bmp = New-Object System.Drawing.Bitmap \$b.Width, \$b.Height; \$g = [System.Drawing.Graphics]::FromImage(\$bmp); \$g.CopyFromScreen(\$b.Location, [System.Drawing.Point]::Empty, \$b.Size); \$bmp.Save('<out>.png'); \$g.Dispose(); \$bmp.Dispose()"
```

Then **look at the image**. A blank or black frame is a failed launch, not a
successful one.

## What a run does and does not settle

Booting to the menu proves startup, asset loading, and the shell UI. It proves
nothing about simulation — veterancy, fire gates, death effects, and projectile
behaviour all need units actually shooting each other, which means a real
skirmish on a real map.

Defer to the user on anything about feel or appearance. Their eye on a live
frame outranks any analysis of a screenshot; when they report something looks
wrong, dig, do not explain why the code should be fine.
