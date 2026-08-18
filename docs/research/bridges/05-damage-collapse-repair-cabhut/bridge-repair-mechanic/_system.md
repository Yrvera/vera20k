# Bridge Repair Mechanic — System Synthesis

> ## ⚠️ INVALIDATED — DO NOT CONSUME THE HEADLINE FINDING
>
> **Status: SYNTHESIS INTERPRETATION IS WRONG.** A team-lead spot-check after user approval re-decompiled `InfantryClass::PerCellProcess` and found the headline DRIFT diagnosis is based on a decoder-doc error that the v3 proofer did not catch on re-submission.
>
> ### What's wrong
>
> The decode doc `fn-InfantryClass-PerCellProcess-C4Plant.md` has a section titled "Bridge-Specific Path: High vs. Low Bridge Detection" that places it under the Mission 0x11 (C4) flow. **The code block that calls `ProcessBridgeDestruction_Low/High` immediately is actually in the Engineer branch (Mission 8/0xB/0x19 with `+0xec3` Engineer flag), NOT the C4 branch.** That code is the Engineer's bridge-REPAIR path (the function name "ProcessBridgeDestruction" is misleading — it's the repair orchestrator).
>
> rust-comparer read the wrong attribution and produced DRIFT rows #15, #16, #63 of `_parity.md` based on the false claim that "gamemd fires immediately on SEAL arrival." This is not what gamemd does for the C4 path.
>
> ### What gamemd actually does on C4-on-CABHUT
>
> SAME timer flow as Rust:
> 1. SEAL Mission_0x11 arrives at CABHUT cell.
> 2. `InfantryClass::PerCellProcess` sets `BuildingClass+0x6DF=1` plus timer fields (`+0x528`, `+0x52C`, `+0x530`, `+0x540`).
> 3. SEAL animates the plant.
> 4. Later: `BuildingClass::Update` sees timer expired, branches on `BuildingTypeClass+0x16B6` (BridgeRepairHut), calls `DestroyBridge_Low/High_OnHutDeath`.
> 5. Bridge collapses; hut survives.
>
> **Rust does the same flow.** The "immediate vs timer" DRIFT diagnosis in this doc is false.
>
> ### What the actual port bug might be (unverified)
>
> The known bug "right-click CABHUT does nothing observable in Rust" requires a different investigation. Possible mechanisms (need verification):
>
> 1. CABHUT footprint impassability → SEAL never satisfies `target_footprint.contains(&attacker_cell)` in `tick_c4_plants` Phase 1 (`world_orders.rs:478`) → marker never set.
> 2. `Look_up_building_in_cell()` in gamemd may resolve "building present in this cell" with different semantics than Rust's footprint check.
> 3. SEAL pathfinding may never reach CABHUT cells in Rust due to passability flags.
> 4. The C4 cursor displays but the order issue / mission assignment path may diverge before reaching `tick_c4_plants`.
>
> ### What this run still got right
>
> - The 30 per-symbol decode docs are individually valid as references (most claims checked by proofer).
> - The 77 MATCH/INTERNAL-ONLY rows in `_parity.md` are accurate — the bridge collapse cascade, repair walkers, zone rebuild, INI surface, struct fields all match.
> - The Phase 0 expansion (engineer-repair-path symbols + audio-callsite closure) is real coverage.
> - The 4 proofer REJECTs caught real INI cross-check errors that improved doc quality.
>
> ### What's invalidated
>
> - `_parity.md` rows #15, #16, #63 (the 3 HIGH DRIFT rows) — DRIFT diagnosis is wrong.
> - The "Summary" + "Control flow" + "Highest-leverage findings" + "Implementation recommendations" sections of this synthesis — built on the wrong premise.
> - The decode doc `fn-InfantryClass-PerCellProcess-C4Plant.md` section "Bridge-Specific Path: High vs. Low Bridge Detection" — wrong branch attribution.
>
> ### Recovery path
>
> 1. Treat the headline finding as unknown. Do NOT use this synthesis to drive `/brainstorm` → `/write-plan` for a fix.
> 2. Run a targeted `/re-investigate` on the actual right-click → C4-plant → CABHUT chain in Rust to find where the real silent-failure point is.
> 3. Optionally: re-run `/decode-system bridge-repair-mechanic --resume` after the Rust C4-on-CABHUT plant chain is re-investigated, to produce a corrected parity report.
>
> ### Skill lesson (for v6+)
>
> v3 proofer's REJECT cycle worked once (#27 was rejected with "bridge path in wrong branch") but the re-accept didn't independently verify the fix actually moved the section to the correct branch. The decoder's fix was probably cosmetic. For v6, proofer should re-verify by re-running the citation pass on the same claim, not just trust the doc structure. Plus: team-lead's mandatory real-time spot-check (v2/v5 HARD-GATE) was not actually executed — I poked teammates when stalled but didn't spot-check completed decode citations. That gap is what let this slip through.

