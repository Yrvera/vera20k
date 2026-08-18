# frontier-capture — CaptureManagerClass (mind-control / capture) — Service Profile

**Slug:** `frontier-capture`
**Status:** promoted from catalog stub (`_frontier.md` §G2) to full profile.
**Authority order:** binary → Ghidra → docs. Image base `0x00400000`.

> **Ghidra session note (honesty):** The live Ghidra MCP bridge was **offline this
> session** — `list_instances` returned 0 instances and `connect_instance` was refused on
> both UDS and the TCP fallback (`127.0.0.1:8089`, WinError 10061), retried twice. I could
> NOT re-decompile live. Every address below is therefore **located / corroborated via the
> existing verified research corpus**, not freshly re-decompiled this session. The
> representative address is cross-corroborated by **5+ independent `[ghidra/verified]`
> docs** (see §Re-verification). Where a fact rests on a single doc it is flagged. Anything
> needing a *fresh* live decompile is marked **NEEDS-LIVE-RECHECK**.

---

## 1. Purpose (one line)

Per-controller manager that owns the reversible mind-control / capture links (Yuri Clone,
Yuri Prime, Psychic Tower, Mastermind, Genetic Mutator, etc.): it captures a victim by
transferring its ownership, tracks every victim in a node list, draws the link lines, runs
the Mastermind overload-damage timer, and restores every victim to its original owner when
the controller dies, transports, or is chrono-warped.

It is **not** the permanent Psychic-Dominator path (that bypasses CaptureManager entirely —
see §6), nor the slave/spawn families (`SlaveManagerClass` / `SpawnManagerClass`), which are
sibling object-AI satellites on the same TechnoClass but are distinct services.

---

## 2. Representative function (re-verified)

**`CaptureManagerClass::CaptureUnit @ 0x00471D40`** — the main capture-execution entry. The
stub's claimed address is **CONFIRMED** (located via corpus, not freshly re-decompiled —
see session note). Signature and flow per `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` §5.3:

```
bool __thiscall CaptureManagerClass::CaptureUnit(this, TechnoClass* target)
  1. Validate target (null + AbstractFlags)
  2. CanCapture(target) @ 0x00471C90 — return false if denied
  3. Override mode (max_control == 1): FreeUnit() every existing node first
  4. owner = target->GetHouse()                         (vtable +0x3C)
  5. target->SetOwner(controller_owner)                 (vtable +0x3D4)   ← ownership transfer
  6. alloc MCNode(0x14): victim, original_owner, capture_frame = g_CurrentFrameCounter,
       link_visible_frames = Rules->MindControlAttackLineFrames
  7. push node into the DynVector at this+0x28
  8. target->MindControlledBy = controller       (victim TechnoClass +0x2C0)
  9. skip scatter for missions 0x10/0x12/0x13; else target->Scatter() (vtable +0x3D0)
 10. DecideUnitFate(target) @ 0x004723B0        (AI disposition)
 11. create Rules->ControlledAnimationType ring anim, store at victim +0x2C8
 12. return true
```

---

## 3. What it owns

### 3.1 Globals / structs

| Address | Symbol | Role |
|---|---|---|
| `0x0089E0F0` | `g_AllCaptureManagers` (DynamicVector\<CaptureManagerClass\*\>) | global registry of every live CaptureManagerClass instance (single-doc, **NEEDS-LIVE-RECHECK** for save/load order use) |
| `0x007E4B40` | `vtable__CaptureManagerClass` (primary) | RTTI ClassID `0x42`, GetSize `0x50` |
| `0x007E4BA4` | `PTR_FUN_007E4BA4` | DynamicVector vtable for the node storage |
| `0x00704E40` | (3D line draw helper) | renders one MC link curve (shared with target-lines render) |
| `0x00424B50` | (anim attach helper) | links the MC ring anim to the victim |

### 3.2 Instance state — `CaptureManagerClass` (size **0x50 / 80 bytes**, ClassID `0x42`)

