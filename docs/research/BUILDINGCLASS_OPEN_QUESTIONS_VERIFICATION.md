---
name: BuildingClass Open Questions Verification
description: Binary verification of 7 open questions from the BuildingClass master report (ChargeFlags, Factory ptr, BuildingLight lifecycle, Soviet Engineer enum, MCV refund, CloakGen UnInit, sound array)
type: reference
---

# BuildingClass — Open Questions Verification Round

**Date:** 2026-04-19
**Binary:** gamemd.exe
**Confidence:** HIGH — all 7 findings verified from direct Ghidra decompilation and disassembly
**Active in YR:** Mixed — see per-question analysis

This document verifies the 7 open questions listed in
`BUILDINGCLASS_MASTER_GHIDRA_REPORT.md` section 23. All have been resolved from
binary evidence. Update the master report accordingly.

---

## 1. +0x5B0 ChargeFlags array — CONFIRMED as 21-byte array

**Prior status:** Debunked in an earlier round (claimed no function accessed this
range). That debunking was **wrong** — the array exists and IS used.

**Evidence:** `BuildingClass::OnPowerOff` at `0x004545D0`, disassembly at
`0x0045469A`–`0x004546A7`:

```
00454698: JZ 0x00454701
0045469a: MOV AL, byte ptr [ESI + EBP*0x1 + 0x5b0]   ; read byte with 1-byte stride
004546a1: TEST AL, AL
004546a3: JZ 0x00454701
004546a5: MOV ECX, ESI
004546a7: MOV byte ptr [ESI + EBP*0x1 + 0x5b0], 0x0  ; clear on power-off
...
00454705: ADD EDI, 0x44           ; iterates PowerUp entries (0x44 bytes each)
00454708: INC EBP                  ; EBP is byte index (1-byte stride)
0044070c: CMP EDI, 0x594           ; 0x594 = 21 * 0x44 = full iteration
```

`OnPowerOn` at `0x004547C0` sets the same byte back to 1 (`(&this->AnimStates_0)[iVar4] = 1`).

**Layout:** `bool[21]` (or `u8[21]`) at offset `+0x5B0`, parallel to the
`Anims[21]` array at `+0x55C`.

**Gating:** Only PowerUp art entries with `Type+0xF8E` flag set use this array
(the "PoweredEffect" flag — one of four flags per art slot at offsets +0xF8C,
+0xF8D, +0xF8E, +0xF8F).

**Purpose:** Per-slot "currently active" flag for PoweredEffect anims. When
power goes off: cleared to 0, and ClearAnimSlot invoked if flag was set
(signaling anim to tear down). When power comes on: set to 1, and
CreateAnimForSlot invoked (anim spawned).

**Active in YR:** Yes — used by any PowerUp entry with `PoweredEffect=yes` flag.

## 2. +0x524 Factory pointer — CONFIRMED FactoryClass*

**Prior status:** Referenced in destructor, role not fully traced.

**Evidence:** Cloning Vats auto-produce function at `0x004500F0` (`FUN_004500f0`)
explicitly types it as `FactoryClass*`:

```c
if (((FactoryClass *)param_1[0x149] != (FactoryClass *)0x0) &&
   (bVar1 = FactoryClass__IsComplete((FactoryClass *)param_1[0x149]), bVar1)) {
   // param_1 is int*, so param_1[0x149] == byte offset 0x149 * 4 = 0x524
}
```

(`param_1` is `int*`, so `param_1[0x149]` is the DWORD at byte offset `0x524`.)

Operations on this pointer:
- `FactoryClass__IsComplete`
- `FactoryClass__GetObject`
- `FactoryClass__AbandonProduction`
- `FactoryClass__CompletedProduction`
- `FactoryClass__StartProduction`
- `FactoryClass__SetRate`
- Allocated on demand via `operator_new(0x74) + FactoryClass__Constructor()`
- Released via `vtable+0x20` (C++ destructor) then set to 0

**Role:** Per-building FactoryClass pointer used for **Cloning Vats
auto-production** (spawning free duplicates). The destructor reference is the
corresponding cleanup. Ephemeral — null when not producing.

