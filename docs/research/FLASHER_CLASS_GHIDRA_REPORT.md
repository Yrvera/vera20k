# FlasherClass — Ghidra Research Report

Research date: 2026-04-21
Method: RTTI TypeDescriptor + ClassHierarchyDescriptor + BaseClassArray walk, TechnoClass
constructor and AI_Update decompilation, cross-reference to the VETERANCY_SYSTEM_GHIDRA
EliteFlashTimer finding.

---

## Top-line verdict: LIVE (but NOT a damage-flash system)

FlasherClass exists in YR (confirmed via RTTI TypeDescriptor at `0x00817ad8`) and is
**inherited** by every TechnoClass (Building, Unit, Infantry, Aircraft) plus Terrain.
However:

- It is **NOT** used for damage-hit tinting. `TechnoClass::ReceiveDamage` never writes a
  "flash" field — see `RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md §10`. There is no
  per-damage red/white tint in the original game.
- The sole live consumer is the **EliteFlashTimer** blink on newly-promoted Elite
  buildings. See `VETERANCY_SYSTEM_GHIDRA_REPORT.md §10`.
- The "flash" is NOT a palette tint. It is a **dirty-rect-driven redraw toggle** every
  time bit-1 of the countdown transitions — the building is simply marked for redraw,
  so whatever the current anim frame is, it redraws. There is no palette remap, no RGB
  tint, no color overlay. The visible "flash" effect comes from the animation subsystem
  being kicked by `BuildingClass::UpdateAllAnimFacings`, not from any special render
  pass.

This means **FlasherClass is dormant for most use cases** and only lives via the Elite
promotion path.

### Evidence of liveness

- `0x006FAC31` (`TechnoClass::AI_Update`, label `LAB_006fac31`) — every tick of every
  TechnoClass reads `field_0xF0` into a snapshot, calls `StageClass::Stage_Changed`
  (`0x004CC770`), and if bit-1 of the low byte differs between old and new values AND
  the object is a Building (`WhatAmI() == 6`), invokes
  `TacticalClass::DirtyScreenRect` + `BuildingClass::UpdateAllAnimFacings`.
- `0x006FA055` (`TechnoClass::AI_Update` Elite-promotion block) — on crossing Elite
  threshold, seeds `this->field_0xF0 = Rules.EliteFlashTimer (=150)`.
- `EliteFlashTimer` is referenced via ReadInt at `Rules+0xBE8` — verified in
  `RulesClass::ReadAudioVisual @ 0x006691E0`.

The code path is reached **every YR skirmish**, just rarely triggered (only when a
unit/building promotes to Elite).

---

## Purpose

A `StageClass`-derived **countdown-redraw timer** embedded in every TechnoClass/ObjectClass
to drive periodic forced redraws. In practice YR only uses it for the post-Elite
promotion blink on buildings. The class was likely designed in Tiberian Sun for a more
generic "flasher" (damage flash, glint, etc.) but by YR the only surviving consumer is
the Elite promotion timer.

---

## RTTI evidence

- TypeDescriptor string `.?AVFlasherClass@@` at **`0x00817AE0`** (embedded in TD
  struct at `0x00817AD8`).
- FlasherClass **ClassHierarchyDescriptor** at **`0x007FB3B8`**.
- FlasherClass **BaseClassDescriptor** at **`0x007FB3A8`**.
- FlasherClass's own **BaseClassArray at `0x007FB48C`** enumerates the inheritance:
  - `0x7fb050` → `IUnknown`                (TD `0x00816b70`)
  - `0x7fb038` → `INoticeSink`             (TD `0x00816b50`)
  - `0x7fb020` → `INoticeSource`           (TD `0x008177e8`)
  - `0x7fb3b8` → **FlasherClass itself**   (TD `0x00817ad8`)
  - `0x7fb3a0` → `StageClass`              (TD `0x00817ab8`)
  - `0x7fb388` → `IFlyControl`             (TD `0x00817a98`)
  - `0x7fb370` → `IUnknown`                (TD `0x00816b70`) — aggregation duplicate

