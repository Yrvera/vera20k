# STR_INIKey_Warpable — 0x00843778

**Proposed Ghidra label:** STR_INIKey_Warpable
**Kind:** string / INI-key
**Task:** decode-string-warpable
**Active in YR:** Yes — read by all four TechnoType ReadINI functions unconditionally

---

## Summary

The `Warpable` per-unit flag in TechnoTypeClass INI sections. A boolean read by
`TechnoTypeClass__ReadINI` (and all four type-specific subclass readers) that determines
whether a unit can be targeted by the ChronoSphere. Stored as a byte at
`TechnoTypeClass+0xD3A`. Not consumed by `TeleportLocomotionClass` itself — governs
ChronoSphere weapon targeting eligibility, which gates the `vtable+0x160` warpable check
called in PostWarpValidation Phase 1.

Verified via `inspect_memory_content 0x00843778` (string confirmed) and
`get_assembly_context 0x00714F65` (struct offset extracted from TechnoTypeClass__ReadINI).

---

## Active in YR

**Yes — unconditionally read** from each unit section during type loading.
All 4 ReadINI callers confirmed via `get_function_callers 0x00714F65`:
- `AircraftTypeClass__ReadINI @ 0x0041CC20`
- `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`
- `InfantryTypeClass__ReadINI @ 0x005240A0`
- `UnitTypeClass__ReadINI @ 0x00747620`

---

## String verification

`inspect_memory_content 0x00843778` (12 bytes):
- Hex: `57 61 72 70 61 62 6C 65 00 00 00 00`
- Detected string: "Warpable" (null at byte 8, char[9])

---

## Struct offset

From `get_assembly_context 0x00714F65` (xref site in `TechnoTypeClass__ReadINI`):

```asm
00714f65: PUSH 0x843778           ; push "Warpable" INI key string
00714f6a: MOV ECX,EBX
00714f6c: CALL 0x00524ec0         ; get INI section
00714f71: PUSH EAX
00714f72: MOV ECX,EDI
00714f74: CALL 0x005295f0         ; ReadBool("Warpable")
00714f79: MOV byte ptr [EBP + 0xd3a],AL   ; store result
```

**TechnoTypeClass+0xD3A** (byte) = Warpable flag.
`EBP` = TechnoTypeClass `this` pointer in ReadINI convention; 0xD3A is a direct byte
offset (instruction uses `byte ptr` explicitly).

---

## INI values

From `ini/rulesmd.ini` — no global `[General]` default. Per-unit value. Most units
do not explicitly set Warpable (field defaults to true for most types in YR, consistent
with standard ChronoSphere gameplay where most units can be targeted).

---

## Proposed Ghidra label

| Symbol | Address | Proposed name |
|---|---|---|
| 0x00843778 | string | STR_INIKey_Warpable |

---

## Out-of-scope refs

- ChronoSphere weapon system — primary consumer of Warpable via vtable+0x160 warpable
  check; not in teleport locomotor scope
- `ReadBool` at 0x005295F0 — general INI reader; not teleport-specific
- `TechnoTypeClass__ReadINI` — general type loading; not teleport-specific

---

## Unverified / YELLOW

- **TechnoTypeClass+0xD39 adjacent field**: Assembly immediately before the Warpable read
  stores a different bool at TechnoTypeClass+0xD39. That key's identity is out of scope.
  YELLOW.
- **Default value**: "True for most units" based on in-game observation. The exact
  ReadBool default argument is not confirmed from this assembly context. YELLOW.
- **vtable+0x160 → TechnoTypeClass+0xD3A chain**: PostWarpValidation calls vtable+0x160
  per occupant as the warpable gate. That vtable method likely reads TechnoTypeClass+0xD3A.
  The chain is inferred; not re-decompiled in this session. YELLOW.
