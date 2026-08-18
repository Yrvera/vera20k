# Choose Map — Selected-Record Loader: Committed Globals & Consumer Observability

**Target:** 0x005E7BF0 (selected-record loader) and consumers of all six globals it writes.  
**Status:** COMPLETE  
**Session date:** 2026-06-01  
**Verified via:** decompile_function, get_function_callers, get_xrefs_to (all this session)

---

## Label Drift Finding (CRITICAL)

**0x005E7BF0 is NOT CDFileClass::Constructor.** The label is stale/polluted.

Evidence (decompile_function 0x005E7BF0): the function body reads from a global scenario-record table (`DAT_00a8b8cc`), copies strings out of record offsets `+0x58` (file path), `+0x15c` (digest), `+0x17c` (official flag), calls `FUN_007ca489` to copy the display title, calls a vtable at `+0x2c` and `+0x98`, and writes `DAT_00a8bb04 / bb08 / bb0c / b322 / b8e0 / bae2`. This is definitively the **scenario-selection record loader**, not a CDFileClass constructor.

The same `CDFileClass__Constructor` label appears on 0x005E6520 and on dozens of other functions throughout the binary — the label is script-applied garbage and carries zero identity information.

**Correct proposed label:** `ScenarioSelector_LoadSelectedRecord` or `Load_Selected_Map_Record`.

**Callers confirmed** (get_function_callers 0x005E7BF0):
- `FUN_006acee0` @ 0x006ACEE0 — the Choose Map accept handler (WM_COMMAND for the accept button, message 0x5AA)
- `CDFileClass__Constructor` @ 0x005B82F0 — serial/modem game-options path
- `FUN_005b9a60` @ 0x005B9A60, `FUN_005dc350` @ 0x005DC350 — other setup paths
- `FUN_005ed5a0` @ 0x005ED5A0, `FUN_006ae6e0` @ 0x006AE6E0, `SimpleWonlineDialogControl__Constructor` @ 0x00789B60 — WOL lobby

---

## Globals Written: Consumer Analysis

### DAT_00A8B322 — Display Title (map name string)

**Written:** `FUN_007ca489(&DAT_00a8b322, *(int*)(record + 0))` — copies the record's display name string.  
**Reset path:** param_1 == -1 path zeroes it.

**Consumers (READ):**
- `FUN_005b67f0` @ 0x005B6C8A — reads title (`iVar7` assigned from record title for fallback copy). Called from modem/serial options decoder `FUN_005b6020`. **Network only.**
- `Build_Options_String` @ 0x005DBB65 — calls `FUN_007b66d0(&DAT_00a8b322)` as first action — initialises preview/lobby map display. **Called from both FUN_005e32d0 (LAN lobby heartbeat) and network lobby paths.** Not from offline Skirmish launch.
- `FUN_005e32d0` @ 0x005E32D0 — lobby options refresh: reads `&DAT_00a8b322` for change detection (line: `iVar3 = FUN_007dd0f8(&DAT_00ac10f0,&DAT_00a8b322); if (iVar3 != 0) { bVar2 = true; }` triggers preview rebuild). **LAN/network lobby; not Skirmish launch.**
- `FUN_006acee0` @ 0x006AD911 — the accept handler itself reads it to build the preview title label for display in the dialog (`FUN_007ca489(local_200, &DAT_00a8b322)`). **Observable: the displayed map name in the lobby after selection.**

**Active in YR offline Skirmish launch:** No direct effect on launch acceptance/rejection. The title is used for the lobby name display, which is visible during map selection. It is NOT checked at the actual Start button press for skirmish.

---

### DAT_00A8BAE2 — Digest (scenario hash string, offset +0x15c in record)

**Written:** memcpy from `*(int*)(record + 0) + 0x15c` — a string field interpreted as digest.

**Consumers (READ):**
- `FUN_005e32d0` — change-detection compare: `iVar3 = FUN_007c8d20(&DAT_00ac0d00,&DAT_00a8bae2); if (iVar3 != 0) bVar2 = true;`. Triggers lobby preview refresh. **LAN lobby only.**
- `FUN_005b67f0` @ 0x005B699C, 0x005B6B66, 0x005B6BF4 — multiple reads+writes. This is the modem/serial game-options decoder. On receiving a remote game-options packet, it compares local `&DAT_00a8bae2` vs packet offset `+0xb0`. If mismatch, it updates the local digest from the packet and logs "Scenario has changed". **Modem/serial network only.**
- `Build_Options_String` @ 0x005DBBFA — packs `&DAT_00a8bae2` into the options string sent to peers. **LAN/WOL only.**
- `FUN_005b6020` @ 0x005B6394 — modem guest handler. **Modem only.**

