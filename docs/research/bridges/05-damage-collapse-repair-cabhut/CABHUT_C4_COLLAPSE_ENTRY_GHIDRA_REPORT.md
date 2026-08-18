# CABHUT C4 Collapse Entry - Ghidra Research Report

**Address(es):** `0x00519630`, `0x0043FB20`, `0x00574000`, `0x00574C20`, `0x005749C0`, `0x00574780`, `0x00575220`, `0x00575540`, `0x00575870`, `0x00575BA0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** C4-on-BridgeRepairHut collapse entry from C4 marker placement through timer expiry, hut gate, hut-centered bridge selection, canonical collapse start, low/high dispatch, and bounded `CollapseBridge_*_*` sweep.
**Non-Scope:** engineer repair mutation, weapon/AoE bridge damage, low-bridge TubeClass pathing, full trigger action semantics, and campaign scripting.
**Confidence:** High for the claimed slice.
**Active in YR:** Yes. Evidence: `CABHUT` is stock YR content with `BridgeRepairHut=yes` and `Immune=yes` in `ini/rulesmd.ini`; C4-capable infantry (`GHOST`, `TANY`) have C4 via Sapper/C4 data; `BuildingClass::Update` calls `DestroyBridge_Low_OnHutDeath` at `0x00440301` and `DestroyBridge_High_OnHutDeath` at `0x0044031B`; those functions also have runtime xrefs from `BombClass::Detonate`.

Working notes seed:
- `Target question`: Does gamemd.exe route C4-on-CABHUT from plant marker/timer expiry into bridge collapse, and if so what exact collapse entry semantics must Rust mirror?
- `Non-goals`: Do not re-investigate unrelated engineer repair, generic weapon/AoE bridge damage, or campaign trigger side effects except where they directly gate this C4 collapse path.
- `Evidence needed to mark COMPLETE`: decompile plus caller/xref or assembly-range evidence for marker set, timer comparison, BridgeRepairHut branch, hut low/high scan, canonical start selection, and `CollapseBridge_*_*` loop bounds.
- `Stop conditions`: stop after the C4 timer-to-collapse entry slice is resolved; list unrelated bridge fallout, trigger, and pathing questions as remaining uncertainty.

## 1. Overview

C4 on a stock YR BridgeRepairHut is a live path. A C4-capable infantry plants a marker on the building, `BuildingClass::Update` waits until `g_CurrentFrameCounter - field_0x528 >= field_0x530`, and the `BridgeRepairHut` branch diverts before normal building damage. The hut survives; the bridge collapse dispatch runs synchronously from the hut cell.

The collapse entry is not a full-span flood fill. The hut entry first searches a 5x5 area around the hut for a bridge overlay; the first overlay match dispatches through `DestroyBridgeFromCell_Low/High`, which canonicalizes the start cell and calls one of four `CollapseBridge_*_*` walkers. Each walker performs at most four axial steps, with up to three per-step `DestroyBridge_Low/High` retries, and breaks early when the next cell leaves the bridge overlay band.

## 2. Class Layout / Key Offsets

| Class | Offset | Type | Purpose | Active in YR / evidence |
|---|---:|---|---|---|
| `InfantryTypeClass` | `+0xEC2` | byte | `C4=yes` gate for Mission_Sabotage plant branch | Yes. INI reader stores `ReadBool("C4")` at `0x00524559`; `InfantryClass::PerCellProcess` reads `param_1[0x1B0]+0xEC2` at `0x00519630`. |
| `BuildingClass` | `+0x528` | int | C4 plant start frame | Yes. Plant side writes near `0x0051A5A7`; timer branch reads in `BuildingClass::Update` at `0x0043FB20`. |
| `BuildingClass` | `+0x530` | int | C4 delay frames | Yes. Plant side copies rules C4 delay; timer branch compares elapsed against it. |
| `BuildingClass` | `+0x540` | pointer | C4 planter / damage attribution | Yes. Set in plant branch, cleared after CABHUT bridge dispatch at `0x00440327`. |
| `BuildingClass` | `+0x6DF` | byte | C4 pending marker for this path | Yes. Set to `1` at `0x0051A5A7`; cleared to `0` at `0x00440320` after hut dispatch. |
| `BuildingTypeClass` | `+0x1577` | byte | `CanC4` default true | Yes. Constructor sets it to `1` at `0x0045E063`; C4 action gate docs show CABHUT does not opt out. |
| `BuildingTypeClass` | `+0x16B6` | byte | `BridgeRepairHut` | Yes. `ReadBool("BridgeRepairHut")` stores at `0x00460E9A`; `BuildingClass::Update` branches on `this->Type[0x16B6]`. |
| `CellClass` | `+0x38` | int | tile index, used in hut 5x5 low-vs-high pre-scan | Yes. `BuildingClass::Update` compares to `[DAT_00ABAD1C, DAT_00ABAD1C+0x10)`. |
| `CellClass` | `+0x44` | int | overlay index, bridge-family bands | Yes. Hut and entry dispatchers test low `[0x4A..0x65]` and high `[0xCD..0xE8]`. |
| `CellClass` | `+0x140` | uint32 | fallback bridge/ramp flags | Conditional. Used only when the hut entry finds no overlay in the 5x5 inner scan. |

## 3. Core Logic

### C4 plant marker

**Verified behavior:** `InfantryClass::PerCellProcess` handles mission `0x11` only when the infantry type has `C4` at `+0xEC2`. It looks up the building in the current cell and requires it to be the infantry's nav target. If the target is not mission `0x13` and is not Iron Curtain active (`vtable +0x160` returns false), then a clear `BuildingClass+0x6DF` is set to `1`; the function also stores the attacker pointer and timer fields.

**Active in YR:** Yes. This is the standard Tanya/SEAL C4 plant path. Evidence: decompile `0x00519630`; assembly context at `0x0051A5A7` shows `MOV byte ptr [EDI + 0x6df],0x1`; INI reader evidence for C4 flag at `0x00524559`.

**Edge detail:** if `BuildingClass+0x6DF` is already nonzero, the branch does not plant another marker; it redirects the infantry toward the target. Active in YR: Yes, on multi-C4 attempts; evidence: decompile `0x00519630`.

### C4 timer expiry and hut branch

**Verified behavior:** `BuildingClass::Update` only enters the detonation body when `field_0x6DF != 0` and either `field_0x528 == -1 && field_0x530 == 0`, or `g_CurrentFrameCounter - field_0x528 >= field_0x530`. For non-hut buildings it passes current health as C4 damage to vtable `+0x16C`. For BridgeRepairHut buildings it skips vtable `+0x16C`, runs the bridge destruction branch, then clears `+0x6DF` and `+0x540`.

**Active in YR:** Yes. Evidence: decompile `0x0043FB20`; assembly context `0x00440301` / `0x0044031B` shows direct calls to low/high hut destruction entries, followed by `MOV [ESI+0x6DF],0` at `0x00440320` and `MOV [ESI+0x540],0` at `0x00440327`.

**Important ordering:** the `BridgeRepairHut` check is before normal C4 damage. CABHUT's `Immune=yes` is not the reason the hut survives; the hut survives because the branch never calls building damage. Active in YR: Yes. Evidence: `BuildingClass::Update` decompile and `ini/rulesmd.ini` `[CABHUT]`.

### Hut-centered low/high selection

**Verified behavior:** before calling `DestroyBridge_*_OnHutDeath`, `BuildingClass::Update` scans a 5x5 square centered on the hut coordinate (`dx=-2..2`, `dy=-2..2`). If any scanned cell has tile index in `[DAT_00ABAD1C, DAT_00ABAD1C + 0x10)` or low overlay in `(0x49, 0x66)`, the low entry is selected; otherwise the high entry is selected.

**Active in YR:** Yes. Evidence: decompile `0x0043FB20` and direct calls at `0x00440301` low / `0x0044031B` high.

**Bound detail:** both loop counters use `< 3` after starting at `-2`, so the 5x5 scan is inclusive of offsets -2, -1, 0, +1, +2 and exclusive of +3. Evidence: decompile `0x0043FB20`.

### Hut destruction entries

**Verified behavior:** `MapClass::DestroyBridge_Low_OnHutDeath` (`0x00574C20`) and `MapClass::DestroyBridge_High_OnHutDeath` (`0x00574000`) first scan a 5x5 square around the input coordinate looking only for the matching overlay family. Low accepts `(0x49,0x66)`; high accepts `(0xCC,0xE9)`. The first overlay match immediately calls `DestroyBridgeFromCell_Low/High` and returns.

**Active in YR:** Yes. Evidence: decompile `0x00574C20` and `0x00574000`; xrefs from `BuildingClass::Update` at `0x00440301` / `0x0044031B` and `BombClass::Detonate` at `0x0043896A` / `0x00438982`.

**Fallback behavior:** if no overlay is found, the entry reads `CellClass+0x140`; if `(flags & 0x500)==0`, it searches all 8 directions up to 3 cells for a cell with `(flags & 0x500)!=0`. If neither `0x100` nor `0x400` is present after that, it returns without collapse. Otherwise it derives a ramp/bridge anchor from `cell+0x24`, `cell+0x2C`, and flags `0x80`/`0x800`, calls `ApplyDamageToCell` up to three times at ramp sites, then updates adjacent bridges and zones.

**Active in YR:** Conditional. It is in the live runtime function, but only used when the overlay-first 5x5 scan misses. Evidence: decompile `0x00574C20` / `0x00574000`.

### Canonical start-cell selection

**Verified behavior:** `DestroyBridgeFromCell_Low` (`0x00574780`) and `DestroyBridgeFromCell_High` (`0x005749C0`) classify the matched overlay into axis subranges, then probe one and two cells "behind" along that axis. Depending on whether those cells remain in the bridge overlay band, they call the matching collapse walker at `matched + 1`, `matched`, `matched - 1`, or a computed fallback coordinate from `FUN_00588C60`.

**Low subranges:** NS set `[0x4A..0x52]`, `[0x5C..0x5F]`, `0x64`; EW set `[0x53..0x5B]`, `[0x60..0x63]`, `0x65`.

**High subranges:** NS set `[0xCD..0xD5]`, `[0xDF..0xE2]`, `0xE7`; EW set `[0xD6..0xDE]`, `[0xE3..0xE6]`, `0xE8`.

**Active in YR:** Yes. Evidence: decompile `0x00574780` / `0x005749C0`; xrefs show `CollapseBridge_NS_Low` call sites at `0x0057492D`, `0x00574976`, `0x005749AF` and `CollapseBridge_NS_High` call sites at `0x00574B8D`, `0x00574BDA`, `0x00574C13`. `FUN_00588C60` decompile at `0x00588C60` confirms it returns coordinate difference `param_1 - param_3` into the output argument.

### Bounded `CollapseBridge_*_*` sweep

**Verified behavior:** the four collapse walkers (`0x00575220`, `0x00575540`, `0x00575870`, `0x00575BA0`) have the same structure with axis and overlay constants substituted:

1. Measure contiguous bridge-overlay extent backward and forward from the canonical seed.
2. Pick `step = -1` if forward count is less than backward count, otherwise `+1`.
3. Compute start coordinate as `seed_axis - (back_count - forward_count) / 2` using signed integer division.
4. Loop with a hard count of `4`.
5. On each step, if the current center overlay is not the terminal cap (`0x64`/`0x65` low or `0xE7`/`0xE8` high), spawn three bridge explosion anims on perpendicular cells.
6. Call `DestroyBridge_Low` or `DestroyBridge_High` up to three times, stopping early when it returns true.
7. Step one cell along the chosen axis and break if the next overlay leaves the bridge family band.
8. Always call `UpdateBridgeZonesHelper` and set `g_Tactical+0xD7C = 1` on exit.

**Active in YR:** Yes. Evidence: decompile `0x00575220`, `0x00575540`, `0x00575870`, `0x00575BA0`; assembly range `0x005755B0..0x005756D4` spot-checked for the loop body and RNG setup; caller xrefs from `DestroyBridgeFromCell_*`.

**Bound detail:** the loop bound is exactly four axial iterations (`local_2c = 4` / `param_1 = 4` depending on variant), not "until the end of the bridge." Active in YR: Yes. Evidence: decompile of all four walkers.

## 4. INI Keys

| Key | Section / source | YR value | Effect in this slice | Active in YR / evidence |
|---|---|---|---|---|
| `BridgeRepairHut` | `[CABHUT]` in `ini/rulesmd.ini` | `yes` | Selects hut branch in `BuildingClass::Update` and engineer repair branch elsewhere | Yes. `ReadBool("BridgeRepairHut")` stores at `0x00460E9A`; `[CABHUT]` sets it. |
| `Immune` | `[CABHUT]` in `ini/rulesmd.ini` | `yes` | Does not gate this C4 bridge branch | Yes as data; not consulted on the hut branch. Evidence: `ObjectTypeClass::ReadINI` stores Immune at `0x005F9510`; `BuildingClass::Update` hut path skips damage. |
| `CanC4` | BuildingType default | default `yes`; CABHUT does not opt out | Allows C4 action/plant path to exist | Yes. Constructor sets `+0x1577 = 1` at `0x0045E063`; prior action-gate report verifies CABHUT does not override. |
| `C4Delay` | `[CombatDamage]` in `ini/rulesmd.ini` | `.03` minutes | Stored as the C4 timer delay frames | Yes. Binary plant path uses rules C4 delay; current Rust parses to 27 ticks. |
| `C4Warhead` | `[CombatDamage]` | retail warhead name | Used on non-hut C4 damage; not applied to CABHUT hut damage | Conditional. Active for non-hut C4; skipped for `BridgeRepairHut=yes`. |
| `BridgeExplosions` | `[General]` | `TWLT026,TWLT036,TWLT050,TWLT070` | Anim pool for collapse walker cosmetic explosions | Yes. Collapse walkers index `RulesClass+0x15C` using count at `+0x168`. |

## 5. Integration Points

| Point | Status | Evidence | Active in YR |
|---|---|---|---|
| C4-capable infantry on-arrival plant | verified | `InfantryClass::PerCellProcess @ 0x00519630`; `+0x6DF` set at `0x0051A5A7` | Yes |
| Building per-tick C4 timer | verified | `BuildingClass::Update @ 0x0043FB20` | Yes |
| Hut low entry | verified | xrefs from `0x00440301` and `0x0043896A` to `0x00574C20` | Yes |
| Hut high entry | verified | xrefs from `0x0044031B` and `0x00438982` to `0x00574000` | Yes |
| Collapse walkers | verified | `DestroyBridgeFromCell_*` xrefs to `0x00575220`, `0x00575540`, `0x00575870`, `0x00575BA0` | Yes |
| Trigger event `0x1F` fallout | out of scope | already covered by `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md` | Yes but not part of C4 entry decision |

Tick-cycle note: the bridge mutation is synchronous inside `BuildingClass::Update` after timer expiry. The later path/render refresh is signaled by `UpdateBridgeZonesHelper` and `g_Tactical+0xD7C`.

## 6. Current Rust Implementation Status

Scanned Rust surfaces:

| Rust surface | Status vs this slice |
|---|---|
| `src/sim/world/world_orders.rs:387` `tick_c4_plants` | Matches the marker/timer shape at the integration level: current-cell/footprint claim, pending marker, `elapsed < delay` skip, detonation at `elapsed >= delay`. |
| `src/sim/world/world_orders.rs:712` `apply_c4_damage_to_building` | Matches the hut branch order: detects `bridge_repair_hut` before invulnerability/damage, dispatches bridge collapse, preserves hut HP, clears pending marker through caller outcome. |
| `src/sim/world/bridge_orchestrator.rs:170` `dispatch_bridge_collapse_from_hut` | Current comments and code use overlay-first scan, fallback plan, and bounded walker. One doc comment at line 229 still says "full-span flood" and should be corrected when code edits are allowed. |
| `src/sim/world/bridge_orchestrator.rs:670` `run_hut_collapse_bounded` | Matches the verified four-step walker shape: extent measurement, signed bias, max 4 steps, max 3 attempts per step, break on leaving overlay band. |
| `src/sim/world/world_orders_bridge_repair_tests.rs:654` `c4_on_cabhut_collapses_bridge_and_hut_survives` | Covers hut survival and marker cleanup, but the assertion message at lines 721-725 still says "entire bridge span." On larger fixtures that would be wrong; add a long-bridge bounded-footprint test. |
| `src/sim/world/world_orders_bridge_repair_tests.rs:925` and `:953` terminal overlay tests | Good coverage for overlay-first scan not falling through to fallback. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Initial working notes | verified | this report header | none |
| `InfantryClass::PerCellProcess` Mission_Sabotage | verified | decompile `0x00519630`; assembly context `0x0051A5A7` | none for C4 marker |
| C4 already-pending early return | verified | decompile `0x00519630` reads `+0x6DF` before setting | none |
| C4 timer comparison | verified | decompile `0x0043FB20` | none |
| BridgeRepairHut branch before damage | verified | decompile `0x0043FB20`; assembly calls `0x00440301`, `0x0044031B` | none |
| Hut 5x5 low/high selection in `BuildingClass::Update` | verified | decompile `0x0043FB20` | none |
| `DestroyBridge_Low_OnHutDeath` overlay-first scan | verified | decompile `0x00574C20`; xrefs | none |
| `DestroyBridge_High_OnHutDeath` overlay-first scan | verified | decompile `0x00574000`; xrefs | none |
| Hut flag fallback | verified for control structure | decompile `0x00574C20` / `0x00574000` | exact rare-map runtime cases not playtested |
| `DestroyBridgeFromCell_Low` canonical start | verified | decompile `0x00574780`; callsite assembly contexts | none |
| `DestroyBridgeFromCell_High` canonical start | verified | decompile `0x005749C0`; xrefs to collapse walkers | none |
| `FUN_00588C60` coordinate helper | verified | decompile `0x00588C60`; assembly context `0x00588C60..0x00588C6E` | none |
| Four `CollapseBridge_*_*` walkers | verified | decompile all four addresses | none |
| Trigger event side effects | deferred | prior docs only | out of scope for C4 entry |
| Live in-game empirical playtest | deferred | static analysis only | optional runtime confirmation |

## 8. Open Questions - Final State

- `[RESOLVED] Q1 - Is C4 plant on CABHUT live in standard YR? -> Yes, via Mission_Sabotage with `InfantryTypeClass+0xEC2` and stock `CABHUT` data.` (evidence: `0x00519630`, `0x00524559`, `ini/rulesmd.ini`)
- `[RESOLVED] Q2 - What sets the C4 marker? -> `InfantryClass::PerCellProcess` sets `BuildingClass+0x6DF = 1` after target/current-cell and mission gates pass.` (evidence: `0x0051A5A7`)
- `[RESOLVED] Q3 - What is the timer expiry condition? -> fire when elapsed frames are greater than or equal to delay frames.` (evidence: `0x0043FB20`)
- `[RESOLVED] Q4 - Does CABHUT take C4Warhead damage? -> No; `BridgeRepairHut` branch skips vtable `+0x16C`.` (evidence: `0x0043FB20`)
- `[RESOLVED] Q5 - Is `Immune=yes` a gate on this branch? -> No for the hut collapse branch; it is data on CABHUT but not read before bridge dispatch.` (evidence: `0x0043FB20`, `0x005F9510`, `ini/rulesmd.ini`)
- `[RESOLVED] Q6 - What coordinate is passed to bridge entry? -> the hut coordinate from vtable `+0x1B8`.` (evidence: assembly context around `0x00440301` / `0x0044031B`)
- `[RESOLVED] Q7 - Is low/high selection overlay or precomputed span driven? -> tile/overlay scan driven; low if low tile/overlay evidence is seen, high otherwise.` (evidence: `0x0043FB20`)
- `[RESOLVED] Q8 - Does hut entry search only one cell? -> No, it performs a second 5x5 overlay scan in `DestroyBridge_*_OnHutDeath`.` (evidence: `0x00574000`, `0x00574C20`)
- `[RESOLVED] Q9 - What happens if the 5x5 overlay scan misses? -> fallback uses `CellClass+0x140` flags and directional probing; if neither `0x100` nor `0x400` is found it returns.` (evidence: `0x00574000`, `0x00574C20`)
- `[RESOLVED] Q10 - How is canonical start selected? -> `DestroyBridgeFromCell_*` classifies overlay subrange and probes one/two cells back along the axis before calling a collapse walker.` (evidence: `0x00574780`, `0x005749C0`)
- `[RESOLVED] Q11 - Is the collapse full-span? -> No; each `CollapseBridge_*_*` invocation has a hard four-step axial loop and can break earlier.` (evidence: `0x00575220`, `0x00575540`, `0x00575870`, `0x00575BA0`)
- `[RESOLVED] Q12 - Are bounds inclusive? -> low family accepts `0x4A..=0x65`; high accepts `0xCD..=0xE8`; code is expressed as `< lower` / `> upper` breaks or `(lower-1)<overlay && overlay<(upper+1)` tests.` (evidence: decompile of hut entries and collapse walkers)
- `[RESOLVED] Q13 - Does the walker retry per cell? -> Yes, up to three calls to `DestroyBridge_Low/High`, breaking on true return.` (evidence: decompile of all four collapse walkers)
- `[RESOLVED] Q14 - What tail refresh occurs? -> `UpdateBridgeZonesHelper` and `g_Tactical+0xD7C=1` occur at collapse walker tail; hut fallback also updates zones.` (evidence: `0x00575220`, `0x00575540`, `0x00575870`, `0x00575BA0`, `0x00574000`, `0x00574C20`)
- `[DEFERRED] Q15 - Campaign trigger behavior after bridge destroyed event 0x1F.` (category: out-of-scope; reason: does not gate C4 entry or collapse footprint; next-step-if-pursued: use `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`)
- `[DEFERRED] Q16 - Empirical runtime playtest with Tanya/SEAL on a custom CABHUT map.` (category: needs-runtime-debugger; reason: static binary path is complete, but no live playtest was run in this slot; next-step-if-pursued: run a small stock-YR scenario)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| C4 marker is planted only once the C4 infantry is in the target building cell; duplicate marker attempts do not create another timer. | `0x00519630`, `0x0051A5A7` | Mostly matched | `src/sim/world/world_orders.rs:387` | Keep pending marker claim tied to target footprint/current-cell resolution and reject duplicate claims while `pending_c4_detonation` exists. | Two SEALs target same CABHUT; only first sets pending marker and bridge collapse fires once. Proposed test: `c4_on_cabhut_second_sapper_does_not_reset_timer`. | Do not reset C4Delay when a second unit reaches the hut. |
| On timer expiry, `BridgeRepairHut` branches before normal C4 damage, so CABHUT survives and marker clears after bridge dispatch. | `0x0043FB20`, `0x00440320`, `0x00440327`; `BridgeRepairHut` reader `0x00460E9A` | Matched | `src/sim/world/world_orders.rs:712` and existing tests | Preserve branch order: hut dispatch before invulnerability, warhead damage, HP mutation, dying state, or kill-credit update. | Iron-curtained or `Immune=yes` CABHUT with pending C4 still dispatches bridge collapse, hut HP unchanged, marker cleared. Proposed test: `c4_on_immune_cabhut_dispatches_before_damage_gate`. | Do not route CABHUT through generic `apply_damage` and rely on `Immune` to preserve HP. |
| Hut overlay path calls `DestroyBridgeFromCell_*` then a bounded four-step `CollapseBridge_*_*` walker, not a full-span flood fill. | `0x00574780`, `0x005749C0`, `0x00575220`, `0x00575540`, `0x00575870`, `0x00575BA0` | Code appears matched; one existing test message still says entire span | `src/sim/world/bridge_orchestrator.rs:170`, `:670`; `world_orders_bridge_repair_tests.rs` | Keep bounded extent/bias/4-step/3-retry behavior and add a long-bridge fixture proving cells outside the bounded footprint remain standing. | C4 on the middle of a long 3-wide high bridge destroys the bounded ~3x6 footprint and leaves remote span cells intact. Proposed test: `c4_on_cabhut_long_bridge_uses_bounded_four_step_sweep`. | Do not implement a full-span BFS/flood fill or assert that every bridge cell must be destroyed. |

### Negative Facts / Do Not Do

- Do not treat `CABHUT Immune=yes` as blocking C4-on-hut collapse. Evidence: `BuildingClass::Update @ 0x0043FB20` reads `BridgeRepairHut` and skips damage; `Immune` reader exists at `0x005F9510` but is not on this branch. Active in YR: Yes.
- Do not damage or kill the hut to trigger bridge collapse. Evidence: non-hut damage call is inside `Type[0x16B6] == 0`; hut branch dispatches bridge destruction and clears marker. Active in YR: Yes.
- Do not require a precomputed Rust span anchor for the overlay-first path. Evidence: hut entries scan cell overlays and `DestroyBridgeFromCell_*` canonicalizes from the matched overlay cell. Active in YR: Yes.
- Do not turn CABHUT collapse into a full-span flood fill. Evidence: every `CollapseBridge_*_*` function uses a hard four-step loop and breaks if the next cell leaves the overlay band. Active in YR: Yes.
- Do not fall through to fallback when the overlay-first scan hits a terminal overlay (`0x64`/`0x65`/`0xE7`/`0xE8`). Evidence: hut entry immediately calls `DestroyBridgeFromCell_*` and returns on the first overlay-family hit. Active in YR: Yes.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/CABHUT_PER_CELL_DESTRUCTION_CASCADE_TRACE.md`
  - Replace lines 100-105 and 130-133 wording with: "gamemd's CABHUT overlay path calls `DestroyBridgeFromCell_*`, which canonicalizes from the matched overlay cell and then calls a bounded `CollapseBridge_*_*` walker. The walker measures both axial extents, biases the start point, runs at most four axial steps with up to three per-step `DestroyBridge_*` attempts, and stops early when the next cell leaves the bridge overlay band. It does not walk the entire bridge span in one call."
  - Replace lines 389-394 wording with: "`DestroyBridgeFromCell_High @ 0x5749C0` should be modeled as canonical start selection plus bounded collapse dispatch, not as a single-shot full-span collapse."

