# A2 grounding — cameo strip (SelectClass) behavior contract

Grounding worknote for the gadget-dialog substrate Rust plan, item **A2 — the cameo
strip click surface (SelectClass cameo gadgets)**. Decoded live from gamemd.exe via
Ghidra MCP this session. Seed addresses came from
`GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md`; every load-bearing claim below
is re-verified with an inline Ghidra-call citation. Confidence tags: **VERIFIED-LIVE** =
read from the binary this session; **INFERRED** = deduced from verified pieces, not read
directly.

Image base 0x00400000. Gadget field layout (from the study, used as given): GadgetClass
+0x0C X, +0x10 Y, +0x14 W, +0x18 H, +0x1C IsToRedraw u8, +0x1D IsSticky u8, +0x1E
IsDisabled u8, +0x20 Flags u32; sizeof 0x24. ControlClass adds +0x24 ID, +0x28 Peer.
Event flag bits: 0x1 LEFTPRESS, 0x2 LEFTHELD, 0x4 LEFTRELEASE, 0x8 LEFTUP, 0x10
RIGHTPRESS, 0x20 RIGHTHELD, 0x40 RIGHTRELEASE, 0x80 RIGHTUP, 0x100 KEYBOARD.

---

## 0. Headline answers (the questions the plan needs)

| Question | Answer | Evidence |
|---|---|---|
| Fire on mouse-DOWN or RELEASE? | **mouse-DOWN (press).** Acts on LEFTPRESS (0x1) and RIGHTPRESS (0x10). It explicitly DISCARDS the LEFTUP bit (0x8). No release-edge branch. | decompile 0x006AAD00; disassemble 0x006AAD00 |
| Posts `ID\|0x8000` for SidebarClass to consume, or calls build code directly? | **Calls build/production/placement code DIRECTLY in-place** (FactoryClass + command-queue writes + Begin_Building_Placement). It ALSO tail-chains ControlClass::Action which posts `ID\|0x8000`, but the cameo ID (1000+slot) is NOT in SidebarClass::Action's handled set, so the posted key is inert. The visible effect is produced inside SelectClass::Action, not by a later consumer. | decompile 0x006AAD00; disassemble tail `006ab970 CALL 0x0048e5a0`; decompile 0x0048E5A0; decompile 0x006A7780 (consumer set) |
| Sticky / ToggleClass press-capture? | **NO.** Plain ControlClass. Constructed with IsSticky=0. No ToggleClass machine, no press-capture, no +0x2C IsPressed field. | decompile 0x006A4DC0 → ControlClass__Constructor(...,Sticky=0); decompile 0x0048E520/0x004E12F0 (ctor arg map) |
| Construction event-mask (Flags +0x20)? | **0x19 = LEFTPRESS \| LEFTUP \| RIGHTPRESS.** (LEFTUP is in the mask so the gadget receives that event, but Action discards it.) | decompile 0x006A4DC0; decompile 0x004E12F0 (param map) |
| Per-slot geometry | **W=0x3c (60), H=0x30 (48)** at construction; X/Y assigned later by InitSurface per visible slot. | decompile 0x006A4DC0 (W=0x3c,H=0x30); decompile 0x006ABF80 (X/Y loop) |
| Count / stride | **240 slots** (4 tabs × 60), stride **0x38 bytes**, base 0x00B07E80. | decompile 0x006A4DC0 (loop 0xf0=240, +0xe dwords=0x38) |
| Cameo ID scheme | **1000 + index**, index = visible-slot position 0..(rows×2−1). | decompile 0x006ABF80 (`iVar2 + 1000` loops); decompile 0x006A7780 |
| Mouse_Enter / Mouse_Leave | Cameo highlight + tooltip-delay hook. Enter sets highlight=1, saves tooltip delay then forces delay=0; Leave sets highlight=0 and restores delay. | decompile 0x006AB990/0x006AB9E0; decompile 0x00724530/0x00724520/0x00724540 |
| Tab-switch swap | Remove ALL 60 of the old tab's slots, then add `rows×2` (visible count) of the new tab's slots. | decompile 0x006A7780 (tab-switch branch) |

