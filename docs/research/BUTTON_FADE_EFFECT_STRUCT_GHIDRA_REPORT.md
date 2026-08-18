# ButtonFadeEffect — Struct Definition & Constructor Ghidra Report

**Investigation date:** 2026-05-19  
**Scope:** ButtonFadeEffect constructor location, struct layout, total size. DynamicVectorClass global excluded (slots 2/3/4).  
**Status:** PARTIAL — struct size and field layout NOT derivable from constructor; root cause documented below.

---

## 1. Executive Summary

**The ButtonFadeEffect struct constructor could not be located via any of the three prescribed approaches.** The reason is structural: `FUN_006071e0` (verified address) is the button-fade animation implementation, and it does NOT allocate ButtonFadeEffect instances at runtime. Instead the entire animation is run as a blocking, per-frame rendering loop with a local timing array. No per-instance `operator_new(N)` call for a ButtonFadeEffect struct was found anywhere in the button-animation call chains.

The `VectorClass<ButtonFadeEffect*>` and `DynamicVectorClass<ButtonFadeEffect*>` RTTI strings are compiler-emitted template instantiation artifacts. No code xref to either type descriptor was found (verified via `get_xrefs_to 0x00820428` and `get_xrefs_to 0x00820460` — both returned no results). This is consistent with a template that was instantiated for linking purposes (or for a code path unreachable in a standard YR skirmish) but whose container is never written to in the observed button-click code path.

**Active in YR:** CONDITIONAL — `FUN_006071e0` (the animation runner) is called during main-menu and WOL dialog interactions. Whether the DVC<ButtonFadeEffect*> global is ever populated in any code path reachable during a standard YR session could not be confirmed in this pass.

---

## 2. Confirmed Struct Properties

### 2.1 ButtonFadeEffect is a POD struct (not a class)

The RTTI mangled name `.?AV?$VectorClass@PAUButtonFadeEffect@@@@` uses the prefix `PAU` = **pointer to user-defined struct** (not `PAV` = class). This means:
- No vtable (no virtual functions)
- No RTTI on the struct itself (only on the container templates)
- Standard C++ POD struct — fields initialized by explicit assignment, not a constructor vtable dispatch

**Verified via:** `search_strings "ButtonFadeEffect"` → 0x00820428, 0x00820460; `read_memory 0x00820428` confirms `PAU` prefix in both RTTI strings.

### 2.2 No standalone ButtonFadeEffect RTTI

Only two RTTI strings exist for ButtonFadeEffect — both are container-template types:
- `0x00820428`: `.?AV?$VectorClass@PAUButtonFadeEffect@@@@`
- `0x00820460`: `.?AV?$DynamicVectorClass@PAUButtonFadeEffect@@@@`

There is NO `.?AUButtonFadeEffect@@` type descriptor. This confirms the struct has no vtable and no RTTI was emitted for it as a standalone type.

**Verified via:** `search_strings "ButtonFadeEffect"` — exactly 2 results, both container-template descriptors. `search_strings "FadeEffect"` — same 2 results.

---

## 3. Animation Implementation (FUN_006071e0)

The button-fade animation is implemented in `FUN_006071e0 @ 0x006071e0` (verified via `decompile_function 0x006071e0`). This is a **blocking, all-buttons animation loop**, not a per-instance struct pattern.

### 3.1 What FUN_006071e0 does

- Called from `FUN_00607fd0 @ 0x00607fd0` (sync path) and indirectly via the `Main_Tick` loop set up by `FUN_00608070 @ 0x00608070` (async path).
- Counts visible animated buttons via `EnumChildWindows(hwnd, FUN_0060a180, 0)` → result stored in `DAT_00ac1cac` (count of active animated buttons).
- Allocates a LOCAL timing array via `operator_new((button_count + 3) * 4)` — this array is NOT `ButtonFadeEffect`; it holds per-button stagger offsets (integers counting up from 1).
- Runs a loop of `iStack_bc + 6` frames, sleeping `Sleep(0x1e)` = 30ms per frame between draws.
- Calls `CC_Draw_Shape` with computed frame indices that step through SHP animation frames.
- Frees the local array at the end via `FUN_007c8b3d(local_17c)`.

