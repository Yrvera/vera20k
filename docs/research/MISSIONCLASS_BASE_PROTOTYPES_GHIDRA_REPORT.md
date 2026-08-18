# MissionClass Base Prototypes — Ghidra Research Report

**Date:** 2026-08-17  
**Binary:** active Yuri's Revenge `gamemd.exe` in Ghidra  
**Mode:** exhaustive slice  
**Scope:** exactly seven base `MissionClass` functions at `0x005B2FD0`, `0x005B3040`,
`0x005B3060`, `0x005B3570`, `0x005B35E0`, `0x005B3650`, and `0x005B36B0`  
**Mutation status:** the research pass was read-only; the bounded application described below was
later applied, independently audited, saved, and read back from `gamemd.exe`

## Application update — 2026-08-17

The safe sequence from this report was subsequently executed against the active Ghidra program:

- Created a new four-byte `/YR_Mission` enum with exactly 33 entries: `NONE = -1` and the verified
  contiguous YR IDs `SLEEP = 0` through `SPYPLANE_OVERFLY = 31`. The stale `/RA2/Mission` type was
  left untouched.
- Retagged only `MissionClass::CurrentMission @ +0xAC`, `SuspendedMission @ +0xB0`, and
  `QueuedMission @ +0xB4` to `/YR_Mission`. Final structure size remains exactly `0xD4`.
- Applied the seven verified `__thiscall` prototypes. `Queue_Mission` is `void`, `Commence` and
  `Restore_Mission` return one-byte `bool`, and `Override_Mission` retains temporary `void *`
  target/destination parameters until a canonical `AbstractClass` datatype exists.
- Associated all seven implicit ECX receivers with `MissionClass *`. Ghidra therefore moved the
  symbols into the `MissionClass` namespace. Their existing leaf names were deliberately not
  renamed, so the fully qualified display is cosmetically redundant (for example,
  `MissionClass::MissionClass__Queue_Mission`) but semantically correct.
- A field-type helper briefly cleared the three field names. This was detected immediately and
  repaired by offset before any prototype application or save. Final readback contains all three
  original names and types.
- No auto-analysis, re-import, destructive datatype recreation, or unrelated rename was run.

An independent read-only auditor and a second post-save assertion pass both found zero mismatches:
33 enum entries, structure size `212 / 0xD4`, seven class members, the expected return and argument
types, ECX auto-`this`, and exact x86 stack storage all match this report. Ghidra reported
`Program saved successfully`.

## Research verdict (pre-application)

All seven machine-level ABIs were resolved. At the end of the read-only research phase, the
prototype writes were held pending a safe mission enum and an explicit decision about the missing
base pointer datatype:

1. `/RA2/Mission` is four bytes wide but contains a stale, shifted value table. Using it
   would produce ABI-correct but semantically wrong decompilation.
2. No `AbstractClass` datatype exists. The two polymorphic pointer arguments of
   `Override_Mission` can be represented temporarily as `void *`, but their proven semantic type
   is `AbstractClass *`.

The safe parser declarations below all passed `validate_function_prototype`; the later application
used `/YR_Mission` in place of `int` and retained `void *` for the two unresolved base pointers:

```c
void __thiscall MissionClass__Assign_Mission(int mission);
int  __thiscall MissionClass__GetCurrentMission(void);
void __thiscall MissionClass__Mission_Dispatch(void);
bool __thiscall MissionClass__Commence(void);
void __thiscall MissionClass__Queue_Mission(int mission, bool commence_now);
void __thiscall MissionClass__Override_Mission(
    int mission,
    void *target,
    void *destination
);
bool __thiscall MissionClass__Restore_Mission(void);
```

After a correct YR mission enum and a canonical `AbstractClass` datatype exist, the three mission
parameters currently declared as `int` and the `GetCurrentMission` return should use the corrected enum, while the two
`void *` positions should become `AbstractClass *`.

## Ownership and virtual-slot identity

The primary vtable is `0x007EDCC0`. Its preceding pointer at `0x007EDCBC` resolves to complete
object locator `0x00805D28`; the COL has offset zero and points to type descriptor `0x00817B18`,
whose name is `.?AVMissionClass@@`.

