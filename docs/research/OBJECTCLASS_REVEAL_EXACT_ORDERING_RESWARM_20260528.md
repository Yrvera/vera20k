# ObjectClass Reveal Exact Ordering - Reswarm Report

**Address(es):** `ObjectClass::Reveal @ 0x005F4EC0`, `ObjectClass::Mark @ 0x005F5850`, `ObjectClass::Set_Raw_Coords @ 0x005F6940`, `DisplayClass::Submit_Object @ 0x004A9720`, `FUN_0055BAA0 @ 0x0055BAA0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact ordering inside `ObjectClass::Reveal @ 0x005F4EC0`: entry gates, `CanEnter` gate placement, `InLimbo` and redraw writes, type coordinate transform, raw coordinate write, `Mark(MARK_PUT)` success/failure behavior, display submit, logic registration, alpha shape, and line trail side effects.
**Non-Scope:** mapping every caller, decompiling every derived `CanEnter` override, fully naming all `g_GameMode` enum values, or proving every object type's default `ObjectTypeClass+0x234`.
**Confidence:** High for the ordered function body and cited callees; Medium for broad Rust deltas because this slot only scanned the relevant lifecycle surfaces statically.
**Active in YR:** Yes for ordinary gameplay reveal/unlimbo paths; conditional for map-editor and nonstandard game-mode bypasses noted below.

## 0. Working Notes

**Target question:** What exact branch and side-effect order does active YR `ObjectClass::Reveal @ 0x005F4EC0` use when moving an object from limbo onto the map?

**Non-goals:** Do not classify every caller, do not audit every derived `CanEnter` implementation, and do not re-prove the already-settled `FUN_0055BAA0` tail-append helper.

**Evidence needed to mark COMPLETE:** decompile plus assembly for `0x005F4EC0`, decompile/assembly for `Mark` and `Set_Raw_Coords`, callee evidence for display/alpha/line-trail effects, and Rust-surface scan for reveal/register handoff implications.

**Stop conditions:** Stop after the exact `Reveal` body order is drained and any wider caller/type/default questions are logged as out-of-scope or remaining uncertainty.

## 1. Overview

`ObjectClass::Reveal` is the base "enter map from limbo" primitive. It returns `0` without side effects for invalid origin, inactive game, not-in-limbo, already-marked, or blocked normal-game cells; otherwise it clears `InLimbo`, clears `NeedsRedraw`, transforms coordinates through the type, writes raw coords, and only then asks `Mark(MARK_PUT)` to attach the object to map/display state.

The important ordering correction is that logic registration through `FUN_0055BAA0` is not before display submission and is not outside the alive branch. It happens only after `Mark(MARK_PUT)` succeeds, only when `Object+0x90 IsAlive` is set, after optional `DisplayClass::Submit_Object`, before alpha-shape allocation, and under `ObjectTypeClass+0x234` plus object/game-mode gates.

## 2. Key Offsets / Fields

| Offset / address | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00AC1380..88` | all-zero coordinate sentinel compared against input `x/y/z` before any object write | `0x005F4ECD..0x005F4EEF`; `read_memory 0x00AC1380 length 12 -> 000000000000000000000000` | Yes |
| `g_GameActive @ 0x00A8E9A0` | must be nonzero before reveal proceeds | `0x005F4EF5..0x005F4EFC` | Yes |
| `Object+0x81` | `InLimbo`; must be nonzero at entry; cleared before coordinate writes; restored only on mark failure | `0x005F4F02..0x005F4F0A`, `0x005F4F4E`, `0x005F521C` | Yes |
| `Object+0x74` | `IsMarked`; reveal refuses already-marked objects before `CanEnter` or coordinate writes | `0x005F4F10..0x005F4F15`; `ObjectClass::Mark @ 0x005F58EC..0x005F58FB` sets it on PUT | Yes |
| `Object+0x80` | `NeedsRedraw`; cleared immediately after `InLimbo` is cleared, before `GetType`/coordinate transform | `0x005F4F55`; `ObjectClass::Mark @ 0x005F586B..0x005F587E` reads it for MARK_CHANGE | Yes |
| `Object+0x90` | `IsAlive`; gates display submission, logic registration, alpha shape, and line trail | `0x005F4FC2..0x005F4FCA`, branch to `0x005F5210` | Yes |
| `Object+0x94` | display-layer membership id used by `DisplayClass::Submit_Object` | `DisplayClass::Submit_Object @ 0x004A9720` reads/writes `param_1[0x25]` | Yes |
| `Object+0x98` | LogicClass membership byte, set by `FUN_0055BAA0` on successful insert | `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`; call at `0x005F5038..0x005F5040` | Yes |
| `Object+0x9C/+0xA0/+0xA4` | raw coordinates written by `Set_Raw_Coords` | `ObjectClass::Set_Raw_Coords @ 0x005F6940..0x005F6957` | Yes |
| `Object+0xA8` | line-trail pointer written after line-trail allocation | `0x005F517F..0x005F5187`; `0x005F5207..0x005F520D` | Conditional on `ObjectType+0x23A` |
| `ObjectType+0xAC` | `AlphaImage` SHP pointer; gates alpha-shape allocation | `0x005F5045..0x005F5053`; `ALPHA_SHAPE_CLASS_LIFECYCLE.md` | Conditional on type |
| `ObjectType+0x234` | logic-registration eligibility gate | `0x005F4FEF..0x005F4FF7` | Conditional on type |
| `ObjectType+0x23A` | `UseLineTrail` gate | `0x005F5155..0x005F515D`; unit docs cite `UseLineTrail=` parser | Conditional on type |
| `ObjectType+0x23B..0x23D` | fallback line-trail RGB color | `0x005F51BA..0x005F51DD` | Conditional on line trail and zero global override |
| `ObjectType+0x240` | line-trail color decrement | `0x005F51EB..0x005F5202` | Conditional on line trail |
| `Rules+0x1863..0x1865` | global line-trail RGB override, used when any component is nonzero | `0x005F518D..0x005F51E8` | Conditional |

