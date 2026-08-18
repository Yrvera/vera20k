# AircraftClass 0xA5 Radio Gate Writers - Ghidra Research Report

**Address(es):** `0x004190B0` (`AircraftClass::Receive_Radio`), writer chain at `0x006F3F40`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** writers/readers of decompiler field `param_1[0xA5]` used by `AircraftClass::Receive_Radio` to gate missions `4`, `0x1A`, `0x1B`, `0x1E`, `0x1F`.
**Non-Scope:** full AircraftClass mission state machine, AirstrikeClass plane-spawn AI, full Boris airstrike behavior.
**Confidence:** High
**Active in YR:** Conditional. The gate code is active for all AircraftClass objects; stock YR creates the `+0x294` AirstrikeClass pointer only for `[BORIS]`, which is InfantryClass, not AircraftClass. Stock aircraft therefore normally have `+0x294 == 0` and are radio-gated during the scoped missions.

## 0. Investigation Contract

**Target question:** What writes the AircraftClass/Foot-era decompiler field `+0xA5` used by `AircraftClass::Receive_Radio`, what does it mean, which missions set/clear it, and is the gated behavior active in standard YR?

**Non-goals:** Do not re-document all AircraftClass missions; do not implement Rust; do not mutate Ghidra; do not audit the whole Boris airstrike subsystem.

**Evidence needed to mark COMPLETE:**

- Decompile plus assembly context for the `Receive_Radio` gate.
- Decompile plus assembly/context for every scoped suspected mission writer.
- Decompile plus assembly/context for actual `+0x294` writers.
- INI/default evidence for the TechnoType fields that cause `+0x294` allocation.
- Current Rust surface scan and one test-name handoff.

**Stop conditions:** Stop once `param_1[0xA5]` is identified, all direct writes relevant to Aircraft/Techno ownership are classified, suspected paradrop mission writers are proven negative, and standard YR activity is resolved from stock INI.

## 1. Overview

The decompiler expression `param_1[0xA5]` is not byte offset `+0xA5`; because `param_1` is `int*`, it is byte offset `0xA5 * 4 = +0x294`. In TechnoClass/FootClass layout this is `AirstrikeClass*`, created by `TechnoClass::Init_Managers` when the object's TypeClass has `AirstrikeTeam > 0`.

`AircraftClass::Receive_Radio` uses this pointer as an exception to a mission firewall. If an aircraft is in mission `4`, `0x1A`, `0x1B`, `0x1E`, or `0x1F` and `+0x294 == 0`, the radio receive function returns `0` before handling the incoming message. No scoped aircraft/paradrop mission writes this pointer.

## 2. Class Layout / Key Offsets

| Offset | Decompiler form | Type | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `+0x294` | `param_1[0xA5]` | `AirstrikeClass*` | Per-Techno airstrike manager pointer | `TechnoClass::Init_Managers @ 0x006F3F40`; `FOOTCLASS_NON_MOVEMENT_FIELDS.md`; `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` | Conditional |
| `AirstrikeClass+0x4C` | `param_1[0x13]` in AirstrikeClass ctor | `TechnoClass*` | Owner/back-pointer used to validate manager ownership | `AirstrikeClass::Constructor @ 0x0041D380` | Conditional |
| `TechnoTypeClass+0x61C` | `AirstrikeTeam` | int | Allocates `AirstrikeClass` if `> 0` | `TechnoClass::Init_Managers @ 0x006F3F40`; `TechnoTypeClass::ReadINI @ 0x00714591` | Yes for Boris only in stock YR |
| `TechnoTypeClass+0x620` | `EliteAirstrikeTeam` | int | Copied into AirstrikeClass | `AirstrikeClass::Constructor @ 0x0041D380` | Yes for Boris only |
| `TechnoTypeClass+0x624/+0x628` | `AirstrikeTeamType` / `EliteAirstrikeTeamType` | AircraftTypeClass* | Copied into AirstrikeClass | `AirstrikeClass::Constructor @ 0x0041D380` | Yes for Boris only |
| `TechnoTypeClass+0x62C/+0x630` | recharge times | int | Copied into AirstrikeClass | `AirstrikeClass::Constructor @ 0x0041D380` | Yes for Boris only |

