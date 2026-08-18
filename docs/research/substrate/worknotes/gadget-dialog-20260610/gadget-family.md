# Framework A — GadgetClass family census (gamemd.exe)

Lane worknotes, 2026-06-10. RESEARCH ONLY; Ghidra read-only.
Method: enumerate every vtable that carries inherited base methods
(`GadgetClass::Input 0x004E1640` slot +0x28, `Clicked_On 0x004E13F0` slot +0x80),
read each vtable raw, diff slots vs base, identify class via dtor/ctor decompile,
then classify instantiation sites.

Evidence classes: **VERIFIED-LIVE** (this session, MCP call cited),
**DOC-INHERITED** (cite doc), **INFERRED** (stated as such).

## 0. Anchor re-verification (VERIFIED-LIVE)

- `GadgetClass` vtable @ `0x007E92BC`: raw read matches prior doc layout —
  slot10(+0x28)=`0x004E1640` Input, slot18(+0x48)=`0x004E1960` Flag_To_Redraw,
  slot27(+0x6C)=`0x004E1550` Draw_Me, slot31(+0x7C)=`0x004E1530` Action,
  slot32(+0x80)=`0x004E13F0` Clicked_On (verified via read_memory 0x007E92BC ×144).
- `ControlClass` vtable @ `0x007E528C`: overrides slot0 dtor `0x0048E660`,
  slot17 Get_ID `0x0048E610`, slot27 Draw_Me `0x0048E620`, slot31 Action
  `0x0048E5A0`, and APPENDS slot33 Set_Peer `0x0048E600`
  (verified via read_memory 0x007E528C ×144).
- **Label-drift note:** prior doc (GADGET_UI_FRAMEWORK report §8) lists base
  GadgetClass vtable slot33=`0x00800AE0` "likely terminator" and slot34
  `0x004E1AD0` "LinkClass helper". Live read confirms slot33 of the BASE vtable
  is the data-segment address `0x00800AE0` → the base vtable ends at slot32
  (33 slots); `0x004E1AD0` after it is unrelated adjacent data, not a slot.
  ControlClass's 34-slot table (Set_Peer appended) is the first widening.

## 1. Family enumeration method + completeness argument

- `get_xrefs_to 0x004E1640` (Input) DATA refs → 20 vtables (each ref at
  vtable+0x28).
- `get_xrefs_to 0x004E13F0` (Clicked_On) DATA refs → the SAME 20 vtables
  (each ref at vtable+0x80).
- Both lists agree ⇒ no family member overrides Input or Clicked_On.
  (A class overriding only one would still appear in the other list; a class
  overriding both would be missed — cross-checked below with a third universal
  slot, see §1.1.)

The 20 vtables (base = ref − slot offset):