---

## 1. SelectClass::Action — 0x006AAD00 (VERIFIED-LIVE)

Source: `decompile_function 0x006AAD00` and `disassemble_function 0x006AAD00`.
Signature `(this, uint eventBits, result*, byte param4)` — thiscall, `this` in ECX.

### 1a. Fire edge — PRESS, not release

The function reads the live event-bit word (`param_2`) and:

- `if ((param_2 & 8) != 0) param_2 &= 0xfffffff7;` — **strips LEFTUP (0x8)**. The release-up
  bit is masked off and never acted on (disasm `006aada8 TEST AL,0x8 / AND AL,0xf7`).
- The RIGHT-click path is gated on `(param_2 & 0x10)` = **RIGHTPRESS** (disasm
  `006aadb5 TEST AL,0x10`). It decrements/cancels production, sets the on-hold flag, plays
  Voc/EVA, and writes a command to the queue.
- The LEFT-click path is gated on `(param_2 & 1)` = **LEFTPRESS** (disasm
  `006aae9a/006ab28f TEST ...,0x1`). It does build / queue / Begin_Building_Placement.

There is no LEFTRELEASE (0x4) or RIGHTRELEASE (0x40) branch anywhere in the body. **Cameos
fire on the mouse-down edge.** This is the same authority model as every other Framework-A
gadget Action: the Action runs as soon as the press event is dispatched. (Confidence:
VERIFIED-LIVE.)

> Parity contrast: study item D-A1 already flags that gamemd SIDEBAR BUTTONS (tabs/repair/
> sell/scroll = ShapeButton/Toggle) fire on RELEASE-inside via the ToggleClass press-capture
> machine. **Cameos are different** — SelectClass is a plain Control with no Toggle machine,
> so the cameo Action runs on the press edge directly. Do not apply the ShapeButton
> release-flip to cameos.

### 1b. Direct build action vs posted key

The body, on LEFTPRESS, resolves the slot's type/factory from the slot record and then:

- For a "special"/super-weapon slot (`iVar14 == 0x1f`): builds a command via FUN_004c6b60 and
  pushes it to the lockstep command ring (`g_CommandBuffer` / `g_CommandQueue_*`, 0x6f-stride,
  cap 0x80) — disasm `006ab02a`–`006ab088` and siblings. Plays VocClass__PlayAtPos /
  VoxClass__PlayEVA.
- For a normal producible: looks up/creates the FactoryClass, checks IsInQueue / progress /
  suspended, and either advances production, marks on-hold (`+0x68` slot flag = 1), or — when
  a unit completed — fetches the object, and if `RTTIType == 6` (building) calls
  `HouseClass__Begin_Building_Placement` (disasm `006ab3c5 CMP EAX,0x6 ... CALL 0x004fb840`),
  otherwise issues a command-queue write. Multiple FUN_004c6970/FUN_004c6ae0 command builders
  feed the same ring-buffer push idiom.
- Right-click (RIGHTPRESS): decrement queue / cancel-on-hold path, also via command-queue
  writes (disasm `006aadbd`–`006ab0a7`).

So **all visible behavior happens inside SelectClass::Action**: production state mutation,
command-queue writes (the lockstep MP path), sound cues, and building placement entry. It is
NOT a "post the ID, let SidebarClass build it later" design.

### 1c. The tail chain — ControlClass::Action 0x0048E5A0 (label-drift note)

The function ends (disasm `006ab970 CALL 0x0048e5a0`) by chaining to **ControlClass::Action
at 0x0048E5A0**, passing `this` in ECX, the (possibly-zeroed) event word, and the result
pointer. On the no-match / out-of-range / "consumed" paths the event word is first zeroed
(`006ab94f MOV dword ptr [ESP+0xb0],0x0`), which makes the chained Action post **0** (no
key).

ControlClass::Action 0x0048E5A0 body (decompile 0x0048E5A0): if eventBits != 0 and ID != 0,
`*result = ID | 0x8000`; on RIGHTRELEASE (0x40) with the mask containing RIGHTPRESS (0x10) it
posts `| 0x4000` instead; then dispatches to the Peer (+0x28) slot +0x4C and calls
FUN_004e1530. For an active LEFTPRESS cameo this posts `(1000+slot) | 0x8000` = `0x83E8 +
slot`.

