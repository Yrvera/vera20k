# Framework A core — GadgetClass/LinkClass base machinery (substrate-service lane)

Date: 2026-06-10. Lane: gadget-core. Ghidra MCP read-only session against gamemd.exe.
Prior authority doc: `docs/research/GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md` (2026-04-22) — this lane RE-VERIFIES its
core claims live and extends with: full vtable slot decode from raw memory, LinkClass mutation API, focus-global
writer enumeration, hold-repeat cadence, and a numbered behavior contract.

Conventions: all addresses gamemd.exe image base 0x400000. "verified via X 0xADDR" = MCP call made this session.

---

## 1. Vtable raw decode

### 1.1 vtable__GadgetClass @ 0x007E92BC — 33 slots (+0x00..+0x80), VERIFIED-LIVE

(verified via read_memory 0x007E92BC len 144; per-slot roles verified by decompiling each slot — see §below)

| Slot | vtbl off | Address | Role |
|---|---|---|---|
| 0 | +0x00 | 0x004E1A60 | scalar-deleting dtor |
| 1 | +0x04 | 0x004E14A0 | Get_Next (thunk) |
| 2 | +0x08 | 0x004E14B0 | Get_Prev (thunk) |
| 3 | +0x0C | 0x005566A0 | LinkClass slot (see §5) |
| 4 | +0x10 | 0x00556700 | LinkClass slot (see §5) |
| 5 | +0x14 | 0x005566D0 | LinkClass slot (see §5) |
| 6 | +0x18 | 0x00556640 | LinkClass slot (see §5) |
| 7 | +0x1C | 0x00556670 | LinkClass slot (see §5) |
| 8 | +0x20 | 0x005565F0 | LinkClass slot (see §5) |
| 9 | +0x24 | 0x004E1480 | Remove/unlink override (clears focus first) |
| 10 | +0x28 | 0x004E1640 | **Input** (verified via decompile_function + disassemble_function 0x004E1640) |
| 11 | +0x2C | 0x004E1570 | Flag_List_To_Redraw |
| 12 | +0x30 | 0x004E14C0 | Delete_List |
| 13 | +0x34 | 0x004E1920 | Extract_Gadget(id) |
| 14 | +0x38 | 0x00488690 | Clear_Attached_List (g_CurrentGadgetList := 0) |
| 15 | +0x3C | 0x004E1460 | Disable |
| 16 | +0x40 | 0x004E1450 | Enable |
| 17 | +0x44 | 0x004AEBA0 | Get_ID (base: 0) |
| 18 | +0x48 | 0x004E1960 | Flag_To_Redraw |
| 19 | +0x4C | 0x0048E650 | Peer_Callback (base: no-op RET 0xC) |
| 20 | +0x50 | 0x004E19A0 | Set_Focus |
| 21 | +0x54 | 0x004E19D0 | Clear_Focus |
| 22 | +0x58 | 0x004E19F0 | Has_Focus |
| 23 | +0x5C | 0x004E1A00 | Any_Redraw_Pending (list walk) |
| 24 | +0x60 | 0x004886A0 | Get_IsToRedraw |
| 25 | +0x64 | 0x004E1A20 | Set_Position |
| 26 | +0x68 | 0x004E1A40 | Set_Size |
| 27 | +0x6C | 0x004E1550 | **Draw_Me(forced)** (base: dirty-flag gate) |
| 28 | +0x70 | 0x004E1510 | Mouse_Enter (base: RET stub) |
| 29 | +0x74 | 0x004E1520 | Mouse_Leave (base: RET stub) |
| 30 | +0x78 | 0x004E1970 | Sticky_Process (acquire/release g_StickyFocus) |
| 31 | +0x7C | 0x004E1530 | Action (base) |
| 32 | +0x80 | 0x004E13F0 | **Clicked_On** / Handle_Input |

**Correction vs prior doc:** the prior report listed "+0x84 = 0x00800AE0 (likely vtable terminator)" and
"+0x88 = 0x004E1AD0 LinkClass helper" as GadgetClass slots 33/34. WRONG — 0x00800AE0 is the RTTI Complete
Object Locator of the NEXT vtable: read_memory 0x00800AE0 gives `00 00 00 00 | 00 00 00 00 | 00 00 00 00 |
90 B7 81 00 | D0 0A 80 00` = COL{sig=0, off=0, cdOff=0, pTypeDesc=0x0081B790, pCHD=0x00800AD0}
(verified via read_memory 0x00800AE0). The GadgetClass vtable's own COL sits at 0x007E92B8 = 0x00800AB0
(verified via read_memory 0x007E92B8). So vtable__GadgetClass ends at +0x80 (33 slots);
0x007E9344 starts a separate vtable (slot0 0x004E1AD0 — LinkClass-shaped, see §5).

### 1.2 vtable__ControlClass @ 0x007E528C — 34 slots (+0x00..+0x84), VERIFIED-LIVE

(verified via read_memory 0x007E528C len 160). Identical to GadgetClass table except:

| vtbl off | GadgetClass | ControlClass | Role |
|---|---|---|---|
| +0x00 | 0x004E1A60 | 0x0048E660 | scalar-deleting dtor |
| +0x44 | 0x004AEBA0 | 0x0048E610 | Get_ID → returns this+0x24 |
| +0x6C | 0x004E1550 | 0x0048E620 | Draw_Me (peer chain first) |
| +0x7C | 0x004E1530 | 0x0048E5A0 | Action (posts ID\|0x8000) |
| +0x84 | — (not a slot) | 0x0048E600 | **Set_Peer** (new virtual introduced by ControlClass) |

All other slots bit-identical to the GadgetClass table (same thunk addresses) — ControlClass overrides exactly
4 slots and introduces 1.

---

## 2. GadgetClass::Input @ 0x004E1640 — exact dispatch contract (VERIFIED-LIVE)

Verified via decompile_function 0x004E1640 AND full disassemble_function 0x004E1640 (decompiler drops the
modifier-word dataflow; assembly is authoritative below).

`__fastcall Input(ECX = list_head)` → returns u16 key code (possibly rewritten by dispatchee).

Exact sequence per call:

1. **Fresh-list reset:** if `g_CurrentGadgetList (0x008B3E8C) != list_head` then `g_StickyFocus (0x008B3E88) := 0`,
   `g_KeyboardFocus (0x008B3E90) := 0`, `g_CurrentGadgetList := list_head`, and a local `list_changed := 1`
   (stack slot ESP+0x18; asm 0x004E1648..0x004E166C).
2. **Queue read:** `ECX=[0x0087F770]` (WWKeyboard instance ptr) → `CALL 0x0054F000` (Check/peek); if non-zero
   → `CALL 0x0054F050` (Get) → `key &= 0xFFFF` (asm 0x004E1671..0x004E1697).
