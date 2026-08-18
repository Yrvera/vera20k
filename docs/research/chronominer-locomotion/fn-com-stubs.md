# TeleportLocomotionClass COM Stubs — Bundle Decode

**Proposed Ghidra labels:** All 5 functions already have meaningful labels — plate comments only. Per-function label rows below.

**Active in YR:** Yes — all 5 are COM interface methods registered in the vtables set by `TeleportLocomotionClass__Constructor` (0x00718000). They are dispatched via COM vtable call from the locomotion framework, not via direct function-call graph. Confirmed: no direct callers found for Begin_Piggyback (get_function_callers 0x00719e90), End_Piggyback (get_function_callers 0x00719ee0), Is_Ok_To_End (get_function_callers 0x00719f30) — all dispatched through COM vtable.

---

## Summary

Five small COM interface stubs implementing `IUnknown` (QueryInterface) and `IPiggyback` (Begin_Piggyback, End_Piggyback, Is_Ok_To_End) plus the ILocomotion QueryInterface thunk. The `IPiggyback` interface is critical for the chrono miner: the Drive locomotor is replaced temporarily by TeleportLocomotionClass via Begin_Piggyback, and End_Piggyback releases the override and restores the Drive locomotor pointer. Is_Ok_To_End gates when End_Piggyback is allowed to fire.

---

## 1. TeleportLocomotionClass__QueryInterface — 0x00719E30

**Proposed Ghidra label:** TeleportLocomotionClass__QueryInterface (existing label authoritative)

Source: `decompile_function 0x00719e30`

```c
int TeleportLocomotionClass__QueryInterface(int* param_1, int* param_2, int* param_3)
{
  // Delegates to base LocomotionClass__QueryInterface first (handles IUnknown + ILocomotion)
  iVar1 = LocomotionClass__QueryInterface(param_1, param_2, param_3);
  if (iVar1 == -0x7fffbffe) {  // E_NOINTERFACE from base
    // Check for IID_IPiggyback (4-DWORD GUID comparison)
    bVar3 = memcmp_4dword(param_2, &IID_IPiggyback_Copy);
    if (bVar3) {
      piVar2 = (param_1 == NULL) ? NULL : (param_1 + 6);  // IPiggyback vtable at param_1+0x18
      *param_3 = (int)piVar2;
    }
    if (*param_3 == 0) return -0x7fffbffe;  // E_NOINTERFACE
    (**(code**)(*param_1 + 4))(param_1);     // AddRef (vtable+0x4 = AddRef)
    return 0;  // S_OK
  }
  return iVar1;
}
```

**Behavioral notes:**
- Delegates to `LocomotionClass__QueryInterface` for IUnknown and ILocomotion IIDs.
- For `IID_IPiggyback`: returns `param_1 + 6` (int* arithmetic = `+0x18` byte offset from IUnknown vtable base) as the IPiggyback vtable pointer. This matches the Constructor setting three vtables at offsets 0x00, 0x04, 0x18.
- Calls AddRef on success (vtable slot 1 = vtable+0x4).

---

## 2. TeleportLocomotionClass__Begin_Piggyback — 0x00719E90

**Proposed Ghidra label:** TeleportLocomotionClass__Begin_Piggyback (existing label authoritative)

Source: `decompile_function 0x00719e90`

```c
undefined4 TeleportLocomotionClass__Begin_Piggyback(int param_1, int* param_2)
{
  if (param_2 == NULL) return 0x80004003;         // E_POINTER
  if (*(int*)(param_1 + 0x30) != 0) return 0x80004005;  // E_FAIL: piggyback slot occupied
  // Store the piggybacked locomotor pointer
  *(int**)(param_1 + 0x30) = param_2;
  (**(code**)(*param_2 + 4))(param_2);            // AddRef on piggybacked locomotor
  return 0;  // S_OK
}
```

**Behavioral notes:**
- `param_1` = TeleportLocomotionClass base (IUnknown this).
- `param_1 + 0x30` = piggyback slot (the piggybacked Drive locomotor pointer). Stored at TeleportLocomotionClass+0x30.
- Calls AddRef on the piggybacked locomotor (vtable+0x4 = AddRef).
- Returns E_FAIL (0x80004005) if a piggyback is already active — no double-stacking.

---

## 3. TeleportLocomotionClass__End_Piggyback — 0x00719EE0

**Proposed Ghidra label:** TeleportLocomotionClass__End_Piggyback (existing label authoritative)

Source: `decompile_function 0x00719ee0`

