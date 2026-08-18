# Adversarial verification pass — verdict ledger (gadget-dialog-20260610 study)

Lane: verification-pass (7th lane file, written 2026-06-10 during the study-doc patch
session). Purpose: make the study doc's highest evidence tier — the 17 named
adversarial-pass verdicts — auditable. The original adversarial session's tool log was
not saved as an artifact; this ledger reconstructs each verdict's evidence trail from
(a) the six lane worknotes, (b) the MCP calls cited inline in the study doc's §1 rows,
and (c) live re-verification calls made THIS session (logged in §3 below).

Evidence provenance classes used in §1/§2:
- **[L]** lane-logged — the underlying MCP call is recorded in the named lane worknote.
- **[S]** study-session-only — the call is cited inline in the study doc but appears in
  no saved lane log; the original session context is gone. Treat as one notch below [L];
  flagged per item. Where load-bearing, a live re-check was made ([R]).
- **[R]** re-verified live this patch session — call made and logged in §3.

## 1. The 17 verdicts

| # | Verdict id | Finding | Evidence |
|---|---|---|---|
| 1 | `input-heldbits-idle-only` | GadgetClass::Input 0x004E1640: held/up flag bits (0x2/0x8/0x20/0x80) are polled ONLY on key==0 ticks; a queued keyboard event yields flags exactly 0x100. Overturns prior GADGET doc §4.2 ("assembled every call"). | [L] gadget-core.md §2 steps 5–7 (decompile_function + disassemble_function 0x004E1640) |
| 2 | `sticky-process-decode` | Sticky_Process 0x004E1970: press bits 0x11 acquire g_StickyFocus iff IsSticky; release bits 0x44 release holder-only (or same-call acquirer). | [L] gadget-core.md §4 (read_memory 0x004E1970 ×48, hand decode); globals-registries.md §3.2 (independent byte decode) |
| 3 | `hittest-seed-tiebreak` | Hit_Test 0x004E15A0: half-open rects; winner = smallest area, signed `<=` tie-break (equal area → later gadget wins); seed best-area = constants 0x007F5BE8/0x007F5BF4 = 1024×768, zero writers. | [L] gadget-core.md §3.1 (decompile_function 0x004E15A0); globals-registries.md §1.1 (get_bulk_xrefs 0x007F5BE8/0x007F5BF4 zero writers; read_memory 0x007F5BE0). [S] disassemble 0x004E15A0 + list_segments (.rdata residency) — study-session-only |
| 4 | `toggle-action-decode` | ToggleClass::Action 0x00723EC0: silent press (key forced 0, consumed) + capture; fire `ID|0x8000` on release; Kind 1 flip / Kind 2 latch-ON-only; drag-off cancel via captured flags-0 hover tracking; release-outside boundary case fires if no idle tick cleared IsPressed. | [L] gadget-core.md §7 (read_memory 0x00723EC0 ×352, hand disassembly, load-bearing branches re-checked byte-by-byte) |
| 5 | `live-gadget-census` | 20-vtable single-spine family; exactly 9 classes live in YR; live population = CRT static initializers (sidebar/radar/commandbar/display TUs) + ONE runtime heap path (TextLabelClass, see correction C2); tactical catcher 0x008A06F8 (flags 0x7F) + minimap region 0x00B04A10 (flags 0x9F) are invisible sticky Action-only gadgets. | [L] gadget-family.md §1/§3/§4 (get_xrefs_to 0x004E1640/0x004E13F0/0x004E12F0 sweeps; full .text E8/E9 + imm32 scan of retail gamemd.exe; read_memory 0x004A8680/0x00652800; decompile_function 0x005D3BA0) |
| 6 | `dual-draw-pump` | Exactly two gadget draw pumps: the Input walk itself (Draw_Me per visited gadget, forced=1 on fresh list) and RenderFrame_main 0x004F4480's per-frame `head->vtbl[+0x2C](0)` draw-if-dirty walk (0x004E1570). No third pump (exhaustive head xref census). | [L] gscreen-chain.md §2 (get_xrefs_to 0x00A8EF54 exhaustive) + §5.2 step 6 (decompile_function 0x004F4480); gadget-core.md §4 (decompile_function 0x004E1570) |
| 7 | `buttons-head-writers` | Buttons head DAT_00A8EF54 has exactly 4 writers: One_Time 0x004F42A0 (=0), base Init_IO 0x004F42E0 (=0), Add_A_Button 0x004F4410 (tail append, double-insert reject), Remove_A_Button 0x004F4450 (unlink → new head). | [L] gscreen-chain.md §2 (get_xrefs_to 0x00A8EF54; decompile FUN_004f4410 / 0x004F4450); globals-registries.md §1.1 (read_memory 0x004F42A0/0x004F42E0) |
| 8 | `ai-cascade-order` | Chain AI dispatch order: MouseClass::AI 0x005BDDC0 → ScrollClass::AI 0x006922E0 → TabClass::AI 0x006D0680 → SidebarClass::AI 0x006A7780 → PowerClass::AI 0x0063FEA0 → RadarClass::AI 0x00653850 → DisplayClass::AI 0x004A9700 → GScreenClass::AI 0x004F4BB0. | [L] gscreen-chain.md §4 (decompile_function 0x005BDDC0/0x006922E0/0x006D0680/0x006A7780; get_function_callees 0x0063FEA0/0x00653850/0x004A9700) |
| 9 | `queued-events-in-logic-update` (OVERTURN) | gscreen-chain §4.2's claim "commands enter the sim inside LogicClass::Update via Process_QueuedEvents 0x0053B560" is WRONG. 0x0053B560's Ghidra label is drift — its body is a 3-state screen-flash machine, not an event-queue executor. Real command entry: Main_Tick 0x0055D360 → FUN_00647260 → FUN_0064C380 → EventClass__Execute 0x004C6CB0, in Main_Tick's tail block AFTER LogicClass::Update 0x0055AFB0 (object-sim pass) and RenderFrame_main, before Network_ServiceLoop. | [R] §3 below (5 calls this session) |
| 10 | `hover-global-sole-writer` | g_HoveredGadget 0x008B3E94: single writer (Input 0x004E16F0) and single reader (Input 0x004E16D9) program-wide; destructors do NOT clear it (stale-pointer hazard → contract G7). | [L] gadget-core.md §5/§8 (get_bulk_xrefs 0x008B3E94; dtor body reads 0x004E1390/0x004E1A60); globals-registries.md §1.1 |
| 11 | `clickedon-sticky-kbd-bypass` | Clicked_On 0x004E13F0: per-gadget mask applied first; the sticky holder bypasses bounds AND the flags==0 early-out; masked-in keyboard events (0x100) bypass the bounds test; keyboard-focus tier requires `flags & 0x100`. | [L] gadget-core.md §3.2 (decompile_function 0x004E13F0) + §2 step 8 (disassemble_function 0x004E1640) |
| 12 | `kbd-routing-accel-registries` | Process_NetworkMessages 0x005D4D50 keyboard routing is three-stage: registration-order IsDialogMessageA over the HWND vector (0x00ABFC90 object) → TranslateAcceleratorA over the {HACCEL,HWND} registry 0x00ABFCBC/count 0x00ABFCC8 → optional message-filter hook 0x00ABFD34 → Translate/Dispatch. | [L] dialog-delta.md #14 (decompile_function 0x005D4E70/0x005D4ED0/0x005D4D50); globals-registries.md §2.1 |
| 13 | `tooltip-inclusive-edges` | ToolTipManager::ProcessMessage 0x00724200 has exactly one caller (Process_NetworkMessages); 'TTIP' SetTimer with ms delay fields (this[+0x228] delay, this[+0x230] re-arm); region point test is inclusive on BOTH edges — deliberately different from the gadget half-open rule. | [L] globals-registries.md §2.4 (decompile_function 0x00724200); gscreen-chain.md §7 (get_function_callers 0x00724200; get_xrefs_to 0x00887368) |
| 14 | `ownerdraw-button-assets` | Owner-draw button 0x00612B70: PCX names always `b{u,d}e_{li,mi,ri}{24,30}.pcx` (second char hardcoded 'e'); disabled = 50% black AlphaBlend, never `bud_*`; asset-1 = SDBTNANM.SHP frames 2 idle / 3 flash (1 Hz timer) / 4 pressed. | [L] dialog-delta.md §2.3 (decompile_function 0x00612B70). [S] decompile 0x00621B80 + read_memory 0x0083587C.. — study-session-only |
| 15 | `include-set-55-ids` | Reposition/slide/paint gating runs off a 55-dialog-id include-set (0x0060C540) plus a disjoint 19-id mode-2 modal set (0x00622820) — not the 4 ids the prior shell doc listed. | [L] dialog-delta.md #9/#15 + §5.1/§5.2 (decompile_function 0x0060C540, 0x00622820) |
| 16 | `record-data-root-convention` | The 0x208 per-control record's data root = bucket+4 (helpers return bucket+4); prior doc's §2.4 map mixed two conventions; +0xB0 is ONE dual-role field (parent paint-mode AND button paint-asset); there is no separate +0xB4 field. | [L] dialog-delta.md headline + §3 (decompile_function 0x00624760 et al.; single-convention record map). [S] "binary-wide byte-pattern scan: zero +0xB4 writers" — study-session-only; lane equivalent is dialog-delta §3's closing convention note |
| 17 | `modal-pump-matrix` | Pump body 0x00623120: Process_NetworkMessages always first; offline (g_GameMode∈{0,5}) or blockers 0x00A8D60E/0x00A8DAB4 → Network_ServiceLoop only (sim+render+frame counter frozen); in-game network modes → guarded Main_Tick (sim advances; gadget input + render suppressed by the g_GameState gate; reentrancy byte DAT_00ABCD58). | [L] gscreen-chain.md §6.1/§6.2 (decompile_function 0x00623120, 0x004E1D00); dialog-delta.md #8; globals-registries.md §2.5 |

