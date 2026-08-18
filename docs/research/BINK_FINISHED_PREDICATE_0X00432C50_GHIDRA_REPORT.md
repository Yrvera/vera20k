# Bink Finished Predicate `0x00432C50` Ghidra Report

Date: 2026-05-27

Status: COMPLETE

## Working Notes

Target question: verify the exact Bink finished/wrap predicate implemented by `0x00432C50`, including handle/object offsets, comparison order/signedness, and the active caller path through Bink movie vtable `+0x14` and owner-draw timer `0x65`.

Non-goals: restart behavior at `0x00432BD0`, `_BinkGoto` argument semantics, update-loop catch-up at `0x00432E40`, Bink open/init, VQA fallback, and explicit draw/surface format.

Evidence needed to mark COMPLETE:

- Fresh live Ghidra MCP decompile of `0x00432C50`.
- Fresh live Ghidra MCP bytes/disassembly evidence for comparison branches.
- Fresh live Ghidra MCP evidence that vtable `0x007EE154 + 0x14` routes to the predicate.
- Fresh live Ghidra MCP evidence that owner-draw timer `0x65` calls vtable `+0x14`.

Stop conditions: stop after proving the predicate and caller path; do not inspect restart/update internals beyond caller evidence.

## Verified Binary Findings

### Predicate Formula

Active in YR: Yes.

Fresh MCP decompile of `0x00432C50`:

```c
undefined4 __fastcall FUN_00432c50(int param_1)
{
  uint uVar1;

  uVar1 = *(uint *)(*(int *)(param_1 + 4) + 0xc);
  if ((uVar1 < *(uint *)(*(int *)(param_1 + 4) + 8)) && (*(uint *)(param_1 + 0x30) <= uVar1)) {
    return 0;
  }
  return 1;
}
```

Decoded bytes from live MCP `read_memory(0x00432C50, 0x30)`:

```asm
00432C50  mov eax, [ecx+0x04]      ; object+4 -> Bink handle
00432C53  push esi
00432C54  mov edx, [eax+0x0C]      ; current frame/position marker
00432C57  mov esi, [eax+0x08]      ; total frame count / upper bound
00432C5A  cmp edx, esi
00432C5C  pop esi
00432C5D  jae 00432C67             ; current >= total => finished
00432C5F  cmp edx, [ecx+0x30]      ; compare against object last marker
00432C62  jb  00432C67             ; current < last_marker => wrapped/finished
00432C64  xor eax, eax             ; not finished
00432C66  ret
00432C67  mov eax, 1               ; finished
00432C6C  ret
```

The predicate returns `1` when `handle[0x0C] >= handle[0x08]` OR `handle[0x0C] < object[0x30]`. It returns `0` only while `handle[0x0C] < handle[0x08]` AND `handle[0x0C] >= object[0x30]`.

Signedness/order: the assembly uses unsigned branches `jae` and `jb`, so the comparisons are unsigned 32-bit comparisons.

### Offsets

Active in YR: Yes.

The predicate uses:

- `BinkObject + 0x04`: pointer to Bink handle.
- `BinkHandle + 0x08`: total/upper-bound frame field used by this predicate.
- `BinkHandle + 0x0C`: current frame/position marker used by this predicate.
- `BinkObject + 0x30`: last-marker/wrap baseline.

The offsets are verified by MCP decompile and instruction bytes above. This report does not assign SDK names beyond the predicate role proven here.

### Vtable Route

Active in YR: Yes.

Fresh MCP `read_memory(0x007EE154, 0x40)` shows Bink movie vtable entries:

```text
0x007EE154 + 0x00 -> 0x005C0A30
0x007EE154 + 0x04 -> 0x005C0580
0x007EE154 + 0x08 -> 0x005C0590
0x007EE154 + 0x0C -> 0x005C0540
0x007EE154 + 0x10 -> 0x005C0550
0x007EE154 + 0x14 -> 0x005C0570
0x007EE154 + 0x18 -> 0x005C05A0
0x007EE154 + 0x1C -> 0x005C05D0
```

Fresh MCP bytes at `0x005C0570` decode as:

```asm
005C0570  mov ecx, [ecx+0x10]
005C0573  jmp 00432C50
```

Ghidra MCP xrefs to `0x00432C50` include `From 005c0573 [UNCONDITIONAL_CALL]`, matching the vtable thunk bytes.

### Owner-Draw Timer Caller

Active in YR: Yes.

Fresh MCP decompile of `OwnerDraw_Static_006153E0` shows the `WM_TIMER` path for timer `0x65`:

```c
if (param_3 != 0x65) { ... }
if ((int *)piVar11[0x16] == (int *)0x0) return 0;
cVar2 = (**(code **)(*(int *)piVar11[0x16] + 4))();
if (cVar2 != '\0') InvalidateRect(param_1,(RECT *)0x0,0);
cVar2 = (**(code **)(*(int *)piVar11[0x16] + 0x14))();
if (cVar2 == '\0') return 0;
if (piVar11[0x17] != 0) {
  (**(code **)(*(int *)piVar11[0x16] + 0x1c))(1);
  Register_heap_pool(s_Looping_movie_00835958);
  return 0;
}
...
```

