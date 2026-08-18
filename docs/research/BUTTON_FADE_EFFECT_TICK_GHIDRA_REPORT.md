# ButtonFadeEffect Global Container + Per-Frame Tick — Ghidra RE Report

**Status: PARTIAL — global container address unconfirmed; see findings below**

**Date:** 2026-05-19  
**Target:** `DynamicVectorClass<ButtonFadeEffect*>` global container and its per-frame walker  
**Confidence axes (content / identity / binding):**  
- RTTI strings — HIGH/HIGH/N/A (directly read from memory)  
- No walker found — absence confirmed via exhaustive callchain search

---

## 1. RTTI Strings Confirmed

Two RTTI type descriptor strings exist at verified addresses:

| Address    | String                                                    | Verified via                   |
|------------|-----------------------------------------------------------|--------------------------------|
| 0x00820428 | `.?AV?$VectorClass@PAUButtonFadeEffect@@@@`               | `read_memory 0x00820418`, `search_strings "ButtonFadeEffect"` |
| 0x00820460 | `.?AV?$DynamicVectorClass@PAUButtonFadeEffect@@@@`         | same                           |

Both type descriptors share the `type_info` vtable pointer `0x007F9594` (verified via `read_memory 0x00820418`).

**No RTTI for the ButtonFadeEffect struct itself** (no `.?AUButtonFadeEffect@@` string found) — consistent with it being a C-style POD struct with no vtable.

**No data cross-references found** to either type descriptor address:
- `get_xrefs_to 0x00820428` → "No references found"
- `get_xrefs_to 0x00820460` → "No references found"
- `get_xrefs_to 0x00820458` → "No references found"

This means the type descriptors are never directly pointed-to by resolved code in Ghidra's analysis, ruling out the standard RTTI data-ref tracing approach.

---

## 2. DynamicVectorClass Layout (Confirmed)

Verified via `decompile_function 0x005253b0` (the true generic `DynamicVectorClass::Add`):

```
+0x00  vtable ptr (4 bytes)
+0x04  data_ptr   (4 bytes) — pointer to heap array of T*
+0x08  capacity   (4 bytes)
+0x0D  is_allocated (1 byte)
+0x10  active_count (4 bytes)
+0x14  grow_amount  (4 bytes)
```

Total: 24 bytes. Zero-initialized in BSS at game start.

**Important:** The generic `DynamicVectorClass::Add` at 0x005253b0 is only called from INI loading code (`TechnoTypeClass__ReadINI`, `WarheadTypeClass__Constructor`, etc.) — it is NOT used by ButtonFadeEffect. The ButtonFadeEffect vector uses inlined add/remove code. The function labeled `DynamicVectorClass__Add` at 0x00726720 is actually a specialized add for the BulletAnimTracker global at 0x00B0F698 (not generic). Verified via `decompile_function 0x00726720`.

---

## 3. BulletAnimTracker — Confirmed NOT ButtonFadeEffect

The global DynamicVectorClass at **0x00B0F698** (BSS) is the BulletAnimTracker vector:

| Field             | Address     | Description                    |
|-------------------|-------------|--------------------------------|
| data_ptr          | 0x00B0F69C  | pointer to heap array          |
| capacity          | 0x00B0F6A0  |                                |
| is_alloc flag     | 0x00B0F6A5  |                                |
| active_count      | 0x00B0F6A8  | `DAT_00b0f6a8`                 |
| grow_amount       | 0x00B0F6AC  |                                |

Users: `BulletAnimTracker__Register`, `DiskLaserClass__DetachFromObject`, `ParticleSystemClass`. Verified via `decompile_function 0x00726720` and `get_xrefs_to 0x00B0F698`.

This is **distinct** from ButtonFadeEffect.

---

## 4. Per-Frame Walker Pattern (BulletAnimTracker — Reference Pattern)

`FUN_00725C70` (0x00725C70) is the BulletAnimTracker per-frame walker, called from `Main_Tick` (0x0055D360). Decompiled via `decompile_function 0x00725c70`:

