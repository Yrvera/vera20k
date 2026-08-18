# Bridge BSS Runtime Constant Sweep - Ghidra Research Report

**Address(es):** `0x00489030..0x00489127`, `0x004AF4A0`, `0x0069EBB0`, `0x006D2120`, `0x00544691`, `0x00543F10`, `0x00547230`  
**Investigation Mode:** exhaustive-slice attempted; final status is partial because live debugger capture was unavailable.  
**Claimed Scope:** init-site/static-value classification for `DAT_0089E870`, `DAT_0089E864`, `DAT_00ABC210`, `DAT_00ABC2D0`, Drive/Ship/Walk bridge-Z globals, and the Rust hardcoded ship `360` assumption.  
**Non-Scope:** full bridge rendering, full locomotor behavior, pathfinding, bridge damage dispatch, runtime map/theater loader correctness outside the named constants.  
**Confidence:** High for static table values and formula shapes; Medium for Z-writer identity where upstream geometry inputs remain runtime-only; Low for exact numeric BSS magnitudes without a post-load capture.  
**Active in YR:** Yes. The consumers are active tactical bridge render, AoE bridge damage, and live Drive/Ship/Walk locomotor paths in standard YR.

## 1. Target Question

Which named bridge globals have recoverable static initializer values, which only have recoverable formulas, and which still require a live post-map-load debugger read to pin exact lepton magnitudes?

## 2. Non-Goals

- Do not re-investigate `Apply_area_damage` draw count/order beyond naming the Z constants it reads.
- Do not re-investigate `FUN_00547230` rendering composition beyond table value and init-site identity.
- Do not implement Rust fixes.
- Do not mutate the Ghidra project.

## 3. Stop Conditions

- Static image read proves a global is BSS-zero and no attached runtime debugger is available.
- Init-site formula reaches another BSS/runtime geometry input whose value is not statically materialized.
- Existing high-confidence bridge docs plus fresh byte reads agree on the static initializer entries.

## 4. Summary Verdict

`DAT_00ABC210` and `DAT_00ABC2D0` are not live-only mysteries: both are BSS in the cold image but have static initializer code with exact entries. Their values can be ported without debugger capture.

`DAT_0089E864` has a static writer formula: it is computed from `DAT_0089E870` as `ftol(DAT_0089E870 * 4 * 0.5)`, i.e. `2 * DAT_0089E870` for integer `DAT_0089E870`.

`DAT_0089E870`, Drive `g_BridgeZOffset`, Ship `g_BridgeZ_Offset`, Walk `FUN_006D2120(60)`, and the Rust ship `360` assumption all depend on runtime-initialized height-step/geometry globals. Their formulas are verified, but exact numeric values still need a post-map-load debugger capture. A non-mutating debugger read was attempted and failed because the debugger server was not running.

## 5. Constant Ledger

| Global / constant | Static image | Static init / formula recovered? | Exact runtime value recovered? | Active consumer | Capture needed? |
|---|---:|---|---:|---|---|
| `DAT_0089E870` | `0` | Partial: writer around `0x00489060..0x0048908B` computes from runtime geometry inputs and shared `0.5` | No | `Apply_area_damage`, `Warhead__SelectExplosionAnim` | Yes |
| `DAT_0089E864` | `0` | Yes: writer at `0x00489100..0x00489127`, `2 * DAT_0089E870` | No, because source is runtime | `Apply_area_damage` Z-window and object-layer selector | Yes, read with `DAT_0089E870` |
| `DAT_00ABC210` | `0` | Yes: `0x00544691`, 10 entries | Yes, statically | `FUN_00547230` railing/shadow emitter | No for table values |
| `DAT_00ABC2D0` | `0` | Yes: `0x00543F10`, 40 entries | Yes, statically | `FUN_00547230` fallback emitter | No for table values |
| Drive `g_BridgeZOffset_Drive @ 0x008A07C4` | `0` | Yes: `0x004AF4A0`, `ftol(g_DriveHeightStep * 4 + 0.5)` | No | Drive destination/process bridge Z | Yes |
| Drive `g_DriveHeightStep @ 0x008A07D0` | `0` | Source is runtime isometric height-step init | No | Drive formulas and thresholds | Yes |
| Ship `g_BridgeZ_Offset @ 0x00B0782C` | `0` | Yes: `0x0069EBB0`, `ftol(g_ShipHeightStep * 4 + 0.5)` | No | Ship bridge Z / under-bridge clearance | Yes |
| Ship `g_ShipHeightStep @ 0x00B07838` | `0` | Source is runtime isometric height-step init | No | Ship formulas and Rust `360` comparison | Yes |
| Walk scale `DAT_00B0CDD8` | `0` | Consumer formula at `0x006D2120`: `ftol((arg - 0.5) * DAT_00B0CDD8)`; Walk passes `60` per prior doc | No | Walk `Head_To_Coord` bridge bump | Yes |
| Walk per-level `DAT_00B45C28` | `0` | Runtime init site documented at `0x75A99B`; not re-drained here | No | Walk movement height denominator | Yes |
| `DAT_00AC13C8` / `DAT_00AC13BC` | `0` | Prior docs verify consumers; values cold-zero here | No | ShouldBeOnBridge / Set_Height bridge add | Yes |

