# Skirmish Mode Role UI Node +0x6B - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005D8CB0`, `0x005CAE70`, `0x005CAE10`, `0x005CAEB0`, `0x005CA6D0`, `0x005CB400`, `0x005D6130`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** provenance of node field `+0x6B` before the selected-mode Start acceptance path reads it, with emphasis on offline Skirmish dialog `0x102`, selected `MPModes` mode objects, and Siege/Unholy role validation.  
**Non-Scope:** online/WOL lobby role UI, gameplay spawn placement after shell exit, full `MultiplayerTeam` class hierarchy, and extracting missing override INI payloads from MIX archives.  
**Confidence:** High that ordinary offline Skirmish controls do not write node `+0x6B`; High that `MultiplayerTeam` vtable method `0x005D8CB0` is the direct node writer; Medium for exact attacker/defender constructor role-value assignment because the relevant constructor thunks are not cleanly function-bounded in the current Ghidra database.  
**Active in YR:** Conditional. The writer and Siege validator are present in `gamemd.exe`, but the exposed stock `ini/mpmodesmd.ini` has no `[Siege]` section; Unholy is exposed but does not read node `+0x6B`.

## 1. Result

No ordinary offline Skirmish dialog `0x102` control writes node `+0x6B` before the selected-mode Start acceptance call. The familiar controls write different destinations:

| Control family | Final destination | Writes node `+0x6B`? | Active in YR |
|---|---|---:|---|
| AI row state `0x50B..0x51D` | `DAT_00A8B27C[slot]` | No | Yes |
| Country `0x6A1`, `0x510..0x521` | session/local country and `DAT_00A8B29C[slot]` | No | Yes |
| Start `0x6A3..0x6AB` | `DAT_00A8B39C`, node `+0x5B`, and `DAT_00A8B2DC[slot]` | No | Yes |
| Team `0x76D..0x774` | `DAT_00A8B3A4`, node `+0x63`, and `DAT_00A8B2FC[slot]` | No | Yes |

The direct writer found in this slice is `MultiplayerTeam` virtual method `0x005D8CB0`. It first calls the role object's validity method at vtable `+0x04`; if that accepts, it writes the role object's dword at object `+0x08` into `DAT_00A8DA78[index] + 0x6B` and returns success.

Active in YR: Conditional. The method is binary-live through multiplayer team/role objects. It is not reached by the ordinary `0x102` Start packing controls verified in `0x006ACEE0`.

## 2. Start Acceptance Ordering

`FUN_006ACEE0` calls the selected `MPModes` object vtable `+0x14` at `0x006AD2D2` before it creates/appends the local node for the current offline Skirmish Start. Only after the selected-mode method accepts does the function read local controls and allocate the local node.

The post-accept local node writes are:

| Node field | Source | Evidence | Active in YR |
|---:|---|---|---|
| `+0x4B` | `DAT_00A8B3AC` | `0x006AD677..0x006AD67F` | Yes |
| `+0x53` | `DAT_00A8B394` | `0x006AD682..0x006AD688` | Yes |
| `+0x5B` | local start `DAT_00A8B39C` | `0x006AD68B..0x006AD691` | Yes |
| `+0x63` | local team `DAT_00A8B3A4` | `0x006AD694..0x006AD699` | Yes |
| `+0x73` | literal `-1` | `0x006AD69C` | Yes |

There is no `+0x6B` write in the offline Start packing block at `0x006AD647..0x006AD6F6`. Active in YR: Yes for dialog `0x102`.

## 3. Direct Writer: MultiplayerTeam +0x08 Method

`0x005D8CB0` is the direct scoped writer:

1. Calls `(*(this->vtable + 0x04))(node_index)`.
2. If that returns false, returns false without writing the node.
3. Reads `this + 0x08`.
4. Stores that dword to `*(DAT_00A8DA78[node_index] + 0x6B)`.
5. Returns success.

Active in YR: Conditional. This is binary-present multiplayer role/team behavior; it requires a mode/team-role path to invoke it, not the generic Skirmish Start packing controls. Evidence: decompile of `0x005D8CB0`.

## 4. Siege Role Objects

The binary has `MultiplayerSiegeDefenderTeam` and `MultiplayerSiegeAttackerTeam` class evidence:

| Role object | Verified string/class evidence | Vtable evidence | Active in YR |
|---|---|---|---|
| Defender team | `MP:DefenderTeam` at `0x0082FFE4`; source string `D:\ra2mdpost\MPSiegeTeam.cpp` | constructor thunk stores vtable `0x007EE7E4`; vtable includes validity method `0x005CAE70` and writer `0x005D8CB0` | Conditional; Siege data absent from exposed stock `mpmodesmd.ini` |
| Attacker team | `MP:AttackerTeam` at `0x00830044`; same source area | constructor thunk stores vtable `0x007EE7F4`; vtable uses the shared writer `0x005D8CB0` | Conditional; Siege data absent from exposed stock `mpmodesmd.ini` |

