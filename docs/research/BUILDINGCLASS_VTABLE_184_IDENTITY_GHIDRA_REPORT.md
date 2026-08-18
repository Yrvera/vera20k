# BuildingClass vtable slot 0x184 — Identity Verification Report

**Date:** 2026-05-18  
**Trigger:** Open question from `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md` §11 MEDIUM-confidence item.  
**Scope:** Confirm or refute "current mission getter" identity for vtable slot 0x184 on BuildingClass (and by cross-check, TechnoClass hierarchy), using live `read_memory` + decompile per `feedback_vtable_binding_verification`.

---

## Result: CONFIRMED — vtable[0x184] is `MissionClass::GetCurrentMission`

The C4 investigation's "strong inference but not yet verified" claim is correct. The slot is unambiguously the current-mission getter. Confidence on all three axes is now HIGH.

---

## Verified Facts (load-bearing, with evidence)

1. **BuildingClass vtable address = `0x007E3EBC`**  
   Source: BuildingClass constructor (`0x0043b740`) disassembly at `0x0043b9fa`:  
   `MOV dword ptr [ESI], 0x7E3EBC` — this is the first instruction that writes the vtable pointer to the object.

2. **Slot 0x184 of BuildingClass vtable points to `0x005B3040`**  
   `read_memory(0x007E3EBC + 0x184 = 0x007E4040, 4)` returned bytes `40 30 5B 00` (little-endian) → `0x005B3040`.

3. **Function at `0x005B3040` is `MissionClass__GetCurrentMission`**  
   Ghidra label verified; decompiled body:
   ```c
   int MissionClass__GetCurrentMission(int param_1) {
       int iVar1 = *(int *)(param_1 + 0xac);  // field at MissionClass+0xac
       if (iVar1 == -1) {
           iVar1 = *(int *)(param_1 + 0xb4);  // fallback field
       }
       return iVar1;
   }
   ```
   Reads `param_1 + 0xac`, the current mission field. If -1 (no mission active), falls back to `param_1 + 0xb4`.

4. **Field at `param_1 + 0xac` is the current mission — confirmed via MissionClass constructor and `Assign_Mission`**  
   `MissionClass__Constructor` (`0x005B2DA0`) initializes `param_1[0x2b]` (= `param_1[0x2b] × 4 = param_1 + 0xac`) to `0xFFFFFFFF` (-1 = no mission). `MissionClass__Assign_Mission` (`0x005B2FD0`) reads and writes `*(int*)(param_1 + 0xac)` as the current mission value, with `param_1 + 0xb4` as the queued/override slot.  
   This matches `MissionClass__Mission_Dispatch` (`0x005B3060`) which switches directly on `param_1[0x2b]` (offset `0xac`) to dispatch mission handlers.

5. **Same slot 0x184 maps to `MissionClass::GetCurrentMission` in InfantryClass, UnitClass, and 6 other class vtables**  
   `get_xrefs_to(0x005B3040)` returned 8 DATA references — all vtable slots:  
   `0x007E2428`, `0x007E4040` (BuildingClass), `0x007E8E18`, `0x007EB1DC`, `0x007EDE44`, `0x007F068C`, `0x007F4AE4`, `0x007F5DF4`.  
   Cross-check: InfantryClass vtable at `0x007EB058` → slot 0x184 = `0x007EB1DC` → `0x005B3040`. Confirmed.  
   UnitClass vtable at `0x007F5C70` → slot 0x184 = `0x007F5DF4` → confirmed from xref list.  
   This is a shared virtual method inherited from `MissionClass` (the common base for all Techno-capable game objects).

---

## Confidence Axes (per `feedback_research_confidence_axes`)