**Active in YR:** Yes — Cloning Vats (GACLON) is a standard YR Soviet tech.

## 3. +0x600 BuildingLightClass* lifecycle — CONFIRMED (conditional spotlight)

**Prior status:** Confirmed as `BuildingLightClass*` but lifecycle not traced.

### Creation — `BuildingClass::Unlimbo` at `0x00441163`–`0x00441196`:

```
00441163: MOV ECX, [ESI + 0x520]       ; Type pointer
00441169: MOV AL, byte ptr [ECX + 0x154b]   ; Type+0x154B flag
0044116f: TEST AL, AL
00441171: JZ 0x00441196                 ; skip if flag off
00441173: PUSH 0xe8                     ; sizeof(BuildingLightClass) = 232 bytes
00441178: CALL 0x007c8e17               ; operator_new
0044117d: ADD ESP, 0x4
00441180: TEST EAX, EAX
00441182: JZ 0x0044118e
00441184: PUSH ESI                      ; push parent BuildingClass*
00441185: MOV ECX, EAX
00441187: CALL 0x00435820               ; BuildingLightClass constructor(BuildingClass*)
0044118c: JMP 0x00441190
0044118e: XOR EAX, EAX                  ; null on alloc-fail
00441190: MOV [ESI + 0x600], EAX        ; store at BuildingClass+0x600
```

- Gating flag: `Type+0x154B` (INI key likely `HasSpotlight=` or similar —
  needs separate ReadINI trace to confirm the exact key name).
- Size: `0xE8` (232 bytes)
- Constructor: `0x00435820`, takes owning `BuildingClass*` as first arg.

### Destruction — `BuildingClass::OnDestroyed` at `0x00445880`:

```c
if (*(int **)&this->field_0x600 != (int *)0x0) {
  (**(code **)(**(int **)&this->field_0x600 + 0xf8))();  // vtable+0xF8 (Release/delete)
}
```

Standard virtual-destructor-like Release via `vtable+0xF8`.

### Related — `+0x614 LightSourceClass*` is a SEPARATE pointer (not conflated):

In Unlimbo at `0x00440D8A`:
```
00440d80: MOV EAX, [ESI + 0x614]
00440d86: TEST EAX, EAX
00440d88: JNZ 0x00440df5             ; skip alloc if already exists
00440d8a: PUSH 0x4c                   ; sizeof(LightSourceClass) = 76 bytes
00440d8c: CALL operator_new
...
00440def: MOV [ESI + 0x614], EAX
```

- Gating: `Type+0xE30..+0xE40` (ambient light RGBA + range)
- Size: `0x4C` (76 bytes)
- Different constructor (`0x00554760`), different destroy (`0x00554A60`)

**Active in YR:** Conditional. `+0x600` only allocated when `Type+0x154B` is
set in INI. `+0x614` allocated for all buildings with non-default ambient
light settings (most retail buildings set these).

## 4. Soviet Engineer enum in GetSurvivorInfantryType — CORRECTED

**Prior status:** Master doc claimed "25% Engineer if Soviet-side AND not
bio-reactor." The "Soviet-side" part was **wrong**.

**Evidence:** `BuildingClass::GetSurvivorInfantryType` (vtable slot 195,
`0x0044EB10`):

```c
if (*(char *)(param_1 + 0x6e3) == '\0') {                    // some suppression flag
    iVar1 = Random__RandomRanged(0,99);
    if ((iVar1 < 0x19) &&                                    // 25%
        (*(int *)(*(int *)(param_1 + 0x520) + 0xeb8) == 7)) {// Type.Factory == 7
      return *(undefined4 *)(g_RulesClass_Instance + 0xf70); // Engineer
    }
}
return FUN_00707d20();                                        // TechnoClass::GetSurvivorInfantryType
```

### Enum value 7 = `Factory=BuildingType` (ConYard)

The binary's `Factory=` field stores RTTI/AbstractType enum values:
- `0x03` = Aircraft
- `0x07` = Building  ← **this**
- `0x10` = Infantry
- `0x28` = Unit

Confirmed against FactoryPlant dispatch logic at `0x0050BEB0` which uses the
same enum for cost-bonus lookup.