| # | vtable | identified as |
|---|---|---|
| 1 | 0x007E92BC | GadgetClass (base) |
| 2 | 0x007E528C | ControlClass |
| 3 | 0x007E8118 | ToggleClass |
| 4 | 0x007E8088 | ShapeButtonClass |
| 5 | 0x007ED10C | ListClass |
| 6 | 0x007E4F84 | CheckListClass |
| 7 | 0x007E5054 | ColorListClass |
| 8 | 0x007E7FCC | DropListClass |
| 9 | 0x007E81A4 | EditClass |
| 10 | 0x007ED21C | SliderClass (slots → 0x006B1xxx; see §3.11) |
| 11 | 0x007E9384 | GaugeClass (see §3.12 — CONTRADICTS prior doc) |
| 12 | 0x007E9430 | TriColorGaugeClass (Draw_Me-only delta vs #11) |
| 13 | 0x007F5B44 | TextLabelClass (Draw_Me 0x0072A4A0 next to ctor 0x0072A440) |
| 14 | 0x007F55DC | TextButtonClass (tentative; Action = ToggleClass::Action 0x00723EC0, code 0x0071Fxxx–0x00720xxx) |
| 15 | 0x007F2FCC | SelectClass/StripClass cameo button (tentative; code 0x006AAxxx–0x006ACxxx, Mouse_Enter/Leave overridden) |
| 16 | 0x007F2F44 | SBGadgetClass (tentative; Gadget-derived, Action-only override 0x006ABA40) |
| 17 | 0x007F3EA0 | UNKNOWN-C (dtor 0x006C6880, Draw_Me 0x006C6640, +3 new virtuals) |
| 18 | 0x007E5E3C | UNKNOWN-F (dtor 0x004A59A0, ctor FUN_004a53b0, Action 0x004A5660) |
| 19 | 0x007E608C | UNKNOWN-G (dtor 0x004AEBB0, Action 0x004AAC10, base Draw_Me = invisible) |
| 20 | 0x007F02BC | UNKNOWN-J radar-area gadget (dtor 0x00658780, Action 0x006539D0, base Draw_Me = invisible) |

(verified via get_xrefs_to 0x004E1640, get_xrefs_to 0x004E13F0, and
read_memory of each vtable, this session)

Supporting ctor-chain evidence (VERIFIED-LIVE):
- `get_xrefs_to 0x004E12F0` (GadgetClass ctor): callers = ControlClass ctor
  0x0048E543, TextLabelClass ctor 0x0072A458, 0x004A86F1 (→ UNKNOWN-G family),
  0x00652884 (→ radar gadget), 0x006A4C21 (→ SBGadget, SidebarClass init).
- `get_xrefs_to 0x0048E520` (ControlClass ctor): callers = ToggleClass ctor
  0x00723E82, SelectClass__StripClass ctor 0x006A4DDB, FUN_004a53b0
  (→ UNKNOWN-F), FUN_004e2500 (→ GaugeClass ctor candidate).
- `get_xrefs_to 0x00723E60` (ToggleClass ctor): callers = ShapeButtonClass ctor
  only (2 sites 0x0069DCFB / 0x0069DD49 — two ctor variants).
  **⇒ TextButtonClass (#14) does NOT call ToggleClass ctor symbolically — needs
  check (its ctor may inline or call a different path).**

ListClass-family note: vtables #5/6/7 do NOT reference `0x004E1960`
Flag_To_Redraw (absent from its xref list) ⇒ ListClass overrides Flag_To_Redraw
(slot18 = 0x00558060-region impl; confirmed by reading 0x007ED10C below).

## 2. Raw vtable slot diffs (VERIFIED-LIVE, read_memory each)

Slot legend (byte offset into vtable): 0=dtor(+0x00), 17=Get_ID(+0x44),
18=Flag_To_Redraw(+0x48), 19=Peer_Callback(+0x4C), 20=Set_Focus(+0x50),
21=Clear_Focus(+0x54), 27=Draw_Me(+0x6C), 28=Mouse_Enter(+0x70),
29=Mouse_Leave(+0x74), 31=Action(+0x7C), 32=Clicked_On(+0x80),
33=Set_Peer(+0x84, ControlClass+).

### 2.1 SelectClass cameo button @ 0x007F2FCC (read_memory ×160)
- dtor 0x006AC780; Get_ID=Control 0x0048E610; Draw_Me=Control 0x0048E620
  (inherited!); **Mouse_Enter 0x006AB990, Mouse_Leave 0x006AB9E0 OVERRIDDEN**
  (tooltip hover hooks); Action 0x006AAD00; Set_Peer inherited 0x0048E600.
  34 slots, ends at data 0x008095B8.

### 2.2 SBGadgetClass @ 0x007F2F44 (read_memory ×160)
- dtor 0x006AC7A0; Get_ID=base 0x004AEBA0 (direct GadgetClass child);
  Draw_Me=base 0x004E1550 (no visual); Action OVERRIDDEN 0x006ABA40;
  33 slots (Gadget-size), ends at data 0x00809568. Next vtable in memory is
  0x007F2FCC (SelectClass) — same translation unit (sidebar).

### 2.3 SliderClass @ 0x007ED21C (read_memory ×176)
- dtor 0x00558180; Get_ID=Control; **Peer_Callback OVERRIDDEN 0x006B2160**;
  Draw_Me 0x006B20F0; Action 0x006B1F50; Set_Peer inherited 0x0048E600;
  then EXTENDED slots: 34=0x006B1D40, 35=0x006B1E50, 36=0x004E30A0,
  37=0x004E30B0, 38=0x00558120, 39=0x006B2070, 40=0x004E25D0, 41=0x004E2650,
  42=0x006B1DC0, 43=0x006B2000. Slots 36/37/40/41 point into the
  0x004E25xx–0x004E30xx region = inherited Gauge-layer methods ⇒ SliderClass
  derives from the 0x004E2xxx class (GaugeClass), overriding 34/35/38/39/42/43.

### 2.4 GaugeClass @ 0x007E9384 (read_memory ×160 — re-read longer below)
- dtor 0x004E30D0; Get_ID=Control 0x0048E610; Draw_Me 0x004E2690;
  Action 0x004E2830; Set_Peer inherited; appended slots 34=0x004E2580,
  35=0x004E25A0, 36=0x004E30A0, 37=0x004E30B0, 38=0x004E30C0, 39=0x004E29A0
  (+ more past 160-byte read — TBD).

### 2.5 TriColorGaugeClass @ 0x007E9430 (read_memory ×160)
- Identical to 0x007E9384 except dtor 0x004E30F0 and Draw_Me 0x004E2B50.
  Classic "override Draw_Me only" sibling.

### 2.6 TextLabelClass @ 0x007F5B44 (read_memory ×160)
- dtor 0x0072A670; Get_ID=base 0x004AEBA0 (direct Gadget child);
  Draw_Me 0x0072A4A0; Action=base 0x004E1530; appends slot33 0x0072A660
  (new virtual, Set_Text candidate). 34 slots.

### 2.7 TextButtonClass (tentative) @ 0x007F55DC (read_memory ×160)
- dtor 0x00720210; Get_ID=Control; Draw_Me 0x0071FFE0;
  **Action = ToggleClass::Action 0x00723EC0 inherited** ⇒ ToggleClass child;
  slot33 0x00720020 (vs Control Set_Peer 0x0048E600 — override or new virtual,
  TBD vs ToggleClass vtable), 34=0x00720200, 35=0x00720070, 36=0x00720140.
  37 slots.

### 2.8 UNKNOWN-C @ 0x007F3EA0 (read_memory ×160)
- dtor 0x006C6880; Get_ID=base (direct Gadget child); Set_Focus=base...
  Draw_Me 0x006C6640 OVERRIDDEN; Action=base 0x004E1530;
  appends 33=0x006C6680, 34=0x006C6740, 35=0x006C67F0. 36 slots.

### 2.9 UNKNOWN-F @ 0x007E5E3C (read_memory ×160)
- dtor 0x004A59A0; Get_ID=Control; Draw_Me 0x004A57B0; Action 0x004A5660;
  Set_Peer inherited. 34 slots (Control-shaped).

### 2.10 UNKNOWN-G @ 0x007E608C (read_memory ×160)
- dtor 0x004AEBB0; Get_ID=base; Draw_Me=base 0x004E1550 (invisible);
  Action OVERRIDDEN 0x004AAC10; 33 slots (Gadget-shaped).

### 2.11 UNKNOWN-J radar gadget @ 0x007F02BC (read_memory ×160)
- dtor 0x00658780; Get_ID=base; Draw_Me=base (invisible);
  Action OVERRIDDEN 0x006539D0; 33 slots (Gadget-shaped). Code region
  0x00653xxx/0x00658xxx = RadarClass translation unit.

## 3. Identifications (VERIFIED-LIVE this session)

### 3.1 GaugeClass EXISTS — prior doc claim REFUTED
- `FUN_004e2500` decompiled: calls `ControlClass__Constructor`, then
  `*this = &vtable__GaugeClass` and touches its own Flag_To_Redraw slot via
  `PTR_FUN_007e93cc` (= 0x007E9384+0x48) (verified via decompile_function
  0x004E2500). vtable__GaugeClass = **0x007E9384**, ctor = **0x004E2500**,
  dtor = 0x004E30D0.
- GADGET_UI_FRAMEWORK_GHIDRA_REPORT §1.1 "GaugeClass does NOT exist in
  gamemd.exe" is **WRONG** — it exists with full vtable, ctor, and a derived
  class. (It searched by name only; the Ghidra DB has the vtable labeled
  `vtable__GaugeClass`, so even the name claim was checkable.)
- Activity: ctor's ONLY disassembled caller is the SliderClass ctor at
  0x006B1B44 (verified via get_xrefs_to 0x004E2500). GaugeClass = ACTIVE as
  base layer of SliderClass; never instantiated standalone from disassembled
  code.

### 3.2 TriColorGaugeClass @ 0x007E9430
- Ctor = **0x004E2A50** (entry confirmed by preceding ret+nop padding,
  read_memory 0x004E2A30): inlines Gauge init (calls ControlClass ctor at
  0x004E2A74, writes Gauge vtable at ~0x004E2A83, then TriColor vtable at
  ~0x004E2AD5; byte hits 0x004E2A85/0x004E2AD7 = imm32 of the two writes,
  verified via search_byte_patterns "84 93 7e 00" / "30 94 7e 00").
- get_xrefs_to 0x004E2A50 → NONE. **DORMANT** (no disassembled instantiation;
  vtable has zero data refs).

### 3.3 Dial8Class EXISTS — prior doc claim REFUTED — DORMANT
- `FUN_004a53b0` decompiled: calls ControlClass ctor, sets
  `*this = &vtable__Dial8Class` (= 0x007E5E3C), computes 8 compass-point line
  endpoints from center/radius (verified via decompile_function 0x004A53B0).
- GADGET_UI_FRAMEWORK report §1.1 "Dial8Class fully stripped from YR" is
  **WRONG** — class + ctor + Draw_Me 0x004A57B0 + Action 0x004A5660 all
  present.
- get_xrefs_to 0x004A53B0 → NONE; only vtable ref is the ctor itself
  (search_byte_patterns "3c 5e 7e 00" → single hit 0x004A53DC). **DORMANT**
  (dead code retained by linker; no instantiation path).

### 3.4 StaticButtonClass @ 0x007F3EA0 (UNKNOWN-C resolved)
- Ghidra-labeled `vtable__StaticButtonClass` (via decompile_function
  0x006C6880 — that function is the scalar-deleting DESTRUCTOR mislabeled
  `StaticButtonClass__Constructor`; **label drift recorded**, same for the
  in-place dtor at ~0x006C6610).
- Param ctor = **0x006C6540** (calls GadgetClass ctor + PixelBuffer init at
  +0x24, writes vtable at ~0x006C6588; read_memory 0x006C6500/0x006C6550).
  Default ctor = **0x006C65D0** (read_memory 0x006C65D0; zero-rect, +0x34=2,
  vtable write imm at 0x006C6600).
- Direct GadgetClass child (slot17 Get_ID = base 0x004AEBA0); appends 3 new
  virtuals 0x006C6680 / 0x006C6740 / 0x006C67F0 (slots 33-35); owns a pixel
  buffer → draws a text/bitmap panel.
- get_xrefs_to 0x006C6540 and (pending) 0x006C65D0 → callers unresolved; see §4.

### 3.5 TextButtonClass @ 0x007F55DC (UNKNOWN-D resolved) — DORMANT-leaning
- Ctor = **0x0071FF20** (entry after nop padding; calls ToggleClass ctor
  0x00723E60 at 0x0071FF41; writes vtable at ~0x0071FF65; read_memory
  0x0071FEF0/0x0071FF30). ToggleClass child (slot31 Action = ToggleClass::
  Action 0x00723EC0 inherited); Draw_Me 0x0071FFE0; appends slots 33-36
  (0x00720020/0x00720200/0x00720070/0x00720140 — Set_Text family, RA1-shape).
- get_xrefs_to 0x0071FF20 → NONE; vtable byte hits only 0x0071FF67 (ctor) and
  0x0071FFC4 (in-place dtor) (search_byte_patterns "dc 55 7f 00").
  **DORMANT** unless a non-disassembled caller exists (see UNVERIFIED).

### 3.6 SelectClass (StripClass cameo button) @ 0x007F2FCC — ACTIVE
- Labeled ctor `SelectClass__StripClass__SidebarClass__Constructor` (calls
  ControlClass ctor at 0x006A4DDB; vtable write 0x006A4DE8).
- **Default ctor = 0x006AACB0** (hand-decoded read_memory 0x006AACA0):
  ControlClass ctor args → ID=0, x=0, y=0, **w=0x3C(60), h=0x30(48)** =
  exact cameo dimensions, flags=0x19, sticky=0; zeroes +0x2C/+0x30; writes
  SelectClass vtable.
- Overrides Mouse_Enter 0x006AB990 / Mouse_Leave 0x006AB9E0 (tooltip hover
  hooks) + Action 0x006AAD00; Draw_Me stays ControlClass (cameos are drawn by
  the strip painter, not gadget Draw_Me).
- Static array evidence: in-place destructor loop at ~0x006A4E20 destroys
  **240 objects of size 0x38** downward from 0x00B0B300 (i.e. array base
  0x00B07E80) — the global cameo-button pool (read_memory 0x006A4E20,
  hand-decoded `mov esi,0xB0B300 / mov edi,0xF0 / sub esi,0x38 / call dtor /
  dec edi / jnz`). ACTIVE-YR: sidebar cameo strips are these gadgets.

### 3.7 SBGadgetClass @ 0x007F2F44 — ACTIVE
- Direct GadgetClass child; ONLY override = Action 0x006ABA40 (+ dtor
  0x006AC7A0); Draw_Me = base no-op (invisible click region).
- Inline-constructed inside SidebarClass ctor (GadgetClass ctor call at
  0x006A4C21, vtable write 0x006A4C31 — get_xrefs_to 0x004E12F0 +
  search_byte_patterns "44 2f 7f 00").
- **Default ctor = 0x006A4E40** (read_memory 0x006A4E20: x=0,y=0,w=1,h=1,
  flags=8, sticky=0, then vtable write at ~0x006A4E54).
- **Label-drift note for prompt anchor:** "SBGadgetClass::Draw 0x0069DEB0" is
  WRONG as a class binding — 0x0069DEB0 is ShapeButtonClass::Draw_Me
  (vtable__ShapeButtonClass slot27 @0x007E80F4 region, confirmed by vtable
  read 0x007E8088); SBGadgetClass has NO Draw_Me override (slot27 =
  0x004E1550 base, read_memory 0x007F2F44).

### 3.8 DisplayClass tactical-screen gadget (RA1 "TacButton" analogue),
vtable 0x007E608C — ACTIVE
- Static CRT initializer at **0x004A86E0** (hand-decoded read_memory
  0x004A8680): constructs global BSS object **DAT_008A06F8** with
  GadgetClass ctor args x=0,y=0,w=0,h=0, flags=0x7F, sticky=1; vtable write
  at 0x004A86FB; registers atexit dtor 0x004A8710.
- Only override = Action **0x004AAC10** (DisplayClass code region); Draw_Me =
  base (invisible). This is the full-tactical-screen input catcher.
- Cross-lane corroboration: globals-registries.md lane found DAT_008A06F8
  re-added to the GScreen Buttons list at scenario init when !g_IsMapEditor.
- NAMING HAZARD: YRpp's "TacticalClass" (global ptr 0x00887324, code
  0x006D2xxx) is the viewport/projection class — a DIFFERENT object. This
  gadget is the RA1-style `TacticalClass : GadgetClass` input gadget living in
  DisplayClass's translation unit.

### 3.9 RTacticalClass radar-area gadget, vtable 0x007F02BC — ACTIVE
- Static CRT initializer at **0x00652870** (hand-decoded read_memory
  0x00652800): global object **0x00B04A10**, GadgetClass ctor args
  x=0,y=0,w=0,h=0, flags=0x9F, sticky=1; vtable write at 0x0065288E; atexit
  dtor 0x006528A0.
- Only override = Action **0x006539D0** (RadarClass region; minimap click
  handling — cf. MINIMAP_GADGETCLASS_CLICK_PROVENANCE_GHIDRA_REPORT).

### 3.10 TextLabelClass @ 0x007F5B44 — ACTIVE
- ctor 0x0072A440 (doc-known, confirmed via get_xrefs_to 0x004E12F0 showing
  base-ctor call at 0x0072A458); vtable write imm at 0x0072A490.
- Callers of ctor: FUN_005d3ba0 + FUN_005d4210 (get_function_callers
  TextLabelClass__Constructor). FUN_005d3ba0 decompiled =
  **MessageListClass::Add_Message**: walks a linked list of up to 14 (0xE)
  label slots, allocates `operator_new(0x4C)` TextLabelClass per message,
  sound via VocClass__PlayAtPos, recursive wrap for long text, g_GameMode
  branches (verified via decompile_function 0x005D3BA0). In-game chat/system
  messages → fires in every MP match and on system notifications. ACTIVE-YR.

### 3.11 SliderClass @ 0x007ED21C — ACTIVE (as ListClass scrollbar)
- Ctor = **0x006B1B20** (undefined-as-function code; entry located by
  hand-decode of read_memory 0x006B1AA0: prologue after nop padding; calls
  GaugeClass ctor 0x004E2500 at 0x006B1B44; writes vtable 0x007ED21C at
  0x006B1B51). **Proves SliderClass : GaugeClass : ControlClass.**
- Slider ctor also calls ShapeButtonClass ctor variant B (0x0069DD30) twice
  at 0x006B1B9D / 0x006B1BD7 (get_xrefs_to 0x0069DD30) — plus/minus arrow
  buttons owned by the slider.
- Overrides: Peer_Callback 0x006B2160 (slot19!), Draw_Me 0x006B20F0, Action
  0x006B1F50, Gauge-layer virtuals 34/35/38/39 (0x006B1D40, 0x006B1E50,
  0x00558120?, 0x006B2070) + extra slots 42/43 (0x006B1DC0, 0x006B2000).
- Instantiation: NO disassembled callers of 0x006B1B20 (get_xrefs_to → none);
  inlined into ListClass ctor (Slider vtable write at 0x00557522 inside the
  ListClass ctor region, search_byte_patterns "1c d2 7e 00"). So the only
  proven live role is the ListClass scroll bar.

### 3.12 Dropship Loadout screen (TS LEGACY, DORMANT-gated) — the 4 mystery
ShapeButtons
- `FUN_004b6c30` (body 0x004B6C30–0x004B9690; Ghidra label
  `CDFileClass__Constructor` is **label drift**, already recorded by
  BUTTON_FADE_EFFECT_TRIGGER_GHIDRA_REPORT) constructs 4 ShapeButtonClass
  buttons at 0x004B72DD / 0x004B7352 / 0x004B73B1 / 0x004B744C
  (get_xrefs_to 0x0069DD30).
- Identity: translation unit = **Dropship.cpp** — strings
  `D:\ra2mdpost\Dropship.cpp` @0x008204A8, `DROPSHIP.PAL` @0x008204EC,
  `LOADOUT.SHP` @0x00820518, INI key `StartingDropships` @0x0083E088
  (verified via search_strings "(?i)dropship|loadout"). Callees include
  BSurface/ConvertClass/ZBuffer ctors, LoadFileFromMIX, StringTable,
  Network_ServiceLoop, Random+Sqrt (starfield) (get_function_callees
  0x004B6C30).
- Reachability: single caller ScenarioClass::Start_Scenario @0x00683D97,
  gated `if (*(int*)(g_ScenarioClass_Instance{0x00A8B230} + 0x34D0) > 0)`
  (hand-decoded read_memory 0x00683D80). +0x34D0 = StartingDropships count
  (INFERRED from string adjacency; key parse site not traced).
  **TS-legacy: dormant in YR unless a map sets StartingDropships — code path
  is live-linked, not stripped.**

## 4. Instantiation census — byte-level ground truth

The UI translation units are heavily UN-DISASSEMBLED in this Ghidra DB:
ctors exist as raw bytes with no functions defined over their callers, so
`get_xrefs_to`/`get_function_callers` systematically under-report. To close
this, the on-disk retail `gamemd.exe` (5,286,504 bytes; PE sections .text
VA 0x401000 fileoff 0x1000 — sanity-checked: file dword @VA 0x007E92BC =
0x004E1A60 matches Ghidra read) was scanned exhaustively for **every E8
call and E9 jump** in .text targeting each family ctor, plus every absolute
imm32 reference (vector-ctor-iterator / CRT-table patterns). This is
complete over direct transfers; residual risk = register-indirect calls only
(see UNVERIFIED).

### 4.1 Scan results (full .text, VERIFIED-LIVE via local byte scan)

| ctor | direct callers found | verdict |
|---|---|---|
| ShapeButtonClass ctorA 0x0069DCF0 | 10: radar init 0x006528D5, 0x00652925; sidebar init 0x006A4C65, 0x006A4CA5, 0x006A4CEE, 0x006A4D45, 0x006A4D85; commandbar init 0x006CFBAE, 0x006CFC05, 0x006CFC45 | ACTIVE |
| ShapeButtonClass ctorB 0x0069DD30 | 9: dropship 0x004B72DD/0x004B7352/0x004B73B1/0x004B744C (dormant path); DropList ctorA 0x004B4E69 (dead); ListClass ctorA 0x00557292/0x005572AF (dead); Slider ctor 0x006B1B9D/0x006B1BD7 (dead) | ACTIVE only via ctorA sites; ctorB callers all dormant |
| TextLabelClass ctor 0x0072A440 | 2: 0x005D3D5B (MessageListClass::Add_Message), 0x005D430B (FUN_005d4210, second MessageList add path) | ACTIVE |
| GaugeClass ctor 0x004E2500 | 1: 0x006B1B44 (Slider ctor — itself dead, see below) | DORMANT at runtime |
| SliderClass ctor 0x006B1B20 | 1: 0x005572C3 (ListClass ctorA — dead) | DORMANT standalone; the LIVE-looking slider is inline in ListClass ctorB — which is itself dead |
| ListClass ctorB 0x00557380 | 1: 0x004B54AF (DropList ctorB — dead) | DORMANT |
| ListClass ctorA ~0x0055725x (region scan 0x00557240–0x005572C4) | 0 | DORMANT |
| DropListClass ctorA (entry zone 0x004B4D74–0x004B4E3F) | 0 | DORMANT |
| DropListClass ctorB 0x004B53E0 | 0 | DORMANT |
| EditClass ctor 0x004C2FC0 | 1: 0x004B4E3F (DropList ctorA — dead) | DORMANT |
| CheckListClass ctor 0x00488280 | 0 | DORMANT |
| ColorListClass ctor (region 0x00488700–0x004887D9) | 0 | DORMANT |
| TextButtonClass ctor 0x0071FF20 | 0 | DORMANT |
| StaticButtonClass ctors 0x006C6540 / 0x006C65D0 | 0 | DORMANT |
| TriColorGaugeClass ctor 0x004E2A50 | 0 | DORMANT |
| Dial8Class ctor 0x004A53B0 | 0 | DORMANT |
| SelectClass 240-array CRT initializer 0x006A4DC0 | referenced from CRT static-init table entry @0x00814B00 (.data) | ACTIVE (startup) |
| SelectClass default ctor 0x006AACB0 | 0 | dormant variant |
| SBGadgetClass default ctor 0x006A4E40 | 0 | dormant variant (live instance built inline by static init ~0x006A4C10) |

**Headline finding:** the entire shell-control wing of Framework A —
ListClass, DropListClass, EditClass, CheckListClass, ColorListClass,
SliderClass, GaugeClass, TriColorGaugeClass, TextButtonClass,
StaticButtonClass, Dial8Class — is **DORMANT in gamemd.exe**. RA2/YR replaced
the TS gadget shell with Win32 RT_DIALOG screens (Framework B); the gadget
shell controls survive as fully-linked dead code reachable only from each
other. The LIVE Framework A surface is exactly: GadgetClass/ControlClass/
ToggleClass (as bases), ShapeButtonClass, SelectClass, SBGadgetClass,
TextLabelClass, the tactical-screen gadget, and RTacticalClass.

### 4.2 The live in-game gadget population (all CRT static initializers,
hand-decoded from read_memory; objects in .data/BSS)

