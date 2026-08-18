# BulletClass Lifecycle (Constructor / Destructor / UpdateTarget) + Tier-1 Field Verifications

**Program:** gamemd.exe
**Source:** Direct decompilation via Ghidra MCP
**Confidence:** HIGH — every claim cites the decompile snippet or assembly site
**Complements:**
- `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` (call-chain summary; this doc adds line-by-line bodies)
- `BULLETCLASS_TRAJECTORY_AND_HOMING.md` (had a speculative claim about +0x150; corrected here)
- `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md` (Init/PostInit at 0x004664C0)
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` (had wrong label for +0x82; corrected here)
- `GREATEST_THREAT_SCAN_GHIDRA_REPORT.md` (had wrong label for +0x82; corrected here)
- `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` (param_1[0xEE] notation explained here)

---

## Key addresses

| Address | Function | Notes |
|---------|----------|-------|
| `0x00466380` | `BulletClass::Constructor` | Allocator-side constructor (Ghidra label correct) |
| `0x00466560` | `BulletClass::~BulletClass` | **Ghidra mislabels the function itself as `BulletClass__Constructor`**; it is the destructor (deregisters, chains to base dtor — tail call now correctly labeled `ObjectClass__Destructor`) |
| `0x004664C0` | `BulletClass::Init` (PostInit) | Sets Type, Owner, Target, WH, Damage, Speed, Bright; sets +0x150 = 0x100 |
| `0x00468430` | `BulletClass::UpdateTarget` | Chrono-warp target retargeter; sole caller is `TeleportLocomotionClass::StateMachineTick` |
| `0x0046B5C0` | COM scalar destructor wrapper | `~BulletClass()` + `if (param_2 & 1) operator_delete(this)` — standard MSVC pattern |
| `0x006C5086` | COM ClassFactory::CreateInstance | Allocates 0x160 bytes via `operator_new`, calls Constructor |
| `0x00710470` | `TechnoClass::SetInOpenTransport` | The sole writer of `+0x82 = 1` in TechnoClass; canonical name for the field |

---

## 1. Tier-1 verifications

### 1.1 BulletClass+0x150 — DirectRocker force scale (Q8.8 fixed-point)

**Prior speculation:** "Possibly draw priority / sort key" (BULLETCLASS_TRAJECTORY_AND_HOMING §7.1)
or "Possibly DrawFlags or Facing" (BULLET_CLASS_LAYOUT line 125–127).

**Verified meaning:** The field is a **force scale for the DirectRocker warhead branch**
in `WarheadTypeClass::Detonate`, applied as a Q8.8 fixed-point multiplier. Default
value 0x100 = 1.0×.

**Write site (the only one):** `BulletClass::Init` at `0x004664C0` writes the constant `0x100`:
```c
*(undefined4 *)(param_1 + 0x150) = 0x100;
```
A binary search for other writers (`MOV [reg+0x150], imm32` and `MOV [reg+0x150], reg`)
inside the bullet-area code range turned up no other writers — the field is set once
at init and never modified.

**Read site:** `WarheadTypeClass::Detonate` at `0x004690B0` (the function operates on
a BulletClass — Ghidra mislabels the param), inside the DirectRocker branch (warhead
flag at offset 0x14F set, target non-NULL, target is not infantry). Decompile:

```c
// param_1 = BulletClass*  (Ghidra labels function as WH::Detonate but its `this` is the bullet)
// param_1[0x4a] = WH (offset 0x128)
// param_1[0x2b] = Type (offset 0xAC)
// param_1[0x2c] = Owner (offset 0xB0)
// param_1[0x43] = Target (offset 0x10C)
// param_1[0x1b] = Health/Damage (offset 0x6C)
// param_1[0x54] = +0x150 (the scale field)

// DirectRocker branch reached when wh+0x14F set AND target != NULL AND target is NOT infantry
fVar6 = ((float)(param_1[0x54] * param_1[0x1b] >> 8)
        * *(float *)(g_RulesClass_Instance + 0x18b4))
        / (float)_DAT_0081aef8;
if ((float)_DAT_007e3cc8 <= fVar6) {
    fVar6 = 4.0;     // saturate at 4.0
}
// ... compute push direction toward target ...
(**(code **)(*piVar14 + 0x3d8))(&uStack_58, fVar6);  // call target.vtbl+0x3D8 (apply rocker force)
piVar14[0xaa] = param_1[0x2c];                        // target+0x2A8 = bullet.Owner (last attacker)
*(int **)(param_1[0x2c] + 0x2a8) = piVar14;           // bullet.Owner+0x2A8 = target
```

**Plain math:**
```
rocker_force = (BulletClass.RockerScale × BulletClass.Damage) / 256
             × Rules+0x18b4 (a global float constant)
             / global_constant_at_0x0081aef8