`0x005CAE70` is the defender-specific validity method. It scans `DAT_00A8DA78[0..DAT_00A8DA84)` and rejects when another node already has dword `+0x6B == 1`, excluding the passed node index. Active in YR: Conditional, only through the Siege defender role object.

The generic attacker validity method at `0x005D8C90` accepts only indexes in range `0 <= index < DAT_00A8DA84`. Active in YR: Conditional, only through the attacker role object path.

## 5. Siege And Unholy Start Validation

Siege Start acceptance at `0x005CA6D0` reads existing node `+0x6B` values from `DAT_00A8DA78` and validates role distribution. Prior verified report maps:

| Node `+0x6B` | Siege meaning from validator | Evidence | Active in YR |
|---:|---|---|---|
| `0` | accepted neutral/unused role for the scan | `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE...`, `0x005CA6D0` | Conditional |
| `1` | defender/besieged; second one rejects as `MP:OnlyOneBeseiged` | same report, `0x005CA709..0x005CA716` | Conditional |
| `2` | attacker; at least one required | same report, `0x005CA70C..0x005CA710` | Conditional |
| other | illegal team role | same report, `0x005CA748` | Conditional |

Unholy Start acceptance at `0x005CB400` does not read node `+0x6B`; it gates acceptance on global byte `DAT_00A8B258` and the base accept method. Active in YR: Conditional, and stock `ini/mpmodesmd.ini` exposes `[Unholy]`.

## 6. MPModes Stock Data Check

The binary registers the `Siege` category in the mode loader, but the exposed local `ini/mpmodesmd.ini` contains `[Battle]`, `[ManBattle]`, `[FreeForAll]`, `[Unholy]`, and `[Cooperative]`, with no `[Siege]` section.

Active in YR: the binary support is present; stock exposed local roster does not make Siege selectable. Evidence: `0x005D7CE0` category registration from the sibling MPModes audit, plus `ini/mpmodesmd.ini:7..27`.

## 7. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| Offline Start packing writes | verified | `0x006AD647..0x006AD6F6` | none for `+0x6B` absence |
| AI/live array destinations | verified from sibling reports | `0x006AD453..0x006AD4E6` | none |
| Direct node `+0x6B` writer | verified | `0x005D8CB0` | none for the write itself |
| Defender duplicate guard | verified | `0x005CAE70` assembly bytes | exact UI surface invoking it in stock data is absent |
| Attacker range guard | verified | `0x005D8C90` | none |
| Siege Start reader | verified by sibling report | `0x005CA6D0` | exact constructor role-value assignment remains medium confidence |
| Unholy Start reader check | verified by sibling report | `0x005CB400` | none; it does not read `+0x6B` |
| Stock local MPModes roster | verified | `ini/mpmodesmd.ini:7..27` | archive extraction of hidden override files remains out of scope |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does any ordinary offline Skirmish `0x102` control write node `+0x6B` before Start validation? No. Start packing writes node `+0x5B`, `+0x63`, and `+0x73`, while AI rows write global arrays. Evidence: `0x006AD647..0x006AD6F6`, `0x006AD453..0x006AD4E6`.

[RESOLVED] OQ-2 - What direct binary method writes node `+0x6B`? `0x005D8CB0`, the multiplayer-team role writer, after a vtable `+0x04` validity check. Evidence: decompile `0x005D8CB0`.

[RESOLVED] OQ-3 - Does Unholy read node `+0x6B`? No. Its Start acceptance gate is `DAT_00A8B258`, not the role field. Evidence: sibling report `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `0x005CB400`.

[RESOLVED] OQ-4 - Is Siege stock-selectable from exposed local `MPModesMD.ini`? No. The binary registers a Siege category, but the exposed roster has no `[Siege]` section. Evidence: `ini/mpmodesmd.ini:7..27`, sibling MPModes audit.

[DEFERRED] OQ-5 - Exact constructor-time integer assignment for the attacker role object. Category: Ghidra-boundary limitation. Reason: the constructor/helper thunks around `0x005CAE10..0x005CAEB0` are not cleanly function-bounded in this database; the direct writer and Siege reader are verified, but a separate class-layout pass should pin every constructor argument.

## Sources

- Ghidra decompiled/read-only: `0x006ACEE0`, `0x005D8CB0`, `0x005D8C90`, `0x004E6030`, `0x005D6130`.
- Ghidra memory/assembly read-only: `0x005CAE10`, `0x005CAE70`, `0x005CAEB0`, vtables `0x007EE7E4`, `0x007EE7F4`, strings around `0x0082FFE4`, `0x00830018`.
- Prior reports: `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md`, `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md`.
- INI: `ini/mpmodesmd.ini`.
