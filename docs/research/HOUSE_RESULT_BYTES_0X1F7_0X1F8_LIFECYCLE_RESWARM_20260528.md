# House Result Bytes +0x1F7/+0x1F8 Lifecycle - Reswarm 2026-05-28

**Address(es):** `HouseClass::Update @ 0x004F83F0`, `HouseClass::Flag_To_Win_Check @ 0x004FC980`, `HouseClass::Flag_To_Win @ 0x004FC9E0`, `HouseClass::Flag_To_Lose @ 0x004FCBD0`, `Check_Win_Condition @ 0x004FCDC0`, `MPlayer_Defeated @ 0x004FC0B0`, trigger/script callers in `TriggerAction::Execute @ 0x006DD8B0` and `TeamClass::Recruit_Or_Add @ 0x006E93F0`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** Direct lifecycle of `HouseClass+0x1F7` and `HouseClass+0x1F8` as result bytes: constructor defaults, normal setter helpers, direct clears, direct readers, and `HouseClass::Update` mapping to the already-settled late frame gate globals.
**Non-Scope:** Full victory/defeat UI, complete trigger/action taxonomy, full multiplayer victory system, savegame raw persistence, and already-settled `Main_Tick` four-global late gate behavior.
**Confidence:** High for constructor/init, helper semantics, known helper callers, `HouseClass::Update` result-to-global bridge, and standard YR activity. Medium for "all raw field xrefs" because this MCP exposes function xrefs but not a direct data-reference census for `+0x1F7/+0x1F8`.
**Active in YR:** Yes. The constructor and `HouseClass::Update` are normal house lifecycle/tick paths; helper callers include standard trigger/script/game-completion routes.

## 0. Investigation Contract

**Target question:** Identify all upstream writers and readers of `HouseClass+0x1F7` and `HouseClass+0x1F8` that feed victory/defeat/session-end late-frame increment gates.

**Non-goals:** Do not rediscover the `Main_Tick` late gate; do not redo the full victory system; do not patch Rust or synthesis docs.

**Evidence needed to mark COMPLETE:** Decompile plus assembly for constructor defaults, `Flag_To_Win`, `Flag_To_Lose`, `Flag_To_Win_Check`, `Check_Win_Condition`, `HouseClass::Update`, helper caller xrefs, and the exact `HouseClass::Update` writes to `DAT_00A83D49` / `DAT_00A8ECD0`.

**Stop conditions:** Stop after the direct lifecycle and Rust-facing handoff are proven, a zero-add pass over the helper callers and `HouseClass::Update` adds no new in-scope question, and this report is written to the required path.

## 1. Overview

`House+0x1F7` is the per-house "has won / victory result pending" byte and `House+0x1F8` is the per-house "has lost / defeat result pending" byte. They are initialized clear, set by result helpers or campaign/team/trigger callers, held while borrowed time/EVA wait completes, then consumed in `HouseClass::Update`.

The late native frame gate does not read these house fields directly. `HouseClass::Update` reads `+0x1F7` and `+0x1F8`; once each result's remaining time reaches zero, it writes one of the session-end globals: `DAT_00A83D49` for victory/session-win route or `DAT_00A8ECD0` for defeat/session-loss route. Those globals are the already-settled `Main_Tick` late frame increment suppressors.

## 2. Key Offsets

| Offset | Type | Meaning in this slice | Default / writer | Active in YR |
|---:|---|---|---|---|
| `House+0x1F5` | byte | defeated marker set by `MPlayer_Defeated` | set at `0x004FC0C2` | Yes |
| `House+0x1F6` | byte | win-pending/scatter phase | constructor clear; `Flag_To_Win_Check` set; `HouseClass::Update` clear | Yes |
| `House+0x1F7` | byte | win result byte | constructor clear; `Flag_To_Win` set; `Flag_To_Lose` and `HouseClass::Update` clear | Yes |
| `House+0x1F8` | byte | loss result byte | constructor clear; `Flag_To_Lose` set; no normal helper clear found after constructor | Yes |
| `House+0x298` | int | win/loss start frame | set by result helpers/checker; read by `Update` | Yes |
| `House+0x2A0` | int | borrowed-time duration | set by result helpers/checker; read by `Update` | Yes |

