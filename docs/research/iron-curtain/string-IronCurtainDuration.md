# string-IronCurtainDuration

## Identity

| Field | Value |
|---|---|
| String | `"IronCurtainDuration"` |
| Address | `0x0083b0b8` |
| INI Section | `[CombatDamage]` |
| INI Key | `IronCurtainDuration=` |
| Type | Integer (frames) |
| Default | 750 |

## Verification

String address verified via `get_xrefs_to 0x0083b0b8` — returns xref from task anchor note (xref confirmed at `0x0066c63e` inside `RulesClass__ReadCombatDamage`).

From `decompile_function 0x0066bbb0` (`RulesClass__ReadCombatDamage`), verbatim:
```c
local_d8 = (double)CONCAT44(*(undefined4 *)(param_1 + 0xfe8), s_IronCurtainDuration_0083b0b8);
// ... CCINIClass__ReadInt
*(undefined4 *)(param_1 + 0xfe8) = uVar3;
```

**Storage field**: `RulesClass + 0xfe8` (4-byte int, frames).

## Semantics

Duration in game frames that an Iron Curtain invulnerability lasts. At 15 fps, 750 frames = 50 seconds. Applied via `TechnoClass__IronCurtain` (0x0070e2b0) which stores this value into `TechnoClass + 0x194`. Active state checked by `TechnoClass__IsIronCurtainActive` (0x0041bf40): if `g_CurrentFrameCounter - apply_frame < duration`, the unit is invulnerable.

## Xref count: 1 (single consumer in ReadCombatDamage)

## Active in YR: Yes
