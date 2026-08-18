# Skirmish House+0x1605C Team Adjunct Consumer - Ghidra Research Report

**Address(es):** `0x00687F10`, `0x005D74A0`, `0x004F9B70`, `0x005D6BE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** post-`ScenarioClass__Create_Houses` consumers of `HouseClass+0x1605C` in the standard offline Skirmish/Battle-style path, with `House+0x16058` used only as contrast.  
**Non-Scope:** full combo population semantics, all game-mode variants, map start placement formulas, and runtime alliance-change UI after the match starts.  
**Confidence:** High for the standard Battle vtable binding and `0x1605C` alliance-consumer behavior; Medium for exact shell sentinel display names because this report did not re-trace dropdown row population.  
**Active in YR:** Yes for standard Battle-style Skirmish when the selected mode object's `+0x88` lifecycle callback runs. Evidence: the standard Battle vtable at `0x007EE184` binds `+0x88` to `0x005D74A0`, and offline Skirmish uses `DAT_00A8B23C` as the selected mode object in `ScenarioClass__Full_Init @ 0x00686B20`.

## 1. Overview

`House+0x1605C` is not the standard Battle explicit-start field. `ScenarioClass__Create_Houses @ 0x00687F10` writes it from the shell team/adjunct value, and the first verified behavioral consumer is the Battle-mode `+0x88` callback at `0x005D74A0`, which auto-allies non-special houses that share the same non-sentinel `0x1605C` value.

The standard Battle start preassignment field remains `House+0x16058`: the vtable `+0x80` method at `0x005D6BE0` reads `House+0x16058` and writes `ScenarioClass+0x1180 + start_index*4`. Docs that call `House+0x1605C` a start location in this path are stale.

## 2. Key Offsets

| Field / source | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| node `+0x5B` / AI array `DAT_00A8B2DC` | explicit start selection copied to `House+0x16058` | `0x006880F2`, `0x006881EC`, `0x005D6C12` | Yes; non-campaign Skirmish house setup |
| node `+0x63` / AI array `DAT_00A8B2FC` | team/alliance adjunct copied to `House+0x1605C` | `0x006880FC..0x00688101`, `0x006881F5..0x006881FB` | Yes; non-campaign Skirmish house setup |
| `House+0x34 -> +0x1A6` | special/non-participant house gate skipped by start/team consumers | `0x005D74C6..0x005D74D0`, `0x005D6C05..0x005D6C10` | Yes; used on standard mode callbacks |
| `House+0x1EC` | player-control flag; used by a separate vtable path at `0x005C3220`, not by the scoped Battle `0x1605C` equality consumer | `0x005C328F`, `0x005C33DC` | Conditional; different mode/object callback |

## 3. Core Logic

### 3.1 Writer in `Create_Houses`

For human node records, `ScenarioClass__Create_Houses` writes:

- `House+0x16058 = NodeNameTag__GetTeam()` near `0x006880F2`.
- `House+0x1605C = *(node + 0x63)` near `0x006880FC..0x00688101`.

For AI rows, it writes:

- `House+0x16058 = DAT_00A8B2DC[row]` near `0x006881EC`.
- `House+0x1605C = DAT_00A8B2FC[row]` near `0x006881F5..0x006881FB`.

Active in YR: Yes. `ScenarioClass__Full_Init @ 0x00686B20` calls `ScenarioClass__Create_Houses` in the non-campaign branch used by offline Skirmish.

### 3.2 First behavioral reader: Battle team/alliance callback

The standard Battle vtable at `0x007EE184` has:

- `+0x80 -> 0x005D6BE0`, explicit start preassignment.
- `+0x84 -> 0x005D6C70`, alternate/fallback start assignment helper.
- `+0x88 -> 0x005D74A0`, team/alliance adjunct consumer.

`0x005D74A0` loops over unordered house pairs. For each outer house it skips special houses and skips `House+0x1605C == -2` and `House+0x1605C == -1`. For each later house it again skips special houses, compares the two `House+0x1605C` values, and when they match it calls `HouseClass__MakeAlly @ 0x004F9B70` in both directions: outer -> inner and inner -> outer.

Tiny details:

- Pairing starts the inner loop at the next index (`edi = outer_index + 1`), so each unordered pair is considered once.
- Sentinel values `-2` and `-1` on the outer house suppress the entire inner scan for that outer house.
- The inner house is not separately tested for `-2`/`-1`; equality with the outer non-sentinel value is the gate, so sentinel matches cannot pass.
- Special houses are filtered before the compare on both sides via `HouseType+0x1A6`.
- The function returns true even when no houses exist or no pair matches.

Active in YR: Yes for standard Battle-style Skirmish mode objects when the mode `+0x88` callback is invoked. Evidence: vtable memory `0x007EE184 + 0x88 -> 0x005D74A0`, `ScenarioClass__Full_Init @ 0x00686B20` uses `DAT_00A8B23C` as the selected non-campaign mode object, and `HouseClass__MakeAlly` is live multiplayer/skirmish alliance code.

### 3.3 Contrast: `0x16058` is the start preassignment field

The standard Battle `+0x80` method at `0x005D6BE0` calls `ScenarioClass__Gather_Start_Positions @ 0x00688380`, loops houses, skips special houses, reads `House+0x16058`, skips `-2`, and writes the house index into `ScenarioClass+0x1180 + start_index*4`.

Active in YR: Yes. `ScenarioClass__Full_Init @ 0x00686B20` invokes selected mode vtable `+0x80` before `ScenarioClass__AssignStartingPoints @ 0x005EE9D0`.

## 4. INI Keys

No map/rules INI key directly writes `House+0x1605C` in this scoped path. The relevant UI/session source is the Skirmish slot/team value packed by the shell before `Create_Houses`.

Related but not direct:

| INI key | Default / location | Effect in this slice | Active in YR |
|---|---|---|---|
| `[MultiplayerDialogSettings] AlliesAllowed` | `no`, `ini/rulesmd.ini:3038` | Shell/game-mode option context; not the direct `0x1605C` field writer in this trace | Conditional; separate option plumbing |
| `[General] AllyReveal` | `yes`, `ini/rulesmd.ini:751` | Affects visibility consequences of alliances, not pair formation in `0x005D74A0` | Yes after alliances exist |

## 5. Integration Points

`House+0x1605C` is written during house creation after country/color/name setup and before observer/local-player post-writes. The alliance consumer is a mode lifecycle method, not the immediate start-preassignment method. The ally mutation itself is delegated to `HouseClass__MakeAlly @ 0x004F9B70`, which sets the ally bit for the other house, clears `EnemyHouseIndex` when needed, may recalculate alliances/EVA in live multiplayer contexts, and refreshes side effects with `FUN_004F42F0(0)`.

Active in YR: Yes. `HouseClass__MakeAlly` explicitly gates some side effects on `g_GameMode != 0`, `g_GameMode == 0`, `g_GameMode == 4`, and `g_MapEditorMode`, so this is not a TS-only dead path; the alliance bit mutation itself is unconditional after `HouseClass__Is_Enemy` passes.

## 6. Current Rust Implementation Status

Rust has UI state for opponent `team` values but does not currently feed a native-style house team/alliance pass:

| Rust surface | Current status | Native delta |
|---|---|---|
| `src/ui/skirmish_shell/state.rs:30..31`, `:58..63` | stores per-opponent `start_position` and `team` | team is UI state only in the scanned launch contract |
| `src/ui/skirmish_shell/state.rs:70..86` | `launch_settings` collapses shell state into `SkirmishSettings` | no per-house team adjunct array equivalent to `House+0x1605C` |
| `src/app_skirmish.rs:25..106` | seeds two MCVs by start waypoints | no pairwise same-team `HouseClass__MakeAlly` equivalent before gameplay |
| `src/sim/world/world_tests.rs:2333` | tests can set `sim.house_alliances` manually | not wired from Skirmish shell team selections |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Create_Houses` writes `0x1605C` | verified | `0x006880FC..0x00688101`, `0x006881F5..0x006881FB` | none |
| standard Battle vtable binding | verified | Ghidra memory at `0x007EE184`; `+0x88 -> 0x005D74A0` | none |
| `0x005D74A0` pairwise equality consumer | verified | Ghidra byte-pattern hit/read-memory plus retail disassembly at `0x005D74A0..0x005D7549` | none |
| `HouseClass__MakeAlly` mutation | verified | decompile `0x004F9B70` | deeper ally-reveal visual effects out of scope |
| `0x16058` start contrast | verified | disassembly `0x005D6BE0..0x005D6C2F`, prior report | none |
| `0x005C3220` alternate reader | touched-not-exhausted | Ghidra byte-pattern hit and disassembly `0x005C3220..0x005C34E0` | different mode callback; not standard Battle `0x1605C` equality consumer |
| serialization/debug readers | touched-not-exhausted | Ghidra byte-pattern hit `0x0064E266` | save/debug formatting context out of scope |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is House+0x1605C the standard Battle start preassignment field? -> No; standard Battle `+0x80` reads House+0x16058.` (evidence: `0x005D6C12`, `0x005D6C2F`)
- `[RESOLVED] OQ-2 - What first behavioral consumer reads House+0x1605C in the standard Battle family? -> The `+0x88` callback at 0x005D74A0 compares same non-sentinel values and mutual-allies matching houses.` (evidence: `0x007EE184 + 0x88`, `0x005D74D4..0x005D751A`)
- `[RESOLVED] OQ-3 - What values are ignored by the alliance pass? -> `-2` and `-1` suppress the outer-house compare; special houses are skipped on both sides.` (evidence: `0x005D74C6..0x005D74E2`, `0x005D74EB..0x005D74F6`)
- `[RESOLVED] OQ-4 - Does the consumer only set one-way alliance? -> No; it calls HouseClass__MakeAlly in both directions for each matching pair.` (evidence: `0x005D750B..0x005D751A`)
- `[RESOLVED] OQ-5 - Does the ally mutation path exist in YR, not just TS legacy? -> Yes; `HouseClass__MakeAlly` handles nonzero `g_GameMode` and multiplayer/skirmish side effects.` (evidence: `0x004F9B70`)
- `[DEFERRED] OQ-6 - Exact user-facing sentinel labels for the Team combo.` (category: out-of-scope; reason: requires dropdown population trace, not needed to identify the `0x1605C` consumer; next-step-if-pursued: audit Skirmish team combo population around control IDs `0x76D..0x774`)
- `[DEFERRED] OQ-7 - Whether every non-Battle mode using `0x005D74A0` has identical shell semantics.` (category: out-of-scope; reason: this slot is standard offline Skirmish/Battle-style only; next-step-if-pursued: enumerate all vtables containing `0x005D74A0`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Same non-sentinel team adjunct values on houses cause mutual alliances before normal play | `0x005D74A0`, `0x004F9B70` | missing | Skirmish launch/session packing and sim alliance initialization | Preserve per-slot team value separately from start position and create mutual alliances for matching team groups | Two Skirmish slots assigned the same Team are allied at game start; different teams remain enemies | Do not overload selected start waypoint as team/alliance data |
| Explicit start preassignment uses `House+0x16058`, not `0x1605C` | `0x005D6BE0..0x005D6C2F` | partial/mismatch: Rust has a simple local start swap | `src/app_skirmish.rs`, future house/session start table | Feed start choices into a start-preassignment table independently from team choices | Player choosing Start 2 and Team 1 starts at waypoint 2 and allies only with Team 1 peers | Do not let same-team values affect start slot assignment |
| `-2` and `-1` team adjunct values do not auto-ally through `0x005D74A0` | `0x005D74D4..0x005D74E2` | unchecked | future Skirmish team-combo/state mapping | Preserve sentinel handling when implementing team combo values | Team Auto/None sentinel rows do not accidentally ally all players | Do not treat every equal default/sentinel as a real team |

## Stale Docs / Follow-up Docs

- `SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md` should replace claims that "`NodeNameTag+0x63` / `House+0x1605C` is start location" with: "`House+0x16058` is the standard Battle explicit-start field; `House+0x1605C` is the team/alliance adjunct consumed by the Battle `+0x88` callback for same-team mutual alliances."
- `LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md` should split its mixed "Start position / Ally" wording: `+0x16058` is start preassignment for the verified Battle consumer, while `+0x1605C` is the alliance/team adjunct for this path.

## Sources

- Ghidra decompiled/read-only: `ScenarioClass__Create_Houses @ 0x00687F10`, `ScenarioClass__Full_Init @ 0x00686B20`, `HouseClass__MakeAlly @ 0x004F9B70`.
- Ghidra memory/search/read-only: byte pattern for displacement `0x1605C` at `0x005D74D6`, `0x005D74FF`, `0x005D7505`, `0x00688103`, `0x006881FD`; vtable memory at `0x007EE184`.
- Retail binary disassembly for functions Ghidra did not have as function bodies: `0x005D74A0..0x005D7549`, `0x005D6BE0..0x005D6C2F`, `0x005C3220..0x005C34E0`, `0x0064E21B..0x0064E2C4`.
- Prior docs checked: `skirmish-ui/SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`, `LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish.rs`, `src/sim/world/world_tests.rs`.