Constructor default proof: full constructor `0x004F54A0` clears `+0x1F5`, `+0x1F6`, `+0x1F7`, `+0x1F8` in a contiguous byte run. Assembly around `0x004F571C..0x004F572E` writes `BL` (zero) to all four bytes. Active in YR: Yes, standard HouseClass construction.

## 3. Core Lifecycle

### 3.1 Set win byte

`HouseClass::Flag_To_Win @ 0x004FC9E0` only enters if all three result bytes are clear:

```text
if House+0x1F7 == 0
and House+0x1F6 == 0
and House+0x1F8 == 0:
    House+0x1F7 = 1
    if skip_borrowed_time == 0:
        House+0x298 = g_CurrentFrameCounter
        House+0x2A0 = borrowed time
return House+0x1F7
```

Evidence: decompile `0x004FC9E0`; assembly `0x004FC9EE` reads `+0x1F7`, `0x004FC9FC` reads `+0x1F6`, `0x004FCA0A` reads `+0x1F8`, and `0x004FCA1C` writes `+0x1F7 = 1`. Active in YR: Yes. Callers include `MPlayer_Defeated`, `Check_Win_Condition`, campaign/team trigger actions, and script action case 7.

Tiny detail: `Flag_To_Win` refuses to set `+0x1F7` if loss `+0x1F8` is already set. This makes an existing loss terminal for the win setter until some other path clears state; no normal clear of `+0x1F8` was found in this slice.

### 3.2 Set loss byte and clear win byte

`HouseClass::Flag_To_Lose @ 0x004FCBD0` clears `+0x1F7` before checking pending or existing loss:

```text
House+0x1F7 = 0
if House+0x1F6 != 0:
    return House+0x1F8
if House+0x1F8 == 0:
    House+0x1F8 = 1
    if skip_borrowed_time == 0:
        House+0x298 = g_CurrentFrameCounter
        House+0x2A0 = borrowed time
return House+0x1F8
```

Evidence: decompile `0x004FCBD0`; assembly `0x004FCBDE` reads `+0x1F6`, `0x004FCBE4` clears `+0x1F7`, `0x004FCBED` reads `+0x1F8`, `0x004FCC05` writes `+0x1F8 = 1`, and `0x004FCDB2` returns `+0x1F8`. Active in YR: Yes. Callers include `MPlayer_Defeated`, trigger actions, and script action case `0x17`.

Tiny detail: losing while `+0x1F6` is set clears `+0x1F7` but does not set `+0x1F8`; it returns current loss state immediately. Do not collapse "Flag_To_Lose called" into "loss byte set."

### 3.3 Pending win checker

`HouseClass::Flag_To_Win_Check @ 0x004FC980` is a soft/pending win helper. It reads `+0x1F7`, `+0x1F6`, `+0x1F8`; if none are set, it sets only `+0x1F6 = 1`, records start frame, writes borrowed time zero, and returns `+0x1F6`. It does not set `+0x1F7` or `+0x1F8`.

Evidence: decompile `0x004FC980`; assembly `0x004FC980` reads `+0x1F7`, `0x004FC98B` reads `+0x1F6`, `0x004FC997` reads `+0x1F8`, and `0x004FC9A2` writes `+0x1F6 = 1`. Function xrefs: `FUN_0064C380`, `FUN_005DA750` twice, and `EventClass::Execute @ 0x004C7BE1` in REMOVEPLAYER/player-control paths. Active in YR: Conditional, reached by trigger/event victory checks.

### 3.4 Check-win condition reader

`FUN_004FCDC0` reads `+0x1F8` first and `+0x1F7` second. If both are zero, it calls `Flag_To_Win(0)`. If either is nonzero and `+0x298 == -1`, it repairs `+0x298` by writing `g_CurrentFrameCounter`.

Evidence: decompile `0x004FCDC0`; assembly `0x004FCDC0` reads `+0x1F8`, `0x004FCDCA` reads `+0x1F7`, `0x004FCDD6` calls `Flag_To_Win(0)`, and `0x004FCDDC..0x004FCDEA` repairs `+0x298`. Active in YR: Yes, called by `TriggerAction::Execute` case `0x45`.