if (rocker_force >= constant_at_0x007e3cc8): rocker_force = 4.0
```

The result is passed to the target's vtable+0x3D8 method (the locomotor / physics
push entry point), which physically rocks the target.

**Why it's always 1.0 in practice:** No INI key or other code path modifies this field
before detonation. It is set to 0x100 in Init and stays that way through the bullet's
flight. The field exists as if it were intended to be modulated (e.g., per-burst
damping, per-aircraft penalty), but in shipping YR no caller modulates it.

**Assembly verification at the read site (`0x004697FC`):**
```
004697d4: MOV EAX, [ESI+0xb0]      ; ESI is BulletClass — confirms via .Owner
...
004697fc: MOV EAX, [ESI+0x150]     ; <-- READ +0x150
00469808: IMUL EAX, [ESI+0x6c]     ; * Damage (ObjectClass.Health)
0046980c: SAR  EAX, 0x8            ; >> 8 (Q8.8 normalize)
00469813: FILD [ESP+0x1c]
00469817: FMUL [ECX+0x18b4]        ; * Rules+0x18b4
0046981d: FDIV [0x0081aef8]        ; / global const
00469823: FCOM [0x007e3cc8]        ; compare to saturation threshold
```

**Better field name:** `RockerScale` (or `LocomotorForceScale` to be more general — the
vtable+0x3D8 call is reused by IsLocomotor warheads too).

---

### 1.2 TechnoClass+0xEE barrel-alternation bit identity — closed via int*-indexing

**Prior claim (`ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` line 395):**
> the `this->field_0xEE & 0x80000001` check is reading the low bit of `CurrentBurstIndex`
> (TechnoClass field — stored near `+0x3B8` but folded into a different offset at `+0xEE`).

This phrasing was confusing — a single field cannot be "stored at +0x3B8 but folded
into +0xEE." The true explanation is the **int*-indexing pitfall** explicitly called
out in CLAUDE.md: when `param_1` is typed `int *`, `param_1[N]` is a **byte offset
of N×4**, not N.

**Verification:** The decompile site uses `param_1[0xEE]`. The function takes
`param_1: int *`. Therefore:
```
byte_offset = 0xEE * 4 = 0x3B8
```
which **is** `CurrentBurstIndex` (TechnoClass+0x3B8) — already documented in
`BURST_WEAPON_FIRING_GHIDRA_REPORT.md` and `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`.

**No new field. No new RE.** The original interpretation was correct; the index-vs-byte
notation just made it look like a separate field. The "TechnoClass+0xEE" referenced in
the open-questions list does not exist as a separate field — it is the same byte as
TechnoClass+0x3B8 viewed through the `int *` decompile lens.

**Recommendation:** When transcribing decompile snippets to docs, always note when
`param_1` is `int *` and convert to byte offsets. The existing `ANIMCLASS_SPAWN_PATHS`
phrasing should be edited for clarity (suggested replacement: "the access uses int*
indexing so `param_1[0xEE]` = byte offset 0x3B8 = CurrentBurstIndex").

---

### 1.3 BulletType+0x2A9 = FirersPalette (no gap — verified)

Already cross-verified in `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`,
`BULLET_CLASS_AI_GHIDRA_REPORT.md`, `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`,
and `READINI_FIELD_MAPS.md`. Re-confirmed during this session by reading
`BulletClass::Init` at `0x004664C0`:
```c
if ((*(char *)(param_2 + 0x2a9) == '\0') || (param_4 == 0)) {
    *(undefined4 *)(param_1 + 0x114) = 0xffffffff;       // -1 = no remap
} else {
    *(undefined4 *)(param_1 + 0x114) = *(undefined4 *)(*(int *)(param_4 + 0x21c) + 0x16054);
    // pull color from owner.House.HouseTypeClass+0x16054
}
```
**No correction needed.** Closed.

---

### 1.4 WeaponType+0x12F = Bright (no gap — verified)

Already cross-verified in `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`,
`FIRE_AT_PIPELINE_GHIDRA_REPORT.md`, `FIRE_AT_ANALYSIS.md`,
`BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`, and `READINI_FIELD_MAPS.md`. Confirmed
flow: `weapon+0x12F` is read as the `bright` parameter (param_8) to
`BulletClass::Init`, which stores it at `bullet+0xE0`. **No correction needed.** Closed.

---

### 1.5 TechnoClass+0x82 — InOpenToppedTransport (corrects two prior docs)

**Conflicting prior labels:**
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md:1078` — calls it `HasBeenPlaced` (likely "InLimbo" or "NeverUnlimboed")
- `GREATEST_THREAT_SCAN_GHIDRA_REPORT.md:674` — calls it `WarpedOutOf / airstrike-in-progress`
- `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md:57` — calls it `InOpenToppedTransport`

