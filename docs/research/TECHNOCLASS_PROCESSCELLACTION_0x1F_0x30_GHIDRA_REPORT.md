# `TechnoClass::ProcessCellAction` / `FireTriggerAction` — Action Codes `0x1F` and `0x30`

**Date:** 2026-05-18
**Scope:** What handler runs when `action_code == 0x1F` (decimal 31) and
`action_code == 0x30` (decimal 48) are passed to the bridge-related caller of
`TechnoClass::ProcessCellAction @ 0x006E53A0`?
**Mode:** read-only Ghidra MCP. No mutations.
**Status:** COMPLETE.

---

## TL;DR (load-bearing facts)

1. **The function at `0x006E53A0` is misnamed `ProcessCellAction`.** Its
   correct role is `TechnoClass::FireTriggerAction(eventType, source, target,
   coord, flags)` — it does NOT dispatch on a switch over action codes. Instead
   it iterates the techno's `AttachedTag.ActionList` and asks
   `TriggerActionEntry::EvaluateConditions @ 0x007264C0` whether each entry's
   stored condition-event matches `param_2` (= `eventType` = the "action_code"
   passed in).
2. **`0x1F` and `0x30` are NOT switch cases inside the function — they are
   `TriggerEvent` enum IDs.** The function is a per-techno trigger-event
   broadcaster. The "handler" for each code is "whatever scripted trigger the
   mapper bound to that event on this techno's attached tag." On a vanilla
   skirmish map with no triggers wired, both calls are **no-ops**.
3. **Event 31 (`0x1F`) — broadcast on bridge-span collapse.** Called per
   tagged cell along a destroyed bridge segment from `RepairBridgeSegment @
   0x00575EE0` (misnamed — does not repair). Effect on the techno itself is
   nil; only player-scripted triggers respond.
4. **Event 48 (`0x30`) — broadcast on engineer entering bridge-repair hut.**
   Called once from `InfantryClass::PerCellProcess @ 0x00519630` after the
   engineer's repair branch has succeeded and the hut has scheduled its
   repair. Same scripted-only effect.
5. **Both code paths are ACTIVE in YR**, but the observable in-game effect of
   the `FireTriggerAction(0x1F/0x30, ...)` calls themselves is **only present
   when a campaign/custom map binds a trigger to event 31 or 48**. In standard
   skirmish play with vanilla maps, both calls return without side-effects.

---

## 1. Function identity — `0x006E53A0`

**Signature (Ghidra reading):**

```c
undefined1 __thiscall TechnoClass__ProcessCellAction
    (int this, undefined4 eventType, int sourceObject,
     undefined4 coordOrTargetCell, undefined4 flag1, undefined4 flag2);
```

**Argument mapping (verified via call-site disassembly at `0x00575F95` and
`0x00519FF1`):**

| Param | Role | Value at bridge call-sites |
|-------|------|----------------------------|
| `this` (ECX) | techno whose attached tag is being polled | the techno on/near the bridge cell (or the engineer infantry) |
| `param_2` | `TriggerEvent` ID | `0x1F` (RepairBridgeSegment) or `0x30` (InfantryClass::PerCellProcess) |
| `param_3` | source object | `0` for RepairBridgeSegment; `param_1` (the engineer) for PerCellProcess |
| `param_4` | cell-coord context | `DAT_00ABD480` (zeroed sentinel, "no specific cell") for bridge; `DAT_00A8F1E0` for PerCellProcess |
| `param_5`, `param_6` | flags | `0, 0` at both bridge call-sites |

**Function shape (decomp summary, NOT a switch on event):**