Sidebar TU initializer chain at 0x006A4C10–0x006A4DDF (read_memory
0x006A4C40 ×352; each initializer = `mov ecx,<global>; call ShapeButton
ctorA; push <teardown>; call atexit`):

| global | count×size | class | identity (DOC-INHERITED from SIDEBAR_SYSTEM_GHIDRA_REPORT global map) |
|---|---|---|---|
| 0x00B07C48 | 4 × 0x60 (loop init at 0x006A4CE0, edi=4, stride 0x60) | ShapeButtonClass | 4 tab buttons |
| 0x00B07DF8 | 1 × 0x60 | ShapeButtonClass | sell/scroll group per doc map |
| 0x00B07E58 | 1 × 0x28 | SBGadgetClass | sidebar body click zone (init ~0x006A4C10: GadgetClass ctor call @0x006A4C21, vtable write @0x006A4C31; teardown thunk at 0x006A4C40 jmps GadgetClass dtor 0x004E1390) |
| 0x00B07E80 | 240 × 0x38 (4 tabs × 60 slots) | SelectClass | cameo click gadgets (CRT init 0x006A4DC0, decompiled verbatim) |
| 0x00B0B328 | 1 × 0x60 | ShapeButtonClass | strip scroll (up/down family) |
| 0x00B0B3A0 | 1 × 0x60 | ShapeButtonClass | repair button (doc map: 0x00B0B3AC–0x00B0B3F0) |
| 0x00B0B408 | 1 × 0x60 | ShapeButtonClass | next object before SHP-pointer block @0x00B0B468 ⇒ last sidebar button |