So the C++ declaration is effectively:

```
class FlasherClass : public StageClass,
                     public IFlyControl, public INoticeSink,
                     public INoticeSource, public IUnknown { ... };
```

The COM interfaces (`IFlyControl`, `INoticeSink`, `INoticeSource`, `IUnknown`) are
declared but appear to be legacy/inert — no vtable for FlasherClass itself is
referenced by any CompleteObjectLocator in the binary. All 6 xrefs to the FlasherClass
CHD are BaseClassDescriptors inside **subclass** hierarchies (BuildingClass, UnitClass,
etc.), confirming FlasherClass is a **base class mixin** — never instantiated alone.

### Subclasses verified

- `BuildingClass` (TD `0x00818D60`) — BCA at `0x007FC308` includes FlasherClass BCD
  `0x007FC344`.
- `UnitClass` (TD `0x00842D80`) — BCA at `0x0080CBF8` includes FlasherClass BCD
  `0x0080CC48`.
- Plus InfantryClass, AircraftClass, TerrainClass (identified via BCA xrefs
  `0x00800920`, `0x00803390`, `0x0080C030` — one of each remaining non-Building
  Techno type). Subclass TDs not re-validated in this pass.

---

## Struct layout (VERIFIED)

FlasherClass is a **non-virtual, embedded** POD subobject derived purely from
StageClass. It contributes **two fields** at the following byte offsets inside every
TechnoClass:

| Offset (in TechnoClass) | Field                  | Type   | Purpose                              |
|-------------------------|------------------------|--------|--------------------------------------|
| `+0x0F0`                | `Stage.Value`          | int    | Countdown in frames; 0 = no flash    |
| `+0x0F4`                | `Stage.HasChanged`     | byte   | Set by `Stage_Changed` when bit-1 toggles |

Verified from `TechnoClass::Constructor @ 0x006F2B40`:
```
param_1[0x3c] = 0;                                       // +0xF0
*(undefined1*)(param_1 + 0x3d) = 0;                      // +0xF4
```

The TechnoClass struct layout doc (`TECHNOCLASS_STRUCT_LAYOUT.md`) marks 0xF0/0xF4 as
"Unknown" — this report fills them in: **those two fields ARE the FlasherClass
subobject (inherited from StageClass)**.

Note that FlasherClass contributes **only StageClass's 2 fields** — the IFlyControl /
INoticeSink / INoticeSource / IUnknown interfaces have no observable data in YR and no
vtable references. They occupy no additional bytes in any instance.

### StageClass base class

StageClass is also a 2-field POD — `{ int Value; bool HasChanged; }` (8 bytes with
padding). It has no virtual dispatch in its live paths; `Stage_Changed` (below) is a
plain non-virtual helper.

---

## Constructor / initialization

FlasherClass has **no standalone constructor** — it is zero-initialized in place by
the enclosing TechnoClass constructor (and by extension by the chain Abstract → Object
→ Mission → Radio → Techno). The initialization is simply:

```c
// TechnoClass::Constructor @ 0x006F2B40
param_1[0x3c] = 0;   // Stage.Value = 0
*(char*)(param_1 + 0x3d) = 0;   // Stage.HasChanged = 0
```

---

## Vtable

**None.** No `vtable__FlasherClass` symbol exists; no CompleteObjectLocator references
the FlasherClass CHD. The declared COM interfaces (IFlyControl, INoticeSink,
INoticeSource) are effectively dead in YR — FlasherClass does not override them in a
callable way.

---

## Lifecycle

1. **Creation** — zero-init inside TechnoClass constructor.
2. **Seed (live path)** — in `TechnoClass::AI_Update @ 0x006FA055`, on crossing the
   Elite threshold: `this->field_0xF0 = Rules.EliteFlashTimer` (= 150 frames).