**Binary evidence (the decisive write site):** A byte-pattern search for
`MOV [reg+0x82], 1` (`C6 86 82 00 00 00 01` and `C6 87 82 00 00 00 01`) inside the
TechnoClass code range returns exactly one match in the bullet/techno area:
`TechnoClass::SetInOpenTransport` at `0x00710470`:

```c
void TechnoClass__SetInOpenTransport(int *param_1) {
    if (param_1 != (int *)0x0) {
        *(undefined1 *)((int)param_1 + 0x82) = 1;     // <-- THE WRITE
        (**(code **)(*param_1 + 0x3d0))();             // vtable+0x3D0 (Hide / RemoveFromMap-ish)
        FUN_0055baa0(param_1, 0);                       // remove from cell occupancy
    }
}
```

The function name is from Ghidra's analyzer (likely RTTI-derived) — not a guess. The
body — set flag, call vtable+0x3D0 (visibility), call cell-removal helper — exactly
matches the semantics of "loaded into an open-topped transport: hide and limbo from
the map but stay alive." That is decisive.

**Why the other docs are wrong:**
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` derived "HasBeenPlaced" from observing that
  `Set_Destination_Internal` early-exits when this flag is set. That gating is also
  consistent with InOpenToppedTransport (a unit inside a transport cannot self-issue
  movement orders) — so it isn't proof of the HasBeenPlaced semantics.
- `GREATEST_THREAT_SCAN_GHIDRA_REPORT.md` derived "WarpedOutOf / airstrike" from
  observing reads of the field during chrono-warp / airstrike code paths. Those reads
  are likely checking "am I in transit / contained" — consistent with the same flag.
  Aircraft-delivered airstrikes and chrono-warped units may share the same "in
  carrier" state machinery.
- `FIRE_AT_PIPELINE_GHIDRA_REPORT.md:109,189` references "Airstrike (`this+0x82`)";
  this read is plausibly a "is this unit currently mid-airstrike / inside its delivery
  vehicle" check — same flag, different read context.

**Verdict:** `TechnoClass+0x82 = InOpenToppedTransport` (canonical, byte). Same byte
across all ObjectClass-derived classes — the BulletClass layout doc had it right.
Reads in airstrike/warp code paths are reading this same flag for "am I currently in
transit/contained," not a separate field.

**Action items for other docs (not edited here — flagging only):**
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md:1078` — change label from `HasBeenPlaced` to
  `InOpenToppedTransport`; cite `TechnoClass::SetInOpenTransport @ 0x00710470`.
- `GREATEST_THREAT_SCAN_GHIDRA_REPORT.md:674,694` — change "WarpedOutOf /
  airstrike-in-progress" to "InOpenToppedTransport (overloaded as 'in transit'
  during chrono-warp and airstrike delivery)."
- `FIRE_AT_PIPELINE_GHIDRA_REPORT.md:109,189` — clarify that "this+0x82" is
  InOpenToppedTransport, used by airstrike-delivery code as an "in carrier" check.

---

## 2. Tier-2 lifecycle: Constructor / Destructor / UpdateTarget bodies

### 2.1 BulletClass::Constructor — `0x00466380`

**Sole caller:** `0x006C50CF` inside `FUN_006C5086` (the COM ClassFactory::CreateInstance
for BulletClass). This call site:
1. `operator_new(0x160)` → 352-byte allocation
2. Calls `BulletClass::Constructor` (this function)
3. (Caller then sets up COM `IUnknown` reference counting and returns the new bullet to the COM client)

**Full body (decompile, with byte-offset annotations):**