## 3. Exact `ObjectClass::Reveal` Order

### 3.1 Entry gates before any object mutation

Order is exact from assembly `0x005F4ECD..0x005F4F4A`.

1. Compare input coord triple against `DAT_00AC1380/84/88`; if all three match, return `0`. Active in YR: Yes. Evidence: assembly `0x005F4ECD..0x005F4EEF`; data bytes are all zero.
2. If `g_GameActive == 0`, return `0`. Active in YR: Yes. Evidence: `0x005F4EF5..0x005F4EFC`.
3. If `Object+0x81 InLimbo == 0`, return `0`. Active in YR: Yes. Evidence: `0x005F4F02..0x005F4F0A`.
4. If `Object+0x74 IsMarked != 0`, return `0`. Active in YR: Yes. Evidence: `0x005F4F10..0x005F4F15`; `Mark(PUT)` writes this byte.
5. If `g_MapEditorMode == 0`, call `CellClass::Get_Cell_At` and then `this->vtable+0x1AC` with args `(cell, -1, -1, 0, 0)`; if that returns nonzero, return `0`. Active in YR: Yes for normal gameplay, because map-editor mode is not the ordinary skirmish/campaign path. Evidence: `0x005F4F1B..0x005F4F44`.
6. If `g_MapEditorMode != 0`, skip the `CanEnter` check and proceed. Active in YR: Conditional, map editor only. Evidence: branch `0x005F4F20..0x005F4F22 -> 0x005F4F4A`.

Tiny detail: the `CanEnter` result is inverted for reveal admission. In this call shape, `0` means allowed and nonzero rejects reveal. Do not implement it as "true means allowed" without matching the native convention at this call site.

### 3.2 State writes and coordinate pipeline

After the gates pass:

1. Write `Object+0x81 InLimbo = 0`. Active in YR: Yes. Evidence: `0x005F4F4E`.
2. Write `Object+0x80 NeedsRedraw = 0`. Active in YR: Yes. Evidence: `0x005F4F55`.
3. Call `this->vtable+0x88` to fetch the object type pointer. Active in YR: Yes. Evidence: `0x005F4F5C`.
4. Copy original input coord into local stack coord. Active in YR: Yes. Evidence: `0x005F4F62..0x005F4F78`.
5. If type pointer is non-null, call `type->vtable+0x6C(out, input)` and replace the local coord with the returned coord. Active in YR: Yes, conditional on type pointer. Evidence: `0x005F4F7C..0x005F4F9B`.
6. Call `this->vtable+0x1B4` with the local coord. For the base implementation, `ObjectClass::Set_Raw_Coords @ 0x005F6940` writes `+0x9C/+0xA0/+0xA4` exactly in x, y, z order. Active in YR: Yes. Evidence: `0x005F4F9F..0x005F4FA8`; callee `0x005F6940..0x005F6957`.

Important failure consequence: if `Mark(MARK_PUT)` fails later, the raw coordinates remain written. The only revert in `Reveal` is `InLimbo = 1`.

### 3.3 `Mark(MARK_PUT)` success and failure behavior

`Reveal` calls `this->vtable+0x124(1)` after raw coords are set. Active in YR: Yes. Evidence: `0x005F4FAE..0x005F4FBC`.

If `Mark` returns zero:

1. `Reveal` writes `Object+0x81 InLimbo = 1`.
2. It returns `0`.
3. It does not restore `NeedsRedraw`, raw coords, `IsMarked`, display layer, logic registration, alpha, or line trail.

Evidence: `0x005F4FBA..0x005F4FBC` branch to `0x005F521C`, then return at `0x005F5223`. Active in YR: Yes.

For base `ObjectClass::Mark`, `MARK_PUT` refuses limboed objects, performs map/radar notifications first, then sets `Object+0x74 IsMarked = 1`, calls `vtable+0x134 MarkNeedsRedraw`, and returns `1` only if `IsMarked` was previously zero. Evidence: `ObjectClass::Mark @ 0x005F5850..0x005F5921`, especially `0x005F58EC..0x005F5904`. Active in YR: Yes. Derived classes can add cell-list/foundation effects through their override path; this report does not audit every derived override.

### 3.4 Alive branch: display, logic registration, alpha, line trail

If `Mark(PUT)` succeeds and `Object+0x90 IsAlive == 0`, `Reveal` returns `1` immediately. It does not submit display, register logic, allocate alpha shape, or allocate line trail. Active in YR: Conditional on dead-but-revealed objects; evidence `0x005F4FC2..0x005F4FCA` branch to `0x005F5210`.

If `IsAlive != 0`, the ordered side effects are:

1. Call `this->vtable+0x78` to compute display layer. If result is not `-1`, call `DisplayClass::Submit_Object`. Active in YR: Yes. Evidence: `0x005F4FD0..0x005F4FE7`.
2. `DisplayClass::Submit_Object` first removes the object from its previous layer when `Object+0x94 != -1`, recomputes layer, calls `DynamicVector__Insert` with unique flag `(layer == 2)`, and writes `Object+0x94 = layer` only on insertion success. Active in YR: Yes. Evidence: `DisplayClass::Submit_Object @ 0x004A9720`.
3. If the type pointer is null, skip logic registration and alpha shape, then still re-fetch type for line-trail test later. In standard typed objects this pointer is non-null. Active in YR: Conditional. Evidence: `0x005F4FE7..0x005F4FE9`, branch to `0x005F514B`.
4. If `ObjectType+0x234 == 0`, skip logic registration. Active in YR: Conditional per object type. Evidence: `0x005F4FEF..0x005F4FF7`.
5. If `ObjectType+0x234 != 0`, call `this->WhatAmI` (`vtable+0x2C`). If `WhatAmI != 0x24`, continue toward registration. If `WhatAmI == 0x24`, call `WhatAmI` a second time and require both `WhatAmI == 0x24` and `ObjectType+0x2B1 != 0`; otherwise skip registration. Active in YR: Conditional on type/class. Evidence: `0x005F4FF9..0x005F5019`.
6. If `g_GameMode == 0` or `g_GameMode == 5`, register without the owner/status check. Otherwise call `this+4` secondary vtable slot `+0x10` with `this+4`; if it returns `-2`, skip registration. Active in YR: Conditional by game mode. Evidence: `0x005F501B..0x005F5036`.
7. Call `FUN_0055BAA0` with `ECX=0x87F778`, object pointer, and unique flag `0`. Active in YR: Yes for eligible ordinary reveal. Evidence: assembly `0x005F5038..0x005F5040`; helper report verifies tail append and `Object+0x98`.
8. If `ObjectType+0xAC AlphaImage` is non-null, compute client coords from current object coords, convert to screen, subtract half the SHP width/height using signed half division sequence, allocate `0x40`, and construct `AlphaShapeClass`; if not map editor, dirty the screen rectangle. Active in YR: Conditional on type AlphaImage. Evidence: `0x005F5045..0x005F5146`; `AlphaShapeClass__Constructor @ 0x00420960`.
9. Re-fetch type through `vtable+0x88`, read `ObjectType+0x23A UseLineTrail`; if false, return `1`. Active in YR: Conditional on type. Evidence: `0x005F514B..0x005F515D`.
10. If `UseLineTrail` is true, allocate `0x210`, call `LineTrail__Constructor`, store pointer at `Object+0xA8`, then set color and decrement. Active in YR: Conditional on type. Evidence: `0x005F5163..0x005F520D`.
11. Line-trail color source order: if `Rules+0x1863..0x1865` are all zero, copy RGB from `ObjectType+0x23B..0x23D`; otherwise copy RGB from the Rules global. Then call `LineTrail__SetColorDecrement` with `ObjectType+0x240` and write owner pointer at line-trail `+4`. Active in YR: Conditional on line trail. Evidence: `0x005F518D..0x005F520D`; `LineTrail__SetColorDecrement @ 0x00556B50`.

Tiny detail: `LineTrail__SetColorDecrement` doubles the decrement when `g_ExtraAnimationsEnabled == 0`, then writes line-trail `+8`. Evidence: `0x00556B50` decompile. Active in YR: Conditional on the extra-animation setting.

## 4. Integration Points

| Integration point | Finding | Evidence | Active in YR |
|---|---|---|---|
| `TechnoClass::Unlimbo @ 0x006F6CA0` | calls `ObjectClass::Reveal` first and returns `0` if reveal fails; all techno-specific playfield, fog, owner, facing, sensor, deploy-fire, and falling-state work happens after reveal success | decompile `0x006F6CA0`, call/return at top | Yes |
| `BulletClassFireRevealArmAndSubmit @ 0x00468670` | calls `ObjectClass::Reveal`; if reveal returns false it returns failure before bullet arming; after reveal it later removes/submits display again as bullet-specific setup | decompile `0x00468670` | Yes |
| Constructors/unlimbo callers | static xrefs include anim, overlay, particle, smudge, terrain, voxel anim, bullet, building light, and TechnoClass paths | `get_function_xrefs 0x005F4EC0` | Yes, conditional by class/path |
| Logic registration | `ObjectClass::Reveal` reaches `FUN_0055BAA0` only through the successful, alive, type-eligible branch | `0x005F4FC2..0x005F5040` | Yes, conditional per branch |

## 5. Current Rust Implementation Status