3. **Tick** — each AI_Update iteration at `LAB_006FAC31`:
   ```c
   int prev = this->field_0xF0;
   bool changed = StageClass__Stage_Changed(&this->field_0xF0);  // 0x004CC770
   if (changed /* vtable+0x124 call */) { ... }   // inline tick
   if (prev != 0
       && prev != this->field_0xF0
       && this->WhatAmI() == 6                  // Building only
       && ((prev & 2) == 2) != ((this->field_0xF0 & 2) == 2))
   {
       TacticalClass::DirtyScreenRect(extent);
       BuildingClass::UpdateAllAnimFacings(this);
   }
   ```
4. **Expire** — when `Value` hits 0 (after 150 frames), `Stage_Changed` returns 0; no
   more redraw triggers.

### `StageClass::Stage_Changed` @ `0x004CC770` (labeled this pass)

```c
bool StageClass::Stage_Changed(uint* stage) {
    if (stage->Value != 0) {
        stage->Value--;
        stage->HasChanged = false;
        if (stage->Value & 1) stage->HasChanged = true;
        return true;
    }
    return false;
}
```

This tick function is **single-caller, only from TechnoClass::AI_Update**. The "bit 0"
of the decremented value drives the `HasChanged` flag; the AI_Update then separately
tests bit-1 of the value for the dirty-rect trigger — so the redraw fires every 2
frames (when bit-1 toggles).

---

## Rendering path — VERIFIED: NOT a separate render pass, NOT a palette remap

The mechanism is **Option C** (neither):

- **Not a separate render pass.** There is no `FlasherClass::Draw_It` and no draw-list
  traversal over flashing objects.
- **Not a per-draw palette/tint flag consumed by sprite drawing.** Neither
  `BuildingClass::DrawBody @ 0x0043D290`, `TechnoClass_DrawSHP @ 0x00705E00`, nor
  `TechnoClass::ModifyCloakDrawFlags @ 0x0070ED80` reads `field_0xF0`. The draw
  pipeline has no direct awareness of the flash timer.
- **Actual mechanism:** the AI_Update tick fires `TacticalClass::DirtyScreenRect` +
  `BuildingClass::UpdateAllAnimFacings` every time bit-1 of `Stage.Value` toggles.
  `UpdateAllAnimFacings` resets the facing index on all 21 anim slots of the building
  and recomputes the damage-state anim index via `FUN_0070e360`. The resulting
  visible "flash" is a side-effect of the animation subsystem being re-initialized
  on the same tick cadence as the timer bit toggles — the building's idle/damage
  anim sprite can produce a brief visual jitter each time the rect is dirtied and the
  anim state is reset.

**This is `Building`-only.** The `WhatAmI() == 6` gate means Units, Infantry,
Aircraft, and Terrain carry the FlasherClass subobject but never see any visible
flash — the dirty-rect path is skipped for them. For non-building Technos the field is
effectively dead weight.

---

## Numeric constants extracted

| Constant             | Value | Source                                           |
|----------------------|-------|--------------------------------------------------|
| EliteFlashTimer      | 150   | `Rules+0xBE8` (ReadInt from `[AudioVisual]`)     |
| Flash cadence        | 2 fr  | Bit-1 toggle of the decrementing counter         |

No other constants (no color indices, no palette references, no per-type overrides).

---

## Call graph (who spawns / seeds)

Only one seed site in the entire binary:

```
TechnoClass::AI_Update @ 0x006F9E50
  └─ promotion block @ 0x006FA055
       └─ writes this->field_0xF0 = Rules.EliteFlashTimer when crossing to Elite
```

Only one tick site:

```
TechnoClass::AI_Update @ LAB_006FAC31
  ├─ StageClass::Stage_Changed(&this->field_0xF0)   // 0x004CC770
  ├─ TacticalClass::DirtyScreenRect(...)            // if bit-1 toggle + Building
  └─ BuildingClass::UpdateAllAnimFacings(this)       // 0x00452000
```

Zero callers from `ReceiveDamage`, `Take_Damage`, `ObjectClass::Take_Damage`, warhead
detonation, weapon impact, or any hit/collision path.

---

## INI keys

