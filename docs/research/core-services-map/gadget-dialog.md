# Core Service Profile — `gadget-dialog` (GadgetClass / Dialog control, Framework A in-game gadget tree)

**Slug:** `gadget-dialog`
**Primary doc:** `docs/research/GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (STUDY+DESIGN, 2026-06-10; Ghidra-verified, addresses cited inline).
**Scope of this profile:** Framework A only — the `GadgetClass`/`ControlClass`/`LinkClass` retained-mode in-game gadget tree (sidebar buttons, cameo strips, command bar, radar buttons, invisible tactical/minimap click catchers, chat/system message labels), its hit-testing, capture, focus, and input routing. Framework B (the Win32 RT_DIALOG owner-draw shell) is the **separate** `shell-dialog` service and is profiled there; this profile names the A↔B meeting points as edges to `shell-dialog`.

---

## Purpose

Single per-tick input + draw authority for **all in-game chrome**. Once per gameplay tick, `GadgetClass::Input 0x004E1640` is called on the Buttons list head `[0x00A8EF54]`; it reads the keyboard/mouse event queue, assembles event flags, runs hover enter/leave transitions, dispatches through three exclusive tiers (sticky capture → keyboard focus → broadcast head→tail walk), draws each visited gadget inside the same walk, and returns a 16-bit result. Live in-game buttons fire `ID|0x8000` (plus `|0x4000` on a right-release when masked-in) on **release-inside** after a silent press+capture, with drag-off cancel — the gadget result IDs are then consumed layer-locally by the GScreen AI cascade, which mutates UI state only. Sim-affecting clicks become queued network commands consumed later in the tick, never inside the UI layer.

Only **9 of 20 family classes are live in YR** (the three bases + ShapeButtonClass, SelectClass cameo, SBGadgetClass, TextLabelClass, the tactical-screen click gadget, and RTacticalClass minimap region). The entire TS shell-control wing (ListClass, EditClass, SliderClass, GaugeClass, Dial8Class, etc.) is linker-retained **dead code** (study §3.3) — RA2/YR replaced it with the Win32 shell (`shell-dialog`).

## Owns

State, globals, structs this service owns (study §2.1, §2.4, §2.5):

- **The in-game gadget list** — Buttons head `[0x00A8EF54]`; intrusive doubly-linked sibling list (Next +0x04 / Prev +0x08), insertion order = hit-priority order = draw order. Exactly 4 writers: zero-stubs `0x004F42A0`/`0x004F42E0`, `Add_A_Button 0x004F4410` (tail-append, double-insert reject), `Remove_A_Button 0x004F4450`.
- **Four focus/capture globals** (each writer-census-exhaustive):
  - `g_StickyFocus [0x008B3E88]` — mouse-capture gadget; written by Sticky_Process acquire/release, Input fresh-list reset, both dtors. Extra reader: `GaugeClass::Action 0x004E285C` (dormant).
  - `g_CurrentGadgetList [0x008B3E8C]` — fresh-list reset detector; Input + Clear_Attached_List `0x00488690` + dtors.
  - `g_KeyboardFocus [0x008B3E90]` — keyboard-event routing target; Set_Focus/Clear_Focus, Input fresh-list reset, dtors.
  - `g_HoveredGadget [0x008B3E94]` — hover enter/leave driver; Input is the **sole writer AND sole reader**; dtors do NOT clear it (G7 stale-pointer hazard).
- **Hit-test seed constants** — `0x007F5BE8 = 1024`, `0x007F5BF4 = 768` (best-area seed 786,432 px², zero writers — NOT live resolution).
- **GadgetClass base vtable `0x007E92BC`** (33 slots, +0x00..+0x80) and the 20 family vtables (study §2.1/§2.2).
- **Gadget struct layout:** +0x00 vtbl, +0x04 Next, +0x08 Prev, +0x0C X, +0x10 Y, +0x14 W, +0x18 H, +0x1C IsToRedraw, +0x1D IsSticky, +0x1E IsDisabled, +0x20 Flags; sizeof 0x24. ControlClass +0x24 ID, +0x28 Peer. ToggleClass +0x2C IsPressed, +0x2D IsOn, +0x30 Kind. Object sizes: ShapeButtonClass 0x60, SBGadgetClass 0x28, SelectClass 0x38, TextLabelClass 0x4C.
- **Live gadget population (static globals):** 4 sidebar tab ShapeButtons `0x00B07C48` (IDs 0xCB..0xCE); repair/sell + strip-scroll singles (`0x00B07DF8`/`0x00B0B328`/`0x00B0B3A0`/`0x00B0B408`, IDs 0x65/0x66/0xC8/0xC9); sidebar body SBGadget `0x00B07E58`; 240 SelectClass cameo gadgets `0x00B07E80` (4 tabs × 60); 2 radar mode ShapeButtons `0x00B04978`/`0x00B04910`; minimap RTacticalClass region `0x00B04A10` (flags 0x9F, sticky); 25-button command bar array `0x00B0C1C0` (IDs 0x80D6..0x80EE); 2 collapse/expand toggles `0x00B0CCB0`/`0x00B0CC40` (IDs 0xF0/0xF1); full-tactical click catcher `0x008A06F8` (flags 0x7F, sticky); heap-built TextLabelClass chat/system labels (0x4C each).

## Key functions & globals (addresses)

| Address | Symbol | Role |
|---|---|---|
| 0x004E1640 | GadgetClass::Input | per-tick dispatch authority (3-tier) — vtable slot +0x28 |
| 0x004E15A0 | Hit_Test | half-open rects, skip disabled, smallest-area `<=` tie-break (later wins), 1024×768 seed (non-virtual) |
| 0x004E13F0 | Clicked_On | per-gadget mask filter + sticky/keyboard bypass — slot +0x80 |
| 0x004E12F0 / 0x004E1390 / 0x004E1A60 | GadgetClass ctor / dtor / scalar-dtor | sticky → Flags\|=5; dtor clears sticky/kbd/current-list but NOT hover |
| 0x004E1970 | Sticky_Process | press 0x11 acquire iff IsSticky; release 0x44 release holder-only — slot +0x78 |
| 0x004E19A0 / 0x004E19D0 / 0x004E19F0 | Set_Focus / Clear_Focus / Has_Focus | keyboard focus steal protocol |
| 0x004E1570 / 0x004E1550 | Draw_All / base Draw_Me | list-walk draw / dirty-gated self-draw |
| 0x004E1920 / 0x004E14C0 | Extract_Gadget(id) / Delete_List | find-by-id / rewind-destroy-forward |
| 0x004E1510 / 0x004E1520 | base Mouse_Enter / Mouse_Leave | RET stubs (overridden only by SelectClass) |
| 0x0048E520 / 0x0048E5A0 / 0x0048E620 / 0x0048E600 | ControlClass ctor / Action / Draw_Me / Set_Peer | Action posts ID\|0x8000 (+\|0x4000 right-release), notifies Peer via Peer_Callback |
| 0x00723E60 / 0x00723EC0 | ToggleClass ctor / Action | ctor flags=5 sticky=1; Action = the button machine (silent press, fire-on-release, drag-off cancel) |
| 0x0069DCF0 / 0x0069DEB0 / 0x0069DE00 | ShapeButtonClass ctorA / Draw_Me / Set_Shape | Draw_Me devirt-called from radar/sidebar/maingame draw; +0x88 Set_Shape is a NEW slot (not repurposed Set_Peer — adversarial correction C1) |
| 0x006AB990 / 0x006AB9E0 | SelectClass Mouse_Enter / Mouse_Leave | only family member with tooltip hooks |
| 0x005D3BA0 / 0x005D4210 | MessageListClass Add_Message paths | heap-build TextLabelClass (ctor 0x0072A440) per chat/system message |
| 0x004F4320 / 0x004F4410 / 0x004F4450 | GScreenClass::Input / Add_A_Button / Remove_A_Button | chain Input entry; Buttons-head registration |
| Globals | g_StickyFocus 0x008B3E88, g_CurrentGadgetList 0x008B3E8C, g_KeyboardFocus 0x008B3E90, g_HoveredGadget 0x008B3E94, Buttons head 0x00A8EF54, hit-test seed 0x007F5BE8/0x007F5BF4 | (see Owns) |

## Tick / render position

- **Input:** `Main_Tick 0x0055D360` → `GScreenClass::Input` (chain slot 9) @ `0x0055D8AB`, gated by `(SpecialFlags&2)==0 && g_GameState==0 && g_GameRunning`. Gadget Input draws to the hidden surface via the `[0x00887314]`↔`[0x0088730C]` swap (O3), restored immediately after. This runs at the **start** of the gameplay-tick frame order (O9: Input → Process_Command → Map_Logic → RenderFrame_main → record/playback → LogicClass::Update object-sim → sound/scroll → queued-command execution → network → frame++).
- **AI-cascade ID consumption** happens inside GScreenClass::Input: Mouse → Scroll → Tab → Sidebar → Power → Radar → Display → GScreen; each layer consumes its own button IDs and mutates UI state only.
- **Sim seam:** the gadget layer NEVER enters the sim. Player commands are queued and executed AFTER the object-sim pass via `Main_Tick → FUN_00647260 → FUN_0064C380 → EventClass::Execute 0x004C6CB0` (adversarial correction C3: NOT inside LogicClass::Update; the "Process_QueuedEvents" label at 0x0053B560 is drift for a screen-flash machine).
- **Render:** dual draw pump (A10) — Input force-draws visited gadgets per tick, AND `RenderFrame_main 0x004F4480` calls the list-walk Draw_All slot (`+0x2C → 0x004E1570`, forced=0) each frame (draw-if-dirty). Draw order = list order = registration order (tail-ward renders on top). In O10 render composition, the gadget Draw_Me(0) walk runs after tactical/sidebar-chrome passes, before chat overlay and tooltip draw.

## Depends-on (outgoing edges)

Each edge: target slug — via-symbol — evidence.

- **`logicclass`** — via `GScreenClass::Input` call site in `Main_Tick 0x0055D360` (gated by g_GameState/SpecialFlags). The per-tick scheduler / tick spine is what invokes gadget Input each gameplay tick at a fixed frame-order position (study O2/O9). The gadget service does not call into LogicClass itself; the edge is the spine driving it. Evidence: study §1.A1, §5-O2/O9 (decompile 0x0055D360).

- **`shell-dialog`** — the two frameworks meet in exactly three VERIFIED-LIVE places (study §0/§2.6/§2.8):
  - via **open-shell-dialog counter `0x00A8ED8C`** — in-game gadget/chrome code READS it (GScreen flip region `0x004F4B5A`, sidebar `0x006A6A00`) to know whether any shell dialog is open.
  - via **modal pump body `0x00623120`** — decides whether the Framework-A world (gadget Input + AI cascade + render + sim) ticks behind a pumping dialog (O12 coexistence matrix).
  - via **shared ToolTipManager singleton `[0x00887368]`** — registrants from BOTH frameworks; gadget/sidebar hover hooks register rects on the same manager owned/driven by the shell's Win32 pump (`ProcessMessage 0x00724200`, sole caller Process_NetworkMessages).
  Evidence: study §2.8, §5-S1/S2, O12.

- **`random-scenario`** (ScenarioClass side) — via scenario/session-start triggering `TabClass::Activate 0x006D04F0` rebuild of the Buttons list, and the tactical-screen catcher re-add gated on `!g_IsMapEditor` at scenario init (study O8, §3.3). Edge is rebuild-trigger lifecycle, not RNG. Note: **no RNG edge** — S5 negative ("no RNG in UI") holds for all decompiled UI bodies (scope-limited, §9). Evidence: study §1.A12, §5-O8.

- **`drawing-helpers`** — via ShapeButtonClass/TextLabelClass/cameo `Draw_Me` slots emitting SHP draw primitives; `ShapeButtonClass::Draw_Me 0x0069DEB0` is devirtualized-called from the radar/sidebar/main-game draw code, and the in-game draw chain (`WM_PAINT FUN_00621E90` / SidebarClass::Draw `0x006A6C30`) issues DrawSHP for SDBTNANM.SHP / SDBTNBKGD.SHP etc. for gadget chrome. Evidence: study §2.2 (Draw_Me devirt note, gadget-core §9.1); `SIDEBAR_CONSTRUCTION_GHIDRA_REPORT.md` §5 draw chain, `SIDEBAR_DRAW_COMPOSITION_ORDER_AND_SURFACE_ORIGIN_GHIDRA_REPORT.md`.

- **`frontier-audio`** — via UI click/insert sounds: SelectClass cameo and MessageList insert call `VocClass__PlayAtPos 0x00750920` (null handle, volume 1.0f, non-spatial), and ToggleClass/owner-draw button transitions play resolved rule sounds. Evidence: study §3.1 (Voc on MessageList insert), §6.2 (click sounds as audio lookups); `SHELL_UI_SOUND_PLAYBACK_PLUMBING_GHIDRA_REPORT.md` §3.2/§5 (`VocClass__PlayAtPos`). (No studied `frontier-audio` slug formalized; named per the frontier convention.)

- **`rules-class`** — via UI sound-index resolution: the effective click/insert sound indices are read from RulesClass audio fields (`RulesClass__ReadAudioVisual 0x006691E0` feeds the GUI button/checkbox/trackbar sounds). The gadget layer triggers playback of rule-resolved sounds rather than hardcoding them. Evidence: `SHELL_UI_SOUND_PLAYBACK_PLUMBING_GHIDRA_REPORT.md` §9, `SKIRMISH_TRACKBAR_CHANGED_VALUE_SOUND_GHIDRA_REPORT.md` (RulesClass__ReadAudioVisual). MEDIUM confidence for the in-game (Framework-A) cameo/button path specifically — directly verified for owner-draw shell controls (Framework B).

- **`lookup-tables`** — via the static read-only hit-test seed constants `0x007F5BE8`/`0x007F5BF4` (.rdata 1024/768) and the modifier-VK pairs `0x00A8EBF8..0x00A8EC0C` (OptionsClass +0x98..). The dispatch reads these as fixed substrate tables. Evidence: study §2.4, §5-G9/G14. (Lightweight edge — a handful of constants, not a large table.)

- **`ini-parsing`** — via OptionsClass modifier-key configuration: the SHIFT/CTRL/ALT VK pairs consumed by G9 are written by `OptionsClass::SetDefaults 0x005FA350` (defaults VK 0x10/0x11/0x12), with potential keyboard(md).ini runtime remap (YELLOW, §9). Edge is read-only consumption of parsed options. Evidence: study §2.4, §5-G9. LOW/MEDIUM confidence (only SetDefaults writes confirmed; INI-driven remap unverified).

## Used-by (incoming edges)

- **`logicclass`** — the tick spine calls `GScreenClass::Input` (which drives gadget Input) once per gameplay tick from `Main_Tick`. It depends on the gadget service to consume input and return the click result for that frame. Evidence: study §5-O2/O9.

- **`shell-dialog`** — reads the gadget service indirectly through the three meeting points: the open-dialog counter is read by in-game gadget code (so the shell publishes; the gadget reads — bidirectional), and the modal pump's coexistence decision gates the gadget world. The shared ToolTipManager is co-owned. Evidence: study §0, §2.6/§2.8, O12.

- **`cell-map`** (MapClass / tactical + minimap surface) — the full-tactical click catcher gadget `0x008A06F8` and the minimap/radar RTacticalClass region `0x00B04A10` are the **only** entry path for tactical-screen and minimap clicks: all map-cell selection/move/attack clicks route through the gadget walk before the Display/Radar cascade layers translate them to cell coordinates and queued commands. So MapClass-facing input depends on this service for click delivery. Evidence: study §1.A8, §5-A8/§5-A3 acceptance; gscreen-chain AI cascade (Display/Radar layers).

- **`frontier-render` / sidebar+radar chrome** — the sidebar/cameo strip painters and radar draw consume the gadget surface (they devirtualize ShapeButtonClass::Draw_Me and read gadget pressed/hover state for chrome). The render pass also draws the gadget Draw_Me(0) walk in composition order. Evidence: study §2.2, §5-O10; `SIDEBAR_CONSTRUCTION_GHIDRA_REPORT.md` §5.

## Open / unverified edges

- **`rules-class` / `frontier-audio` for the in-game cameo/button click sound** — directly verified for Framework-B owner-draw shell controls; for Framework-A ShapeButton/cameo the Voc-on-insert is verified (study §3.1) but the rule-field resolution of the specific index for in-game buttons is DOC-INHERITED / register-passed (study §9.2 "Voc indices for click/slide sounds register-passed").
- **`ini-parsing` modifier remap** — only `OptionsClass::SetDefaults` writes to the modifier-VK pairs are confirmed; an indexed keyboard(md).ini runtime remap would be invisible to the census (study §9.1, YELLOW). Edge confidence LOW.
- **No RNG edge (`random-scenario` RNG stream)** — S5 negative is scope-limited: "no RNG in UI" proven only for decompiled UI bodies, not corpus-wide (study §9.1/§5-S5). The ScenarioClass edge that IS real is rebuild-lifecycle (scenario start → Activate), not RNG.
- **`drawing-helpers` exact primitive set** — Draw_Me devirt edge is verified at the call-shape level; the precise SHP blitter entry points for each gadget class beyond ShapeButtonClass (TextLabel Set_Text-shape 0x0072A660, SelectClass strip painter) are partially DOC-INHERITED.
- **Two non-Main_Tick callers of GScreenClass::Input** (`FUN_00648350`/`FUN_00648710`, radar region) — roles unknown (study §9.1); could be an additional incoming edge into the dispatch from the radar/minimap path.
