# TechnoClass IC State Fields — Struct Decode

**Symbol:** `TechnoClass IC state fields (+0x18c, +0x190, +0x194, +0x1a4, +0x1c4)`
**Address range:** `TechnoClass+0x18c..0x1c4`
**Kind:** struct
**Runbook:** struct-decode-v1
**Verified via:** `decompile_function 0x0070e2b0`, `disassemble_function 0x0070e2b0`,
`decompile_function 0x0041bf40`, `decompile_function 0x004deae4`,
`decompile_function 0x00701900` (TechnoClass__ReceiveDamage),
`decompile_function 0x0070c270` (TechnoClass__Save),
`decompile_function 0x006f2b40` (TechnoClass__Constructor),
`decompile_function 0x00706640` (TechnoClass__Draw),
`decompile_function 0x00706ed0` (TechnoClass__Render),
`decompile_function 0x006f5190` (TechnoClass__DrawExtras),
`decompile_function 0x006f60d0` (TechnoClass__DrawBehind),
`decompile_function 0x006f9e50` (TechnoClass__AI_Update)

---

## Summary

Five fields in the IC state block of `TechnoClass` govern when the Iron Curtain (or Force
Shield) effect is active and how it was applied. Four of the five are fully decoded and
verified; one (`+0x1a4`) is confirmed written but its reader has not been found in the
IC system.

---

## Active in YR

**Yes.** Every field in this block is live in standard YR skirmish: `+0x18c`/`+0x194` drive
`TechnoClass__IsIronCurtainActive` which is called on every render pass, damage event, and
AI update for any IC'd unit. `+0x1c4` controls spark colors in `TechnoClass__ReceiveDamage`.

---

## Field Table

| Offset | Size | Type | Semantic | Written by | Read by | Confidence |
|--------|------|------|----------|------------|---------|------------|
| `+0x18c` | 4 | `i32` | IC apply frame (`-1` = never applied) | `TechnoClass__IronCurtain` (0x0070e2b0): `MOV [ECX+0x18c], EAX` (EAX = g_CurrentFrameCounter) | `TechnoClass__IsIronCurtainActive` (0x0041bf40): `*(+0x18c) != -1` guard | GREEN |
| `+0x190` | 4 | `undefined4` | Dead/vestigial write — garbage from local stack (NOT source_house) | `TechnoClass__IronCurtain` (0x0070e2b0): `MOV [ESI+0x4], EAX` where EAX = `[ESP+0x8]` (below entry-ESP, uninitialized) | None found in IC system | GREEN — dead write confirmed by assembly frame analysis |
| `+0x194` | 4 | `i32` | IC duration in frames | `TechnoClass__IronCurtain` (0x0070e2b0): `MOV [ESI+0x8], EDX` (EDX = duration arg) | `TechnoClass__IsIronCurtainActive` (0x0041bf40): `iVar1 = *(+0x194)` | GREEN |
| `+0x1a4` | 4 | `undefined4` | **YELLOW — purpose unresolved** | `TechnoClass__IronCurtain` (0x0070e2b0): `XOR EAX,EAX; MOV [ECX+0x1a4],EAX` (cleared to 0) | No reader found in IC, rendering, AI, damage, or save paths | YELLOW |
| `+0x1c4` | 4 | `i32` (bool int) | `1` = Force Shield active; `0` = Iron Curtain | `TechnoClass__IronCurtain` (0x0070e2b0): `MOV [ECX+0x1c4], 0x1` or `0` via is_force_shield branch | `TechnoClass__ReceiveDamage` (0x00701900): spark color = `6` if `*(+0x1c4) != 0`, else `1` | GREEN |

---

## Detailed Field Analysis

### +0x18c — IC apply frame (i32, -1 = never)

**Write:** `TechnoClass__IronCurtain` at `0x0070e2b0`:
```asm
0070e2b0: MOV EAX,[0x00a8ed84]   ; EAX = g_CurrentFrameCounter
0070e2c3: MOV [ECX+0x18c],EAX    ; this->+0x18c = current frame
```