Layout cross-check: 0x00B07C48+4·0x60=0x00B07DC8 ✓ doc; 0x00B07DF8+0x60=
0x00B07E58; +0x28=0x00B07E80; +240·0x38=0x00B0B300 ✓ doc; 0x00B0B408+0x60=
0x00B0B468 = doc's sidebar-SHP-pointer block. **ShapeButtonClass object size
= 0x60; SBGadgetClass = 0x28 (bare GadgetClass size); SelectClass = 0x38.**

RadarClass TU initializers (read_memory 0x00652800/0x006528B0):
- RTacticalClass global @ **0x00B04A10** (flags 0x9F, sticky, zero rect).
- ShapeButtonClass globals @ **0x00B04978** (init 0x006528D0) and
  **0x00B04910** (init 0x00652920) — the two radar-frame mode buttons.

CommandBar TU initializers (read_memory 0x006CFB80 ×224):
- **25-element ShapeButtonClass array @ 0x00B0C1C0, stride 0x60** (loop init
  at 0x006CFBA0, edi=0x19) — matches CommandBar_Dispatch ID range
  0x80D6..0x80EE (25 IDs, GADGET_UI_FRAMEWORK §12) exactly.
- Two singles @ **0x00B0CCB0** (init 0x006CFC10) and **0x00B0CC40**
  (init 0x006CFC40) — the sell-mode/repair-mode pair by ID adjacency
  (0x80F0/0x80F1; binding to which-is-which NOT verified).