3. **Mouse coordinate source:** if `(key & 0xFF) == 1 || == 2` (covers LEFT press 0x001 AND release 0x801;
   RIGHT press 0x002 AND release 0x802) → x,y from `[[0x0087F770]]+0x00/+0x04` (keyboard-cached event coords);
   ELSE (keyboard event or key==0) → x,y live from mouse object `[0x00887640]` via its vtable +0x2C (Get_X)
   / +0x30 (Get_Y) (asm 0x004E169B..0x004E16CD).
4. **Hover transition (BEFORE dispatch):** `Hit_Test(list_head, x, y)` @ 0x004E15A0; if result !=
   `g_HoveredGadget (0x008B3E94)`: old? → `old->vtbl+0x74` (Mouse_Leave); store new; new? → `new->vtbl+0x70`
   (Mouse_Enter) (asm 0x004E16D0..0x004E16FC). Fires on every Input call where hover changed, including the
   same call that dispatches a click.
5. **Event-flag word (EDI):** key==0 → 0; key 1→0x1 LEFTPRESS; 2→0x10 RIGHTPRESS; 0x801→0x4 LEFTRELEASE;
   0x802→0x40 RIGHTRELEASE (asm 0x004E16FF..0x004E1733).
6. **Modifier word (stack ESP+0x14, init 0):** three key-code PAIRS are polled via `Keyboard::Down`
   (0x0054F5C0): pair `[0x00A8EC08]/[0x00A8EC0C]` → bit 0x1; pair `[0x00A8EC00]/[0x00A8EC04]` → bit 0x2;
   pair `[0x00A8EBF8]/[0x00A8EBFC]` → bit 0x4 (asm 0x004E1738..0x004E17D6). These globals hold configurable
   VK codes (see §4.4 for writer identification).
7. **Held/up polling ONLY when key==0:** if key==0: `Down(1)` (VK_LBUTTON) → flags |= 0x2 LEFTHELD else 0x8
   LEFTUP; `Down(2)` → 0x20 RIGHTHELD else 0x80 RIGHTUP. If key!=0 and flags still 0 → flags = 0x100 KEYBOARD
   (asm 0x004E17DA..0x004E1822). **CORRECTION vs prior doc §4.2**, which claimed held/up bits are assembled
   "every call ... even when the event is a keyboard event" — FALSE: a keyboard event yields flags==0x100
   only; held/up bits exist only on no-event ticks.
8. **Three-tier dispatch** (asm 0x004E1822..end), order load-bearing:
   - **Tier 1 — sticky:** if `g_StickyFocus != 0`: `sticky->vtbl+0x6C(0)` (Draw_Me, unforced);
     `sticky->vtbl+0x80(&key, flags, x, y, 0)` — NOTE 5th arg (modifier word) hardwired 0;
     re-read `g_StickyFocus`; if still non-zero → `->vtbl+0x6C(0)` and return key; if dispatch CLEARED sticky
     → falls through to draw the (former sticky, ESI-cached) gadget once via +0x6C(0), return key.
   - **Tier 2 — keyboard focus:** else if `g_KeyboardFocus != 0 && (flags & 0x100)`: same pattern
     (Draw_Me(0); +0x80(&key, flags, x, y, 0); re-read global; Draw_Me(0)); return key. Non-keyboard events
     DO NOT go to the keyboard-focus gadget — they fall through to the list walk only if focus is null or
     event non-keyboard? **No** — note exact structure: tier 2 is taken only when both conditions hold;
     otherwise tier 3.
   - **Tier 3 — broadcast walk:** for g = list_head; g; g = g->vtbl+0x04 (): `g->vtbl+0x6C(list_changed)`
     (forced=1 on a fresh list, else 0); if `g->IsDisabled (+0x1E) == 0` and
     `g->vtbl+0x80(&key, flags, x, y, modifier_word) != 0` → break (consumed). After break, the CONSUMER gets
     one more `+0x6C(0)`. Gadgets after the consumer get neither Draw_Me nor Handle_Input this tick.
9. **Return value:** `MOV EAX,[ESP+0x10]` — the key slot whose ADDRESS was passed to every +0x80 dispatch.
   ControlClass::Action rewrites it to `ID|0x8000` (KN_BUTTON). So Input returns either the raw key code or a
   button ID with 0x8000 (and possibly 0x4000) set.

**Drawing is driven from Input itself** — there is no separate draw pump for gadget lists; every Input tick
walks Draw_Me. Sticky/focus tiers draw their gadget twice (pre+post dispatch).

---

## 3. Hit_Test @ 0x004E15A0 and Clicked_On @ 0x004E13F0 (VERIFIED-LIVE)

### 3.1 Hit_Test (verified via decompile_function 0x004E15A0)

- Walk starts at `this` (the head passed to Input) and proceeds via vtbl+0x04 `Get_Next` — **head→tail
  forward walk**.
- Skip any gadget with `IsDisabled (+0x1E) != 0`.
- In-bounds test is **half-open**: `X <= mx && mx < X+W && Y <= my && my < Y+H` (right/bottom edge excluded).
- Initial best size = screen dims: best_w = `DAT_007F5BE8`, best_h = `DAT_007F5BF4`.
- Tie-break: candidate wins iff `W*H <= best_W*best_H` (decompile shows `a-b==0 || a<b`, i.e. `<=`, signed
  32-bit multiply). **EQUAL area → the LATER gadget in walk order replaces the earlier** (last-in-list wins).
- Returns winner pointer or NULL. No recursion, no children — flat sibling list only.

### 3.2 Clicked_On (verified via decompile_function 0x004E13F0)

