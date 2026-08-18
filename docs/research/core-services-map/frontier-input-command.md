# frontier-input-command — Keyboard/command + mouse action dispatch (substrate profile)

**Slug:** `frontier-input-command`
**Status:** PROMOTED from catalog stub (was `_frontier.md` §I1). Profile-level, not a full
class decode — verifies the representative address + the cross-service edges only.
**Authority order:** binary → Ghidra → docs.
**Active in YR:** YES — the entire input chain (keyboard dispatch + mouse action resolve +
cursor update) runs every frame of active gameplay (`Main_Tick` gameplay block, modes 0/3/4/5).
Only chat/beacon/alliance sub-paths are MP-specific; the core hotkey + click resolver is universal.

> ⚠️ **Ghidra connectivity note (this session):** The Ghidra MCP bridge was **not reachable**
> for the duration of this promotion (`list_instances` → `{"instances": []}`; `connect_instance`
> → UDS 0 found, TCP `127.0.0.1:8089` actively refused — `[WinError 10061]`; repeated retries).
> The representative address and every cross-service edge below are therefore **corroborated
> against multiple independent `[ghidra/verified]` docs already in the research corpus**, not a
> fresh live decompile this session. The representative address `0x0055DEE0` is confirmed by
> **three independent docs** (`HOTKEY_SYSTEM` decompiles it as `FUN_0055dee0`; `ADDRESS_MAP`
> and the spine spec both list it) — strong multi-doc agreement. Deeper offsets carry their
> source docs' grade. Items needing a *live* re-pull are flagged **NEEDS-LIVE-REVERIFY**.
> Per RE discipline, no address below is claimed "re-verified live this session"; the honest
> status is **LOCATED / docs-corroborated**.

---

## 1. Purpose

The **front of the input chain** — the out-of-sim stage that turns raw OS keyboard/mouse
events into game intent, *before* any of it becomes a lockstep command. Two parallel sub-chains:

1. **Keyboard / `CommandClass` dispatch** — `Process_Command @ 0x0055DEE0` reads the
   `WWKeyboardClass` ring buffer each frame, does a two-pass hotkey-table lookup (base VK, then
   VK+modifiers), and fires the matching `CommandClass::Execute` (or a hardcoded built-in key).
   ~89 registered command objects, bound from `KEYBOARDMD.INI`.
2. **Mouse action determination** — `DisplayClass::DetermineAction @ 0x00692610` resolves
   *what order a click at the tactical viewport issues* (the action code) given cursor cell,
   target object, modal UI mode (sell/repair/deploy/chrono/place/SW), and the selected unit's
   polymorphic `What_Action_*` capability; `DisplayClass::SetCursorFromAction @ 0x004AAE90`
   maps that action code to the cursor SHP frame each hover tick.

This service answers: *given a keypress or a click on the map, what is the player's intent and
what cursor confirms it?* Its **outputs** (action code, cursor graphic, the command it issues)
become `EventClass` records for `frontier-net-eventqueue` (E1) — it sits **in front of** the
lockstep boundary, never crosses it.

> **Naming note:** the stale Ghidra label `LogicClass::AI` on `0x0055DEE0` is **DRIFT** — this
> is the keyboard/command input dispatcher (`ProcessKeyboardInput` in `HOTKEY_SYSTEM`), NOT the
> per-object AI tick loop (that is `LogicClass::PerTickUpdate @ 0x0055AFB0`). The spine spec and
> `ADDRESS_MAP` both flag the mislabel.

---

## 2. Representative address — LOCATED + corroborated (3 docs)