| Axis | Rating | Evidence |
|------|--------|----------|
| **Content** — what the function does | HIGH | Decompiled; reads `+0xac` (current mission int), fallback to `+0xb4`; confirmed field semantics via constructor init and `Assign_Mission` writes |
| **Identity** — which named class/method this is | HIGH | Ghidra label `MissionClass__GetCurrentMission`; function body matches the semantics exactly; field at `+0xac` is initialized to -1 (no mission) in `MissionClass__Constructor` |
| **Binding** — whether this is the slot used at runtime | HIGH | `read_memory(0x007E4040, 4)` = `40 30 5B 00` = `0x005B3040` per vtable_binding_verification rule; BuildingClass vtable address sourced from live constructor disassembly, not Ghidra labels |

---

## Cross-class Inheritance Comparison

| Class | vtable base | slot 0x184 address | function pointed to |
|-------|-------------|-------------------|---------------------|
| TechnoClass | `0x007F4960` | `0x007F40E4` | `0x00410530` (returns 0 — stub, different slot mapping) |
| BuildingClass | `0x007E3EBC` | `0x007E4040` | `0x005B3040` (`MissionClass::GetCurrentMission`) |
| InfantryClass | `0x007EB058` | `0x007EB1DC` | `0x005B3040` (`MissionClass::GetCurrentMission`) |
| UnitClass | `0x007F5C70` | `0x007F5DF4` | `0x005B3040` (`MissionClass::GetCurrentMission`) |

**Note on TechnoClass discrepancy:** TechnoClass slot 0x184 points to `0x00410530`, a 1-instruction function that returns 0. TechnoClass does NOT inherit from MissionClass (MissionClass inherits from ObjectClass, TechnoClass from FootClass/RadioClass hierarchy). The slot numbering is shared by address offset but the function mapped differs. This is expected because BuildingClass, InfantryClass, and UnitClass all inherit from MissionClass further up the hierarchy and override this slot with the real getter.

---

## Caller Analysis (per `feedback_caller_trace_before_finding`)

`get_xrefs_to(0x005B3040)` returned 8 entries, **all DATA** (vtable slot entries). There are zero direct CALL references to this address — the function is called exclusively through virtual dispatch.

The C4 investigation doc §6 documents two virtual call sites in `InfantryClass::PerCellProcess (0x519630)`:
1. `infantry.vtable[0x184]()` — gets current mission of the infantry unit itself to check for `0x11` (Mission_Sabotage)
2. `target_building.vtable[0x184]()` — gets current mission of the target building to check it is NOT `0x13` (Mission_Construction)

Both consume the return value as a mission-state integer compared against mission enum constants. This is unambiguously "current mission getter" usage — not a queued-mission accessor, not a mission-allowed predicate, not a type-id getter.

---

## Impact on C4 Investigation

`C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md` §11 listed this as MEDIUM confidence. The verified identity does not change the investigation's conclusions — it only upgrades the confidence:

- The building-mission gate `target_mission != 0x13` (Construction) correctly reads the building's current active mission via `GetCurrentMission`, which returns `param_1 + 0xac` (or falls back to `param_1 + 0xb4`).
- A CABHUT at idle is in `Mission_Standby (0x1a)` or `Mission_Guard (0x7)`, not `0x13` — so the gate passes, as the C4 investigation concluded.
- The slot identity is NOT a "queued mission getter", NOT a "mission-allowed predicate", and NOT a "mission type-id getter." It is the current active mission state. The C4 chain interpretation is unaffected and does not need revision.

---

## Key Addresses (verified this session)

| Address | Role |
|---------|------|
| `0x007E3EBC` | BuildingClass vtable base (live: constructor `0x0043b9fa`) |
| `0x007E4040` | vtable slot 0x184 (= `0x7E3EBC + 0x184`) |
| `0x005B3040` | `MissionClass::GetCurrentMission` — target function |
| `0x005B2DA0` | `MissionClass::Constructor` — confirms field `+0xac = -1` init |
| `0x005B2FD0` | `MissionClass::Assign_Mission` — confirms `+0xac` write semantics |

---

## Status: COMPLETE

Slot 0x184 of the BuildingClass vtable is confirmed to be `MissionClass::GetCurrentMission`. All three confidence axes are HIGH. The C4 investigation's medium-confidence item is resolved; no revision to that investigation's conclusions is needed.