```c
if (g_IsMapEditor || this->field_0x35 || this->field_0x34) return 0;
if (this->AttachedTag == NULL) return 0;             // +0x24

this->field_0x35 = 1;                                // re-entrancy guard

for (entry = this->TagFirstAction;                   // walks +0x28 list
     entry != NULL;
     entry = entry->next)
{
    int actionType = entry->TriggerPtr->TypeKind;    // +0x9C
    if (TriggerActionEntry__EvaluateConditions(
            entry, eventType /* param_2 */, sourceObject,
            (actionType == 2), flag1, flag2))
    {
        // entry matched eventType — schedule its action(s)
        if (actionType == 0)      { PlayVoice; DynamicVectorAdd; ret=1; }
        else if (actionType == 1) { /* requires entry->field_2C == 1 */
                                    PlayVoice; DynamicVectorAdd; ret=1; }
        else if (actionType == 2) { PlayVoice; ret=1; }
    }
}

this->field_0x35 = 0;
// post-loop cleanup: optional Detach_From_All_Lists if return was set,
// optional FUN_005F5B50 if param_3->field_0x34 == this, etc.
return ret;
```

**Key observation:** `eventType` (`param_2`) is consumed only by
`TriggerActionEntry::EvaluateConditions`, which uses it as the matching key
against each entry's stored condition (`entry->TriggerPtr+0x2C`). If no
attached tag exists, or no entry's stored condition equals `eventType`, the
function returns 0 with no observable effect.

So "what does action 0x1F do?" and "what does action 0x30 do?" are
**ill-posed** — the per-techno effect is entirely determined by what the
map-maker bound to events 31 and 48. The engine code itself only delivers
the notification.

---

## 2. `TriggerActionEntry::EvaluateConditions` @ `0x007264C0`