```c
void FUN_00725c70(void) {
    int i = 0;
    if (0 < DAT_00b0f6a8) {
        do {
            entry = *(int**)(DAT_00b0f69c + i*4);
            expired = (**(code**)(*entry + 0x44))();   // vtable+0x44 = tick method
            if (!expired) {
                i++;
            } else {
                // inline removal: compact array, decrement count
                (**(code**)(*entry + 8))(entry);        // vtable+0x08 = destructor
            }
        } while (i < DAT_00b0f6a8);
    }
}
```

This is the **confirmed pattern** for how any DynamicVectorClass per-frame walker looks in gamemd.exe.

---

## 5. Main_Tick Call Graph (Full)

`Main_Tick` at 0x0055D360 — verified via `decompile_function 0x0055d360` and `get_function_callees 0x0055d360`. The end-of-tick sequence is:

```c
g_CurrentFrameCounter++;
if (DAT_00b07784 != 0 && DAT_00b07784 < g_CurrentFrameCounter) {
    FUN_00684290();     // scenario pause clear
    DAT_00b07784 = 0;
}
FUN_0055e160();         // frame timing / sleep
FUN_00725c70();         // BulletAnimTracker walker
FUN_00637270();         // unit selection / waypoint processing
```

All functions called from Main_Tick were examined. No function among them ticks a ButtonFadeEffect container. The full callee list (verified via `get_function_callees 0x0055d360`) includes: `LogicClass__AI`, `LogicClass__PerTickUpdate`, `Map__Logic`, `RenderFrame_main`, `GScreenClass__Input`, `House_AI_Tick`, `FUN_00647260`, `FUN_00637550`, etc. None of these contain ButtonFadeEffect iteration logic.

---

## 6. Sidebar Render Path — Exhaustive Check

The following sidebar functions were decompiled and checked for ButtonFadeEffect references:

| Function                          | Address    | ButtonFadeEffect? | Verified via                              |
|-----------------------------------|------------|-------------------|--------------------------------------------|
| `MainGame_SidebarDraw`            | 0x006D0A30 | None              | `decompile_function 0x006d0a30`            |
| `SidebarClass__Draw`              | 0x006A6C30 | None              | `decompile_function 0x006a6c30`            |
| `SidebarClass__Action`            | 0x006A7780 | None              | `decompile_function 0x006a7780`            |
| `StripClass__AI`                  | 0x006A8B30 | None              | `decompile_function 0x006a8b30`            |
| `StripClass__Draw`                | 0x006A9540 | None              | `decompile_function 0x006a9540`            |
| `StripClass__ActivateButtons`     | 0x006A93F0 | None              | `decompile_function 0x006a93f0`            |
| `SidebarClass__AddCameo`          | 0x006A6300 | None              | `decompile_function 0x006a6300`            |
| `SidebarClass__BlitToScreen`      | 0x006A70E0 | None              | `decompile_function 0x006a70e0`            |
| `SBGadgetClass__Draw`             | 0x0069DEB0 | None              | `decompile_function 0x0069deb0`            |
| `SidebarClass__Init_Clear`        | 0x006A5030 | None              | `decompile_function 0x006a5030`            |
| `SelectClass__Action`             | 0x006AAD00 | None              | `decompile_function 0x006aad00`            |
| `FUN_0069E010` (per-gadget flash) | 0x0069E010 | None (unrelated)  | `decompile_function 0x0069e010`            |
| `RenderFrame_main`                | 0x004F4480 | None              | `get_function_callees 0x004f4480`          |
| `TacticalClass_Draw`              | 0x006D3D10 | None              | `get_function_callees 0x006d3d10`          |

The per-gadget flash at `FUN_0069E010` is a **simple counter toggle** on individual gadget objects (fields +0x38/+0x3C/+0x34), called from `SidebarClass__Action` in a loop over `DAT_00B07C48`. This is distinct from ButtonFadeEffect.

---

## 7. Generic `DynamicVectorClass::Add` Callers — Exhaustive Check

`get_function_callers 0x005253b0` returned: `FUN_00675210`, `FUN_007162f0`, `InfantryTypeClass__ReadINI`, `RulesClass__ReadAudioVisual`, `RulesClass__ReadCombatDamage`, `RulesClass__ReadGeneral`, `TechnoTypeClass__ReadINI`, `TiberiumClass__Constructor`, `TiberiumClass__ReadINI_Fields`, `WarheadTypeClass__Constructor`, `WarheadTypeClass__ReadINI`, `WeaponTypeClass__ReadINI`.