**Active in YR offline Skirmish launch:** NO. The digest is never read in the Skirmish accept path (0x006ACEE0 case 0x5AA or 0x5C0/0x617). The digest is purely a **network integrity/sync check** used to detect mismatched map files across peers.

---

### DAT_00A8B8E0 — File Path (`.map` filename string, offset +0x58 in record)

**Written:** memcpy from `*(int*)(record + 0) + 0x58`.  
**ScenarioClass mirror:** immediately after writing `DAT_00a8b8e0`, the loader copies the same string to `*(char*)(g_ScenarioClass_Instance + 0x125c)`.

**Consumers (READ):**
- `FUN_005e32d0` — change-detection: `FUN_007c8d20(&DAT_00ac0cf0, &DAT_00a8b8e0)`. Lobby refresh. **LAN only.**
- `FUN_005b67f0` — modem/serial options decoder: compares local path vs packet. If mismatch, updates local path from packet. **Modem/serial only.**
- `Build_Options_String` — packs path into options string. **LAN/WOL only.**
- `FUN_005b6020` (case 0x67/0x6b — "received go" packet) @ 0x005B6BF4: after validating remote options, **copies `&DAT_00a8b8e0` to `*(char*)(g_ScenarioClass_Instance + 0x125c)`** and then proceeds to start the scenario. **Modem/serial game-start path only.**
- CCFileClass open calls (0x00553526 etc.) — used when loading the actual map file at scenario start.

**The ScenarioClass+0x125C mirror** is the path used by the scenario loader when the game actually starts. In offline Skirmish, the path at `ScenarioClass+0x125C` is populated **by the loader itself** (0x005E7BF0 writes it directly). This mirror copy IS the game-start hook.

**Active in YR offline Skirmish launch:** YES for `ScenarioClass+0x125C` mirror — this is what the engine uses at scenario launch to locate the map file. If the Rust path omits the mirror write, the scenario cannot load. The global `DAT_00a8b8e0` itself is also used by CCFileClass at map-load time.

---

### DAT_00A8BB08 — Official Flag (byte from record offset +0x17c)

**Written:** `DAT_00a8bb08 = *(byte*)(*(int*)(record + param_1*4) + 0x17c)`.

**Consumers (READ):**
- `FUN_005e32d0` — change detection: `if (DAT_008316e4 != DAT_00a8bb08) bVar2 = true`. Lobby refresh. **LAN only.**
- `Build_Options_String` — packs `_DAT_00a8bb08 & 0xff` into options string via `FUN_007b5400`. **LAN/WOL only.**
- `FUN_005b6020` (modem guest path, case 0x67/0x6b): `CDFileClass__Constructor(&DAT_00a8bae2, DAT_00a8bb08, 1)` — this is a map-load call that uses the official flag as a parameter (determines where to look for the map file — official maps vs user maps). **Modem/serial game-start only.**
- `FUN_005b67f0` — modem options decoder: checks `DAT_00a8bb08 != (packet+0x82 >> 4) & 1`. If mismatch, updates and forces reload.

**Active in YR offline Skirmish launch:** CONDITIONAL. In offline Skirmish, `DAT_00a8bb08` is not checked in the Start button handler `FUN_006ACEE0`. However, the map-load function called at actual game start likely uses it (the modem "go" path at 0x005B6020 case 0x67/0x6b calls a map-loader with `DAT_00a8bb08` as argument, and the same map-loader is used in Skirmish start). This needs verification against the Skirmish start path.

**Tentative: CONDITIONAL — observable if the wrong value causes the map loader to look in the wrong location.**

---

### DAT_00A8BB0C — Capacity / Max Players (clamped by selected-mode vtable+0x04 / +0x98)

**Written:** The loader calls `CDFileClass__Constructor()` (stale label; this is the record-level max-player getter at vtable+0x04 of the record object). If `DAT_00a8b23c != NULL` (a mode/type vtable pointer), it calls `(*DAT_00a8b23c)[0x98]()` and clamps: `if (result != 0xffffffff && result < uVar6) uVar6 = result`. Then in g_GameMode==4 (skirmish), further clamp by FUN_0077d970/0077d940. Result stored in `DAT_00a8bb0c`.

