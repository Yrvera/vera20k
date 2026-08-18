---
title: Core Services Map — shell-dialog (Shell dialog framework / Framework B)
slug: shell-dialog
date: 2026-06-25
status: profile (derived from SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md, Ghidra-verified that session; edges confirmed via research-index this session)
source: docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md (primary) + MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT + PERTICKUPDATE_FULL_ORDERING_LADDER + RULESCLASS/MultiplayerDialogSettings
---

# shell-dialog — Win32 owner-draw shell (Framework B)

## Purpose

The engine's **front-end and modal UI service**: everything the player sees when
*not* in a live tactical scenario, plus in-game options/validation modals. Owns the
menu/setup shell — main menu, single-player intermediate dialog, skirmish setup,
options, load/save, quit/validation modals, movies, WOL panels. Built on native
modeless Win32 `HWND` dialogs created from PE `RT_DIALOG` templates, every child
control owner-draw-subclassed behind one shared wndproc, hand-pumped by a custom
message loop.

This is **Framework B**, distinct from and never sharing dispatch with **Framework A**
(`gadget-dialog`, the in-game retained-mode GadgetClass widget tree: sidebar, radar,
tabs, command bar, cameos). A faithful Rust port keeps them as two separate services.

## Owns

- **Dialog factory + lifecycle**: `CreateDialogIndirectParamA` modeless dialogs under
  the game window (`FUN_00622650`), show (`FUN_00622800`), teardown with LIFO compact
  + focus restore (`FUN_00622720`).
- **Two independent registries**:
  - LIFO display/focus stack: `DAT_00b72d28` (HWND), `DAT_00b72d2c` (id), stride 8,
    depth `DAT_00b72f50`, top mirror `DAT_00b72f44`/`DAT_00b72f48`.
  - Keyboard-routing array: `DAT_00abfc94` (HWND[]), count `DAT_00abfca0`.
- **Owner-draw runtime registries (HWND-keyed hashtables)**: `DAT_00AC18C0`
  (HWND→paint proc), `DAT_00AC1B48` (HWND→original WndProc), `DAT_00AC1B00`
  (HWND→0x208 record), `DAT_00AC1DE8` (z-order), `DAT_00AC48DC` (paint-depth),
  `DAT_00AC48D4` (one-time color/PCX-preload guard).
- **The 0x208 per-control record** — control kind (`+0x68`), resource id (`+0x70`
  int idx 0x1c), paint-mode/asset (`+0xB0`), slide markers (`+0xB4`/`+0xC1`/`+0xC2`),
  hover/flash (`+0xC5`), first-paint slide state machine (`+0x1FC`, 1→2→3), text
  buffer (`+0x28`), per-dialog offscreen BSurface (`+0x14`).
- **Composition surfaces/theme**: `DAT_00887310` AlternateSurface (composition
  target), default shell text color `DAT_00ac18a4` = `0xFFFF` (yellow), disabled
  text `DAT_00ac1cb4` = `#9F0000`; static dims `DAT_007F5BE0..F0`.
- **Navigation state machine**: `Main_Game 0x0052D9A0` — maps each dialog's return
  code (via `GWLP_USER` result pointer + sentinel) to the next dialog or scenario
  launch. This is the result-routed loop, not per-dialog handlers.
- **The hand-rolled modal pump body** `FUN_00623120` (the loop itself lives in each
  owner) and the in-game skirmish-default seed struct (`RulesClass + 0x1480..0x14BB`,
  11 ints + 16 bools) populated from INI.

Note: the substrate is **wndproc-dispatched, not vtable-dispatched** — no
GadgetClass-style vtable. The only COM/vtable indirection is the DSurface/BSurface
blit slots, which belong to the render backend (`drawing-helpers`), not here.

## Key functions & globals (addresses)