Signature (from Input's call): `Clicked_On(this, key_ptr, flags, mx, my, modifier)` — 5 stack args.

1. `flags &= this->Flags (+0x20)` — per-gadget event mask applied FIRST.
2. Return 0 (not consumed) iff ALL of: `this != g_StickyFocus` AND `(flags & 0x100) == 0` AND
   (`flags == 0` OR `(uint)(mx - X) >= W` OR `(uint)(my - Y) >= H`).
   - The bounds test uses **unsigned** compares, so `mx < X` wraps and fails — equivalent to the half-open
     rect `X <= mx < X+W`, `Y <= my < Y+H`. Edge inclusivity identical to Hit_Test: left/top edge IN,
     right/bottom edge (x+w, y+h) OUT.
   - **Keyboard events (0x100) bypass the bounds check** — a masked-in keyboard event dispatches regardless
     of mouse position.
   - **The sticky gadget bypasses bounds AND the flags==0 early-out** — masked-flags==0 ticks still reach
     Action while `this == g_StickyFocus` (this drives ToggleClass hover-out tracking during press-hold, §7).
3. Otherwise tail-calls `this->vtbl+0x7C` Action(flags_masked, key_ptr, modifier); returns its result.

### 3.3 GadgetClass::Constructor @ 0x004E12F0 (verified via decompile_function)

`(this, x, y, w, h, flags, sticky)`: +0x0C=X, +0x10=Y, +0x14=W, +0x18=H; Next(+0x04)=0, Prev(+0x08)=0
(LinkClass init inlined); IsToRedraw(+0x1C)=0; IsSticky(+0x1D)=sticky; IsDisabled(+0x1E)=0; Flags(+0x20)=flags;
vtable=0x007E92BC; if sticky → `Flags |= 5` (LEFTPRESS|LEFTRELEASE forced into mask).

Field layout (confirmed): +0x00 vtbl, +0x04 Next, +0x08 Prev, +0x0C X, +0x10 Y, +0x14 W, +0x18 H,
+0x1C IsToRedraw (u8), +0x1D IsSticky (u8), +0x1E IsDisabled (u8), +0x20 Flags (u32). sizeof(GadgetClass)=0x24.
ControlClass adds +0x24 ID, +0x28 Peer (sizeof 0x2C). ToggleClass adds +0x2C IsPressed (u8), +0x2D IsOn (u8),
+0x30 Kind (u32) (verified via decompile_function 0x00723E60).

---

## 4. Base method bodies (VERIFIED-LIVE, per-slot)

| Addr | Role | Body (exact) | Evidence |
|---|---|---|---|
| 0x004E1530 | GadgetClass::Action | `if flags==0 return 0; IsToRedraw=1; vtbl+0x78(flags); return 1` (takes 3 stack args, uses only flags; RET 0xC) | decompile_function 0x004E1530 + disasm of caller 0x0048E5A0 |
| 0x004E1550 | GadgetClass::Draw_Me(forced) | `if !forced && !IsToRedraw return 0; IsToRedraw=0; return 1` | decompile_function 0x004E1550 |
| 0x004E1960 | Flag_To_Redraw | `IsToRedraw=1` | decompile_function 0x004E1960 |
| 0x004E1970 | Sticky_Process(flags) | see below — NOT a defined function in DB; decoded from read_memory 0x004E1970 (48 bytes) |
| 0x004E19A0 | Set_Focus | `if g_KeyboardFocus: old->Flag_To_Redraw(); old->Clear_Focus(); this->Flags |= 0x100; g_KeyboardFocus = this` | decompile_function 0x004E19A0 |
| 0x004E19D0 | Clear_Focus | `if g_KeyboardFocus==this: Flags &= ~0x100; g_KeyboardFocus = 0` | decompile_function 0x004E19D0 |
| 0x004E19F0 | Has_Focus | `return this == g_KeyboardFocus` | read_memory 0x004E19F0 (`8B15 903E8B00 33C0 3BCA 0F94C0 C3`) |
| 0x004E1A00 | Any_Redraw_Pending | walk from `this` via Get_Next; return 1 if any IsToRedraw | read_memory 0x004E19F0+0x10 manual decode |
| 0x004E1A20 | **Set_Position(x,y)** | `X=arg1; Y=arg2; RET 8` — prior doc called this slot "Get_Rect": WRONG | read_memory (`8B442404 8B542408 89410C 895110 C20800`) |
| 0x004E1A40 | Set_Size(w,h) | `W=arg1; H=arg2; RET 8` | same read |
| 0x004E1450 | Enable | `IsDisabled=0; IsToRedraw=1; tail-JMP vtbl+0x54 Clear_Focus` | decompile_function 0x004E1450 |
| 0x004E1460 | Disable | `IsDisabled=1; IsToRedraw=1; tail-JMP vtbl+0x54 Clear_Focus` | decompile_function 0x004E1460 |
| 0x004E1510 / 0x004E1520 | Mouse_Enter / Mouse_Leave | single `RET` stubs | read_memory 0x004E1510 |
| 0x004AEBA0 | GadgetClass::Get_ID | `XOR EAX,EAX; RET` | read_memory 0x004AEBA0 |
| 0x0048E650 | Peer_Callback base | `RET 0xC` no-op (3 args) | read_memory 0x0048E650 |
| 0x00488690 | Clear_Attached_List | `MOV [0x008B3E8C],0; RET` — statics-style "forget current list" | read_memory 0x00488690 |
| 0x004886A0 | Get_IsToRedraw | `MOV AL,[ECX+0x1C]; RET` | same read |
| 0x004E1570 | Draw_All(forced) | walk from `this` via Get_Next calling `vtbl+0x6C(forced)` on each. Ghidra label is `LocomotionClass__ForEach_SetSlopeIndex` — **LABEL DRIFT** (gadget code, nothing to do with locomotion); prior doc name "Flag_List_To_Redraw" also imprecise (it draws, it does not set dirty flags) | decompile_function 0x004E1570 |
| 0x004E1920 | Extract_Gadget(id) | walk from `this` via Get_Next; first gadget with `Get_ID()==id`; NULL if id==0 or miss | decompile_function 0x004E1920 |
| 0x004E14C0 | Delete_List | walk to HEAD via Get_Prev; then per gadget: `Clear_Focus(); next = Get_Next(); scalar_dtor(1)` — next captured BEFORE delete (safe) | decompile_function 0x004E14C0 |

**Sticky_Process @ 0x004E1970** (manual x86 decode of read_memory bytes — address is mid-block, no DB function):

```
MOV AL,[ECX+0x1D]            ; IsSticky
TEST AL,AL ; MOV AL,[ESP+4]  ; AL := flags low byte
JZ not_sticky
TEST AL,0x11 ; JZ skip       ; LEFTPRESS|RIGHTPRESS?
MOV [0x008B3E88],ECX         ; g_StickyFocus = this   (ACQUIRE on press)
JMP test_release
not_sticky: CMP [0x008B3E88],ECX ; JNZ ret   ; only current holder can release
test_release: TEST AL,0x44 ; JZ ret          ; LEFTRELEASE|RIGHTRELEASE?
MOV dword [0x008B3E88],0     ; g_StickyFocus = 0      (RELEASE on release)
ret: RET 4
```

Semantics: press-bits acquire capture (only if IsSticky), release-bits release it (acquirer-only, or in the
same call that acquired). A flags word containing both press+release acquires then immediately releases.

### 4.1 Destructors (VERIFIED-LIVE)

- `~GadgetClass` @ 0x004E1390 (verified via disassemble_function): restore vtable 0x007E92BC; if
  `this==g_KeyboardFocus` → clear Flags bit 0x100 + null global; if `this==g_StickyFocus` → null; if
  `this==g_CurrentGadgetList` → null (destroying the head detaches the whole current-list association);
  tail-JMP 0x005565A0 (~LinkClass).
- Scalar-deleting wrapper @ 0x004E1A60: same body + `if (flag&1) operator_delete(this)` (verified via
  decompile_function 0x004E1A60). **LABEL DRIFT:** Ghidra labels both 0x004E1390 and 0x004E1A60
  `GadgetClass__Constructor` — both are destructors (bodies clear globals and call ~LinkClass; the real ctor
  is 0x004E12F0).
- `~LinkClass` @ 0x005565A0 (verified via decompile_function 0x005565A0): set vtable__LinkClass; unlink
  (`Prev->Next=Next; Next->Prev=Prev; Prev=Next=0`). **LABEL DRIFT:** Ghidra label `LinkClass__Constructor`
  — it is the destructor. vtable__LinkClass = **0x007E9344** (confirmed: dtor body references slot ptr
  PTR @ 0x007E935C = 0x007E9344+0x18 = Head_Of_List).

---

## 5. LinkClass — mutation API (VERIFIED-LIVE)

Field layout: +0x00 vtbl, +0x04 Next, +0x08 Prev (Get_Next @ 0x00556620 = `MOV EAX,[ECX+4]; RET`;
Get_Prev @ 0x00556630 = `MOV EAX,[ECX+8]; RET`; verified via read_memory 0x00556620 — both reached from the
gadget vtable through 5-byte JMP thunks 0x004E14A0/0x004E14B0, verified via read_memory 0x004E14A0:
`E9 7B 51 07 00` → 0x00556620 / 0x00556630).

| vtbl off | Addr | Name | Exact semantics (all verified via decompile_function at addr) |
|---|---|---|---|
| +0x0C | 0x005566A0 | **Add(after)** | `this->Remove(); next=after->Next; after->Next=this; this->Prev=after; this->Next=next; if(next) next->Prev=this; return Head_Of_List()` — inserts **immediately after** the argument |
| +0x10 | 0x00556700 | **Add_Tail(list)** | `this->Remove(); tail=list->Tail_Of_List(); tail->Next=this; this->Prev=tail; this->Next=0; return Head_Of_List()` |
| +0x14 | 0x005566D0 | **Add_Head(list)** | `this->Remove(); head=list->Head_Of_List(); head->Prev=this; this->Next=head; this->Prev=0; return this` |
| +0x18 | 0x00556640 | Head_Of_List | walk Prev until Prev==0 (cycle-guard: stop if walk returns to start) |
| +0x1C | 0x00556670 | Tail_Of_List | walk Next until Next==0 (same cycle-guard) |
| +0x20 | 0x005565F0 | **Zap** | `Next=0; Prev=0` — NO neighbor repair (leaves the old list dangling around the hole; callers use it when the neighbors are being torn down wholesale) |
| +0x24 | 0x00556730 | **Remove** (LinkClass) | repair neighbors (`Prev->Next=Next; Next->Prev=Prev`), zero own links, return new head: old head if self wasn't head, else `tail->Head_Of_List()`, else NULL if list was just self |
| +0x24 (GadgetClass override) | 0x004E1480 | Remove | `vtbl+0x54 Clear_Focus(); then LinkClass::Remove` — **unlinking a gadget always drops its keyboard focus** (verified via decompile_function 0x004E1480) |

Insertion-position summary: `Add` = positional (after a given node); `Add_Tail` = append; `Add_Head` =
prepend. Every insert API **self-Removes first** — a gadget can never be in two lists or linked twice.

Destruction mid-iteration: covered by (a) dtor clears all 4 globals that could dangle (sticky, keyboard,
hover is NOT cleared — see §6.1 — current-list head); (b) ~LinkClass repairs neighbor links; (c) Delete_List
captures `next` before destroying; (d) Input's broadcast walk calls Get_Next on the gadget AFTER its
Handle_Input returns — a gadget that destroys ITSELF inside Action would be use-after-free; the framework has
no guard for that case (no gadget in gamemd does it).

**HOVER DANGLING HAZARD (new finding):** neither ~GadgetClass 0x004E1390 nor the scalar dtor 0x004E1A60
clears `g_HoveredGadget (0x008B3E94)` — only the next Input call's Hit_Test overwrites it; the Mouse_Leave
virtual is then invoked on the STALE pointer if hover moved off a destroyed gadget (verified by reading both
dtor bodies — no reference to 0x008B3E94; confirmed via get_xrefs_to 0x008B3E94, §8). In practice gamemd
destroys gadget lists only between modal loops, then resets the list head, but the contract must record it.

---

## 6. ControlClass (VERIFIED-LIVE)

### 6.1 ctor @ 0x0048E520 (verified via decompile_function)

`(this, id, x, y, w, h, flags, sticky)` → forwards (x,y,w,h,flags,sticky) to GadgetClass ctor **unchanged**;
ID(+0x24)=id; Peer(+0x28)=0; vtable=0x007E528C. **CORRECTION vs prior doc §6.1**: ControlClass ctor does NOT
hardcode `Sticky=1, Flags=5` — that hardcode lives in ToggleClass ctor 0x00723E60 (§7).

### 6.2 Action @ 0x0048E5A0 (verified via decompile_function + disassemble_function)

Ghidra label is `GadgetClass__Action` — **LABEL DRIFT** (it sits in the ControlClass vtable +0x7C slot;
GadgetClass's own Action is 0x004E1530). Body (asm-accurate):

```
Action(this, flags, key_ptr, modifier):   ; RET 0xC
  if flags != 0:
      if ID(+0x24) == 0:  *key_ptr = 0
      else:               *key_ptr = ID | 0x8000
                          if (flags & 0x40) && (Flags(+0x20) & 0x10): *key_ptr = ID | 0xC000
  if Peer(+0x28): Peer->vtbl+0x4C Peer_Callback(flags, key_ptr, this)
  return GadgetClass::Action(this, flags, key_ptr, 0)     ; → 1 if flags!=0 (and runs Sticky_Process)
```

The `|0x4000` right-release marker requires RIGHTPRESS (0x10) in the gadget's own mask — confirmed
(`TEST BL,0x40` + `TEST byte [ESI+0x20],0x10` at 0x0048E5BB..0x0048E5C6).

### 6.3 Other overrides

- Draw_Me @ 0x0048E620: `if Peer: Peer->vtbl+0x6C(0); return base Draw_Me(forced)` — peer drawn first,
  unforced (verified via decompile_function 0x0048E620).
- Get_ID @ 0x0048E610: `MOV EAX,[ECX+0x24]; RET`; Set_Peer @ 0x0048E600: `MOV [ECX+0x28],arg; RET 4`
  (verified via read_memory 0x0048E600 len 32).
- **0x0048E550 is NOT a function entry** — decompile_function 0x0048E550 resolves into the body of
  ControlClass ctor 0x0048E520. Prior doc's "Route_Event helper @ 0x0048E550" claim is wrong as stated; the
  call that ToggleClass::Action actually makes is `CALL 0x0048E5A0` (ControlClass::Action) — see §7 decode
  (rel32 at 0x00723F56: E8 45 A6 D6 FF → 0x0048E5A0; at 0x00723FCB: E8 D0 A5 D6 FF → 0x0048E5A0).

---

## 7. ToggleClass — press/release/hold state machine (VERIFIED-LIVE)

ctor @ 0x00723E60 (verified via decompile_function): calls ControlClass ctor with **flags=5, sticky=1**
hardcoded; vtable=0x007E8118; IsPressed(+0x2C)=0; IsOn(+0x2D)=0; Kind(+0x30)=0.

vtable__ToggleClass @ 0x007E8118 (verified via read_memory len 160): identical to ControlClass table except
slot0 dtor=0x004B5810 and **+0x7C Action = 0x00723EC0**; +0x80 stays base Clicked_On 0x004E13F0; +0x84
Set_Peer 0x0048E600. (Next vtable's COL 0x007FFF60 follows at +0x88.)

### Action @ 0x00723EC0 — full manual decode (NOT a defined function in DB; verified via read_memory
0x00723EC0 len 352, hand-disassembled; load-bearing branches re-checked byte-by-byte)

```
Action(this, flags, key_ptr, modifier):                 ; RET 0xC
  ; live cursor test against own rect (uses WWMouse [0x00887640] vtbl+0x2C/+0x30, NOT the event coords):
  inside = (uint)(mouse_x - X) < W  &&  (uint)(mouse_y - Y) < H

  if flags == 0:                                        ; pure re-dispatch tick (only reachable as sticky holder
      if inside && !IsPressed:  IsPressed=1; Flag_To_Redraw()    ;  or via hover... see note A)
      if !inside && IsPressed:  IsPressed=0; Flag_To_Redraw()

  Sticky_Process(flags)                                 ; vtbl+0x78 — acquire on 0x11, release on 0x44

  if flags & 0x11:                                      ; PRESS (left or right)
      IsPressed=1; Flag_To_Redraw()
      ControlClass::Action(flags & ~0x11, key_ptr, 0)   ; press bits stripped → no ID posted
      *key_ptr = 0                                      ; force key result to 0
      return 1                                          ; CONSUMED (stops broadcast walk)

  if flags & 0x44:                                      ; RELEASE
      if !IsPressed:
          flags &= ~0x44                                ; stale release (drag-off already cleared) → strip
      elif inside:
          if Kind==1: IsOn = !IsOn                      ; flip
          elif Kind==2: if !IsOn: IsOn = 1              ; LATCH-ON only (never turns off on click)
          IsPressed=0; Flag_To_Redraw()                 ; release bits KEPT → fires below
      else:  ; IsPressed but released outside
          IsPressed=0; Flag_To_Redraw()                 ; NOTE: release bits KEPT → still fires (note B)

  return ControlClass::Action(flags, key_ptr, 0)        ; posts ID|0x8000 iff flags still non-zero
```

**Note A — who delivers flags==0:** on idle ticks Input assembles held/up bits (0x2/0x8/0x20/0x80); a
ToggleClass mask of 5 zeroes them in Clicked_On, and Clicked_On's early-out then returns 0 — EXCEPT while
`this == g_StickyFocus` (press captured), where the early-out is bypassed and Action(0) runs every Input
tick. So the press-and-hold drag-off/drag-back visual (button pops out/in) is powered by the sticky
re-dispatch, NOT by general hover. A non-pressed button gets its hover frame from ShapeButtonClass::Draw_Me
reading IsPressed — set only via this path or presses.

**Note B — release-outside edge:** if a release event arrives while IsPressed is STILL 1 and the cursor is
outside (possible only when press and release were processed with no intervening flags==0 tick that would
have cleared IsPressed), the release bits are NOT stripped (the `JZ 0x00723FB1 → JMP 0x00723FC1` path skips
the `AND EBX,~0x44` at 0x00723FBE) and the button FIRES (`ID|0x8000`). The prior doc's flat
"dragged off before release — cancel" claim holds only because the held-tick hover tracker normally clears
IsPressed first. **CORRECTION vs prior doc §6.2**: Kind==2 is latch-ON only (`if IsOn skip; IsOn=1`), not
`IsOn ^= 1`; Kind==1 is the flip.

**Press is silent:** the press path posts NO id (`*key_ptr = 0` after the stripped-flags Action call) and
returns 1 (consumed). Buttons report on RELEASE via the tail call.

### 7.1 Hold-repeat cadence (scroll buttons)

Mechanism (assembled from verified pieces above): repeat does NOT live in ToggleClass — it is an EVENT-MASK
property. Per Input tick with no queued event, flags = LEFTHELD(0x2)|RIGHTUP(0x80) etc. (§2 step 7). For a
gadget whose Flags mask includes 0x2, Clicked_On masks to non-zero → Action(0x2) → in ToggleClass::Action
neither 0x11 nor 0x44 matches → falls to tail `ControlClass::Action(0x2, key_ptr)` → posts `ID|0x8000`
**every Input call while the button is held**. Repeat rate therefore == Input call rate (once per frame in
the in-game loop), with no initial delay and no acceleration. Verification of an actual repeat consumer is in
§9 (sidebar strip up/down buttons constructed with mask containing 0x2).

### 7.2 ToggleClass helpers (VERIFIED-LIVE via read_memory 0x00723EA0 len 32)

- 0x00723EA0 Turn_On: `IsOn(+0x2D)=1; tail-JMP vtbl+0x48 Flag_To_Redraw`.
- 0x00723EB0 Turn_Off: `IsOn(+0x2D)=0; tail-JMP vtbl+0x48 Flag_To_Redraw`.
  Both called from SidebarClass::Action 0x006A7780 (tab activate/deactivate paths).

---

## 8. Global registry — every base-framework global, with full writer sets (VERIFIED-LIVE)

All writer/reader sets below from `get_bulk_xrefs` (exhaustive per Ghidra xref DB).

| Global | Addr | Role | Writers (complete) |
|---|---|---|---|
| g_StickyFocus | 0x008B3E88 | mouse-capture gadget | 0x004E13BC (~GadgetClass: null-if-this), 0x004E1A8C (scalar dtor: null-if-this), 0x004E1667 (Input fresh-list reset → 0), 0x004E197F (Sticky_Process acquire → this), 0x004E1993 (Sticky_Process release → 0). Readers: Clicked_On 0x004E13F4, Input ×3, Sticky_Process, dtors, **GaugeClass::Action 0x004E285C** (LEFTHELD-drag gate, §9) |
| g_CurrentGadgetList | 0x008B3E8C | head of list last passed to Input (reset detector ONLY — nothing else consumes it) | 0x004E13CD (~GadgetClass), 0x004E1A9A (scalar dtor), 0x00488690 (Clear_Attached_List → 0), 0x004E1659 (Input → new head) |
| g_KeyboardFocus | 0x008B3E90 | keyboard-event-routing gadget | 0x004E13AC + 0x004E13D7 (~GadgetClass), 0x004E1A7F + 0x004E1AA4 (scalar dtor), 0x004E19C6 (Set_Focus → this), 0x004E19E1 (Clear_Focus → 0), 0x004E166C (Input fresh-list reset → 0) |
| g_HoveredGadget | 0x008B3E94 | last Hit_Test result, drives Enter/Leave | **0x004E16F0 (Input) is the ONLY writer; 0x004E16D9 (Input) the only reader** — destructors do NOT clear it (stale-hover hazard, §5) |
| Hit_Test seed dims | 0x007F5BE8 = 1024, 0x007F5BF4 = 768 | initial "best area" seed = constant 786,432 px² | **zero writers** (read_memory values; xref census) — NOT live resolution; cross-confirmed by sibling lane globals-registries.md |
| WWKeyboard instance | 0x0087F770 | queue Check 0x0054F000 / Get 0x0054F050 / Down(vk) 0x0054F5C0; event coords at instance+0x00/+0x04 | written at init 0x006BC2AE/0x006BC2B6/0x006BEA78 |
| Mouse singleton | 0x00887640 | vtbl+0x2C/+0x30 = live mouse X/Y | written at 0x006BDF25 (WinMain) etc.; identity discussion in sibling lane gscreen-chain.md §1.1 ("g_DisplayChain" Ghidra label is drift) |
| Modifier key codes | 0x00A8EC08/0C (→bit 0x1), 0x00A8EC00/04 (→bit 0x2), 0x00A8EBF8/FC (→bit 0x4) | OptionsClass fields +0xA8/+0xAC, +0xA0/+0xA4, +0x98/+0x9C of instance @ 0x00A8EB60 | sole writer OptionsClass::SetDefaults 0x005FA350: SHIFT pair = 0x10, CTRL pair = 0x11, ALT pair = 0x12 (verified via decompile_function 0x005FA350; statically 0 — read_memory 0x00A8EBF8) |
| Buttons list head (in-game) | DAT_00A8EF54 | GScreenClass "Buttons" — the list head passed to Input each frame | covered by sibling lane gscreen-chain.md (writers 0x004F42A0/0x004F42E0/0x004F443C/0x004F446A); not re-verified here |

So the modifier word of §2 step 6 = `1=SHIFT, 2=CTRL, 4=ALT` (each a configurable two-key pair, both defaulting
to the generic VK). Other readers of the same pairs (0x0055DCxx ListClass keyboard nav, 0x00700xxx,
0x00692xxx DisplayClass, 0x00731Cxx, 0x0051E3xx, 0x004474xx, 0x004D79xx) confirm the same pair-poll idiom
framework-wide.

---

## 9. GaugeClass EXISTS — prior-doc refutation (VERIFIED-LIVE)

GADGET_UI_FRAMEWORK_GHIDRA_REPORT §1.1 claims "`GaugeClass` — NO, not in binary; `Dial8Class` — NO" (TS
ghosts). **WRONG for GaugeClass:**

- vtable @ **0x007E9384** has RTTI COL 0x00800B38 → TypeDescriptor 0x00822868 = `.?AVGaugeClass@@`
  (verified via read_memory 0x007E9380, 0x00800B38, 0x00822868).
- vtable @ **0x007E9430** → COL 0x00800B90 → TypeDescriptor 0x00822888 = `.?AVTriColorGaugeClass@@`
  (verified via read_memory 0x007E942C, 0x00800B90, 0x00822888).
- Both vtables' +0x7C Action slot = **0x004E2830** = thumb-drag state machine (decompile_function
  0x004E2830): gate `(flags & 1) || ((flags & 2) && this == g_StickyFocus)` — i.e. LEFTPRESS always, LEFTHELD
  only while this gadget holds sticky capture. This is the canonical **sticky-held drag consumer**: mask
  includes 0x2; every Input tick while captured re-dispatches with LEFTHELD and the thumb tracks the live
  mouse coordinate. Fields: +0x2D enabled-ish gate, +0x2E IsHorizontal selector (picks Get_X vs Get_Y),
  [0xD] value, [0xE] drag offset; extended virtuals +0x8C/+0x98/+0xA0/+0xA4 (pixel↔value transforms).
- Reached from SliderClass code: FUN_006B1F50 calls 0x004E2830 directly twice (get_xrefs_to 0x004E2830;
  0x006B1Fxx = SliderClass area, prior doc puts ~SliderClass @ 0x006B1D00) — SliderClass derives GaugeClass.
- Dial8Class: NOT re-checked this session — prior doc's absence claim stays UNVERIFIED here.

Confidence: content HIGH (decompiled), identity HIGH (RTTI name read from binary), binding HIGH for
SliderClass→GaugeClass::Action (direct calls + vtable membership).

### 9.1 ShapeButtonClass notes (vtable raw decode @ 0x007E8088, read_memory len 144)

Diffs vs ToggleClass vtable: slot0 dtor 0x004B57F0; +0x6C Draw_Me = **0x0069DEB0** (the prompt's anchor name
"SBGadgetClass::Draw" is drift — it is the +0x6C Draw_Me slot of vtable 0x007E8088, prior doc's
ShapeButtonClass); +0x7C Action = 0x00723EC0 (inherited ToggleClass::Action — confirms scroll/tab buttons run
the §7 machine); **+0x84 = 0x0069DE00 Set_Shape** (read_memory 0x0069DE00: frees owned shape, stores new ptr
at +0x58, zeroes W/H) — the slot that holds Set_Peer 0x0048E600 in ControlClass/ToggleClass is REPURPOSED in
ShapeButtonClass. A Rust port must not assume slot +0x84 is uniformly Set_Peer across the hierarchy.
0x0069DEB0 is additionally called DIRECTLY (devirtualized) from RadarClass::Draw (0x006531CB/D6,
0x00653501/0C), SidebarClass::Draw (0x006A6E88..0x006A6EC2), MainGame_SidebarDraw 0x006D0DE3 (get_xrefs_to
0x0069DEB0) — i.e. sidebar chrome force-draws buttons outside the Input pump.

---

## 10. Sidebar scroll buttons: the actual repeat story (VERIFIED-LIVE) — cross-doc DRIFT found

From SidebarClass::Init @ 0x006A5310 (decompile_function; gadget bases 0x00B0B328 and 0x00B07E80-area...
the two scroll-button blocks write absolute statics):

- Scroll button A @ base 0x00B0B328: IsSticky(+0x1D)=1 (0x00B0B345), **Flags(+0x20)=0x55** (0x00B0B348),
  ID(+0x24)=0xC9 (0x00B0B34C), +0x4C=1 (sidebar surface), +0x50=shape.
- Scroll button B @ base 0x00B0B408: same with ID=0xC8 (0x00B0B42C), Flags @ 0x00B0B428 = 0x55.
- 0x55 = LEFTPRESS|LEFTRELEASE|RIGHTPRESS|RIGHTRELEASE — **NO LEFTHELD (0x2)**.

Consequences (with §7 machine): press = silent + sticky-capture; held ticks = masked-to-0 → hover tracking
only, NO ID posts; release inside = single post of `0x80C9`/`0x80C8` (right-release posts `0xC0C9`/`0xC0C8`).

Consumer: SidebarClass::Action @ 0x006A7780 (decompile_function): matches `(key & ~0x4000) == ID|0x8000`
(so right-click scrolls identically), then **clears the button's IsPressed directly** (writes 0x00B0B354 /
0x00B0B434 = 0) and adjusts the active strip's top row by a computed **page** (`rows = (strip_px_height …)/0x32`;
0xC9 → +page with end-guard, 0xC8 → −page with zero-guard). IsPressed of these buttons has **no other
readers** (get_bulk_xrefs 0x00B0B354/0x00B0B434: single write each, no reads) — nothing polls held state.