```c
undefined4 * __fastcall BulletClass::Constructor(undefined4 *param_1)  // ECX = this
{
    int iVar1;
    char cVar2;

    ObjectClass__Constructor();                       // base ctor: sets ObjectClass fields (vtables, location, health, etc.)
    param_1[0x2b] = 0;                                 // +0xAC = Type pointer = NULL
    param_1[0x2c] = 0;                                 // +0xB0 = Owner pointer = NULL
    *(undefined1 *)(param_1 + 0x2d) = 0;               // +0xB4 = IsNetPlayerOwned byte = 0

    ProximityDetector__Init();                         // initializes embedded sub-object at +0xB8 (sets +0xB8/+0xC4 = g_CurrentFrame)

    param_1[0x48] = 0;                                 // +0x120 = ApproachSum (low half) = 0
    *(undefined1 *)(param_1 + 0x41) = 1;               // +0x104 = IsActive byte = 1
    *(undefined1 *)((int)param_1 + 0x105) = 1;         // +0x105 = IsCourseLocked byte = 1
    *(undefined1 *)(param_1 + 0x38) = 0;               // +0xE0 = Bright byte = 0
    param_1[0x40] = 0;                                 // +0x100 = unknown int = 0
    param_1[0x42] = 0;                                 // +0x108 = CourseLockCounter = 0
    param_1[0x43] = 0;                                 // +0x10C = Target = NULL
    param_1[0x44] = 0;                                 // +0x110 = TargetSpeed = 0
    param_1[0x45] = 0xffffffff;                        // +0x114 = HouseColorIndex = -1 (no remap)
    param_1[0x46] = 0;                                 // +0x118 = ApproachSampleCount = 0
    param_1[0x49] = 0;                                 // +0x124 = ApproachSum (high half) = 0
    param_1[0x4a] = 0;                                 // +0x128 = WH = NULL
    *(undefined1 *)(param_1 + 0x4b) = 0;               // +0x12C = AnimFrame byte = 0
    *(undefined1 *)((int)param_1 + 0x12d) = 0;         // +0x12D = AnimTimer byte = 0
    param_1[0x4c] = 0;                                 // +0x130 = WeaponType = NULL
    *(undefined2 *)(param_1 + 0x53) = 0xffff;          // +0x14C = LastCell.X = -1 (short)
    *(undefined2 *)((int)param_1 + 0x14e) = 0xffff;    // +0x14E = LastCell.Y = -1 (short)
    param_1[0x55] = 0;                                 // +0x154 = BounceAnim = NULL
    *(undefined1 *)(param_1 + 0x56) = 0;               // +0x158 = IsWaitingForAnim byte = 0

    *param_1     = &vtable__BulletClass;               // +0x00 = primary vtable @ 0x7E46E4
    param_1[1]   = &vtable__BulletClass__secondary_4;  // +0x04 = IRTTITypeInfo vtable @ 0x7E46C8
    param_1[2]   = &vtable__BulletClass__secondary_8;  // +0x08 = INoticeSink vtable @ 0x7E46C0
    param_1[3]   = &vtable__BulletClass__secondary_12; // +0x0C = INoticeSource vtable @ 0x7E46B8

    AbstractClass__AssignUniqueID(param_1 + 1);        // sets UniqueID at +0x10 via global allocator

    param_1[0x4d] = 0;                                 // +0x134 = SourceCoord.X = 0
    param_1[0x4e] = 0;                                 // +0x138 = SourceCoord.Y = 0
    param_1[0x4f] = 0;                                 // +0x13C = SourceCoord.Z = 0
    param_1[0x50] = 0;                                 // +0x140 = TargetCoord.X = 0
    param_1[0x51] = 0;                                 // +0x144 = TargetCoord.Y = 0
    param_1[0x52] = 0;                                 // +0x148 = TargetCoord.Z = 0

    // Register in BulletClass global array (DAT_00a8ed40 = vector header)
    if (DAT_00a8ed48 <= DAT_00a8ed50) {                // count >= capacity?
        if ((DAT_00a8ed4d == '\0') && (DAT_00a8ed48 != 0)) {
            return param_1;                             // can't grow — bail without registering
        }
        if (DAT_00a8ed54 < 1) {
            return param_1;
        }
        cVar2 = (**(code **)(DAT_00a8ed40 + 8))(DAT_00a8ed54 + DAT_00a8ed48, 0);  // vtable+8 = Resize
        if (cVar2 == '\0') {
            return param_1;                             // resize failed — bail
        }
    }

    iVar1 = DAT_00a8ed50 * 4;
    DAT_00a8ed50 = DAT_00a8ed50 + 1;
    *(undefined4 **)(DAT_00a8ed44 + iVar1) = param_1;  // append `this` to array

    return param_1;
}
```

**Note: +0x150 (RockerScale) is NOT set here** — it is set later by `BulletClass::Init`
(0x004664C0) when the bullet is initialized for firing.

**Global array layout (`DAT_00a8ed40` = `BulletClass::Array` — the engine-wide bullet
DynamicVector):**

| Offset | Field |
|--------|-------|
| `DAT_00a8ed40 + 0x00` | vtable / type info |
| `DAT_00a8ed44` | data pointer (array of `BulletClass *`) |
| `DAT_00a8ed48` | capacity |
| `DAT_00a8ed4d` | byte: "is growable" flag |
| `DAT_00a8ed50` | current count |
| `DAT_00a8ed54` | growth increment |

The constructor's last block is the standard `DynamicVector::Add(this)` inlined.
The destructor mirrors this (see §2.2).

---

### 2.2 BulletClass::~BulletClass — `0x00466560`

