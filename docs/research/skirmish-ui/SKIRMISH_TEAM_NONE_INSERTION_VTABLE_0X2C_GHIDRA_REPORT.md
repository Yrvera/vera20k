# Skirmish Team None Insertion Vtable +0x2C - Ghidra Research Report

**Address(es):** `0x004E5B60`, `0x004E5D60`, `0x005E2F80`, `0x005D5DC0`, `0x005D5B90..0x005D5D14`, `0x006AE6E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The selected-mode/session vtable `+0x2C` rule used by `FUN_004E5B60` to decide whether the Skirmish Team combo inserts the `None` item.  
**Non-Scope:** Full WOL/online team semantics, gameplay alliance resolution after launch, and unrelated team/start destination naming already covered by adjacent reports.  
**Confidence:** High for the helper rule and standard offline Skirmish call path; Medium for stock override-file defaults because the repo only exposes `ini/mpmodesmd.ini`, not the retail `MPBattleMD.ini` payload.  
**Active in YR:** Conditional. The helper is active in standard offline Skirmish; insertion of `None` depends on the selected multiplayer mode object's `MustAlly` flag.

## 1. Overview

`FUN_004E5B60` rebuilds one Team combo (`0x76D..0x774`). It inserts `GUI:NoneAsSymbols` with item data `-2` only when the selected multiplayer mode object's vtable `+0x2C` returns a negative value.

For standard offline Skirmish (`g_GameMode` neither `3` nor `4`), the object is resolved by `FUN_005E2F80()`, which looks up the currently selected multiplayer mode id in `DAT_00A8B250` and falls back to the first mode if lookup fails. The concrete `MultiplayerGameMode` vtable `+0x2C` method at `0x005D5DC0` returns `-2` when the mode's `MustAlly` byte at object offset `+0x3F` is `0`, and returns `0` when `MustAlly` is nonzero.

## 2. Key Offsets And Values

| Item | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `MultiplayerGameMode+0x3F` | `MustAlly` boolean read by vtable `+0x2C` | constructor reads string key `MustAlly` at `0x008308A0`; `0x005D5DC0` reads `[ECX+0x3F]` | Yes, conditional by selected mode |
| vtable `+0x2C` | Team None availability method | `0x005D5DC0`: `byte+0x3F == 0 -> -2`, nonzero -> `0` | Yes |
| `GUI:NoneAsSymbols` | Team combo `None` display item | `FUN_004E5B60`, string `0x00822BF8`, item data `-2` | Conditional |
| `0x45F` | Offline Skirmish `None` string id path | `FUN_004E5B60` offline branch after negative `+0x2C` result | Yes |
| `0x456` | Network/WOL branch `None` string id path | `FUN_004E5B60` online branch with `DAT_00A8B23C` | Conditional; not standard offline Skirmish |

## 3. Core Logic

### Offline Team Combo Population

`FUN_004E5B60(hwnd, control_id)` performs this sequence for offline Skirmish:

1. Hide the combo and clear it with message `0x14B`.
2. Configure owner-draw/list behavior with messages `0x4DD` and `0x4DE`, max visible rows `9`.
3. Because `g_GameMode` is not `3` or `4`, call `FUN_005E2F80()` to get the selected `MultiplayerGameMode`.
4. If no mode object exists, skip `None`.
5. Call selected mode vtable `+0x2C`.
6. If the return is negative, load string id `0x45F`, insert it, and set item data `-2`.
7. Always append Team A-D entries from `DAT_008B3FC0` with item data `0..3`.
8. Select row `0`, clear the disabled/grey flag with `0x4F1`, and restore visibility if it was visible before rebuild.

Active in YR: Yes for standard offline Skirmish. Evidence: `FUN_004E5D60` calls `FUN_004E5B60` for all eight team controls when not in `g_GameMode == 3 || 4`; `FUN_006AE6E0` calls `FUN_004E5AC0`, `FUN_004E5D60`, and later refreshes the selected mode then calls `FUN_004E5D60` again.

### Concrete Vtable Rule

The concrete method at `0x005D5DC0` is:

```text
if (*(u8 *)(this + 0x3F) == 0) return -2;
return 0;
```

Evidence: Ghidra assembly context at `0x005D5DC0` reads `[ECX+0x3F]`, uses `NEG/SBB/AND 0x2`, then `ADD EAX,-0x2`. The result is `-2` for zero and `0` for nonzero.

Active in YR: Yes, when `FUN_004E5B60` receives a normal `MultiplayerGameMode` object from `FUN_005E2F80`. No TS-only gate was found in this path.

### MustAlly Initialization

The `MultiplayerGameMode` constructor initializes `+0x3F` to `0`, then reads the `MustAlly` key from the mode/rules INI context:

| Address / key | Behavior | Active in YR |
|---|---|---|
| `0x005D5BF6` | initializes `+0x3F` to `0` | Yes |
| `0x005D5CED..0x005D5D07` | reads string key `MustAlly` (`0x008308A0`) and writes result to `+0x3F` | Yes |
| `0x005D5D0C..0x005D5D11` | if `MustAlly` is true but `AlliesAllowed` (`+0x3C`) is false, clears `MustAlly` back to `0` | Yes |

This means `MustAlly=true` suppresses the `None` item only when allies are also allowed. A mode cannot keep `MustAlly=true` while `AlliesAllowed=false` in this constructor.

Active in YR: Yes for `MPModesMD.ini`-created `MultiplayerGameMode` objects. Evidence: loader uses `MPModesMD.ini` (`0x00830A18`) and constructs/sorts mode objects into `DAT_00ABFDA4`.

## 4. INI Keys

| Key / file | Default / observed value | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| `MPModesMD.ini` | defines mode ids and override files | source of `MultiplayerGameMode` list; Battle id `1` is listed before TeamGame id `9` | `ini/mpmodesmd.ini:7..10` | Yes |
| `MustAlly` | constructor default `false` if absent | `false` => vtable `+0x2C` returns `-2` => Team combo inserts `None`; `true` => returns `0` => no `None` | key string `0x008308A0`; `0x005D5DC0` | Conditional |
| `AlliesAllowed` | constructor default `true` before INI read | if false, clears `MustAlly` even if `MustAlly` read true | key string `0x008308AC`; `0x005D5D0C..0x005D5D11` | Conditional |

The local repo has `ini/mpmodesmd.ini` but not extracted `MPBattleMD.ini` / `MPTeamMD.ini`, so this report does not claim every stock override-file value. For the standard offline Skirmish default Battle path, the binary default for missing `MustAlly` is false, so `None` is inserted unless the selected mode's override INI explicitly sets `MustAlly=true`.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Team refresh entry | `FUN_004E5D60` iterates all 8 team controls; standard offline path calls `FUN_004E5B60`, not disabled `FUN_004E5CB0` | `FUN_004E5D60` | Yes |
| Selected mode lookup | `FUN_005E2F80` calls `FUN_005D5F30(DAT_00A8B250)` and falls back to `FUN_005D5E10()` | `0x005E2F80..0x005E2F94` assembly context | Yes |
| Skirmish init | `FUN_006AE6E0` calls team table init and team refresh before and after selected mode normalization | `FUN_006AE6E0` | Yes |
| Post-refresh selection | after selected map/mode setup, AI team controls are set to `-2` when selected mode `+0x3C`/`AlliesAllowed` is false; otherwise to `3` | `FUN_006AE6E0` loop after `DAT_00A8B23C = local_4` | Yes |
| Online/WOL branch | when `g_GameMode == 3 || 4`, `FUN_004E5B60` uses `DAT_00A8B23C` directly and string id `0x456` | `FUN_004E5B60` | Conditional; outside standard offline Skirmish |

## 6. Current Rust Implementation Status

Current Rust Skirmish shell state has a generic opponent team field but does not model the stock `MustAlly`-driven presence/absence of `None`, the selected multiplayer mode object, or the `AlliesAllowed` post-refresh default selection behavior.

Evidence: `src/ui/skirmish_shell/state.rs` and prior Skirmish UI reports; no Rust files were modified for this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_004E5B60` offline `None` insertion branch | verified | decompile `0x004E5B60`; negative `+0x2C` return inserts item data `-2` | none |
| Concrete `MultiplayerGameMode` vtable `+0x2C` | verified | assembly `0x005D5DC0..0x005D5DCD` | none |
| `MustAlly` constructor field | verified | constructor block `0x005D5BF6`, `0x005D5CED..0x005D5D11`; string `0x008308A0` | none |
| Standard offline Skirmish call path | verified | `FUN_004E5D60`, `FUN_006AE6E0`, `FUN_005E2F80` | none |
| Online/WOL branch | touched-not-exhausted | `FUN_004E5B60` branch uses `DAT_00A8B23C` and id `0x456` | full WOL/team behavior out of scope |
| Stock override INI values | touched-not-exhausted | repo `ini/mpmodesmd.ini` lists mode ids/override filenames | extract retail `MPBattleMD.ini` / `MPTeamMD.ini` if exact stock mode flags are needed |