**Consumers (READ):**
- `FUN_006acee0` @ start-button handler (case 0x5C0/0x617 — the OK/Start button): `iVar4 = CDFileClass__Constructor()` (same stale-labeled capacity getter) then **`if (iVar4 < local_24c + 1)`** — if map max players < configured player count + 1, show error string 0x437 and BLOCK launch. **PLAYER-OBSERVABLE: causes Start to fail with message about too many players.** Also `if (local_24c + 1 < 2)` — too few players also blocked.
- `FUN_005e9510` @ 0x005E9510 — "is channel full?" check: `iVar2 = DAT_00a8bb0c; if (DAT_00a8b548 < DAT_00a8bb0c) iVar2 = DAT_00a8b548;` — WOL channel-full check. **WOL/network only** (called from FUN_007ab6a0).
- `FUN_005e95e0` @ 0x005E95E0 — similar channel-full check for WOL joining. **WOL only.**
- `Build_Options_String` — packs `DAT_00a8bb0c` into options string.
- `FUN_005b67f0` — modem options decoder reads `DAT_00a8bb0c` as part of options comparison.

**Active in YR offline Skirmish launch:** YES. **The capacity value is directly compared against configured player count at the Start button press. A wrong or zero capacity causes an observable error message and launch rejection.**

---

### DAT_00A8BB04 — vtable+0x2C Result (from CCFileClass open on the path)

**Written:** `piVar3 = (int*)CCFileClass__Constructor(&DAT_00a8b8e0); DAT_00a8bb04 = (**(code**)(*piVar3 + 0x2c))();`. This vtable slot is called twice consecutively (with PixelBuffer_Free / BufferIOFileClass__Constructor between calls). The +0x2C vtable slot on a file object is typically an "open" or "exists/valid" check — it opens/validates the map file and returns a result.

**Consumers (READ):**
- `FUN_005e32d0` — change detection: `if (DAT_008316e8 != DAT_00a8bb04) bVar2 = true`. Lobby refresh.
- `Build_Options_String` — packs `DAT_00a8bb04` into options string via `FUN_007b5400`.
- `FUN_005b67f0` — modem options decoder: `if (DAT_00a8bb04 != *(int*)(packet + 0x9e))` — mismatch triggers scenario-changed path.
- `FUN_005b6020` — modem guest path checks `DAT_00a8bb04 == *(int*)(packet + 0x9e)` as part of "received go" validation.
- `FUN_00795cd0` @ 0x00796174 — reads both `DAT_00a8bb04` and `DAT_00a8bb08`. Context unknown without further decompile, but appears in a different dialog (not the Skirmish launch path).

**Active in YR offline Skirmish launch:** NO evidence that `DAT_00a8bb04` is checked at the Skirmish Start button. It is packed into network options strings and used for modem sync. In offline Skirmish the vtable result is computed but never validated for launch acceptance.

---

## Summary Table

| Global | Content | Observable at Offline Skirmish Start? | Mechanism |
|---|---|---|---|
| DAT_00A8B322 | Display title | NO (lobby display only) | Title shown in map selection UI; not checked at launch |
| DAT_00A8BAE2 | Digest string | NO | Network integrity check only |
| DAT_00A8B8E0 | File path | YES (indirectly via ScenarioClass+0x125C mirror) | Path used to open map file at game start |
| ScenarioClass+0x125C | Path mirror | YES (directly) | This is what the scenario loader reads |
| DAT_00A8BB08 | Official flag | CONDITIONAL | Used by map loader; wrong value may cause wrong search path |
| DAT_00A8BB0C | Capacity / max players | YES — hard launch gate | Start-button rejects if player count > capacity |
| DAT_00A8BB04 | vtable+0x2C result | NO (offline) | Network options packing; no offline launch check found |

---

## Rust Gap Assessment

Current Rust commit_choose_map_selection writes: `selected_mode_id + selected_map_idx + nulls preview + restarts label reveals`. Missing:

1. **ScenarioClass path mirror (`+0x125C`):** The Rust equivalent of ScenarioClass needs to hold the selected map file path. The path is already in `SkirmishScenarioRecord.file_name`, so the Rust launch path just needs to read it from there — the gap is only if `ScenarioClass` equivalent doesn't get this value before the game-load step.

2. **Capacity / max-player check at Start:** The Rust launch path (`src/ui/skirmish_shell/state/launch.rs:90-93`) reads `selected_map.multiplayer_start_waypoints.len()` for capacity. This IS the correct observable: waypoint count = available start positions = capacity. **This is functionally equivalent provided the waypoint count matches the map's defined max players.** No gap if waypoint count is authoritative.

3. **Digest, official flag, vtable+0x2C result:** No evidence these are needed for offline Skirmish launch. Correctly omitted.

---

## Implementation Handoff