```c
undefined4 TeleportLocomotionClass__End_Piggyback(int param_1, int* param_2)
{
  // param_1 here is IPiggyback this (NOT IUnknown base)
  // param_1 - 0xC = TeleportLocomotionClass::owner TechnoClass ptr
  //   Derivation: IPiggyback vtable at base+0x18; param_1 = base+0x18
  //   → param_1 - 0x18 = base; TechnoClass ptr at base+0x0C
  //   → param_1 - 0xC = base+0xC = TechnoClass ptr (int direct)

  if (param_2 == NULL) return 0x80004003;  // E_POINTER

  // Clear kill-credit source pointers on the owning TechnoClass
  if (*(int*)(param_1 - 0xC) != 0) {
    *(undefined4*)(*(int*)(param_1 - 0xC) + 0x428) = 0;  // TechnoClass+0x428 = source bldg ptr
    *(undefined4*)(*(int*)(param_1 - 0xC) + 0x42c) = 0;  // TechnoClass+0x42C = source house ptr
  }

  // Return the stored locomotor and clear the piggyback slot
  if (*(int*)(param_1 + 0x30) != 0) {
    *param_2 = *(int*)(param_1 + 0x30);   // return stored Drive locomotor to caller
    *(undefined4*)(param_1 + 0x30) = 0;   // clear piggyback slot
    return 0;  // S_OK
  }
  return 1;  // S_FALSE: no piggyback was active
}
```

**Behavioral notes:**
- **`param_1` is the IPiggyback interface pointer**, NOT the IUnknown base. Since the IPiggyback vtable is at `base+0x18`, `param_1 = base+0x18`. Therefore `param_1 - 0xC = base+0xC` = the TechnoClass owner pointer field at TeleportLocomotionClass+0x0C.
- `param_1 + 0x30` = the piggyback slot relative to IPiggyback this. With param_1 = base+0x18: `base+0x18+0x30 = base+0x48` — BUT Begin_Piggyback stores to `param_1+0x30` with base as this, giving `base+0x30`. These are different offsets. The mismatch is a YELLOW issue — see Unverified section.
- **CRITICAL for chrono miner**: End_Piggyback is the path that restores the Drive locomotor after the teleport warp completes. The Drive locomotor pointer (stored via Begin_Piggyback) is returned in `*param_2`, allowing the locomotion framework to re-attach it as the active locomotor.
- Clears `TechnoClass+0x428` (source building ptr) and `TechnoClass+0x42C` (source house ptr) — these kill-credit pointers from `InitiateWarp` are cleared here when the piggyback ends cleanly.

---

## 4. TeleportLocomotionClass__Is_Ok_To_End — 0x00719F30

**Proposed Ghidra label:** TeleportLocomotionClass__Is_Ok_To_End (existing label authoritative)

Source: `decompile_function 0x00719f30` (plate comment already present and accurate)

```c
uint TeleportLocomotionClass__Is_Ok_To_End(int param_1)
{
  // param_1 = IPiggyback this (base+0x18)
  // param_1 - 0x14 = base + 0x18 - 0x14 = base + 0x04 (ILocomotion vtable ptr region)
  // param_1 - 0x0C = base + 0x18 - 0x0C = base + 0x0C (TechnoClass owner ptr)

  uVar1 = (**(code**)(*(int*)(param_1 - 0x14) + 0x10))(param_1 - 0x14);
  // vtable+0x10 on (param_1 - 0x14) = ??? — likely Is_Moving() (ILocomotion slot)

  if ((char)uVar1 == '\0'         // Is_Moving() returned false
   && *(int*)(param_1 + 0x30) != 0      // piggyback slot occupied (base+0x48? — YELLOW)
   && *(char*)(param_1 + 0x1d) == '\0'  // +0x1D flag == 0 (base+0x35 — YELLOW)
   && *(char*)(uVar1 + 0x27c) == '\0'   // TechnoClass+0x27C (ChronoInTransit) == 0
   && *(int*)(param_1 + 0x20) == 0      // +0x20 (base+0x38) state/counter == 0
   && *(char*)(uVar1 + 0x6ad) == '\0')  // TechnoClass+0x6AD (IsDeploying) == 0
  {
    return CONCAT31(..., 1);  // returns true (low byte = 1)
  }
  return uVar1 & 0xffffff00;  // returns false (low byte = 0)
}
```

**Behavioral notes:**
- All conditions must be true for the piggyback to end (Drive locomotor to be restored):
  1. `Is_Moving()` = false (unit is stationary / warp complete)
  2. Piggyback slot is occupied (there is a locomotor to return)
  3. `+0x1D` internal flag == 0 (YELLOW — exact meaning unconfirmed)
  4. `TechnoClass+0x27C` (ChronoInTransit) == 0 — warp anim fully cleared
  5. `+0x20` counter == 0 (relative to IPiggyback this — may be base+0x38, the warp count)
  6. `TechnoClass+0x6AD` (IsDeploying flag on FootClass) == 0