### 3.5 House update result-to-global bridge

`HouseClass::Update @ 0x004F83F0` reads win byte first, then loss byte:

1. If `+0x1F7 != 0`, compute remaining borrowed time from `+0x298/+0x2A0`.
2. If remaining is nonzero, skip to the loss-byte check without setting a global.
3. If remaining is zero and campaign special-count check allows ending, pump EVA/audio for up to `0x78` radar-timer units while servicing network.
4. Clear `+0x1F7 = 0`.
5. Map the resolved win to `DAT_00A83D49` or `DAT_00A8ECD0` depending on campaign/multiplayer/local/co-op relation.
6. Then read `+0x1F8`.
7. If `+0x1F8 != 0` and remaining reaches zero, run the same EVA/network wait pattern and map resolved loss to `DAT_00A8ECD0` or inverse `DAT_00A83D49`.

Evidence: decompile `0x004F83F0`; assembly `0x004F86FE` reads `+0x1F8`; `0x004F867C`, `0x004F8692`, `0x004F86EE`, and `0x004F87BB` write `DAT_00A83D49 = 1`; `0x004F86F7`, `0x004F879C`, and `0x004F87B2` write `DAT_00A8ECD0 = 1`. Active in YR: Yes, standard house update.

Tiny detail: win processing clears `+0x1F7` after the borrowed-time/EVA wait expires, before setting the session-end global. Loss processing does not clear `+0x1F8` in this function; after it resolves, the session-end global is set and the session handler takes over.

Tiny detail: the mapping is not simply "win byte -> victory global, loss byte -> defeat global." For non-local/non-player campaign and non-local multiplayer cases, the resolved branch can set the inverse global. The correct user-visible route is relative to local/player/session completion, not merely the house whose result byte was set.

## 4. Upstream Callers That Set Or Gate These Bytes

Function-xref evidence for helper callers:

| Helper | Xref callers | Result | Active in YR |
|---|---|---|---|
| `Flag_To_Win` | `TriggerAction::Execute @ 0x006DDD7F`, `0x006DEA53`, `0x006DEA83`; `Check_Win_Condition @ 0x004FCDD6`; `FUN_006EDE40`; `MPlayer_Defeated @ 0x004FC6B7`; `TeamClass::Recruit_Or_Add @ 0x006E98D1` | may set `+0x1F7 = 1` | Conditional, trigger/script/game-completion paths |
| `Flag_To_Lose` | `TriggerAction::Execute @ 0x006DDD9B`; `FUN_006EDE60`; `MPlayer_Defeated @ 0x004FC68B`; `TeamClass::Recruit_Or_Add @ 0x006E98ED` | always clears `+0x1F7`; may set `+0x1F8 = 1` | Conditional, trigger/script/game-completion paths |
| `Flag_To_Win_Check` | `FUN_0064C380`, `FUN_005DA750` twice, `EventClass::Execute @ 0x004C7BE1` | reads `+0x1F7/+0x1F8`; sets `+0x1F6` only | Conditional, event/trigger pending-win paths |

`TriggerAction::Execute` active cases visible in decompile:

| Case | Behavior | Evidence | Active in YR |
|---:|---|---|---|
| `1` / `2` | conditional win/loss by player country/type comparison, calls `Flag_To_Win(0)` or `Flag_To_Lose(0)` | decompile `0x006DD8B0`; call xrefs `0x006DDD7F`, `0x006DDD9B` | Conditional: map trigger actions |
| `0x43` | force win, calls `Flag_To_Win(1)` bypassing borrowed-time calculation | decompile `0x006DD8B0`; call xref `0x006DDD7F` context shows `PUSH 0x1` | Conditional: map trigger action |
| `0x44` | force lose, calls `Flag_To_Lose(1)` bypassing borrowed-time calculation | decompile `0x006DD8B0`; call xref `0x006DDD9B` context shows `PUSH 0x1` | Conditional: map trigger action |
| `0x45` | calls `Check_Win_Condition` | decompile `0x006DD8B0`; `FUN_004FCDC0` | Conditional: map trigger action |