**DRIFT vs SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT §5.3**, which claims hold-repeat = "each Input() tick
while sticky-held dispatches Action again [→ one row per tick]". The live mask (0x55, no held bit) makes the
sticky re-dispatch deliver flags==0 → no Action-ID per tick; the binary path is one PAGE per click on
release, with StripClass::AI's scroll animation supplying the rows-per-tick visual. The per-tick-rows player
observation may still hold via the animation, but the *mechanism* in that doc contradicts the live mask —
flag for reconciliation (default DRIFT until someone re-traces StripClass::AI).

**DRIFT vs GADGET_UI_FRAMEWORK §12**: it routes "0x8065/0x8066 → strip scroll up/down" — live decompile of
0x006A7780 shows 0x8065 → FUN_004AC8C0 and 0x8066 → FUN_004AC660 (repair/sell handlers per
SIDEBAR_SYSTEM_GHIDRA_REPORT line 621), while scroll is 0x80C9/0x80C8 keyed off DAT_00B0B34C/DAT_00B0B42C.

---

## 11. Label-drift log (Ghidra DB, this session)

| Addr | Current Ghidra label | Actual role (evidence) |
|---|---|---|
| 0x004E1390 | `GadgetClass__Constructor` | ~GadgetClass (disassemble_function: clears focus globals, tail-JMP ~LinkClass) |
| 0x004E1A60 | `GadgetClass__Constructor` | GadgetClass scalar-deleting dtor (decompile) |
| 0x005565A0 | `LinkClass__Constructor` | ~LinkClass (decompile: unlink + neighbor repair) |
| 0x004E1570 | `LocomotionClass__ForEach_SetSlopeIndex` | GadgetClass Draw_All(forced) — slot +0x2C (decompile; pure gadget-list walk) |
| 0x0048E5A0 | `GadgetClass__Action` | ControlClass::Action (sits in ControlClass vtable +0x7C; GadgetClass::Action is 0x004E1530) |
| 0x0048E550 | (prior doc: "Route_Event helper") | NOT a function entry — inside ControlClass ctor body |
| 0x00887640 | `g_DisplayChain` | mouse singleton (see sibling lane gscreen-chain.md §1.1) |
| 0x00723EC0, 0x004E1970, 0x00488690, 0x004886A0, 0x004E14A0/B0, 0x00556620/30, 0x004E1A00, 0x0069DE00 | no function defined | live code reached via vtables (decoded from raw bytes this session) |

