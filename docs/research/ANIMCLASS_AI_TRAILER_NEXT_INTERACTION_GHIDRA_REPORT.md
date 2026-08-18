# AnimClass::AI TrailerAnim, TrailerSeperation, and Next Interaction - Ghidra Report

**Address(es):** `AnimClass::AI @ 0x00423AC0`, trailer branch `0x004242A6..0x00424322`, `Next=` branch `0x004247F3..0x00424932`, `AnimClass::Constructor @ 0x00421EA0`, `AnimTypeClass::Constructor @ 0x00427530`, `AnimTypeClass::ReadINI @ 0x00427D00`  
**Investigation Mode:** exhaustive-slice  
**Scope:** exact active-YR runtime behavior for `TrailerAnim`, signed `TrailerSeperation`, child spawn cadence, coordinates, constructor row args, and interaction with `Next=` in-place morphing.  
**Non-Scope:** `BounceAnim`, `ExpireAnim`, bouncer impact physics, draw-order/depth, audio details outside constructor delay/Middle timing, and Rust code edits.  
**Confidence:** High for branch order, signedness, spawn args, and `Next` handoff; Medium for exhaustive stock-content liveness because this slot checked representative stock `artmd.ini` entries, not every possible parent constructor path.

## Working Notes Gate

- `Target question` - Verify whether periodic `TrailerAnim` spawns use parent or next type state, how signed `TrailerSeperation` gates cadence, and what `Next=` does to trailer-related state.
- `Non-goals` - Do not inspect `BounceAnim`/`ExpireAnim`; do not edit Rust; do not relabel or mutate Ghidra; do not broaden into draw traversal or bouncer impact.
- `Evidence needed to mark COMPLETE` - Decompile plus assembly for trailer branch and `Next` branch order; constructor evidence for row args and child field initialization; INI/default evidence for active YR keys; Rust surface scan; implementation handoff with concrete test names.
- `Stop conditions` - Stop when trailer/Next interaction is resolved, every open question is resolved or deferred, and only this report plus the shared claims file are written.

## Summary

`TrailerAnim=` is checked near the top of `AnimClass::AI`, before the first-AI guard, delay countdown, timer/frame advancement, loop exhaustion, and `Next=`. The branch reads the currently installed parent `AnimTypeClass` from `AnimClass+0xC8`, checks that parent type's `TrailerAnim` at `+0x308`, then uses that same parent type's signed `TrailerSeperation` at `+0x30C` against the global frame counter.

`Next=` is later. When the current animation reaches its boundary and loop byte reaches zero, `AnimClass::AI` writes `AnimClass+0xC8 = old_type->Next` and resets playback for the same object. It does not allocate, it does not run a second trailer check in that same AI visit, and it does not reset any per-instance trailer counter because the native trailer cadence is not per-instance. The next AI visit samples the new type's `TrailerAnim`/`TrailerSeperation`.

Active in YR: Yes/Conditional. The branches are live in `gamemd.exe`; stock YR `artmd.ini` has active examples with `TrailerAnim`/`TrailerSeperation` (`DBRIS*`, `METLARGE`, `METSMALL`) and `Next=` (`METSTRAL -> SMOKEY`). A single stock type with both `TrailerAnim` and `Next` was not found in the bounded scan, so combined same-object interaction is active for modded content and for any standard object whose current and next types independently define these keys.

## Verified Binary Findings

1. **Trailer branch order is before `Next=`.**  
   Active in YR: Yes, conditional on an active `AnimClass` reaching AI. Evidence: `AnimClass::AI` decompile shows the trailer block before `HideIfNoOre`, expired check, first-AI guard, delay countdown, timer/frame advancement, loop/end handling, and `Next=`. Assembly `0x004242A6..0x00424322` contains the trailer constructor call; `Next=` begins later around `0x004247F3` after loop-byte handling.

2. **Trailer cadence is global-frame modulo, not a parent-local countdown.**  
   Active in YR: Conditional on `TrailerAnim != null`. Evidence: assembly `0x004242CA..0x004242DF` loads `AnimType+0x30C`, compares it to `1`, then executes `MOV EAX,[0x00A8ED84]`, `CDQ`, `IDIV ECX`, `TEST EDX,EDX`. `0x00A8ED84` is the global frame counter used elsewhere in `AnimClass::AI`.

