# AStar Pathfind Search TechnoType+0xD94 COM Path - Ghidra Research Report

**Address(es):** `0x0042C900` primary, target branch `0x0042CA4F..0x0042CABC`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Resolve the deferred `TechnoTypeClass+0xD94` branch inside `AStar_pathfind_search`, including field identity, object/COM use, YR activity, caller conditions, and Rust handoff.  
**Non-Scope:** Retry-edge exclusion semantics, full `Zone_precheck`, full Fly/Jumpjet locomotion, and unrelated `TechnoType+0xD94` consumers.  
**Confidence:** High for raw branch behavior and field identity; Medium for runtime frequency of nonstandard JumpJet-infantry A* callers.  
**Active in YR:** Conditional. The code is live in retail YR, but the branch fires only when `AStar_pathfind_search` is entered with an Infantry object whose type has `JumpJet=yes`.

## Working Notes Required By Parent

Target question: What does `AStar_pathfind_search` branch `0x0042CA4F..0x0042CABC` do with `TechnoType+0xD94`, and is it live in standard YR pathfinding?

Non-goals: Do not re-open settled retry-edge exclusion semantics; do not investigate unrelated A* helpers beyond caller/liveness evidence.

Evidence needed to mark COMPLETE: decompile plus disassembly for the target branch, field/object identity evidence, caller conditions, YR activity gate/default evidence, Rust surface scan, and no unresolved open questions inside this slice.

Stop conditions: Stop at the `0xD94` COM branch boundary, immediate callers/gates, and Rust handoff; record adjacent gaps rather than expanding.

Evidence note: Ghidra MCP tools were not exposed in this session. This report uses direct Capstone disassembly of retail `gamemd.exe`, existing Ghidra reports, INI source, and Rust source scans. No Ghidra mutation was possible or performed.

## 1. Overview

The deferred branch is a JumpJet-infantry special case inside the generic A* path entry. It is not a Drive/Hover/Ship hook and not a normal Fly row-9 pathing hook. When `this->WhatAmI()` returns `0xF` and the object's infantry type has `JumpJet=yes`, the function overwrites its local movement-zone row with constant `7` (`MovementZone::Infantry`), then queries the object's locomotor for `IID_IPersist`, calls `IPersist::GetClassID` into a local stack GUID, and releases it. The GUID value written by `GetClassID` is not read later in `AStar_pathfind_search`.

Active in YR: Conditional. The branch is in live `AStar_pathfind_search`, and stock YR has JumpJet infantry (`[JUMPJET]`, `[LUNR]`), but standard Jumpjet movement normally bypasses this A*/`Zone_precheck` stack. It matters only when a JumpJet infantry enters `FootClass::Find_Path -> FootClass::Run_AStar -> AStar_pathfind_search`, such as a ground-walk/fallback or nonstandard caller.

## 2. Class Layout / Key Offsets

| Field / object | Offset / address | Meaning | Active in YR |
|---|---:|---|---|
| `AbstractClass::WhatAmI` vtable slot | `vtable+0x2C` | Branch gate; `InfantryClass::WhatAmI @ 0x00523340` returns `0xF`. | Yes |
| Infantry type pointer | `InfantryClass+0x6C0` | Direct type pointer used by the branch, not virtual `GetTechnoType`. | Yes |
| `TechnoTypeClass+0xD94` | byte | `JumpJet` flag; default false, parsed from `JumpJet=`. | Conditional |
| Locomotor interface pointer | `TechnoClass/FootClass+0x674` | COM locomotor pointer queried by branch. | Yes when object has locomotor |
| IID at `0x00818858` | `0000010c-0000-0000-c000-000000000046` | `IID_IPersist`. | Yes as COM support check |
| Local movement-zone row | stack local `[esp+0x48]` in this function body | Usually caller row or `Type+0x5B4`; branch overwrites to `7`. | Conditional |
| MovementZone row 7 | constant `7` | Infantry movement-zone row in the 13-row passability matrix. | Yes |

## 3. Core Logic

### 3.1 Branch gates and side effects

Verified behavior:

1. `AStar_pathfind_search` resolves the movement-zone row earlier from the caller argument or from `TechnoTypeClass+0x5B4`.
2. It then calls `this->vtable+0x2C`.
3. Only if the result is exactly `0xF` does it inspect `InfantryClass+0x6C0`.
4. It reads byte `type+0xD94`.
5. If the byte is nonzero, it writes constant `7` to local `[esp+0x48]`.
6. It reads the object locomotor pointer at `+0x674`.
7. If the locomotor pointer is non-null, it calls `locomotor->QueryInterface(IID_IPersist, &local_interface)`.
8. On success it calls the returned interface vtable slot `+0x0C` with a pointer to local stack storage, then releases the interface via slot `+0x08`.
9. The local stack storage written by slot `+0x0C` is not referenced again in the function.

Evidence: raw assembly `0x0042CA43..0x0042CABC`; `0x00818858` bytes decode to `IID_IPersist`; only local reference to `[esp+0x20]` in `0x0042C900..0x0042CCCC` is the `GetClassID` output pointer at `0x0042CAAF`.

Active in YR: Conditional. The code runs in retail YR A*, but the branch requires Infantry RTTI and `JumpJet=yes`.

### 3.2 The `+0xD94` field is `JumpJet`, not Teleporter and not a generic COM flag

