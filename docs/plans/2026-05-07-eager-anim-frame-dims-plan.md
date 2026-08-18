# Eager Init of ArtEntry frame_width/frame_height — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Eagerly populate per-anim SHP frame dimensions on `ArtEntry` at startup, eliminating the `(30, 30)` default fallback in `try_dispatch_anim_smudge` and unblocking 2×2-crater selection for V3-class explosions.

**Architecture:** Add two `u16` fields to `ArtEntry`. After ArtRegistry construction in `app_init.rs`, walk smudge-flagged anims (Crater/Scorch/ForceBigCraters), load their SHP frame 0, store frame_width/height. Replace `DEFAULT_ANIM_FRAME_DIM` in the smudge dispatcher with the populated values. Establishes a small new `rules/` → `AssetManager` dependency (one method, layering-correct).

**Design Doc:** Brainstorm output recorded inline in conversation 2026-05-07 (no separate design doc per the user-preferred workflow established in the smudge atlas session).

---

## Grounding Summary

**Docs:**
- [ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md](ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md) — ledger #8/#9 establish that gamemd's non-ForceBig anim path passes `(AnimType+0x29C, AnimType+0x2A0)` as dmg/dmg2, lazy-cached from SHP frame 0; default fallback is 30/30.
- [docs/plans/2026-05-06-smudge-system-design.md](docs/plans/2026-05-06-smudge-system-design.md) — explicitly flagged eager AnimType frame-dim init as deferred follow-up #1.

**Ghidra-verified (this session):**
- `AnimClass::Start @ 0x424F00` — reads `AnimType+0x29C` and `AnimType+0x2A0`; lazy-populates from `SHP_frame_rect_getter(rect, AnimType+0x298)` if `-1` (uncached).
- `SHP_frame_rect_getter @ 0x69E7E0` — reads 4 shorts at `frame_table + frame_idx*0x18 + 0`: `[frame_x, frame_y, frame_width, frame_height]`. Width is field 2, height is field 3.
- AnimType+0x298 is the `Start` frame; verified zero overlap of `Start=` (217 anims) with `Crater=`/`Scorch=`/`ForceBigCraters=` (~30 anims) — so for our scope, **frame 0 is always correct**.