**The local `operator_new` allocation is the timing array, not ButtonFadeEffect:**

```c
// Inside FUN_006071e0 (verified via decompile_function 0x006071e0):
local_17c = operator_new((iVar5 + 3) * 4);   // iVar5 = button_count
// ... fills with sequential integers 1, 2, 3, ...
// local_14c = DAT_00ac1cac (count of animated buttons)
```

**Active in YR:** YES — verified callers include `FUN_00607fd0`, `FUN_00608260`, `FUN_00622b50` (main dialog handler), and `SimpleWonlineDialogControl__Constructor @ 0x00789b60`.

### 3.2 Button control struct flags used by the animation

The animation system works via flags in the **control record** (the UI control data struct, looked up via the `DAT_00ac1b*` hash table). These are NOT ButtonFadeEffect fields — they are fields within the window control entry:

| Offset from control record | Type | Purpose | Evidence |
|---|---|---|---|
| `+0xc1` | byte | "is animated button" flag — set by `FUN_00608380` | `decompile_function 0x00608380` |
| `+0xc2` | byte | "animation pending/running" flag — set by `FUN_00608070`, cleared when done | `decompile_function 0x00608070` |
| `+0xc9` | byte | "focus state" toggle (used for SDBTNANM path focus frame). PATCHED 2026-05-20: was incorrectly listed as `+0xc5`. In `OwnerDraw_Button_00612B70`, the access path is `piVar17 = piVar20 + 1` (record + 4 bytes) followed by `*(bool*)((int)piVar17 + 0xc5)` — i.e. byte offset `4 + 0xc5 = 0xc9` from the record start. `+0xc1` and `+0xc2` use the direct record pointer and are correct. | `decompile_function 0x00612b70` |
| `+0xb4` | int  | button kind (1 = animated button). PATCHED 2026-05-20: was incorrectly listed as `+0x2d`. In `FUN_00608260` / `FUN_00608070`, the access is `piVar1[0x2d]` where `piVar1` is `int *` — actual byte offset = `0x2d × 4 = 0xb4`. Classic `int*` array-index vs byte-offset pitfall (see CLAUDE.md "Decompilation pitfall: param_1 pointer arithmetic"). | `decompile_function 0x00608260`, `FUN_0060c7d0` |

### 3.3 Animation trigger path

```
Click → WM_LBUTTONDOWN in OwnerDraw_Button_00612B70:
    → VocClass__PlayAtPos (sound)
    → CallWindowProcA (default proc)

Click handled upstream → FUN_00608260 (via FUN_005e6b49 or FUN_00612690):
    → checks piVar1[0x2d] == 1   ← record+0xb4 byte-offset, piVar1 is int* (PATCHED 2026-05-20)
    → checks *(piVar1 + 0xc1) != 0 (is animated button flag, byte-offset 0xc1)
    → VocClass__PlayAtPos
    → EnableWindow(hwnd, 0)
    → FUN_006071e0()   ← THE animation loop runs synchronously
    → EnableWindow(hwnd, restored)
    → InvalidateRect

OR async path via FUN_00608070:
    → sets *(control + 0xc2) = 1
    → loops calling Main_Tick() until flag cleared
    → FUN_00607fd0 called from WM_PAINT when +0xc2 is set:
        → FUN_006071e0()
        → clears +0xc2
```

---

## 4. DynamicVectorClass<ButtonFadeEffect*> RTTI Analysis

The Complete Object Locator (COL) for `DynamicVectorClass<ButtonFadeEffect*>` was located:

- **COL address:** 0x00800210 — verified via `read_memory 0x00800210`: `[0x00000000][0x00000000][0x00000000][0x00820458][0x00800200]`
- **pTypeDescriptor:** 0x00820458 → DVC<BFE*> type descriptor (confirmed by `search_byte_patterns "58 04 82 00"` → 0x008001d8, 0x0080021c)
- **No code xrefs to the vtable:** `get_xrefs_to 0x00800224` (expected vtable address) → no results