DisplayClass TU: tactical-screen gadget global @ **0x008A06F8**
(init 0x004A86E0, flags 0x7F, sticky).

MessageListClass: heap TextLabelClass per visible message
(operator_new(0x4C) in Add_Message 0x005D3BA0) — **TextLabelClass object
size 0x4C**.

## 5. Vtable override diff — full family (VERIFIED-LIVE, read_memory each vtable)

Base slot values (GadgetClass): dtor 0x004E1A60 | Get_ID 0x004AEBA0 |
FlagRedraw 0x004E1960 | PeerCb 0x0048E650 | SetFocus 0x004E19A0 |
ClearFocus 0x004E19D0 | Draw_Me 0x004E1550 | MouseEnter 0x004E1510 |
MouseLeave 0x004E1520 | Action 0x004E1530 | Clicked_On 0x004E13F0.

| class (vtable) | slots | overrides vs parent (slot: addr) |
|---|---|---|
| GadgetClass (0x007E92BC) | 33 (0..32) | — base. NOTE: prior doc's "slot33/34" are adjacent data, not slots. |
| ControlClass (0x007E528C) | 34 | 0:0x0048E660, 17 Get_ID:0x0048E610, 27 Draw_Me:0x0048E620, 31 Action:0x0048E5A0; +33 Set_Peer:0x0048E600 (new) |
| ToggleClass (0x007E8118) | 34 | vs Control: 0:0x004B5810, 31 Action:0x00723EC0 only |
| TextButtonClass (0x007F55DC) | 37 | vs Toggle: 0:0x00720210, 27 Draw_Me:0x0071FFE0, 33:0x00720020(!overrides Set_Peer slot); +34..36 new: 0x00720200, 0x00720070, 0x00720140 |
| ShapeButtonClass (0x007E8088) | 35 | vs Toggle: 0:0x004B57F0, 27 Draw_Me:0x0069DEB0; +34 Set_Shape:0x0069DE00 (new) |
| ListClass (0x007ED10C) | 50 | vs Control: 0:0x00558060, 3/4/5 link-mgmt:0x00557E10/0x00557EB0/0x00557E60, 9 Unlink:0x00557F00, 18 FlagRedraw:0x00557FD0 (calls base — explains earlier xref), 19 PeerCb:0x00557B10, 25 Get_Rect:0x00557570, 27 Draw_Me:0x00557920, 31 Action:0x00557830, 33:0x005576A0; +34..49 new list API (0x00557BE0, 0x00557A20, 0x004886B0, 0x00557B00, 0x00557AE0, 0x00557AC0 Get_Item, 0x00557FB0, 0x00557730, 0x00557800, 0x00557CB0, 0x00558010, 0x00557F40, 0x00557D10, 0x00557B70, 0x00557A70, 0x00557D20) |
| CheckListClass (0x007E4F84) | 50 | vs List: 0:0x004886E0, 31 Action:0x004884A0, 33:0x00488310 (Add), 38:0x00488350, 39:0x00488360, 41:0x004886C0, 42:0x00488380, 44:0x004883F0, 45:0x004886D0, 49:0x00488520 |
| ColorListClass (0x007E5054) | 52 | vs List: 0:0x00488F00, 33:0x00488EE0, 41:0x00488910, 42:0x004888B0, 49:0x00488980; +50/51 new: 0x00488850, 0x00488960 |
| EditClass (0x007E81A4) | 39 | vs Control: 0:0x004B57D0, 20 Set_Focus:0x004C3570, 27 Draw_Me:0x004C3110, 31 Action:0x004C3190; +34..38 new: 0x004C30E0, 0x004B55A0, 0x004C32A0, 0x004C32E0, 0x004C3420 |
| DropListClass (0x007E7FCC) | 46 | vs Edit: 0:0x004B55F0, 3/4/5:0x004B4F20/0x004B4F40/0x004B4F60, 8:0x004B4EE0, 9:0x004B4F80, 19 PeerCb:0x004B50C0 (expand/collapse), 21 ClearFocus:0x004B50A0 (auto-collapse), 25 Get_Rect:0x004B54E0; +39..45 new: 0x004B4FB0, 0x004B4FF0, 0x004B5010, 0x004B5530, 0x004B5030, 0x004B55B0, 0x004B55D0. **Draw_Me/Action slots = 0x004C3110/0x004C3190 INHERITED from EditClass — prior doc's "DropListClass::Draw_Me/Action" naming for these addresses is an identity drift (they are EditClass methods).** |
| GaugeClass (0x007E9384) | ≥40 | vs Control: 0:0x004E30D0, 27 Draw_Me:0x004E2690, 31 Action:0x004E2830; +34..39 new gauge API: 0x004E2580, 0x004E25A0, 0x004E30A0, 0x004E30B0, 0x004E30C0, 0x004E29A0 (read truncated at 40 dwords; Slider inherits 0x004E25D0/0x004E2650 at 40/41 ⇒ Gauge has ≥42 slots) |
| TriColorGaugeClass (0x007E9430) | as Gauge | vs Gauge: 0:0x004E30F0, 27 Draw_Me:0x004E2B50 only |
| SliderClass (0x007ED21C) | 44 | vs Gauge: 0:0x00558180, 19 PeerCb:0x006B2160, 27 Draw_Me:0x006B20F0, 31 Action:0x006B1F50, 34:0x006B1D40, 35:0x006B1E50, 38:0x00558120, 39:0x006B2070; +42/43: 0x006B1DC0, 0x006B2000 |
| TextLabelClass (0x007F5B44) | 34 | vs Gadget: 0:0x0072A670, 27 Draw_Me:0x0072A4A0; +33 new (Set_Text-shape): 0x0072A660 |
| StaticButtonClass (0x007F3EA0) | 36 | vs Gadget: 0:0x006C6880(scalar)/in-place ~0x006C6610, 27 Draw_Me:0x006C6640; +33..35 new: 0x006C6680, 0x006C6740, 0x006C67F0 |
| SelectClass (0x007F2FCC) | 34 | vs Control: 0:0x006AC780, 28 Mouse_Enter:0x006AB990, 29 Mouse_Leave:0x006AB9E0, 31 Action:0x006AAD00 (Draw_Me NOT overridden — strip painter draws cameos) |
| SBGadgetClass (0x007F2F44) | 33 | vs Gadget: 0:0x006AC7A0, 31 Action:0x006ABA40 only (invisible) |
| Tactical gadget (0x007E608C) | 33 | vs Gadget: 0:0x004AEBB0, 31 Action:0x004AAC10 only (invisible) |
| RTacticalClass (0x007F02BC) | 33 | vs Gadget: 0:0x00658780, 31 Action:0x006539D0 only (invisible) |