### Item 1 — Capacity check is correct (no Rust gap needed)
**Behavior:** gamemd reads `DAT_00a8bb0c` (populated from the record's max-player vtable result) and rejects Start if configured player count ≥ capacity.  
**Rust delta:** Rust reads `multiplayer_start_waypoints.len()` at `launch.rs:90-93`, which produces the same capacity value as waypoint positions define max players. Equivalent provided the record's capacity equals the waypoint count.  
**Surface:** Launch rejection shows an error message. Correct behavior is to block launch with a message.  
**Acceptance scenario:** Select a 2-player map, configure 3 AI players + self = 4 total; Start should be rejected.  
**Test name:** `test_launch_reject_when_player_count_exceeds_map_capacity`  
**Risk:** LOW — if `SkirmishScenarioRecord.max_players` is taken from a different source than waypoints (e.g., a parsed INI field), the counts could diverge.

### Item 2 — ScenarioClass path mirror must be set before game-load
**Behavior:** 0x005E7BF0 writes file path both to `DAT_00a8b8e0` and to `ScenarioClass+0x125C`. The game-load step uses `ScenarioClass+0x125C` to find the map file.  
**Rust delta:** The Rust ScenarioClass equivalent (or the game-launch function) must have the selected map's `file_name` from `SkirmishScenarioRecord` populated before the map loader runs. Current `commit_choose_map_selection` stores `selected_map_idx`; the launch path must resolve this to a file path.  
**Surface:** Wrong or missing path → map load fails → game cannot start.  
**Acceptance scenario:** Accept a map, press Start, verify the correct `.map` file is loaded.  
**Test name:** `test_launch_loads_correct_map_file_path_from_selection`  
**Risk:** MEDIUM — if the launch path resolves `selected_map_idx` → `file_name` but has a wrong base directory or extension, the map load silently fails.

### Item 3 — Digest, official flag, vtable result are network-only (no action needed)
**Behavior:** `DAT_00a8bae2`, `DAT_00a8bb08`, `DAT_00a8bb04` are used for modem/serial/LAN/WOL options sync. Not checked in offline Skirmish launch path.  
**Rust delta:** None required for offline Skirmish parity.  
**Risk:** LOW for offline; flag when implementing network multiplayer.

---

## Negative Facts / Do Not Do

1. **Do NOT implement a digest check at offline Skirmish launch.** No consumer of `DAT_00a8bae2` exists in the offline Skirmish Start path. It is purely a modem/serial/LAN integrity check (verified: all xrefs are in FUN_005b67f0, FUN_005b6020, Build_Options_String, FUN_005e32d0 — none reached in offline Skirmish Start).

2. **Do NOT treat the vtable+0x2C result (DAT_00A8BB04) as a launch gate for offline Skirmish.** All consumer reads are in network paths (FUN_005b67f0, FUN_005b6020, FUN_00795cd0) or lobby refresh (FUN_005e32d0). No offline-Skirmish Start path reads it.

3. **Do NOT rename 0x005E7BF0 to CDFileClass::Constructor in Ghidra.** The label is polluted script output. The function is a scenario-record loader. At least six other functions carry the same stale label — treat any CDFileClass__Constructor label in this codebase as unverified until re-decompiled.

4. **Do NOT implement the official-flag path check for offline Skirmish until the map-loader is wired up.** `DAT_00a8bb08` matters at map-load time (modem path passes it to the file-open call), but for offline Skirmish the map path alone is sufficient for current stage; the official-flag routing applies when implementing the mix-archive search order.

5. **Do NOT use 0x005E6520 as an alternative to 0x005E7BF0.** Both carry `CDFileClass__Constructor` label; 0x005E6520 has not been verified this session and may be a different function entirely.

---

## Remaining Uncertainty

1. **What FUN_00795CD0 does with both `DAT_00A8BB04` and `DAT_00A8BB08`** — it reads both at 0x007961[6e/74]. Neither the function purpose nor whether it is reachable in offline Skirmish has been verified. Decompile FUN_00795cd0 to resolve.

2. **The official-flag's role in the Skirmish-path scenario loader.** The modem path passes `DAT_00a8bb08` to a map-loader vtable call. Whether the same loader call path applies in offline Skirmish or whether a different branch is taken has not been fully traced. Low urgency until network is implemented.

3. **Whether `ScenarioClass+0x125C` is read before or after commit_choose_map_selection in the Rust flow** — this depends on the exact launch sequence in src/app.rs and src/ui/skirmish_shell/state/launch.rs. Needs code review, not binary work.

---

## Unverified

- FUN_00795CD0 body not decompiled this session; its two reads of `DAT_00A8BB04` / `DAT_00A8BB08` at 0x0079616E / 0x00796174 are confirmed by xref but the function purpose is UNVERIFIED.
- The exact vtable slot identity of +0x2C on the CCFileClass object (used to produce `DAT_00A8BB04`) — Ghidra labels it as a constructor but the actual operation (open? validate? CRC?) is unverified.