**Stub claim (I1):** `Process_Command @ 0x0055DEE0` (hotkey dispatcher — "the stale
`LogicClass::AI` label is wrong per the LogicClass study"); action resolve
`DisplayClass::DetermineAction @ 0x00692610`; cursor `DisplayClass::SetCursorFromAction
@ 0x004AAE90`.

**Verdict:** **All three addresses CONFIRMED** against the corpus (could not re-decompile live
this session — see connectivity note). The stub's framing (front-of-spine, feeds E1) is correct.

### 2a. `Process_Command @ 0x0055DEE0` — keyboard/command dispatcher

- `HOTKEY_SYSTEM_GHIDRA_REPORT.md §3` decompiles **`FUN_0055dee0` (636 bytes)** as
  *"Keyboard Input Dispatch … called every frame from the main game loop (`FUN_0055d360`)"* —
  i.e. caller = `Main_Tick @ 0x0055D360`. The doc's own header marks it
  `[ghidra/verified]` (all findings via live decompilation).
- `ADDRESS_MAP.md:22` lists `0x0055DEE0 | LogicClass::AI (input/event dispatcher, not the
  object-AI tick loop)` — same address, label DRIFT explicitly flagged.
- `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md §1` places it in the **Main_Tick prelude**, step 2:
  *"`Process_Command()` — translate local input into queued game commands."*

Two-pass dispatch (HOTKEY §3): strip key-up bit → `uVar2` (key+mods); strip ALL mod+up bits →
`uVar6` (base VK). FIRST lookup base VK, gate on `command->AcceptsModifiers(uVar2)` (vtable+0x14);
on false, SECOND lookup key+mods. Then `GetCategory`/debug-gate → `CanExecute` (vtable+0x18) →
`Execute` (vtable+0x20) → `IsRepeatable` (vtable+0x1C, consumes up to 10 repeat events).
Hotkey table `DAT_0087f680` (sorted `{u32 keycode, CommandClass*}` 8-byte entries, binary search
`FUN_0055f6e0`, MRU cache `DAT_0087f690`), populated from `KEYBOARDMD.INI`.

### 2b. `DisplayClass::DetermineAction @ 0x00692610` — mouse action resolver

- `DISPLAYCLASS_GHIDRA_REPORT.md §5.1` and `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md`
  (both `[ghidra/verified]`) decompile `0x00692610`: a **two-stage polymorphic dispatch** —
  (1) consult global UI mode flags (sell `DAT_00880998`, power `DAT_0088099a`, waypoint
  `DAT_0088099b`, repair `DAT_0088099c`, guard `DAT_00880999`, SW target `DAT_008809a0`) →
  inject override action codes; (2) `SelectBestObjectForAction @ 0x005353D0` picks one "best"
  selected techno; (3) virtual dispatch `best->vtable[0x70]` (cell path, `What_Action_OnCell`)
  or `best->vtable[0x74]` (target path, `What_Action_OnObject`). Returns an **action code**
  (integer logical intent, ~30+ codes — 0 none, 1 force-move, 2 attack, 7 select, 8 select/enter,
  0x0A spy, 0x0C engineer-capture, 0x0F sell, 0x14 garrison, 0x21/0x22 power, 0x2B SW-target,
  0x3C beacon, 0x3D enter-bridge, …).

### 2c. `DisplayClass::SetCursorFromAction @ 0x004AAE90` — action → cursor graphic

- Confirmed by **multiple** `[ghidra/verified]` docs: `TECHNO_HOVER_HEALTH_FLAG_0431…§ "decompile
  of 0x004AAE90"`, `SELECTION_BRACKETS_PIPS_DRAW_ORDER…:367`, `coord-cell-conversions/
  fn-map-is-cell-in-playfield.md:152` (`DisplayClass__SetCursorFromAction 0x004aae90`),
  `DETERMINE_ACTION_DOWNSTREAM…:411`. Maps the action code to a cursor SHP frame each hover tick,
  and additionally sets the hover-health flag `+0x431` on the filtered object (cleared by
  `TechnoClass::AI_Update`).

---

## 3. What it owns (globals / structures, with addresses)

| Owned state | Address | Meaning | Grade / source |
|---|---|---|---|
| Hotkey binding table | `0x0087F680` (`DAT_0087f680`) | Sorted `{u32 keycode, CommandClass*}` 8-byte entries; binary-searched (`FUN_0055f6e0`); from `KEYBOARDMD.INI` | `HOTKEY_SYSTEM §3` |
| Hotkey MRU cache | `0x0087F690` (`DAT_0087f690`) | O(1) repeat-lookup cache for the table | `HOTKEY_SYSTEM §3` |
| `WWKeyboardClass` ring | object `+0x114` (read `+0x314`, write `+0x318`) | 256 × `ushort` circular event buffer; `(ptr+1)&0xFF` wrap; Peek `FUN_0054f000`, Get `FUN_0054f050`, Flush `FUN_0054f720` | `HOTKEY_SYSTEM §2` |
| Scroll-direction bitmask | `0x00ABCE14` (`DAT_00abce14`) | Arrow-key scroll flags: 0x0001 up / 0x0010 down / 0x0100 left / 0x1000 right (set on arrow-down, cleared on arrow-up) | `HOTKEY_SYSTEM §4` |
| Chat-mode state | `0x00ABCE18` (`DAT_00abce18`) | 1 = team chat, 2 = ally chat, 3 = all-chat (set by `ProcessChatInput FUN_0055e420` Enter/Backslash/Backspace) | `HOTKEY_SYSTEM §4` |
| Modal cursor-mode flags | `0x00880998`–`0x008809a0` | sell `0x00880998` / guard `0x00880999` / power `0x0088099a` / waypoint `0x0088099b` / repair `0x0088099c` / SW-target `0x008809a0` — consulted first by `DetermineAction` | `HOTKEY_SYSTEM §25 Layer 3` |
| Hover-health flag (on techno) | techno `+0x431` | set by `SetCursorFromAction` on filtered hover object; cleared by `TechnoClass::AI_Update` | `TECHNO_HOVER_HEALTH_FLAG_0431…` |
| `CommandClass` registry | ~89 objects (registered in `Register_Game_Commands @ 0x00532150`) | per-command vtable (GetName/Desc/Category/AcceptsModifiers/CanExecute/IsRepeatable/Execute at +0x04…+0x20); simple=4B, parameterized=8B | `HOTKEY_SYSTEM §5–6` |

> **Does NOT own** the event/command queue (`g_CommandBuffer`, DoList) — that is
> `frontier-net-eventqueue` (E1). This service *produces into* it via the builder `0x004C6AE0`.

---

## 4. Key functions (LOCATED addresses)

| Function | Address | Role | Grade / source |
|---|---|---|---|
| `Process_Command` / `ProcessKeyboardInput` | `0x0055DEE0` | per-frame keyboard ring → two-pass hotkey lookup → `CommandClass::Execute` or hardcoded key. Caller `Main_Tick @ 0x0055D360`. (Label `LogicClass::AI` = DRIFT.) | `HOTKEY_SYSTEM §3` (decomp); `ADDRESS_MAP:22`; spine §1 |
| `ProcessChatInput` | `0x0055E420` | chat-key intercept BEFORE hotkey lookup (Enter/Backslash/Backspace → `DAT_00abce18`) | `HOTKEY_SYSTEM §3–4` |
| `Register_Game_Commands` | `0x00532150` | registers ~89 `CommandClass` objects + 3 forced rebinds (Delete→Delete, Escape→Options, Space→CenterOnRadarEvent) | `HOTKEY_SYSTEM §6` |
| `WWKeyboardClass::WindowProc` | `0x0054F790` | WM_KEY*/WM_CHAR/WM_*BUTTON* → enqueue into ring (drops Alt+Tab; pause→Escape-only) | `HOTKEY_SYSTEM §2` |
| `GScreenClass::Input` | `0x004F4320` | per-tick input entry (reads mouse X/Y, snapshots key, hands to gadget root or chain `vtable[10]` = `MouseClass::Process_Input 0x005BDDC0`) | `GSCREEN_RTACTICAL §7` |
| `DisplayClass::Dispatch` | `0x006922E0` | per-frame mouse dispatch: scroll/radar-hover `FUN_00692F30` first, then command-bar/sidebar | `DISPLAYCLASS §5`, `MINIMAP_CLICK_DRAG…` |
| `Tactical::MouseButtonHandler` | `0x006930A0` (`FUN_006930a0`) | WM_LBUTTONDOWN path; guards (`g_GameActive`, map loaded, not modal) → screen-to-cell → DetermineAction → SetCursorFromAction | `HOTKEY_SYSTEM §25` |
| Screen-to-cell | `0x00692300` (`FUN_00692300`) | pixel → cell + 3D pos + object-under-cursor + shroud/fog filter (`FUN_006d6590`/`FUN_006d2280`/`FUN_006da380`) | `HOTKEY_SYSTEM §25 Layer 2` |
| `DisplayClass::DetermineAction` | `0x00692610` | modal-mode override + best-object polymorphic `What_Action_*` → **action code** | `DISPLAYCLASS §5.1`, `DETERMINE_ACTION_DOWNSTREAM` |
| `SelectBestObjectForAction` | `0x005353D0` | scores the selection, returns the single techno that applies the action (ties by distance) | `DETERMINE_ACTION_DOWNSTREAM §2` |
| `DisplayClass::SetCursorFromAction` | `0x004AAE90` | action code → cursor SHP frame; sets hover-health `+0x431` | `TECHNO_HOVER_HEALTH_FLAG_0431…`, `DETERMINE_ACTION_DOWNSTREAM:411` |
| `DisplayClass::BandBox_LeftUp` | `0x004AB9B0` (`FUN_004ab9b0`) | mouse-up command executor: band-box select / placement / per-action command dispatch to ALL selected units (`FUN_004ae750`); **builds network command packets** | `HOTKEY_SYSTEM §25 Layer 4` |
| event builder | `0x004C6AE0` (`FUN_004c6ae0`) | constructs an `EventClass` record (writes `+3 = g_CurrentFrameCounter` placeholder issue-frame). **Callers = `DisplayClass::BandBox_LeftUp`, `SelectClass::Action`, `StripClass::AI`.** | `frontier-net-eventqueue §4` (`get_function_callers 0x004C6AE0`) |
| Issue-to-selection | `0x004AE750` (`FUN_004ae750`) | fans a resolved action out to every selected unit via `vtable+0x70/0x74` (compute mission) + `vtable+0x140/0x144` (execute) | `HOTKEY_SYSTEM §25 Layer 5` |

---

## 5. Plug point (tick spine)

**OUT-OF-SIM input stage, in the `Main_Tick` prelude — NOT a `PerTickUpdate` rung.**

Per `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md §1`, the live gameplay per-frame order, inside the
`((SpecialFlags & 2)==0) && g_GameState==0 && g_GameRunning!=0` block, **before** `PerTickUpdate`:

```
1. GScreenClass::Input (0x004F4320)   — collect local mouse/keyboard  ← this service (entry)
2. Process_Command   (0x0055DEE0)     — keyboard → fire CommandClass / queue commands  ← this service
3. Network_Keepalive  (when (frame & 7)==7 && mode 4)
4. Map__Logic()                        — the live command/event QUEUE is EXECUTED here (E1's stage)
5. RenderFrame_main (0x004F4480)       — cursor + tactical draw
   then [state-hash record/verify], FUN_00551a30, then PerTickUpdate (the 28-rung ladder)
```

- **Keyboard** dispatch is literally **prelude step 2** (`Process_Command`).
- **Mouse action resolution** (`DetermineAction`/`SetCursorFromAction`) runs from the input/
  hover dispatch (`GScreenClass::Input → DisplayClass::Dispatch`, prelude step 1) and from the
  WM_LBUTTONDOWN handler `Tactical::MouseButtonHandler @ 0x006930A0`; cursor commit is part of
  `RenderFrame_main` (prelude step 5).
- **The command a click issues** is built (`0x004C6AE0`) and pushed into `g_CommandBuffer` here;
  it is **executed** one prelude-step later inside `Map__Logic` (E1's stage), still before the
  rung ladder.

**Spine-spec tie:** **PRELUDE, before Rung A.** This service is the producer that feeds the
prelude's `Map__Logic` command-execution stage. It consumes **zero** PerTickUpdate rung slots and
draws **zero** RNG (the §3 lockstep RNG order begins at Rung H; input dispatch is upstream of it).

**Render tie:** cursor graphic emitted by `SetCursorFromAction` is composited in
`RenderFrame_main @ 0x004F4480` (the §3 render entry is `TacticalClass_Draw @ 0x006D3D10`, called
from the frame driver).

---

## 6. Outgoing edges (depends-on)

| → Service (slug) | Via (symbol) | Evidence |
|---|---|---|
| `frontier-net-eventqueue` | clicks/hotkeys build `EventClass` records via `0x004C6AE0` (callers `DisplayClass::BandBox_LeftUp`, `SelectClass::Action`, `StripClass::AI`) into `g_CommandBuffer`; `BandBox_LeftUp @ 0x004AB9B0` also builds network packets (0x0B place, 0x12 sidebar, 0x15 spy, 0x16/0x17 engineer) | `frontier-net-eventqueue §4/§7` (`get_function_callers 0x004C6AE0`); `HOTKEY_SYSTEM §25 Layer 4` |
| `target-scoring` | `DetermineAction` calls `SelectBestObjectForAction @ 0x005353D0` (priority/distance scoring) to pick the techno that applies the action; legal-action resolution per selected unit | `DETERMINE_ACTION_DOWNSTREAM §2` |
| `cell-map` | screen-to-cell `FUN_00692300` reads cell occupancy / object-under-cursor + shroud/fog (`FUN_00586360`/`FUN_005865e0`); modal/cell legality gates DetermineAction | `HOTKEY_SYSTEM §25 Layer 2` |
| `frontier-render-tactical` | screen→cell uses the tactical transforms `FUN_006d6590` (pixel→cell) / `FUN_006d2280` (sub-cell); cursor commit rides `RenderFrame_main`/`TacticalClass_Draw` | `HOTKEY_SYSTEM §25 Layer 2`; spine §1 |
| `techno-foot` | action fan-out `FUN_004ae750` invokes per-unit `What_Action_*` (`vtable+0x70/0x74`) + execute (`vtable+0x140/0x144`); the resolved order targets selected `FootClass`/`TechnoClass` units | `DETERMINE_ACTION_DOWNSTREAM §1`; `HOTKEY_SYSTEM §25 Layer 5` |
| `rules-class` | `CommandClass` bindings sourced from `KEYBOARDMD.INI`; modal capabilities (deploy/sell/repair eligibility) read from techno-type rules | `HOTKEY_SYSTEM §3`, §6 |
| `gadget-dialog` | when a gadget root exists, `GScreenClass::Input` routes input to `GadgetClass::Input @ 0x004E1640` (in-game dialog controls consume the event first) | `GSCREEN_RTACTICAL §7` |

---

## 7. Incoming edges (used-by)

| ← Service (slug) | Via (symbol) | Evidence |
|---|---|---|
| `logicclass` | `Main_Tick @ 0x0055D360` calls `GScreenClass::Input` + `Process_Command` in the prelude every gameplay frame (before `PerTickUpdate`) | spine spec §1; `GSCREEN_RTACTICAL §8` |
| `frontier-net-eventqueue` | E1 is the **direct consumer** — the records this service builds (`0x004C6AE0`) are what E1 drains/executes via `Map__Logic → FUN_00647260 → FUN_0064C380 → EventClass::Execute` | `frontier-net-eventqueue §7/§8` |
| `frontier-sidebar` | sidebar cameo clicks route through the same input chain; `StripClass::AI` is one of the 3 event-builder callers (sidebar self-issues Place/Begin via the queue), and `SidebarClass::Action @ 0x006A7780` sits in the mouse dispatch chain | `frontier-net-eventqueue §4`; `GSCREEN_RTACTICAL §7` |
| `frontier-radar` | minimap click-to-recenter / beacon placement reaches the radar via the same dispatch (`DisplayClass::Dispatch` radar-hover precedence) | `MINIMAP_CLICK_DRAG_INVERSE_TRANSFORM…` |

---

## 8. Active-in-YR / TS-legacy

- **Active in YR — YES.** Keyboard dispatch (`Process_Command`), mouse action resolution
  (`DetermineAction`/`SetCursorFromAction`), and the click→command path run **every gameplay
  frame in every mode** (campaign 0, LAN 3, WOL 4, skirmish 5). This is the player's entire
  interaction surface — maximally player-visible (every keypress, every click, every cursor
  change).
- **MP-specific (not TS-legacy):** chat keys (`ProcessChatInput`), beacon placement, alliance
  toggle, and the network-packet sub-types in `BandBox_LeftUp` are reachable only in multiplayer
  — current YR code, network-gated, not dead.
- **No TS-legacy dead path identified in the core hotkey/action resolver.** Candidate residuals
  to confirm on a live pass: some action codes in `DetermineAction`'s switch / `SetCursorFromAction`
  mapping are guess-labeled where the cursor entry was "—" (`DETERMINE_ACTION_DOWNSTREAM` MEDIUM),
  and the `Ctrl+Alt+Shift+M` Win32 `RegisterHotKey` is a likely dev/debug shortcut. **Neither is
  established as TS-legacy** — flagged as UNCHECKED, not asserted.

---

## 9. Scale flags (30-player / 20k-unit target)

- The hotkey table and `CommandClass` registry are **fixed sets** (driven by `KEYBOARDMD.INI`,
  not by player/unit count) — no scaling concern.
- Action resolution is **per local player's selection**, independent of total player count; the
  determinism boundary is downstream in E1. Input dispatch carries **no lockstep state** — it is
  purely local. The only scale-relevant downstream constraint (house-array-order dispatch at 30
  players) lives in E1, not here.
- `SelectBestObjectForAction` scans the local selection set (`g_CurrentObjects`), bounded by
  selection size, not world size — fine at 20k units.

---

## 10. Remaining uncertainty / follow-ups

1. **Ghidra was unreachable this session.** Re-run `decompile_function 0x0055DEE0`,
   `0x00692610`, `0x004AAE90`, and `get_function_callers 0x004C6AE0` live to upgrade the
   docs-corroborated status to fresh-live grade. **NEEDS-LIVE-REVERIFY.**
2. **Guess-labeled action codes** in `DetermineAction`/`SetCursorFromAction` (the "—" cursor
   entries) — enumerate the full action→cursor mapping live. (`DETERMINE_ACTION_DOWNSTREAM §4`.)
3. **`CommandClass` Execute bodies** are not all decoded — which commands self-issue `EventClass`
   records vs mutate UI-local state directly is only partially traced (e.g. team commands).
4. **Modal-mode flag owners** (`0x00880998`–`0x008809a0`) read from decomp context, not
   `read_memory`-confirmed — verify symbol binding live (YELLOW).

---

## 11. Sources

- `docs/research/HOTKEY_SYSTEM_GHIDRA_REPORT.md` (§2 input capture, §3 keyboard dispatch
  `FUN_0055dee0`, §4 hardcoded keys / chat, §5–6 CommandClass hierarchy + registry, §25 left-click
  dispatch chain) — primary, `[ghidra/verified]`.
- `docs/research/DISPLAYCLASS_GHIDRA_REPORT.md` §5 / §5.1 (`DetermineAction 0x00692610`, cursor
  pipeline).
- `docs/research/DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` (polymorphic `What_Action_*`
  dispatch, `SelectBestObjectForAction 0x005353D0`, action→cursor mapping).
- `docs/research/GSCREEN_RTACTICAL_GHIDRA_REPORT.md` §7–8 (`GScreenClass::Input 0x004F4320`,
  Main_Tick orchestration).
- `docs/research/MINIMAP_CLICK_DRAG_INVERSE_TRANSFORM_GHIDRA_REPORT.md` (`DisplayClass::Dispatch
  0x006922E0`, radar hover precedence).
- `docs/research/building-selection-brackets/TECHNO_HOVER_HEALTH_FLAG_0431_BUILDING_PIPS_GHIDRA_REPORT.md`
  (`SetCursorFromAction 0x004AAE90` decompile, `+0x431` flag).
- `docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` §1 (Main_Tick prelude order;
  `Process_Command` = step 2; `Map__Logic` = live command stage; representative-label DRIFT).
- `docs/research/ADDRESS_MAP.md:22` (`0x0055DEE0` input/event dispatcher, `LogicClass::AI` label
  flagged).
- `docs/research/core-services-map/frontier-net-eventqueue.md` §4/§7/§8 (builder `0x004C6AE0`
  callers; E1↔I1 edge).
- `docs/research/core-services-map/_frontier.md` §I1 (seed stub).