**Label drift recorded:** Ghidra's decompiler names 0x0048E5A0 `GadgetClass__Action`, but
the body reads `this+0x24` (the ControlClass ID field) and posts `ID|0x8000` / dispatches the
Peer — this is **ControlClass::Action**, not the base GadgetClass::Action. The base/Control
bodies are byte-identical in this build, but the SelectClass tail-call target is specifically
0x0048E5A0 = ControlClass::Action. (Verify-binding: read 0x007F2FCC slot +0x7C = 0x006AAD00,
and 0x006AAD00 disasm tail-calls 0x0048E5A0.)

**Net effect on the posted key:** the cameo posts `(1000+slot)|0x8000`. SidebarClass::Action
(0x006A7780) consumes `0x8066` (sell), `0x8065` (repair), `0x80CB..0x80CE` (tabs),
`DAT_00b0b34c|0x8000` and `DAT_00b0b42c|0x8000` (the two strip-scroll buttons) — **and
nothing in the 0x83E8+ range**. The cameo key falls through SidebarClass::Action's branches
without effect and is then handled by the generic GScreen path (no-op). So the posted key is
observationally inert for cameos; the in-place action is what the player sees. (Confidence:
VERIFIED-LIVE on the consumer set; INFERRED that the cameo key is inert — by exhaustion of
the consumer branches read in 0x006A7780.)

---

## 2. Cameo ID scheme (VERIFIED-LIVE)

- **240 slots** total = 4 tabs × 60. The ctor loop runs 0xf0 = 240 iterations (decompile
  0x006A4DC0). Array base 0x00B07E80, stride 0x38 bytes (0xe dwords).
- **Construction-time ID = 0** for every slot (ControlClass__Constructor first arg = 0 in the
  ctor — decompile 0x006A4DC0). IDs are assigned at REGISTRATION time, not construction.
- **Runtime ID = 1000 + index** where index is the visible-slot position. In InitSurface
  (decompile 0x006ABF80) two loops use `1000`: a clear loop `FUN_00724730(iVar2 + 1000)` for
  iVar2 0..0x3c (60) — clears tooltip entries for IDs 1000..1059 — and a register loop
  assigning `iStack_1c = iVar2 + 1000` for iVar2 0..(rows×2−1), registering tooltips for the
  visible cameos. The slot→ID map is therefore **ID = 1000 + (row*2 + col)** in
  registration/visible order, NOT 1000 + absolute-tab-slot. (Confidence: VERIFIED-LIVE.)
- The slot record the Action reads is a parallel per-slot struct in the StripClass at
  `factory/type/cost` offsets (slot record stride 0x34, fields +0x58 type-idx, +0x5c
  RTTIType, +0x60 some count, +0x64 factory-ptr — disasm `006aad40`–`006aad54`). This is the
  StripClass cameo data array, distinct from the GadgetClass array; the gadget only carries
  the click rect + ID + tooltip hook.

---

## 3. SelectClass ctor / CRT static init — 0x006A4DC0 (VERIFIED-LIVE)

`decompile_function 0x006A4DC0` (label `SelectClass__StripClass__SidebarClass__Constructor`,
a CRT static-init thunk referenced from the static-init table; default-ctor variant
0x006AACB0 has zero callers per the study):

```
puVar2 = &DAT_00b07e80;
iVar1  = 0xf0;                                  // 240 slots
do {
  ControlClass__Constructor(0, 0, 0, 0x3c, 0x30, 0x19, 0);  // (this via ECX)
  puVar2[0xb] = 0;                              // +0x2c = 0  (ControlClass Peer field)
  puVar2[0xc] = 0;                              // +0x30 = 0  (SelectClass-specific field)
  *puVar2 = &vtable__SelectClass__StripClass__SidebarClass;  // vtable @ 0x007F2FCC
  puVar2 = puVar2 + 0xe;                        // +0x38 stride
  iVar1  = iVar1 + -1;
} while (iVar1 != 0);
FUN_007c978a(&LAB_006a4e10);                    // register atexit dtor
```