`TeamClass::Recruit_Or_Add` active script cases:

| Case | Behavior | Evidence | Active in YR |
|---:|---|---|---|
| `7` | calls `Flag_To_Win(0)` on `g_PlayerPtr`, marks script action complete byte `+0x80 = 1` | decompile `0x006E93F0`; assembly `0x006E98C9..0x006E98D6` | Conditional: team script action |
| `0x17` | calls `Flag_To_Lose(0)` on `g_PlayerPtr`, marks script action complete byte `+0x80 = 1` | decompile `0x006E93F0`; assembly `0x006E98E5..0x006E98F2` | Conditional: team script action |

`FUN_006EDE40` / `FUN_006EDE60` wrappers call `Flag_To_Win(0)` / `Flag_To_Lose(0)` respectively on `g_PlayerPtr`, then write caller object `+0x80 = 1`. Active in YR: Conditional; these are script/action wrappers, not the `Main_Tick` globals.

## 5. No INI Keys

No `rulesmd.ini`, `rules.ini`, `artmd.ini`, or `art.ini` key directly configures `House+0x1F7` or `House+0x1F8`. They are engine state bytes. Trigger/script actions in maps can call the helper functions indirectly, but the bytes are not INI-configured defaults.

Active in YR: Yes for engine hardcoded lifecycle; conditional for map trigger/script paths.

## 6. Current Rust Implementation Status

Rust already has house-level `is_defeated`, `has_won`, and `has_lost` booleans in `src/sim/house_state.rs:31..36`, initialized false at `src/sim/house_state.rs:67..69`.

Rust defeat/victory resolution currently lives in `Simulation::check_defeat` at `src/sim/world/mod.rs:704..766` and is called late in `Simulation::tick` at `src/sim/world/mod.rs:1810..1815`. It sets `is_defeated` and `has_won` directly. It does not model:

- `Flag_To_Win` guard on `has_won == 0 && flag_to_win_pending == 0 && has_lost == 0`.
- `Flag_To_Lose` clearing `has_won` before checking pending/loss.
- `flag_to_win_pending` (`House+0x1F6`) and its scatter phase.
- borrowed-time start/duration fields (`+0x298/+0x2A0`) for result byte expiry.
- `HouseClass::Update` conversion of resolved result bytes into typed session-end globals before late frame commit.
- inverse local/session mapping where a non-local result can drive the opposite victory/defeat global.