## Sources

- Ghidra decompile: `0x00519630` `InfantryClass::PerCellProcess`
- Ghidra decompile: `0x0043FB20` `BuildingClass::Update`
- Ghidra decompile: `0x00574000` `MapClass::DestroyBridge_High_OnHutDeath`
- Ghidra decompile: `0x00574C20` `MapClass::DestroyBridge_Low_OnHutDeath`
- Ghidra decompile: `0x005749C0` `MapClass::DestroyBridgeFromCell_High`
- Ghidra decompile: `0x00574780` `MapClass::DestroyBridgeFromCell_Low`
- Ghidra decompile: `0x00575220`, `0x00575540`, `0x00575870`, `0x00575BA0` collapse walkers
- Ghidra decompile: `0x00588C60` coordinate helper
- Ghidra xrefs: `0x00574000`, `0x00574C20`, `0x00575BA0`, `0x00575540`
- Ghidra assembly contexts: `0x00440301`, `0x0044031B`, `0x00440320`, `0x0051A5A7`, `0x00460E9A`, `0x0045E063`, `0x00524559`, `0x00588C60`
- `ini/rulesmd.ini` `[CABHUT]`, `[CombatDamage]`, `[General]`, `[GHOST]`, `[TANY]`
- Prior docs: `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`, `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`, `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md`, `traces/CABHUT_C4_TIMER_EXPIRY_BRIDGE_BRANCH_TRACE.md`