## 6. Recovered Static Tables

`DAT_00ABC210` entry layout is four 32-bit integers: `(frame_1based, required_sub_tile, dx, dy)`. `frame_1based == 0` means the consumer returns before using offsets.

| Slot | Frame 1-based | Required sub-tile | DX | DY |
|---:|---:|---:|---:|---:|
| 0 | 0 | 0 | 0 | 0 |
| 1 | 0 | 0 | 0 | 0 |
| 2 | 0 | 0 | 0 | 0 |
| 3 | 0 | 0 | 0 | 0 |
| 4 | 13 | 6 | 48 | 12 |
| 5 | 0 | 0 | 0 | 0 |
| 6 | 14 | 1 | 48 | 12 |
| 7 | 0 | 0 | 0 | 0 |
| 8 | 0 | 0 | 0 | 0 |
| 9 | 0 | 0 | 0 | 0 |

`DAT_00ABC2D0` is a 40-entry fallback table with the same layout. Nonzero draw entries are:

| Slot | Frame 1-based | Required sub-tile | DX | DY |
|---:|---:|---:|---:|---:|
| 20 | 1 | 0 | 30 | 30 |
| 21 | 1 | 0 | 30 | 30 |
| 22 | 2 | 1 | 60 | 15 |
| 23 | 3 | 1 | 60 | 15 |
| 24 | 4 | 1 | 60 | 15 |
| 25 | 5 | 0 | 90 | 30 |
| 26 | 6 | 0 | 60 | 15 |
| 27 | 7 | 1 | 30 | 0 |
| 28 | 8 | 0 | 60 | 15 |
| 29 | 9 | 0 | 60 | 15 |
| 30 | 10 | 1 | 0 | -15 |
| 31 | 11 | 1 | 0 | -15 |
| 32 | 12 | 0 | 60 | 15 |

Tiny detail: slots 17, 18, and 19 carry offsets `(30,30)` but frame `0`, so `FUN_00547230` skips before offsets matter.

## 7. Evidence Notes