**⚠️ Ghidra mislabels this function as `BulletClass__Constructor` (same name as
0x00466380).** It is the destructor — the body deregisters from arrays, clears
member pointers, and chains to the base destructor. The mislabeling is most likely
because Ghidra's name analyzer found two functions with constructor-shaped vtable
writes and assigned both the constructor name. The function at `0x00466560` is
actually `BulletClass::~BulletClass`.

**Sole caller:** `0x0046B5C3` inside `FUN_0046b5c0` (the COM scalar destructor
wrapper — see §2.5).

**Full body (decompile, with byte-offset annotations):**

```c
void __fastcall BulletClass::~BulletClass(undefined4 *param_1)  // ECX = this
{
    int iVar1;
    undefined4 *local_4;

    // Re-establish vtable pointers — required so virtual calls during destruction
    // (e.g., the chained ObjectClass dtor) dispatch to BulletClass methods.
    *param_1   = &vtable__BulletClass;
    param_1[1] = &vtable__BulletClass__secondary_4;
    param_1[2] = &vtable__BulletClass__secondary_8;
    param_1[3] = &vtable__BulletClass__secondary_12;

    local_4 = param_1;
    Detach_From_All_Lists();                           // pre-destruction cleanup (verified: get_function_by_address 0x007258d0 → "Detach_From_All_Lists")

    // If the bullet is in the "waiting for impact anim" array (+0x158 set),
    // remove it from that secondary tracking vector at DAT_00b0f5b8.
    if (*(char *)(param_1 + 0x56) != '\0') {           // +0x158 = IsWaitingForAnim
        local_4 = param_1;
        iVar1 = (**(code **)(DAT_00b0f5b8 + 0x10))(&local_4);  // vtable+0x10 = Find_Index(this)
        if (((iVar1 != -1) && (iVar1 < DAT_00b0f5c8)) &&
            (DAT_00b0f5c8 = DAT_00b0f5c8 + -1, iVar1 < DAT_00b0f5c8)) {
            do {                                       // shift array elements down to fill the hole
                iVar1 = iVar1 + 1;
                *(undefined4 *)(DAT_00b0f5bc + -4 + iVar1 * 4)
                    = *(undefined4 *)(DAT_00b0f5bc + iVar1 * 4);
            } while (iVar1 < DAT_00b0f5c8);
        }
    }

    if (g_GameActive != '\0') {
        ObjectClass__Conceal();                        // hide / remove from cell occupancy if game running
    }

    param_1[0x2b] = 0;                                 // +0xAC: Type = NULL (clear pointer to type)
    param_1[0x2c] = 0;                                 // +0xB0: Owner = NULL (clear back-pointer to firer)
    param_1[0x55] = 0;                                 // +0x154: BounceAnim = NULL (clear anim pointer)

    // Remove from the main BulletClass DynamicVector (DAT_00a8ed40)
    local_4 = param_1;
    iVar1 = (**(code **)(DAT_00a8ed40 + 0x10))(&local_4);  // vtable+0x10 = Find_Index(this)
    if (((iVar1 != -1) && (iVar1 < DAT_00a8ed50)) &&
        (DAT_00a8ed50 = DAT_00a8ed50 + -1, iVar1 < DAT_00a8ed50)) {
        do {                                           // shift to close hole
            iVar1 = iVar1 + 1;
            *(undefined4 *)(DAT_00a8ed44 + -4 + iVar1 * 4)
                = *(undefined4 *)(DAT_00a8ed44 + iVar1 * 4);
        } while (iVar1 < DAT_00a8ed50);
    }

    ObjectClass__Destructor();                         // chains to base dtor (verified: decompile_function 0x00466560 — tail call now reads ObjectClass__Destructor)
    return;
}
```

**The tail call is now correctly labeled `ObjectClass__Destructor` in Ghidra** (verified:
`decompile_function 0x00466560` — stale `⚠️ MISLABEL` note removed). The chained call
from a derived destructor's tail can only legally be the base destructor in MSVC's ABI,
consistent with this label.

**Two arrays involved:**

| Global | Purpose |
|--------|---------|
| `DAT_00a8ed40` | Main `BulletClass::Array` — every live bullet is registered here on construct, deregistered on destruct |
| `DAT_00b0f5b8` | Secondary "waiting for impact anim" tracker — bullets register here when their impact animation hasn't finished playing yet (between `BulletDetonation` and `~BulletClass`). The destructor only deregisters from this if `+0x158 = IsWaitingForAnim` is set. |

---

### 2.3 BulletClass::UpdateTarget — `0x00468430`

**Sole caller:** `0x007193EE` inside `TeleportLocomotionClass::StateMachineTick`
(at `0x007192F0`).