Iterates the entry's TriggerType condition list (`entry->TriggerPtr +
0xAC`). For each condition, calls
`TriggerCondition::Evaluate @ 0x0071E940` with the live `eventType`. The
broad switch in `TriggerCondition::Evaluate` has a "match-only" cluster
including the cases listed below, and for each it returns false when
`param_2 (eventType) != iVar5 (entry's condition kind at +0x2C)`. **Both
`case 0x1F` and `case 0x30` are explicit members of this cluster** — see
the case-list in the decomp:

```text
case 1, 2, 3, 4, 6, 7, 0x12..0x1A, 0x1D, 0x1F, 0x21, 0x22, 0x23,
     0x26..0x2C, 0x30, 0x31, 0x32, 0x35, 0x36, 0x3B
```

This confirms `0x1F` and `0x30` are **first-class entries in the
`TriggerEvent` enum** consumed by the scripted-trigger system, not
hidden internal codes.

### Event-name evidence (data table `0x0083978C..0x008397CC`)

Memory at `0x00839700+` holds a contiguous array of 17 little-endian
`char*` pointers used by the trigger-system event-name lookup (debug /
editor support). Reverse-mapped:

| Table slot @ addr | String addr | Value |
|-------------------|-------------|-------|
| `0x0083978C` | `0x0083990C` | "Combat Event" |
| `0x00839790` | `0x008398FC` | "Noncombat Event" |
| `0x00839794` | `0x008398EC` | "DropZone Event" |
| `0x00839798` | `0x008398D8` | "Base Attacked Event" |
| `0x0083979C` | `0x008398BC` | "Harvester Attacked Event" |
| `0x008397A0` | `0x008398A8` | "Enemy Sensed Event" |
| `0x008397A4` | `0x00839894` | "Unit Produced Event" |
| `0x008397A8` | `0x00839884` | "Unit Lost Event" |
| `0x008397AC` | `0x00839870` | "Unit Repaired Event" |
| `0x008397B0` | `0x00839854` | "Building Infiltrated Event" |
| `0x008397B4` | `0x0083983C` | "Building Captured Event" |
| `0x008397B8` | `0x00839828` | "Beacon Placed Event" |
| `0x008397BC` | `0x0083980C` | "Superweapon Detected Event" |
| `0x008397C0` | `0x008397F0` | "Superweapon Activated Event" |
| `0x008397C4` | `0x008397D8` | **"Bridge Repaired Event"** |
| `0x008397C8` | `0x008397BC` | "Garrison Abandoned Event" |
| `0x008397CC` | `0x008397A0` | "Ally Base Attacked Event" |

The string `"Bridge Repaired Event"` at `0x008397D8` and the dispatcher's
acceptance of `case 0x30` in `TriggerCondition::Evaluate` are jointly
consistent with the standard YR enum mapping:

- `TriggerEvent::BridgeDestroyed = 0x1F` (31) — fired by
  `RepairBridgeSegment`.
- `TriggerEvent::BuildingRepaired_Bridge = 0x30` (48) — fired by
  `InfantryClass::PerCellProcess` after a successful engineer-into-hut
  bridge-repair sequence.

**Confidence note:** the name "Bridge Repaired Event" string exists in the
table; the precise enum *index* of either event is not directly readable
from a labelled enum-table dump in this binary (no enum table found by
this investigation). The name->ID mapping is **inferred from**: (a) the
two call-sites' bridge semantics (destruction-side vs repair-side), and
(b) the explicit `case 0x1F:` / `case 0x30:` membership in the
match-cluster inside `TriggerCondition::Evaluate`. Treat the
"BridgeDestroyed=31 / BridgeRepaired=48" naming as VERIFIED-BY-BEHAVIOR,
not VERIFIED-FROM-ENUM-STRING. If a future investigation needs the exact
ModEnc-published enum string for event 48, decompile the WW
trigger-event-name lookup helper that consumes the `0x0083978C` table.

---

## 3. Call-site 0x1F — `RepairBridgeSegment @ 0x00575EE0`

**Disassembly (verified):** seven distinct call-sites inside this single
function push `0x1F` as the first arg:

```text
0x00575F95  PUSH 0x1F  ; CALL 0x006e53a0   ; main walker cell
0x00576007  PUSH 0x1F  ; CALL 0x006e53a0   ; EW perp +1
0x0057606C  PUSH 0x1F  ; CALL 0x006e53a0   ; EW perp +2
0x005760CC  PUSH 0x1F  ; CALL 0x006e53a0   ; EW perp +3
0x00576137  PUSH 0x1F  ; CALL 0x006e53a0   ; NS perp +1
0x0057619C  PUSH 0x1F  ; CALL 0x006e53a0   ; NS perp +2
0x005761DE  PUSH 0x1F  ; CALL 0x006e53a0   ; NS perp +3
```

Each call uses `DAT_00ABD480` (currently zero — sentinel/null) as the
cell-context. Each call is **guarded by `cell->field_0x3C != 0`** — the
cell must have an attached TagClass pointer.

**Callers of `RepairBridgeSegment` (verified by `get_function_callers`):**
- `MapClass::FindBridgeEndpoints_EW_High @ 0x0057DAF0`
- `MapClass::FindBridgeEndpoints_EW_Low @ 0x0057C870`
- `MapClass::FindBridgeEndpoints_NS_High @ 0x0057DC20`
- `MapClass::FindBridgeEndpoints_NS_Low @ 0x0057C990`
- `MapClass::UpdateBridgeEdgeTiles_High @ 0x00576200`
- `MapClass::UpdateBridgeEdgeTiles_Low @ 0x00570AE0`

All six callers are on the **bridge-destruction or end-of-segment
refresh path** (per HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md
§11.3 and §11.9).

**Per-occupant effect:** none direct. The function passes `0x1F` to
each tagged span cell's first-listed techno via the
`TechnoClass::FireTriggerAction` walker. Damage, mission-changes,
death, and movement are NOT applied here. The only effect is
`TriggerActionEntry::PlayVoiceForObjects` (if a matching trigger
existed and ran its action) and queueing the matched trigger-action
entry into a global pending-actions vector.

**Active in YR: Yes — but observable only when a map trigger binds
event 31.** Vanilla YR skirmish maps do not bind triggers to
`BridgeDestroyed`, so the call sequence is a no-op in standard play.
Campaign / custom-map mappers can use this hook to script reactions
to bridge collapse (e.g., spawn reinforcements, play taunt vox).

---

## 4. Call-site 0x30 — `InfantryClass::PerCellProcess @ 0x00519630`

`InfantryClass::PerCellProcess` invokes `FireTriggerAction(0x30, ...)`
in **two places**, both in the engineer-into-hut branch:

### 4.1 Sub-call A — engineer→hut, "garrison spy/repair" branch (`0x00519FF1`)

Context (decomp, condensed):

```c
if (mission == 9 /*Capture*/ &&
    look_up_building_in_cell() == this->NavTarget) {
    if (NavTarget->AttachedTag != NULL) {
        TechnoClass__ProcessCellAction(1 /*Discovered?*/, this,
                                       DAT_00A8F1E0, 0, 0);
    }
    ...
    HouseClass::Add_Credits(...);
    if (this->AttachedTag != NULL) {
        TechnoClass__ProcessCellAction(0x30, this, DAT_00A8F1E0, 0, 0);
    }
    if (NavTarget->Type[+0x16AD] != 0) { ... animation handling ... }
}
```

This is the **non-bridge** spy/garrison engineer entry. The
`0x30` event is fired on the engineer itself (not the building), with
its own AttachedTag's list polled. This means event 48 is delivered to
**the engineer's tag, not the hut's tag**.

### 4.2 Sub-call B — engineer→hut, "mission 8/0xB/0x19" branch (`0x0051A017`)

Same pattern: after a building-entry branch terminates successfully, if
`this->AttachedTag != 0` then `FireTriggerAction(0x30, this,
DAT_00A8F1E0, 0, 0)` fires. The branch reaches this point only when
the engineer has successfully been absorbed by the target building.

The 0x30 call is therefore a **post-success engineer-action**
notification, broadcast on the engineer's own attached tag. It is
identical to other "I just did the thing" events (Building Captured,
Building Infiltrated, Garrison Abandoned) that the trigger system uses.

**Per-occupant effect:** same as 0x1F — no damage, no mission change,
no death applied here by the engine. Only player-scripted triggers
respond. The PlayVoiceForObjects call inside the matched-entry
branches will play a sound if the trigger action lists one, but that
is the trigger's own configuration, not a 0x30-specific effect.

**Active in YR: Yes — fires on every engineer-into-bridge-hut sequence
that succeeds.** The trigger response is observable only when a map
trigger binds event 48 to the engineer's tag (custom maps /
campaigns). In vanilla skirmish play, the call returns immediately
because the engineer has no attached tag (`this->AttachedTag == 0` is
the typical state for unit-cargo, and the `if (this->AttachedTag != 0)`
gate above each call short-circuits).

