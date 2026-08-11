---
layout: default
title: VERA20k Contributor Roadmap
---

# VERA20k Contributor Roadmap

**Rebuild Yuri's Revenge faithfully in Rust, then take it beyond the original engine's limits.**

> **North star:** an experienced Yuri's Revenge player can complete an ordinary 30-60 minute stock skirmish, with any faction on representative retail maps, without repeatedly noticing differences in gameplay, visuals, sound, or response.

These are target outcomes, not current-status claims or completion percentages.

## Where we are going

1. **Trust retail content.** Load the player's legally owned archives, standalone MD INIs and verified overrides, maps, text, graphics, audio, and movies correctly.

2. **Finish a convincing stock skirmish.** Go from skirmish setup to battlefield to result, with familiar control, combat, economy, AI, presentation, and sound.

3. **Complete the Yuri's Revenge battlefield.** Support Allied, Soviet, and Yuri identity, specialist units, naval and aircraft play, superweapons, stock scripts, and common map mechanics.

4. **Preserve and connect matches.** Save, restore, replay, diagnose, and play across real machines without changing deterministic game state.

5. **Reach VERA scale.** Prove responsive, deterministic battles at up to 30 players and 20,000 units with measured benchmarks.

6. **Go beyond retail.** Add new modes and features only behind explicit boundaries that cannot silently change stock behavior.

The first two outcomes are the main delivery path. Persistence, multiplayer, presentation, and performance work can advance independently when their boundaries are clear.

## The first complete skirmish

Close this journey in order:

1. **Choose and load** - select a stock setup, load the scenario, and create houses, starts, and forces.
2. **Advance a stable first frame** - preserve native-compatible command, scheduler, presentation, and late-frame timing.
3. **Move and reveal** - select, command, path, locomote, update occupancy, and reveal terrain.
4. **Attack and die** - target, fire, apply damage, resolve lifecycle consequences, and present feedback.
5. **Harvest and earn** - choose work, collect, return, dock, deposit, and release.
6. **Build and recover power** - fund production incrementally, place or exit the result, and update power and radar.
7. **Finish and return** - let AI, teams, scripts, and triggers reach a result and return through the skirmish shell.

The gate is one production-route stock skirmish that runs for 30-60 minutes with deterministically reproducible state evolution and no repeatedly noticeable ordinary-play divergence.

## Pick a contribution

- **Good first contributions:** documentation, setup, diagnostics, tools, legal synthetic fixtures, parser boundaries, and focused tests.
- **With engine experience:** maps, UI, rendering, audio, rules, and one reproduced gameplay divergence.
- **Advanced:** persistence, replay, AI, specialist mechanics, and cross-system behavior.
- **Maintainer-paired:** scheduler order, lifecycle ownership, synchronized randomness, networking, hashes, and hot-path optimization.

If the curated issue lists are empty, agree on one bounded slice in Discord before coding.

## Work one small slice

1. Start from an observed player problem or an accepted contributor issue.
2. Reproduce one route and find the smallest Rust owner.
3. Establish retail data, native evidence, or an explicit **unverified** label.
4. Create a short-lived `feature/<topic>` branch before editing.
5. Fix the first player-visible or determinism-relevant divergence.
6. Run the focused check and the production route, record what remains unknown, then stop.

### Ready for review

- The outcome is stated in plain language.
- The change preserves deterministic order, identity, randomness, and native same-frame/tick consequence timing where relevant.
- Behavioral work has a focused regression test; visible work exercises the production path.
- Pixel/frame-parity visual work has a capture, parser work a boundary fixture, and performance work a profile.
- No retail binary, archive, map, key, or extracted asset is committed.

### Useful commands

```bash
cargo check -p vera20k
cargo test -p vera20k --lib module_path::
cargo run --bin vera20k
```

### Start here

- [Good first issues](https://github.com/Yrvera/vera20k/issues?q=is%3Aissue%20state%3Aopen%20label%3A%22good%20first%20issue%22)
- [Discord](https://discord.gg/kmjRUn5m5F)
- [README and setup](https://github.com/Yrvera/vera20k/blob/main/README.md)
- [Authoritative Rust module map](https://github.com/Yrvera/vera20k/blob/main/src/lib.rs)
- [Current pull requests](https://github.com/Yrvera/vera20k/pulls)