**This is a critical correction to existing docs.** The bullet trajectory doc
(`BULLETCLASS_TRAJECTORY_AND_HOMING.md` §2.9) implies UpdateTarget is called from
`BulletClass::AI` when a homing missile loses its target. **It is not.** The function
is called only from the chrono-warp state machine. When a unit being targeted by
homing missiles enters teleport state, `TeleportLocomotionClass::StateMachineTick`
walks all in-flight bullets that target the warping unit and calls UpdateTarget on
each, so the bullets either retarget to the cell at the warp source (if the cell
is still on-map) or null their Target (if not).

The "homing target loss" handling that exists in `BulletClass::AI` (the
`Target->IsOnMap()` check, the FlightLevel detonation gate) is in-line in AI — not
delegated to UpdateTarget. The two systems do similar things for different reasons.

**Full body:**

```c
void __fastcall BulletClass::UpdateTarget(int param_1)
{
    int iVar1;
    int iVar2;
    char cVar3;
    int *piVar4;
    undefined4 uVar5;
    undefined1 local_c[12];

    // Get target's current world coords via vtable+0x48 (AbstractClass::GetCoords)
    piVar4 = (int *)(**(code **)(**(int **)(param_1 + 0x10c) + 0x48))(local_c);
    iVar1 = *piVar4;       // target.X
    iVar2 = piVar4[1];     // target.Y

    if (g_MapEditorMode == 0) {
        // Check target's IsOnMap via vtable+0x54
        cVar3 = (**(code **)(**(int **)(param_1 + 0x10c) + 0x54))();
        if ((cVar3 == '\0') &&
            (((short)(iVar1 + (iVar1 >> 0x1f & 0xffU) >> 8) != DAT_0089ddf0 ||
             ((short)(iVar2 + (iVar2 >> 0x1f & 0xffU) >> 8) != DAT_0089ddf2)))) {
            // Target NOT on map AND target's coords are NOT the off-map sentinel cell
            // → retarget to the CellClass at target's last position (preserves "ground impact")
            uVar5 = MapClass__Get_CellClass(&stack0xffffffe8);
            *(undefined4 *)(param_1 + 0x10c) = uVar5;     // bullet.Target = cell
            return;
        }
    }
    // Otherwise (target IS on map, OR coords are the sentinel, OR map editor mode)
    // → null the target, bullet will detonate without snap-to-target
    *(undefined4 *)(param_1 + 0x10c) = 0;
    return;
}
```

**Plain logic:**
```
target_pos = bullet.Target.GetCoords()                    // vtable+0x48
if not g_MapEditorMode:
    if not bullet.Target.IsOnMap():                       // vtable+0x54
        cell_x_cells = (target_pos.X + sign_extend) >> 8  // leptons → cell coord (high 8 bits)
        cell_y_cells = (target_pos.Y + sign_extend) >> 8
        if (cell_x_cells, cell_y_cells) != (DAT_0089ddf0, DAT_0089ddf2):
            // target is gone-but-on-map: redirect bullet at the ground cell where target was
            bullet.Target = MapClass.Get_CellClass(target_pos)
            return
bullet.Target = NULL
```

**Coord-to-cell conversion:** `(int + (int >> 0x1F & 0xFF)) >> 8` is the
sign-correcting ASR-by-8 used everywhere in RA2 to convert leptons to cell
coordinates (cell = leptons / 256, with sign-bias for negatives).

**Sentinels (`DAT_0089DDF0`, `DAT_0089DDF2`):** These are the map's "off-map"
sentinel cell coords (negative, used to mark "no valid cell here"). When the target's
coords match the sentinel, retargeting to a cell is impossible — the function falls
through to clear Target.

**Global `g_MapEditorMode`:** Skips the IsOnMap check in the editor (presumably so
that placed-but-not-running scenarios behave deterministically).

---

### 2.4 Tier-2 corrections to existing docs

The `BULLETCLASS_TRAJECTORY_AND_HOMING.md` excerpt at §2.9:
> ```
> // BulletClass::UpdateTarget (0x00468430):
> target_coords = Target->GetCoords()
> if !Target->IsOnMap():
>     cell_at = target_coords -> CellClass
>     if cell is valid (not map edge sentinel):
>         Target = CellClass at target's position  // retarget to ground
>     else:
>         Target = NULL
> ```

— is **logically correct** but the surrounding text framing ("Lost Target Handling")
implies it runs when a homing bullet loses its target. It does not. It runs only
from the chrono-warp state machine. The lost-target handling in `BulletClass::AI`
(the FlightLevel detonation gate) is a separate inline mechanism.

Suggested edit to the trajectory doc: rename §2.9 from "Lost Target Handling" to
"Target Update from Chrono-Warp," and add a cross-reference to
`TeleportLocomotionClass::StateMachineTick` as the sole caller.