**Read:** `TechnoClass__IsIronCurtainActive` at `0x0041bf40`:
```c
if (*(int *)(param_1 + 0x18c) != -1) {
    iVar2 = g_CurrentFrameCounter - *(int *)(param_1 + 0x18c);  // elapsed
    if (iVar2 < iVar1) {  // iVar1 = duration
        iVar1 = iVar1 - iVar2;  // remaining
        return CONCAT31(..., 0 < iVar1);  // true if still active
    }
}
return 0;  // -1 sentinel → not active
```

The `-1` sentinel means "IC was never applied." The constructor does not explicitly
initialize this field (no write to `+0x18c` or `+0x190..0x1c4` range seen in
`decompile_function 0x006f2b40`), and `TechnoClass__Save` does not serialize it
(`decompile_function 0x0070c270` confirmed — IC fields are absent from the Save chain).

---

### +0x190 — dead write (local stack garbage)

**Write:** `TechnoClass__IronCurtain` at `0x0070e2cd`:
```asm
0070e2c9: MOV EAX,[ESP+0x8]   ; ← below entry ESP; local frame garbage
0070e2cd: MOV [ESI+0x4],EAX  ; this->+0x190 = garbage
```

After `SUB ESP,0xC` + `PUSH ESI` (total frame shift 0x10), `[ESP+0x8]` = entry-ESP − 0x8 —
this is below the caller's stack frame and is never initialized. The Ghidra decompiler
renders this as `local_8`. The preflight note `+0x190 = source_house` is WRONG; `source_house`
is at `[ESP+0x18]` post-frame = original `[X+8]` and is **never stored** in any TechnoClass field.
See `fn-TechnoClass-IronCurtain.md` §Assembly Analysis for the full frame derivation.

**No reader found.** The field is a vestigial slot — probably was `source_house` storage in
a prior version but was disconnected.

---

### +0x194 — IC duration (i32, frames)

**Write:** `TechnoClass__IronCurtain` at `0x0070e2d8`:
```asm
0070e2d8: MOV [ESI+0x8],EDX  ; this->+0x194 = duration (arg1)
```

`duration` is verified as `[ESP+0x10]` post-frame = original `[X+4]` = arg1 (confirmed by
frame analysis in `disassemble_function 0x0070e2b0`).

**Read:** `TechnoClass__IsIronCurtainActive`: `iVar1 = *(int *)(param_1 + 0x194)` — used
as the total duration to compare against elapsed frames.

---

### +0x1a4 — unknown (YELLOW)

**Write:** `TechnoClass__IronCurtain` at `0x0070e2d0–0x0070e2d2`:
```asm
0070e2d0: XOR EAX,EAX
0070e2d2: MOV [ECX+0x1a4],EAX  ; this->+0x1a4 = 0
```

**Reader search:** Examined `TechnoClass__IsIronCurtainActive`, `TechnoClass__ReceiveDamage`,
`TechnoClass__DrawExtras`, `TechnoClass__DrawBehind`, `TechnoClass__Draw`,
`TechnoClass__Render`, `TechnoClass__AI_Update`, `TechnoClass__Save`, `TechnoClass__Constructor`,
`TechnoClass__StartFidget` (IC dispatch). **None of them read `+0x1a4`.**

**Contextual clues:**
- `+0x1a4` sits between the IC state fields (`+0x18c..+0x1a0`) and the WarpAttach-adjacent
  fields (`+0x1a8` onward, written by `FUN_0070e300` at vtable+0x1d0).
- `FUN_0070e300` (the apparent EndIronCurtain counterpart) does NOT write `+0x1a4`.
- The field is cleared to `0` on IC apply — not set to a specific value.
- Not serialized in `TechnoClass__Save`.

**Most likely interpretation:** `+0x1a4` is a field from an adjacent system (possibly part of
a timer/counter block) that gets incidentally cleared on IC apply, or it guards some behavior
in a non-IC caller not traced in this session. It does NOT drive any IC-visible behavior
in the functions examined.