| Rust surface | Current shape | Delta / implication |
|---|---|---|
| `src/sim/world/world_spawn.rs` | map/runtime spawn functions insert the entity then call the reveal API `self.reveal(...)` (world_spawn.rs:260, 438), which routes through `register_live_object`; the limbo spawn path inserts WITHOUT registering (world_spawn.rs:587-589, comment "Limbo objects are NOT registered… mirroring ObjectClass+0x98") | **[CORRECTED 2026-05-29 — re-confirmed by Reading `src/sim/world/world_spawn.rs:259-260,437-438,587-589`]** The earlier "call `register_live_object` directly" wording is STALE: spawns now go through the `reveal`/`unlimbo` membership API, and limbo-created objects are correctly left out of the active order until reveal. **Genuine REMAINING delta:** that reveal API is still ungated — it carries no exact `Reveal` gate/order surface (null/game-active/limbo/marked/`CanEnter`/`Mark(PUT)`-success), so an object reaching `reveal` always registers regardless of native reveal success semantics. |
| `src/sim/world/mod.rs` | `register_live_object` (mod.rs:680) tail-appends behind a per-entity `+0x98` membership guard (`in_logic_vector`); `conceal`/`unregister_live_object` (mod.rs:689) does the gated compacting remove; `live_object_order_snapshot` (mod.rs:745) returns the `LogicVector` (`self.logic`, mod.rs:319) verbatim; a debug invariant (mod.rs:724) cross-checks order length vs flagged-entity count | **[CORRECTED 2026-05-29 — re-confirmed by Reading `src/sim/world/mod.rs:680-746` + `src/sim/game_entity.rs:172`]** The earlier "appends if absent / sorted `EntityStore` fallback / lacks `+0x98`" wording is STALE: the port now has a `LogicVector` returned verbatim (no sorted fallback — see mod.rs:740 "No sorted-ID fallback (was DRIFT)"), and the object-local `+0x98` membership byte IS modelled as `in_logic_vector` (game_entity.rs:172), set at mod.rs:682 and cleared at mod.rs:694. **Genuine REMAINING delta:** `reveal`/`unlimbo` (mod.rs:703/715) delegate straight to `register_live_object` with NO native `IsAlive` (`+0x90`), `ObjectType+0x234`, or `Mark(PUT)`-success reveal-registration gate chain — registration eligibility is unconditional, not gated as in §3.4 / §8. |
| `src/sim/aircraft/drop_payload.rs` | comments indicate failed placement must not unlimbo passenger into occupancy | Directionally aligned with native failed reveal staying in limbo, but exact base `Reveal` raw-coordinate side effect and logic registration order are not represented |
| render/alpha/line trail surfaces | general alpha and render systems exist, but no scanned exact `ObjectClass::Reveal` alpha-shape/line-trail creation path tied to sim reveal | Future render handoff needs app/render-layer event after native reveal success, not before mark success |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectClass::Reveal` entry gates | verified | decompile and assembly `0x005F4EC0..0x005F4F4A`; data read `0x00AC1380` | none |
| `CanEnter` call placement and polarity | verified | assembly `0x005F4F1B..0x005F4F44` | derived `CanEnter` class semantics out-of-scope |
| `InLimbo` / `NeedsRedraw` writes | verified | `0x005F4F4E..0x005F4F55` | none |
| coordinate transform and raw coord write | verified | `0x005F4F5C..0x005F4FA8`; `Set_Raw_Coords @ 0x005F6940` | exact per-type transform bodies out-of-scope |
| `Mark(MARK_PUT)` success/failure behavior | verified | `0x005F4FAE..0x005F5223`; `Mark @ 0x005F5850` | derived Mark override cell effects out-of-scope |
| display submission placement | verified | `0x005F4FC2..0x005F4FE7`; `DisplayClass::Submit_Object @ 0x004A9720` | full display-vector semantics beyond Submit not exhausted |
| `ObjectType+0x234` registration branch | verified | `0x005F4FEF..0x005F5040`; helper report | full type default table out-of-scope |
| alpha-shape creation order | verified | `0x005F5045..0x005F5146`; `AlphaShapeClass__Constructor @ 0x00420960`; alpha lifecycle doc | draw-time alpha composition out-of-scope |
| line-trail creation order | verified | `0x005F514B..0x005F520D`; `LineTrail__Constructor`, `LineTrail__SetColorDecrement` | full line-trail update/draw out-of-scope |
| Rust reveal equivalence | touched-not-exhausted | static scan of `src/sim/world/world_spawn.rs`, `src/sim/world/mod.rs`, `src/sim/aircraft/drop_payload.rs` | implementation design/tests |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-REVEAL-001 - What is the first gate? -> input coord triple equal to the all-zero global sentinel returns 0 before object writes.` (evidence: `0x005F4ECD..0x005F4EEF`; `read_memory 0x00AC1380`)
- `[RESOLVED] OQ-REVEAL-002 - Is the game-active gate before limbo checks? -> Yes, `g_GameActive` is checked before `InLimbo` and `IsMarked`.` (evidence: `0x005F4EF5..0x005F4F0A`)
- `[RESOLVED] OQ-REVEAL-003 - Can Reveal run on non-limbo objects? -> No, `InLimbo==0` returns 0.` (evidence: `0x005F4F02..0x005F4F0A`)
- `[RESOLVED] OQ-REVEAL-004 - Does already-marked block Reveal? -> Yes, `Object+0x74 != 0` returns 0 before `CanEnter`.` (evidence: `0x005F4F10..0x005F4F15`)
- `[RESOLVED] OQ-REVEAL-005 - Where is CanEnter? -> Only when `g_MapEditorMode==0`, after entry gates and before any object write; nonzero return rejects reveal.` (evidence: `0x005F4F1B..0x005F4F44`)
- `[RESOLVED] OQ-REVEAL-006 - What state writes happen before coordinates? -> `InLimbo=0`, then `NeedsRedraw=0`.` (evidence: `0x005F4F4E..0x005F4F55`)
- `[RESOLVED] OQ-REVEAL-007 - Does type transform happen before Set_Raw_Coords? -> Yes, type vtable `+0x6C` may replace the local coord before vtable `+0x1B4`.` (evidence: `0x005F4F7C..0x005F4FA8`)
- `[RESOLVED] OQ-REVEAL-008 - What does base Set_Raw_Coords write? -> x/y/z to object `+0x9C/+0xA0/+0xA4`.` (evidence: `0x005F6940..0x005F6957`)
- `[RESOLVED] OQ-REVEAL-009 - What reverts on Mark failure? -> Only `InLimbo` is restored to 1; raw coords are not restored.` (evidence: `0x005F521C..0x005F5223`)
- `[RESOLVED] OQ-REVEAL-010 - Is logic registration before display submission? -> No, optional `DisplayClass::Submit_Object` comes first inside the alive branch.` (evidence: `0x005F4FD0..0x005F5040`)
- `[RESOLVED] OQ-REVEAL-011 - Is logic registration outside IsAlive? -> No, `IsAlive==0` skips it and returns success after Mark.` (evidence: `0x005F4FC2..0x005F4FCA`, `0x005F5210`)
- `[RESOLVED] OQ-REVEAL-012 - Is alpha shape before or after logic registration? -> After logic registration branch.` (evidence: `0x005F5038..0x005F504D`)
- `[RESOLVED] OQ-REVEAL-013 - Is line trail before alpha? -> No, line trail is last before return.` (evidence: `0x005F514B..0x005F520D`)
- `[DEFERRED] OQ-REVEAL-014 - What does every derived `CanEnter` return code mean?` (category: `out-of-scope`; reason: this slot only proves Reveal's call placement and polarity; next-step-if-pursued: investigate each class vtable `+0x1AC` owner.)
- `[DEFERRED] OQ-REVEAL-015 - Which stock object types have `ObjectType+0x234`, `+0xAC`, and `+0x23A` set?` (category: `out-of-scope`; reason: exact type default matrix is a separate parser/type-class slice; next-step-if-pursued: audit ObjectTypeClass/BulletTypeClass/AnimTypeClass constructors and `ReadINI` stores.)
- `[DEFERRED] OQ-REVEAL-016 - What exact semantic name maps to `g_GameMode` values 0, 5, and owner check `-2`?` (category: `requires-different-system-context`; reason: branch shape is verified but enum naming needs a game-mode investigation; next-step-if-pursued: trace `g_GameMode` writers and secondary vtable `+0x10` return meanings.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| `Reveal` has no side effects until null/game-active/limbo/marked/normal-game-CanEnter gates pass; failed `CanEnter` leaves `InLimbo`, `NeedsRedraw`, coords, display, and live order untouched. | `0x005F4ECD..0x005F4F44` | **[CORRECTED 2026-05-29 — Read `src/sim/world/mod.rs:703-717`, `world_spawn.rs:260,438`]** A `reveal`/`unlimbo` membership API now exists and spawns route through it (not raw `register_live_object`); but it is ungated — no null/game-active/limbo/marked/`CanEnter` gate sequence, so reveal never fails | `src/sim/world/mod.rs::reveal`; `src/sim/world/world_spawn.rs`; future `reveal_object` gate surface | Separate object construction/storage from reveal; only after the native gates pass may Rust clear limbo, set coords, mark occupancy, and register eligible logic. | Create a limbo object, force placement blocked, attempt reveal; assert object remains limbo/stored, live order unchanged, occupancy unchanged, and no reveal event emitted. | `reveal_blocked_canenter_has_no_live_side_effects` | Do not register or occupy an object before native reveal gates and `Mark(PUT)` success. |
| After gates pass, `Reveal` clears `InLimbo`, clears `NeedsRedraw`, applies type coordinate transform, writes raw coords, then calls `Mark(PUT)`; if Mark fails, only `InLimbo` is restored and raw coords remain changed. | `0x005F4F4E..0x005F5223`; `Set_Raw_Coords @ 0x005F6940` | Rust does not model this narrow partial-failure state | future sim lifecycle state, occupancy/reveal API | Preserve order and partial failure if an implementation can fail after coordinate assignment. | Arrange a test double where `Mark(PUT)` fails after raw coords; assert `InLimbo=true`, coords equal transformed requested coords, no live registration. | `reveal_mark_failure_reverts_only_limbo` | Do not roll back coordinates or redraw flags unless binary evidence shows another caller does it. |
| Logic registration through `FUN_0055BAA0` occurs after optional display submission and only in the `IsAlive` branch, with `ObjectType+0x234` and game-mode/object gates; alpha and line-trail creation happen after that. | `0x005F4FC2..0x005F520D`; helper report `0x0055BAA0` | **[CORRECTED 2026-05-29 — Read `src/sim/world/mod.rs:680-705`, `game_entity.rs:172`, `world_spawn.rs:587-589`]** The reveal/conceal membership API + `+0x98` byte (`in_logic_vector`) now exist, and limbo-created objects are NOT registered at construction (only at reveal). **Still absent:** the `Mark(PUT)`-success → `IsAlive` (`+0x90`) → `ObjectType+0x234` → game-mode/object gate chain — `reveal` registers unconditionally, with no render-side alpha/line-trail reveal effects emitted | `src/sim/world/mod.rs::reveal` / `register_live_object`; `src/sim/world/world_spawn.rs`; future render event bridge | Register only successfully marked, alive, logic-enabled objects, and emit alpha/line-trail setup after registration branch in the same reveal operation. | Reveal four objects: dead logic-enabled, alive nonlogic, alive logic with alpha, alive line-trail; assert only alive logic enters live order, alpha event follows register, line-trail follows alpha. | `reveal_registers_only_alive_logic_enabled_before_visual_trails` | Do not treat successful mark alone, non-limbo state, or stored entity existence as LogicClass membership. |

## 9. Negative Facts / Do Not Do

- Do not call `Reveal` on an already on-map object and treat it as idempotent success. Active in YR: Yes; `InLimbo==0` returns `0` at `0x005F4F02..0x005F4F0A`.
- Do not ignore the `IsMarked` gate. Active in YR: Yes; `Object+0x74 != 0` returns `0` before `CanEnter` at `0x005F4F10..0x005F4F15`.
- Do not run `CanEnter` after clearing limbo or writing coords. Active in YR: Yes; `CanEnter` is before object mutation at `0x005F4F1B..0x005F4F44`.
- Do not register dead objects into the LogicClass vector merely because `Mark(PUT)` succeeded. Active in YR: Yes; `IsAlive==0` jumps to success return before `FUN_0055BAA0` at `0x005F4FC2..0x005F4FCA`.
- Do not allocate alpha shapes or line trails before logic registration branch. Active in YR: Yes; registration call `0x005F5038..0x005F5040` precedes alpha `0x005F5045..0x005F5146` and line trail `0x005F514B..0x005F520D`.

## 10. Remaining Uncertainty

- Derived `CanEnter` bodies were not audited; this report proves only the base reveal call order, argument constants, and return polarity at the call site.
- Full stock-type matrix for `ObjectType+0x234`, `ObjectType+0xAC`, `ObjectType+0x23A`, and line-trail colors remains a parser/type-class follow-up.
- Exact semantic names for `g_GameMode == 0`, `g_GameMode == 5`, and the secondary-vtable `-2` skip remain unresolved; branch behavior and placement are verified.

## 11. Stale Docs / Follow-up Wording

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md` should replace the `ObjectClass::Reveal` summary with: "`ObjectClass::Reveal @ 0x005F4EC0` first rejects all-zero coord sentinel, inactive game, `InLimbo==0`, `IsMarked!=0`, and, in normal non-editor mode, a nonzero `CanEnter(cell,-1,-1,0,0)` result. Only then does it clear `InLimbo`, clear `NeedsRedraw`, transform coords through `ObjectTypeClass vtable+0x6C`, write raw coords, and call `Mark(MARK_PUT)`. On `Mark` failure it restores only `InLimbo=1`; raw coords remain changed. On `Mark` success with `IsAlive=1`, it optionally submits to display, then conditionally calls `FUN_0055BAA0` under `ObjectType+0x234` and game/object gates, then creates alpha shape and line trail effects. `IsAlive=0` returns success after mark without display, logic registration, alpha, or line trail."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ALPHA_SHAPE_CLASS_LIFECYCLE.md` should replace its reveal creation preconditions with: "Alpha shape creation in `ObjectClass::Reveal` occurs only after successful `Mark(MARK_PUT)`, only while `IsAlive!=0`, after optional display submission and after the logic-registration branch, and only when the type pointer is non-null and `ObjectType+0xAC AlphaImage` is non-null. The earlier `Object+0x81` check is the `InLimbo` entry gate, not a discovered-by-player test; `Object+0x74` is the already-marked gate, not a dead/alive test."

## Sources

- Ghidra decompile/read-only:
  - `ObjectClass::Reveal @ 0x005F4EC0`
  - `ObjectClass::Mark @ 0x005F5850`
  - `ObjectClass::Set_Raw_Coords @ 0x005F6940`
  - `DisplayClass::Submit_Object @ 0x004A9720`
  - `AlphaShapeClass__Constructor @ 0x00420960`
  - `LineTrail__Constructor @ 0x00556A20`
  - `LineTrail__SetColorDecrement @ 0x00556B50`
  - `TechnoClass::Unlimbo @ 0x006F6CA0`
  - `BulletClassFireRevealArmAndSubmit @ 0x00468670`
- Ghidra assembly/data:
  - `ObjectClass::Reveal @ 0x005F4EC0..0x005F522C`
  - `ObjectClass::Mark @ 0x005F5850..0x005F5921`
  - `ObjectClass::Set_Raw_Coords @ 0x005F6940..0x005F695A`
  - `read_memory 0x00AC1380 length 12`
- Prior research:
  - `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
  - `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`
  - `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`
  - `docs/research/ALPHA_SHAPE_CLASS_LIFECYCLE.md`
- Rust surfaces scanned:
  - `src/sim/world/world_spawn.rs`
  - `src/sim/world/mod.rs`
  - `src/sim/aircraft/drop_payload.rs`