| Address | Role |
|---|---|
| `0x00622650` | Dialog factory — CreateDialogIndirectParamA + push LIFO stack + register routing |
| `0x00622b50` | Common shell DLGPROC — WM_INITDIALOG/DESTROY/PAINT/hit-test |
| `0x00622820` | Init bridge — subclass children + slide-group markers |
| `0x00622800` | Show — ShowWindow + SetForegroundWindow |
| `0x00622720` | Teardown — slide-out, DestroyWindow, LIFO compact, restore focus |
| `0x00623120` | Modal pump tick (body) — Process_NetworkMessages first, then mode-gated Main_Tick |
| `0x005d4d50` | Process_NetworkMessages — Peek/Get loop, IsDialogMessageA per registered HWND |
| `0x005d4e70` / `0x005d4ed0` | Register / unregister HWND in keyboard-routing array |
| `0x0060f9a0` | Owner-draw subclass setup (`ownrdraw.cpp`) — classify, install wndproc, alloc record |
| `0x00610ca0` | Shared subclass wndproc — input/paint dispatch heart |
| `0x00623340` | 0x208 record initializer (zero, kind=0xB, font=g_GAME_FNT) |
| `0x0060c4a0` | Reposition pass — expand parent fullscreen + EnumChildWindows(ResizeShellChildControl) |
| `0x0060c540` | Include-test + slide-marker setter |
| `0x0060c0c0` | ResizeShellChildControl — per-child first-match-wins re-anchor |
| `0x00621E90` | WM_PAINT_Handler — mode-1/mode-2 parent composition + flip |
| `0x0060CF00` | Dialog background table — id → (convert, small SHP, large SHP) |
| `0x00612B70` | OwnerDraw_Button — SDBTNANM frames 2/3/4 |
| `0x006153E0` | OwnerDraw_Static — kind 0..4 text/image/SHP/movie |
| `0x00621040` | ShellText__DrawInRect — 0x00BBGGRR color permutation, default yellow |
| `0x006071E0` | Slide animation loop — SHP-frame sweep, 30ms/tick, N+~6 ticks |
| `0x00531CC0` / `0x00531F60` | Main-menu 0xE2 runner / proc (button → 1..6) |
| `0x006AE2C0` | Skirmish 0x102 runner (Start 0x617 / Back 0x5C0) |
| `0x005D3490` | Generic CSF modal helper — template family by text-slot presence |
| `0x0052D9A0` | Main_Game — navigation state machine |
| `0x00671EA0` | RulesClass__ReadMultiplayerDialogSettings — INI → skirmish defaults |

## Tick / render position

**Not on the `LogicClass::PerTickUpdate` sim spine.** This service runs the *outer*
loop when there is no live scenario, OR sits as a modal on top of one. Its tick is the
hand-rolled pump body `FUN_00623120`, invoked by each owner's loop:

1. `Process_NetworkMessages 0x005d4d50` always runs first (Peek/Get + IsDialogMessageA
   per registered dialog → keyboard/input/repaint stays live).
2. Then **mode-gated sim advance**: offline campaign (`g_GameMode==0`) and offline
   skirmish (`g_GameMode==5`), or the `DAT_00A8D60E`/`DAT_00A8DAB4` blockers, take a
   `Network_ServiceLoop`-only branch and **do NOT call Main_Tick** — the world FREEZES
   behind offline Options while the dialog stays responsive. Only network modes (LAN 3,
   WOL/Internet 4) call `Main_Tick @ 0x0055D360` (guarded by `DAT_00ABCD58`/`FUN_0055CBF0`
   reentrancy), which then drives `LogicClass::PerTickUpdate @ 0x0055AFB0`.

**Render**: owns its own paint pass (`WM_PAINT_Handler 0x00621E90`) — offscreen
BSurface ← right-panel chrome → MNSCRN background → owner-draw controls → single flip
to AlternateSurface. This is parallel to (not part of) the in-scenario render pass.

## Depends-on (outgoing edges)