Similarly for `VectorClass<ButtonFadeEffect*>`:
- **COL address:** 0x008001a8 (reconstructed from `read_memory 0x008001a0`)
- **No code xrefs to vtable** at 0x008001bc

**Conclusion:** These template RTTI structures are compiler-generated and present in the binary but not referenced from any reachable code. The DVC<ButtonFadeEffect*> global (if it exists) is never written to in the button-click animation path.

---

## 5. What "ButtonFadeEffect" Likely Represents

Based on the evidence, the most likely interpretation is one of:

**A) Legacy template declaration:** `VectorClass<ButtonFadeEffect*>` and `DynamicVectorClass<ButtonFadeEffect*>` were declared in WW's UI system header and compiled in, but the actual animation was refactored to use the monolithic `FUN_006071e0` approach instead. The RTTI survives from the original design.

**B) Separate code path:** The DVC<ButtonFadeEffect*> global is used in a different code path not exercised by the main-menu buttons — perhaps in an older dialog style or a TS-era dialog system. (Flag: Tiberian Sun legacy possible, unverified.)

The button animation the player observes on the main menu comes entirely from `FUN_006071e0`, with no ButtonFadeEffect struct instances being allocated.

---

## 6. Open Questions (deferred — out of scope for this slot)

- `[DEFERRED-slot2]` Address of the global `DynamicVectorClass<ButtonFadeEffect*>` instance (if any).
- `[DEFERRED-slot3]` Per-frame tick site — whether there is any code path that DOES walk a ButtonFadeEffect vector.
- `[DEFERRED-slot4]` Whether ButtonFadeEffect is used in WOL or TS-era dialog paths (needs caller analysis of all DVC<ButtonFadeEffect*> Add/Remove calls, if any exist).
- `[DEFERRED]` CrossDissolveEffect struct and its relation to ButtonFadeEffect (similar RTTI pattern: `VectorClass<CrossDissolveEffect*>`, `DynamicVectorClass<CrossDissolveEffect*>` at 0x008203b0/0x008203e8).

---

## 7. Ghidra MCP Calls (Read-Only, This Session)

- `get_xrefs_to 0x00820428` — no xrefs to VC<BFE*> type descriptor
- `get_xrefs_to 0x00820460` — no xrefs to DVC<BFE*> type descriptor
- `read_memory 0x00820428` (64 bytes) — confirmed RTTI string content and type_info vtable at 0x007f9594
- `read_memory 0x00820460` (64 bytes) — confirmed DVC RTTI string
- `search_byte_patterns "58 04 82 00"` — found COL references at 0x008001d8, 0x0080021c
- `search_byte_patterns "20 04 82 00"` — found VC COL references at 0x008001b4, 0x008001c0
- `read_memory 0x00800210` (32 bytes) — identified DVC COL at 0x00800210
- `read_memory 0x008001a0` (32 bytes) — identified VC COL at 0x008001a8
- `get_xrefs_to 0x00800224` — no code xrefs to DVC vtable candidate
- `decompile_function 0x006071e0` — confirmed animation loop, local timing array allocation
- `decompile_function 0x00607fd0` — confirmed sync animation trigger, flag +0xc2 check
- `decompile_function 0x00608070` — confirmed async animation trigger, flag +0xc2 set
- `decompile_function 0x00608260` — confirmed click handler checks [0x2d]==1 and +0xc1
- `decompile_function 0x00612b70` — confirmed WM_LBUTTONDOWN only plays sound, no struct alloc
- `decompile_function 0x00531F60` — confirmed main menu dialog WM_COMMAND has no animation
- `decompile_function 0x00622b50` — confirmed dialog message pre-handler
- `decompile_function 0x0060a180` — confirmed DAT_00ac1cac as animated-button count
- `decompile_function 0x00608380` — confirmed +0xc1 flag setter
- `search_strings "ButtonFadeEffect"` — 2 results, both container RTTI
- `search_strings "FadeEffect"` — same 2 results; no standalone struct RTTI
- `get_xrefs_to 0x007f9594` — confirmed type_info vtable used across all RTTI structs