Not patched (read-only lane); hand to the labeler pass.

---

## 12. BEHAVIOR CONTRACT — base gadget service (G1..G25)

Each clause cites its §/verification above. "Tick" = one GadgetClass::Input call on a list head.

- **G1 — Storage.** A gadget list is an intrusive doubly-linked sibling list (Next +0x04 / Prev +0x08);
  no parent/child tree, no z-index field. [§3.3, §5]
- **G2 — Insertion.** `Add(after)` inserts immediately after a node; `Add_Tail(list)` appends; `Add_Head(list)`
  prepends; every insert implicitly Removes the node from any previous list first. [§5: 0x005566A0/0x00556700/0x005566D0]
- **G3 — Removal.** `Remove` repairs both neighbors and returns the recomputed head; `Zap` zeroes own links
  WITHOUT repairing neighbors; GadgetClass::Remove additionally Clear_Focus()es itself first. [§5: 0x00556730, 0x005565F0, 0x004E1480]
- **G4 — Construction defaults.** GadgetClass(x,y,w,h,flags,sticky): all state zeroed except geometry, mask,
  IsSticky; `sticky → Flags |= 0x05`. ControlClass adds ID/Peer (no extra defaults). ToggleClass hardcodes
  flags=5, sticky=1. [§3.3, §6.1, §7]