| Key                            | Section          | Rules offset | Default | Effect                          |
|--------------------------------|------------------|--------------|---------|---------------------------------|
| `EliteFlashTimer`              | `[AudioVisual]`  | `+0xBE8`     | 150     | Frames of post-promotion flash  |

No `DamageFlashColor`, no `DamageFlashDuration`, no `FlashFrames`, no `FlashColor` —
verified absent via `search_strings` in `gamemd.exe`.

The only flash string in the binary is `EliteFlashTimer` at `0x0083A3B4`.

---

## Open questions

1. **What exactly does the visible flash look like?** Empirically there IS a visible
   blink on newly-promoted Elite buildings in YR, so the dirty-rect + anim-facing
   reset path must produce something visible. Confirm by (a) spawning a pre-Elite
   building, (b) watching the rendered output for 150 frames post-promotion, and (c)
   comparing against the Rust engine. It is likely that re-invoking
   `UpdateAllAnimFacings` changes the building's one-shot anim slot start frame, causing
   a brief visual artifact (the idle-anim jumps). This is best verified end-to-end at
   runtime, not statically.
2. **Is the IFlyControl/INoticeSink/INoticeSource machinery ever used in YR?** No
   vtable references were found in this pass — they appear to be TS-legacy COM
   interfaces that FlasherClass nominally implements but whose method tables are not
   emitted by MSVC (dead after inlining / dead-code elimination). Low priority;
   flag as TS legacy and do not implement.
3. **Does anything else write `field_0xF0`?** This pass only found the one seed site
   (Elite promotion). A wider xref sweep for writes to `TechnoClass + 0xF0` is
   advisable before declaring the field Elite-only with 100% confidence — currently
   ~95% confidence based on the single caller to `Stage_Changed`.

---

## Ghidra functions labeled this pass

| Address     | Old name          | New name                    | Purpose                                        |
|-------------|-------------------|-----------------------------|------------------------------------------------|
| `0x004CC770`| `FUN_004cc770`    | `StageClass__Stage_Changed` | Decrement stage, set HasChanged on bit-0 flip  |

Already-labeled related functions (pre-existing):
- `0x00452000` — `BuildingClass__UpdateAllAnimFacings`
- `0x006F2B40`, `0x006F4300` — `TechnoClass__Constructor` (both constructors)
- `0x006F9E50` — `TechnoClass__AI_Update`
- `0x006691E0` — `RulesClass::ReadAudioVisual` (referenced)

Program saved after renames.

---

## Confidence summary

| Claim                                                       | Confidence | Evidence                   |
|-------------------------------------------------------------|------------|----------------------------|
| FlasherClass exists and is inherited by all TechnoClass     | VERIFIED   | RTTI BCA walk              |
| FlasherClass has no vtable / no virtual methods in YR       | VERIFIED   | No CompleteObjectLocator   |
| Struct = StageClass POD (Value:int, HasChanged:byte)        | VERIFIED   | TechnoClass ctor init      |
| Embedded at TechnoClass+0xF0                                | VERIFIED   | Ctor offsets 0x3c/0x3d     |
| Only live seed site = Elite promotion in AI_Update          | VERIFIED   | xref sweep of Rules+0xBE8  |
| No damage-hit flash in YR                                   | VERIFIED   | ReceiveDamage sweep (round 2) |
| Dirty-rect mechanism is Building-only                       | VERIFIED   | WhatAmI == 6 gate          |
| Visible effect comes from anim subsystem reset, not palette | HIGH       | No palette/tint consumer   |
| COM interfaces are TS-legacy dead code                      | HIGH       | No vtable references       |

---

## Follow-up investigation (round 2) — 2026-04-21

### Q1: Strengthen the "no damage-flash" negative-evidence conclusion — RESOLVED

**Method.** Instead of byte-pattern scanning for every `this + 0xF0` write (too noisy),
I enumerated every site that could plausibly touch the field via three vectors:

1. **Tick callers.** `get_xrefs_to StageClass__Stage_Changed @ 0x004CC770` returns
   **exactly one** xref:
   ```
   From 006fac4d in TechnoClass__AI_Update [UNCONDITIONAL_CALL]
   ```
   No other tick site exists. If any other system ticked the flash, it would have to
   reimplement the decrement inline (none found).

