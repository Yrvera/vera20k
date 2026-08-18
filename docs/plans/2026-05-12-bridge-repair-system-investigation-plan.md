# Bridge Repair System — Investigation Plan

> **For Claude:** This plan scopes a `/re-investigate` pass covering the bridge
> repair + hut-death-destroys-bridge path. Execute by running
> `/re-investigate bridge repair` with this plan loaded as context, OR dispatch
> the function inventory to subagents in batches.

**Topic:** Bridge repair via engineer entering BridgeRepairHut (CABHUT)
**PLUS** the hut-death-destroys-bridge path (C4 / strength=0 → bridge
collapse), because the scoping pass confirmed they share the walker function
in the binary.

**Scope Size:** Medium-Large — ~28 functions, ~3 INI keys, 1 major doc
correction confirmed during scoping.

**Est. Effort:** ~7–11h of `/re-investigate` work
(~6 FULL × 20-30min + ~8 MEDIUM × 8-10min + ~14 LIGHT × 3-5min).

**Prior Research:**
- [BRIDGE_SYSTEM.md](../../../ra2-rust-game-docs/BRIDGE_SYSTEM.md) §"Bridge Repair Hut Interaction" — trigger flow summary; has a verified-wrong claim about `field_0x6DF` (it's a self-destruct flag, NOT a repair flag — see §2.7 below)
- [BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md) — recent companion; mentions repair addresses in passing
- [MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md) — `UpdateBridgeZonesHelper` internals; hut registry at `MapClass+0x1160` (= `DAT_008B41A8`)
- [ENGINEER_CAPTURE_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/ENGINEER_CAPTURE_GHIDRA_REPORT.md) — engineer capture flow; **does NOT cover the BridgeRepairHut branch**
- [HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md) — 18-state damage machine; only documents forward (Healthy→Destroyed) direction
- [LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md) — 18-walker RecalcAttributes callers; confirms walker family
- [docs/gap-scans/2026-05-06-gap-scan-bridges-deep.md](../gap-scans/2026-05-06-gap-scan-bridges-deep.md) §D2.5 — flagged "no bridge repair path at all" in Rust
- Project memory entry `project_c4_bridge_hut_followup` — open bug: C4 on CABHUT does nothing in-game. The scoping pass confirms this is the SAME dispatch chain as repair.

**Expected Output:** new report at
`docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`,
plus a correction note added to BRIDGE_SYSTEM.md §"Bridge Repair Hut Interaction"
about the `field_0x6DF` semantics.

**Next Pipeline Step:** `/brainstorm` (paused brainstorm session resumes from
this report) → `/write-plan` → implement.

---

## 1. Goal

Answer five specific questions so the paused `/brainstorm` for runtime
bridge-walkable invalidation can resume with a verified spec:

1. **Trigger semantic.** Confirm or refute the claim from the scoping pass that:
   - Engineer enters CABHUT cell → `InfantryClass::PerCellProcess` (0x519630)
     reads `Type[0x16B6]` → dispatches `Process*Destruction` walker via the
     same code path as hut death (i.e., the dispatcher is **direction-agnostic**;
     repair vs destruction is decided by current cell state, not by who called).
   - `field_0x6DF` is a SELF-DESTRUCT flag set on hut damage/death, NOT a
     "repair pending" flag as BRIDGE_SYSTEM.md currently claims.

2. **Walker state-transition table.** For each walker
   (RepairBridgeWalker_NS_Low/EW_Low/NS_High/EW_High at 0x57F6A0, 0x57FBC0,
   0x5800D0, 0x580600), enumerate:
   - Input cell states + walker context → output cell state writes
   - Specifically the reverse transition (Destroyed → Healthy{variant=?}):
     does it always write `Healthy{variant=0}` or pick a specific variant?
   - Does it skip intermediate states (PartialCollapseA/B), or unwind them
     step-by-step?
   - Cell+0x11E (`bridge_state` byte) before/after.
   - Overlay byte (cell+0x44) before/after — does it restore to a fixed
     value or one matching the original tile?

3. **Cell-selection scope per repair trigger.**
   - The 5×5 scan in PerCellProcess / BuildingClass::Update — is it really
     a 5×5 square (`-2..=+2`), or some other shape (e.g., the LAT_RETRIGGER
     doc mentions `DAT_00abad1c..+0x10` = 16-byte range, suggesting a 4×4
     or specific offset table)?
   - Does the dispatcher pick ONE bridge group to operate on, or all bridge
     groups whose cells fall within the scan?
   - For LOW vs HIGH, does the overlay-byte range gate which dispatcher runs
     (0x4A-0x65 → low, BridgeSet-relative → high)?
   - Does the walker re-scan adjacent cells to extend the operation to the
     full bridge segment, or does it stop at the 5×5 boundary?

4. **Endpoint flag flow on repair.**
   - When walker writes Destroyed → Healthy, does it directly write the
     `endpoint.active = true` flag, or does it rely on a downstream
     `refresh_endpoint_active_flags` equivalent?
   - Does repair fire a `zones_dirty` equivalent in the binary
     (i.e., InvalidateBridgeZones → UpdateBridgeZonesHelper rebuild)?
   - Is `MapClass::UnregisterBridgeRepairHut` (0x577920) the inverse —
     called only on hut destruction, removing the hut from the registry,
     which then prevents future repairs?

5. **Audio/visual dispatch.**
   - Confirm EVA_BridgeRepaired playback at 0x519BC4 — what determines
     whether the EVA fires (sole engineer who entered, or broadcast)?
   - Confirm `VocClass__PlayAt(RulesClass+0x248)` — what RulesClass field
     is at +0x248? Is it `RepairBridgeSound`?
   - Is there a particle/anim spawn (Welding particle, repair anim) that
     fires per cell, or only one at the hut?

The report must conclude with **"Active in YR: yes/no/conditional"** for
every claim, and a one-line **observable-impact verdict** for each.

---

## 2. Prior Research Inventory

| Report | Scope | Confidence | Known Gaps |
|--------|-------|------------|------------|
| BRIDGE_SYSTEM.md §"Bridge Repair Hut Interaction" | Trigger flow summary | MEDIUM | **field_0x6DF semantic is wrong** per scoping; walker internals undocumented; state transitions undocumented |
| BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md | Damage side; lists repair addresses as "verify exists" | HIGH for damage | No repair-side coverage |
| MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md | `UpdateBridgeZonesHelper` Phase 1–8; hut registry location | HIGH | Registry layout (struct of each entry) not detailed; lookup paths only partially traced |
| ENGINEER_CAPTURE_GHIDRA_REPORT.md | Engineer capture for Capturable=yes buildings | HIGH for capture | **Does NOT cover BridgeRepairHut branch** — different dispatch (PerCellProcess, not Mission_Capture) |
| HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md | 18-state machine, forward direction | HIGH for damage | Reverse transitions (Destroyed → Healthy) completely undocumented |
| LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md | RecalcAttributes 18 walker callers; ToggleBridgePavement (0x56E990) | HIGH | Walker bodies not decompiled |
| docs/gap-scans/2026-05-06-gap-scan-bridges-deep.md §D2.5 | Confirms Rust has zero repair coverage | HIGH | High-level gap statement only |

**Conflicts between reports (RESOLVE in execution):**

- **Conflict A — `field_0x6DF` semantic.** BRIDGE_SYSTEM.md says it's a
  "repair flag" set when an engineer enters the hut. Scoping pass says it's
  a self-destruct flag set on hut damage/death (by ReceiveDamage and
  BombClass::Detonate), used by BuildingClass::Update to fire the
  bridge-destruction path. The latter matches the C4-on-CABHUT memory
  entry's expected behavior. **Re-verify whose interpretation is right.**

- **Conflict B — Dispatcher identity.** BRIDGE_SYSTEM.md says
  FUN_00574C20 / FUN_00574000 are the "repair dispatchers." Scoping says
  they're `DestroyBridge_*_MapInit` (destruction kickoff from
  BuildingClass::Update + BombClass::Detonate), and the actual repair-side
  dispatcher is something invoked via `InfantryClass::PerCellProcess`. The
  function `RepairBridge_Low/High` (0x57F200/0x57F440) is the walker
  driver, called from BOTH dispatchers with different cell-state inputs.