- **G5 — Fresh-list reset.** Passing a different head than the previous Input call nulls g_StickyFocus and
  g_KeyboardFocus, stores the new head in g_CurrentGadgetList, and force-draws every gadget this tick
  (Draw_Me(forced=1) in the walk). g_CurrentGadgetList has no other consumer. [§2 step 1/8; §8]
- **G6 — Coordinate source.** Mouse-button events (codes 0x001/0x002/0x801/0x802, i.e. low byte 1 or 2) use
  the event-queue coords cached at WWKeyboard+0/+4; keyboard events and idle ticks use live mouse X/Y from
  the mouse singleton vtbl+0x2C/+0x30. [§2 step 3]
- **G7 — Hover transitions.** Hit_Test runs every tick BEFORE dispatch; on change, old gadget's Mouse_Leave
  (vtbl+0x74) fires before the global is updated, then new gadget's Mouse_Enter (vtbl+0x70). Base impls are
  RET stubs. The hover global is written nowhere else — including destructors (stale-pointer hazard). [§2 step 4, §8]
- **G8 — Event flags.** 0x1 LEFTPRESS, 0x4 LEFTRELEASE, 0x10 RIGHTPRESS, 0x40 RIGHTRELEASE from the queued
  event; 0x2/0x8 LEFTHELD/LEFTUP and 0x20/0x80 RIGHTHELD/RIGHTUP polled ONLY on no-event ticks; a queued
  non-mouse event yields exactly 0x100 KEYBOARD. Never both event-bits and held-bits in one tick. [§2 steps 5/7]