---

**Synthesized:** 2026-05-24
**Source:** 30 per-symbol decode docs in this directory + `_parity.md` (80 rows after rust-comparer completed the engineer-repair-path compares post-recycling-of-deleted-IDs).
**Status:** ⚠️ **INVALIDATED** — see correction block above. Body below is preserved for reference but should not be consumed as ground truth.

---

## Summary

The bridge-repair-mechanic system covers all gamemd.exe code paths involving the **CABHUT** (`BridgeRepairHut=yes`) building: the C4-on-CABHUT bridge-destruction path (SEAL/Tanya planting C4 on a bridge repair hut), the engineer-repair path (Engineer entering CABHUT to repair an adjacent broken bridge), and the bridge-collapse cascade that fires from either entry point. The deliverable identifies the **C4-on-CABHUT port bug** — a known Rust-side regression where right-clicking CABHUT with a SEAL/Tanya selected produces no observable result — and pinpoints its exact root cause.

**Player-facing behavior in gamemd:**

- **C4 path:** SEAL/Tanya walks to CABHUT, arrives at the building's cell, and the bridge collapses *immediately on that same frame*. The hut itself is not damaged (CABHUT has `Immune=yes` in `rulesmd.ini`, but more importantly, the C4-plant code path for CABHUT short-circuits to bridge destruction rather than running the standard C4 timer + building damage flow). All units near the destroyed bridge cells are dropped or destroyed; pathfinding zones rebuild; bridge debris animations play.
- **Repair path:** an Engineer walks to CABHUT, arrives at the cell, and the adjacent broken bridge is restored — span by span, with `BridgeRepaired` (`RepairBridgeSound=`) playing at the engineer's position on completion.
- **Hut survival:** CABHUT survives both paths. It can be C4'd repeatedly (well, until the bridge is gone) and repaired by re-entering.

**Player-facing behavior in the Rust port (current bug):**

- **C4 path:** SEAL/Tanya cursor shows "Demolish" (correct). Right-click on CABHUT does nothing observable. The bridge does not collapse on arrival; the bridge does not collapse on the C4 timer; the hut does not die. **Root cause confirmed by this run** — see DRIFT findings below.
- **Repair path:** works correctly (MATCH per `RepairBridgeSound` row 41 + repair walker compares).

---

## Symbol scope

**Inventory of decoded symbols:**