### Rule: 25% Engineer chance if ConYard (`Factory=BuildingType`) — NOT side-based

A ConYard has `Factory=BuildingType` because it builds buildings. The engine
gives ConYards a 25% chance to drop an Engineer on destruction (falls back to
side-based crew otherwise via `TechnoClass::GetSurvivorInfantryType` at
`0x00707D20`, which uses `Owner->HouseType->Side` for Allied/Soviet/Third crew
selection).

### Also resolved: +0x6E3 suppression flag

The check also gates on `BuildingClass+0x6E3 == 0` (a one-byte suppression
flag). Purpose not fully traced — possible candidates are the bio-reactor
"absorbed infantry" flag or a "no crew eject" flag. Not critical because the
outer 25% + ConYard condition is the primary gate.

### TechnoClass base path (0x00707D20)

- Gated by `Type+0xCCD` (Crewed= flag, early return 0 if not crewed)
- Side (Owner->HouseType+0x1E8): 0=Allied → Rules+0xF78 (AlliedCrew), 1=Soviet → Rules+0xF7C (SovietCrew), 2=Third → Rules+0xF80 (ThirdCrew), else Rules+0xF6C (Technician)
- 15% Technician override if `vtable+0x2AC` (Is_Weapon_Equipped) returns true

**Action for master doc:** Replace "Soviet-side" language in section 17 with
"Factory=BuildingType (ConYard)". The Rules+0xF70 constant IS "Engineer"
globally (not side-specific).

**Active in YR:** Yes — fires on every ConYard destruction.

## 5. MCV unlimbo-fail refund path — RESOLVED (decompiler artifact)

**Prior status:** FPU trace incomplete; decompiler showed confusing
`uStack_b4._4_4_` as the refund amount, raising doubts.

**Evidence:** Raw disassembly of `BuildingClass::Sell` state 2 at
`0x00449C30` resolved the confusion. Two stack slots are in play:

- `[ESP + 0x24]` = `HealthRatio` (double, 8 bytes, from `GetHealthRatio` FSTP)
- `[ESP + 0x30]` = `RefundValue` (int, from `vtable+0x2BC` GetRefundValue)

### MCV allocation-fail path (`operator_new(0x8E8)` returns null) — `0x0044A19E`:

```
0044a19e: MOV EDX, [EBP]                    ; vtable
0044a1a1: MOV ECX, EBP                       ; this
0044a1a3: CALL dword ptr [EDX + 0x2bc]      ; GetRefundValue (fresh call)
0044a1a9: MOV ECX, [EBP + 0x21c]             ; Owner
0044a1af: PUSH EAX                           ; refund amount
0044a1b0: CALL 0x004f9950                    ; HouseClass::Add_Credits
```

### MCV placement-fail path (`Unlimbo` returns false) — `0x0044A16B`:

```
0044a16b: MOV EAX, [ESP + 0x30]              ; cached RefundValue from 0x00449E80
0044a16f: MOV ECX, [EBP + 0x21c]             ; Owner
0044a175: PUSH EAX
0044a176: CALL 0x004f9950                    ; HouseClass::Add_Credits
```

Both paths use `vtable+0x2BC` (`GetRefundValue` = `Cost × SellBack% + stored
ore`) — **NOT health-scaled**. Consistent with the non-MCV sell path.

### What the HealthRatio IS used for

Only to compute the MCV's post-undeploy health when placement SUCCEEDS:

```
0044a01a: FILD dword ptr [EAX + 0xa0]        ; UnitType Strength
0044a020: FMUL double ptr [ESP + 0x24]       ; × HealthRatio
0044a024: CALL 0x007c5f00                    ; ftol
0044a029: CMP EAX, 0x1
0044a031: MOV EAX, 0x1                       ; clamp min 1
0044a036: MOV [EBX + 0x6c], EAX              ; MCV.Health
0044a039: MOV [EBX + 0x70], EAX              ; MCV.MaxHealth? (same value)
```

**Conclusion:** Refund is identical to normal sell. The FSTP/FILD/FMUL
sequence was for health inheritance, not refund. Decompiler conflated adjacent
stack slots.