- **Conflict C — RepairBridgeSegment semantic.** BRIDGE_SYSTEM.md says it
  "walks 3-wide clearing objects" (= RepairBridgeSegment). gap-scan §D2.5
  says it fires `ProcessCellAction(0x1F, ...)` on cells with non-null
  TagClass* (trigger-action firing, NOT tile repair). Both could be true
  in different branches — re-verify the full body.

---

## 3. Function Inventory

| #  | Phase | Address    | Current Name                                                          | Scope Reason                                                                            | Depth Target | TS-Legacy Risk |
|----|-------|------------|-----------------------------------------------------------------------|-----------------------------------------------------------------------------------------|--------------|----------------|
| 1  | 1     | 0x519630   | `InfantryClass::PerCellProcess`                                       | Repair-side entry. Reads `Type[0x16B6]`, dispatches walker, fires EVA + sound (0x519BC4). Verify exact branch. | **FULL** | Low |
| 2  | 1     | 0x43FB20   | `BuildingClass::Update`                                               | Destruction-side entry. Tests `field_0x6DF` + `Type[0x16B6]`, does 5×5 scan, dispatches DestroyBridge_*_MapInit. | **FULL** | Low |
| 3  | 1     | 0x57F200   | `MapClass::RepairBridge_Low`                                          | Outer walker driver — calls 3 walkers × 2 axes. Owns the repair-vs-destruction state-transition logic. | **FULL** | Low |
| 4  | 1     | 0x57F440   | `MapClass::RepairBridge_High`                                         | Outer walker driver, high-bridge variant. Verify byte-similarity to #3 (likely compiled twins).         | **FULL** | Low |
| 5  | 1     | 0x575EE0   | `RepairBridgeSegment`                                                 | Per-cell action: object clear AND/OR trigger-action firing. Resolves Conflict C.                  | **FULL** | Low |
| 6  | 1     | 0x570050   | `ProcessBridgeDestruction_Low`                                        | True low-bridge destruction dispatcher (per scoping correction). Calls #3.                       | **FULL** | Low |
| 7  | 2     | 0x573540   | `ProcessBridgeDestruction_High`                                       | True high-bridge destruction dispatcher. Calls #4.                                                  | MEDIUM | Low |
| 8  | 2     | 0x57F6A0   | `MapClass::RepairBridgeWalker_NS_Low`                                 | Walker — per-cell state transition (NS, low). Inputs/outputs to state-transition table for Q2.        | MEDIUM | Low |
| 9  | 2     | 0x57FBC0   | `MapClass::RepairBridgeWalker_EW_Low`                                 | Walker — EW, low. Likely compiled twin of #8.                                                       | MEDIUM | Low |
| 10 | 2     | 0x5800D0   | `MapClass::RepairBridgeWalker_NS_High`                                | Walker — NS, high.                                                                                     | MEDIUM | Low |
| 11 | 2     | 0x580600   | `MapClass::RepairBridgeWalker_EW_High`                                | Walker — EW, high. Likely compiled twin of #10.                                                     | MEDIUM | Low |
| 12 | 2     | 0x574000   | `MapClass::DestroyBridge_High_MapInit` (misnamed)                     | Called from BuildingClass::Update and BombClass::Detonate on hut death. Wraps #7.                  | MEDIUM | Low (despite name) |
| 13 | 2     | 0x574C20   | `MapClass::DestroyBridge_Low_MapInit` (misnamed)                      | Low-bridge equivalent of #12. Wraps #6.                                                              | MEDIUM | Low (despite name) |
| 14 | 2     | 0x438720   | `BombClass::Detonate`                                                 | C4 path. Sites at 0x438982/0x43896A call #12/#13 on hut detonation. Verify wiring.                | LIGHT | Low |
| 15 | 2     | 0x577920   | `MapClass::UnregisterBridgeRepairHut`                                 | Removes hut from registry (+0x1160) + parallel array (DAT_008B41A8/AC). Called on hut destruction. | MEDIUM | Low |
| 16 | 2     | (unknown) | `ReceiveDamage` or `field_0x6DF` setter                                | Find writers of `[reg + 0x6DF] = 1`. Likely on hut taking damage threshold. Resolves Conflict A.    | MEDIUM | Low |
| 17 | 3     | 0x4D9290   | `FootClass::Mission_Enter`                                            | Vtable-only; not the bridge-repair entry. **Confirm not in the repair chain.**                 | LIGHT | Low |
| 18 | 3     | 0x442230   | `BuildingClass::ReceiveDamage` (suspected)                            | Candidate setter of `field_0x6DF` on damage. Verify and pull threshold.                          | LIGHT | Low |
| 19 | 3     | 0x4E7F4D   | (unfunctioned) — hut registry write site                              | In unfunctioned bytes. `create_function` first. Likely a building Init/Place helper.            | MEDIUM | **Unknown** — verify it's live in YR |
| 20 | 3     | 0x4E7F78   | (unfunctioned) — hut registry write site                              | Sibling of #19.                                                                                       | MEDIUM | **Unknown** |
| 21 | 3     | 0x67F9C0   | `FUN_0067F9C0`                                                        | Reads/writes hut registry. Purpose unknown — possibly save/load or AI logic.                       | LIGHT | **Unknown** |
| 22 | 3     | 0x684C30   | `FUN_00684C30`                                                        | Reads/writes hut registry. Purpose unknown.                                                          | LIGHT | **Unknown** |
| 23 | 3     | 0x519BC4   | (inside #1) — VoxClass::PlayEVA call site                              | EVA_BridgeRepaired dispatch. Confirm which player hears it and gate conditions.                  | LIGHT | Low |
| 24 | 3     | (varies)   | `VocClass::PlayAt` call inside #1                                     | Sound playback via `RulesClass + 0x248`. Identify the rules field name.                            | LIGHT | Low |
| 25 | 3     | (4 sites)  | `MapClass__UpdateBridgeEdgeTiles_*` callers of #5                       | RepairBridgeSegment is also called by Edge fixup (0x57671C, 0x570FFC). Verify shared semantic.   | LIGHT | Low |
| 26 | 3     | (2 sites)  | `FindBridgeEndpoints` callers of #5                                   | RepairBridgeSegment caller from endpoint-finding paths. May or may not be repair-related.        | LIGHT | Low |
| 27 | 3     | 0x56E990   | `MapClass::ToggleBridgePavement`                                      | Toggles damage-variant bit 0x2000 on cells. Repair must invert this state too.                   | LIGHT | Low |
| 28 | 3     | 0x47D2B0   | `CellClass::RecalcAttributes`                                         | Called by walkers. Already documented in BRIDGE_DEFERRED_MECHANICS — confirm `level_override` use during repair. | LIGHT | Low |

**Phase 1 checkpoint:** after #1–#6, the executor must summarize:
- The verified trigger flow end-to-end (resolves Conflicts A and B)
- The walker state-transition table (resolves Q2 partially via #3 = the driver)
- Confirmation whether `RepairBridge_*` is direction-agnostic or one-way

If Phase 1 reveals the dispatcher routing differs from the scoping
pass's claim, revise the plan before Phase 2.

---

## 4. Detail Checklist

### Magic numbers / constants to decode
- The 5×5 scan offsets (`-2..=+2` in both X and Y per BRIDGE_SYSTEM.md, but
  scoping mentions a 16-byte range at DAT_00abad1c — clarify the actual
  offset table or hardcoded loop bounds in #1 and #2)
- BridgeStrength threshold for `field_0x6DF` set (probably 0 or some low HP)
- The dispatcher overlay-range gates: low = 0x4A..0x65, high = BridgeSet-relative
  (resolve the exact BridgeSet range read at runtime)
- EVA sound IDs from `RulesClass+0x248` and similar fields
- Walker step count per axis (NS / EW): how many cells does each walker traverse?

### Bit flags / masks
- `Type[0x16B6]` byte = BridgeRepairHut flag (verified)
- `BuildingClass+0x6DF` byte = (scoping says self-destruct flag — verify)
- Cell.Flags bit 0x80 (bridge_walkable) — does the walker write this on repair?
  (Inverse of SetBridgeDirection's anchor clear on destroy.)
- Cell.Flags bit 0x100 (bridge cell) — repair must restore
- Cell.Flags bit 0x200 (bridgehead) — repair must restore on the right cells
- Cell.Flags bit 0x400 (destroyed marker) — repair must clear
- Cell.Flags bit 0x2000 (damaged variant per LAT_RETRIGGER doc) — repair must clear

### State machine states / branches
- Suspected reverse transitions per walker:
  - `Destroyed → Healthy{variant=?}` (which variant byte does it write?)
  - `PartialCollapseA → Healthy{variant=?}` (if intermediate states are
    visited during repair, or skipped)
  - `Damaged → Healthy{variant=?}` (does damage-but-not-destroyed reset to
    intact via the same trigger?)
- Walker may pick variant from the LAT pattern table at `DAT_0081CC30`
  (Latin square `{0,1,2,3, 3,2,1,0, 2,3,0,1, 1,0,3,2}`) keyed on cell parity

### INI keys to verify
- `BridgeRepairHut=yes` on CABHUT — verified parsed in Rust at
  object_type.rs:924, parsed flag at +0x16B6 in binary
- `RepairBridgeSound=BridgeRepaired` in [AudioVisual] — verified, parsed
  at ruleset.rs:738 in Rust
- `DestroyableBridges=yes` in [CombatDamage] — confirms the repair path is
  active in retail; verify the gate location in binary
- `BridgeStrength=1500` — relevant for the C4 / damage trigger threshold
- `BridgeExplosions=` list — read by BlowUpBridge (already in BRIDGE_DEFERRED
  report); verify NO inverse "BridgeRepairAnims" key exists

### Struct offsets to extract
- `BuildingTypeClass+0x16B6` (BridgeRepairHut flag) — already known
- `BuildingClass+0x6DF` (self-destruct or repair flag — resolve Conflict A)
- `MapClass+0x1160` (hut registry array)
- `MapClass+0x116C` (hut registry counter)
- `DAT_008B41A8` / `DAT_008B41AC` (parallel hut data array)
- `DAT_008B41B8` (parallel array count)
- `RulesClass+0x248` (RepairBridgeSound field — verify)

### Clamps, rounding, off-by-ones
- The 5×5 scan: is it `-2..=+2` (25 cells incl. center) or `-2..+2` (24 cells excl. one corner)?
- Walker step count: does it run while `cell.has(0x100)`, or for a fixed N steps?
- Whether the walker stops at bridge endpoints or runs past them

### Edge cases to test
- Repair trigger when no destroyed cells in scan range → no-op or sound only?
- Repair trigger when bridge is Damaged (not Destroyed) → does it heal back to Healthy?
- Multiple bridges within the 5×5 scan range → which one wins?
- Hut destroyed AND adjacent bridge already destroyed → does it scan?
- Stacked engineers entering CABHUT in same tick → multiple repairs?
- Both end-huts of a bridge alive but center cells destroyed — does ONE engineer-entry repair the whole span?
- C4 on hut while bridge is ALREADY destroyed — does anything fire?

### Timing / ordering
- Does PerCellProcess fire EVA BEFORE or AFTER the walker runs?
- Does the walker mutate cells immediately or queue work for later in the tick?
- Where does this fit in `advance_tick`? (movement → repair-trigger → walker → zone rebuild → ...)
- Does the walker fire `zones_dirty` per cell or once at the end?

### TS-legacy flags
- `field_0x6DF` semantic — TS may have used it differently; verify YR behavior
- The 4 unknown registry-touching functions (#19, #20, #21, #22) — flag as
  TS-legacy risk until proven live in YR

### Vtable dispatches
- Walker likely doesn't use vtables (it's MapClass-internal). Confirm.
- VoxClass::PlayEVA, VocClass::PlayAt are direct calls per scoping.

---

## 5. INI Keys in Scope

| Key | Section | Default | Suspected Purpose | Currently Parsed in Rust? |
|-----|---------|---------|-------------------|----------------------------|
| `BridgeRepairHut=yes` | `[CABHUT]` (and any other hut definitions) | yes (CABHUT) | Gates which buildings can act as repair huts | Yes — `object_type.rs:924` (`bridge_repair_hut: bool`) |
| `RepairBridgeSound=BridgeRepaired` | `[AudioVisual]` | `BridgeRepaired` | Sound played on successful repair | Yes — `ruleset.rs:738` (`repair_sound: Option<String>`) |
| `DestroyableBridges=yes` | `[CombatDamage]` | yes | Master gate for destruction (and probably repair-readiness) | Unknown — Rust may not parse this yet |
| `BridgeStrength=1500` | `[CombatDamage]` | 1500 | Damage threshold for bridge tiles | Unknown — Rust parsing status unverified |
| `BridgeExplosions=` (list) | `[CombatDamage]` | (4 anims) | Animation IDs for bridge destruction; **may NOT have a repair counterpart** | Unknown |

**No new INI keys should be added during this investigation.** The repair
side reuses the existing keys above. If the executor finds an undocumented
key that gates repair, that's a finding worth flagging.

---

## 6. Caller & Integration Map

### Callers of `InfantryClass::PerCellProcess` (#1)
This is itself an Infantry per-tick callback (~1 xref). The relevant trigger
is the engineer-on-CABHUT case — verify by tracing what calls PerCellProcess
on infantry move (probably from movement-step / cell-arrival logic).

### Callers of `BuildingClass::Update` (#2)
100+ callers (per-tick dispatch from World update). Only the repair-branch
matters; the rest is unrelated.

### Callers of `RepairBridge_Low` / `_High` (#3, #4)
Per scoping, only `ProcessBridgeDestruction_*` (#6, #7) call these. So
the call hierarchy is:
```
Engineer step ┐
              ├─→ InfantryClass::PerCellProcess (#1)
              │       └─→ (dispatch — TBD which function in repair direction)
              │             └─→ RepairBridge_Low/High (#3/#4)
              │
Hut C4/death ┐
              ├─→ BuildingClass::Update (#2)
              │       └─→ DestroyBridge_*_MapInit (#12/#13)
              │             └─→ ProcessBridgeDestruction_Low/High (#6/#7)
              │                   └─→ RepairBridge_Low/High (#3/#4)
              └─→ BombClass::Detonate (#14)
                      └─→ DestroyBridge_*_MapInit (#12/#13)
                            └─→ (same chain)
```

The walker function (#3/#4) is the convergence point. Decompilation of #1
must reveal how it reaches the walker on the repair side (potentially via
a different intermediate, or possibly via the same #6/#7 with different
inputs that drive state in the opposite direction).

### Callers of `RepairBridgeSegment` (#5) — 6 total
- 2 from FindBridgeEndpoints family (Phase 3 — verify relevance)
- 2 from UpdateBridgeEdgeTiles_High/Low (#25 — verify relevance)
- 2 unidentified

### Callers of `MapClass::UnregisterBridgeRepairHut` (#15)
1 data-ref caller. Verify it's called only on hut destruction (not on
hut damage-but-not-destroyed).

### Rust integration today
- **Trigger:** No engineer-enter-CABHUT branch exists. Engineer is despawned
  in `tick_capture_orders` ([src/sim/world/world_orders.rs:147-209](../../src/sim/world/world_orders.rs#L147-L209)) without checking `bridge_repair_hut`.
- **Building update:** No `BuildingClass::Update` equivalent in Rust (buildings
  are generic `Entity` with `category: Structure`). No per-tick scan.
- **State transition:** `body_cell_advance_state` is forward-only at
  [src/sim/bridge_state/mod.rs:756-810](../../src/sim/bridge_state/mod.rs#L756-L810).
- **Sound:** `repair_sound` parsed at [src/rules/ruleset.rs:700-738](../../src/rules/ruleset.rs#L700-L738) but never emitted.
- **EVA:** No `BridgeRepaired` SimSoundEvent variant.
- **Zones rebuild:** Existing path at
  [src/sim/world/bridge_orchestrator.rs:309-324](../../src/sim/world/bridge_orchestrator.rs#L309-L324) WORKS for the repair direction; just needs the trigger to fire.

### Callers NOT investigated
- The non-bridge callers of BuildingClass::Update (100+ — only the repair-branch matters)
- The non-bridge callers of BombClass::Detonate (any C4 path that doesn't target a hut)
- Save/load and AI paths that might touch the hut registry (unless TS-legacy risk demands it)

---

## 7. TS-Legacy Risk Register

1. **`RepairBridgeWalker_*_*MapInit` suffix** — Ghidra's `_MapInit` name on
   #12 / #13 suggests these may have been map-init-only in TS. Per scoping
   they're called at runtime in YR. Verify the suffix is a Ghidra label
   error, not a TS gate.
2. **The 4 unknown registry-touching functions** (#19, #20, #21, #22) — may
   be save/load (TS-era format) or debug/editor paths. Trace their callers
   before concluding they're live.
3. **`MapClass::UnregisterBridgeRepairHut`** — assumes a registration path
   exists. If registration is map-load-only and never called at runtime,
   the function is legacy.
4. **`FogOfWar` not encountered** in any scoped function — flag if it
   appears during execution (FogOfWar defaults to false in YR per CLAUDE.md).
5. **`field_0x6DF`** — TS may have used this for something else entirely.
   Conflict A's resolution will tell us.

---

## 8. Current Rust Implementation Surface

| Subsystem | Status | Files |
|-----------|--------|-------|
| `BridgeRepairHut` flag parse | Implemented | [src/rules/object_type.rs:924](../../src/rules/object_type.rs#L924) — parsed, zero consumers |
| `RepairBridgeSound` parse | Implemented | [src/rules/ruleset.rs:700, 738](../../src/rules/ruleset.rs#L700) — parsed as `repair_sound: Option<String>`, zero consumers |
| Engineer→CABHUT trigger | **NOT implemented** | [src/sim/world/world_orders.rs:147-209](../../src/sim/world/world_orders.rs#L147-L209) — capture path has no BridgeRepairHut branch |
| BuildingClass::Update equivalent | **NOT implemented** | No per-tick building update scan in Rust |
| `field_0x6DF` analog | **NOT implemented** | No "needs repair" / "self-destruct flagged" field on buildings |
| Bridge state reverse transition | **NOT implemented** | [src/sim/bridge_state/mod.rs:756-810](../../src/sim/bridge_state/mod.rs#L756-L810) — forward only |
| Repair-sound emission | **NOT implemented** | No `SimSoundEvent::BridgeRepaired` variant |
| EVA_BridgeRepaired dispatch | **NOT implemented** | No EVA dispatch system in repair context |
| Zone rebuild infrastructure | Ready, no trigger | [src/sim/world/bridge_orchestrator.rs:309-324](../../src/sim/world/bridge_orchestrator.rs#L309-L324) — already supports `zones_dirty` rebuild; only needs the repair trigger to flip the flag |
| Overlay grid mutation | Ready, no caller | [src/sim/overlay_grid.rs](../../src/sim/overlay_grid.rs) — supports cell mutation; no repair-side caller yet |

**Coverage scorecard:** 2/10 implemented (parse-only), 2/10 ready-to-wire,
**6/10 not started.**

---

## 9. Deferred Open Questions

Questions the scoping pass surfaced but couldn't answer; executor must resolve:

1. **Conflict A** — Is `BuildingClass+0x6DF` a self-destruct flag (scoping)
   or a repair flag (BRIDGE_SYSTEM.md)? Settled by tracing setters.
2. **Conflict B** — Is `RepairBridge_Low/High` direction-agnostic (handles
   both repair AND destruction via the same walker, decided by current
   cell state) or one-direction-only?
3. **Conflict C** — Does `RepairBridgeSegment` clear objects, fire trigger
   actions, or both? Settled by full decompilation of #5.
4. **The actual repair-side dispatcher** — what does `InfantryClass::PerCellProcess`
   call to reach the walker? Maybe it goes through #6/#7 (`Process*Destruction`)
   with different inputs, maybe through a separate function not yet found.
5. **`field_0x6DF` setters** — where exactly is it written to 1? On hut
   damage threshold? On hut Destruct() call? On engineer-enter?
6. **The 4 unknown registry-touching functions** (#19-#22) — what enclosing
   functions call them? Are they live in YR?
7. **Walker variant byte** on repair — when reverting Destroyed → Healthy,
   does the walker pick variant 0, the original variant (if stored), or
   compute from LAT?
8. **Multi-hut interaction** — two CABHUTs serving the same bridge: does
   destroying one drop the bridge, or does the other prevent destruction?
9. **End-to-end repair scope** — does ONE engineer-enter repair the WHOLE
   bridge group, or just the cells within the 5×5 scan?

---

## 10. Execution Strategy

**Recommended: single `/re-investigate` session.** Function count (28) is
on the high side for one pass but each function is small (the walkers are
~300 bytes; PerCellProcess and Update have repair-relevant branches that
are isolated). One session avoids context fragmentation across the cross-cutting
trigger / dispatcher / walker chain.

Within the session:
- **Phase 1 first**: functions #1–#6 (the load-bearing 6). Checkpoint with
  Conflicts A/B/C resolved + walker state-transition table sketched.
- **User review of Phase 1 findings** before proceeding (the dispatcher
  routing in scoping was a major correction — Phase 1 might surface more).
- **Phase 2**: #7–#16 (walker family + auxiliaries).
- **Phase 3**: #17–#28 (callers, registry sites, audio, edge cases).

If Phase 1 takes longer than expected, split Phase 3 into a follow-up
investigation rather than rushing.

---

## 11. Success Criteria

The executed research document must:

- Resolve Conflicts A, B, C explicitly with disassembly citations.
- Produce the walker state-transition table covering Destroyed→Healthy,
  PartialCollapse{A,B}→Healthy, Damaged→Healthy transitions including
  variant byte assignment.
- Map the cell-selection scope per repair trigger (5×5? bridge-group-wide?
  group-aware via registry lookup?).
- Identify every cell-field write the walker performs (cell.Flags bits,
  cell+0x11E, cell+0x44 overlay byte, cell+0x2C anchor pointer,
  cell+0x11E bridge_state, ...).
- State whether the binary fires `InvalidateBridgeZones` or
  `UpdateBridgeZonesHelper` on repair (the zones_dirty analog).
- Trace the EVA + sound dispatch through #1 with the gate conditions
  (which player hears EVA, when sound plays).
- Resolve every open question from §9 — answered or re-documented as
  unresolved.
- State "Active in YR: yes/no/conditional" with one-line trigger-frequency
  for every claim, especially:
  - Engineer-enters-CABHUT repair (frequency: every match with a damaged
    bridge and a player who has engineers)
  - CABHUT-dies-destroys-bridge (frequency: matches with C4 / SEAL units —
    the open project memory bug)
- Add a correction note to BRIDGE_SYSTEM.md §"Bridge Repair Hut Interaction"
  if Conflicts A/B/C confirm the scoping's findings.

---

## Sources

**Ghidra addresses sampled** (light scoping):
0x519630, 0x43FB20, 0x57F200, 0x57F440, 0x575EE0, 0x570050, 0x573540,
0x57F6A0, 0x57FBC0, 0x5800D0, 0x580600, 0x574000, 0x574C20, 0x438720,
0x577920, 0x442230, 0x4E7F4D, 0x4E7F78, 0x67F9C0, 0x684C30, 0x519BC4,
0x56E990, 0x47D2B0, 0x4D9290. Hut registry data: DAT_008B41A8 /
DAT_008B41AC / DAT_008B41B8 / MapClass+0x1160. Strings: "BridgeRepaired"
(EVA voice), "EVA_BridgeRepaired" (string at 0x825538 — sole xref from
0x519BC4 inside #1).

**Docs searched:**
BRIDGE_SYSTEM.md, BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md,
MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md,
ENGINEER_CAPTURE_GHIDRA_REPORT.md,
MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md,
BUILDINGCLASS_MISSION_REPAIR_AND_PRODUCE.md,
BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md,
HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md,
LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md,
docs/gap-scans/2026-05-06-gap-scan-bridges-deep.md.

**INI files checked:**
ini/rulesmd.ini (lines 419, 529, 615, 721, 804, 816, 888, 3029, 16336–16348),
ini/rules.ini (line 9448–9460), ini/artmd.ini line 4143, eva.ini SoundList 46/57.

**Related plans:**
- 2026-05-11-bridge-locomotor-layer-correctness — parent of the damage-side investigation.
- 2026-05-12-bridge-mechanics-deferred-investigation-plan — yesterday's investigation surfacing the bit-0x80-mutation finding that motivated this.
- 2026-05-07-bridges-tier2-damage-state-machine — already-implemented damage side.

**Project memory entry that this plan addresses:**
[project_c4_bridge_hut_followup](../../../<local>/.claude/projects/<claude-project>/memory/project_c4_bridge_hut_followup.md)
— "SEAL/Tanya C4 on CABHUT does nothing in-game". The scoping pass confirms
this is the SAME dispatch chain as engineer-repair; both will be resolved
together by this investigation.