Decoded bytes from live MCP around `0x00615B80`:

```asm
00615B80  cmp dword ptr [esp+0x98], 0x65
00615B88  jne 00615C2C
00615B8E  mov ecx, [esi+0x58]
00615B91  cmp ecx, ebx
00615B93  je  006162C5
00615B99  mov edx, [ecx]
00615B9B  call dword ptr [edx+0x04]   ; update
00615BB2  mov ecx, [esi+0x58]
00615BB5  mov edx, [ecx]
00615BB7  call dword ptr [edx+0x14]   ; finished predicate
00615BBA  test al, al
00615BBC  je  006162C5
00615BC2  cmp dword ptr [esi+0x5C], ebx
00615BC5  je  00615BEC
00615BC7  mov esi, [esi+0x58]
00615BCA  push 1
00615BCC  mov ecx, esi
00615BCE  mov eax, [esi]
00615BD0  call dword ptr [eax+0x1C]   ; restart path, out of scope
```

This proves the predicate is not dead code: owner-draw timer `0x65` calls Bink movie vtable `+0x14` after the update slot.

### Bink Handle Construction Path

Active in YR: Yes for resolved `.BIK` movies.

Fresh MCP decompile of `VQMovieHandle__Constructor` at `0x005C07D0` shows the Bink branch assigns `&vtable__BinkMovieHandle`, stores Bink object at wrapper `+0x10`, and copies width/height from the Bink object:

```c
*puVar6 = &vtable__BinkMovieHandle;
puVar6[4] = iVar4;
...
puVar6[2] = **(undefined4 **)(iVar4 + 4);
puVar6[3] = *(undefined4 *)(*(int *)(iVar4 + 4) + 4);
```

Fresh MCP bytes at `0x005C0870..0x005C08A6` include `c7 00 54 e1 7e 00`, storing vtable pointer `0x007EE154` into the wrapper object. This is the same vtable whose `+0x14` entry routes to `0x00432C50`.

## Implementation Handoff

- Finished predicate includes wrap detection -> fresh MCP decompile/bytes at `0x00432C50..0x00432C6C` -> current Rust only checks `current_frame >= frame_count()` -> `src/render/bink_movie.rs::step` / `BinkMovieSurface::step` -> acceptance test `bink_movie_finished_detects_current_below_last_marker_wrap` -> do not collapse finished state to total-frame-only.
- Predicate comparisons are unsigned and ordered as `(current >= total) OR (current < last_marker)` -> assembly uses `jae` at `0x00432C5D` and `jb` at `0x00432C62` -> Rust should avoid signed frame-index comparisons or saturating signed deltas -> acceptance test `bink_movie_finished_uses_unsigned_marker_comparisons` -> do not model wrap as signed negative elapsed frame count.
- Owner-draw timer calls update before finished -> fresh MCP owner-draw bytes `0x00615B99..0x00615BB7` -> Rust should preserve update-then-finished-loop ordering in movie stepping -> acceptance test `owner_draw_movie_timer_checks_finished_after_update_slot` -> do not check end/loop before running the timer update slot.

## Negative Facts / Do Not Do

- Do not treat Bink finished as `current_frame >= frame_count()` only; wrap detection `current < object+0x30` is verified.
- Do not use signed comparisons for the predicate; the binary uses unsigned `jae`/`jb`.
- Do not infer restart details from this report; vtable `+0x1C` is observed only as the caller after a true finished result and is out of scope here.
- Do not use the older stale `+0x14/+0x1C` table if it maps `+0x14` to something other than the finished predicate thunk `0x005C0570 -> 0x00432C50`.
- Do not treat owner-draw timer `0x65` as checking finished before update; the timer path calls vtable `+0x04` first, then `+0x14`.

## Remaining Uncertainty

None for this slot's target predicate and active caller proof.

Still out of scope for this report: exact `_BinkGoto(handle, 1, 1)` restart behavior, whether `_BinkGoto` immediately decodes/copies, and the SDK-level names of Bink handle fields beyond their proven predicate role.

## Stale-Doc Replacement Wording

For stale vtable/end wording in `docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` or related plans, replace any `+0x14` row that does not describe the finished predicate with:

> Bink movie vtable `0x007EE154 + 0x14` points to thunk `0x005C0570`, which loads the Bink object from wrapper `+0x10` and jumps to `0x00432C50`. The predicate returns finished when unsigned `BinkHandle+0x0C >= BinkHandle+0x08` or unsigned `BinkHandle+0x0C < BinkObject+0x30`; it returns not-finished only when the current marker remains below the total marker and is not below the stored last marker. Owner-draw timer `0x65` calls vtable `+0x04` update first, invalidates on change, then calls this `+0x14` finished predicate.