## 3. Core Logic

### Receive_Radio gate

Decompile at `0x004190B0`:

```text
switch (this->Mission) {
  case 4:
  case 0x1A:
  case 0x1B:
  case 0x1E:
  case 0x1F:
    if (this->AirstrikePtr == 0) return 0;
}
continue normal AircraftClass radio switch...
```

Assembly context confirms the byte offset:

```text
004190b6: MOV EAX,dword ptr [ESI + 0xac]
004190bc: ADD EAX,-0x4
004190bf: CMP EAX,0x1b
004190cc: JMP dword ptr [ECX*0x4 + 0x41952c]
004190d3: MOV EAX,dword ptr [ESI + 0x294]
004190d9: TEST EAX,EAX
004190db: JNZ 0x004190e6
004190e0: XOR EAX,EAX
004190e3: RET 0xc
```

The same mission set and same pointer exception appear in:

- `AircraftClass::Enter_Idle_Mode @ 0x004176F0`; assembly `0x00417750` reads `[ESI+0x294]` and returns early when it is zero.
- `AircraftClass::Assign_Mission @ 0x0041B9F0`; assembly `0x0041BA12` reads `[ECX+0x294]` to decide whether to allow non-scoped replacement missions.
- `AircraftClass::Queue_Mission_Override @ 0x0041BA90`; assembly `0x0041BAB2` reads `[ECX+0x294]`.
- `AircraftClass::Set_NavCom_Override @ 0x0041BB30`; assembly `0x0041BB52` reads `[ECX+0x294]`.

### Actual writers

| Writer | Write | Evidence | Meaning |
|---|---|---|---|
| `TechnoClass::Constructor` | `this+0x294 = 0` | decompile plus assembly at `0x006F2E09`: `MOV dword ptr [ESI + 0x294],EBX` after `EBX=0` | Default: no AirstrikeClass manager. |
| `TechnoClass::Init_Managers` | `this+0x294 = new AirstrikeClass(this)` if `Type+0x61C > 0` | decompile `0x006F3F40`; assembly at `0x006F41EE`: `MOV dword ptr [ESI + 0x294],EAX` after `CALL 0x0041D380` | Creates per-techno airstrike manager. |
| `TechnoClass` destructor chain | releases `this+0x294` if manager owner pointer matches `this`, then clears it | decompile `0x006F4510`; assembly at `0x006F45BE`: `MOV dword ptr [ESI + 0x294],EBX` | Owned-manager cleanup only. |
| `TechnoClass::PointerExpired` | clears `this+0x294` if expired pointer equals current pointer | decompile `0x00707800`; assembly `0x0070782D` compare, `0x00707835` clear | Generic pointer invalidation / detach cleanup. |

### Negative writer check for suspected missions

No scoped aircraft mission writes `+0x294`.

- `Mission_ParaDropApproach @ 0x004155F0` writes `+0x6D2` (`IsStrafe`) and queues mission `0x1F`; no `+0x294` write.
- `Mission_ParaDropOverfly @ 0x004157C0` only updates reveal/path destination; no `+0x294` write.
- `Mission_Open @ 0x004158E0` decrements `+0x6D3` payload count and queues mission `0x1B`; no `+0x294` write.
- `Mission_Rescue @ 0x00415960` writes `+0x6D2`, checks `+0x6D3`, queues `0x1A` or `4`; no `+0x294` write.
- `Mission_QMove @ 0x00415A50` assigns off-map destination or clears destination; no `+0x294` write.

## 4. INI Keys