**Repo pattern this mirrors:**
- [src/render/sprite_atlas.rs:453-477](src/render/sprite_atlas.rs#L453-L477) — exact load-SHP-and-read-header template: resolve image_id, build candidates, iterate, parse SHP, read frame data. Our populate method follows this pattern.
- [src/render/overlay_atlas.rs:622-651](src/render/overlay_atlas.rs#L622-L651) — `probe_terrain_shp_frame_count` is the closest semantic analog (load SHP header, return derived value).
- [src/rules/art_data.rs:618](src/rules/art_data.rs#L618) — `anim_shp_candidates` already exists; we call it directly. No new candidate-builder code.

**INI keys driving behavior:**
- `artmd.ini` per-anim: `Crater=`, `Burn=`, `ForceBigCraters=`, `Image=`, `Theater=`, `NewTheater=`. All already parsed into `ArtEntry`.
- No new INI keys.

**Unknowns after grounding:**
- None blocking. The "what frame index?" question was the only material unknown; resolved to "frame 0 is always correct for smudge-flagged anims" via the Start= cross-check above. Edge case (modded SmudgeType anim with Start>0 AND Crater=yes) is out of scope; note as known limitation.

## Key Technical Decisions

- **Approach (A) from brainstorm: post-construction `populate_anim_frame_dims` method on `ArtRegistry`** — **Confidence:** high. **Source:** brainstorm 2026-05-07; layering-correct (rules→assets); minimal API surface (one method, one new dep).
- **Selective population: only crater/scorch/force_big anims** — **Confidence:** high. **Source:** smudge_dispatch.rs:100,109,125 — only these flags reach the frame-dim path. Skipping non-smudge anims saves ~970 SHP header reads at startup.
- **SHP load failure → keep (30, 30) defaults** — **Confidence:** high. **Source:** ledger #9; matches gamemd's "uncached first-call" fallback.
- **Frame index = 0** — **Confidence:** high. **Source:** Ghidra cross-check confirmed zero overlap of Start= with smudge flags.
- **Field type: `u16`** — **Confidence:** high. **Source:** `ShpFrame.frame_width: u16` matches gamemd's stored short read.
- **Default value: 30 (constant `DEFAULT_ANIM_FRAME_DIM`)** — **Confidence:** high. **Source:** ledger #9; existing constant in `smudge_dispatch.rs:18` (or similar — to be reused at `ArtEntry` default).
- **Theater extension: pass map's theater_ext from app_init** — **Confidence:** high. **Source:** `map_data.header.theater` available at `app_init.rs:231`. Anim SHP dimensions are theater-independent for typical content (visual shape stable across .tem/.sno/.urb), so the theater choice doesn't drift the dims.

## Open Questions

### Resolved during planning
- **Frame index gamemd reads:** frame 0 (Ghidra-verified above).
- **Frame rect format:** `frame_width` (field 2) and `frame_height` (field 3) of the per-frame rect, not canvas dims.
- **Where does the populate method live?** On `ArtRegistry` in `src/rules/art_data.rs` (Approach A). Alternative B (app-layer free function) rejected for absorbing gameplay logic into app_init per CLAUDE.md.
- **Test helper at `shp_vehicle_sequence.rs:97`:** confirmed exists per smudge-plan revision history (2026-05-06 Task 2 subagent log). Needs the same default updates.

### Deferred to implementation
- None.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/art_data.rs` | Add `frame_width: u16, frame_height: u16` (default 30) to `ArtEntry`; add `pub fn populate_anim_frame_dims(&mut self, asset_manager, theater_ext, theater_name)` method on `ArtRegistry` |
| Modify | `src/rules/shp_vehicle_sequence.rs:97` | Update `make_art_entry` test helper to set new fields to 30 |
| Modify | `src/app_init.rs` | Call `populate_anim_frame_dims` between `merge_art_data` and `r.art_registry = a.clone()`; mutate the source `art` not the clone so both sides see populated dims |
| Modify | `src/sim/combat/smudge_dispatch.rs` | Replace `DEFAULT_ANIM_FRAME_DIM` constant lookups with `entry.frame_width as i32` / `entry.frame_height as i32`; remove the constant if no longer used |

## Interface Changes

- **`ArtEntry` gains 2 fields** (`frame_width: u16, frame_height: u16`). Public struct. Constructors:
  - Real constructor at `src/rules/art_data.rs` (around line 211+, inside `ArtRegistry::from_ini`'s parse loop) — initialize to 30 (or `DEFAULT_ANIM_FRAME_DIM` if reused as a public const).
  - Test helper `make_art_entry` at `src/rules/shp_vehicle_sequence.rs:97` — initialize to 30.
  - `cargo build` will catch any other constructors via Rust's "all fields required" rule.
- **`ArtRegistry` gains `populate_anim_frame_dims` method.** New public method; one call site (app_init.rs).
- **No changes to `try_dispatch_anim_smudge` signature or behavior contract.** Only the source of dmg/dmg2 changes from a constant to per-entry data.

## Sim Checklist

This plan touches `src/sim/combat/smudge_dispatch.rs` — Task 4. Sim/ checklist:

- [x] All math uses `fixed`-point or integer — `entry.frame_width as i32` is integer; threshold check `0x3C < dmg` is unchanged.
- [x] No new state added to deterministic state hash — `ArtEntry.frame_width/height` is read-only post-init data on `RuleSet`, not gameplay state.
- [x] No new dependencies on render/ui/sidebar/audio/net — sim still reads only from `&RuleSet`.
- [x] Tick ordering impact: none — `try_dispatch_anim_smudge` already runs in the existing combat→drainer phase.
- [x] BTreeMap iteration order: N/A — populate iterates ArtRegistry's internal HashMap; iteration order can be non-deterministic but that's fine because each entry's value is independently populated (no cross-entry effects).

## Risk Areas

1. **Determinism — read order independence.** `populate_anim_frame_dims` iterates the ArtRegistry's HashMap. Iteration order is non-deterministic across runs, but each entry's frame dims are read independently from the SHP file (deterministic by content). No cross-entry state. Final ArtRegistry state is bit-identical regardless of iteration order. Verified by inspection.

2. **SHP load failure path.** If a smudge-flagged anim's SHP is missing or malformed, the populate method must keep the 30/30 defaults (matching gamemd's uncached fallback). Test via in-game observation of whichever anims fail to load (debug-log them).

3. **Theater-ext correctness.** Anim SHPs may exist as `name.tem`/`name.sno`/`name.urb` for theater-keyed types (`Theater=yes`). The existing `anim_shp_candidates` helper handles this — we call it unchanged. Verified by inspection of [src/render/sprite_atlas.rs:458-464](src/render/sprite_atlas.rs#L458-L464) which uses the same helper successfully today for SHP frame-count probing.

4. **Constructor coverage.** `ArtEntry` is constructed in (a) `art_data.rs` parser and (b) `shp_vehicle_sequence.rs:97` test helper. Rust's struct initialization rules will produce a compile error for any uncovered constructor. `cargo build` is the safety net.

5. **Cloned ArtRegistry in RuleSet.** The line `r.art_registry = a.clone()` clones the populated ArtRegistry into RuleSet. Must populate `a` BEFORE the clone so both sides see the dims.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| Task 4 | dmg/dmg2 = `entry.frame_width / frame_height` (not `DEFAULT=30`) | Eliminates the size-filter rejection for V3-class explosions — TWLT070 (et al.) have visible content rect ~120×96, threshold check `0x3C < 120 AND 0x32 < 96` passes → 2×2 craters become selectable. With (30, 30), check fails → only 1×1 craters. | In-game V3 strike kills target on temperate ground; observe 2×2 crater appearing post-fix vs 1×1 pre-fix. |
| Task 2 | SHP-load failure → keep 30/30 defaults | Matches gamemd's uncached fallback. Without this, missing-asset anims would crash or panic. | Force-corrupt one SHP filename in test (or rely on in-game robustness — log a debug warning per failed anim). |
| Task 2 | Read frame **0** (not Start=) | Ghidra confirmed zero overlap of Start= with smudge flags in retail data; using frame 0 is correct for our scope. | Already verified in grounding phase; no further check needed. |
| Task 2 | Read frame's `frame_width`/`frame_height` (not SHP canvas `width`/`height`) | gamemd reads the visible content bounding rect, not the canvas envelope. Wrong field → over-sized values → all anims would always pass the threshold (over-correction; would pick big smudges where gamemd picks small for low-damage anims). | Code review: confirm we read `shp.frames[0].frame_width`, NOT `shp.width`. |
| Task 3 | Populate BEFORE the `r.art_registry = a.clone()` line in app_init.rs | If populate runs after the clone, dispatcher reads from the un-populated copy on RuleSet → defaults to 30 silently. | Code review: trace the assignment order in app_init.rs:268. |

---

## Tasks

### Task 1: Add frame_width/frame_height fields to ArtEntry

**Why:** Foundation. Defining the data slot before populating it. Two constructors must set the new fields; Rust's compile-time check ensures we don't miss any.

**Files:**
- Modify: `src/rules/art_data.rs` (struct + parser)
- Modify: `src/rules/shp_vehicle_sequence.rs` (test helper at line 97)

**Pattern:** Mirrors the prior smudge-system Task 2 pattern (extending `ArtEntry` with new fields, updating both constructors).

**Step 1: Add fields to ArtEntry struct**

Locate the `pub struct ArtEntry` block at [src/rules/art_data.rs:19](src/rules/art_data.rs#L19). After the existing smudge-flag fields (`scorch`, `crater`, `force_big_craters` around line 32-34), insert:

```rust
    /// SHP frame 0's visible-content bounding-rect width, in pixels.
    /// Used by the smudge dispatcher as a damage-tier proxy for size selection.
    /// Default 30 — matches the original engine's uncached first-call fallback;
    /// replaced with the actual SHP frame width by `populate_anim_frame_dims`
    /// if the anim has a Crater/Scorch/ForceBigCraters spawn flag.
    pub frame_width: u16,
    /// SHP frame 0's visible-content bounding-rect height, in pixels.
    /// See `frame_width`.
    pub frame_height: u16,
```

**Step 2: Set the defaults in the parser**

Locate the `types.push(ArtEntry { ... })` block inside `ArtRegistry::from_ini` at [src/rules/art_data.rs](src/rules/art_data.rs) (search for "force_big_craters," to find the existing initializer near line 358). Add the two new fields with the default value `30`:

```rust
            // ... existing initializer fields ...
            force_big_craters,
            frame_width: 30,
            frame_height: 30,
            // ... remaining fields ...
```

**Step 3: Update the test helper**

Locate `make_art_entry` at [src/rules/shp_vehicle_sequence.rs:97](src/rules/shp_vehicle_sequence.rs#L97). Add the same two field defaults:

```rust
        // ... existing fields ...
        force_big_craters: false,
        frame_width: 30,
        frame_height: 30,
        // ... remaining fields ...
```

**Step 4: Verify**

Run: `cargo check --package vera20k`
Expected: PASS. If you see "missing field `frame_width` in initializer" or similar, locate the additional ArtEntry constructor it's pointing at, set the same defaults there, and re-run.

If new constructors are surfaced beyond `art_data.rs` and `shp_vehicle_sequence.rs:97`, **STOP and report** before adding more — the prior smudge-plan execution caught two such drifts. The plan should not silently expand its file map.

**Step 5: Commit**

`rules: add frame_width/frame_height fields to ArtEntry (default 30)`

---

### Task 2: Add populate_anim_frame_dims method on ArtRegistry

**Why:** This is the core enrichment step. Walks smudge-flagged anims, loads their SHP, reads frame 0's bounding rect, stores width/height on the entry. Falls back to 30/30 silently when the SHP is unavailable.

**Files:**
- Modify: `src/rules/art_data.rs` (new method on ArtRegistry)

**Pattern:** Mirrors the load-SHP-and-read-header pattern at [src/render/sprite_atlas.rs:453-477](src/render/sprite_atlas.rs#L453-L477).

**Step 1: Add the method**

Find the `impl ArtRegistry` block in `art_data.rs` (the one containing `from_ini`, `get`, etc). After the existing methods, insert:

```rust
    /// Eagerly populate `frame_width`/`frame_height` on entries whose anim
    /// has a smudge-spawn flag (Crater/Burn/ForceBigCraters). Reads frame 0
    /// of each anim's SHP via the shared `anim_shp_candidates` filename
    /// pipeline. Anims without a loadable SHP keep the (30, 30) defaults
    /// from their initial parse.
    ///
    /// Returns `(populated, fallback)` for diagnostic logging:
    ///   `populated` = anims whose SHP was found and dims were stored
    ///   `fallback`  = smudge-flagged anims whose SHP failed to load
    pub fn populate_anim_frame_dims(
        &mut self,
        asset_manager: &crate::assets::asset_manager::AssetManager,
        theater_ext: &str,
        theater_name: &str,
    ) -> (u32, u32) {
        let mut populated: u32 = 0;
        let mut fallback: u32 = 0;

        // Collect (name, image_id) pairs first to satisfy the borrow checker:
        // we read &self via resolve_effective_image_id then mutate &mut self
        // when writing the dims.
        let pending: Vec<(String, String)> = self
            .iter_entries()
            .filter(|(_name, entry)| entry.crater || entry.scorch || entry.force_big_craters)
            .map(|(name, _entry)| {
                let image_id: String = self.resolve_effective_image_id(name, name);
                (name.to_string(), image_id)
            })
            .collect();

        for (name, image_id) in pending {
            let candidates: Vec<String> = crate::rules::art_data::anim_shp_candidates(
                Some(self),
                &name,
                &image_id,
                theater_ext,
                theater_name,
            );
            let shp_bytes: Option<&[u8]> = candidates
                .iter()
                .find_map(|c| asset_manager.get_ref(c));
            let Some(data) = shp_bytes else {
                fallback += 1;
                continue;
            };
            let Ok(shp) = crate::assets::shp_file::ShpFile::from_bytes(data) else {
                fallback += 1;
                continue;
            };
            let Some(frame) = shp.frames.first() else {
                fallback += 1;
                continue;
            };
            if let Some(entry) = self.get_mut(&name) {
                entry.frame_width = frame.frame_width;
                entry.frame_height = frame.frame_height;
                populated += 1;
            } else {
                fallback += 1;
            }
        }
        (populated, fallback)
    }
```

This depends on `ArtRegistry` having `iter_entries()` returning `(&str, &ArtEntry)` and `get_mut(&str) -> Option<&mut ArtEntry>`.

**Step 2: Verify the helper methods exist (or add them)**

Before running the build, check whether `iter_entries()` and `get_mut()` exist on `ArtRegistry`. Grep:

```
Grep: "fn iter_entries|fn get_mut" in src/rules/art_data.rs
```

If `iter_entries` is missing: add it as `pub fn iter_entries(&self) -> impl Iterator<Item = (&str, &ArtEntry)>` returning the registry's internal map iterator.

If `get_mut` is missing: add it as `pub fn get_mut(&mut self, name: &str) -> Option<&mut ArtEntry>` returning a mutable reference into the internal map.

If either exists with a slightly different signature, **STOP and report** — adapt the populate method to the existing API rather than introducing parallel ones.

**Step 3: Verify**

Run: `cargo check --package vera20k`
Expected: PASS. The compiler will warn `populate_anim_frame_dims` is unused; that's expected (Task 3 wires it in).

**Step 4: Commit**

`rules: ArtRegistry::populate_anim_frame_dims for smudge-flagged anims`

---

### Task 3: Wire populate_anim_frame_dims into app_init

**Why:** Calls the populate method at startup so the dims are populated before any sim/render code reads them. Critical timing: the call must run **before** `r.art_registry = a.clone()` so both the source `art` and the cloned `RuleSet.art_registry` see the populated dims.

**Files:**
- Modify: `src/app_init.rs` (around line 262-269)

**Pattern:** Single-line method call inside the existing rules+art conditional block.

**Step 1: Insert the populate call**

At [src/app_init.rs:262-269](src/app_init.rs#L262-L269), the existing block looks like this:

```rust
    if let (Some(r), Some(a)) = (rules.as_mut(), art.as_ref()) {
        r.merge_art_data(a);
        // Retain the art registry on RuleSet so dispatchers (e.g. smudge
        // spawning) can read per-anim spawn flags via &RuleSet alone.
        // Cloned because downstream consumers in this function still read
        // through the `art` Option (lighting, sidebar, sim spawn, etc.).
        r.art_registry = a.clone();
    }
```

We need `art.as_mut()` (not `as_ref()`) to call the mutating populate method on it. Change the binding and add the populate call **before** the clone:

```rust
    if let (Some(r), Some(a)) = (rules.as_mut(), art.as_mut()) {
        r.merge_art_data(a);
        // Eagerly populate per-anim SHP frame dimensions so the smudge
        // dispatcher can size-filter without falling back to the (30, 30)
        // default that always loses the threshold check.
        let (populated, fallback) = a.populate_anim_frame_dims(
            &asset_manager,
            theater_ext,
            &map_data.header.theater,
        );
        log::info!(
            "Anim frame dims: {} populated, {} fallback (defaults to 30x30)",
            populated,
            fallback,
        );
        // Retain the art registry on RuleSet so dispatchers (e.g. smudge
        // spawning) can read per-anim spawn flags via &RuleSet alone.
        // Cloned because downstream consumers in this function still read
        // through the `art` Option (lighting, sidebar, sim spawn, etc.).
        r.art_registry = a.clone();
    }
```

**Step 2: Plan-vs-reality check before edit**

Before editing, verify:
- The `if let` block at line 262 still binds `art.as_ref()` (not already `as_mut()` or refactored).
- `theater_ext: &str` is in scope at this point (set at line 231).
- `map_data.header.theater` is accessible as `&str`.
- `asset_manager` is in scope.

If any of these has shifted, STOP and report.

**Step 3: Verify**

Run: `cargo build --package vera20k`
Expected: PASS, no warnings about `populate_anim_frame_dims` being unused. The boot log on the next engine run should show e.g. `Anim frame dims: 30 populated, 0 fallback (defaults to 30x30)`. Engine still functional; smudge dispatch still uses DEFAULT in this task — Task 4 swaps it.

**Step 4: Commit**

`app_init: eagerly populate ArtEntry frame dims after merge_art_data`

---

### Task 4: Replace DEFAULT_ANIM_FRAME_DIM in smudge_dispatch with ArtEntry dims

**Why:** Closes the loop. After this task, V3-class explosions pass the size threshold and select 2×2 SmudgeTypes when the impact lands on Morphable terrain.

**Files:**
- Modify: `src/sim/combat/smudge_dispatch.rs` (around line 100-141)

**Pattern:** Direct field read substitution; no structural change.

**Step 1: Replace the constant reads**

Locate the `try_dispatch_anim_smudge` body at [src/sim/combat/smudge_dispatch.rs:87](src/sim/combat/smudge_dispatch.rs#L87). The current code around lines 106-107:

```rust
    let dmg = DEFAULT_ANIM_FRAME_DIM;
    let dmg2 = DEFAULT_ANIM_FRAME_DIM;
```

Replace with:

```rust
    let dmg: i32 = entry.frame_width as i32;
    let dmg2: i32 = entry.frame_height as i32;
```

`entry` is the `ArtEntry` resolved at line 100 via `art.get(anim_name)`. The conversion to `i32` matches the existing signature of `SmudgeGrid::try_place(... dmg: i32, dmg2: i32 ...)`.

**Step 2: Remove the now-unused DEFAULT_ANIM_FRAME_DIM constant**

Search the file for `DEFAULT_ANIM_FRAME_DIM`. If the only remaining references are the constant definition itself and the comments around it, delete the constant and update the doc comments. If the constant is referenced elsewhere (e.g., by tests or building-destruction dispatchers that intentionally pass `100` instead of frame dims), leave the constant alone — only remove if all references are gone.

If the constant is referenced from a test file, **STOP and report** — the test may be encoding the (30, 30) drift assumption; we want to preserve test coverage of the fallback path, but the test target may need updating.

**Step 3: Plan-vs-reality check**

Before edit, verify:
- `entry` is the ArtEntry resolved at the top of `try_dispatch_anim_smudge` (line 100 currently).
- `entry.frame_width` and `entry.frame_height` are accessible (Task 1 added them as public fields).
- The existing `dmg` / `dmg2` are passed only into `try_place` calls within the same function (no other consumers).
- `BUILDING_SMUDGE_DMG` (used by the building-destruction path at line 183 / 189) is a separate constant and is **NOT** changed by this task.

**Step 4: Verify**

Run: `cargo test --package vera20k smudge` — all 33 smudge tests must still PASS.
Run: `cargo build --package vera20k` — clean build.

If a smudge test fails (e.g., one that asserts on size-filter behavior with mock anim dims), STOP and report. The test may be encoding the old DEFAULT=30 assumption; need to check whether the test still expresses correct gamemd parity.

**Step 5: Commit**

`combat: smudge dispatch reads frame dims from ArtEntry instead of fixed default`

---

### Task 5: In-game verification

**Why:** Confirm the populate path runs end-to-end and that V3-class strikes now produce 2×2 craters on temperate ground.

**Files:** None modified. Manual visual check.

**Step 1: Boot a temperate skirmish**

Open the engine on a temperate map. Check the boot log for:
```
Anim frame dims: N populated, M fallback (defaults to 30x30)
```
Expected: `N` ≈ 30 (the count of smudge-flagged anims in retail artmd.ini), `M` should be small (~0-5 if a few anim SHPs are missing from retail mix archives).

If `N == 0`, the populate method ran but found no smudge-flagged anims — likely an iter_entries / flag-read bug. STOP and investigate.

If the log line is absent entirely, the populate call didn't run — Task 3 wiring is broken. STOP and investigate.

**Step 2: Test V3 strike on clear temperate terrain**

Spawn a V3 launcher and fire at a target standing on a clear, flat, Morphable temperate cell with no overlay/building. Observe:

- [ ] The killing strike produces a visible crater. (Already verified in the prior smudge-atlas verification.)
- [ ] The crater is **2×2 cells** (a larger multi-cell footprint), not 1×1. This is the test for this fix.
- [ ] Side-by-side vs retail YR: the same warhead/anim combo on the same cell produces a comparably-sized crater.

If craters are still 1×1 after the fix:
- Check the boot log: did populate actually run with `populated > 0`?
- Check whether the warhead's killing-anim has a 2×2 SmudgeType available in `[SmudgeTypes]` (CR2-class or CRATER05+ are 2×2 in retail rules — verify via `[CR2] Width=` etc.).
- Check whether the anim's SHP frame 0 has frame_width > 60 AND frame_height > 50. If not, gamemd would also pick 1×1 (no drift; just an anim that happened to be small).

**Step 3: Test with a smaller anim**

If you have access to a weapon whose AnimList anim has a sub-60×50 frame size (e.g., small grenade explosion or RPG warhead), verify it still produces 1×1 craters — confirming we didn't over-correct by populating dims that always pass the threshold.

**Step 4: Commit**

`smudge: in-game verification — V3 craters are 2x2 on temperate ground`

(If verification reveals a parity drift, STOP and report findings rather than papering over.)

---

## Sources & References

- **Brainstorm (this session):** Approach (A) approved 2026-05-07 — post-construction `populate_anim_frame_dims` method on ArtRegistry, called from app_init after merge_art_data.
- **Prior plan (resolved follow-up):** [docs/plans/2026-05-06-smudge-system-plan.md](docs/plans/2026-05-06-smudge-system-plan.md) Follow-up #1 ("Eager SHP frame-width/height init for ArtEntry") — this plan implements that.
- **Ghidra reports (research base):**
  - [ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md](ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md) — ledger #8/#9.
  - [docs/plans/2026-05-06-smudge-system-design.md](docs/plans/2026-05-06-smudge-system-design.md) — explicit deferral note.
- **gamemd.exe addresses (kept here, not in Rust comments):**
  - `0x00424F00` — `AnimClass::Start` (lazy-populates dims if uncached).
  - `0x0069E7E0` — `SHP_frame_rect_getter` (reads frame_x/y/width/height).
  - AnimType `+0x29C` = cached frame width; `+0x2A0` = cached frame height; `+0x298` = Start frame (defaults 0).
- **INI keys driving behavior:**
  - `artmd.ini` per-anim: `Crater=`, `Burn=`, `ForceBigCraters=`, `Image=`, `Theater=`, `NewTheater=`. All already parsed.
- **Related code:**
  - [src/rules/art_data.rs:19](src/rules/art_data.rs#L19) — `ArtEntry` struct (this plan extends).
  - [src/rules/art_data.rs:618](src/rules/art_data.rs#L618) — `anim_shp_candidates` (this plan calls).
  - [src/rules/art_data.rs:462](src/rules/art_data.rs#L462) — `resolve_effective_image_id` (this plan calls).
  - [src/render/sprite_atlas.rs:453-477](src/render/sprite_atlas.rs#L453-L477) — load-SHP-and-read-header pattern this plan mirrors.
  - [src/render/overlay_atlas.rs:622-651](src/render/overlay_atlas.rs#L622-L651) — `probe_terrain_shp_frame_count` semantic analog.
  - [src/sim/combat/smudge_dispatch.rs:87-142](src/sim/combat/smudge_dispatch.rs#L87-L142) — `try_dispatch_anim_smudge` (this plan modifies).
  - [src/app_init.rs:262-269](src/app_init.rs#L262-L269) — wiring point.
  - [src/rules/shp_vehicle_sequence.rs:97](src/rules/shp_vehicle_sequence.rs#L97) — `make_art_entry` test helper.
- **Prior session commits (smudge atlas, just landed on dev):**
  - `21ed036` — render: shift smudge sprite position by half a tile to land on cell center
  - `f052ec5` — rules: remove vestigial SmudgeTypeDef.is_theater field
  - `67bdf83` — docs: correct frame_offset semantics in smudge code comments
  - `6892da15`, `03f3e40d`, `473267bc`, `62c82823` — smudge atlas registration chain.

## Follow-up tasks (not in this plan)

1. **Finding #1 from the V3 trace-action: kill-only emission gap.** Smudges only spawn when V3 strikes kill something. gamemd spawns smudges on every warhead detonation regardless of kill. Bigger architectural touch — needs its own brainstorm session.
2. **(Optional) Generalize populate_anim_frame_dims to all anims** if a future feature needs frame dims for non-smudge anims (e.g., HUD effects that scale based on anim size). YAGNI for now.