- **G9 — Modifier word.** SHIFT=1, CTRL=2, ALT=4 (each a configurable 2-key pair, defaults VK 0x10/0x11/0x12
  from OptionsClass::SetDefaults), polled fresh each tick; passed as 5th Handle_Input arg ONLY in the
  broadcast walk — hardwired 0 for sticky-tier and focus-tier dispatch. [§2 step 6, §8]
- **G10 — Dispatch precedence.** sticky > keyboard-focus > broadcast walk; tiers are exclusive per tick.
  Keyboard-focus tier requires `flags & 0x100`; non-keyboard events skip it entirely (fall to walk). [§2 step 8]
- **G11 — Sticky/focus tier draw cadence.** The dispatched gadget gets Draw_Me(0) immediately before AND
  after its Handle_Input (the post-draw re-reads the global; if dispatch released capture, the formerly-sticky
  gadget still gets the post-draw). [§2 step 8]
- **G12 — Broadcast walk.** Head→tail; every visited gadget gets Draw_Me(list_changed) BEFORE its dispatch;
  disabled gadgets are drawn but not dispatched; the first Handle_Input returning non-zero stops the walk and
  receives one extra Draw_Me(0); gadgets after the consumer get neither call this tick. [§2 step 8]
- **G13 — Return value.** Input returns the 16-bit key code, as possibly rewritten through the &key
  out-param by the dispatched gadget: ControlClass posts `ID|0x8000`, plus `|0x4000` iff RIGHTRELEASE fired
  AND the gadget's mask contains RIGHTPRESS (0x10). ID==0 posts 0. [§2 step 9, §6.2]