| Kind | Count | Notable |
|---|---|---|
| Functions | 17 | `InfantryClass::PerCellProcess` (C4-plant entry), `BuildingClass::Update` (timer-expiry dispatch), 6-function collapse cascade chain, 4 collapse walkers, 4 helpers (`UpdateBridgeZonesHelper`, `UpdateAdjacentBridges_High`, `IsBridgeRampTile`, `IsLowBridgeEndpointTile`), 3 INI readers, 6 Phase 0 additions (`ProcessBridgeDestruction_Low/High`, `RepairBridge_Low/High`, `DestroyBridge_Low/High`) |
| Structs | 5 | BuildingClass C4 fields (+0x528, +0x52C, +0x530, +0x540, +0x6DF), BuildingTypeClass bridge fields (+0x1577 CanC4, +0x16B6 BridgeRepairHut, +0x1701 InvisibleInGame), ObjectTypeClass gate fields (+0x22E Bombable, +0x231 LegalTarget, +0x232 Insignificant, +0x233 Immune — corrected from prior +0xC4D claim), CellClass bridge fields, InfantryTypeClass C4 + Engineer |
| Strings | 3 | `BridgeRepairHut`, `RepairBridgeSound` (the §19 audio-callsite gap closure), `BridgeStrength` |
| Globals | 0 | none decoded standalone (`g_CurrentFrameCounter` and `g_RulesClass_Instance` referenced incidentally) |

**Total: 25 symbols** (after removing 1 redundant string decode and 1 broken decode-with-wrong-address that was deleted during the run).

**TS-legacy excluded by scope-explorer Phase 0:** 6 symbol groups (UpdateRamp_* family ~20 fns, InvalidateBridgeZones, SetBridgeDirection helpers, UpdateAdjacentBridges_Low, BridgeOptions flags, BridgePavementSpanWalker). All deferred to follow-up if surface; none on the C4 or repair hot paths.

---

## Control flow

```
                                  C4 path (SEAL/Tanya)                                    Repair path (Engineer)
                                  =====================                                    =====================

Player click on CABHUT → SEAL pathfinds to building            Player click on damaged bridge → Engineer pathfinds to CABHUT
                       │                                                                   │
                       ▼                                                                   ▼
   InfantryClass::PerCellProcess (0x00519630) — cell-arrival entry         InfantryClass::PerCellProcess
   Mission_Sabotage branch (mission 0x11), C4 flag check (InfTC+0xEC2)     Engineer branch (InfTC+0xEC3)
                       │                                                                   │
                       ▼                                                                   ▼
              IronCurtain guard (vtable+0x160 on target)        Bridge type detection (5×5 scan around CABHUT for bridge overlay)
              "Already planted" guard (BC+0x6DF)                              │
                       │                                                     ▼
                       ▼                            ProcessBridgeDestruction_Low (0x00570050) or
        Type 6 + BridgeRepairHut detected?          ProcessBridgeDestruction_High (0x00573540)
                       │                                                     │
              ┌────────┴────────┐                          (5×5 overlay scan, fast-path)
              │ YES (CABHUT)    │ NO (normal building)                       │
              │                 │                                            ▼
              ▼                 ▼                                  RepairBridge_Low / RepairBridge_High
   ★ IMMEDIATE BRANCH ★    Set BC+0x6DF = 1                       (Walk span, restore tiles, rebuild zone)
   5×5 bridge scan         Set BC+0x528 = frame                                │
   Call ProcessBridge      Set BC+0x530 = countdown                            ▼
   Destruction_Low/High    Set BC+0x540 = SEAL ptr                  EVA_BridgeRepaired sound emit
   Clear all infantry      Stop SEAL, route to plant                Zone rebuild (UpdateBridgeZonesHelper)
   nav-target refs                       │
   building.KillAnim()                   │
                                         ▼ (...some ticks later...)
                                BuildingClass::Update (0x0043FB20)
                                C4 timer expires for normal building
                                Damage building (vtable+0x16C → TakeDamage)
                                Eventually: building destroyed
              │
              ▼
   ProcessBridgeDestruction_Low/High
              │
              ▼
   DestroyBridge_Low/High_OnHutDeath (0x00574000 / 0x00574C20)
              │
              ▼
   5×5 overlay scan → DestroyBridgeFromCell_Low/High
              │
              ▼
   Canonical-start anchor selection
              │
              ▼
   CollapseBridge_NS_Low/EW_Low/NS_High/EW_High (4 walkers)
   - 4 axial iterations, 3-retry inner loop
   - 3 perpendicular debris animations per step
   - 4-call RNG order per anim (X-jitter, Y-jitter, delay, anim-index)
   - DestroyBridge_Low / DestroyBridge_High per-cell tile destroy
              │
              ▼
   UpdateBridgeZonesHelper (unconditional zone rebuild)
   UpdateAdjacentBridges_High (rim cells updated)
   g_Tactical+0xD7C = 1 (dirty screen)
              │
              ▼
   Bridge destroyed. CABHUT survives.
```

