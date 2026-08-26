---
name: run-game
description: "Build and launch VERA20k normally as a player would. Use when the user asks to run, start, or play the game; always uses the current branch's release build with no quickplay, capture, debug, or developer mode."
---

# Run VERA20k normally

Open the ordinary game menu from the current task branch and leave the game
running for the user. This is an interactive player session, not a test or
automated validation mode.

## Required behavior

1. Resolve the current checkout with `git rev-parse --show-toplevel`. Build that
   checkout so an unmerged feature is not accidentally tested against `main`.
2. Check `Get-Process cargo,rustc,vera20k -ErrorAction SilentlyContinue` first.
   Wait if Cargo or rustc is active. If the game is already running, report that
   instance instead of starting another one.
3. Build only the normal release executable:

   ```powershell
   cargo build --release -p vera20k --bin vera20k
   ```

4. Use the primary worktree as the launch working directory so its ignored
   `config.toml` supplies the local retail-data path, while launching the
   executable built from the current checkout. Preserve `RA2_DIR` when it is
   the configured retail-data source.
5. Launch with every other inherited `RA2_*` environment variable removed from
   the child process. This keeps `RA2_QUICKPLAY`, capture settings, developer
   shells, debug spawns, and asset overrides out of a normal run. Restore the
   caller's environment immediately after starting the child.
6. Confirm the process opens a responsive visible window, then leave it running
   and tell the user the release build and branch that were launched.

Never substitute a debug build, quickplay map, scripted capture, test harness,
or automated input. Do not take screenshots, inspect gameplay, or claim parity
unless the user separately asks for those actions.