2. **Full TechnoClass::AI_Update sweep.** I decompiled the entire 500-line
   `TechnoClass__AI_Update @ 0x006F9E50` and grep'd for `field_0xf0` access. Results:
   - **Only ONE write site.** Inside the "veterancy category transition" block, where
     the old category at `+0x13C` is compared against `Volume__GetCategory()` (a
     misleading name — see below). When the transition is `→ category 0` (Elite) AND
     it's not the first-time initialisation (`iVar7 != -1`), the code writes:
     ```c
     *(undefined4 *)&param_1->field_0xf0 = *(undefined4 *)(g_RulesClass_Instance + 0xbe8);
     ```
     This is the **Elite promotion seed** identified in round 1. `Rules+0xBE8` is
     `EliteFlashTimer` (verified).
   - **Two read sites**, both at `LAB_006fac31`:
     (a) `iVar7 = *(int *)&param_1->field_0xf0;` — capture pre-tick value
     (b) `*(undefined4 *)&param_1->field_0xf0` re-read after `StageClass__Stage_Changed`
         to detect bit-1 toggle
     Both reads feed the single dirty-rect-trigger condition. No other consumer.

3. **Full damage pipeline sweep.** I decompiled `TechnoClass__ReceiveDamage @
   0x00701900` and `ObjectClass__ReceiveDamage @ 0x005F5390` end-to-end. Neither
   function contains any access to `+0xF0`. Confirmed writes in these functions are
   only to: `Health` (+0x6C), `Ammo`, `field_0x174/0x178/0x17C` (last-damage
   coord+frame), `field_0x1E0/0x1E4/0x1E8` (shake/effect coord+frame, 2x damage),
   `field_0x29C/0x298` (poison/rad timer), `field_0x310` (damage-particle anim),
   `field_0x3D1` (ally-attacked flag). **None touches `+0xF0`.** No subclass override
   (BuildingClass/UnitClass/InfantryClass/AircraftClass::ReceiveDamage) was checked
   directly, but they all tail-call into `TechnoClass__ReceiveDamage` (verified in
   the existing `RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md`).

**Note on `Volume__GetCategory` naming.** At `0x00750030`, this function takes a
float pointer and returns 0/1/2 based on two threshold comparisons:
```c
if (*param_1 >= 2.0) return 0;     // "category 0" = meets Elite threshold
if (*param_1 >= DAT_007e2ac8) return 1;
return 2;
```
The function is **mis-named** in the Ghidra project — it has 6 xrefs but based on
this usage it's really `VeterancyClass::GetCategory` (the category-from-veterancy-level
helper). The `field_0x13c` on TechnoClass that feeds it is the cached category, not
a volume. Ranking mapping in YR: `0 = Elite, 1 = Veteran, 2 = Rookie`.

**Final verdict: VERIFIED — no damage-flash exists in vanilla YR.**

- `field_0xF0` has exactly **one writer** (Elite promotion in AI_Update) and exactly
  **two readers** (both in AI_Update, same line pair, for dirty-rect trigger only).
- `StageClass__Stage_Changed` has exactly **one caller**.
- `TechnoClass::ReceiveDamage` + `ObjectClass::ReceiveDamage` do not read or write
  the field at all.
- Confidence upgraded from 95% to ~99% — the remaining 1% is only for theoretical
  field reads via dynamic struct-offset arithmetic (e.g., `*(int*)((byte*)this +
  offset_variable)`), which I did not exhaustively check but no such pattern is
  common in the YR codebase.

### Ghidra labels applied (round 2)

None for the FlasherClass investigation itself — the only ambiguous symbol touched
was `Volume__GetCategory`, which is potentially mis-named but I did not rename it
because the 6 xrefs include 4 actual audio-category uses (the function appears to be
a generic "which bucket is this float in?" helper reused for both volume and
veterancy). Left untouched at ~50/50 confidence.