## 8. Open Questions - Final State

[RESOLVED] OQ-TN-001 - What concrete method is vtable `+0x2C` for standard offline Skirmish Team combo population? It is the `MultiplayerGameMode` method at `0x005D5DC0`, reached through the selected mode object from `FUN_005E2F80`. Evidence: `FUN_004E5B60`, `0x005E2F80`, vtable assembly at `0x005D5DC0`.

[RESOLVED] OQ-TN-002 - What does the method return? It returns `-2` when `this+0x3F == 0`, otherwise `0`. Evidence: `0x005D5DC0..0x005D5DCD`.

[RESOLVED] OQ-TN-003 - What does `this+0x3F` mean? It is the `MustAlly` flag read from key string `0x008308A0`, defaulted to `0`, and cleared if `AlliesAllowed` is false. Evidence: constructor block `0x005D5BF6`, `0x005D5CED..0x005D5D11`.

[RESOLVED] OQ-TN-004 - How does the return value control `None`? `FUN_004E5B60` inserts `GUI:NoneAsSymbols` item data `-2` only when the vtable return is negative. Evidence: `FUN_004E5B60`.

[RESOLVED] OQ-TN-005 - Is this standard offline Skirmish-active? Yes. `FUN_004E5D60` calls `FUN_004E5B60` for all team controls outside `g_GameMode == 3 || 4`; `FUN_006AE6E0` invokes that refresh in dialog `0x102` initialization. Evidence: `FUN_004E5D60`, `FUN_006AE6E0`.

[DEFERRED] OQ-TN-006 - Exact retail values inside `MPBattleMD.ini`, `MPTeamMD.ini`, and other override files. Category: out-of-scope. Reason: the binary rule is resolved; extracting every stock override file is a separate content audit.

## Sources

- Ghidra decompiled/read-only: `0x004E5B60`, `0x004E5D60`, `0x004E5ED0`, `0x005E2F80`, `0x005D5F30`, `0x005D5E10`, `0x006AE6E0`.
- Ghidra assembly context: `0x005D5DC0..0x005D5DCD`.
- Binary string/constructor evidence: `0x008308A0` = `MustAlly`, `0x008308AC` = `AlliesAllowed`, constructor writes around `0x005D5BF6`, `0x005D5CED..0x005D5D11`.
- Prior reports: `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`, `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`.
- INI checked: `ini/mpmodesmd.ini`.