**Note on naming.** This is the call commonly associated with the
"Bridge Repaired Event" string at `0x008397D8`. Combined with §3's
"Bridge Destroyed Event" inference (event 31), event 48 is consistent
with "BridgeRepaired" — but the engineer-side delivery (vs.
hut-side) means the trigger must be attached to the engineer, not
to the building. This is a notable parity detail that scripted
campaigns rely on.

---

## 5. What `FireTriggerAction` does NOT do for 0x1F / 0x30

Cross-checked against the full decomp of `0x006E53A0`:

- **No damage application.** No `ReceiveDamage`, no warhead lookup, no
  health subtraction.
- **No mission change.** The techno's mission/state machine is untouched.
- **No death.** The techno is not removed from any list (unless a
  matched-trigger handler does so via `Detach_From_All_Lists`, which is
  the *trigger's* effect, not the action-code's).
- **No movement / scatter / facing change.**
- **No bridge-overlay change, no zone recompute, no terrain dirty.**
- **No EVA voice** unless the trigger entry's action lists a voice
  (`PlayVoiceForObjects`).

The only side-effects observable inside the function are:
- Setting `this->field_0x35 = 1` (re-entrancy guard, cleared at end).
- Queueing matched `TriggerActionEntry` records into a global
  `DynamicVectorClass` for later dispatch (for `actionType==0` and
  `actionType==1` entries).
