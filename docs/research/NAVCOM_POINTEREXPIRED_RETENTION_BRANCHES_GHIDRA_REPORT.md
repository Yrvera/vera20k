# NavCom PointerExpired Retention Branches - Ghidra Research Report

**Address(es):** `0x004D9960` (`FootClass::PointerExpired`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** NavCom cleanup/retention when `Foot+0x5A4` equals the expired pointer; `SuspendedNavCom` and queue scrub order only as needed for pointer expiry.
**Non-Scope:** NavCom producers, OnArrival tail hooks, Set_Destination preprocessing flags, full mission enum naming, and full pointer-expiry handling outside NavCom/SuspendedNavCom/NavQueue.
**Confidence:** High for branch mechanics and offsets; Medium for player-frequency of mission-8 Occupier retention.
**Active in YR:** Conditional. `PointerExpired` is active for Foot-derived objects; the retention branches are gated by runtime pointer, object-state, mission, sensor, and type conditions listed below.

## Working Notes

Target question: When `FootClass::PointerExpired @ 0x004D9960` sees `NavCom == expired`, exactly which branches retain vs clear NavCom, especially sensor-retained destroyed targets and the infantry Occupier branch?
Non-goals: Do not re-open NavCom producer research, Set_Destination preprocessing, OnArrival non-queue tail hooks, or unrelated TechnoClass pointer cleanup.
Evidence needed to mark COMPLETE: decompile plus assembly-context ranges for the clear/retain branches; xref/caller evidence that the function is live; INI/binary evidence for `InfantryType+0xEB4` as `Occupier`; current Rust surface and acceptance scenarios.
Stop conditions: stop after all scoped clear/retain paths, SuspendedNavCom order, EnterQueue/NavQueue scrub order, Rust handoff, and stale-doc wording are resolved or explicitly deferred.

## 1. Overview

`FootClass::PointerExpired` first delegates inherited cleanup, then processes Foot-owned references. For this slice, the important fact is that `NavCom` is not always cleared when it equals the expired pointer. It is retained if either the sensor branch clears the local `should_clear` flag or the infantry Occupier branch jumps around the final clear. Otherwise `NavCom_Aux` and `NavCom` are zeroed together.

## 2. Class Layout / Key Offsets

| Offset | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `Foot+0x5A0` | pointer | `NavCom_Aux`; zeroed only on the final NavCom clear path | decompile `0x004D9960`; asm `0x004D9ABD..0x004D9AC3` | Yes, conditional on final clear |
| `Foot+0x5A4` | pointer | `NavCom`; primary navigation target tested against expired pointer | decompile `0x004D9960`; asm compare `0x004D9A0F` | Yes |
| `Foot+0x5A8` | pointer | `SuspendedNavCom`; cleared before NavCom retention tests if it equals expired | decompile `0x004D9960`; asm `0x004D9A01..0x004D9A09` | Yes |
| `Foot+0x588/+0x58C/+0x598` | dynamic vector | NavQueue buffer and active count; scrubbed after EnterQueue | decompile `0x004D9960`; asm `0x004D9B97..0x004D9BE9` | Yes when count > 0 |
| `Foot+0x5AC/+0x5B0/+0x5BC` | dynamic vector | EnterQueue buffer and active count; scrubbed before NavQueue | decompile `0x004D9960`; asm `0x004D9B43..0x004D9B95` | Yes when count > 0 |
| `InfantryType+0xEB4` | byte | `Occupier=` flag | `BuildingClass::AddGarrisonOccupant @ 0x00522910`; `InfantryTypeClass::ReadINI @ 0x005244D5`; `rulesmd.ini` E1/E2/INIT | Conditional on infantry type |

## 3. Core Logic

### 3.1 Entry and live paths

`FootClass::PointerExpired` is entered through the Foot vtable and subclass wrappers. Ghidra xrefs show the Foot vtable data reference plus direct subclass wrappers:

| Caller / source | Evidence | Active in YR |
|---|---|---|
| Foot vtable data reference | xref from `0x007E8CBC` to `0x004D9960` | Yes for Foot-derived detach notifications |
| `AircraftClass::Detach` wrapper | decompile plus call at `0x0041B66E` | Yes for aircraft |
| infantry/building-label wrapper at `0x0051AA1E` | decompile plus call at `0x0051AA1E`; label is suspect but wrapper is live code | Conditional |
| Foot convoy-delete wrapper | decompile plus call at `0x007446EE` | Conditional |

The first call in `FootClass::PointerExpired` is inherited cleanup: asm `0x004D996F..0x004D9973` pushes the expired pointer and flag, then calls `0x007077C0` (`TechnoClass::PointerExpired`). This report does not claim the inherited cleanup, except that it runs before the Foot NavCom checks.

### 3.2 SuspendedNavCom is scrubbed before NavCom retention

If `Foot+0x5A8 == expired`, the function clears it before looking at `Foot+0x5A4`. Assembly `0x004D9A01..0x004D9A09` compares `[ESI+0x5A8]` with the expired pointer in `EDI`, then writes zero. Immediately after, asm `0x004D9A0F` compares `[ESI+0x5A4]` with the same expired pointer.

Active in YR: Yes. There is no TS-only flag in this branch. It is conditional only on `SuspendedNavCom == expired`.

### 3.3 Sensor-retained destroyed-target branch

When `NavCom == expired`, the function initializes the clear flag to true. It then checks:

- the caller flag byte (`param_3`) is zero;
- the expired pointer is non-null;
- expired object byte at `+0x14` has bit `0x01` set;
- the expired object can provide coordinates through vtable `+0x48`;
- the cell returned by `CellClass::Get_Cell_At` has nonzero `SensorCountForHouse(owner_house)`.

If all of those pass, asm `0x004D9A57..0x004D9A60` tests the sensor-count result and zeroes `BL`, the local should-clear flag. The later final-clear block tests `BL` at `0x004D9AB9`; with `BL == 0`, it jumps over the `NavCom_Aux/NavCom` zero writes.

Active in YR: Conditional. The code is on the active Foot pointer-expiry path and has no TS-only global gate. The branch only fires for `param_3 == 0`, an expired live/non-null object with object flag bit `0x01`, and a sensor-visible expired object's cell for the unit owner.

### 3.4 Infantry Occupier retention branch

The second retention path runs after the sensor test. It retains `NavCom` by jumping directly to `0x004D9AC9`, bypassing the zero writes at `0x004D9ABD` and `0x004D9AC3`, when all of the following hold:

- current mission getter via vtable `+0x184` returns `8` (`0x004D9A66..0x004D9A6F`);
- `What_Am_I` via vtable `+0x2C` returns `0x0F`, i.e. InfantryClass (`0x004D9A75..0x004D9A7B`);
- owner type pointer at `Foot+0x6C0` has `byte +0xEB4 != 0` (`0x004D9A7D..0x004D9A8B`);
- `NavCom` is non-null (`0x004D9A8D..0x004D9A95`);
- the expired object byte at `+0x14` has bit `0x02` set (`0x004D9A97..0x004D9A9B`);
- expired object active byte `+0x90` is nonzero (`0x004D9A9D..0x004D9AA5`);
- expired object health at `+0x6C` is greater than zero (`0x004D9AA7..0x004D9AAA`);
- expired object's current mission via vtable `+0x184` is not `0x13` (`0x004D9AAE..0x004D9AB7`).

If any required condition fails, control reaches `0x004D9AB9`; if the sensor branch did not already set `BL = 0`, the function clears `NavCom_Aux` and `NavCom`.

Active in YR: Conditional. `InfantryType+0xEB4` is live YR data: `InfantryTypeClass::ReadINI @ 0x005244D5` writes the bool, `BuildingClass::AddGarrisonOccupant @ 0x00522910` uses it for normal garrison admission, and stock YR `rulesmd.ini` has `Occupier=yes` for `[E1]`, `[E2]`, and `[INIT]` while `[GGI]` overrides to `Occupier=no;yes`. The exact mission-8 producer/frequency was not expanded in this slot.

### 3.5 Final clear and queue scrub order

If neither retention route bypasses the clear, the final clear writes:

1. `Foot+0x5A0 = 0` at `0x004D9ABD`;
2. `Foot+0x5A4 = 0` at `0x004D9AC3`.

After NavCom processing, the function handles a separate `+0x5C8/+0x5CC` target-cell cache, then scrubs queues:

1. EnterQueue active count `Foot+0x5BC` and buffer `Foot+0x5B0` at `0x004D9B43..0x004D9B95`;
2. NavQueue active count `Foot+0x598` and buffer `Foot+0x58C` at `0x004D9B97..0x004D9BE9`.

Both queue loops remove every matching entry, decrement count, shift remaining pointers left, then decrement the loop index so duplicate adjacent matches are also removed.

Active in YR: Yes, conditional on queue counts and matching entries.

## 4. INI Keys

| Key | Section(s) checked | Default / value | Effect | Evidence | Active in YR |
|---|---|---|---|---|---|
| `Occupier=` | infantry types | absent defaults false; stock `[E1]`, `[E2]`, `[INIT]` set yes; `[GGI]` has `no;yes` | Enables the `InfantryType+0xEB4` condition used by the retention branch | `InfantryTypeClass::ReadINI @ 0x005244D5`; `rulesmd.ini:3720,3870,4335,4877`; base `rules.ini:3137,3227` | Conditional |

No NavCom-specific INI key controls the sensor branch or the queue scrub order.

## 5. Integration Points

The active call chain is pointer-expiry/detach notification into `FootClass::PointerExpired`, with `TechnoClass::PointerExpired` always called first. Subclass wrappers at `0x0041B66E`, `0x0051AA1E`, and `0x007446EE` call `0x004D9960` and then clear their own fields.

The scoped function reads the expired object's vtable `+0x48` for coordinates, the owner house pointer through `Foot+0x21C -> +0x30` for the sensor query, the owner's current mission through vtable `+0x184`, the owner's class identity through vtable `+0x2C`, and the expired object's current mission through vtable `+0x184`.

## 6. Current Rust Implementation Status

Rust now has a partial `NavigationState` in `src/sim/components.rs` with `nav_com_aux`, `nav_com`, `suspended_nav_com`, and `nav_queue` variants using `NavTargetRef::{Cell, Entity}`. Normal cell move setup writes `nav_com` in `src/sim/movement/navcom.rs`, and queue appending appears in `src/sim/movement/movement_commands.rs`.

The scoped pointer-expiry behavior is still missing. `Simulation::despawn_entity` in `src/sim/world/mod.rs` clears radio contacts and removes the entity, but it does not scan other entities' `navigation` fields. Combat death handling decrements counts and despawns entities, but no FootClass-style `PointerExpired` pass scrubs `nav_com`, `suspended_nav_com`, or `nav_queue` with the verified retention exceptions.

One current mismatch outside this pointer-expiry slice: `finalize_finished_entities` clears `navigation.nav_queue` on normal movement completion. Native queue completion pops/rotates in OnArrival and PointerExpired only scrubs matching expired entries. That broader arrival behavior belongs to the OnArrival/NavQueue slots, not this report.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass::PointerExpired @ 0x004D9960` scoped body | verified | decompile `0x004D9960`; asm contexts `0x004D9960..0x004D9BE9` | none for scoped branches |
| Inherited `TechnoClass::PointerExpired` call order | verified for order only | asm call `0x004D996F..0x004D9973`; decompile `0x007077C0` | inherited field semantics out of scope |
| `SuspendedNavCom` clear before NavCom tests | verified | asm `0x004D9A01..0x004D9A0F` | none |
| Sensor retention | verified | decompile `0x004D9960`; asm `0x004D9A1B..0x004D9A60`, final test `0x004D9AB9` | runtime incidence not measured |
| Infantry Occupier retention | verified | decompile `0x004D9960`; asm `0x004D9A66..0x004D9AB7` | exact mission-8 producer/frequency out of scope |
| Final NavCom clear | verified | asm `0x004D9AB9..0x004D9AC3` | none |
| EnterQueue scrub before NavQueue | verified | asm `0x004D9B43..0x004D9BE9` | none |
| `InfantryType+0xEB4` identity | verified | `0x00522910`, `0x005244D5`, `rulesmd.ini` stock entries | none |
| Rust pointer-expiry cleanup | verified missing | `src/sim/world/mod.rs`, `src/sim/components.rs`, `src/sim/movement/navcom.rs` | implement future cleanup pass |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the exact target? -> `FootClass::PointerExpired @ 0x004D9960` NavCom/SuspendedNavCom/NavQueue cleanup when `NavCom == expired`.` (evidence: user scope; decompile `0x004D9960`)
- `[RESOLVED] OQ-02 - Is this function on active YR paths? -> Yes/conditional: Foot vtable data plus subclass wrappers call it.` (evidence: xrefs `0x007E8CBC`, `0x0041B66E`, `0x0051AA1E`, `0x007446EE`)
- `[RESOLVED] OQ-03 - Does `SuspendedNavCom` clear before NavCom retention tests? -> Yes, `+0x5A8` is tested/cleared before `+0x5A4` is compared.` (evidence: asm `0x004D9A01..0x004D9A0F`)
- `[RESOLVED] OQ-04 - Does sensor visibility retain `NavCom`? -> Yes, when `param_3 == 0`, expired object bit `0x01` is set, and sensor count for owner house is nonzero.` (evidence: asm `0x004D9A1B..0x004D9A60`)
- `[RESOLVED] OQ-05 - Does infantry `Occupier` retain `NavCom`? -> Yes, but only under the full mission/class/type/object-liveness gate listed in section 3.4.` (evidence: asm `0x004D9A66..0x004D9AB7`)
- `[RESOLVED] OQ-06 - Is `+0xEB4` really `Occupier`? -> Yes.` (evidence: `BuildingClass::AddGarrisonOccupant @ 0x00522910`; `InfantryTypeClass::ReadINI @ 0x005244D5`; `rulesmd.ini` stock entries)
- `[RESOLVED] OQ-07 - What exactly clears `NavCom`? -> Only the final clear path writes `+0x5A0=0` and `+0x5A4=0`; retention branches skip these writes.` (evidence: asm `0x004D9AB9..0x004D9AC3`)
- `[RESOLVED] OQ-08 - Are duplicate queue entries removed? -> Yes, loops decrement the loop index after a removal, so adjacent duplicates are rechecked.` (evidence: decompile `0x004D9960`; asm `0x004D9B8B..0x004D9B95`, `0x004D9BDF..0x004D9BE9`)
- `[RESOLVED] OQ-09 - Which queue is scrubbed first? -> EnterQueue first, NavQueue second.` (evidence: asm `0x004D9B43..0x004D9BE9`)
- `[RESOLVED] OQ-10 - Is this TS-only? -> No TS-only gate was found; branches are runtime-conditional YR code.` (evidence: decompile `0x004D9960`; stock `Occupier=` data)
- `[RESOLVED] OQ-11 - What Rust surface is affected? -> `NavigationState` and despawn/death cleanup paths.` (evidence: `src/sim/components.rs`, `src/sim/world/mod.rs`)
- `[DEFERRED] OQ-12 - What player action most often produces mission value 8 for the Occupier branch?` (category: out-of-scope; reason: this slot only verifies PointerExpired retention mechanics; next-step-if-pursued: trace mission assignment producers for mission enum `8`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `SuspendedNavCom` is cleared before NavCom clear/retention tests when it equals the expired pointer. | `0x004D9A01..0x004D9A0F` | missing cleanup pass | `src/sim/components.rs`, `src/sim/world/mod.rs` despawn/death cleanup | Scan live entities on despawn and clear `suspended_nav_com == Entity(expired)` regardless of later NavCom retention. | Destroy a target referenced by both `suspended_nav_com` and `nav_com`; suspended clears even if NavCom is sensor-retained. Proposed test: `pointer_expired_clears_suspended_before_sensor_retains_navcom`. | Do not tie suspended cleanup to the final NavCom clear flag. |
| `NavCom == expired` is retained if the sensor branch sets `should_clear=false`; otherwise the final clear zeroes `NavCom_Aux` and `NavCom` unless the Occupier branch bypasses it. | `0x004D9A1B..0x004D9A60`, `0x004D9AB9..0x004D9AC3` | missing cleanup pass and sensor-visible cell check | navigation cleanup plus vision/sensor query surface | For entity NavCom expiry, compute native-equivalent retention before clearing. Retained NavCom must not clear aux via the final path. | Unit moving toward an enemy entity whose death cell is sensor-visible keeps `nav_com = Entity(dead)`/or an equivalent retained target until later resolution, while non-visible death clears. Proposed test: `pointer_expired_sensor_visible_target_retains_navcom`. | Do not always clear entity NavCom on despawn; that loses native pursuit-to-last-known-cell behavior. |
| Occupier infantry can retain `NavCom` under mission `8`, infantry class, `Occupier=yes`, expired object bit `0x02`, active byte nonzero, health positive, and expired mission not `0x13`. | `0x004D9A66..0x004D9AB7`; `0x00522910`; `0x005244D5`; `rulesmd.ini:3720,4335,4877` | missing mission enum and cleanup predicate | future mission state + `NavigationState` cleanup | Preserve NavCom only when the complete gate matches; any failed condition falls back to sensor/final-clear behavior. | Occupier GI in mission 8 targeting a still-active, positive-health object keeps NavCom; same setup with `Occupier=no` GGI clears. Proposed test: `pointer_expired_occupier_mission8_retains_navcom_only_when_all_gates_pass`. | Do not reduce this to "all infantry Occupier retains"; the target object liveness bits, health, and mission `0x13` exclusion are part of the gate. |

### Negative Facts / Do Not Do

- Do not always clear `NavCom` when the expired pointer matches. Evidence: sensor branch zeroes `BL` at `0x004D9A60`; Occupier branch jumps to `0x004D9AC9` at `0x004D9AB7`.
- Do not treat `SuspendedNavCom` as protected by retention. Evidence: `+0x5A8` is cleared at `0x004D9A09` before the `+0x5A4` compare.
- Do not scrub only the first queue match. Evidence: both queue loops decrement the loop index after removal, rechecking shifted entries.
- Do not treat `InfantryType+0xEB4` as an unknown TS-era flag. Evidence: `InfantryTypeClass::ReadINI @ 0x005244D5` writes it from the bool chain, `BuildingClass::AddGarrisonOccupant @ 0x00522910` uses it, and stock YR infantry set it.
- Do not clear `NavCom_Aux` on retained NavCom. Evidence: the only `+0x5A0` clear in the scoped NavCom block is the final-clear path at `0x004D9ABD`, skipped by both retention paths.

### Stale Docs / Follow-up Docs

- `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` section 11 OQ-5 can be replaced with: "Resolved by `NAVCOM_POINTEREXPIRED_RETENTION_BRANCHES_GHIDRA_REPORT.md`: the `should_clear=false` infantry branch is not a generic capture shortcut. It retains `NavCom` only when current mission via vtable `+0x184` is `8`, `What_Am_I` is `0x0F`, `InfantryType+0xEB4` (`Occupier`) is true, current `NavCom` is non-null, the target has object flag bit `0x02`, target active byte `+0x90` is true, target health `+0x6C` is positive, and target mission via vtable `+0x184` is not `0x13`; otherwise the final clear path still applies unless sensor retention already set `should_clear=false`."
- `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` section 10 current Rust status is stale after the newer NavCom phase: Rust now has `NavigationState` with `nav_com`, `suspended_nav_com`, and `nav_queue`, but still lacks the verified `PointerExpired` cleanup/retention pass.

## Remaining Uncertainty

- Exact producer/frequency of current mission value `8` for the Occupier retention branch was intentionally not expanded. The binary branch and type gate are verified; how often normal YR enters that state should be traced in a mission-assignment slot if prioritization needs it.

## Sources

- Ghidra: `0x004D9960` (`FootClass::PointerExpired`), `0x007077C0` (`TechnoClass::PointerExpired` inherited call target), `0x005B3040` (`MissionClass::GetCurrentMission`), `0x00522910` (`BuildingClass::AddGarrisonOccupant`), `0x005244D5` (`InfantryTypeClass::ReadINI`), wrappers/call sites `0x0041B66E`, `0x0051AA1E`, `0x007446EE`.
- Assembly contexts: `0x004D9960..0x004D9BE9`, especially `0x004D9A01..0x004D9AC3` and `0x004D9B43..0x004D9BE9`.
- Docs: `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`, `docs/research/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`, `docs/research/BUILDINGCLASS_VTABLE_184_IDENTITY_GHIDRA_REPORT.md`, `docs/research/GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini:3720`, `ini/rulesmd.ini:3870`, `ini/rulesmd.ini:4335`, `ini/rulesmd.ini:4877`; base `ini/rules.ini:3137`, `ini/rules.ini:3227`.
- Rust scan: `src/sim/components.rs`, `src/sim/movement/navcom.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/world/mod.rs`, `src/sim/movement/movement_tick.rs`.
