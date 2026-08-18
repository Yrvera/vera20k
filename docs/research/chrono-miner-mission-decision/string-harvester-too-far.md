# STR "HarvesterTooFarDistance" — Decode

**Proposed Ghidra label:** `STR_INIKey_HarvesterTooFarDistance`

---

## Summary

`"HarvesterTooFarDistance"` is an INI key string at `0x0083C480` used by `RulesClass__ReadGeneral` to parse the `[General]` section INI value into `RulesClass+0xD78`. The parsed integer is the cell-distance threshold for the non-chrono harvester return path: if the non-chrono harvester's distance to the target refinery exceeds this value (in cells × 256 = leptons), it drives toward the refinery rather than reserving it and waiting. Default stock YR value is `5` cells.

---

## Active in YR

**Yes.** Called from `RulesClass__ReadGeneral` which runs on game start. The stored value at `RulesClass+0xD78` is consumed by `UnitClass__Mission_Harvest` case 2 (RETURN state) for the non-chrono harvester branch (`UnitTypeClass.Teleporter == 0`). Stock value is 5 cells.

---

## Type

**INI key string** — passed as the second argument to `CCINIClass__ReadInt` to look up `[General] HarvesterTooFarDistance` in `rules(md).ini`.

---

## Consumers

### 1. `RulesClass__ReadGeneral` — write site

Verified via `get_xrefs_to 0x0083C480` → `From 0066ffe3 in RulesClass__ReadGeneral [DATA]`.

The decompile (from extracted output of `decompile_function` on the containing function) shows:

```c
uVar10 = CCINIClass__ReadInt(PTR_s_General_007f0c9c,
                              str_HarvesterTooFarDistance,   // 0x0083C480
                              *(undefined4 *)(param_1 + 0xd78));  // existing default
*(undefined4 *)(param_1 + 0xd78) = uVar10;   // store parsed value
```

- Section: `[General]` (via `PTR_s_General_007f0c9c`)
- Default argument: existing value at `param_1+0xD78` (pre-initialized to `5` from INI or binary default)
- Stored at: `RulesClass+0xD78` (direct `int` write, no shift/scale)

The adjacent read immediately after stores `ChronoHarvTooFarDistance` at `RulesClass+0xD7C`, confirmed by the plate comment on `RulesClass__ReadGeneral`:
> `rules+0xD78 = HarvesterTooFarDistance (default 5)`
> `rules+0xD7C = ChronoHarvTooFarDistance (default 50)`

(verified via `decompile_function 0x0066ffe3` → plate comment content)

### 2. `UnitClass__Mission_Harvest` — read/consumer site

From the decompile of `UnitClass__Mission_Harvest @ 0x0073E5E0` (verified in this session), case 2 (RETURN state):

```c
// Non-chrono harvester close-return branch (cVar1 == '\0' means not Teleporter):
if (iVar8 <= *(int *)(g_RulesClass_Instance + 0xd78) * 0x100) {
LAB_0073ee51:
    iVar8 = (**(code **)(*param_1 + 0x278))(2, piVar3);  // radio 0x02 to refinery
    if (iVar8 == 1) {
        param_1[0x2f] = 3;  // advance to state 3 (dock approach)
        ...
    }
}
```

The comparison `distance <= Rules.HarvesterTooFarDistance * 0x100` uses leptons (distance is computed as `Sqrt(dx² + dy² + dz²)` in leptons, and `Rules+0xD78 * 256` converts cells to leptons). If the non-chrono harvester is within this many cells of the refinery, it sends HELLO (`0x02`) and transitions to state 3. If farther, it falls through to the far-return path.

The chrono miner (Teleporter=yes) uses `Rules+0xD7C` (`ChronoHarvTooFarDistance`) for the same decision — the two thresholds are applied in parallel branches distinguished by `UnitTypeClass.Teleporter` flag (`UnitTypeClass+0xCD4`).

---

## Stock YR usage

From `ini/rulesmd.ini` line 293:
```
HarvesterTooFarDistance=5 ;gs If a harvester is farther than this from the refinery it wants, it will move next to it instead of reserving it and refigure things out when it stops.  This should be small to approximate the wait time concern versus driving to the next refinery.
```

From `ini/rules.ini` line 234: identical entry with value `5`.

**Default: 5 cells.** The threshold in leptons is `5 × 256 = 1280`.

String address `0x0083C480` verified via `read_memory 0x0083C480` → bytes `48 61 72 76 65 73 74 65 72 54 6F 6F 46 61 72 44 69 73 74 61 6E 63 65 00` = `"HarvesterTooFarDistance\0"` ✓.

---

## Out-of-scope refs

- `RulesClass__ReadGeneral` at `0x0066D530` — not decoded in this session; only the specific ReadInt call site is in scope.
- `ChronoHarvTooFarDistance` at `0x0083C464` / `RulesClass+0xD7C` — sibling key, decoded separately in task #9.

---

## Unverified

YELLOW — not verified in this session:

- The exact binary address of the `CCINIClass__ReadInt` call for `HarvesterTooFarDistance` within `RulesClass__ReadGeneral`. Location is identified as near `0x0066FFE3` (the DATA xref into `ReadGeneral`) but the precise call address was not extracted.
- Whether `RulesClass+0xD78` is the same field accessed by any other function besides `Mission_Harvest` case 2. No further xref search was performed on the storage address.