`src/app_sim_tick.rs:151..159` still uses app pause/run_sim gating rather than the native per-session result-byte-to-late-gate bridge.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Full House constructor defaults for `+0x1F7/+0x1F8` | verified | decompile `0x004F54A0`; assembly `0x004F571C..0x004F572E` | none for defaults |
| `Flag_To_Win` reads/writes | verified | decompile `0x004FC9E0`; assembly `0x004FC9EE..0x004FCA1C` | none |
| `Flag_To_Lose` reads/writes | verified | decompile `0x004FCBD0`; assembly `0x004FCBDE..0x004FCC05`, `0x004FCDB2` | none |
| `Flag_To_Win_Check` reads | verified | decompile `0x004FC980`; assembly `0x004FC980..0x004FC9A2` | none |
| `Check_Win_Condition` reader/repair | verified | decompile `0x004FCDC0`; assembly `0x004FCDC0..0x004FCDEA` | none |
| `HouseClass::Update` result processing | verified | decompile `0x004F83F0`; assembly writer sites `0x004F867C`, `0x004F8692`, `0x004F86EE`, `0x004F86F7`, `0x004F879C`, `0x004F87B2`, `0x004F87BB` | complete for result-to-global bridge |
| Helper caller census by function xrefs | verified | `get_function_xrefs` for three helpers | raw data-xref census unavailable through current MCP |
| `MPlayer_Defeated` result-helper calls | verified | decompile `0x004FC0B0`; xrefs `0x004FC6B7`, `0x004FC68B` | full victory decision tree out-of-scope |
| `TriggerAction::Execute` result-helper calls | verified | decompile `0x006DD8B0`; xrefs around `0x006DDD7F`, `0x006DDD9B` | full trigger action taxonomy out-of-scope |
| `TeamClass::Recruit_Or_Add` result-helper calls | verified | decompile `0x006E93F0`; xrefs `0x006E98D1`, `0x006E98ED` | full script action taxonomy out-of-scope |
| Save/load raw persistence of these bytes | verified by follow-up | `HOUSE_RESULT_BYTES_SAVE_LOAD_PERSISTENCE_1F6_1F7_1F8_GHIDRA_REPORT.md`: HouseClass IPersist load/save uses AbstractClass raw body serialization; size virtual returns `0x160B8`; load-specific constructor does not reset `+0x1F6/+0x1F7/+0x1F8` or borrowed-time fields in the verified range | none for scoped fields |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-HRB-001 - What are the target fields? -> `House+0x1F7` is win result; `House+0x1F8` is loss result.` (evidence: helper decompiles `0x004FC9E0`, `0x004FCBD0`)
- `[RESOLVED] OQ-HRB-002 - What are constructor defaults? -> both bytes default to zero in the full constructor.` (evidence: assembly `0x004F5728`, `0x004F572E`)
- `[RESOLVED] OQ-HRB-003 - Who sets `+0x1F7`? -> `Flag_To_Win` sets it to one after checking `+0x1F7/+0x1F6/+0x1F8` are all zero.` (evidence: decompile `0x004FC9E0`; assembly `0x004FCA1C`)
- `[RESOLVED] OQ-HRB-004 - Who clears `+0x1F7`? -> `Flag_To_Lose` clears it immediately, and `HouseClass::Update` clears it after win borrowed-time expiry before writing a session-end global.` (evidence: `0x004FCBE4`; decompile `0x004F83F0`)
- `[RESOLVED] OQ-HRB-005 - Who sets `+0x1F8`? -> `Flag_To_Lose` sets it if `+0x1F6 == 0` and `+0x1F8 == 0`.` (evidence: decompile `0x004FCBD0`; assembly `0x004FCC05`)
- `[RESOLVED] OQ-HRB-006 - Is there a direct clear of `+0x1F8` during normal result processing? -> None found in the helper/update slice; constructor clears it, `HouseClass::Update` resolves it to globals without clearing it.` (evidence: decompile `0x004FCBD0`, `0x004F83F0`)
- `[RESOLVED] OQ-HRB-007 - Does `Flag_To_Win_Check` set either result byte? -> No, it reads both and sets only pending byte `+0x1F6`.` (evidence: `0x004FC980..0x004FC9A2`)
- `[RESOLVED] OQ-HRB-008 - Does `Check_Win_Condition` set a result byte directly? -> No direct field write; it calls `Flag_To_Win(0)` only if both result bytes are zero, otherwise it may repair start frame.` (evidence: `0x004FCDC0..0x004FCDEA`)
- `[RESOLVED] OQ-HRB-009 - When does `HouseClass::Update` read win/loss bytes? -> Early in the house tick, before power clamping, defeat detection, and AI production choices.` (evidence: decompile ordering `0x004F83F0`)
- `[RESOLVED] OQ-HRB-010 - How does win byte feed the late frame gate? -> after remaining time reaches zero, `HouseClass::Update` may set `DAT_00A83D49` or `DAT_00A8ECD0` depending on local/session context.` (evidence: writer assembly `0x004F867C`, `0x004F8692`, `0x004F86EE`, `0x004F86F7`)
- `[RESOLVED] OQ-HRB-011 - How does loss byte feed the late frame gate? -> after remaining time reaches zero, `HouseClass::Update` sets defeat global for local/current player and victory global for inverse non-local route.` (evidence: writer assembly `0x004F879C`, `0x004F87B2`, `0x004F87BB`)
- `[RESOLVED] OQ-HRB-012 - Are these paths standard YR active? -> Yes for constructor/update; conditional for triggers/scripts/game completion.` (evidence: decompile and helper xrefs)
- `[RESOLVED] OQ-HRB-013 - Is ordinary pause/menu involved? -> No; result bytes are house/session result state, not pause state.` (evidence: no pause/menu reads in helper/update decompiles; late gate doc proves pause is separate)
- `[RESOLVED_BY_FOLLOWUP] OQ-HRB-014 - Are the bytes raw-serialized in save/load? -> Yes. HouseClass IPersist save/load raw-writes/raw-reads the full `0x160B8` HouseClass body through AbstractClass serialization, so `+0x1F6/+0x1F7/+0x1F8` and borrowed-time fields `+0x298/+0x2A0` persist byte-for-byte. The load-specific constructor `0x004F5190` and House load fixups do not reset them in the verified range.` (evidence: `HOUSE_RESULT_BYTES_SAVE_LOAD_PERSISTENCE_1F6_1F7_1F8_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-HRB-015 - Is there a complete raw data-xref census for every field access?` (category: `bounded-cost-too-high`; reason: current MCP exposes function xrefs but not direct data xrefs for struct offsets; next-step-if-pursued: run a Ghidra data-reference script or memory-pattern search against `+0x1F7/+0x1F8`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Flag_To_Win` sets `+0x1F7` only when `+0x1F7`, `+0x1F6`, and `+0x1F8` are all zero. Active in YR: Yes. | `0x004FC9E0`; assembly `0x004FC9EE..0x004FCA1C` | missing guard; Rust sets `has_won` directly in `check_defeat` | `src/sim/house_state.rs`, `src/sim/world/mod.rs::check_defeat`, future native result helper | add native-equivalent result transition or helper so prior loss/pending win blocks a new win exactly as gamemd does | try to win a house while `has_lost` is already set; native-equivalent state must keep win false and leave loss true | proposed test `flag_to_win_ignored_when_loss_or_pending_result_set`; risk: direct `has_won=true` bypasses byte lifecycle |
| `Flag_To_Lose` clears `+0x1F7` first, then if `+0x1F6 != 0` returns without setting `+0x1F8`; otherwise sets `+0x1F8` if not already set. Active in YR: Yes. | `0x004FCBD0`; assembly `0x004FCBDE..0x004FCC05` | missing; Rust has no pending win byte and does not model loss override order | `src/sim/house_state.rs`, result transition helpers, trigger/runtime result actions | implement loss transition order as clear-win -> pending check -> set-loss, not a single enum swap | call lose while win-pending is set; win byte clears, loss byte remains false, pending remains for update processing | proposed test `flag_to_lose_clears_win_but_does_not_set_loss_when_win_pending`; risk: modeling as mutually exclusive enum loses the transient branch |
| `HouseClass::Update` converts expired result bytes to session-end globals and only then the already-settled late frame gate skips `g_CurrentFrameCounter++`. Active in YR: Yes. | `HouseClass::Update @ 0x004F83F0`; writer assembly `0x004F867C`, `0x004F86F7`, `0x004F879C`, `0x004F87BB`; late gate doc `MAIN_TICK_LATE_FRAME_INCREMENT_GATE_GLOBALS_RESWARM_20260528.md` | missing; Rust has `has_won/has_lost` but no borrowed-time expiry and no typed late frame stop reason | `src/sim/world/mod.rs::advance_tick`, `src/app_sim_tick.rs`, future native frame-clock/session-end state | result bytes must expire inside house update, set typed victory/defeat stop reason, and suppress late frame commit on that same tick | set a local player loss byte with remaining time zero before house update; update sets defeat stop reason and late frame remains `N` | proposed test `expired_house_loss_sets_late_defeat_gate_before_frame_commit`; risk: ending immediately at `check_defeat` or app pause gate creates wrong frame order |