No family member overrides Input (slot10), Clicked_On (slot32), Hit_Test
(non-virtual 0x004E15A0), or Get_Next (slot1) — the dispatch spine is shared
by all 20 classes (triple xref cross-check §1).

## 6. CENSUS TABLE (final)

| class | ctor | vtable | key overrides | instantiation | ACTIVE-YR verdict |
|---|---|---|---|---|---|
| GadgetClass | 0x004E12F0 | 0x007E92BC | base | base subobject only (self vtable refs = own ctor/dtors) | ACTIVE (substrate) |
| ControlClass | 0x0048E520 | 0x007E528C | Get_ID/Draw_Me/Action/Set_Peer | base subobject only | ACTIVE (substrate) |
| ToggleClass | 0x00723E60 | 0x007E8118 | Action (press/release SM) | base of ShapeButton/TextButton only (callers: 2 ShapeButton ctor variants + TextButton ctor 0x0071FF41) | ACTIVE (substrate) |
| ShapeButtonClass | A:0x0069DCF0 B:0x0069DD30 | 0x007E8088 | Draw_Me 0x0069DEB0, +Set_Shape 0x0069DE00 | 37 live globals: 4 tabs + 4 sidebar singles + 2 radar + 25 cmdbar array + 2 cmdbar singles (all CRT static init); dormant: 4 dropship, list/slider/droplist children | **ACTIVE** |
| SelectClass (StripClass cameo) | CRT init 0x006A4DC0 (default-ctor variant 0x006AACB0 dormant) | 0x007F2FCC | Mouse_Enter/Leave (tooltips), Action 0x006AAD00 | 240 × 0x38 @0x00B07E80, built at startup | **ACTIVE** |
| SBGadgetClass | inline @~0x006A4C10 (default 0x006A4E40 dormant) | 0x007F2F44 | Action 0x006ABA40 | 1 global @0x00B07E58 | **ACTIVE** |
| TextLabelClass | 0x0072A440 | 0x007F5B44 | Draw_Me, +Set-text virtual | heap, MessageListClass::Add_Message (0x005D3D5B, 0x005D430B), size 0x4C | **ACTIVE** (chat/system messages) |
| Tactical-screen gadget | static init 0x004A86E0 | 0x007E608C | Action 0x004AAC10 | 1 global @0x008A06F8 | **ACTIVE** (all tactical clicks) |
| RTacticalClass | static init 0x00652870 | 0x007F02BC | Action 0x006539D0 | 1 global @0x00B04A10 | **ACTIVE** (minimap clicks) |
| ListClass | A ~0x0055725x, B 0x00557380 | 0x007ED10C | 10 overrides + 16 new | only DropList ctorB (dead) | **DORMANT** |
| DropListClass | A ~0x004B4Dxx, B 0x004B53E0 | 0x007E7FCC | 9 overrides + 7 new | none | **DORMANT** |
| EditClass | 0x004C2FC0 | 0x007E81A4 | Set_Focus/Draw_Me/Action + 5 new | only DropList ctorA (dead) | **DORMANT** |
| CheckListClass | 0x00488280 | 0x007E4F84 | Action/Add + 8 | none | **DORMANT** |
| ColorListClass | ~0x004887xx | 0x007E5054 | 5 + 2 new | none | **DORMANT** |
| SliderClass | 0x006B1B20 | 0x007ED21C | 8 + 2 new | only ListClass ctorA (dead) | **DORMANT** |
| GaugeClass | 0x004E2500 | 0x007E9384 | Draw_Me/Action + 6 new | only Slider ctor (dead) | **DORMANT** (exists; prior doc wrong) |
| TriColorGaugeClass | 0x004E2A50 | 0x007E9430 | Draw_Me only | none | **DORMANT** |
| TextButtonClass | 0x0071FF20 | 0x007F55DC | Draw_Me + 4 | none | **DORMANT** |
| StaticButtonClass | 0x006C6540 / 0x006C65D0 | 0x007F3EA0 | Draw_Me + 3 new | none | **DORMANT** |
| Dial8Class | 0x004A53B0 | 0x007E5E3C | Draw_Me 0x004A57B0, Action 0x004A5660 | none | **DORMANT** (exists; prior doc wrong) |