**Crucial detail:** the C4 path for CABHUT **never goes through** `BuildingClass::Update`'s timer-expiry dispatch (the right-hand side of the "Type 6 + BridgeRepairHut" branch). Only normal C4 targets do. CABHUT is shorted out at the `PerCellProcess` step into `ProcessBridgeDestruction_*` directly.

The `BuildingClass::Update` bridge branch (5×5 scan + dispatch) *exists* in gamemd, but is only reachable via `BombClass::Detonate` (demo-truck explosions and similar AoE events, not C4 plants). Verified per `get_function_callers 0x00574C20`: 2 callers — `BombClass::Detonate` and `BuildingClass::Update`.

---

## State machine

### Per-building C4 state (normal C4 targets, NOT CABHUT)

```
                 ┌──────────────────┐
                 │ NOT PLANTED       │   +0x6DF = 0, +0x528 = -1
                 └────────┬─────────┘
                          │ SEAL arrives + plants C4
                          ▼
                 ┌──────────────────┐
                 │ PLANTED, COUNTING │   +0x6DF = 1
                 │                   │   +0x528 = current_frame
                 │                   │   +0x530 = countdown
                 │                   │   +0x540 = SEAL ptr
                 └────────┬─────────┘
                          │ frame_now − +0x528 >= +0x530
                          │ (timer expires in BuildingClass::Update)
                          ▼
                 ┌──────────────────┐
                 │ DETONATED         │   For normal building: building dies via TakeDamage
                 └──────────────────┘   For CABHUT: dead branch — this path doesn't apply
```

### CABHUT path (no per-building C4 state required)

CABHUT bypasses the state machine entirely:

```
                 ┌──────────────────┐
                 │ NORMAL (READY)    │   no C4 fields written
                 └────────┬─────────┘
                          │ SEAL with C4 arrives at CABHUT cell
                          │ PerCellProcess detects BridgeRepairHut
                          ▼
                 ┌──────────────────┐
                 │ BRIDGE COLLAPSING │   ProcessBridgeDestruction_Low/High fires
                 │                   │   ON THE SAME FRAME — no timer
                 └────────┬─────────┘
                          │ Cascade completes (single frame, animations
                          │ scheduled for subsequent frames)
                          ▼
                 ┌──────────────────┐
                 │ HUT NORMAL,       │   CABHUT survives; bridge gone
                 │ BRIDGE GONE       │   Engineer can re-enter to repair
                 └──────────────────┘
```

---

## INI surface