### Stale Docs / Follow-up Docs

- `docs/research/MAIN_TICK_LATE_FRAME_INCREMENT_GATE_GLOBALS_RESWARM_20260528.md`: replace "exact upstream lifecycle for `House+0x1F7` / `House+0x1F8` result bytes is out-of-scope" with: "`House+0x1F7` is set by `Flag_To_Win` only when win/pending/loss bytes are all zero, cleared by `Flag_To_Lose` and by `HouseClass::Update` after win borrowed-time expiry; `House+0x1F8` is set by `Flag_To_Lose` after clearing win and only when pending win is clear; `HouseClass::Update` maps expired result bytes into `DAT_00A83D49` / `DAT_00A8ECD0` before the late Main_Tick frame gate."
- `docs/research/TIMING_SCHEDULER_TICK_SPINE_SYSTEM_MODEL_SYNTHESIS.md`: add under doc-patch-ready facts: "The victory/defeat late frame gate globals are downstream of per-house result bytes, not direct trigger results: `HouseClass::Update` consumes `House+0x1F7/+0x1F8` after borrowed-time expiry and writes the session-end globals."

## 10. Negative Facts / Do Not Do

- Do not model `House+0x1F7` / `House+0x1F8` as the `Main_Tick` late gate bytes themselves. Active in YR: No; evidence: `Main_Tick` reads globals, while `HouseClass::Update` maps house bytes to globals.
- Do not let `Flag_To_Win` set win after loss or pending win. Active in YR: No; evidence: `0x004FC9EE..0x004FCA12` requires all three bytes clear.
- Do not let `Flag_To_Lose` always set loss. Active in YR: No; evidence: it clears win, then exits early if `+0x1F6 != 0` at `0x004FCBDE..0x004FCBF3`.
- Do not assume `+0x1F7` maps only to victory global or `+0x1F8` maps only to defeat global. Active in YR: No; evidence: `HouseClass::Update` has inverse branch writes to `DAT_00A8ECD0` from win processing and `DAT_00A83D49` from loss processing depending on local/session context.
- Do not implement these bytes as INI options. Active in YR: No; evidence: hardcoded constructor/helper/update state; no relevant rules/art key found.