| Vtable slot | Function | Address |
|---:|---|---:|
| `+0x05C` | `Mission_Dispatch` | `0x005B3060` |
| `+0x184` | `GetCurrentMission` | `0x005B3040` |
| `+0x1E8` | `Queue_Mission` | `0x005B35E0` |
| `+0x1EC` | `Commence` | `0x005B3570` |
| `+0x1F0` | `Assign_Mission` | `0x005B2FD0` |
| `+0x1F4` | `Override_Mission` | `0x005B3650` |
| `+0x1F8` | `Restore_Mission` | `0x005B36B0` |

Raw bytes at `0x007EDEA8` are
`E0 35 5B 00 70 35 5B 00 D0 2F 5B 00 50 36 5B 00 B0 36 5B 00`, proving the
five consecutive verb slots directly.

## Exact ABI ledger

### `0x005B2FD0` — Assign_Mission

**Accepted prototype:** `void __thiscall Assign_Mission(int mission)` until the enum is fixed.

- `ECX` is the receiver; the first instruction reads `[ECX+0xAC]`.
- After a local `SUB ESP,0xC`, `MOV EAX,[ESP+0x10]` reads the original first argument as a full
  dword.
- `RET 4` proves one four-byte stack slot.
- The exit value in `EAX` is path-dependent incidental state, not a coherent return contract.
- An active caller at `0x007017B6..0x007017C0` pushes mission `5`, invokes vtable `+0x1F0`, and
  immediately overwrites `AL`; it does not consume a return value.

**Result:** `void`, one 32-bit mission argument.

### `0x005B3040` — GetCurrentMission

**Accepted prototype:** `int __thiscall GetCurrentMission(void)` until the enum is fixed.

- Uses `ECX` only and ends in a plain `RET`.
- Returns `[this+0xAC]`; if it is `-1`, returns `[this+0xB4]` instead.
- `UnitClass::PerCellProcess` calls vtable `+0x184` at `0x0073A324` and `0x0073A334`, then compares
  the full `EAX` to `7` and `0x19`.

**Result:** one 32-bit mission return, no explicit arguments.

### `0x005B3060` — Mission_Dispatch

**Accepted prototype:** `void __thiscall Mission_Dispatch(void)`.

- Saves `ECX` as the receiver in `EDI` and has no incoming argument cleanup.
- All 31 exits are plain `RET` instructions.
- Different dispatch paths leave unrelated values in `EAX`; there is no uniform return contract.
- The active Techno AI path calls it directly at `0x006FA655` and immediately loads a new value
  into `EDX`, ignoring the call result.

**Result:** `void`, no explicit arguments.

### `0x005B3570` — Commence

**Accepted prototype:** `bool __thiscall Commence(void)`.

- Uses `ECX` as the receiver, has no explicit arguments, and ends in plain `RET` on both paths.
- Success executes `MOV AL,1`; failure executes `XOR AL,AL`.
- `BuildingClass::Update` calls vtable `+0x1EC` at `0x0043FE43` and tests `AL` at
  `0x0043FE49`.
- Only `AL` is authoritative. The upper 24 bits of `EAX` are not normalized on success or failure,
  so a four-byte `BOOL` return would be wrong.

**Result:** one-byte C++ `bool`, no explicit arguments.

### `0x005B35E0` — Queue_Mission

**Accepted prototype:** `void __thiscall Queue_Mission(int mission, bool commence_now)` until the
enum is fixed.

- Entry `MOV EAX,[ESP+4]` reads the mission as a full dword.
- After `PUSH ESI`, `MOV AL,[ESP+0xC]` reads only the low byte of the original second stack slot.
- `RET 8` proves exactly two four-byte stack slots.
- Active callers use canonical `0` or `1` for the second argument. For example,
  `0x00701568..0x0070156E` pushes `1`, pushes mission `5`, calls vtable `+0x1E8`, then immediately
  loads `ECX`; it does not test `AL`.
- The Aircraft override at `0x0041BA90` forwards both padded stack slots and ends in `RET 8`; full
  dword forwarding is an x86 stack-slot fact, not a 32-bit logical type.
- The return is **void**, not bool. Guard exits can reach `POP ESI; RET 8` before the body loads
  the boolean argument into `AL`; other exits preserve results from `ReadyToCommence` or
  `Commence`. `AL` is therefore path-dependent and has no Queue return meaning.