3. **`TrailerSeperation` is signed and not zero-guarded.**  
   Active in YR: Conditional. Evidence: signed `CDQ` + `IDIV ECX` at `0x004242D5..0x004242DD`; no `TEST ECX,ECX` or equivalent follows the load from `AnimType+0x30C`. The only special case is `CMP ECX,1` / `JZ 0x004242E1`. `AnimTypeClass::ReadINI` calls the int reader and stores `EAX` to `+0x30C` at `0x00428646..0x0042864B`; constructor default is zero at `AnimTypeClass::Constructor @ 0x00427530`.

4. **Trailer child constructor row is fixed by the parent branch.**  
   Active in YR: Conditional on modulo pass and allocation success. Evidence: assembly `0x004242F6..0x0042431D` pushes `reverse=0`, `zAdjust=0`, `drawFlags=0x600`, `loop=1`, coordinate out-pointer, `delay=1`, then calls parent vtable `+0x48` and finally calls `AnimClass::Constructor @ 0x00421EA0` with `type = parent.Type->TrailerAnim`. The child is a normal allocated `AnimClass` of size `0x1C8`.

5. **Coordinates are sampled from the parent before constructor, using the parent virtual coordinate path.**  
   Active in YR: Conditional. Evidence: `CALL dword ptr [EDX + 0x48]` at `0x0042430A`, with `ECX=ESI` parent, fills the local coordinate pointer then `AnimClass::Constructor` receives that returned coordinate pointer. Prior trailer report mapped this virtual to `AnimClass::GetCoords_WithOwnerOffset @ 0x00422BE0`; attached parents therefore include owner-relative offset.

6. **Child runtime fields are child-type driven after construction.**  
   Active in YR: Yes for every trailer child. Evidence: trailer branch passes the child `AnimTypeClass*` as the constructor type; `AnimClass::Constructor @ 0x00421EA0` then reads fields from `param_1[0x32]` / the child type for `End`, `LoopEnd`, `Rate`, `RandomRate`, `Reverse`, bouncer flags, loop count, and immediate/delayed `Middle()` behavior. Parent lifecycle fields are not copied into the child.

7. **Trailer children do not inherit parent owner object or owner house from this branch.**  
   Active in YR: Conditional on trailer spawn. Evidence: the assembly row at `0x004242F6..0x0042431D` contains only constructor arguments, no call to `AnimClass::SetOwnerObject` or owner-house write. `AnimClass::Constructor @ 0x00421EA0` initializes owner object/house-related fields to null/zero before type setup.

8. **`Next=` mutates the same `AnimClass` object in place and does not allocate.**  
   Active in YR: Conditional on `AnimType+0x2C8 != null` after loop exhaustion. Evidence: decompile at `AnimClass::AI @ 0x004247F3..0x00424932` loads `old_type->Next`, tests non-null, writes it to `AnimClass+0xC8`, performs `End`/`LoopEnd` fill for the new type, clears inactive byte `+0x19B`, resets damage/translucency/timer/current frame fields, calls `AnimClass::Middle`, and returns. No `operator_new(0x1C8)` appears in this branch.

9. **`Next=` does not reset a trailer counter.**  
   Active in YR: Yes/Conditional. Evidence: no per-instance trailer counter exists in the verified trailer branch; cadence is computed from global frame modulo each visit. The `Next=` branch resets playback state but not `+0x19C` first-AI guard, not constructor delay `+0x184`, and no trailer-specific field. The practical effect is: old type may spawn a trailer before the `Next` transition in tick `T`; new type's trailer keys are first eligible on the object's next AI visit.

10. **`Next=` uses the new type's fields after the transition, not the old type's trailer fields.**  
    Active in YR: Conditional on subsequent AI visits. Evidence: after `MOV [ESI+0xC8], ECX` in the `Next=` branch, later AI visits reload `AnimClass+0xC8` at the trailer branch (`0x004242BA`) and therefore read the new type's `+0x308/+0x30C`. No old-type pointer is retained for trailer checks.

11. **`Next=` loop byte semantics differ from constructor loop multiplication.**  
    Active in YR: Conditional on `Next`. Evidence: constructor computes `(byte)type.LoopCount * (byte)ctorLoop` and clamps results `<2` to `1`; the `Next=` branch writes `*(byte *)(this+0x195) = *(byte *)(next_type+0x2C4)` directly in the decompile. This matters because a future generic runtime should not assume the constructor loop multiplier is re-applied on `Next`.