| Key | Section | Default (stock YR) | Storage | Effect |
|---|---|---|---|---|
| `BridgeRepairHut` | `[<building section>]` (CABHUT has this) | `no` for buildings generally; `yes` for CABHUT (rulesmd.ini:16348) | `BuildingTypeClass+0x16B6` byte | Marks a building as a CABHUT-style bridge repair hut. Gates the immediate-collapse branch in `PerCellProcess` and the timer-expiry branch in `BuildingClass::Update`. |
| `RepairBridgeSound` | `[AudioVisual]` | `BridgeRepaired` (rulesmd.ini:721) | `RulesClass+0x248` (VocClass index) | Sound emitted at the engineer's position when bridge-repair completes. |
| `BridgeStrength` | `[CombatDamage]` | `1500` (rulesmd.ini:816) | `RulesClass+0x1740` int | Bridge tile HP vs weapon fire (out of C4/repair scope; included as part of the bridge INI surface). |
| `C4` | `[<infantry section>]` (SEAL, Tanya have this) | varies | `InfantryTypeClass+0xEC2` byte | Gates whether infantry can plant C4 on buildings/bridges. |
| `Engineer` | `[<infantry section>]` (Engineer has this) | varies | `InfantryTypeClass+0xEC3` byte | Gates whether infantry can repair bridges. |
| `Immune` | `[<object section>]` (CABHUT has `Immune=yes` per rulesmd.ini:16340) | `0` (false) | `ObjectTypeClass+0x233` (corrected from prior +0xC4D) | Gates `ReceiveDamage` — Immune objects take 0 weapon damage. CABHUT's `Immune=yes` means it survives stray weapon fire. **NOT in the C4 plant call chain** — the C4 plant path checks `BridgeRepairHut`, not `Immune`. |
| `CanC4` | `[<building section>]` | `1` (true) for all buildings; CABHUT inherits the default | `BuildingTypeClass+0x1577` byte | Gates whether C4 cursor appears on the building. Default-true means CABHUT is C4-clickable without explicit opt-in. |

---

## Observable behaviors

For each input, the listed output is what the player observes in gamemd:

| Trigger | gamemd observable output |
|---|---|
| Select SEAL/Tanya, right-click CABHUT (enemy bridge repair hut) | SEAL paths to CABHUT. The frame SEAL arrives at the building's cell, **the bridge collapses immediately**. Bridge debris animations spawn. Sound: collapse. Hut survives. Any units on the destroyed bridge cells fall. Pathfinding zones update — units paths recompute around the gone bridge. |
| Select SEAL/Tanya, right-click any non-CABHUT enemy building | SEAL paths to building. On arrival, plants C4 (visible bomb sprite on building). C4Planted sound. Building dies after `c4_delay` frames. Standard damage path. |
| Select Engineer, right-click damaged bridge spans | Engineer paths to nearest CABHUT. On arrival, repairs adjacent bridge span by span. `BridgeRepaired` sound emitted at engineer position on completion. Pathfinding zones rebuild. |
| Demo truck explosion near CABHUT | `BombClass::Detonate` → `DestroyBridge_*_OnHutDeath` directly (bypasses CABHUT detection, hits bridge collapse cascade). Bridge collapses; CABHUT may or may not survive depending on AoE. |
| C4 a normal building during `Immune=yes` | C4 plant proceeds normally (Immune doesn't gate the C4 plant code path; only `ReceiveDamage`). Building takes C4 damage when timer expires — which respects `Immune` via `ReceiveDamage` early-out. So `Immune=yes` buildings are unaffected by C4 *damage*, but still go through the timer state. (Note: CABHUT bypasses this entirely via the bridge branch.) |
| Right-click CABHUT with second SEAL while another C4 is already pending | gamemd: `BuildingClass+0x6DF` would be set if this was a normal C4 target — but CABHUT never writes +0x6DF, so the "already planted" guard doesn't apply. The second SEAL also fires immediately (probably destroying nothing further if the bridge is gone). |
| Reload a saved game mid-C4-countdown | C4 state on normal buildings (+0x528, +0x6DF) is part of the building save state. (Outside the C4-on-CABHUT path; CABHUT has no per-instance C4 state to save.) |

---

## Parity report rollup

Full per-row report in `_parity.md` (51 rows). Highest-leverage findings (HIGH severity, player-visible, every-match-on-trigger):

### HIGH

1. **C4-on-CABHUT detonation mechanism is wrong (DRIFT — root cause).** gamemd fires bridge collapse IMMEDIATELY on SEAL cell arrival via `InfantryClass::PerCellProcess` → `ProcessBridgeDestruction_Low/High`. Rust routes CABHUT through `tick_c4_plants` Phase 1 → `pending_c4_detonation` timer → `BuildingClass::Update`-equivalent timer path. **In Rust the bridge collapses after the C4 countdown delay; in gamemd it collapses the frame SEAL arrives.** Sources: `_parity.md` row #15 (compare #51), row #63 (compare #52), row #16 (footprint-entry sub-finding).

2. **C4-on-CABHUT footprint-entry requirement causes silent failure (DRIFT — symptom).** Rust's `tick_c4_plants` Phase 1 requires `target_footprint.contains(&attacker_cell)` before setting `pending_c4_detonation`. If CABHUT's footprint cells are impassable (no-enter terrain), SEAL never satisfies the check, no C4 marker is ever set, **the bridge never collapses**. gamemd has no such footprint requirement — `PerCellProcess` fires bridge destruction the frame SEAL reaches the building cell, period. Source: `_parity.md` row #16 (compare #51 row 2).

3. **Same root cause confirmed from InfantryClass side (DRIFT).** gamemd's `PerCellProcess` Mission_Sabotage branch unambiguously calls `ProcessBridgeDestruction_Low` or `_High` immediately when the target is a BridgeRepairHut, **without writing +0x6DF**. Rust has no equivalent immediate-dispatch branch — it routes everything through the unified C4 timer path. Source: `_parity.md` row #63 (compare #52).

### MATCH / INTERNAL-ONLY (no parity drift)

77 rows of MATCH and INTERNAL-ONLY findings (3 HIGH DRIFT + 77 clean = 80 total). The bridge collapse cascade itself AND the engineer-repair path are both implemented correctly in Rust:

- **5×5 scan order, overlay band detection, fallback flag walk** (rows 17–22)
- **4 collapse walkers** — all 4-iteration loops, 3-retry inner, 3 perpendicular debris per step, **RNG lockstep preserved** for multiplayer determinism (rows 28–34)
- **Zone rebuild + adjacent bridge update** unconditional after collapse (rows 35–37)
- **Tile/overlay/flag detection** (`IsBridgeRampTile`, `IsLowBridgeEndpointTile`) (rows 38–39)
- **INI surface** parsed identically — `BridgeRepairHut`, `RepairBridgeSound`, `BridgeStrength` all MATCH (rows 41, 65, 66, 40)
- **Engineer repair path** — `ProcessBridgeDestruction_Low/High` scan + fast path + zone rebuild all MATCH (rows 67–73)
- **RepairBridge_Low/High walkers** — overlay axis split (NS/EW sub-ranges), 3-case anchor detection, all MATCH (rows 74–77)
- **DestroyBridge_Low per-cell tile destroyer** — overlay range check, anchor detection, not-bridge return signal all MATCH/INTERNAL-ONLY (rows 78–80)
- **C4 timing fields, gate flags, infantry type flags** — all INTERNAL-ONLY (rows 42–49, 58–60)

**The bridge collapse cascade itself is solid.** The bug is only at the entry point — how C4 on CABHUT gets routed in the first place.

---

## Edge cases / known parity hazards

1. **The CABHUT bypass is unique.** `PerCellProcess` has a *special* immediate-collapse branch for `Type == 6 + BridgeRepairHut=yes`. No other building type uses this path. Implementing it requires recognizing the BridgeRepairHut flag specifically and short-circuiting the standard C4 plant flow.

2. **`+0x6DF` is dual-purpose.** Per the prior research (Phase 2 §14 of `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`), `BuildingClass+0x6DF` is set by both the C4-plant flow AND a Crewed-survivor cooldown. CABHUT only writes the C4 side (and in fact, CABHUT *doesn't even write it* on the bridge branch — the field is bypassed). The dual-purpose semantic doesn't apply to the CABHUT bug fix.

3. **`+0x140 = 0x500` bitmask** is the bridge-present gate for fallback flag walk. Identical bitmask values in Rust (`BRIDGE_FLAG_STRUCTURAL|BRIDGE_FLAG_DESTROYED_OR_RAMP`).

4. **Collapse walker RNG order** is load-bearing for multiplayer determinism: X-jitter → Y-jitter → frame-delay → anim-index. Rust preserves this exactly. Do not reorder.

5. **`Immune=yes` is NOT the C4 gate.** This was the prior hypothesis (refuted by the 2026-05-12 GATE INVESTIGATION and re-confirmed by this run). `Immune` gates `ReceiveDamage`, not `PerCellProcess` C4 plant. Don't add an `Immune` check to the Rust C4 plant code — it would be wrong.

6. **Theater-specific tile-type globals.** `DAT_00ABAD1C`, `DAT_00ABC2B4`, etc. are theater-init globals (set per map at load). The Rust port uses static terrain flags (`is_wood_bridge_repair_tile`, `bridge_transition`) instead — semantically equivalent on the C4 path, but if anyone needs to compare ramp detection logic exactly, the gamemd side reads theater globals at runtime.

7. **CABHUT footprint impassability.** If the CABHUT's foundation cells are coded as impassable for infantry (likely for game-design reasons — CABHUT looks small but covers multiple cells the unit can't walk onto), this combined with Rust's footprint-entry check produces the silent-failure symptom. The fix described below avoids the footprint check for CABHUT.

---

## Per-symbol doc index

**Functions (17):**
- C4 path: `fn-InfantryClass-PerCellProcess-C4Plant.md` (★ key doc), `fn-BuildingClass-Update-BridgeBranch.md`
- Collapse cascade: `fn-DestroyBridge-{Low,High}-OnHutDeath.md`, `fn-DestroyBridgeFromCell-{Low,High}.md`, `fn-CollapseBridge-{NS,EW}-{Low,High}.md` (4 walkers)
- Helpers: `fn-UpdateBridgeZonesHelper.md`, `fn-UpdateAdjacentBridges-High.md`, `fn-IsBridgeRampTile.md`, `fn-IsLowBridgeEndpointTile.md`
- INI readers: `fn-BuildingTypeReadINI-BridgeRepairHut.md`, `fn-ReadAudioVisual-RepairBridgeSound.md`, `fn-ReadCombatDamage-BridgeStrength.md`
- Engineer-repair path (Phase 0 additions): `fn-ProcessBridgeDestruction-{Low,High}.md`, `fn-RepairBridge-{Low,High}.md`, `fn-DestroyBridge-{Low,High}.md` (per-cell tile destroyers)

**Structs (5):**
- `struct-BuildingClass-C4-fields.md`, `struct-BuildingTypeClass-BridgeFields.md`, `struct-ObjectTypeClass-GateFields.md` (★ Immune offset correction), `struct-CellClass-BridgeFields.md`, `struct-InfantryTypeClass-C4-Engineer.md`

**Strings (3):**
- `string-BridgeRepairHut.md`, `string-RepairBridgeSound.md` (★ §19 audio gap closure), `string-BridgeStrength.md`

---

## Implementation recommendations (for /brainstorm → /write-plan)

The parity report's 3 HIGH DRIFT rows translate into a precise port-bug fix:

### Fix: add the CABHUT immediate-collapse short-circuit to Rust's C4 plant flow

Currently, Rust's `tick_c4_plants` Phase 1 (`world_orders.rs:478` per `_parity.md` row #16) checks `target_footprint.contains(&attacker_cell)` before setting `pending_c4_detonation`. CABHUT's footprint cells are impassable, so SEAL can't reach them and the C4 marker is never set.

**Proposed fix path:**

1. **Detect CABHUT in the C4 plant entry path.** When the SEAL has Mission_Sabotage targeting a building, BEFORE the footprint-entry check, branch on `target_type.bridge_repair_hut`.
2. **For BridgeRepairHut targets:** the trigger condition should be SEAL arriving adjacent to (or in) the CABHUT's cell-footprint perimeter — not strictly inside the footprint. Match gamemd's `Look_up_building_in_cell()` semantic: "is the SEAL's current cell associated with a CABHUT in the standard way infantry interact with buildings?"
3. **On trigger, IMMEDIATELY call the equivalent of `ProcessBridgeDestruction_Low/High`** (the engineer-repair-path orchestrator). Rust's bridge orchestrator at `bridge_orchestrator.rs:151` has the entry — `dispatch_bridge_collapse_from_hut` — that already exists. Bypass `pending_c4_detonation` entirely for CABHUT targets.
4. **Do NOT set `pending_c4_detonation` for CABHUT targets.** Don't run the C4 timer for them. The whole chain through `BuildingClass::Update`-equivalent is dead code for CABHUT in gamemd.
5. **After the collapse fires, clear the SEAL's C4 nav-target reference** (matches gamemd's "sweep all infantry, clear nav-target to destroyed hut" loop) and play `building.KillAnim()` equivalent if any visual cue is needed (gamemd does this; check Rust's equivalent).

### Secondary fixes (lower-leverage, low priority)

6. **Confirm the `RepairBridgeSound` emit position.** Per `_parity.md` row #41, Rust emits at the engineer's position — but verify it fires on completion-frame and not earlier (i.e., when repair *starts*).
7. **`ObjectTypeClass+0x233 Immune` field is not implemented in Rust.** Per `_parity.md` row #50, it's not in the C4 plant path (so no fix needed for the CABHUT bug), but the field is missing for weapon-damage immunity in general. Out of scope for this report but worth noting in a follow-up disparity-scan if CABHUT/superweapons-immunity is ever player-visible.

---

## Open questions to surface to user before downstream consumption

1. **Theater-tile-init globals (`DAT_00ABAD1C`).** Used by `ProcessBridgeDestruction_Low` for tile-type dispatch arithmetic. Inferred role; runtime values not directly decoded. If ramp/edge tile classification produces drift in a specific theater (snow, urban), this is the place to dig. Likely NOT blocking for the C4 fix.
2. **`BuildingClass+0x52C` field** — written by Crewed-survivor path; not written by CABHUT bridge branch. Confirmed not in C4-on-CABHUT scope. If general C4-damage-receive parity is later audited, re-check this field's semantics.
3. **`UpdateAdjacentBridges_High` visual edge tile updates** — Rust uses span-head records, gamemd updates tile visuals directly. INTERNAL-ONLY per parity row #37, but if a player notices bridge-edge visual artifacts after collapse, this is the place to investigate.
4. ~~Engineer-repair-path parity~~ — **CLOSED.** The deleted Phase 0 compares (#80/83/86/89/92) were recreated with new task IDs by rust-comparer and processed. Rows 67–80 of `_parity.md` cover the engineer-repair path completely; all MATCH or INTERNAL-ONLY. No new DRIFT in this path.

---

## Verdict

The system is well-bounded, well-decoded, and the **C4-on-CABHUT port bug is precisely diagnosed**. The fix is localized:

- Touch `world_orders.rs` `tick_c4_plants` to detect `target_type.bridge_repair_hut` BEFORE the footprint check
- Route detected CABHUT targets to `bridge_orchestrator.rs::dispatch_bridge_collapse_from_hut` immediately
- Skip `pending_c4_detonation` for CABHUT (no timer)
- Test the fix against the existing ignored test `c4_on_cabhut_destroys_bridge_when_upstream_immune_lifted` (which already expects this behavior)

Estimated implementation effort: small. The bridge-orchestrator + collapse cascade already works correctly (verified by 30+ MATCH/INTERNAL-ONLY parity rows). Only the entry-point routing needs a new short-circuit branch.

Downstream: feed `_system.md` to `/brainstorm` for design spec, then `/write-plan` for implementation steps.