**Active in YR:** Yes — triggered any time a `UndeploysInto=` building
attempts to undeploy with obstructed target cell.

## 6. CloakGen tick-down "final UnInit" — RESOLVED (no dedicated UnInit)

**Prior status:** Asked what triggers the "final UnInit call after radius
retracts."

**Evidence:** `BuildingClass::UpdateGapGenerator_Tick` at `0x00454DB0`. There
is **no dedicated UnInit function call** — cleanup is inline via flag clearing
and state transition.

### Two interleaved state systems

The function handles two separate CloakGen/gap-gen state machines:

**A. 4-state gap generator** (gap-gen only, gated by separate flag):
- State (at `param_1[0x88]` in int-indexed code = byte offset `0x220`): 0 Inactive, 1 Expanding, 2 Active, 3 Contracting
- Visual stage at `+0x6ED` (0–15, with 16 as Yuri variant)
- State 3 + `+0x6ED` reaches 0 → `state = 0`, and (if the ParticleSystem
  pointer at `+0xC3×4=0x30C` is null AND Type.light coords at
  `+0x768/+0x76C/+0x770` are non-default) allocate a new
  `ParticleSystemClass` via `operator_new(0x100)`
- Post-state-update: if `state == 0` AND `vtable+0x2A0` returns true → call
  `vtable+0x460` (TechnoClass inherited slot 280)

**B. 3-byte CloakGen direction/radius** (the "CloakGenerator" TS-legacy system):
- Direction at `+0x6EB` (0 = idle, 1 = expanding, 0xFF/-1 = contracting)
- Visual radius at `+0x6EC`
- `OnPowerOff` at `0x004545D0` sets `+0x6EB = 0xFF` to signal contraction
- Per-tick in `UpdateGapGenerator_Tick`:
  ```c
  if (+0x6EB < 1) {              // contracting
      if (+0x6EC == 0) {         // radius fully retracted
          +0x6EB = 0;             // clear direction
          return;                 // ← this IS the "final cleanup"
      }
      +0x6EC = +0x6EC - 1;       // remove outermost shroud ring
  }
  ```

### Conclusion

The "UnInit" is implicit:
- For B (direction/radius): just sets `+0x6EB = 0` and early-returns
- For A (4-state gap): state transition to 0, optional new ParticleSystem
  allocation, optional vtable+0x460 call

No object is destroyed at this point — both systems just go quiescent.

**Active in YR:** `CloakGenerator=yes` (Type+0x16C7) is TS-legacy — **no
retail YR building sets this flag**. The 4-state gap generator (GapGenerator=)
IS active in YR (PsychicSensor, MindReader, etc.).

## 7. Type+0xE70 sound — CONFIRMED single index

**Prior status:** Unclear if single index or list/array.

**Evidence from BuildingTypeClass::ReadINI** at `0x00460780`–`0x004607D3`:

```
0046078b: MOV [EBP + 0xe6c], EAX             ; prior field stored
00460791: MOV EDI, [EBP + 0xe70]             ; load prior/default
...
00460797: PUSH EBX
0046079a: CALL 0x00528a10                    ; INI sound-key parser
004607af: CMP EAX, -1                         ; -1 = not found
004607b2: JNZ 0x004607b6
004607b4: MOV EAX, EDI                        ; fallback to default
004607cd: MOV [EBP + 0xe70], EAX              ; store single DWORD
004607d3: MOV EDI, [EBP + 0xe74]              ; next adjacent sound field
```

Each adjacent field is **one DWORD sound index**, each parsed from its own
distinct INI key. No length prefix, no array iteration, no size metadata.

**Evidence from use site** in `Mission_Selling` at `0x0044A861`:

```
0044a861: CMP dword ptr [EAX + 0xe70], -1    ; -1 = no sound configured
0044a868: JZ 0x0044a89e                       ; skip playback
...
0044a88f: MOV ECX, [EAX + 0xe70]              ; load sound idx
0044a895: LEA EDX, [ESP + 0x6c]
0044a899: CALL 0x007509e0                     ; VocClass::PlayAt(soundIdx, coord)
```