**Result:** `void`, a 32-bit mission, and a logical one-byte `bool` passed in a four-byte stack
slot.

### `0x005B3650` — Override_Mission

**Accepted current parser prototype:**
`void __thiscall Override_Mission(int mission, void *target, void *destination)`.

- `ECX` is the receiver and `RET 0xC` proves three four-byte stack arguments.
- The base reads only the first argument, but the other two are real inherited-API parameters:
  - `TechnoClass::Override_Mission @ 0x007013A0` forwards all three to the base, then passes the
    second argument to vtable `+0x3C8` (`TechnoClass::Assign_Target @ 0x006FCDB0`).
  - `FootClass::Override_Mission @ 0x004D8F40` forwards all three through Techno, then passes the
    third argument to vtable `+0x480` with flag `1` (`Assign_Destination`).
  - `AircraftClass::Override_Mission @ 0x0041BB30` preserves the same three-argument contract and
    ends in `RET 0xC`.
- The target setter accepts both object and cell targets, reads the common vtable and abstract
  flags, and can substitute a `CellClass *`. The destination family likewise handles multiple
  abstract object kinds. `ObjectClass *`, `TechnoClass *`, or `CellClass *` would each be too
  narrow.
- An active caller at `0x00702B3A..0x00702B41` supplies `(mission=1, target=EAX,
  destination=null)` through vtable `+0x1F4` and does not consume a return.

**Result:** `void`; semantic arguments are `(Mission, AbstractClass *, AbstractClass *)`. Use
`void *` only as a temporary Ghidra parser type while `AbstractClass` is absent.

### `0x005B36B0` — Restore_Mission

**Accepted prototype:** `bool __thiscall Restore_Mission(void)`.

- Uses `ECX` as receiver, has no explicit arguments, and ends in plain `RET`.
- Success executes `MOV AL,1`; failure executes `XOR AL,AL`.
- `TechnoClass::Restore_Mission @ 0x007013E0` calls the base at `0x007013E3` and tests `AL` at
  `0x007013E8`.
- As with `Commence`, only `AL` is normalized; a four-byte `BOOL` would be wrong.

**Result:** one-byte C++ `bool`, no explicit arguments.

## Blocking datatype finding: current Mission enum is stale

The existing `/RA2/Mission` datatype is four bytes, so it would not break the stack ABI. Its
labels are nevertheless wrong for the active executable. Examples:

| Meaning | Current Ghidra enum | Live YR table |
|---|---:|---:|
| `QMove` | `4` | `3` |
| `Harvest` | `7` | `10` |
| `Unload` | `12` | `16` |
| `Selling` | `15` | `19` |
| `Spyplane Approach` | `25` | `30` |

The active name-pointer table at `0x00816CAC` gives this complete nonnegative domain:

| ID | Name | ID | Name |
|---:|---|---:|---|
| 0 | Sleep | 16 | Unload |
| 1 | Attack | 17 | Sabotage |
| 2 | Move | 18 | Construction |
| 3 | QMove | 19 | Selling |
| 4 | Retreat | 20 | Repair |
| 5 | Guard | 21 | Rescue |
| 6 | Sticky | 22 | Missile |
| 7 | Enter | 23 | Harmless |
| 8 | Capture | 24 | Open |
| 9 | Eaten | 25 | Patrol |
| 10 | Harvest | 26 | Paradrop Approach |
| 11 | Area Guard | 27 | Paradrop Overfly |
| 12 | Return | 28 | Wait |
| 13 | Stop | 29 | Attack Move |
| 14 | Ambush | 30 | Spyplane Approach |
| 15 | Hunt | 31 | Spyplane Overfly |

Mission storage also uses `-1` as the no-mission sentinel. A corrected enum should represent that
sentinel as well as IDs `0..31`.

This finding also affects the already-created `/MissionClass` structure: its three mission fields
have correct four-byte width and offsets, but Ghidra may currently display the wrong symbolic name
for their runtime values.

## External corroboration, not authority

The open Electronic Arts Red Alert `MISSION.H` declares the ancestral family as mission return,
void assignment, bool commence, void override with two generic targets, and bool restore. The
current community YRpp `MissionClass.h` corroborates `AbstractClass *` for the two override
pointers and `bool` for Queue's second argument.