ControlClass__Constructor signature (decompile 0x0048E520):
`(this, ID, X, Y, W, H, Flags, Sticky)` → it forwards `GadgetClass__Constructor(X, Y, W, H,
Flags, Sticky)` and stores ID at +0x24, Peer=0 at +0x28.

GadgetClass__Constructor arg map (decompile 0x004E12F0):
`(this, X→+0xC, Y→+0x10, W→+0x14, H→+0x18, Flags→+0x20, Sticky→+0x1D)`; sets IsToRedraw/
IsDisabled = 0; **if Sticky != 0 it ORs Flags |= 5** (cameos are Sticky=0 so no OR happens).

**Resolved construction parameters for every cameo slot:**

| Field | Value | Meaning |
|---|---|---|
| ID (+0x24) | 0 | placeholder; reassigned to 1000+index at registration |
| X (+0x0C) | 0 | placeholder; set by InitSurface per visible slot |
| Y (+0x10) | 0 | placeholder; set by InitSurface per visible slot |
| W (+0x14) | **0x3c = 60** | cameo width (RA2 cameo 60×48) |
| H (+0x18) | **0x30 = 48** | cameo height |
| Flags (+0x20) | **0x19** | LEFTPRESS(0x1) \| LEFTUP(0x8) \| RIGHTPRESS(0x10) |
| IsSticky (+0x1D) | **0** | NOT sticky — plain Control |
| Peer (+0x28) | 0 | no peer |
| +0x2C, +0x30 | 0 | ControlClass/SelectClass extra fields zeroed |

vtable override matrix confirmed by `read_memory 0x007F2FCC`: slot +0x70 = 0x006AB990
(Mouse_Enter), +0x74 = 0x006AB9E0 (Mouse_Leave), +0x7C = 0x006AAD00 (Action), slot0 dtor =
0x006AC780. Draw_Me is NOT overridden — the strip painter draws the cameo bitmaps; the gadget
is a click rect only. (Confidence: VERIFIED-LIVE.)

---

## 4. Mouse_Enter 0x006AB990 / Mouse_Leave 0x006AB9E0 (VERIFIED-LIVE)

`decompile_function 0x006AB990` + `disassemble_function 0x006AB990`:

```
FUN_00724530();         // ECX = [0x00887368] tooltip manager:  +0x22c = +0x228  (SAVE delay)
FUN_00724520(0);        // ECX = [0x00887368]:                   +0x228 = 0       (delay -> 0)
this->highlight (+0x34) = 1;
this->strip (+0x2c) -> +0x3c = 1;   // mark strip dirty
FUN_004f42f0(0);        // ECX = 0x0087f7e8: request redraw
DAT_00884b8f = 1;       // global redraw flag
DAT_00b0b518 = 1;       // sidebar redraw flag
```

`decompile_function 0x006AB9E0` (Mouse_Leave):

```
FUN_00724540();         // ECX = [0x00887368]:  +0x228 = +0x22c   (RESTORE saved delay)
this->highlight (+0x34) = 0;
this->strip (+0x2c) -> +0x3c = 1;   // mark strip dirty
FUN_004f42f0(0);
DAT_00884b8f = 1;
DAT_00b0b518 = 1;
```

Helper bodies (decompile 0x00724530/0x00724520/0x00724540): the tooltip manager is the global
at **0x00887368** (its ECX, confirmed in the disasm `006ab994 MOV ECX,[0x00887368]`); field
**+0x228 = active tooltip delay (ms)**, **+0x22c = saved delay**. So:

- **Mouse_Enter**: save current delay → +0x22c, then force the active delay to **0** so the
  cameo tooltip appears immediately while hovering a cameo; set the cameo's own highlight flag
  (+0x34 = 1) and mark the strip + sidebar dirty.
- **Mouse_Leave**: restore the saved delay (+0x228 = +0x22c), clear highlight (+0x34 = 0),
  mark dirty.