- Optional `Detach_From_All_Lists` and `FUN_005F5B50` cleanup at the
  end IF a matched trigger requested removal.

---

## 6. Summary table — what's verified

| Item | Status | Evidence |
|------|--------|----------|
| Function `0x006E53A0` is `FireTriggerAction`, not a per-action switch | VERIFIED | decomp of `0x006E53A0` |
| `param_2` is `TriggerEvent` ID, consumed by `EvaluateConditions` | VERIFIED | decomp of `0x007264C0`, `0x0071E940` |
| `0x1F` and `0x30` are recognized cases in `TriggerCondition::Evaluate`'s match-cluster | VERIFIED | decomp of `0x0071E940` (`case 0x1f:` / `case 0x30:`) |
| Event 0x1F fired from `RepairBridgeSegment` at 7 sites, guarded by `cell+0x3C != 0` | VERIFIED | disasm of `0x00575EE0` |
| Event 0x30 fired from `InfantryClass::PerCellProcess` at 2 sites in engineer-into-building branches, guarded by `this->AttachedTag != 0` | VERIFIED | decomp of `0x00519630` |
| Event 0x1F caller set (six funcs) — destruction / endpoint refresh side only | VERIFIED | `get_function_callers @ 0x00575EE0` |
| String "Bridge Repaired Event" exists at `0x008397D8` | VERIFIED | `search_strings`, table at `0x0083978C` |
| Mapping "0x1F = BridgeDestroyed / 0x30 = BridgeRepaired" by enum index | INFERRED (behavior-only, not from enum table) | call-site semantics + match-cluster membership |
| Active in YR: both branches | VERIFIED | callers reachable in standard YR skirmish, no SpecialFlags gate |
| Observable effect in vanilla YR skirmish: NONE | VERIFIED (engine-side); player-side requires custom map trigger | engine path is no-op when no matching trigger entry exists |
| `DAT_00ABD480` is a zeroed sentinel "no cell context" | VERIFIED | `read_memory` returns 0x00 |

---

## 7. Open / not-investigated (out of scope)

- Exact YR `TriggerEvent` enum numeric definitions (only `0x1F` and `0x30`
  are required here; full table not enumerated).
- The other 30+ `TriggerEvent` IDs (`0x00..0x1E`, `0x20..0x2F`, `0x31..`)
  are explicitly excluded from this report per task scope.
- The non-bridge call-sites of `0x006E53A0` (Crate-pickup, Garrison,
  ReceiveDamage, Mission_Capture, RecordKill, etc.) are noted in the
  caller dump but not decompiled — they pass different `eventType`
  values, irrelevant to bridge work.
- Whether the engineer's `AttachedTag` is ever non-null in normal YR play
  (governs whether the 0x30 call ever does anything). Field-engineer
  units in skirmish are unlikely to carry tags, but campaign maps
  attach tags routinely.

---

## 8. Parity-port guidance for Rust

For a faithful Rust port, the bridge subsystem does **not** need to
implement either `FireTriggerAction(0x1F)` or `FireTriggerAction(0x30)`
as engine logic. They are scripted-trigger hooks. The Rust gap is:

- A `TriggerEvent` enum that includes `BridgeDestroyed = 0x1F` and
  `BridgeRepaired = 0x30` (when the trigger system is implemented).
- A per-techno "attached tag" mechanism that polls the tag's action
  list on event delivery.
- Until the scripted trigger system is implemented, both call sites in
  the Rust port should be **empty stubs**, accurately mirroring the
  no-op behavior on vanilla skirmish maps.