> **YELLOW:** Purpose unresolved. Field is cleared on IC apply but no reader found in the IC
> system. A future struct decode of the full `+0x18c..0x1c8` block may identify it as part
> of a parallel timer pair (e.g., an apply-frame + duration pair for a different effect that
> overlaps the IC region).

---

### +0x1c4 — is_force_shield (i32 bool)

**Write:** `TechnoClass__IronCurtain` at `0x0070e2e4` / `0x0070e2f4`:
```asm
0070e2e2: JZ   0x0070e2f4       ; if is_force_shield == 0
0070e2e4: MOV [ECX+0x1c4], 0x1  ; Force Shield
0070e2f4: MOV [ECX+0x1c4], EAX  ; Iron Curtain (EAX = 0)
```

**Read:** `TechnoClass__ReceiveDamage` at `0x00701900`:
```c
if (*(int *)((int)pThis + 0x1c4) != 0) {
    spark_color = 6;  // Force Shield → blue-white sparks
} else {
    spark_color = 1;  // Iron Curtain → gold sparks
}
```

This is **player-visible**: bullets/projectiles hitting a Force-Shield'd unit emit a different
spark color than bullets hitting an Iron-Curtain'd unit. Both use the same damage-immunity
code path in `ReceiveDamage` but branch on `+0x1c4` for the visual feedback.

---

## Serialization

`TechnoClass__Save` (`decompile_function 0x0070c270`) does **not** serialize any of the IC
state fields (`+0x18c..+0x1c4`). The block spans fields from `+0x18c` to past `+0x1c4`, but
none appear in the Save function's field enumeration.

**Observable consequence:** Loading a saved game after IC was applied causes IC state to be
reset — the unit will appear unprotected on reload. This is expected YR behavior (IC does
not persist across save/load in the original game).

---

## Vtable Context

From `inspect_memory_content` at `0x007e23f0` (unit vtable page):

| Vtable addr | Pointer | Function |
|-------------|---------|----------|
| `0x007e23fc` | `0x0070e340` | `BuildingClass__SetCoords` (sibling, writes `+0x1a8..+0x1b0`) |
| `0x007e2400` | `0x0070e300` | `FUN_0070e300` (writes `+0x1a8..+0x1b0`, clears `+0x1c0`) |
| `0x007e2404` | `0x0041bf40` | `TechnoClass__IsIronCurtainActive` ← **confirmed** |
| `0x007e2408` | `0x006f7970` | `FUN_006f7970` (unknown, noted in manifest) |

---

## Out-of-Scope Refs

| Symbol | Reason |
|--------|--------|
| `FUN_0070e300` | Vtable-adjacent; writes `+0x1a8`, `+0x1ac`, `+0x1b0`, `+0x1c0` — appears to be EndIronCurtain counterpart; scope-explorer should evaluate |
| `TechnoClass__Save` / `TechnoClass__Load` | IC fields confirmed NOT serialized; full save/load decode is out-of-scope |
| `RulesClass+0x18A8` (IronCurtainColor) | Applied as tint in rendering; the downstream `CC_Draw_Shape` call that uses the color has not been found in this decode |
| `TechnoClass+0x1a8..+0x1b0` | WarpAttach-adjacent fields written by `FUN_0070e300`; sibling to the IC block; struct context needed |

---

## Unverified Claims

> **YELLOW**

- `TechnoClass+0x1a4` purpose: field is cleared on IC apply but no reader found in the
  functions examined. Could be a timer-pair field for another system, or a write-only
  diagnostic field. No evidence that it gates any player-visible behavior.
- Constructor initialization: the constructor does not explicitly write `+0x18c..+0x1c4`.
  These fields may be zero-initialized by the allocator, or the `-1` sentinel for `+0x18c`
  may be set by a subclass constructor not examined here.
- The vtable slot at `+0x1d0` (`FUN_0070e300`) and `+0x1c8` are adjacent to
  `IsIronCurtainActive` — their full role (StartIronCurtain / EndIronCurtain vtable
  overrides) has not been fully decoded in this task.