This exactly matches the study's "HighlightOn saves the tooltip DelayMs and sets it to 0;
Mouse_Leave restores." +0x34 is a SelectClass-specific field (beyond ControlClass's 0x2C/0x30)
and IS the cameo highlight state read by the strip painter. SelectClass is the ONLY
Framework-A live class overriding Mouse_Enter/Mouse_Leave (confirmed: every other live
vtable in the study's matrix leaves +0x70/+0x74 at base). (Confidence: VERIFIED-LIVE.)

> Hover-change cadence: per study A4/G7, Hit_Test runs every Input tick before dispatch; on a
> change the old gadget's Mouse_Leave then the new gadget's Mouse_Enter fire. So moving the
> cursor cameo→cameo restores-then-zeros the delay (net: stays 0 while over any cameo);
> cameo→non-cameo restores the normal delay.

---

## 5. Registration + tab-switch swap (VERIFIED-LIVE)

Two registries are involved and must not be conflated:

1. **The gadget Buttons list** (hit-test + draw walk) — Add_A_Button FUN_004F4410 /
   Remove_A_Button "Hide_Cameo_Slots" 0x004F4450.
2. **The tooltip manager** (separate, 0x00887368) — FUN_00724580 (add tip) / FUN_00724730
   (remove tip). The IDs 1000+ live in BOTH but the tooltip registry is what carries the
   cameo name/cost text.

### 5a. Add_A_Button / Remove_A_Button (VERIFIED-LIVE)

`decompile_function 0x004F4410` (Add_A_Button):
```
if (this->is_disabled-test(button) != 0) return 0;      // vtable +0x2c probe
if (g_GadgetListHead [0x00a8ef54] != 0)
     button->Add_after_or_into_list(g_GadgetListHead);   // vtable +0x10 (list insert)
else g_GadgetListHead = button;                          // first button becomes head
return 1;
```

`decompile_function 0x004F4450` (Remove_A_Button, Ghidra label `Hide_Cameo_Slots`):
```
if (this->probe(button) == 0) return 0;                  // vtable +0x2c
g_GadgetListHead [0x00a8ef54] = button->Remove();        // vtable +0x24 (unlink, return new head)
return 1;
```

So registration = insert into the single live gadget list at head 0x00A8EF54 (the same list
walked by Input/Hit_Test/Draw_All per study A1/A10). Order of insertion = hit-priority order =
draw order. (Confidence: VERIFIED-LIVE.)

### 5b. Visible-count formula (VERIFIED-LIVE)

Appears identically in InitSurface (0x006ABF80) and SidebarClass::Action (0x006A7780):

```
topAdj = (g_ScenarioClass_Instance + 0x34b8 != 0) ? 0x12 : 0x1a;   // 18 if observer/special, else 26
rows   = (((DAT_00886f9c - topAdj) - DAT_00b0b4f8) - 7 + g_SidebarWidth) / 0x32;
visible_cameos = rows * 2;                                          // 2 columns
```

- `DAT_00886f9c` = screen/sidebar height source.
- `DAT_00b0b4f8` = strip top Y offset (cameo strip origin).
- `/0x32` = **÷50** → each cameo ROW is 50 px tall (48 cameo + 2 gap).
- `×2` = **2 columns** of cameos.
- The `0x12 vs 0x1a` top adjust depends on `g_ScenarioClass_Instance+0x34b8` (a scenario
  flag, INFERRED to be observer/no-production state — when set, +0x34b8 also forces special
  scroll behavior in 0x006A7780). The `g_PlayerPtr == DAT_00ac1198` test (observer house)
  separately clamps the scroll step to 1, but does NOT change the row count.

This matches the study's candidate formula exactly: the `/0x32` (50-px row) and `×2`
(columns) shape are confirmed. (Confidence: VERIFIED-LIVE on the arithmetic; INFERRED on the
semantic meaning of the two DAT_ inputs.)

### 5c. Per-slot geometry assignment in InitSurface (VERIFIED-LIVE)

`decompile_function 0x006ABF80`, the cameo-position loop (outer over 4 tabs, stride 0xD20
bytes = 60 slots; inner rows×2 columns):

```
for tab in 0..4:                                   // outer, base += 60 slots each
  for row in 0..rows:                              // iVar7 rows
    for col in 0..2:                               // 2 columns
      slot->SetPos( DAT_00b0b4fc*col + DAT_00b0b4f4,        // X = colStride*col + xBase
                    DAT_00b0b500*row + 1 + DAT_00b0b4f8 );  // Y = rowStride*row + 1 + yBase
      slot->vtbl[+0x48]();                          // finalize/clip
```

- X = `DAT_00b0b4f4 + col*DAT_00b0b4fc` (xBase + column stride).
- Y = `DAT_00b0b4f8 + 1 + row*DAT_00b0b500` (yBase + 1 + row stride; row stride is the 50-px
  step matching `/0x32`).
- Only the FIRST `rows` rows of each tab get real positions here; this primes all 4 tabs'
  visible slots. (Confidence: VERIFIED-LIVE on the position formula; the DAT_ layout-constant
  values are runtime sidebar metrics, not read this session — see UNKNOWNs.)

### 5d. Tab-switch remove/add sequence (VERIFIED-LIVE)

`decompile_function 0x006A7780` (the function the study called "SidebarClass::AI" — it is
actually **SidebarClass::Action**, the result-key consumer; label drift recorded). On a tab
button key in `0x80CB..0x80CE` where `key - 0x80CB != currentTab` (i.e. a real tab change):

```
FUN_00723eb0();                                   // tooltip suppress (hide current tip)
oldtab = this->currentTab (+0x14e7 dword)
this[oldtab*0x3e5 + 0x558] = 0;                   // clear old tab "active" flag
for iVar5 in 0..0x3c (60):                         // REMOVE ALL 60 old-tab slots
    Hide_Cameo_Slots(&DAT_00b07e80
        + (this[oldtab*0x3e5 + 0x55f]*0x3c + iVar5) * 0xe);   // Remove_A_Button 0x004F4450
this->currentTab (+0x14e7) = key - 0x80CB;        // commit new tab
FUN_00723ea0();                                   // tooltip restore
newtab = this->currentTab
this[newtab*0x3e5 + 0x558] = 1;                   // set new tab "active" flag
visible = rows*2;                                  // visible-count formula (5b)
for iVar6 in 0..visible:                           // ADD only visible new-tab slots
    FUN_005565f0(slotGadget);                      // zero Next/Prev (+4/+8) before insert
    FUN_004f4410(&DAT_00b07e80
        + (this[newtab*0x3e5 + 0x55f]*0x3c + iVar6) * 0xe);   // Add_A_Button 0x004F4410
DAT_00884b8f = 1;                                  // redraw
SidebarClass__UpdateScrollButtons();
this+0x53a7 = 1;
VocClass__PlayAtPos(...);                          // tab-switch click sound
```

Key asymmetry to reproduce exactly:

- **Remove uses a fixed 60-count loop** (clears every slot of the old tab regardless of how
  many were visible) — safe because Hide_Cameo_Slots no-ops on a slot not in the list.
- **Add uses the `rows×2` visible count** — only the on-screen cameos get inserted.
- Slot base = `this[tab*0x3e5 + 0x55f] * 0x3c` (a stored per-tab slot-group index; INFERRED ==
  tab index, since the four tabs map to the four 60-slot blocks). The `*0xe` converts slot
  index → dword offset (0x38 bytes).
- `FUN_005565f0` zeros the gadget's Next/Prev (+4/+8) before re-adding — re-insert hygiene so
  a slot moved between lists doesn't carry stale links.
- The full sequence is **remove-old-then-add-new**, with the tab field committed BETWEEN the
  two loops, and a single redraw flag + click sound at the end. (Confidence: VERIFIED-LIVE.)

Also note (decompile 0x006A7780): on every call, BEFORE the tab/scroll dispatch, it walks the
4 StripClass instances calling `StripClass__AI` (the +0x551 stride 0x3e5 loop) and ticks the
flash/scroll animation counters (`+0x14e5/+0x14e6`). The strip-scroll buttons
(`DAT_00b0b34c|0x8000` up, `DAT_00b0b42c|0x8000` down) adjust the per-tab scroll offset
(+0x563) by `rows` (a full page), confirming study item D-A4 "one page per click, no per-tick
repeat" — mask 0x55 has no HELD bit.

### 5e. Tooltip registration for the visible cameos (VERIFIED-LIVE)

In InitSurface (0x006ABF80), after the tab buttons (IDs 0xCB..0xCE registered via
FUN_00724730/FUN_00724580) and a clear loop over IDs 1000..1059, the visible cameos are
registered into the tooltip manager:
```
for iVar2 in 0..(rows*2):
    rect = gadget[iVar2] X/Y/W/H (from &DAT_00b07e90 walk, stride 0xe)
    iStack_1c = iVar2 + 1000;          // tooltip ID = 1000 + visible index
    flag = 1;
    FUN_00724580(&entry);              // add tooltip
```
This is the surface that shows the cameo name/cost on hover (the text the Mouse_Enter
zero-delay path makes appear instantly). (Confidence: VERIFIED-LIVE.)

---

## 6. SBGadgetClass — sidebar body click zone (0x00B07E58, Action 0x006ABA40)

Action 0x006ABA40 is NOT an analyzed function in this Ghidra DB; hand-decoded from
`read_memory 0x006ABA40`:

```
006aba40: PUSH 0x0
006aba42: PUSH 0x0
006aba44: MOV  ECX, 0x0087f7e8          ; sidebar/cursor manager (same ECX as FUN_005bdc80 caller)
006aba49: CALL 0x005BDC86               ; -> FUN_005bdc80(0,0): reset cursor/mouse-shape
006aba4e: MOV  EAX, 1                    ; return 1 (handled / swallow)
006aba53: RET  0xc
```

(CALL rel32 0xfff12232 from next-ip 0x006aba54 resolves to 0x005BDC86, inside FUN_005bdc80
at 0x005BDC80.)

**Verdict:** the invisible sidebar-body gadget **swallows clicks on the panel background**
(returns 1 = handled, posts no ID) and resets the cursor shape. Its only job is to stop a
click that landed on empty sidebar chrome (not on a cameo/button) from falling through to the
tactical map underneath. It carries **no ID** (posts nothing) and does the cursor reset.

- **Mask / sticky:** UNKNOWN from a static read — its global 0x00B07E58 reads all-zero in the
  static image (runtime-constructed; the SBGadget ctor inline in the sidebar init sets these,
  but that init function is not analyzed at 0x006A4C10 and was not decoded this session). The
  study lists SBGadget as invisible, Action-only. Its event mask is NOT read this session.
- **Belongs to A6, not A2.** This is the sidebar-body catcher, adjacent to but distinct from
  the cameo strip. Listed here only because the prompt asked; the A2 cameo contract does not
  depend on it. (Confidence: VERIFIED-LIVE on the Action body/return; UNKNOWN on mask/sticky.)

---

## 7. Rust port contract — what A2 must reproduce

The Cameo `GadgetBehavior` and the driver's cameo-strip sync/tab-swap must reproduce exactly:

### Cameo gadget (per slot)
- **Geometry:** W=60, H=48; X/Y set per visible slot by the strip layout (X = xBase +
  col*colStride, Y = yBase + 1 + row*rowStride, rowStride = the 50-px row step; 2 columns).
- **Event mask = 0x19** (LEFTPRESS | LEFTUP | RIGHTPRESS). Not sticky. Not a Toggle.
- **Fire edge = mouse-DOWN.** Left-press → build/queue/advance/place; right-press →
  decrement/cancel/hold. The LEFTUP bit, though in the mask, is discarded. No release-edge
  logic, no press-capture, no drag-off-cancel for cameos (that machine is for the
  ShapeButton sidebar buttons only).
- **ID = 1000 + visible-index** (row*2 + col, in registration order). The posted result key
  `(1000+idx)|0x8000` is inert (no consumer); reproduce it for completeness but the build
  action is performed in-place when the press is dispatched.
- **Action is direct:** on left-press the cameo behavior must itself drive the production
  state machine + emit the lockstep command (queue a build command), play the click Voc +
  EVA cue, and on a completed building enter placement mode — not "post an ID and let the
  sidebar build it."
- **Mouse_Enter:** set this cameo's highlight flag = 1; save the tooltip delay and force the
  active tooltip delay to 0 (cameo tooltip shows immediately); mark strip + sidebar dirty.
- **Mouse_Leave:** clear highlight = 0; restore the saved tooltip delay; mark dirty.
- **Draw:** the cameo gadget does NOT draw itself; the strip painter draws the cameo bitmap +
  highlight + cost/queue overlays. The gadget is a pure click rect + hover hook.

### Cameo strip driver (sync + tab-swap)
- **Visible count = rows × 2** where `rows = ((sidebarHeight − topAdj − stripTopY − 7 +
  sidebarWidth) / 50)`, topAdj = 18 in the observer/special-scenario state else 26. Only
  `rows×2` cameos are registered into the hit/draw list per tab.
- **Registration order = hit-priority = draw order** (single ordered list; insert at head if
  empty else into the list).
- **Tab switch (remove-then-add):** (1) hide the current tooltip; (2) clear old tab active
  flag; (3) **remove all 60 of the old tab's cameo gadgets** from the list (over-remove is
  fine); (4) commit the new tab index; (5) restore tooltip; (6) set new tab active; (7) **add
  only `rows×2` of the new tab's cameo gadgets** (zeroing each gadget's link fields first);
  (8) update scroll buttons; set redraw flag; play the tab click Voc. Commit the tab field
  BETWEEN remove and add.