| Target slug | Via symbol | Evidence |
|---|---|---|
| **logicclass** | `FUN_00623120` modal pump → `Main_Tick 0x0055D360` → `LogicClass::PerTickUpdate 0x0055AFB0` (call site `0x0055DC99`) | On network-eligible branches the pump advances the sim spine; offline freezes (MODAL_PUMP_00623120 §3.4; PERTICKUPDATE_FULL_ORDERING_LADDER). The shell pump is the *caller* that keeps the tick spine alive behind in-game network modals. |
| **rules-class** | `RulesClass__ReadMultiplayerDialogSettings 0x00671EA0`; skirmish-default struct at `RulesClass + 0x1480..0x14BB` | Skirmish dialog 0x102 seeds money/unit sliders, TechLevel(10), GameSpeed(1), AIDifficulty, AIPlayers + 16 bools (Bases/Crates/Shroud/FogOfWar=no/…) directly from the parsed RulesClass global (doc §1 item 9, C14, §2.3). |
| **ini-parsing** | `RulesClass__ReadMultiplayerDialogSettings 0x00671EA0` reads `[MultiplayerDialogSettings]` via CCINIClass accessors; quit-confirm writes `ra2md.ini` before graceful return | INI accessor reads populate the seed struct; the navigation cascade writes the INI on quit (doc C12, C14). |
| **drawing-helpers** | `WM_PAINT_Handler 0x00621E90` → DSurface/BSurface vtable `+8` blit / `+0x14` fill; `RightPanel__Draw 0x0072E450`, `Background_Overlay 0x0072E730`; `ShellText__DrawInRect 0x00621040` glyph blit; SHP frame draw in owner-draw button paint | Every composition blit, the final flip, right-panel chrome, MNSCRN/SHP background, and 1-bpp glyph text route through the blitter/SHP draw primitives (doc §2.5, C8, C10). |
| **lookup-tables** | Dialog background table `FUN_0060CF00` (id→convert/small SHP/large SHP); owner-draw class-routing strcmp cascade in `FUN_0060f9a0`; static dims `DAT_007F5BE0..F0` | Static read-only id→asset and class→proc/kind dispatch tables drive background selection and control classification (doc §2.2, §2.3, `0x0060CF00`). |
| **frontier-net** | `Process_NetworkMessages 0x005d4d50` + `Network_ServiceLoop` branch in `FUN_00623120` | The pump services network message Peek/Get and the network service loop every tick, before/instead of sim advance (doc C2; MODAL_PUMP_00623120 §3.2). No studied core-service slug owns netcode, so this edge points at the network frontier. |
| **frontier-audio** | `MenuSlideIn`/GUIMoveInSound at slide start (`FUN_006071E0`); `ShellButtonSlideSound` at slide end (active code, stock-empty key) | Slide animation fires shell sound cues through the audio service (doc §1 "Animate", C11, §3 active list). No audio core-service slug yet → frontier-audio. |

## Used-by (incoming edges)

| Source slug | Via | Evidence |
|---|---|---|
| **logicclass** (indirect / inverse of above) | `Main_Game 0x0052D9A0` owns the outer loop that calls the pump and, on launch, hands off to `Main_Tick`/the in-scenario loop | The shell is the *entry gate* to a scenario: navigation routes a dialog result (e.g. skirmish Start 0x617) into scenario init, after which the LogicClass tick spine takes over. The two are coupled at the boundary, not in steady state. |
| **rules-class** | reverse of the seed edge | `rules-class` provides values; shell *displays/edits* them, then writes chosen settings back into scenario setup. The producing direction is rules→shell; shell→scenario is the consumer of edited values. |
| **gadget-dialog** (Framework A) | none at runtime — sibling, not dependent | Explicitly do NOT share dispatch (doc §0). Listed only to assert the **absence** of an edge: the in-game GadgetClass tree neither calls nor is called by the shell substrate. |

The shell-dialog service has **no in-tactical-sim consumers**: nothing in
`logicclass`/`techno-foot`/`cell-map`/`factory-house`/`damage-helpers` reads or calls
it during a tick. It is a top-of-stack service. Its only "downstream" is the scenario
launch hand-off via `Main_Game`.

## Open / unverified edges

- **Template-id selection inside the modal family** (`FUN_005D3490`): the
  body+OK→`0xCE` / +cancel→`0x120` / +3rd→`0x121` template-id choice is NOT visible
  inside `0x005D3490` (it calls the factory with a caller-supplied template). The
  caller that picks the template id is untraced — wiring `ModalKind` requires tracing
  a caller of `0x005D3490` (doc C13 [UNCHECKED]). (Partly resolved 2026-06-12 in
  `src/ui/shell/modal.rs` with binary citations; verify against current binary before
  relying.)
- **frontier-net edge granularity**: `Network_ServiceLoop` address/owner not pinned to
  a studied slug; treated as frontier-net. Exact net entry inside `Main_Tick`
  (`0x0055DE4A`) is in the LogicClass/net boundary, not re-verified here.
- **frontier-audio edge**: `MenuSlideIn`/`ShellButtonSlideSound` cue dispatch path into
  the audio mixer not traced to a function address in this profile (stock end-cue is
  silent — empty INI key, rulesmd.ini:712).
- **+0x6C vs +0x70 dialog-id reads** in reposition branches: doc flags per-branch
  ambiguity (some branches read id via `+0x6C`, others via int-index 0x1c = `+0x70`);
  confirm per-branch before wiring layout (doc §2.4 note).
- **Full per-dialog WM_COMMAND→result maps** beyond 0xE2/0x100/0x102 are not
  enumerated; `0x101`/`0x129` procs lack Ghidra function boundaries (doc Sources).