- **G14 — Hit-test rule.** Half-open rects; disabled gadgets invisible to hit-test; winner = smallest
  area with `<=` tie-break on a head→tail walk ⇒ on equal area the LATER gadget wins; seed best-area is the
  CONSTANT 1024×768 = 786,432 px² (not live resolution) — a gadget with area > seed can never win. [§3.1, §8]
- **G15 — Per-gadget event filtering.** Clicked_On masks incoming flags by the gadget's Flags first; it
  early-outs (0) unless: gadget is the sticky holder (always dispatches, even masked-0), or masked flags
  contain 0x100 (keyboard bypasses bounds), or masked flags ≠ 0 AND the point is inside the half-open rect.
  [§3.2]
- **G16 — Base Action.** Consumes ANY non-zero masked flags: sets IsToRedraw, runs Sticky_Process, returns 1.
  Returns 0 only for masked-0. [§4: 0x004E1530]
- **G17 — Sticky capture protocol.** Press bits (0x11) acquire g_StickyFocus iff IsSticky; release bits
  (0x44) release iff this holds capture (or in the same call that acquired). Run on every Action via base
  chain; ToggleClass also invokes it directly before branching. [§4 Sticky_Process, §7]
- **G18 — Keyboard focus protocol.** Set_Focus steals: old holder gets Flag_To_Redraw + Clear_Focus
  (clearing its Flags bit 0x100), new holder gets Flags|=0x100. Clear_Focus is self-conditional. Has_Focus is
  pointer equality. Enable, Disable, Remove(unlink) and destruction all force Clear_Focus. [§4, §5]
- **G19 — Redraw flags.** Flag_To_Redraw sets only the local IsToRedraw byte; Draw_Me(forced=0) no-ops unless
  dirty, then clears the bit and reports "draw yourself" to the subclass; Enable/Disable set the dirty bit
  unconditionally; Any_Redraw_Pending scans from a given node tail-ward only. [§4]
- **G20 — Draw order/driver.** Drawing is driven by the same Input walk (head→tail) — later gadgets blit
  later ⇒ tail-ward gadgets render on top, consistent with G14's later-wins tie-break. Draw_All (slot +0x2C)
  exists for forced redraw and has no static callers (vtable-only); engine chrome additionally force-draws
  specific buttons by direct devirtualized Draw_Me calls outside Input. There is no full-frame clear. [§2, §4, §9.1]
- **G21 — ControlClass layering.** Draw_Me draws the Peer (unforced) before itself; Action posts the ID per
  G13, notifies the Peer via Peer_Callback(flags, &key, this) (base impl = no-op), then chains to base
  Action — so every Control click also runs the G17 protocol. [§6.2, §6.3]
- **G22 — Toggle/button machine.** Press: IsPressed=1, capture, consume silently (return 1, key forced 0).
  Hold: per-tick sticky re-dispatch with masked-0 flags tracks the live cursor in/out of the rect, popping
  IsPressed accordingly. Release: not-pressed → release-bits stripped (no fire); pressed+inside → Kind 1
  flips IsOn, Kind 2 latches IsOn=1 (never off), fire `ID|0x8000`; pressed+outside → no toggle but the
  release-bits are NOT stripped (fires if no idle tick cleared IsPressed first — boundary case). [§7]
- **G23 — Hold-repeat is a mask property.** A held button repeats its ID every tick iff its Flags mask
  includes held bits (0x2/0x20) — the machinery adds no timer, no initial delay, no acceleration. GaugeClass
  thumb-drag (LEFTHELD + sticky gate) is the live consumer; the sidebar strip scroll buttons (mask 0x55) do
  NOT repeat through this channel — they fire once per click on release and scroll a page. [§7.1, §9, §10]
- **G24 — Destruction.** ~GadgetClass clears g_KeyboardFocus (incl. the gadget's 0x100 bit), g_StickyFocus
  and g_CurrentGadgetList if they point at the dying gadget, but NOT g_HoveredGadget; then ~LinkClass unlinks
  with neighbor repair. Delete_List rewinds to head and destroys forward, capturing Next before each delete.
  [§4.1, §4 Delete_List]
- **G25 — Modal/list-swap hygiene.** Clear_Attached_List (slot +0x38) zeroes only g_CurrentGadgetList,
  guaranteeing the next Input call takes the G5 reset path (fresh focus + forced redraw) — this is the
  documented way the engine swaps between gadget pages. [§4: 0x00488690, §8]

---

## UNVERIFIED (YELLOW)

- Whether `keyboard(md).ini` remapping can overwrite the OptionsClass modifier pairs at runtime (only
  SetDefaults writes were found in the xref census; INI-driven writes could use indexed addressing invisible
  to the census). Defaults VK_SHIFT/VK_CONTROL/VK_MENU are verified.
- StripClass::AI scroll-animation cadence ("one row per tick" player-visible claim from
  SIDEBAR_TIMING_AND_TOOLTIPS §5) — not re-traced this session; only the *mechanism* claim of §5.3 is
  refuted (G23/§10).
- Dial8Class absence (prior-doc claim) — not re-checked.
- GaugeClass field/extended-slot semantics beyond the Action gate (value transforms at vtbl +0x8C/+0x98/
  +0xA0/+0xA4 inferred from call shape, not decompiled individually).
- The exact identity of FUN_0069DFF0 (called after each button block in SidebarClass::Init — presumed
  Set_Shape-adjacent initializer, not decompiled).
- 0x0069DE00 Set_Shape tail (W/H reassignment from shape header) — first 48 bytes decoded only.