## 7. Behavioral notes for parity (mixed evidence)

- **Tooltip hook is gadget-level:** SelectClass overrides Mouse_Enter
  (0x006AB990) / Mouse_Leave (0x006AB9E0) — the only family member that does.
  Sidebar tooltips key off these, with text resolution in
  SidebarClass::ResolveTooltip 0x006AC210 (DOC-INHERITED:
  SIDEBAR_SYSTEM/SIDEBAR_TIMING_AND_TOOLTIPS reports).
- **All live buttons share ToggleClass::Action 0x00723EC0** as the
  press/release state machine (ShapeButton inherits slot31). No sound is
  played inside the gadget layer for sidebar buttons; click sounds come from
  downstream Action-ID consumers. The one gadget-adjacent sound VERIFIED-LIVE
  this session: MessageListClass::Add_Message plays VocClass__PlayAtPos when
  inserting a chat line (decompile 0x005D3BA0).
- **Command-bar = 25 ShapeButtons** (IDs 0x80D6..0x80EE, array @0x00B0C1C0)
  + 2 singles — the in-game command bar is pure Framework A, constructed at
  process startup, not at scenario start.
- **ListClass composes its scrollbar inline:** ctorB inline-constructs
  Toggle→ShapeButton up (write 0x0055740B/0x00557462), Toggle→ShapeButton
  down (0x0055747F/0x005574E0), Slider (0x00557522), then own vtable
  (0x00557544). SliderClass standalone ctor additionally builds 2 own
  ShapeButtons (0x006B1B9D/0x006B1BD7). All dormant.