---

### 2.5 COM scalar destructor wrapper — `FUN_0046B5C0`

This is the standard MSVC C++ scalar destructor used by COM `Release` paths:

```c
undefined4 __thiscall FUN_0046b5c0(undefined4 param_1, byte param_2)
{
    BulletClass__Constructor();    // ⚠️ MISLABEL — Ghidra labels this as Constructor but it is `~BulletClass()` at 0x00466560
    if ((param_2 & 1) != 0) {
        FUN_007c8b3d(param_1);     // operator delete(this)
    }
    return param_1;
}
```

`param_2` is the standard MSVC vector-deletion flag:
- bit 0 set → call `operator_delete` after the destructor (when the object was
  heap-allocated and ownership transfers to the caller of the wrapper)
- bit 0 clear → just run the destructor (when the object is a stack temporary or
  embedded, and the caller will free the storage itself)

The "constructor" call is the one at `0x00466560` (the destructor — see Ghidra
mislabel note in §2.2).

---

## 3. Verification log

| Claim | Evidence | Verdict |
|-------|----------|---------|
| `BulletClass+0x150` is a Q8.8 fixed-point scale | Read at `0x004697FC`: `MOV EAX,[ESI+0x150]; IMUL EAX,[ESI+0x6c]; SAR EAX,0x8` (multiply by Damage, divide by 256) | ✓ verified |
| `+0x150` default value is 0x100 | Init at `0x004664C0` writes `*(int *)(param_1 + 0x150) = 0x100` | ✓ verified |
| `+0x150` has no other writers | Byte-pattern search for `MOV [reg+0x150], imm32` (`C7 86/87/83 50 01 00 00`) returns no matches in bullet code; `MOV [reg+0x150], reg` matches are all in unrelated Blitter struct (offset coincidence) | ✓ verified |
| `+0x150` is read in DirectRocker branch (wh+0x14F set, target non-NULL, target not infantry) | Decompile of WH::Detonate at `0x004690b0`: the `else` clause of `(wh+0x14F == 0 \|\| target == NULL \|\| target->vtbl+0x2c == 1)` contains the +0x150 read; vtbl+0x3D8 is then called on target with the computed force | ✓ verified |
| `+0x150` force is saturated at 4.0 | Decompile: `if ((float)_DAT_007e3cc8 <= fVar6) fVar6 = 4.0;` | ✓ verified (per binary; `_DAT_007e3cc8` value is the saturation comparison threshold) |
| `+0xEE` "barrel-alternation field" = `param_1[0xEE]` (int*-indexed) = byte offset 0x3B8 = CurrentBurstIndex | `param_1[0xEE]` with `param_1: int *` evaluates to `*(int *)((char *)param_1 + 0xEE * 4)` = byte offset 0x3B8; CurrentBurstIndex documented at +0x3B8 in `BURST_WEAPON_FIRING_GHIDRA_REPORT.md` | ✓ verified (notation pitfall, no new field) |
| `TechnoClass+0x82 = InOpenToppedTransport` | Sole `MOV [reg+0x82],1` writer in techno code is `TechnoClass::SetInOpenTransport @ 0x00710470`; body sets flag, calls vtable+0x3D0 (Hide), calls cell-removal helper | ✓ verified |
| `TechnoClass+0x82` is NOT "HasBeenPlaced" | The TECHNOCLASS_EXPANDED claim was inferred from `Set_Destination_Internal` early-exit; that gating is also consistent with InOpenToppedTransport (units in transports cannot self-issue moves) — therefore not decisive | ✓ corrected |
| `TechnoClass+0x82` is NOT "WarpedOutOf" | The GREATEST_THREAT_SCAN claim was inferred from reads in chrono-warp/airstrike code paths; same flag, just used as "in transit" marker by those paths | ✓ corrected |
| Constructor at `0x00466380` registers in `DAT_00a8ed40` array | Last block: `DAT_00a8ed50++; *(BulletClass **)(DAT_00a8ed44 + iVar1) = param_1` | ✓ verified |
| Constructor's sole caller is COM ClassFactory | `get_xrefs_to(0x00466380)` → `0x006C50CF in FUN_006C5086` (COM CreateInstance) | ✓ verified |
| `0x00466560` is the destructor, not a second constructor | Body deregisters from arrays, clears Type/Owner/BounceAnim, chains to ObjectClass tail-call. Ghidra's "Constructor" label collides with the real ctor at `0x00466380`. | ✓ verified |
| Destructor's sole caller is COM scalar destructor wrapper | `get_xrefs_to(0x00466560)` → `0x0046B5C3 in FUN_0046b5c0`; FUN_0046b5c0 body matches MSVC `~T(); if (flag) operator_delete(this);` pattern | ✓ verified |
| Destructor's tail call is `ObjectClass__Destructor` | Now correctly labeled in Ghidra (verified: `decompile_function 0x00466560` — reads `ObjectClass__Destructor()`); also confirmed by MSVC ABI: tail-position chain from derived dtor must be the base dtor | ✓ verified |
| `BulletClass::UpdateTarget` sole caller is TeleportLocomotionClass::StateMachineTick | `get_xrefs_to(0x00468430)` → `0x007193EE in TeleportLocomotionClass__StateMachineTick` (function entry `0x007192F0`) | ✓ verified |
| UpdateTarget retargets to ground cell when target is off-map but coords are valid | Decompile: if `!IsOnMap` && coords ≠ sentinel → `bullet.Target = Get_CellClass(target_pos)`; else → `bullet.Target = NULL` | ✓ verified |