12. **`Next=` calls `Middle()` immediately and does not reapply first-AI guard.**  
    Active in YR: Conditional on `Next`. Evidence: the `Next=` branch calls `AnimClass::Middle()` before returning, but the decompile shows no write setting `AnimClass+0x19C` to `1`; constructor is not called. The next AI visit therefore is not a newly constructed first visit.

## Active-YR Content Evidence

- `ini/artmd.ini` has representative trailer parents: `[DBRIS1LG]`, `[DBRIS5LG]`, `[DBRIS8LG]` with `TrailerAnim=SMOKEY2`, `TrailerSeperation=2`; `[METLARGE]` with `TrailerAnim=SMOKEY2`, `TrailerSeperation=1`; `[METSMALL]` with `TrailerAnim=METSTRAL`, `TrailerSeperation=1`.
- `ini/artmd.ini` has representative `Next=` child content: `[METSTRAL]` has `Next=SMOKEY`.
- Active in YR: Conditional. These keys are consumed by the active `AnimTypeClass::ReadINI` path and the active `AnimClass::AI` path. This slot did not prove that a stock single parent type has both `TrailerAnim` and `Next`; modded combined content is nevertheless active because both branches are in the same live object AI and use normal art metadata.

## Current Rust Surface

- `src/rules/art_data.rs` currently parses `next`, `trailer_anim`, and signed `trailer_seperation`; focused tests include `TrailerSeperation=-2`.
- `src/app_building_anim.rs` contains an app-side `AnimRuntime` with first-AI guard, loop handling, and in-place `Next` for garrison/building animation surfaces.
- `src/sim/components.rs` has `WorldEffect` plus `AnimClassSpawnDescriptor`, but `WorldEffect::tick_with_start_sound` still advances one-shot frame counts and does not implement generic `Next` or periodic `TrailerAnim` emission.
- There is no generic globally registered `AnimClass` runtime that can emit trailer child constructor rows in native AI order for all `AnimClass`-like objects.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Trailer spawn uses current type's `TrailerAnim`/signed `TrailerSeperation` before frame advancement and before `Next=`; cadence is `global_frame % separation`, with `1` special-cased. | Add generic anim AI visit hook before lifecycle advancement; use absolute sim tick/frame, not per-runtime countdown. | `src/sim/components.rs`, generic anim runtime/effect bridge, `src/app_building_anim.rs` if reused. | Two active parent anims with `TrailerSeperation=2` emit on the same even global frame; on a boundary tick the old type's trailer can emit before morphing to `Next`. | `anim_trailer_emits_before_next_using_global_frame_modulo` | High: per-instance countdown or placing trailer after `Next` changes visible smoke cadence and child type. |
| `Next=` mutates the same object, does not allocate, does not reset first-AI guard, and does not reset a trailer counter. | Keep in-place transition for every generic runtime; do not call constructor on `Next`; ensure next AI visit samples new type trailer fields. | `src/app_building_anim.rs`, future generic `AnimClass` runtime, `WorldEffect` replacement path. | Parent `A` with `TrailerAnim=TA`, `TrailerSeperation=1`, `End=1`, `Next=B`; `B` with `TrailerAnim=TB`: tick `T` spawns `TA` then morphs to `B`; tick `T+1` may spawn `TB`, not another `TA`. | `anim_next_does_not_spawn_new_object_and_trailer_type_switches_next_visit` | High: treating `Next` as a spawn or re-running trailer after morph reorders child effects. |
| Trailer child constructor row is `(type=parent.Type.TrailerAnim, coords=parent.GetCoords(), delay=1, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`; child lifecycle comes from child type. | Emit an `AnimClassSpawnDescriptor` preserving these fields and instantiate child runtime from the trailer type's metadata, not parent metadata. | `AnimClassSpawnDescriptor`, `WorldEffect` migration, app render anim queues. | Parent at attached/offset coords emits child with delay `1`; child does not call `Middle()` immediately and uses child `Next`/Rate/Loop fields later. | `anim_trailer_child_constructor_row_uses_delay_one_and_child_type_runtime` | Medium-high: copying parent lifecycle or using delay zero changes sound/frame timing and chained trailer behavior. |

## Negative Facts / Do Not Do