All are INI loading functions. **None is a ButtonFadeEffect allocation.** Confirms ButtonFadeEffect vector uses inlined WW-style Add (same pattern as the mislabeled add at 0x00726720 that was actually BulletAnimTracker-specific).

---

## 8. MSFadeAnim — Related but Distinct

`MSFadeAnim` class RTTI at 0x008300C8. Vtable at `vtable__MSFadeAnim` = 0x007EE938. Per-frame tick at 0x005CBDA0 (verified via `read_memory 0x007ee938` + layout). MSFadeAnim is a **Main Screen animation** class (used for in-game fade animations, e.g., during screen transitions), NOT ButtonFadeEffect. The MS* classes (MSShapeAnim, MSFadeAnim, MSOverlayAnim) are all in the same RTTI region but are unrelated to sidebar button fading.

---

## 9. ButtonFadeEffect Global Address — NOT FOUND

**Finding:** The global `DynamicVectorClass<ButtonFadeEffect*>` BSS address could not be confirmed.

**Reason:** All ButtonFadeEffect-related code appears to use inlined DynamicVectorClass operations (no call through the generic Add at 0x005253b0). Since the container is in BSS (zero at startup), it has no initializer reference. The RTTI type descriptors have zero data xrefs in Ghidra's analysis. None of the known sidebar, strip, or main-tick functions contain the iteration pattern.

**Possible explanations:**

1. **Dead / TS-legacy code**: The ButtonFadeEffect class and vector may have been compiled in from a TS-era prototype but never activated in YR. The RTTI strings exist in the binary but the code that uses them may be unreachable from any YR game flow. Supporting evidence: no callers found anywhere in the known sidebar/render/input callchains.

2. **Ghidra analysis gap**: The code that creates/ticks ButtonFadeEffect objects may exist in a region Ghidra failed to disassemble (e.g., inside unanalyzed padding between functions, or a byte-pattern-triggered function boundary issue). Supporting evidence: the function at 0x006ABA40 (SBGadgetClass click handler) lacked Ghidra function boundaries and had to be read as raw bytes.

---

## 10. Candidate Region for Further Investigation

If ButtonFadeEffect IS used, the most likely location is the unanalyzed code in the **0x006ABA00–0x006ABB60** range (SBGadgetClass click/hover handlers). The raw bytes at 0x006ABA40 show a small stub that calls into SidebarClass, and at 0x006ABA53 begins a larger function with a loop pattern (verified via `read_memory 0x006aba40`). These functions were not decompilable via MCP (Ghidra hadn't defined function boundaries). A `create_function` call at 0x006ABA53 and subsequent decompilation would be the next step.

Additionally, examine functions between 0x006AB986 and 0x006ABA40 (gap between `SelectClass__Action` end and the first SBGadget click handler).

---

## Summary

| Item                              | Status         | Confidence         |
|-----------------------------------|----------------|--------------------|
| RTTI strings exist in binary      | CONFIRMED      | HIGH               |
| Type descriptor at 0x00820458     | CONFIRMED      | HIGH               |
| Data xrefs to type descriptor     | NONE FOUND     | HIGH (absence)     |
| Generic Add caller = ButtonFadeEffect | NOT FOUND  | HIGH (absence)     |
| Global container BSS address      | UNKNOWN        | —                  |
| Per-frame walker function         | NOT FOUND      | —                  |
| Walker invocation in Main_Tick    | NOT FOUND      | —                  |
| ButtonFadeEffect active in YR     | LIKELY DEAD/TS | LOW-MEDIUM         |

**Active in YR:** Likely NO — no evidence of any live call path to ButtonFadeEffect code was found across the full Main_Tick callchain, sidebar render path, and all gadget input handlers. The RTTI strings suggest the class was compiled in but the feature appears dormant. This assessment should be confirmed by examining the unanalyzed 0x006ABA40–0x006ABB60 region via `create_function` + decompile.