`TechnoTypeClass+0xD94` is the `JumpJet=` type flag. Existing audited docs verify the parser/default layout: default false in `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, with `+0xD94 = JumpJet`; the Rocketeer audit verifies `TechnoTypeClass+0xD94` as the JumpJet flag used by `FootClass::Locomotion_AI`.

Stock INI evidence:

| Section | Relevant stock values | Active in YR |
|---|---|---|
| `[JUMPJET]` | `JumpJet=yes`, `Locomotor={92612C46-...}`, `MovementZone=Fly`, `HoverAttack=yes` at `rulesmd.ini:3921/3948/3950/3966` | Yes |
| `[LUNR]` | `JumpJet=yes`, `Locomotor={92612C46-...}`, `MovementZone=Fly`, `HoverAttack=yes` at `rulesmd.ini:4715/4740/4742/4758` | Yes |
| `[SHAD]`, `[HIND]`, `[SCHP]`, `[SCHD]` | `JumpJet=yes` plus Jumpjet locomotor, but they are vehicle/aircraft types, not Infantry RTTI `0xF` | Branch No |
| `[DISK]`, `[ZEP]` | Jumpjet locomotor and `MovementZone=Fly`, but no `JumpJet=yes` in stock `rulesmd.ini` scan | Branch No |

Active in YR: Yes as parsed type data; Conditional for this branch because `WhatAmI()==0xF` is also required.

### 3.3 `WhatAmI()==0xF`, not locomotor-kind `0xF`

The gate at `0x0042CA43..0x0042CA4D` calls vtable slot `+0x2C`. Existing `ABSTRACTCLASS_GHIDRA_REPORT.md` identifies this slot as `WhatAmI`, and raw disassembly confirms `InfantryClass::WhatAmI @ 0x00523340` is `mov eax, 0xF; ret`.

This rules out the earlier attractive interpretation that the branch is a "locomotor-kind 0xF" or Drive/Hover/Ship hook. The branch is specifically an Infantry-class object check, and the direct `+0x6C0` type-pointer read matches InfantryClass layout.

Active in YR: Yes; InfantryClass is live and stock JumpJet infantry exists.

### 3.4 COM call identity and output relevance

The GUID at `0x00818858` is `0000010c-0000-0000-c000-000000000046`, which is `IID_IPersist`. The vtable slot `+0x0C` on `IPersist` is `GetClassID`, so the branch obtains the locomotor COM class ID into a local stack buffer and releases the interface.

No instruction later in `AStar_pathfind_search @ 0x0042C900` reads `[esp+0x20]`, `[esp+0x24]`, `[esp+0x28]`, or `[esp+0x2C]` after this call. The pathfinding-relevant side effect of the branch is therefore the earlier `local_movement_zone = 7`, not the COM class ID. The COM call may still assert/report unexpected COM failure through `0x007DC720`; it is not a routing input.

Active in YR: Conditional. It executes for JumpJet infantry A* entries with a non-null locomotor pointer.

### 3.5 Caller/liveness conditions

The ordinary A* path is `FootClass::Find_Path @ 0x004D3920 -> FootClass::Run_AStar @ 0x004CBBA0 -> AStar_pathfind_search @ 0x0042C900`. That path is active for ground/pathfinding objects.

Prior verified Fly/Jumpjet path-entry work says standard FlyLocomotion and normal JumpjetLocomotion move entries do not use `FootClass::Find_Path`/A*/`Zone_precheck`: Fly stores a destination and uses flight physics; Jumpjet move entry calls `Find_Nearby_Passable_Cell` with `zone_id=-1`.

Therefore:

- Active in YR: Yes for the generic A* caller stack.
- Active in YR: Yes for stock JumpJet infantry data.
- Active in standard stock JumpJet move orders: No evidence for normal move orders; prior report says normal Jumpjet movement bypasses A*.
- Active in YR if a JumpJet infantry enters A*: Conditional; branch fires and coerces the row to Infantry.

## 4. INI Keys

| Key | Parser/default evidence | Stock values relevant to branch | Active in YR |
|---|---|---|---|
| `JumpJet=` | `TechnoTypeClass+0xD94`, default false per `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`; parser/use verified by Rocketeer audit | `yes` on `[JUMPJET]`, `[LUNR]`, `[SHAD]`, `[HIND]`, `[SCHP]`, `[SCHD]`; absent on `[DISK]`, `[ZEP]` | Yes |
| `MovementZone=` | `TechnoTypeClass+0x5B4`; row parser mapping says `Fly=9`, `Infantry=7` | JumpJet infantry stock data says `MovementZone=Fly`; branch overrides to row 7 only inside A* | Yes |
| `Locomotor=` | Jumpjet locomotor CLSID `{92612C46-F71F-11d1-AC9F-006008055BB5}` | present on stock jumpjet infantry and several vehicles/aircraft | Yes |
| `HoverAttack=` | `TechnoTypeClass+0x390`; stock JumpJet infantry set `yes` | makes Rust's current short walk fallback not trigger for stock `[JUMPJET]`/`[LUNR]` | Yes |

## 5. Integration Points

| Entry / function | Relevance | Evidence | Active in YR |
|---|---|---|---|
| `FootClass::Find_Path @ 0x004D3920` | Live caller chain into `Run_AStar`; full internals out of scope. | prior A* reports and raw assembly call chain | Yes |
| `FootClass::Run_AStar @ 0x004CBBA0` | Calls `AStar_pathfind_search` with owner object and default row arg. | raw assembly `0x004CBC24..0x004CBC31` | Yes |
| `AStar_pathfind_search @ 0x0042C900` | Target function; branch coerces JumpJet infantry A* row to 7. | raw assembly `0x0042CA43..0x0042CABC` | Conditional |
| Normal FlyLocomotion move/process | Bypasses A*/Zone_precheck. | `FLY_JUMPJET_ROW9_PATH_ENTRY_AUDIT_GHIDRA_REPORT.md` | Yes |
| Normal Jumpjet move entry | Bypasses A*/Zone_precheck; uses `Find_Nearby_Passable_Cell(zone_id=-1)`. | `FLY_JUMPJET_ROW9_PATH_ENTRY_AUDIT_GHIDRA_REPORT.md` | Yes |

## 6. Current Rust Implementation Status

`src/rules/object_type.rs` parses `JumpJet=` into `ObjectType.jumpjet`, and `src/rules/locomotor_type.rs` has binary row values `Infantry=7`, `Fly=9`. `src/sim/world/world_commands.rs` routes air-layer Jumpjet moves to `air_movement::issue_air_move_command`, with a short ground-walk fallback only for Jumpjet infantry where `!HoverAttack` and distance is `<=3`. Since stock `[JUMPJET]` and `[LUNR]` have `HoverAttack=yes`, this fallback does not appear to affect stock YR Rocketeer/Lunar infantry movement.

Potential delta: if Rust intentionally supports the nonstandard JumpJet-infantry ground/A* fallback, that fallback should use `MovementZone::Infantry` for the A*/zone precheck, not the type's `MovementZone::Fly`. The binary branch proves the coercion happens inside `AStar_pathfind_search` when the fallback reaches A*.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Branch instructions `0x0042CA4F..0x0042CABC` | verified | raw assembly from retail `gamemd.exe` | none |
| `WhatAmI()==0xF` identity | verified | `ABSTRACTCLASS_GHIDRA_REPORT.md`; raw `0x00523340` disassembly | none |
| `TechnoType+0xD94` identity | verified | `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`; `units/allied/JUMPJET.md`; stock INI scan | none |
| `IID_IPersist` identity | verified | bytes at `0x00818858` decode to IID | none |
| `GetClassID` output use | verified | stack-reference scan of `0x0042C900..0x0042CCCC` | none |
| Normal Jumpjet movement bypass | verified-by-prior | `FLY_JUMPJET_ROW9_PATH_ENTRY_AUDIT_GHIDRA_REPORT.md` | no expansion in this slot |
| Runtime frequency of rare A* JumpJet-infantry entries | deferred | static evidence only | debugger instrumentation if needed |
| Rust surface | touched-sufficient | source scan of rules, world commands, zone search | no code modified |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - What branch gates entry? -> `WhatAmI()==0xF` and `InfantryType+0xD94 != 0`.` (evidence: `0x0042CA43..0x0042CA5D`, `0x00523340`)
- `[RESOLVED] OQ-2 - What is `+0xD94`? -> `JumpJet=` byte on `TechnoTypeClass`, default false.` (evidence: `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, `units/allied/JUMPJET.md`)
- `[RESOLVED] OQ-3 - What object field is read? -> direct `InfantryClass+0x6C0` type pointer, not a virtual call.` (evidence: `0x0042CA4F..0x0042CA55`, Infantry layout docs)
- `[RESOLVED] OQ-4 - What movement-zone value changes? -> local row is overwritten with constant `7`, the Infantry row.` (evidence: `0x0042CA69`, `src/rules/locomotor_type.rs`)
- `[RESOLVED] OQ-5 - What GUID is queried? -> `IID_IPersist` (`0000010c-0000-0000-c000-000000000046`).` (evidence: bytes at `0x00818858`)
- `[RESOLVED] OQ-6 - What COM method is called? -> `IPersist::GetClassID(&local_guid)` then `Release`.` (evidence: `0x0042CAAD..0x0042CABF`, IID identity)
- `[RESOLVED] OQ-7 - Is the COM result used? -> No later read of the local output buffer inside this function.` (evidence: stack-reference scan)
- `[RESOLVED] OQ-8 - Is this active in stock YR data? -> Conditional; stock JumpJet infantry exists, but normal Jumpjet move orders bypass A*.` (evidence: INI scan; Fly/Jumpjet path-entry audit)
- `[RESOLVED] OQ-9 - Is this TS-only dead code? -> No; stock YR has `JumpJet=yes` infantry and live A* caller stack. Runtime entry is conditional, not TS-only.` (evidence: INI scan; `0x004CBBA0 -> 0x0042C900`)
- `[RESOLVED] OQ-10 - Does Rust need stock pathfinding implementation? -> Not for ordinary stock Jumpjet air movement, but yes as a guardrail if implementing JumpJet-infantry ground/A* fallback.` (evidence: Rust source scan; binary branch)
- `[DEFERRED] OQ-11 - How often does retail YR hit this branch in live skirmish?` (category: needs-runtime-debugger; reason: static analysis proves conditions but not hit frequency; next-step-if-pursued: breakpoint/log `0x0042CA4F` with object type section and caller)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| If `AStar_pathfind_search` is entered by Infantry (`WhatAmI()==0xF`) with `JumpJet=yes`, gamemd coerces the movement-zone row to `7` (`Infantry`) before `Zone_precheck`/A*. Active in YR: Conditional. | `0x0042CA43..0x0042CA69`; `0x00523340`; `+0xD94` parser docs | Missing/unchecked for nonstandard JumpJet ground fallback | `src/sim/world/world_commands.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/pathfinding/zone_search.rs` | Any JumpJet-infantry ground/A* fallback should path as Infantry row, not Fly row. | Create a JumpJet infantry with `HoverAttack=no`, `MovementZone=Fly`, distance <=3 over infantry-passable terrain; fallback path should use Infantry row. Proposed test: `jumpjet_infantry_walk_fallback_uses_infantry_zone_row`. | Do not apply to normal airborne Jumpjet move orders. |
| The branch's COM block queries `IID_IPersist`, calls `GetClassID`, and releases; the returned CLSID is not consumed by this function. Active in YR: Conditional. | `0x0042CA73..0x0042CABC`; IID bytes at `0x00818858`; stack-reference scan | No Rust delta for COM; Rust has no COM locomotor layer | no gameplay surface unless Rust models binary diagnostic/error hooks | Implement no gameplay effect for the COM call; document it as non-routing validation. | Run JumpJet-infantry fallback path without any COM object; result should depend on Infantry row only. Proposed test: `jumpjet_infantry_astar_does_not_require_locomotor_com_class_id`. | Do not invent a locomotor class-ID based routing switch. |
| Stock `[JUMPJET]` and `[LUNR]` set `HoverAttack=yes`, so Rust's current short-walk fallback does not appear to trigger for those stock infantry; normal Jumpjet movement bypasses A*. Active in YR: Yes for stock data, No for normal branch hit. | `rulesmd.ini:3921/3966`, `4715/4758`; Fly/Jumpjet path-entry audit | likely no stock Rust delta | `src/sim/world/world_commands.rs`, air movement | Preserve normal Jumpjet air move bypass; add guardrail coverage if fallback remains. | Rocketeer move across disconnected land/water should issue air movement, not ground A*. Proposed test: `stock_rocketeer_move_bypasses_jumpjet_infantry_astar_row_override`. | Do not route stock Rocketeer movement through `zone_search` just to model this branch. |

### Negative Facts / Do Not Do

- Do not call this `Teleporter=`. Evidence: `TechnoType+0xD94` is `JumpJet=` in `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md` and `units/allied/JUMPJET.md`.
- Do not call the `0xF` gate a locomotor kind. Evidence: the call is vtable `+0x2C` (`WhatAmI`), and `InfantryClass::WhatAmI @ 0x00523340` returns `0xF`.
- Do not implement the COM block as a pathfinding extension. Evidence: the branch queries `IID_IPersist`, calls `GetClassID`, releases, and the output stack buffer is not read later in `AStar_pathfind_search`.
- Do not use Fly row 9 for JumpJet infantry that enters A*. Evidence: branch overwrites the local row with constant `7` at `0x0042CA69`.
- Do not apply the Infantry-row override to Jumpjet vehicles/aircraft. Evidence: `WhatAmI()==0xF` gate excludes non-Infantry objects even when their type has `JumpJet=yes`.

### Stale Docs / Follow-up Docs

- `docs/research/BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`: replace "locomotor-kind 0xF + `vtable+0x4C` (Drive-class hook)" wording with "`AStar_pathfind_search` calls object `WhatAmI` via vtable `+0x2C`; value `0xF` is InfantryClass, not a locomotor kind. If that Infantry object's type has `JumpJet=yes` (`TechnoType+0xD94`), the function coerces the local movement-zone row to `7` (`Infantry`) and then performs a non-routing `IID_IPersist::GetClassID` query on the locomotor."
- `docs/research/FLY_JUMPJET_ROW9_PATH_ENTRY_AUDIT_GHIDRA_REPORT.md`: narrow "Would A* consume Fly row 9 if entered with row 9?" to "A* consumes the supplied/derived row normally, except JumpJet infantry: `0x0042CA4F..0x0042CA69` rewrites the local row to `7` before `Zone_precheck`/A*."
- `docs/research/miner/MINER_DOCK_GAPS_RESEARCH.md`: replace "`TypeClass+0xD94` (Teleporter=yes)" with "`TypeClass+0xD94` is `JumpJet=yes`; Teleporter is a different field and must not be inferred from this offset."

## Sources

- Raw assembly read from retail `<ra2-install>/gamemd.exe`: `0x0042C900..0x0042CCCC`, target `0x0042CA4F..0x0042CABC`; `0x004CBBA0..0x004CBC3B`; `0x00523340`.
- Data bytes read from retail binary: GUID at `0x00818858`; strings at `0x008187F0`, `0x00818820`.
- Existing reports referenced: `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`, `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, `units/allied/JUMPJET.md`, `FLY_JUMPJET_ROW9_PATH_ENTRY_AUDIT_GHIDRA_REPORT.md`, `ABSTRACTCLASS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned read-only: `src/rules/locomotor_type.rs`, `src/rules/object_type.rs`, `src/sim/world/world_commands.rs`, `src/sim/movement/jumpjet_movement.rs`, `src/sim/pathfinding/zone_search.rs`.