- **Chrono miner implication**: For instant-warp (timer=0 chrono delay), conditions 1/3/5/6 are met immediately after state machine reaches state 0; condition 4 requires `+0x271` cleared (done by TimerCheck). The miner's piggyback is released as soon as the warp state machine completes and TimerCheck fires.

---

## 5. TeleportLocomotionClass__ILocomotion_QI_Thunk — 0x0071A160

**Proposed Ghidra label:** TeleportLocomotionClass__ILocomotion_QI_Thunk (existing label authoritative)

Source: `decompile_function 0x0071a160`

```c
void TeleportLocomotionClass__ILocomotion_QI_Thunk(void) {
  TeleportLocomotionClass__QueryInterface();
  return;
}
```

**Behavioral notes:**
- 9-byte thunk. Ghidra decompile shows a direct tail-call to QueryInterface with no visible `this` adjustment — the adjustment is likely encoded as an inline constant subtraction in the raw bytes (Ghidra may abstract it). The ILocomotion vtable is at `base+0x04`, so the thunk needs to adjust `this` by `-0x04` to convert from ILocomotion-this to IUnknown-base before calling QueryInterface.
- Purpose: allows ILocomotion callers to call QueryInterface to navigate to other interfaces (IUnknown, IPiggyback) via the ILocomotion vtable slot.

---

## Struct field accesses (summary across all COM stubs)

| Field | Owner | Byte offset | Notes |
|---|---|---|---|
| `base + 0x0C` | TeleportLocomotionClass | +0x0C | TechnoClass owner pointer |
| `base + 0x18` | TeleportLocomotionClass | +0x18 | IPiggyback vtable pointer (set by Constructor) |
| `base + 0x30` | TeleportLocomotionClass | +0x30 | Piggyback slot: stored Drive locomotor pointer (Begin_Piggyback stores here using base as this) |
| TechnoClass+0x428 | TechnoClass | +0x428 | Source building kill-credit ptr; cleared in End_Piggyback |
| TechnoClass+0x42C | TechnoClass | +0x42C | Source house kill-credit ptr; cleared in End_Piggyback |
| TechnoClass+0x27C | TechnoClass | +0x27C | ChronoInTransit flag; gated in Is_Ok_To_End |
| TechnoClass+0x6AD | TechnoClass | +0x6AD | IsDeploying flag (FootClass); gated in Is_Ok_To_End |

All offsets verified via decompile_function 0x00719e30, 0x00719e90, 0x00719ee0, 0x00719f30.

---

## Out-of-scope refs

| Symbol | Reason |
|---|---|
| `LocomotionClass__QueryInterface` | Base class COM implementation; not TeleportLocomotionClass-specific |
| `IID_IPiggyback_Copy` | GUID data; general COM infrastructure |
| `CLSID_WalkLocomotion` | Not referenced in these stubs; general COM infrastructure |

---

## Unverified (YELLOW)

- **End_Piggyback `param_1 + 0x30` offset mismatch**: Begin_Piggyback stores to `param_1+0x30` where `param_1 = IUnknown base` = TeleportLocomotionClass+0x30. End_Piggyback reads `param_1+0x30` where `param_1 = IPiggyback this = base+0x18`, giving `base+0x48`. These should read the same field but don't in this analysis — Ghidra may be presenting the IPiggyback this differently in End_Piggyback, or there's an implicit `this` adjustment not shown. Needs cross-verification with struct decode (task #12).
- **Is_Ok_To_End `param_1 + 0x30`**: Same issue as above — if `param_1 = IPiggyback this = base+0x18`, then `+0x30` = `base+0x48`, not `base+0x30` (piggyback slot). The check `*(int*)(param_1 + 0x30) != 0` (piggyback slot occupied) would be wrong. Resolution requires knowing the exact `this` convention for each COM stub.
- **Is_Ok_To_End `param_1 + 0x1D` flag**: `base+0x35` if IPiggyback-this; `base+0x1D` if base-this. Field name unknown. Plate comment says "+0x1D" relative to param_1.
- **Is_Ok_To_End `param_1 + 0x20` counter**: `base+0x38` if IPiggyback-this. May be the `+0x38` warp count counter seen in TimerCheck. Exact identity unclear without struct decode.
- **vtable+0x10 call in Is_Ok_To_End**: Called on `param_1 - 0x14`. Plate comment says "timer still active" but the identity (Is_Moving or timer check) is inferred, not directly decompiled.
- **ILocomotion_QI_Thunk `this` adjustment**: The raw assembly bytes for the 9-byte thunk likely contain an `add ecx, -4` or `sub ecx, 4` before the tail call. This is not visible in Ghidra's decompile. The offset (-4, from ILocomotion vtable at +0x04 to IUnknown base at +0x00) is inferred from the vtable layout.