| Off | Field | Notes |
|---|---|---|
| 0x00–0x0C | 4 vtables | primary + 3 secondary (INoticeSink / IRTTITypeInfo) |
| 0x10–0x23 | AbstractClass base | inherited |
| 0x24 | DynVector vtable | node storage header |
| 0x28 | `nodes_data` | array of `MCNode*` |
| 0x2C | `nodes_capacity` | |
| 0x30 | `nodes_is_valid` | byte |
| 0x34 | `nodes_count` | current victim count |
| 0x38 | `nodes_grow_step` | default 10 |
| 0x3C | `max_control` | from controlling weapon's `Damage` |
| 0x40 | `infinite_mind_control` | from weapon `InfiniteMindControl` (Mastermind mode) |
| 0x41 | `overload_spark_active` | byte |
| 0x44 | `overload_spark_delay` | spark-visual cooldown |
| 0x48 | `owner` | owning controller TechnoClass\* |
| 0x4C | `overload_tick_timer` | countdown to next overload damage tick |

### 3.3 `MCNode` sub-struct (size **0x14 / 20 bytes**, one per controlled victim)

| Off | Field | Notes |
|---|---|---|
| 0x00 | `victim` | controlled TechnoClass\* |
| 0x04 | `original_owner` | HouseClass\* to restore on free |
| 0x08 | `capture_frame` | frame captured; **-1 = permanent link line** |
| 0x0C | (reserved) | from uninitialized reg |
| 0x10 | `link_visible_frames` | from `Rules->MindControlAttackLineFrames` |

### 3.4 TechnoClass fields it reads/writes (owned by `techno-foot`, mutated here)

| Off | Field | Who sets |
|---|---|---|
| +0x2BC | `CaptureManager` (ptr to this) | created in `TechnoClass::Init_Managers @ 0x006F3F40` if primary weapon's warhead has `MindControl=yes` |
| +0x2C0 | `MindControlledBy` (controller ptr, on victim) | CaptureUnit sets / FreeUnit clears |
| +0x2C4 | `PermanentlyMindControlled` (byte) | **NOT this service** — Psychic Dominator only (§6) |
| +0x2C8 | `MindControlAnim` (AnimClass\*) | CaptureUnit attaches / FreeUnit removes |

---

## 4. Key functions + globals (re-verified via corpus)

| Address | Name | Role |
|---|---|---|
| `0x004717D0` | `Constructor (full)` | `(owner, maxControl, infiniteMC)` → fields 0x48/0x3C/0x40 |
| `0x00471890` | `Constructor (default)` | save/load deserialization ctor |
| `0x00471A50` | `Update` | per-tick overload damage + sparks — **active only if `infinite_mind_control`** |
| `0x00471C90` | `CanCapture` | gate (see §5.1) |
| `0x00471D40` | **`CaptureUnit`** | **representative fn** — capture execution |
| `0x00471FF0` | `FreeUnit` | release one victim (restore owner, remove anim, clear 0x2C0) |
| `0x00472140` | `FreeAll` | reverse loop → FreeUnit each (controller death/transport/warp) |
| `0x00472160` | `DrawLinks` | render MC link curves |
| `0x004722F0` | `GetOriginalOwner` | lookup a victim's stored original house |
| `0x00472330` | `SetOriginalOwner` | update on house reassignment |
| `0x004723B0` | `DecideUnitFate` | AI disposition of captured/freed unit (see §TS-legacy) |
| `0x00472640` | `ShouldDrawLinks` | render gate (selection / link-timer) |
| `0x00472720` | `Save` / `0x004728E0` `Load` | serialization |
| `0x004729A0` | `GetSize` → `0x50` / `0x004729B0` `GetClassID` → `0x42` | |
| `0x004729C0` | `Destructor` | |
| `0x006F3F40` | `TechnoClass::Init_Managers` | **creates** the CaptureManager (incoming, §IN) |
| `0x007105E0` | `TechnoClass::IsMindControlled` | `(+0x2C0 != 0) \|\| (+0x2C4 != 0)` |
| `0x00710460` | `TechnoClass::FreeAllMindControlCaptures` | wrapper → `FreeAll` |
| `0x004690B0` | `WarheadTypeClass::Detonate` | MC dispatch at `0x00469211`; calls CaptureUnit at `0x004692D0` (incoming, §IN) |