YRpp currently declares Queue itself as returning bool. The live `gamemd.exe` body contradicts
that declaration: the return register is path-dependent and no checked caller consumes it. Binary
evidence therefore wins, and this report records Queue as `void`.

## Pre-application Ghidra state and applied annotation sequence

At the end of this pass:

- all seven functions still have `calling_convention: unknown`, `return_type: undefined`, and no
  declared parameters;
- `MissionClass` has zero detected member functions;
- the `/MissionClass` structure remains size `0xD4`;
- no analysis or metadata mutation was performed.

The later annotation pass followed this order:

1. Introduce a non-destructive, correctly valued YR mission enum; do not delete or silently
   recreate the stale type until all of its existing uses are inventoried.
2. Replace the three `/MissionClass` mission-field types only after readback proves the corrected
   enum.
3. Decide whether to create a canonical `AbstractClass` structure or temporarily use `void *` for
   the two Override pointers.
4. Apply each of the seven prototypes individually with `__thiscall`, immediately read back its
   signature, variables, and decompilation, and stop on the first disagreement.
5. Only then associate the implicit receiver with `MissionClass`. That operation moves functions
   into the class namespace, so the namespace/name side effect must be intentional and verified.
6. Save only after all seven independent readbacks pass.

## Rust-facing handoff

No Rust change follows from this prototype-only slice. The current Rust mission layer already
models queued/current mission state, explicit commencement, override/restore, and polymorphic
navigation targets. The useful output here is better binary tooling metadata and the discovery
that the existing Ghidra enum must not be treated as Yuri's Revenge authority.

Relevant current Rust locations:

- `src/sim/mission/verb.rs`
- `src/sim/mission/authority.rs`
- `src/sim/components.rs` (`NavTargetRef`, documented as native `AbstractClass *`)

## Coverage and open-question log

### Resolved

- `[RESOLVED] OQ-01` — MissionClass primary RTTI and vtable identity.
- `[RESOLVED] OQ-02` — Assign receiver, one argument, cleanup, and void return.
- `[RESOLVED] OQ-03` — GetCurrent receiver, no arguments, and 32-bit mission return.
- `[RESOLVED] OQ-04` — Dispatch receiver, no arguments, and void return.
- `[RESOLVED] OQ-05` — Commence receiver, no arguments, and one-byte bool return.
- `[RESOLVED] OQ-06` — Queue has two four-byte stack slots and a logical bool second argument.
- `[RESOLVED] OQ-07` — Queue return is void, not bool.
- `[RESOLVED] OQ-08` — Override has three real arguments and `RET 0xC`.
- `[RESOLVED] OQ-09` — Override's second and third arguments share the semantic
  `AbstractClass *` type.
- `[RESOLVED] OQ-10` — Restore has no arguments and returns one-byte bool.
- `[RESOLVED] OQ-11` — bool returns are valid only in `AL`, not normalized four-byte `BOOL`.
- `[RESOLVED] OQ-12` — current Ghidra metadata has no implicit typed receiver or declared
  parameters for these seven functions.
- `[RESOLVED] OQ-13` — existing `/RA2/Mission` width is compatible but its values are stale.
- `[RESOLVED] OQ-14` — complete live mission-name domain is IDs `0..31`, plus storage sentinel
  `-1`.
- `[RESOLVED] OQ-15` — all seven safe interim prototype strings pass Ghidra validation.
- `[RESOLVED] OQ-19` — all seven receivers are associated with `MissionClass *`; the namespace
  move was accepted and the existing leaf names were intentionally preserved.
- `[RESOLVED] OQ-21` — post-save enum, structure, member, prototype, receiver, and storage readback
  passed with zero mismatches.

### Deferred outside this slice

- `[DEFERRED] OQ-16` — exact non-destructive migration plan for every existing user of the stale
  `/RA2/Mission` enum.
- `[DEFERRED] OQ-17` — canonical `AbstractClass` datatype construction and whether existing
  `ObjectClass` should be remodeled to embed it.
- `[DEFERRED] OQ-18` — prototype synchronization for Techno, Foot, Aircraft, and other derived
  overrides.
- `[DEFERRED] OQ-20` — MissionClass fields outside the seven-function ABI slice, including the
  unresolved semantic role at `+0xCC`.

## Evidence anchors