- Do not implement `TrailerSeperation` as a per-instance countdown; binary uses signed global-frame modulo.
- Do not silently treat `TrailerSeperation=0` plus non-null `TrailerAnim` as disabled; the binary reaches signed divide-by-zero.
- Do not evaluate the new `Next` type's trailer in the same AI visit that performed the `Next` transition.
- Do not implement `Next=` as a new `AnimClass` constructor spawn; it mutates the same object.
- Do not initialize trailer children from parent lifecycle fields; only the spawn row comes from the parent branch, then the child constructor reads the child type.

## Remaining Uncertainty

- Exact scheduler same-pass behavior for a newly appended trailer child in every vector-cursor position is deferred to the object registration/update-order slot.
- This slot did not prove a stock retail type with both `TrailerAnim` and `Next` on the same current type; combined interaction is verified as active for modded content and for runtime chains where the current and next types independently define the keys.
- Exact sound result of `delay=1` children is inferred through constructor/Middle order; full sound playback timing is covered by audio-specific reports, not this slot.

## Stale Docs / Replacement Wording

- `docs/research/ANIMCLASS_AI_TRAILERANIM_PERIODIC_SPAWNS_GHIDRA_REPORT.md` Section 6 is stale where it says Rust does not parse `TrailerAnim` or `TrailerSeperation`. Replace with: "Current Rust parses `Next`, `TrailerAnim`, and signed `TrailerSeperation` in `src/rules/art_data.rs`, but only app-side garrison/building `AnimRuntime` implements in-place `Next`; generic `WorldEffect` and the broader AnimClass-like runtime still do not emit periodic trailer child rows or generic `Next` transitions."
- `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md` already contains the corrected `TrailerSeperation=0` wording and does not need the older zero-disables replacement.
- `docs/research/traces/ANIMTYPE_SPAWN_METADATA_NEXT_CONTRACT_TRACE_20260528.md` remains accurate for this slice: metadata parse passes, generic `WorldEffect` `Next` and trailer runtime emission are still not implemented.

## Open Questions Log

- `[RESOLVED] OQ-01 - Does trailer spawn run before or after Next? -> Before Next.` Evidence: `0x004242A6..0x00424322` precedes `0x004247F3..0x00424932`.
- `[RESOLVED] OQ-02 - Does Next reset/continue trailer counters? -> Neither; no per-instance trailer counter exists.` Evidence: global-frame signed modulo at `0x004242D5..0x004242DD`; no trailer field writes in `Next` branch.
- `[RESOLVED] OQ-03 - Which type's trailer fields are used on the transition tick? -> The old/current type at AI entry/trailer branch.` Evidence: branch reads `AnimClass+0xC8` before `Next` writes the new pointer.
- `[RESOLVED] OQ-04 - Which type's fields initialize the child? -> Child `TrailerAnim` type's own fields after constructor begins.` Evidence: constructor receives `parent.Type+0x308` as type and then reads fields through `this->Type`.
- `[RESOLVED] OQ-05 - Does Next create a new object? -> No.` Evidence: no allocation in `Next` branch; in-place write to `AnimClass+0xC8`.
- `[DEFERRED] OQ-06 - Can a newly created trailer child AI on the same global tick and spawn its own trailer immediately?` Category: scheduler-order; reason: object-vector append/cursor behavior belongs to a separate update-order slot.

## Sources

- Ghidra read-only decompile: `AnimClass::AI @ 0x00423AC0`.
- Ghidra read-only assembly context: trailer branch `0x004242A6..0x00424322`; signed modulo `0x004242CA..0x004242DF`; constructor call row `0x004242F6..0x0042431D`; `Next` branch context around `0x004247F3..0x00424932`.
- Ghidra read-only decompile: `AnimClass::Constructor @ 0x00421EA0`.
- Ghidra read-only decompile: `AnimTypeClass::Constructor @ 0x00427530`.
- Ghidra read-only assembly context: `AnimTypeClass::ReadINI` key readers around `0x00428588`, `0x0042863A`, `0x0042864B`.
- Repo INI: `ini/artmd.ini`, `ini/art.ini`.
- Rust surface scan: `src/rules/art_data.rs`, `src/app_building_anim.rs`, `src/sim/components.rs`.

## Status

COMPLETE for the scoped TrailerAnim / signed TrailerSeperation / Next interaction slice.