---

## 5. Tick / render / load plug points

### 5.1 Per-tick — spine **rung T** (the universal per-object AI fan-out)

The CaptureManager has **no entry on the LogicClass live vector of its own**. It is ticked
**indirectly**, *inside* the controlling techno's AI:

- Spine rung **T** (`#20`, `0x005F3E70` `ObjectClass::AI` fan-out, vt+0x5c) →
  FootClass `0x004DA530` → `TechnoClass::AI_Update @ 0x006F9E50` → (if `+0x2BC` non-null)
  `CaptureManagerClass::Update @ 0x00471A50`.
- `Update` does real work **only when `infinite_mind_control` (off +0x40) is set** — i.e.
  the **Mastermind overload-damage** system. For ordinary single/limited MC controllers it
  is effectively a no-op tick.

> **Rung-letter correction vs the stub:** `_frontier.md` §G2 said "rung N (object pass)".
> The canonical spine (`LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` §2, 28-rung A–AB ladder)
> labels the MAIN object-vector tick as **rung T (#20)**, driver `0x005F3E70`. CaptureManager
> rides that fan-out via TechnoClass::AI_Update. Cite **rung T**, not N.

**Lockstep note:** `Update`'s overload damage uses `ReceiveDamage` (deterministic, no RNG of
its own per the report). The capture *itself* (CaptureUnit) runs inside warhead Detonate on
the bullet-impact tick, which is also inside rung T. CaptureUnit's only randomness is the
victim `Scatter()` and `DecideUnitFate` AI roll — both downstream of rung T's RNG fan-out.

### 5.2 Render — `TacticalClass::Draw` link pass

`CaptureManagerClass::DrawLinks @ 0x00472160` is called from **`TacticalClass::Draw`** at
callsite **`0x006D47BF`** (per `MIND_CONTROL_GHIDRA_REPORT.md` §DrawLinks and
`MIND_CONTROL_LINK_LINES_DRAWLINKS_GHIDRA_REPORT.md`), gated by `ShouldDrawLinks @ 0x00472640`
(victim/controller selected, OR link timer `MindControlAttackLineFrames` still running, OR
`capture_frame == -1` permanent). Color from `controller->House + 0x56F9`; 32-segment curved
line with a `timeGetTime()` scroll phase. This is the same TechnoClass render loop as
target/action lines but a **separate** draw step. Render entry maps under
`frontier-render-tactical` (`TacticalClass_Draw @ 0x006D3D10`).

### 5.3 Load-time / save

`Save @ 0x00472720` / `Load @ 0x004728E0` and the global registry `0x0089E0F0` participate
in `frontier-saveload`'s per-class walk + pointer swizzle (the `original_owner` / `victim`
MCNode pointers and the `owner` back-ref are swizzle targets). **NEEDS-LIVE-RECHECK** for the
exact save section ordering.

---

## 6. Active-in-YR + TS-legacy

- **Active in YR:** YES — core stock mechanic. Yuri Clone / Yuri Prime / Psychic Tower /
  Mastermind / Genetic Mutator / Magnetron-via-warhead all route through CaptureManager.
  Fires in essentially every match involving the Yuri faction. The Mastermind overload-damage
  tiers are stock-live (`OverloadCount=3,6,10,50` / `OverloadDamage=0,50,100,500` /
  `OverloadFrames=30,60,60,60` in `rulesmd.ini`).