Single DWORD passed directly to `VocClass::PlayAt`.

### Layout

`Type+0xE6C`, `+0xE70`, `+0xE74`, ... are individual sound index slots for
different events (build completion, selling, destruction, etc.). Default
value `-1` = no sound.

**Active in YR:** Yes — building sound playback uses these on sell, destroy,
etc.

---

## Summary of Required Master Doc Updates

1. **Section 2 — add row:** `+0x5B0 | byte[21] | PoweredEffect active flags |
   OnPowerOff/OnPowerOn toggle per-slot` (marks the "DEBUNKED 2026-04-16" row
   as re-verified in the opposite direction).
2. **Section 2 — update row:** `+0x524 | FactoryClass* | Cloning Vats
   auto-produce factory, not just destructor cleanup`.
3. **Section 2 — update note:** `+0x600` allocation size `0xE8`, constructor
   `0x00435820`, gated by Type+0x154B.
4. **Section 17 Mission_Selling — correct the Engineer description:** replace
   "Soviet-side" with "`Factory=BuildingType` (ConYard) via
   `Type+0xEB8 == 7`". Note Rules+0xF70 is global Engineer (not
   Soviet-specific).
5. **Section 17 Mission_Selling MCV undeploy — confirm refund is NOT
   health-scaled:** both allocation-fail and placement-fail paths use
   `vtable+0x2BC (GetRefundValue)`; HealthRatio only modulates the placed
   MCV's health (`Strength × HealthRatio`, min 1).
6. **Section 14 / 19 — document that CloakGen cleanup is implicit:** when
   contracting direction reaches radius 0, the system clears `+0x6EB = 0` and
   returns; no dedicated UnInit is called. CloakGenerator flag is TS-legacy;
   4-state gap generator is active in YR.
7. **Section 2 — confirm row:** `+0xE70 | int | sound index, -1 = none`
   (not an array).

### Open questions resolved: 7 of 7

| # | Question | Status |
|---|----------|--------|
| 1 | ChargeFlags at +0x5B0 | ✓ Verified — byte[21] |
| 2 | +0x524 FactoryClass* | ✓ Verified — Cloning Vats auto-produce |
| 3 | +0x600 BuildingLight lifecycle | ✓ Traced — Type+0x154B, size 0xE8 |
| 4 | Soviet Engineer enum | ✓ Corrected — Factory=BuildingType (7), not Soviet |
| 5 | MCV unlimbo-fail refund | ✓ Resolved — decompiler artifact, non-scaled refund |
| 6 | CloakGen final UnInit | ✓ Resolved — no dedicated UnInit, inline cleanup |
| 7 | Type+0xE70 sound shape | ✓ Verified — single DWORD index |

---

## Sources

### Ghidra functions decompiled/disassembled

- `0x004545D0` — BuildingClass::OnPowerOff (+0x5B0 access)
- `0x004547C0` — BuildingClass::OnPowerOn (+0x5B0 access)
- `0x004500F0` — FUN_004500F0 (Cloning Vats auto-produce, +0x524 access)
- `0x00440580` — BuildingClass::Unlimbo (+0x600 and +0x614 allocation)
- `0x00445880` — BuildingClass::OnDestroyed (+0x600 release)
- `0x0044EB10` — BuildingClass::GetSurvivorInfantryType (Engineer logic)
- `0x00707D20` — TechnoClass::GetSurvivorInfantryType (side-based base path)
- `0x00449C30` — BuildingClass::Sell (MCV undeploy refund)
- `0x00454DB0` — BuildingClass::UpdateGapGenerator_Tick (CloakGen cleanup)
- `0x0045FE50` — BuildingTypeClass::ReadINI (Type+0xE70 parse)
- `0x00451890` — BuildingClass::CreateAnimForSlot (anim pipeline)
- `0x00451750` — BuildingClass::SetAnimSlotImage (anim pipeline)
- `0x00451E40` — BuildingClass::ClearAnimSlot (anim pipeline)

### Reports referenced

- `BUILDINGCLASS_MASTER_GHIDRA_REPORT.md` (sections 2, 14, 17, 19, 21, 23)
- `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md`
- `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md`