Primary body reads: `0x005B2FD0`, `0x005B3040`, `0x005B3060`, `0x005B3570`,
`0x005B35E0`, `0x005B3650`, `0x005B36B0`.  
Derived override reads: `0x007013A0`, `0x004D8F40`, `0x0041BB30`, `0x007013E0`.  
Setter evidence: `0x006FCDB0`, vtable `+0x3C8`; `0x00741970`, vtable `+0x480`.  
Active caller reads: `0x0073A324`, `0x006FA655`, `0x0043FE43`, `0x007017BA`,
`0x0070156E`, `0x00702B41`, `0x007013E3`.  
RTTI/vtable reads: `0x007EDCBC`, `0x00805D28`, `0x00817B18`, `0x007EDD1C`,
`0x007EDE44`, `0x007EDEA8`.  
Mission-name table: `0x00816CAC` with 32 pointer entries.

## Tier 1 corridor outcome — derived-class lifecycle slots (applied 2026-08-17, Claude Code session)

Scope: slots +0x05C (AI/dispatch), +0x184 (GetCurrentMission), +0x1E8 (Queue),
+0x1EC (Commence), +0x1F0 (Assign), +0x1F4 (Override), +0x1F8 (Restore) across
RadioClass 0x007F0508, TechnoClass 0x007F4960, FootClass 0x007E8C94,
UnitClass 0x007F5C70, AircraftClass 0x007E22A4, BuildingClass 0x007E3EBC.
Slot semantics proven from MissionClass base vtable 0x007EDCC0 (all seven slots
read back to the seven base functions). Snapshot before mutations:
<local>/Documents/ghidra-backups/2026-08-17-tier1 (15 files, 162,144,265 bytes, verified).

42-row disposition:
- 24 INHERITED base (Radio all 7; Techno/Foot/Unit/Building +0x184,+0x1E8,+0x1EC,+0x1F0
  minus overrides below; Aircraft +0x184).
- 5 INHERITED cross-class override (Unit +0x1F4/+0x1F8 = Foot bodies; Building
  +0x1F4/+0x1F8 = Techno bodies; Aircraft +0x1F8 = Foot body).
- 3 VERIFIED already fully typed (0041ba90 Aircraft Queue, 0041b9f0 Aircraft Assign,
  007013a0 Techno Override — 2026-08-04/Codex work, untouched).
- 10 APPLIED this tier, each save_program + readback verified:
  006f9e50 TechnoClass__AI_Update, 004da530 FootClass__AI, 007360c0 UnitClass__AI,
  00414bb0 AircraftClass__AI → void __thiscall(MissionClass* this);
  0043fb20 BuildingClass__Update → void __thiscall(BuildingClass* this) (kept
  pre-existing receiver, normalized return);
  0041bb30 AircraftClass__Override_Mission, 004d8f40 FootClass__Override_Mission
  → void __thiscall(MissionClass*, YR_Mission, void* target, void* destination);
  007013e0 TechnoClass__Restore_Mission, 004d8f80 FootClass__Restore_Mission
  → bool __thiscall(MissionClass*);
  0041b870 (see mislabel below) → bool __thiscall(MissionClass*).
ABI evidence: same-slot substitutability against the Codex-proven base prototypes
(a vtable caller's stack discipline forces every override at that slot).

Residuals / flags:
- WRONG LABEL (FIXED 2026-08-17, user-authorized): 0041b870 was named
  AircraftClass__Override_Mission but sits at +0x1EC (Commence) and chains to
  MissionClass__Commence, colliding with the real override at 0041bb30. Renamed
  to AircraftClass__Commence; saved and read back.
- Offsets beyond MissionClass 0xD4 render as this[N].field artifacts (e.g.
  this[6].SuspendedMission = +0x5A8 SuspendedNavCom). Expected embedded-base
  residual; fix arrives with FootClass-layer typing, not by growing MissionClass.
- TOOL DEFECT: set_function_prototype resets a previously-applied this-type to
  void* (observed on 0043fb20); always re-apply set_function_this_type and read
  back after any prototype edit.
- Class association prepends MissionClass:: to names already carrying class
  prefixes (MissionClass::FootClass__AI). Cosmetic; renames out of scope.
- AI-slot naming is inconsistent across classes (AI vs AI_Update vs Update);
  left as-is.