- **DropListClass : EditClass** (proven by slot inheritance 20/27/31 and
  Edit-ctor call 0x004B4E3F inside DropList ctorA);
  **SliderClass : GaugeClass : ControlClass** (proven by Gauge-ctor call
  0x006B1B44 + vtable layering).
- The five base-Draw_Me "invisible" gadgets (SBGadget, Tactical, RTactical)
  rely on Action-only dispatch — their hit rects are set at runtime by
  sidebar/display/radar layout code, not at construction (ctors pass zero or
  1×1 rects).

## 8. UNVERIFIED (YELLOW)

- **Dormancy scans cover E8/E9 direct transfers + imm32 references only.**
  A register-indirect `call eax` to a dead-listed ctor with the address
  computed (not stored as imm32 or in .data) would evade the scan. No such
  pattern is plausible for VC++6 ctor calls, but it is not excluded.
- **Family completeness** rests on no derived class overriding ALL of
  Input+Clicked_On+Get_Next simultaneously (three independent xref sweeps
  agreed on the same 20 vtables). A hypothetical gadget overriding all three
  base slots would be missed.
- Which sidebar single ShapeButton global (0x00B07DF8 / 0x00B0B328 /
  0x00B0B3A0 / 0x00B0B408) is sell vs repair vs scroll-up vs scroll-down —
  partially DOC-INHERITED (repair @0x00B0B3A0 region per SIDEBAR doc);
  exact per-global binding not verified this session (SHP names/coords not
  decoded from the initializers' Set_Shape calls).
- Which commandbar single (0x00B0CCB0 vs 0x00B0CC40) is sell-mode vs
  repair-mode (IDs 0x80F0/0x80F1) — INFERRED from dispatch-ID adjacency.
- ScenarioClass+0x34D0 = "StartingDropships count" — INFERRED from the
  Dropship.cpp string cluster + gate shape; the INI-key parse site writing
  +0x34D0 was not traced.
- GaugeClass true vtable slot count (read stopped at 40 dwords; ≥42 implied
  by Slider's inherited slots 40/41).
- The radar ShapeButton pair's player-visible roles (which SHP each shows)
  not decoded.
- The two MessageListClass Add paths' caller sets (who calls FUN_005d3ba0 /
  FUN_005d4210 besides chat) not enumerated.

## 9. Label-drift log (for Ghidra cleanup later — NOT applied, read-only)

| address | current label | actual role (evidence) |
|---|---|---|
| 0x004B6C30 | CDFileClass__Constructor | Dropship_Loadout screen (Dropship.cpp strings; called once from Start_Scenario 0x00683D97) |
| 0x004B4D50 | DriveLocomotionClass__Release_Piggybacked_Helper | generic COM Release-pointer helper (44 call sites across shell/WOL) |
| 0x006C6880 | StaticButtonClass__Constructor | StaticButtonClass scalar-deleting DESTRUCTOR |
| ~0x006C6610 | StaticButtonClass__Constructor (per decompile of 0x006C6616) | StaticButtonClass in-place destructor |
| 0x006A4DC0 | SelectClass__StripClass__SidebarClass__Constructor | CRT static initializer that builds the 240-element cameo array (not a per-object ctor) |
| 0x0069DEB0 | (prompt anchor "SBGadgetClass::Draw") | ShapeButtonClass::Draw_Me (ShapeButton vtable slot27; SBGadget Draw_Me = base 0x004E1550) |
| 0x004C3110 / 0x004C3190 | doc-named DropListClass::Draw_Me / ::Action | EditClass::Draw_Me / ::Action (DropList inherits both) |
| inside dtors | "GadgetClass__Constructor" callee | GadgetClass destructor 0x004E1390 (ctor/dtor label swap in dtor bodies) |

## 10. MCP call log (verification anchors)

read_memory: 0x007E92BC, 0x007E528C, 0x007F2FCC, 0x007ED21C, 0x007F3EA0,
0x007F55DC, 0x007F5B44, 0x007E5E3C, 0x007E608C, 0x007E9384, 0x007E9430,
0x007F02BC, 0x007F2F44, 0x007E8118, 0x007E8088, 0x007ED10C, 0x007E7FCC,
0x007E81A4, 0x007E4F84, 0x007E5054 (vtables); 0x004A8680, 0x00652800,
0x006B1AA0, 0x004E2A30/50, 0x0071FEF0/30, 0x006C6500/50/D0, 0x006AACA0,
0x006A4E20, 0x006A4C40, 0x006528B0, 0x006CFB80, 0x00683D80, 0x004B72A0/B8,
0x004C2FB0, 0x005572C0, 0x005573E0, 0x004B4E60, 0x004B53C0, 0x00488280,
0x004887A0, 0x00488F00 (code windows).
get_xrefs_to: 0x004E1960, 0x004E1640, 0x004E13F0, 0x004E14A0, 0x004E12F0,
0x0048E520, 0x00723E60, 0x0069DCF0, 0x0069DD30, 0x004E2500, 0x004A53B0,
0x004E2A50, 0x006B1B20, 0x0071FF20, 0x006C6540, 0x006C65D0, 0x004C2FC0,
0x00488280, 0x004B53E0, 0x006AACB0, 0x006A4E40, 0x006A4DD0, 0x004B6C30,
0x006C6680.
decompile_function: 0x006C6880, 0x004A53B0, 0x004E2500, 0x004AEBB0,
0x00658780, 0x006C6616, 0x005D3BA0, 0x004B4D50, 0x006A4DC0.
get_function_callers: ShapeButtonClass__Constructor, TextLabelClass ctor,
FUN_004b9890. get_function_callees: 0x004B6C30. get_function_by_address:
0x004B6C30, 0x006A4DDB, others. search_byte_patterns: 15 vtable imm32
patterns. search_strings: dropship/loadout. Local file scan: full .text
E8/E9+imm32 sweep of retail gamemd.exe (sanity-anchored to Ghidra bytes).
