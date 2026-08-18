# BuildingClass Missions, INI Keys, and Vtable — Verification Report

**Binary:** `gamemd.exe` (Yuri's Revenge 1.001)
**Date:** 2026-04-19
**Method:** Live Ghidra MCP decompilation of ReadINI, Mission_Dispatch, and BuildingClass vtable memory.
**Confidence:** High — all findings verified by reading raw vtable bytes at `0x007E3EBC`, decompiling `BuildingTypeClass::ReadINI` at `0x0045FE50`, and cross-referencing `MissionClass::Mission_Dispatch` at `0x005B3060`.

---

## Summary

| Area | Result |
|------|--------|
| Mission handler addresses | **11 / 11 correct** |
| Mission handler size claims | 7 correct, 4 inaccurate (see detail) |
| INI key → offset mappings | **31 / 31 correct** |
| BuildingClass vtable base | **Correct** (`0x007E3EBC`) |
| Vtable slot 23/53/54/64/91 | **5 / 5 correct** |

No address corrections needed. Four size claims need correction (Guard, Return, Guard, Construction). See "Mission handler size corrections" below.

---

## Part 1: Mission handler verification

### How dispatch works

`MissionClass::Mission_Dispatch` at `0x005B3060` is a big switch on `param_1[0x2B]` (`CurrentMission`). Each case does an indirect call through the object's vtable at a fixed offset:

```c
case 1:  iVar2 = (**(code **)(*param_1 + 0x210))();  // Attack
case 5:  iVar2 = (**(code **)(*param_1 + 0x21c))();  // Retreat
case 6:  iVar2 = (**(code **)(*param_1 + 0x21c))();  // Sleep -> same as Retreat
case 7:  iVar2 = (**(code **)(*param_1 + 0x240))();  // Enter
case 8:  iVar2 = (**(code **)(*param_1 + 0x214))();  // Guard
case 10: iVar2 = (**(code **)(*param_1 + 0x224))();  // Return
case 11: iVar2 = (**(code **)(*param_1 + 0x220))();  // Stop
case 16: iVar2 = (**(code **)(*param_1 + 0x23c))();  // Eaten/Rescue
case 17: iVar2 = (**(code **)(*param_1 + 0x214))();  // Harvest -> same slot as Guard
case 18: iVar2 = (**(code **)(*param_1 + 0x244))();  // Construction
case 19: iVar2 = (**(code **)(*param_1 + 0x248))();  // Selling
case 20: iVar2 = (**(code **)(*param_1 + 0x24c))();  // RepairAndProduce
case 22: iVar2 = (**(code **)(*param_1 + 0x250))();  // Missile
case 24: iVar2 = (**(code **)(*param_1 + 0x254))();  // Unload
```

These vtable offsets are into the BuildingClass primary vtable at `0x007E3EBC`.

### Vtable slots 132-149 (the building mission handler block)

Reading raw memory at `0x007E40CC` (= `0x007E3EBC + 0x210`) gives:

| Slot | Vtable offset | Mission# | Address | Ghidra name | Body size (bytes) |
|------|---------------|----------|---------|-------------|-------------------|
| 132 | +0x210 | 1 Attack | `0x0044ACF0` | `BuildingClass__Mission_Attack` | 1175 |
| 133 | +0x214 | 8 Guard, 17 Harvest | `0x0044B760` | thunk → `0x005B2E50` | 5 (thunk) + 6 (target) |
| 134 | +0x218 | (unused in dispatch) | `0x005B2E60` | — | — |
| 135 | +0x21C | 5 Retreat, 6 Sleep | `0x004496B0` | `FUN_004496b0` (Retreat) | 903 |
| 136 | +0x220 | 11 Stop | `0x00449A40` | `FUN_00449a40` (Stop) | 8 |
| 137 | +0x224 | 10 Return | `0x0044B770` | thunk → `0x005B2E90` | 5 (thunk) + 6 (target) |
| 143 | +0x23C | 16 Eaten/Rescue | `0x0044D880` | `FUN_0044d880` | 640 |
| 145 | +0x244 | 18 Construction | `0x00449A50` | `FUN_00449a50` (Construction) | 355 |
| 146 | +0x248 | 19 Selling | `0x00449C30` | `BuildingClass__Sell` | 3991 |
| 147 | +0x24C | 20 RepairAndProduce | `0x0044B780` | `BuildingClass__MissionRepairAndProduce` | 4605 |
| 148 | +0x250 | 22 Missile | `0x0044C980` | `BuildingClass__Mission_Missile` | 3105 |
| 149 | +0x254 | 24 Unload | `0x0044E440` | `FUN_0044e440` | 853 |

All 11 claimed handler addresses are **correct**.

### Mission handler size corrections

The docs' claimed sizes are accurate for the full handlers but wrong for the thunk-based ones:

| Mission | Claimed size | Actual size | Notes |
|---------|--------------|-------------|-------|
| Attack | ~1174 B | 1175 B | Matches (off by 1, rounding) |
| Retreat | ~902 B | 903 B | Matches |
| Construction | ~434 B | **355 B** | **Claim inaccurate** — actual body is `0x00449A50 – 0x00449BB2`. Ghidra confirms `C3` ret + `90` padding at 0x00449BB2, new function begins at 0x00449BC0. |
| Selling | ~3989 B | 3991 B | Matches |
| RepairAndProduce | ~4604 B | 4605 B | Matches |
| Missile | ~3104 B | 3105 B | Matches |
| Guard | ~26 B | **5 B thunk + 6 B target** | **Claim inaccurate** — `0x0044B760` is a 5-byte `JMP` thunk to `0x005B2E50` (which just returns `0x1C2`, a 450-frame delay). The "Guard" mission is effectively a no-op stub for buildings. |
| Return | ~16 B | **5 B thunk + 6 B target** | **Claim inaccurate** — `0x0044B770` is a 5-byte `JMP` thunk to `0x005B2E90` (returns `0x1C2`, no-op). |
| Stop | ~8 B | 8 B | Matches. `mov eax,[ecx]; jmp dword ptr [eax+0x21C]` — tail-calls Retreat via vtable. |
| Eaten | (no claim) | 640 B | New data. |
| Unload | (no claim) | 853 B | New data. |

### Shared-slot missions (important for parity)

Two dispatch cases point at the same slot as another mission. These are **intentional aliases** in the dispatch table, not duplicates:

- **case 6 Sleep** → `+0x21C` → same as **case 5 Retreat**. Buildings treat Sleep as Retreat.
- **case 17 Harvest** → `+0x214` → same as **case 8 Guard**. Buildings treat Harvest as Guard (both are the no-op stub returning `0x1C2`).

### Thunk notes (Guard / Return)

Both `0x0044B760` and `0x0044B770` are 5-byte PLT-style thunks:

```
0x0044B760:  E9 EB 76 16 00       jmp 0x005B2E50   ; Guard/Harvest thunk
0x0044B770:  E9 1B 77 16 00       jmp 0x005B2E90   ; Return thunk

0x005B2E50:  B8 C2 01 00 00 C3    mov eax, 0x1C2 ; ret        (no-op, 450 frame delay)
0x005B2E90:  B8 C2 01 00 00 C3    mov eax, 0x1C2 ; ret        (no-op, 450 frame delay)
```

These are effectively `MissionClass`'s default handlers — BuildingClass doesn't override them. In a Rust implementation we can represent Guard and Return on buildings as simply "no-op, re-check in 450 frames."

---

## Part 2: INI key → struct offset verification

Decompiled `BuildingTypeClass::ReadINI` at `0x0045FE50` (labeled `BuildingTypeClass_ReadINI_Water` in Ghidra — actual function covers both water-bound check and the full type init). `param_1` is typed as `int` throughout, so all `*(type *)(param_1 + 0xNNNN)` offsets are **direct byte offsets** (no multiply-by-4 gotcha).

All 31 claimed offsets verified correct:

| INI Key | Claimed | Actual | Status |
|---------|---------|--------|--------|
| `Radar=` | +0x16A4 | +0x16A4 | ✓ |
| `SpySat=` | +0x16A5 | +0x16A5 | ✓ |
| `UnitRepair=` | +0x16A9 | +0x16A9 | ✓ |
| `UnitReload=` | +0x16AA | +0x16AA | ✓ |
| `Bunker=` | +0x16AB | +0x16AB | ✓ |
| `Cloning=` | +0x16AC | +0x16AC | ✓ |
| `Grinding=` | +0x16AD | +0x16AD | ✓ |
| `InfantryAbsorb=` | +0x16AF | +0x16AF | ✓ |
| `SecretLab=` | +0x16B0 | +0x16B0 | ✓ |
| `Refinery=` | +0x16BB | +0x16BB | ✓ |
| `Weeder=` | +0x16BC | +0x16BC | ✓ |
| `WeaponsFactory=` | +0x16BD | +0x16BD | ✓ |
| `LaserFencePost=` | +0x16BE | +0x16BE | ✓ |
| `LaserFence=` | +0x16BF | +0x16BF | ✓ |
| `Hospital=` | +0x16C1 | +0x16C1 | ✓ |
| `Armory=` | +0x16C2 | +0x16C2 | ✓ |
| `CloakGenerator=` | +0x16C7 | +0x16C7 | ✓ |
| `SensorArray=` | +0x16C8 | +0x16C8 | ✓ |
| `Helipad=` | +0x16CB | +0x16CB | ✓ |
| `OrePurifier=` | +0x16CC | +0x16CC | ✓ |
| `FactoryPlant=` | +0x16CD | +0x16CD | ✓ |
| `Power=` (output) | +0xEE0 | +0xEE0 | ✓ |
| `Power=` (drain) | +0xEE4 | +0xEE4 | ✓ |
| `ExtraPower=` | +0xEE8 | +0xEE8 | ✓ |
| `Upgrades=` | +0x14E0 | +0x14E0 | ✓ |
| `MaxNumberOccupants=` | +0x1580 | +0x1580 | ✓ |
| `CanBeOccupied=` | +0x157B | +0x157B | ✓ |
| `CanOccupyFire=` | +0x157C | +0x157C | ✓ |
| `PowersUpBuilding=` | +0xE88 | +0xE88 | ✓ |
| `PowersUpToLevel=` | +0x16FC | +0x16FC | ✓ |
| `NumberOfDocks=` | +0x1780 | +0x1780 | ✓ |

### Notable Power= semantics (verified)

The `Power=` key shares two fields (+0xEE0 and +0xEE4) with a sign-split convention:

```c
iVar15 = *(int *)(param_1 + 0xee0);   // prior output
if (iVar15 < 1) {
    iVar15 = -*(int *)(param_1 + 0xee4);  // else use -drain as default
}
iVar15 = CCINIClass__ReadInt(iVar21, s_Power_, iVar15);
*(int *)(param_1 + 0xee0) = iVar15;
if (iVar15 < 0) {
    *(int *)(param_1 + 0xee4) = -iVar15;   // negative value → drain
    *(int *)(param_1 + 0xee0) = 0;
} else {
    *(int *)(param_1 + 0xee4) = 0;         // positive value → output, zero drain
}
```

So a single `Power=` INI line populates either output (+0xEE0) or drain (+0xEE4) depending on sign — never both. `ExtraPower=` at +0xEE8/+0xEEC uses the same split pattern.

### Notable PowersUpBuilding= semantics

`PowersUpBuilding=` is read as a string into a 0x18-byte stack buffer, then `memcpy`'d to `param_1 + 0xE88` only if the buffer is non-empty. This is a string field in the struct, not a pointer. The 0x18 = 24-byte limit matches the INI name-buffer convention used elsewhere in the binary.

### Notable NumberOfDocks= semantics

`NumberOfDocks=` is read via a different CCINI base: `CCINIClass__ReadInt(param_1 + 0x24, s_NumberOfDocks_, iVar21)`. Result into +0x1780. Also guards a separate field +0x178C (likely "current docks count"): if `NumberOfDocks < +0x178C`, something is clamped (read lines 4941-4944 of decompilation for full context).

---

## Part 3: BuildingClass vtable verification

**Vtable base: `0x007E3EBC`** — confirmed correct.

Key slot verification (reading raw memory):

| Slot | Offset | Purpose | Claimed | Actual | Status |
|------|--------|---------|---------|--------|--------|
| 23 | +0x05C | Update | `0x0043FB20` | `0x0043FB20` | ✓ |
| 53 | +0x0D4 | OnDestroyed | `0x00445880` | `0x00445880` | ✓ |
| 54 | +0x0D8 | Unlimbo | `0x00440580` | `0x00440580` | ✓ |
| 64 | +0x100 | ExitObject | `0x00443C60` | `0x00443C60` | ✓ |
| 91 | +0x16C | ReceiveDamage | `0x00442230` | `0x00442230` | ✓ |

All 5 vtable slot claims correct. No discrepancies.

---

## Appendix: Commands used for verification

```
# Vtable memory dump (slots 0..199)
curl http://127.0.0.1:8089/read_memory?address=0x007E3EBC&length=400&format=hex
curl http://127.0.0.1:8089/read_memory?address=0x007E40CC&length=400&format=hex

# Mission_Dispatch switch table
curl http://127.0.0.1:8089/decompile_function?address=0x005B3060

# ReadINI (19,474 bytes of decompilation)
curl http://127.0.0.1:8089/decompile_function?address=0x0045FE50

# Individual handler sizing
curl http://127.0.0.1:8089/get_function_by_address?address=0x0044ACF0
# (repeated for each handler)
```

Functions newly created in Ghidra during this session (were `FUN_` but no function body defined):
- `0x0044D880` (Mission_Eaten/Rescue)
- `0x0044E440` (Mission_Unload)
- `0x005B2E50`, `0x005B2E90` (MissionClass default Guard/Return handlers returning 0x1C2)
- `0x0044B760`, `0x0044B770` (JMP thunks to the above)