- **NOT this service (separate path):** the **Psychic Dominator** superweapon does a
  **permanent** ownership transfer via `PsychicDominator::MindControlArea @ 0x0053B080` →
  `SetOwner(house, permanent=1)`, sets `+0x2C4`, uses `PermaControlledAnimationType`, and
  stores **no** CaptureManager link. If the dominated unit already had a CaptureManager link,
  the PD calls `FreeUnit` first to detach it cleanly. The PD path belongs to `frontier-super`,
  not here. Do not conflate `+0x2C4` (permanent) with the CaptureManager link `+0x2C0`.
- **TS-legacy caution:** `DecideUnitFate @ 0x004723B0` carries elaborate AI probability
  tables (`Rules+0xE4C/0xE68/0xE84/0xEA0`, debug string `"AICapture: I think, %s, so I roll
  %d => %s"`). The function **is** called in YR, but the report flags the probability tables
  as possibly vestigial TS inheritance — **verify the outcome distribution against live YR**
  before porting the AI branch. The core capture/free/link/overload mechanics are confirmed
  live, not TS-legacy.

---

## 7. Outgoing edges (this service depends on / calls into)

| To service | Via symbol / mechanism | Evidence |
|---|---|---|
| `techno-foot` | victim/controller are TechnoClass\*; `SetOwner` (vt +0x3D4), `GetHouse` (vt +0x3C), `Scatter` (vt +0x3D0), `ReceiveDamage`; reads/writes techno fields +0x2BC/+0x2C0/+0x2C8 | CaptureUnit §5.3 steps 4–11; Update overload `ReceiveDamage` |
| `factory-house` | `SetOwner` re-homes the victim into the controller's **HouseClass**; original-owner restore on free; MC link color from `House+0x56F9` | CaptureUnit step 5, FreeUnit step 4, DrawLinks color read |
| `mission-radio` | post-capture `DecideUnitFate` issues `SetMission(Guard 0xF)` / Hunt / join-team; capture skips scatter for missions 0x10/0x12/0x13 | DecideUnitFate §5.8; CaptureUnit step 9 |
| `damage-helpers` | Mastermind overload `Update` applies tiered damage to the **controller** via `ReceiveDamage` using `Rules+0xFA8` warhead | Update §5.6 |
| `rules-class` | reads `MindControlAttackLineFrames`(+0x310), `ControlledAnimationType`(+0x320), `OverloadCount/Damage/Frames`(+0xEE8/0xF04/0xF20), `YuriMindControlSound`/`MindClearedSound`/`MasterMindOverloadDeathSound` | §2.4/2.5 INI offsets |
| `frontier-anim` | creates the MC ring `AnimClass` (`ControlledAnimationType`) on capture, attaches at victim +0x2C8, removes on free | CaptureUnit step 11 / FreeUnit step 2 |
| `frontier-render-tactical` | `DrawLinks` runs inside `TacticalClass::Draw` (callsite `0x006D47BF`) via shared 3D-line helper `0x00704E40` | §5.2 |
| `frontier-audio-voc` | plays `YuriMindControlSound` on capture, `MindClearedSound` (per-type TechnoType+0x5B0 else global Rules+0x264) on free, `MasterMindOverloadDeathSound` on overload death | CaptureUnit step 8 flow / FreeUnit step 3 / Update |
| `random-scenario` | victim `Scatter()` + `DecideUnitFate` AI roll (1–100) consume the synchronized RNG inside rung T | CaptureUnit steps 9–10; DecideUnitFate |
| `frontier-saveload` | `Save`/`Load` + MCNode pointer swizzle (victim/original_owner/owner) | §5.3 (**NEEDS-LIVE-RECHECK**) |

## 8. Incoming edges (who drives / calls this service)

