---
name: run-game
description: "Build and launch the current branch's normal release game when asked to run, start, or play VERA20k."
---

# Run the game

Open the ordinary menu and leave a visible player session running.

1. Resolve the checkout with `git rev-parse --show-toplevel`.
2. Check `Get-Process cargo,rustc,vera20k -ErrorAction SilentlyContinue`.
   Wait for active builds. Report an existing game instance instead of duplicating it.
3. Build this checkout:

   ```powershell
   cargo build --release -p vera20k --bin vera20k
   ```

4. Launch that executable with the primary worktree as working directory, so its
   ignored `config.toml` supplies retail-data configuration. Preserve `RA2_DIR`
   when it supplies the configured data path.
5. Remove every other inherited `RA2_*` variable from the child environment;
   restore any temporarily changed caller environment immediately after launch.
6. Confirm a responsive visible window, leave it running, and report build/branch.

Do not substitute debug, quickplay, capture, developer mode, a test harness, or
automated input. Gameplay inspection, screenshots, and parity claims require their
own requested scope.