## 2. The three lane-claim overturns (exec-summary corrections)

| # | Correction | Overturned lane claim | Evidence |
|---|---|---|---|
| C1 | ShapeButtonClass does NOT repurpose the Set_Peer slot. Vtable 0x007E8088 is 35 slots: +0x84 = 0x0048E600 (Set_Peer, INHERITED from ControlClass), +0x88 = 0x0069DE00 (Set_Shape, NEW slot 34); the vtable ends at +0x88 (next dword 0x00800010 is the RTTI COL of vtable__ToggleClass 0x007E8118, whose slot 0 = 0x004B5810 follows). | gadget-core.md §9.1 ("+0x84 = 0x0069DE00 Set_Shape … REPURPOSED") — a one-slot misread; gadget-family.md §5's row ("+34 Set_Shape (new)") was correct. | [R] read_memory 0x007E8088 ×152 this session (§3) |
| C2 | The live gadget population is NOT built purely by CRT static initializers: TextLabelClass gadgets (0x4C bytes) are heap-built at runtime by MessageListClass::Add_Message (operator_new(0x4C) + ctor 0x0072A440; call sites 0x005D3D5B / 0x005D430B). | An earlier lane-draft "all live gadgets are static-init" generalization (final gadget-family.md already carries the correction). | [L] gadget-family.md §3.10/§4.2 (decompile_function 0x005D3BA0; get_xrefs_to 0x0072A440) |
| C3 | Player commands do not enter the sim inside LogicClass::Update — see verdict 9. | gscreen-chain.md §4.2. | [R] §3 below |