| From service | Via symbol / mechanism | Evidence |
|---|---|---|
| `techno-foot` | `TechnoClass::Init_Managers @ 0x006F3F40` allocates the CaptureManager when the primary weapon's warhead has `MindControl=yes`; `TechnoClass::AI_Update @ 0x006F9E50` calls `Update` each tick; `IsMindControlled`/`FreeAllMindControlCaptures` wrappers | §3.4, §4, §5.1 |
| `damage-helpers` | `WarheadTypeClass::Detonate @ 0x004690B0` dispatches the MC path at `0x00469211`, calling `CaptureUnit` at `0x004692D0` — the actual capture trigger | §6 capture flow steps 2–6 |
| `logicclass` | spine rung **T** (`0x005F3E70` object fan-out) is what reaches `TechnoClass::AI_Update` → `Update`; the death/free callers (`ReceiveDamage`, transport, temporal warp) also fire inside rung T | spine §2 rung T; FreeAll callers §5.5 |
| `frontier-render-tactical` | `TacticalClass::Draw` calls `DrawLinks` (gated by `ShouldDrawLinks`) at `0x006D47BF` | §5.2 |
| `frontier-super` | `PsychicDominator::MindControlArea @ 0x0053B080` calls `FreeUnit` to detach an existing CaptureManager link before its permanent capture | §6 / §10 |
| Free/cleanup callers (all `techno-foot`-owned) | `FreeAll` from controller death (`TechnoClass::ReceiveDamage @ 0x00702112`, `BuildingClass::ReceiveDamage @ 0x004424F9`), enter-transport (`0x0070FDBD`), chrono-warp (`TemporalClass::InitiateWarp @ 0x0071AF48`); `FreeUnit` from victim `Mission_Enter` (`0x0051A2DA`/`0x0073A2CD` …) | §5.5 / §7 |

---

## 9. Cross-service interaction gates (CanCapture @ 0x00471C90)

Capture is **blocked** if (any): target null; same owner; `ImmuneToPsionics`
(TechnoType+0xD35); target temporal-warping (+0x2E4) and infantry; already mind-controlled
(`IsMindControlled` — either link); **Iron Curtain / Force Shield** active (+0x2CC timer
non-zero → ties `damage-helpers`/IronCurtain); capacity full (unless `infinite_mind_control`
or override `max_control==1`); target mission 0x12/0x13 (selling). These are the parity-load-
bearing edge conditions — see `IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md` and
`MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` §5.2/§10.

---

## 10. Re-verification (this session)

- **Live Ghidra:** unavailable (see header session note). No fresh decompile performed.
- **Representative address `0x00471D40` (CaptureUnit):** **CONFIRMED via corpus** — appears
  as `[ghidra/verified]` in `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` §5.3/§11,
  `MIND_CONTROL_GHIDRA_REPORT.md`, `TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md`,
  `SELECTION_LIFECYCLE_GHIDRA_REPORT.md`, and `GI_GHIDRA_REPORT.md` §P3.3 — 5 independent
  docs agree on address + role. Stub claim accurate.
- **Other key addresses** (Constructor, CanCapture, FreeUnit, FreeAll, DrawLinks, Update,
  DecideUnitFate, struct offsets): drawn from the `MIND_CONTROL_SYSTEM` report's
  confidence-summary "Verified" rows + the DrawLinks-specific report. Offset corrections
  dated 2026-05-29 for the Rules overload-table offsets are already folded in.
- **Stub plug-point rung:** **corrected** from "rung N" to **rung T (#20)** per the canonical
  28-rung spine spec.
- **NEEDS-LIVE-RECHECK** items (flag for next Ghidra-up session): global registry
  `0x0089E0F0` save/load usage; exact save section ordering; whether `DecideUnitFate`'s AI
  probability tables fire with stock YR distributions (TS-vestigial suspicion).

## 11. Source docs

- `docs/research/MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` (primary, high-confidence)
- `docs/research/MIND_CONTROL_GHIDRA_REPORT.md`
- `docs/research/MIND_CONTROL_LINK_LINES_DRAWLINKS_GHIDRA_REPORT.md`
- `docs/research/PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md` (permanent-MC contrast)
- `docs/research/TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md` / `TECHNOCLASS_SYSTEMS_GHIDRA_REPORT.md`
- `docs/research/IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md` (CanCapture gate)
- `docs/research/SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md` / `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` (sibling satellites)
- `docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` (rung T plug point)
- `docs/research/core-services-map/_frontier.md` §G2 (seed stub)
