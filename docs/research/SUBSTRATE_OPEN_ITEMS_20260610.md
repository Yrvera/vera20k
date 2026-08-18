# Cell/Map Substrate — Open Items Carried Forward (2026-06-10)

**Successor tracker** for the items left open when
`CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` was **CLOSED** (2026-06-10). That study stays
the evidence archive (verified native contracts, offset maps, slice history — do NOT re-derive from
scratch); THIS doc is the live work list. Item numbers reference the study's §4.2 for traceability.

## Implementation items (each needs a /brainstorm → /write-plan pass before code)

| # | Item | Severity / trigger frequency | Evidence anchor (study §) |
|---|------|------------------------------|---------------------------|
| 1 | **A\* per-neighbor live `Can_Enter_Cell`** — replace the per-command entity-block snapshot with live classification, or prove the snapshot bit-equivalent on a contended scenario. Includes the **bridge-layer hard-block set constructed empty** (`bump_crush.rs:126` — bridge-deck blockers invisible to A\*'s hard set). | HIGH class; residual staleness now limited to one mover's own multi-step expansion (generation-counter mitigation landed) | §4.2 #5, §8 Slice 6 |
| 2 | **Reservation-on-intent (`+0xDC`-style)** — native blocks a cell a unit is moving toward before arrival; Rust commits dest only after a successful move. Two vehicles can still both path toward one empty cell within a tick. Rides with item 1. | MEDIUM; fires in dense traffic every match | §4.2 #7, §5 C-RECORD #10 |
| 3 | **A\* ground-layer diagonal corner-cutting** — `core.rs:1249` exempts Ground from flanking-cardinal validation while `zone_build.rs:503-514` requires both cardinals on every layer. Verify the native A\* neighbor rule in Ghidra FIRST (the zone side is the verified one), then align. | MEDIUM; any diagonal path past a blocked corner | §4.2 #9b |
| 4 | **Radiation green glow (render)** — sim core landed (`86b0d4bf`); the per-site LightSource (intensity `min(level×RadLightFactor, 2000)`, RadColor tint × `remaining/duration`, RadLightDelay stepping) needs dynamic-light infrastructure in the render layer. `RadiationState::sites()/iter_cells()` already expose everything render needs. | Player-visible on EVERY Desolator deploy | study §2.6 + §4.2 #12 residual |
| 5 | **Slice 3c — FNPC caller migration** — ~39 of ~40 engine Find_Nearby_Passable_Cell analog callsites still on separate implementations (miner dock `miner_dock_sequence.rs`, `bunker_link.rs`, scatter, chrono outbound, rally, crates, start positions). Facade + first caller + playfield diamond all landed. | MEDIUM aggregate; per-caller visibility varies | §8 Slice 3 |
| 6 | **Slice 4 — duplicated-field consolidation** — Level held in 4-5 homes, LandType in 4 slots, overlay byte partitioned across 2 homes; collapse to one authoritative home + derived caches. Internal-only IF done right — gate on full-replay hash regression. | Hygiene/risk-reduction; no direct player-visible drift | §4.1, §8 Slice 4 |
| 7 | ✅ **RESOLVED 2026-06-10 (`8d008598`)** — Crowd-jam was **invented behavior (DRIFT), removed.** Adversarial 4-lane Ghidra pass enumerated the COMPLETE speed-determination chain in all three ground locomotors' Process_Movement (Drive `0x004B2630`, Ship `0x006A1C80`, Walk `0x0075AEC0`): per-tick speed = `clamp1(g_SpeedType_LandType_Table[ST + LT*9]) × cliff/slope × (0.5 if 0) × (0.75 if HealthRatio ≤ ConditionYellow)`. **No occupancy / neighbor-count / spacing input feeds speed anywhere** (the only count-like reads — `Can_Enter_Cell` result codes, `Scatter_Objects` — drive block/repath, never a speed scalar); no `0.7` constant exists in any speed path. Removed `crowd_speed_factor` + `CROWD_*` consts + the two crowd fields; `compute_cell_speed_modifier` no longer takes `OccupancyGrid`. | ✅ DONE | §4.2 #10, §7 #5 |
| 8 | **#6 hygiene** — retire the dead-with-stock-INI `zone_layer_for_speed_type` fallback in the three terrain-entry sites; align missing-INI-section default with the native all-zeros (= reject). Pure cleanup; live path already native (INI speed table primary). | None in stock play | §4.2 #6 (downgraded) |
| 9 | **UI-vs-sim gap-radius source** — UI range circle prefers per-object `(Super)GapRadiusInCells`, sim suppression uses `[General] GapRadius` only. Identical on stock INI. | LOW; mods only | §4.2 #13 |
| 10 | ✅ **RESOLVED 2026-06-11 (`a29b7886` on `dev`)** — height-based LOS re-enabled after the parity review. Verified instruction-level at `MapClass::RevealShroud 0x005673a0` (+ `RevealAroundCell 0x005678e0`): the obstruction "mirror" table (`InitRevealMirrorTable 0x00563908`) matches Rust `REVEAL_MIRROR` **253/253** (full diff), the reveal spiral matches, threshold is `obs_level > viewer+3` (signed), single-cell sample, `viewer_level = z/30 = level`. The **one drift** was the obstruction sample location: native uses `target + mirror[i] + (2,2)`; Rust omitted the `+2` (confirmed by 2 independent decodes + the spiral table carrying no `-2` bias to cancel it). Added the `+2`; flipped the live-tick flag to `rules.general.reveal_by_height` (default true). Fog is in the state hash → first elevation+shroud golden pins to the new values; no flat fixture shifts (suite green, 3880). | ✅ DONE | study 2026-06-04 refresh block |
| 11 | ✅ **RESOLVED 2026-06-11 (`2e73ac1e` on `dev`)** — native cliff/slope model implemented. Instruction-level trace (Drive `0x004B2630` `0x004b3cd5..da6`, Ship `0x006A1C80`) **locked the direction**: per-tick speed ×= one of four `[General]` coefficients by `SpeedType==Track` × direction, comparing destination-cell `GetGroundHeight` (ESI) vs the mover's current coord `obj+0x9c` (EDI; the head-to lives on the locomotor at `+0x40`). **Uphill** (dest higher) → `TrackedUphill`/`WheeledUphill` (`+0x768`/`+0x778`); **downhill** (dest lower) → `TrackedDownhill`/`WheeledDownhill` (`+0x770`/`+0x780`); vanilla **1.0 up (no change) / 1.2 down (faster)**. The "crossed/inverted" worry was a swapped-offset artifact — loader FSTP `@0x0066f234..` proves `+0x768=TrackedUphill` (downhill is the boost, matching INI intent). Replaced the invented `slope_climb`/`slope_descend` (0.6/1.2) with the 4 coefficients; parse the real keys; wired `from_general` into the World at map load (was dead code → always `::default()`). All tests green (3873). §8.6 step 3 corrected this pass (it had UP/DOWN→offset inverted). **Residual follow-ups:** (a) infantry slope is a separate precomputed-foot path (`FootClass+0x530`) — they take the wheeled branch here (stock-identical 1.0/1.2); (b) the Mission_Move gate and `GetGroundHeight` (continuous, ramp-aware) vs Rust's integer cell-`level` trigger are not yet modelled. | ✅ DONE (vehicles); infantry-foot + trigger fidelity = follow-up | §8.6 (corrected this pass) |

## Research-side opens (Ghidra; no code until verified)

- **CliffBack overlay-LAT branch site** — which landtypes reach the unconditional overlay-LAT reclass site
  in `RecalcAttributes` (a Road-landtype cell under an overlay-LAT tile at a cliff base would natively
  reclass; Rust's eligible set is the tail set only). Study §3.
- **FootClass rad-damage residual gates** — `vtbl+0x54()==0`, `this+0x81==0` identities (Rust uses
  airborne/limbo as the INFERRED analog). Study §9.
- **CellClass `+0x30`** persisted swizzled pointer slot — read-side sweep before declaring unused;
  `+0x5C/+0x60` (ctor −1, no accessor); `+0x50` wall-owner reader side. Study §9.
- **MapClass `+0x70`** 10-byte zone-cell records — 9 of 10 bytes unidentified. Study §9.
- **LightConvert refcount on save/load** (render-layer parity). Study §9.
- **Slice 5 broader sweep** — routine regression proofs that PathGrid/ZoneGrid/TerrainCostGrid rebuilds
  reproduce live state (occupancy list-order contract is proven; these grids are pure functions of
  serialized state).
- **Infantry slope mechanism** (item 11 residual) — `WalkLocomotion` does NOT use the vehicle 4-coefficient
  path; infantry slope is a precomputed double at `FootClass+0x530` read by `FootClass__Get_Slope_Speed_Factor
  @ 0x004DC760`. Decode what writes `+0x530` (and whether it derives from the same `[General]` keys or a foot-
  specific source) before giving infantry their own slope path. Rust currently routes Foot through the wheeled
  branch (stock-identical 1.0/1.2).
- **Vehicle slope trigger fidelity** (item 11 residual) — native applies the slope mult only under `Mission_Move`
  (`vtbl+0x2c()==1`) and compares `GetGroundHeight` (continuous, ramp-aware) of dest vs current; Rust applies on
  any integer cell-`level` delta during the move step. Verify whether attack-move/guard-pursuit (non-Mission_Move)
  should skip the mult, and whether ramp sub-cell height changes the trigger surface.
- ✅ **RESOLVED 2026-06-11 — Reveal-center Z-shift** (item 10 residual; was HIGH on elevation maps).
  `reveal_radius_into` (`src/sim/vision/mod.rs`) now shifts the spiral center `-z_shift` per axis where
  `z_shift = position.z / 2`. Re-verified instruction-level at `MapClass::RevealShroud 0x005673a0` and
  `RevealAroundCell 0x005678e0` (live decompile this pass): the spiral uses the Z-shifted cell (`uVar13`/`sVar8`),
  the shift is computed **before** the `RevealByHeight` gate (so it applies unconditionally), and the height
  obstruction cell `psVar9[2] + (local_24 - local_14)` with `local_14 = (shifted-orig) - 2` expands to
  `orig + spiral + mirror + 2` — i.e. **the shift cancels in the obstruction**, leaving it relative to the raw
  foot cell. Rust reproduces this by adding `z_shift` back into the obstruction (`obs = rx + mdx + 2 + z_shift`),
  so the just-landed height-LoS check is unchanged. Magnitude correction vs the report's `z/30` (which assumed
  Rust stored leptons): Rust's `position.z` IS the integer level, so the shift is `position.z / 2` — and since
  `position.z*15` is always a multiple of 15, the `AdjustForZ` float `+1`/`+0.5` fixups never change the integer
  cell result, making `position.z/2` **exact for all z** (not just even levels). Tests: `test_reveal_center_z_shift`
  added; `test_height_los_high_viewer_sees_past_cliff` + the two elevation-sight-bonus tests updated for the shifted
  center. Suite green (3963). `REVEAL_Z_SHIFT_GHIDRA_REPORT.md` §6 `z/30` note is superseded by `position.z/2`.
- **Sight-10 height-LoS** (item 10 residual; LOW — sight 10 is rare) — the Rust sight-10 outer ring uses a sqrt
  fallback that skips the height check; native's mirror table has 309 entries (sight 0–10) and checks the ring.

## Landed 2026-06-10 (for context, all on `dev`)

- `86b0d4bf` Slice 7 — per-cell radiation field service (closes study §4.2 #12 sim core).
- `7044fcec` Slice 3b — playfield diamond built from the map header, threaded into the FNPC occupancy check.
- `8a7e2ea4` CliffBackImpassability — full consequence set (zone/speed/buildability/base-snapshot bake).
- Slice 5 acceptance tests (`snapshot.rs`) incl. the re-entry ordering case.
- §4.2 #6 downgraded with evidence (live terrain-entry path already reads the native INI speed table).
- `8d008598` item 7 — removed the invented crowd-jam speed term (`terrain_speed.rs`), verified absent across the Drive/Ship/Walk Process_Movement speed chains by an adversarial 4-lane Ghidra pass. Slope-factor drift split out as new item 11 (research-blocked on the up/down direction mapping).
- `2e73ac1e` item 11 (on `dev`, rebased + fast-forwarded) — native cliff/slope model: 4 `[General]` coefficients by Track×direction, uphill 1.0 / downhill 1.2, wired into the World at map load. Direction locked by instruction-level trace; §8.6 step 3 corrected. Tests green (3879). Infantry-foot + trigger-fidelity residuals moved to research-side opens. (`50d42a4f` rides along: preserved a concurrent session's `speed_multiplier_clamps_at_one` test so the ff didn't drop it.)
- `a29b7886` item 10 (on `dev`, rebased + fast-forwarded) — re-enabled height-based LOS after the parity review; the `REVEAL_MIRROR` table verified 253/253 vs `InitRevealMirrorTable`, and the one drift (missing `+2` obstruction offset) fixed. Suite green (3880). Reveal-center Z-shift + sight-10 residuals filed in research-side opens. Corrected the stale "mirror table not extracted" claim in `SHROUD_ALGORITHM_DISTILLED.md`.