| Key | Section in stock YR | Offset | Default | Stock value | Effect |
|---|---|---:|---:|---|---|
| `AirstrikeTeam` | `[BORIS]` | `TechnoType+0x61C` | `0` | `2` in `rulesmd.ini:4649` | Enables allocation of `AirstrikeClass` at `Techno+0x294`. |
| `EliteAirstrikeTeam` | `[BORIS]` | `+0x620` | `0` | `4` in `rulesmd.ini:4650` | Elite team size copied into manager. |
| `AirstrikeTeamType` | `[BORIS]` | `+0x624` | null | `BPLN` in `rulesmd.ini:4652` | Aircraft type copied into manager. |
| `EliteAirstrikeTeamType` | `[BORIS]` | `+0x628` | null | `BPLN` in `rulesmd.ini:4653` | Elite aircraft type copied into manager. |
| `AirstrikeRechargeTime` | `[BORIS]` | `+0x62C` | `0` | `100` in `rulesmd.ini:4655` | Cooldown copied into manager. |
| `EliteAirstrikeRechargeTime` | `[BORIS]` | `+0x630` | `0` | `50` in `rulesmd.ini:4656` | Elite cooldown copied into manager. |

Reader evidence: `TechnoTypeClass::ReadINI @ 0x00714591..0x0071463F` reads these fields. Assembly shows `PUSH 0x843b84` (`AirstrikeTeam`) then stores `EAX` to `[EBP+0x61C]`; subsequent reads store `[EBP+0x620]`, resolve type strings into `[EBP+0x624/+0x628]`, and store recharge fields `[EBP+0x62C/+0x630]`.

Stock INI scan found these keys only under `[BORIS]` in `rulesmd.ini`. No stock aircraft type (`[AircraftTypes]` entries such as `BPLN`, `PDPLANE`, `CARGOPLANE`, `ORCA`, `BEAG`, `HORNET`) has `AirstrikeTeam`.

## 5. Integration Points

- `AircraftClass::Constructor @ 0x00413D20` calls `AircraftClass::InitFromType`.
- `AircraftClass::InitFromType @ 0x00413F80` calls `TechnoClass::Init_Managers`, so an aircraft type with `AirstrikeTeam > 0` would get a non-null `+0x294`.
- `TechnoClass::Init_Managers @ 0x006F3F40` is shared by buildings, infantry, units, and aircraft. Xrefs include `AircraftClass::InitFromType @ 0x00413F85`, `InfantryClass::InitFromType @ 0x00517CC4`, `UnitClass::Constructor @ 0x007355F1`, `UnitClass::InitFromType @ 0x00746814`, and `BuildingClass::Init_Managers @ 0x00442C43`.
- The radio gate is local to `AircraftClass::Receive_Radio`; Boris gets an AirstrikeClass pointer in standard YR, but Boris is InfantryClass, so this exact AircraftClass radio gate is not reached by Boris.

## 6. Current Rust Implementation Status

Rust has aircraft mission and paradrop surfaces:

- `src/sim/aircraft/mod.rs`: `AircraftMission::{ParaDropApproach, ParaDropOverfly, Move, Guard, Attack, ReturnToBase, Docking}` and `tick_aircraft_missions`.
- `src/sim/aircraft/paradrop_mission.rs`: approach/overfly handlers.
- `src/sim/superweapon/paradrop.rs`: spawns paradrop aircraft with initial `AircraftMission::ParaDropApproach`.
- `src/sim/docking/aircraft_dock.rs`: aircraft ammo/docking support.

