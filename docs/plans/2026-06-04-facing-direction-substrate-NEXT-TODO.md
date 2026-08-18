# Facing / Direction Substrate — Continue-Tomorrow TODO

**Created:** 2026-06-04 (for 2026-06-05 session)
**Branch:** `facing-direction-substrate` — worktree `<local>/Documents/ra2-rust-game-facing` (off `dev`)
**Status of foundation (S1–S4): DONE, committed, green.**

---

## Where things stand

The lookup-table foundation (S1–S4 of the plan) shipped — additive, hash-neutral, no
consumer cutover. 4 commits on the branch:

| Commit | Slice |
|--------|-------|
| `df48c782` | S1 cell-delta service (re-export + checked/unchecked accessors) |
| `2cab5e45` | S2 integer `LEPTON_DELTAS` (closes **D1 data layer**) |
| `0a3ecedd` | S3 quantization + 16-bit + muzzle |
| `f4072624` | S4 `DRAGON_FRAME_TABLE` (closes **D3 data layer**) |

New files (all under the substrate tree + one `pub mod` line):
- `src/sim/substrate/mod.rs`
- `src/sim/substrate/direction_tables/{mod.rs, cell.rs, lepton.rs, quantize.rs, dragon.rs}`
- `src/sim/mod.rs` (+`pub mod substrate;`)

**Verification done:** 11 substrate exact-equality tests pass; full suite **3600 passed /
0 failed / 17 ignored**, all integration binaries clean. New files are clippy-clean.

**Two carry-over facts:**
- `ini/` was copied into the worktree (gitignored — needed for `include_str!` at build).
- Pre-existing clippy `approx_constant` **errors** live in `src/render/vxl_normals.rs` +
  `src/render/vxl_raster.rs` (NOT this slice; present on `dev`). Ignore them — not in scope.

---

## First thing tomorrow (pick one)

> The foundation sits **unused** until a cutover re-points a consumer onto the new tables.
> Every cutover changes player-visible behavior, so each starts with a **verification/research
> gate**, NOT straight-to-code (research-first discipline; `feedback_design_approval...`).

### Option A (recommended) — Land the branch, then D3 DRAGON cutover
1. **Merge `facing-direction-substrate` → `dev`** (hash-neutral/additive → safe; the user
   decides when). This banks the foundation.
2. **D3 DRAGON cutover (narrowest verified-formula fix):** re-point `app_fire_effects` (the
   `Rotates=yes` projectile frame path) onto `substrate::direction_tables::dragon_frame_index`,
   replacing the current wrong cell-delta formula (study retire list / FACING_BYTE doc §9).
   - **GATE before code:** verify the `bam` source in Ghidra —
     `bam = ftol((atan2(-VelY,VelX) - π/2) · (-32768/π))` using live BulletClass velocity;
     constants `0x007E2820 = π/2`, `0x007E2818 ≈ -32768/π` (study §5). Confirm the Rust side
     feeds the *same* angle so `dragon_frame_index(bam)` lands on gamemd's frame.
   - Start with `/re-investigate` or `/trace-action` on the DRAGON/AAHeatSeeker2 fire→frame
     path, then `/brainstorm` → `/write-plan`.

### Option B — D1 lepton cutover (higher impact, more entangled)
Re-point the ground locomotor 8-direction step onto `lepton_delta` → fixes diagonal speed
(gamemd advances **±256** per axis on a diagonal; Rust currently uses the **±181** sin/cos
diagonal via `facing_to_movement`). Closes D1's *behavior* half.
- **GATE:** read-trace the ground locomotors first — the facing the step consumes is produced
  by **float `atan2`** in Rust (`fixed_math.rs:280-311`), and gamemd's infantry walk facing is
  *genuinely* atan2 too (study §4.6 correction). So this is NOT "swap sin/cos for the table"
  cleanly; it needs the locomotor read-trace + the atan2-bit-identity question scoped before
  any edit. Keep `facing_to_movement` for aircraft/continuous-heading movers.

### Option C — S5 drive-track gate (unblocks the big move)
Verify `transform_track_point` (`drive_track.rs:44-62`) flag math (swap_xy / negate_x /
negate_y + paired facing transforms) against the binary's `Transform_Track_Coords`, and diff
the full 72-entry TurnTrack / 16-entry RawTrack / ~492 TrackPoint arrays vs
`read_memory 0x007E7A28 / 0x007E7B28` (study §4.5 / §8 S5). This is the **blocking gate** that
must clear before the ~3,393-line drive-track move can happen. Pure verification work, no code.

---

## Deferred (later slices, not tomorrow unless chosen above)
- **S5** drive-track move + full byte-equality (blocked on Option C gate).
- **S6/S7** the D1/D3 *behavior* cutovers (Options A/B).
- **S8 / U19** FacingClass / turret turn parity — **stateful** (timer-interpolator vs gamemd
  per-frame ClampToROT). Per-frame equality test required; separate plan.

## Read-first docs (cold-start pointers)
- Plan: `docs/plans/2026-06-04-facing-direction-substrate-plan.md`
- Study (binary-verified table values + DRIFT ledger D1/D3/D5, Verification Log #1/#2/#4/#5):
  `docs/research/substrate/tables/FACING_DIRECTION_SUBSTRATE_STUDY.md`
- Roadmap rows U1–U4: `docs/research/substrate/LOOKUP_TABLE_SUBSTRATE_SERVICE_STUDY.md`