- Fresh `read_memory` confirmed static image zeros for `0x0089E864`, `0x0089E870`, `0x00ABC210`, `0x00ABC2D0`, `0x008A07C4`, `0x008A07D0`, `0x00B0782C`, `0x00B07838`, `0x00B0CDD8`, `0x00B45C28`, `0x00AC13C8`, and `0x00AC13BC`.
- Fresh byte read at `0x00489100` confirms `DAT_0089E864` writer loads `DAT_0089E870`, multiplies by four, multiplies by `0.5` at `0x007E1738`, calls `ftol`, and writes `0x0089E864`.
- Fresh byte reads at `0x004AF4A0` and `0x0069EBB0` confirm Drive and Ship use the same pattern: load the per-locomotor height step, multiply by four, add `0.5`, `ftol`, write separate globals.
- Fresh byte read at `0x006D2120` confirms Walk helper subtracts `0.5` from the integer argument before multiplying by `DAT_00B0CDD8` and calling `ftol`.
- Fresh byte reads at `0x00544691` and `0x00543F10`, plus `BRIDGE_THEATER_LOAD_TABLE_WRITERS_GHIDRA_REPORT.md`, confirm exact `DAT_00ABC210` and `DAT_00ABC2D0` entries.
- Runtime debugger memory was attempted for the named globals but failed with: debugger server not running at `127.0.0.1:8099`.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `DAT_00ABC210` static values | verified | `0x00544691` bytes; `BRIDGE_THEATER_LOAD_TABLE_WRITERS_GHIDRA_REPORT.md` | none for values |
| `DAT_00ABC2D0` static values | verified | `0x00543F10` bytes; `BRIDGE_THEATER_LOAD_TABLE_WRITERS_GHIDRA_REPORT.md` | none for values |
| `FUN_00547230` table consumer identity | verified | prior decompile reports; slot/sub-tile doc | full render composition out of scope |
| `DAT_0089E864` formula | verified | `0x00489100` bytes | runtime numeric read |
| `DAT_0089E870` writer/source | touched-not-exhausted | `0x00489030..0x0048908B` bytes | upstream runtime geometry values and live numeric read |
| Drive bridge-Z formula | verified | `0x004AF4A0` bytes; locomotor report | runtime `0x008A07C4` and `0x008A07D0` values |
| Ship bridge-Z formula | verified | `0x0069EBB0` bytes; locomotor report | runtime `0x00B0782C` and `0x00B07838` values |
| Ship hardcoded Rust `360` parity | touched-not-exhausted | Rust grep; ship formula | post-load `g_ShipHeightStep` capture |
| Walk bridge-Z formula | verified for helper | `0x006D2120` bytes; Walk doc | runtime `DAT_00B0CDD8` and caller-side value |
| `DAT_00AC13C8` / `DAT_00AC13BC` | deferred | cold-zero static reads; prior consumer docs | live capture if included in same debugger session |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-1 - Are `DAT_00ABC210` and `DAT_00ABC2D0` runtime-only values? -> No. They are BSS-zero in the cold image but have static initializer code with exact entries.` (evidence: `0x00544691`, `0x00543F10`, `BRIDGE_THEATER_LOAD_TABLE_WRITERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-2 - Is there a separate wood railing table for `FUN_00547230`? -> No. Both `DAT_00ABC1F8` and `DAT_00AA1098` base ranges select the same `DAT_00ABC210` table by local index.` (evidence: `BRIDGE_THEATER_LOAD_TABLE_WRITERS_GHIDRA_REPORT.md`, `BRIDGE_RAILING_SLOT_SUBTILE_SOURCE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-3 - What is `DAT_0089E864` relative to `DAT_0089E870`? -> It is written as `2 * DAT_0089E870` via `*4` then `*0.5` and `ftol`.` (evidence: `0x00489100` byte read)
- `[DEFERRED] OQ-4 - What exact integer does `DAT_0089E870` hold after theater/scenario init?` (category: `needs-runtime-debugger`; reason: source path depends on runtime geometry globals that are BSS-zero in static image; next-step-if-pursued: load a stock YR map and read `0x0089E870` plus upstream geometry globals)
- `[DEFERRED] OQ-5 - Does Rust ship `360` equal live `ftol(g_ShipHeightStep * 4 + 0.5)`?` (category: `needs-runtime-debugger`; reason: `g_ShipHeightStep @ 0x00B07838` is BSS-zero statically; next-step-if-pursued: read `0x00B07838` and `0x00B0782C` post-map-load)
- `[DEFERRED] OQ-6 - Do Drive, Ship, and Walk bridge-Z magnitudes converge numerically?` (category: `needs-runtime-debugger`; reason: all source globals are runtime initialized; next-step-if-pursued: capture `0x008A07D0/0x008A07C4`, `0x00B07838/0x00B0782C`, `0x00B0CDD8`, `0x00B45C28`)
- `[DEFERRED] OQ-7 - Are `DAT_00AC13C8` and `DAT_00AC13BC` equal to common RA2 lepton conventions?` (category: `needs-runtime-debugger`; reason: both cold-read zero; next-step-if-pursued: include them in the same post-load capture)

## 10. Evidence Needed To Mark COMPLETE

Run a non-mutating debugger memory capture after standard YR has loaded a stock map/theater. Minimum addresses:

```text
0x0089E870 0x0089E864
0x008A07D0 0x008A07C4
0x00B07838 0x00B0782C
0x00B0CDD8 0x00B45C28
0x00AC13C8 0x00AC13BC
```

Optional validation capture:

```text
0x00ABC210 length 0xA0
0x00ABC2D0 length 0x280
```

The optional railing-table capture should match the static initializer values above; mismatch would indicate either a later mutator or wrong process/state.

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `DAT_00ABC210` and `DAT_00ABC2D0` have exact static initializer entries, with `DAT_00ABC210` shared by both high/low slope-piece base ranges | `0x00544691`, `0x00543F10`, `0x00547230` docs | partially implemented / needs audit | `src/render/bridge_railing_atlas.rs`, `src/app_instances/bridges.rs`, `src/map/theater.rs` | use static table entries and range-base slot, not overlay names as the binary selector | `bridge_railing_uses_slope_piece_slot_not_subtile` screenshot/table test | Do not wait for debugger capture to fill the table; do not invent a second wood table for this emitter |
| AoE bridge Z constants are lepton-space; `DAT_0089E864 = 2 * DAT_0089E870`, and Block A/B Z gates use `(Level-2)*LevelHeight + BridgeBase` exclusive lower bound | `0x00489100`, `DAT_0089E864` report, dispatcher findings | mismatch suspected | `src/sim/combat/combat_aoe.rs`, `src/sim/bridge_state/mod.rs` | keep Z-window in leptons and thread captured `LevelHeight`/base relation; preserve strict lower bound | `bridge_aoe_z_window_lower_bound_is_level_minus_2_exclusive` deterministic test | Do not compare raw height-level integers to the binary lepton window |
| Drive/Ship bridge-Z offsets are per-locomotor runtime globals with round-half-up `height_step * 4`; Walk uses `(60 - 0.5) * DAT_00B0CDD8` round-down | `0x004AF4A0`, `0x0069EBB0`, `0x006D2120` | current Rust uses level constants / hardcoded ship `360` | `src/util/lepton.rs`, `src/sim/movement/*`, `src/sim/combat/combat_aoe.rs` where Z feeds damage/layering | represent captured lepton constants as runtime-theater constants or verified derived constants; keep per-locomotor split | `ship_bridge_z_offset_matches_captured_gamemd_value` and `walk_bridge_z_rounding_is_half_down` | Do not collapse Drive/Ship/Walk into one `+4 levels` or assume `90*4` without capture |

## 12. Negative Facts / Do Not Do

- Do not call `DAT_00ABC210` unknown pending live debugger capture; its static entries are recovered.
- Do not model `DAT_00ABC210` as separate concrete and wood tables for `FUN_00547230`; the binary uses one table for both slope-piece base ranges.
- Do not treat `DAT_0089E864` as an INI-read `BridgeHeight`; its writer derives it from `DAT_0089E870`.
- Do not treat Rust's `360` as verified ship bridge-Z; it remains an unproven assumption until `0x00B07838/0x00B0782C` are captured.
- Do not port these Z gates in height-level units when the binary consumers compare lepton-space coordinates.

## 13. Stale Docs / Follow-up Wording

Replace stale wording equivalent to:

> Exact railing table values after theater load require a live debugger capture.

with:

> `DAT_00ABC210` and `DAT_00ABC2D0` are BSS-zero in the cold image but their exact entries are recovered from static initializer code at `0x00544691` and `0x00543F10`; live capture is useful only to validate that no later mutator changed them.

Keep wording equivalent to:

> Z-window and locomotor bridge-Z numeric magnitudes require post-map-load debugger reads.

but make it more specific:

> The formula shapes and writer sites are statically verified, but exact post-init magnitudes for `DAT_0089E870`, `0x008A07D0/0x008A07C4`, `0x00B07838/0x00B0782C`, `0x00B0CDD8`, `0x00B45C28`, `0x00AC13C8`, and `0x00AC13BC` require a live post-map-load debugger capture.

## Sources

- Fresh Ghidra `read_memory`: `0x00489030`, `0x00489100`, `0x004AF4A0`, `0x0069EBB0`, `0x006D2120`, `0x00544691`, `0x00543F10`
- Fresh Ghidra `read_memory`: cold static globals `0x0089E864`, `0x0089E870`, `0x00ABC210`, `0x00ABC2D0`, `0x008A07C4`, `0x008A07D0`, `0x00B0782C`, `0x00B07838`, `0x00B0CDD8`, `0x00B45C28`, `0x00AC13C8`, `0x00AC13BC`
- `docs/research/bridges/01-assets-map-load-overlay/BRIDGE_THEATER_LOAD_TABLE_WRITERS_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/DAT_0089E864_BRIDGE_THRESHOLD_IDENTITY_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md`
- `docs/research/bridges/06-render-presentation-audio/BRIDGE_RAILING_SLOT_SUBTILE_SOURCE_GHIDRA_REPORT.md`
- `docs/research/bridges/_parity_scan/dispatcher-rng-gate_findings.md`
- `docs/research/bridges/_parity_scan/locomotion-height_findings.md`

## Status

**PARTIAL.** Static table values and formula identities are recovered. Exact runtime numeric BSS magnitudes are blocked on a post-map-load debugger capture.
