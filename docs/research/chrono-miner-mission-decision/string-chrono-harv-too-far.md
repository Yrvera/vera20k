# "ChronoHarvTooFarDistance" — Decode Doc

**Proposed Ghidra label:** `STR_INIKey_ChronoHarvTooFarDistance`

## Summary

The string `"ChronoHarvTooFarDistance"` at `0x0083C464` is the INI key read by
`RulesClass::ReadGeneral` from the `[General]` section of `rules(md).ini`. It stores a
cell-distance integer threshold into `RulesClass+0xD7C`. At runtime,
`UnitClass__Mission_Harvest` reads `Rules+0xD7C`, multiplies by `0x100` (256) to convert
cells to leptons, and uses the result as the chrono miner "too far" threshold in state 2
RETURN. Chrono miners within this distance use the radio-accept (close) dock path; those
beyond it fall back to the `QueueingCell` passable-cell search.

## Active in YR

**Yes.** The string has exactly one xref: `RulesClass__ReadGeneral @ 0x00670003` (DATA),
verified via `get_xrefs_to 0x0083C464`. `ReadGeneral` is called unconditionally at game
start from the main INI load path. The default value (`50`) is present in both
`ini/rules.ini` line 235 and `ini/rulesmd.ini` line 294 (verified by in-repo grep).

## Type

INI key — `[General]` section, `rules(md).ini`.

## Consumers

| Function | Address | Role | Struct offset stored |
|----------|---------|------|---------------------|
| `RulesClass__ReadGeneral` | `0x00670003` | Reads via `CCINIClass__ReadInt`, stores result | `RulesClass+0xD7C` |
| `UnitClass__Mission_Harvest` | `0x0073E5E0` | Reads `Rules+0xD7C`, multiplies `× 0x100` | — (comparison only) |

The ReadGeneral site:
```c
// from RulesClass__ReadGeneral 0x0066D530 (grep of saved decompile output)
uVar10 = CCINIClass__ReadInt(PTR_s_General_007f0c9c,
                             str_ChronoHarvTooFarDistance,    // 0x0083C464
                             *(undefined4 *)(param_1 + 0xd7c));  // default = current value
*(undefined4 *)(param_1 + 0xd7c) = uVar10;
```

The Mission_Harvest consumer (state 2, chrono branch):
```c
// from decompile_function 0x0073E5E0
if (iVar8 <= *(int *)(g_RulesClass_Instance + 0xd7c) * 0x100) goto LAB_0073ee51;
// iVar8 = Euclidean distance in leptons (from Sqrt_Approx)
// LAB_0073ee51 = radio-accept (close) path
```
The multiplier `× 0x100` converts cells → leptons (1 cell = 256 leptons).

## Stock YR Usage

| INI | Section | Key | Default value | Comment |
|-----|---------|-----|---------------|---------|
| `rules.ini` | `[General]` | `ChronoHarvTooFarDistance` | `50` | verified line 235 |
| `rulesmd.ini` | `[General]` | `ChronoHarvTooFarDistance` | `50` | verified line 294 |

The `rulesmd.ini` comment: *"Same as [HarvesterTooFarDistance], but for Chrono harvesters.
Rather than have them teleport super far and then repick an ore patch (or teleport super far
and drive super far back), they will stay on their side of the map (like for two bases)".*

Default 50 cells → 50 × 256 = 12800 leptons ≈ 50 cells threshold. This is 10× larger than
`HarvesterTooFarDistance` (5 cells) because chrono miners teleport rather than drive.

## Out-of-Scope Refs

- `HarvesterTooFarDistance` string and `Rules+0xD78` — companion key; task #10
- `CCINIClass__ReadInt` — INI system scope
- `PTR_s_General_007f0c9c` — pointer to the `[General]` section string

## Unverified Claims (YELLOW)

None. All claims in this doc are verified via:
- `get_xrefs_to 0x0083C464` — single xref to `RulesClass__ReadGeneral`
- Grep of saved `RulesClass__ReadGeneral` decompile output confirming `param_1+0xD7C` store
- In-repo grep of `ini/rules.ini` and `ini/rulesmd.ini` confirming default `50`
- `decompile_function 0x0073E5E0` confirming `Rules+0xD7C * 0x100` comparison in state 2