---

## 4. Open questions

1. **`+0x150` modulation:** No INI key or runtime path modifies `+0x150` from its
   default of 0x100 in shipping YR. Does any mod-extension (Ares, Phobos) hook this
   field? Out of scope here; flagged for vanilla-vs-mod parity work.

2. **`DAT_00b0f5b8` array identity:** Confirmed to be the "bullets waiting for impact
   anim" tracker. The exact vtable type would be useful for cross-referencing other
   limbo-state work; not investigated further this session.

3. **`g_MapEditorMode`:** UpdateTarget skips the IsOnMap check in editor mode. The
   exact set of behaviors gated by this global is not documented anywhere in the
   `ra2-rust-game-docs/` archive and may be worth a separate small audit.

4. **Suggested doc edits (not performed in this session):**
   - `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md:1078` — relabel `+0x82` to `InOpenToppedTransport`.
   - `GREATEST_THREAT_SCAN_GHIDRA_REPORT.md:674,694` — clarify `+0x82` is the same
     InOpenToppedTransport byte (not a separate WarpedOutOf field).
   - `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md:395-397` — rephrase the "+0xEE folded
     into +0x3B8" passage to explicitly call out the `int *` indexing convention.
   - `BULLETCLASS_TRAJECTORY_AND_HOMING.md` §2.9 — rename "Lost Target Handling" to
     "Target Update from Chrono-Warp" and note the sole caller is `TeleportLocomotionClass`.
   - `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md:125` — relabel `+0x150` to "RockerScale
     (DirectRocker force scale, Q8.8 fixed-point, default 0x100)" and remove the
     "DrawFlags or Facing" speculation.
   - `BULLETCLASS_TRAJECTORY_AND_HOMING.md` §7.1 — same correction for `+0x150`.

---

## 5. Decompilation source functions

| Address | Name | Role |
|---------|------|------|
| `0x00466380` | `BulletClass::Constructor` | Field initialization + array registration |
| `0x00466560` | `BulletClass::~BulletClass` | Cleanup + array deregistration (Ghidra still mislabels function itself as Constructor; tail call now correctly labeled ObjectClass__Destructor) |
| `0x004664C0` | `BulletClass::Init` | PostInit (Type/Owner/Target/WH/Damage/Speed/Bright); writes `+0x150 = 0x100` |
| `0x00468430` | `BulletClass::UpdateTarget` | Chrono-warp target retargeter |
| `0x004690B0` | `WarheadTypeClass::Detonate` (operates on BulletClass — Ghidra mislabels param) | Reads `+0x150` for DirectRocker force at `0x004697FC` |
| `0x0046B5C0` | COM scalar destructor wrapper | `~BulletClass(); if (flag) operator_delete(this);` |
| `0x00710470` | `TechnoClass::SetInOpenTransport` | Sole writer of `+0x82 = 1` in techno code; canonical name for the field |
| `0x007192F0` | `TeleportLocomotionClass::StateMachineTick` | Sole caller of `BulletClass::UpdateTarget` |

## 6. Confidence assessment

| Topic | Confidence | Notes |
|-------|-----------|-------|
| `+0x150` semantics (RockerScale Q8.8) | **HIGH** | Read site decompiled, formula verified, default value confirmed, no other writers found |
| `+0xEE` is int*-indexing for `+0x3B8` | **HIGH** | Pure notation explanation; cross-references existing well-documented field |
| `+0x82 = InOpenToppedTransport` | **HIGH** | Sole writer is a Ghidra-named function whose body matches the semantics |
| Constructor body | **HIGH** | Full decompile, all offsets cross-checked against existing layout doc |
| Destructor body | **HIGH** | Full decompile; mislabel explained by analyzer collision; tail-call identity confirmed by structure |
| UpdateTarget body + caller | **HIGH** | Full decompile, sole caller identified, contradiction with prior doc resolved |