- **Scroll buttons** adjust the per-tab scroll offset by a full page (`rows`) per click, no
  per-tick repeat (mask has no HELD bit). Observer house clamps the step to 1.
- **Tooltip registry** (separate from the gadget list): register the visible cameos' name/cost
  tips under IDs 1000+idx; the zero-delay-on-hover behavior comes from the Mouse_Enter hook.
- **Hover cadence:** on cursor moving between cameos, Leave-then-Enter fire (net delay stays 0
  while over any cameo; restored to normal on leaving the strip).

---

## 8. UNKNOWN / blockers

- **SBGadgetClass (sidebar body) event mask + sticky bit** — UNKNOWN, not read this session.
  Its global 0x00B07E58 reads zero statically (runtime-constructed) and its inline ctor
  (study: ~0x006A4C10) is not an analyzed function in this DB. NOT a blocker for A2 (it is the
  A6 body catcher), but needed before building A6. Action body IS confirmed (swallow + cursor
  reset, no ID).
- **Layout-constant values** `DAT_00b0b4f4 / 4fc` (cameo X base + column stride),
  `DAT_00b0b4f8 / 500` (Y base + row stride), `DAT_00886f9c` (height source), `DAT_00b04a...`
  radar offsets — the *formula* shape is verified, but the concrete pixel values are runtime
  sidebar-metric globals, not read this session. The Rust port should source the actual cameo
  X/Y/stride from the real sidebar metrics (or read these DAT_ values from the binary) before
  pixel-exact placement. INFERRED defaults: rowStride = 50 (from /0x32), cameo 60×48.
- **`g_ScenarioClass_Instance + 0x34b8` semantics** (the 18-vs-26 topAdj selector) — INFERRED
  to be the observer/no-production scenario flag, not verified to a named field this session.
  It changes the visible row count, so its exact trigger matters for pixel/row parity; verify
  before shipping the row-count formula.
- **Per-tab slot-group index `this[tab*0x3e5 + 0x55f]`** — INFERRED == tab index (4 tabs map
  to the four 60-slot blocks at 0x00B07E80). Not proven to be an identity map; if a tab can be
  remapped to a different block this would change which 60 slots a tab owns. Verify if the
  port ever reorders tabs.
- **Whether the posted cameo key `(1000+idx)|0x8000` is consumed anywhere outside
  SidebarClass::Action** — INFERRED inert (by exhausting the 0x006A7780 branch set). Not
  proven across the full GScreen dispatch chain. Low risk (the in-place action is the visible
  behavior regardless), but flagged.