## 3. Live re-verification log — 2026-06-10 patch session (all read-only)

1. `get_function_callers 0x00647260` → exactly one caller: `Main_Tick @ 0x0055D360`.
2. `get_function_callees 0x00647260` → includes `FUN_0064C380` (also keyboard
   Check/Get 0x0054F000/0x0054F050, timer helpers, timeGetTime) — the
   0x00647260→0x0064C380 edge.
3. `get_function_callers 0x004C6CB0` → exactly one caller: `FUN_0064C380 @ 0x0064C380`.
4. `get_function_by_address 0x004C6CB0` → `EventClass__Execute`, body
   0x004C6CB0–0x004C8113.
5. `decompile_function 0x0055D360` (Main_Tick) — verified call order:
   GScreenClass__Input → Process_Command → Map__Logic → RenderFrame_main →
   (record/playback service) → FUN_00551A30 → **LogicClassPerTickUpdateLiveVector
   (= LogicClass::Update 0x0055AFB0, the object-sim pass)** → scroll-modifier polling +
   FUN_00637550/FUN_005D4430 service → frame-time accounting → **FUN_00647260()** →
   FUN_00637550 → Network_ServiceLoop → pause-byte gate → g_CurrentFrameCounter++ →
   FUN_0055E160. So the command-entry call sits AFTER the object-sim pass and after
   render, before network service and the frame-counter increment.
   **Reconciliation note:** gscreen-chain.md §5's Main_Tick step list omits the
   0x00647260 call — it is subsumed in that list's step 9 ("Sound/radar service,
   Network_ServiceLoop"); the step list is otherwise consistent with this decompile.
6. `decompile_function 0x0053B560` — the Ghidra label `Process_QueuedEvents` is
   **label drift**: the body is a 3-state (DAT_00A9FAB0 = 1/2/3) screen-flash state
   machine — DD RGB-shift white fill via the Composite surface 0x0088731C vtable,
   surface-pointer swap, a forced RenderFrame_main, VocClass__PlayAtPos /
   SoundEvent__Release / VoxClass__ResumeEVA calls. No event queue, no EventClass
   reference anywhere in the body. (The Main_Tick scenario-delay branch's call to
   0x0053B560 is therefore also this flash service, not command entry.)
7. `read_memory 0x007E8088 ×152` — ShapeButtonClass vtable dump for C1: slot values
   +0x6C=0x0069DEB0 Draw_Me, +0x7C=0x00723EC0 ToggleClass::Action, +0x80=0x004E13F0
   Clicked_On, **+0x84=0x0048E600 Set_Peer (inherited)**, **+0x88=0x0069DE00 Set_Shape
   (new)**; +0x8C=0x00800010 (next vtable's COL), +0x90=0x004B5810 (ToggleClass dtor =
   vtable 0x007E8118 slot 0).

Identity caveat (stated, not hidden): the names "Queue_AI" for 0x00647260 and "DoList"
for 0x0064C380 are INFERRED from RA-source shape and the verified call topology; the
EDGES and the in-Main_Tick position are verified above, the *names* are not
binary-derived. EventClass__Execute 0x004C6CB0 is the existing Ghidra label
(navigation hint; body not re-read this session beyond the caller check).

## 4. Items deliberately left YELLOW

Anything a verdict above could not anchor to a saved call stays out of the study doc's
verified body — harvested into study §9: the [S]-flagged sub-citations (list_segments
for the hit-test seed constants' segment; 0x00621B80/0x0083587C for owner-draw PCX
strings; the +0xB4 binary-wide byte scan), plus the per-lane YELLOW sections.