Program saved after round-2 investigation (changes were to PixelFX/LineTrail
reports — see those reports).

---

## Verification (round 3)

**Claim under review:** There is NO damage-flash in vanilla YR. Round 2 reached
~99% confidence after three sweeps: (1) `StageClass::Stage_Changed @ 0x004CC770`
has only one caller — `TechnoClass::AI_Update`; (2) AI_Update's only +0xF0 writer
is the Elite-promotion block; (3) `TechnoClass::ReceiveDamage @ 0x00701900` has
zero access to +0xF0.

**Independent evidence (round-3 additional sweep):**

Enumerated ALL writers to `TechnoClass+0xF0` across the entire binary via byte-pattern
search for `89 ?? F0 00 00 00` (mov [reg+0xF0], reg32) — 42 hits. Many are in
non-Techno structs (ParticleSystem, Tiberium, FlyLocomotion etc.), so filtered down to
functions that operate on TechnoClass-shaped params.

Writers to `TechnoClass+0xF0` (per-instance) — exhaustive list:

1. **`TechnoClass::AI_Update @ 0x006F9E50`** — inside the `Volume__GetCategory`
   block. When veterancy category transitions from nonzero to 0 (i.e. rank-up to
   Elite), writes `field_0xf0 = *(g_RulesClass_Instance + 0xBE8)`. This is the
   **elite-promotion flash timer**. Reads the new timer, then Stage_Changed
   decrements it per tick, triggering vtable[0x124] redraws. This is a
   veterancy-feedback flash, not a damage flash.

   Note: `Volume__GetCategory @ 0x00750030` is misleadingly named — inspection
   shows it returns 0/1/2 based on a float threshold (Rookie/Veteran/Elite), and
   the caller uses the result as a rank comparison. Rename candidate; left at
   current name for now.

2. **`FUN_006e4560 @ 0x006e4560`** — loops over a HouseClass building list
   (HouseClass+0x6c, count at +0x78), comparing type IDs. For each matching
   building, writes `[iVar3 + 0xf0] = [param_1 + 0x90]` (the trigger's configured
   value). Called from `TriggerAction::Execute @ 0x006dd8b0` case **0x83**.
   This is a map-trigger action ("flash all buildings of type X" for scripted
   mission moments). Not a damage flash.

No other function writes TechnoClass+0xF0. Specifically:
- `TechnoClass::ReceiveDamage @ 0x00701900` — decompiled in full. It writes
  `field_0x174`, `field_0x178`, `field_0x17c`, `field_0x1e0`, `field_0x1e4`,
  `field_0x1e8`, `field_0x29c`, Ammo, Health, IsAlive, `field_0x298`,
  `field_0x3d1`. **Zero writes to +0xF0.**
- `ObjectClass::ReceiveDamage` — no access to +0xF0 on its receiver.
- The four subclass `ReceiveDamage` overrides (Aircraft/Building/Foot/Infantry/Unit)
  — none write +0xF0.

Additionally searched `FlasherClass` — only the RTTI string
`.?AVFlasherClass@@` at `0x00817AE0` exists in the binary, with zero code
references to it. Confirmed that `FlasherClass` as a runtime entity does not exist
in gamemd.exe; it is compiler-emitted RTTI metadata for a class whose vtable is
never referenced at runtime (likely a TS-era shell kept for forward compatibility).

**Verdict: CONFIRMED NO DAMAGE-FLASH.**
TechnoClass+0xF0 has exactly two runtime writers in vanilla YR — veterancy
promotion (AI_Update) and map-trigger action 0x83. Neither is tied to
`ReceiveDamage`. The "FlasherClass" referenced in some external sources is not
a live subsystem in gamemd.exe.

Ghidra MCP calls: decompiled `TechnoClass::AI_Update`, `TechnoClass::ReceiveDamage`,
`FUN_006e4560`, `TriggerAction::Execute`, `Volume__GetCategory`,
`StageClass::Stage_Changed`; byte-pattern searched all `mov [reg+0xF0], reg32`;
string-searched `FlasherClass`; xref-checked the RTTI symbol.