Rust does not currently model `TechnoClass+0x294` / `AirstrikeClass*`; source scan found `Airstrike` only as a warhead flag parser (`src/rules/warhead_type.rs`), not the per-Techno airstrike manager or the aircraft radio exception.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AircraftClass::Receive_Radio` mission gate | verified | decompile `0x004190B0`; assembly `0x004190B6..0x004190E3` | none |
| Offset identity `param_1[0xA5] == byte +0x294` | verified | decompile pointer type `int*`; assembly `[ESI+0x294]` at `0x004190D3` | none |
| `TechnoClass::Constructor` default clear | verified | decompile; assembly `0x006F2E09` | none |
| `TechnoClass::Init_Managers` allocation writer | verified | decompile `0x006F3F40`; assembly `0x006F41EE`; caller xrefs | none |
| `AirstrikeClass::Constructor` owner/type copies | verified | decompile `0x0041D380` | none |
| `TechnoClass` destructor cleanup | verified | decompile `0x006F4510`; assembly `0x006F45BE` | none |
| `TechnoClass::PointerExpired` cleanup | verified | decompile `0x00707800`; assembly `0x0070782D/0x00707835` | none |
| `Mission_ParaDropApproach` as suspected writer | verified-negative | decompile `0x004155F0` | none |
| `Mission_ParaDropOverfly` as suspected writer | verified-negative | decompile `0x004157C0` | none |
| `Mission_Open`, `Mission_Rescue`, `Mission_QMove` | verified-negative | decompile `0x004158E0`, `0x00415960`, `0x00415A50` | none |
| Stock YR INI activity | verified | `rulesmd.ini:4593`, `4649..4656`; no stock aircraft `AirstrikeTeam` keys found | none |
| Full AirstrikeClass plane-spawn behavior | deferred | out-of-scope | separate Boris airstrike investigation if needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the target field? -> decompiler index 0xA5 is byte +0x294, an AirstrikeClass pointer.` (evidence: `0x004190D3`, `0x006F3F40`, `FOOTCLASS_NON_MOVEMENT_FIELDS.md`)
- `[RESOLVED] OQ-02 - Does Receive_Radio gate the five scoped missions? -> yes, mission set {4, 0x1A, 0x1B, 0x1E, 0x1F} returns 0 if +0x294 is null.` (evidence: `0x004190B0`, assembly `0x004190B6..0x004190E3`)
- `[RESOLVED] OQ-03 - Do ParaDropApproach or ParaDropOverfly write +0x294? -> no.` (evidence: decompile `0x004155F0`, `0x004157C0`)
- `[RESOLVED] OQ-04 - Do Open/Rescue/QMove write +0x294? -> no.` (evidence: decompile `0x004158E0`, `0x00415960`, `0x00415A50`)
- `[RESOLVED] OQ-05 - What creates +0x294? -> TechnoClass::Init_Managers creates AirstrikeClass when Type+0x61C > 0.` (evidence: `0x006F3F40`, assembly `0x006F41EE`)
- `[RESOLVED] OQ-06 - What clears +0x294? -> constructor clears to 0; destructor and PointerExpired clear owned/expired pointers.` (evidence: `0x006F2E09`, `0x006F45BE`, `0x00707835`)
- `[RESOLVED] OQ-07 - Which stock YR objects get +0x294? -> stock INI has AirstrikeTeam keys only under [BORIS], not aircraft types.` (evidence: `rulesmd.ini:4593`, `4649..4656`; INI grep)
- `[RESOLVED] OQ-08 - Is the radio gate active in standard YR? -> yes as a gate/default block for aircraft in the scoped missions; the AirstrikeClass exception is not used by stock aircraft.` (evidence: `0x004190B0`, stock INI)
- `[DEFERRED] OQ-09 - How does AirstrikeClass spawn/manage BPLN planes?` (category: out-of-scope; reason: target is radio receive gating only; next-step-if-pursued: investigate AirstrikeClass AI/callers from `0x0041D380` and `Warhead Airstrike` detonation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Aircraft radio receive returns `0` before normal handling for missions `4`, `0x1A`, `0x1B`, `0x1E`, `0x1F` when `Techno+0x294 == null`. | `AircraftClass::Receive_Radio @ 0x004190B0`; assembly `0x004190D3..0x004190E3` | missing/unchecked: Rust has radio contacts but no AircraftClass receive switch or Airstrike manager exception | future `sim` radio/aircraft command receiver; `src/sim/aircraft/mod.rs`; `src/sim/superweapon/paradrop.rs` | During paradrop/open/rescue/qmove-style aircraft missions, ignore incoming radio commands unless an AirstrikeClass-style manager exists. | Proposed test: `aircraft_radio_receive_blocks_non_airstrike_paradrop_missions` | Do not implement this as a new paradrop flag; it is the generic `AirstrikeClass*` pointer. |
| No scoped paradrop mission sets/clears the gate pointer; missions only transition mission IDs or write `+0x6D2/+0x6D3`. | decompile `0x004155F0`, `0x004157C0`, `0x004158E0`, `0x00415960`, `0x00415A50` | Rust paradrop mission currently has no such pointer, which is fine for stock aircraft gating but incomplete for moddable AirstrikeTeam aircraft | `src/sim/aircraft/paradrop_mission.rs`; future type manager state | Keep paradrop payload/strafe state separate from Airstrike manager existence. | Paradrop aircraft receiving a dock/move-away radio while over drop target remains on drop path. | Do not set/clear a fake `radio_gate` when entering/leaving ParaDropApproach/Overfly. |
| `+0x294` is allocated from `AirstrikeTeam > 0`, copied from TechnoType airstrike keys; stock YR only Boris has these keys. | `TechnoClass::Init_Managers @ 0x006F3F40`; `AirstrikeClass::Constructor @ 0x0041D380`; `TechnoTypeClass::ReadINI @ 0x00714591..0x0071463F`; `rulesmd.ini:4649..4656` | missing: Rust parses Warhead `Airstrike`, but not TechnoType `AirstrikeTeam*` manager fields | rules parser for TechnoType/ObjectType, future Airstrike manager sim | If AirstrikeTeam support is added, the manager should live on all Techno categories, not only aircraft. | Boris airstrike cooldown/team fields create a manager for Boris, while stock BPLN/PDPLANE do not. | Do not special-case this as AircraftType-only or Boris-only in core layout. |

## Negative Facts / Do Not Do

- Do not treat `param_1[0xA5]` as byte offset `+0xA5`; it is byte `+0x294`.
- Do not call it a paradrop mission flag. It is `AirstrikeClass*`.
- Do not list `Mission_ParaDropApproach` or `Mission_ParaDropOverfly` as writers; both are verified negative.
- Do not assume the AirstrikeClass exception is used by stock aircraft; stock airstrike owner is `[BORIS]` infantry.
- Do not use `+0x6D2` (`IsStrafe`) or `+0x6D3` (payload/landing byte in paradrop reports) to emulate this radio gate.

## Remaining Uncertainty

The exact AirstrikeClass AI lifecycle and BPLN spawn/return handling were not re-investigated because they are outside this radio-gate slice. This does not affect the identity or writer set for `Techno+0x294`.

## Stale Docs / Follow-up Docs

Suggested replacement wording for broad aircraft docs that currently imply "no owner building" around this gate:

> In the AircraftClass mission/radio protection overrides, decompiler `param_1[0xA5]` is `TechnoClass+0x294`, the `AirstrikeClass*` manager pointer. The scoped missions `{QMove=4, Open=0x1A, Rescue=0x1B, ParaDropApproach=0x1E, ParaDropOverfly=0x1F}` block radio/mission/nav replacement when this pointer is null. It is not an owner-building pointer and is not written by the paradrop missions.

## Sources

- Ghidra decompile: `0x004190B0`, `0x004155F0`, `0x004157C0`, `0x004158E0`, `0x00415960`, `0x00415A50`, `0x004176F0`, `0x0041B9F0`, `0x0041BA90`, `0x0041BB30`, `0x00413F80`, `0x0041D380`, `0x006F3F40`, `0x006F4510`, `0x00707800`.
- Ghidra assembly contexts: `0x004190D3`, `0x00417750`, `0x0041BA12`, `0x0041BAB2`, `0x0041BB52`, `0x006F2E09`, `0x006F41EE`, `0x006F45BE`, `0x0070782D`, `0x00707835`, `0x00714591..0x0071463F`.
- Existing docs: `FOOTCLASS_NON_MOVEMENT_FIELDS.md`, `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`, `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, `AIRCRAFTCLASS_GHIDRA_REPORT.md`, `units/soviet/BORIS.md`, `units/soviet/BPLN.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