## 11. Remaining Uncertainty

- Full raw data-reference census for `House+0x1F7/+0x1F8` was not available through the current read-only MCP tool surface. Function xrefs to the helper writers were drained, and all known lifecycle owners from prior docs were decompiled.
- Save/load persistence semantics for these exact bytes are proven by follow-up `HOUSE_RESULT_BYTES_SAVE_LOAD_PERSISTENCE_1F6_1F7_1F8_GHIDRA_REPORT.md`: `+0x1F6/+0x1F7/+0x1F8` and `+0x298/+0x2A0` persist byte-for-byte through HouseClass raw-body save/load.
- Full trigger/script action taxonomy is out-of-scope; this report only classifies cases that call the result helpers.

## Sources

- Read-only Ghidra decompile: `HouseClass::Constructor @ 0x004F54A0`, simple constructor `0x004F5190`, `HouseClass::Update @ 0x004F83F0`, `HouseClass::Flag_To_Win_Check @ 0x004FC980`, `HouseClass::Flag_To_Win @ 0x004FC9E0`, `HouseClass::Flag_To_Lose @ 0x004FCBD0`, `FUN_004FCDC0`, `HouseClass::MPlayer_Defeated @ 0x004FC0B0`, `TriggerAction::Execute @ 0x006DD8B0`, `TeamClass::Recruit_Or_Add @ 0x006E93F0`, `FUN_006EDE40`, `FUN_006EDE60`.
- Read-only Ghidra function xrefs: `HouseClass__Flag_To_Win`, `HouseClass__Flag_To_Lose`, `HouseClass__Flag_To_Win_Check`.
- Read-only Ghidra assembly context: constructor `0x004F571C..0x004F572E`; helper field accesses `0x004FC980..0x004FCDEA`; `HouseClass::Update` global writer sites `0x004F867C`, `0x004F8692`, `0x004F86EE`, `0x004F86F7`, `0x004F879C`, `0x004F87B2`, `0x004F87BB`; script/action callers `0x006DDD7F`, `0x006DDD9B`, `0x006E98D1`, `0x006E98ED`, `0x006EDE4B`, `0x006EDE6B`.
- Prior docs: `docs/research/MAIN_TICK_LATE_FRAME_INCREMENT_GATE_GLOBALS_RESWARM_20260528.md`, `docs/research/DEFEAT_WIN_LOSS_SYSTEM_GHIDRA_REPORT.md`, `docs/research/MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md`, `docs/research/HOUSECLASS_GHIDRA_REPORT.md`.
- Follow-up save/load doc: `docs/research/HOUSE_RESULT_BYTES_SAVE_LOAD_PERSISTENCE_1F6_1F7_1F8_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/house_state.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`.
