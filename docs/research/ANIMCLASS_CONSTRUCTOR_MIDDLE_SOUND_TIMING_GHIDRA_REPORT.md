# AnimClass Constructor, Middle, And Sound Timing - Ghidra Report

**Address(es):** `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::AI @ 0x00423AC0`, `AnimClass::Middle @ 0x00424CE0`, `AnimClass::Start @ 0x00424F00`, `AnimClass::Destroy @ 0x004255B0`, `AnimClass::UpdateLoopingSound @ 0x00750D40`, `VocClass::PlayAt @ 0x007509E0`, `AnimTypeClass::Constructor @ 0x00427530`, `AnimTypeClass::ReadINI @ 0x00427D00`  
**Investigation Mode:** exhaustive-slice  
**Scope:** constructor delay semantics, first-AI guard order, `Middle()` start timing, `StartSound`/`Report` and `StopSound` call timing, and `Next=` in-place transition timing for generic `AnimClass` runtime work.  
**Non-Scope:** full audio queue priority, mixing, volume/pan parity, all `VocClass` internals, draw traversal, bouncer physics, and Rust edits.  
**Confidence:** High for call order, offsets, delay behavior, and `Next=`/`Middle()` timing; Medium for exact audible result of the pre-Middle looping sound path because this slice verified `UpdateLoopingSound` allocation/maintenance gates but did not live-capture mixer output.  
**Active in YR:** Yes for the functions and metadata fields; individual sound playback is conditional on non-invisible anims and valid `StartSound`/`Report`/`StopSound` entries.

## Working Notes Gate

- `Target question` - When do constructor, `Middle()`, first-AI guard, `StartSound`/`Report`, `StopSound`, delay `0`, delay `1`, and `Next=` transitions fire relative to one another?
- `Non-goals` - Do not inspect unrelated audio system queueing, full `VocClass` priority behavior, draw/render behavior, or bouncer/teleport spawn taxonomies.
- `Evidence needed to mark COMPLETE` - Decompile evidence for constructor field initialization and delay branch; AI top sound update, first-AI guard, and delay countdown order; `Middle()` sound/start behavior; `Destroy()` stop-sound behavior; `ReadINI` sound fields; `Next=` branch call to `Middle()` without constructor.
- `Stop conditions` - Stop after every scoped timing question is resolved or explicitly deferred, and write only this report plus the shared swarm claims row.

## Summary

`AnimClass::Constructor` initializes the object, appends/registers it, sets the first-AI guard byte, stores constructor delay at `AnimClass+0x184`, and calls `Middle()` immediately only when constructor `delay == 0`. `Middle()` is the native "animation begins" routine: it runs a virtual mode call, handles `StartSound`/`Report` through the start-sound handle, releases/detaches sound handles on the opposite branch, and calls `AnimClass::Start()` only when the type `Start` frame is zero.

`delay=1` children, including trailer children, do not call `Middle()` in the constructor. On their first AI visit, `AnimClass::AI` still executes the top-of-function looping-sound maintenance check before the first-AI guard. The first-AI guard then clears and returns before the delay countdown. On the next AI visit, the delay decrements from `1` to `0`, calls `Middle()`, and returns. Therefore `delay=1` suppresses immediate constructor/Middle/Start behavior and delays `Middle()` until the second AI visit, not merely the next scheduler opportunity.

`Next=` is not a constructor path. When the loop byte expires and `Type->Next` is non-null, `AnimClass::AI` writes the new type pointer into the same object, resets playback fields, calls `Middle()` immediately, and returns. It does not set the first-AI guard and does not apply the constructor delay argument.

## Verified Findings

1. **Sound metadata defaults to absent.**  
   Active in YR: Yes. `AnimTypeClass::Constructor @ 0x00427530` initializes `StartSound/Report` at `AnimType+0x2F8` (`param_1[0xBE]`) and `StopSound` at `AnimType+0x2FC` (`param_1[0xBF]`) to `-1`.

2. **`StartSound=` and `Report=` share one runtime field.**  
   Active in YR: Yes. `AnimTypeClass::ReadINI @ 0x00427D00` reads `StartSound` first into `+0x2F8`; if missing or lookup returns `-1`, it reads `Report` into the same field. `StopSound` is read separately into `+0x2FC`.

3. **Constructor stores the constructor delay at `AnimClass+0x184`.**  
   Active in YR: Yes. In `AnimClass::Constructor @ 0x00421EA0`, `param_1[0x61] = param_4`; byte offset is `0x184`. This is the constructor `delay` argument used by all spawn rows.

4. **Constructor sets the first-AI guard byte.**  
   Active in YR: Yes. Constructor sets byte `AnimClass+0x19C` via `*(undefined1 *)(param_1 + 0x67) = 1`. This guard is independent of constructor delay.

5. **Constructor delay zero calls `Middle()` immediately.**  
   Active in YR: Yes. Near the constructor tail, after loop-count setup, `if (param_1[0x61] == 0) AnimClass__Middle();`. This means teleport rows, damage-fire rows, bouncer `BounceAnim`, and bouncer `ExpireAnim` rows with `delay=0` run `Middle()` during construction.

6. **Constructor delay nonzero skips constructor-time `Middle()`.**  
   Active in YR: Yes. The same constructor branch has no alternate start call for nonzero delay. A trailer child constructed with `delay=1` is registered/revealed with initialized fields but has not run `Middle()` yet.

7. **The top of `AnimClass::AI` performs looping-sound maintenance before first-AI guard and delay.**  
   Active in YR: Yes/Conditional. `AnimClass::AI @ 0x00423AC0` first checks `AnimClass+0x198` false and `Type+0x2F8 != -1`, obtains coordinates, and calls `AnimClass::UpdateLoopingSound @ 0x00750D40`. This occurs before bouncer/visibility/trailer logic, before the first-AI guard at `+0x19C`, and before delay countdown at `+0x184`.

8. **First-AI guard clears and returns before delay countdown.**  
   Active in YR: Yes. In `AnimClass::AI`, after trailer/visibility gates, `if (AnimClass+0x19C != 0) { AnimClass+0x19C = 0; return; }`. The `+0x184` delay countdown block is later. A newly constructed `delay=1` child therefore needs one AI visit to clear the first guard, then a later AI visit to count delay down and call `Middle()`.

9. **Delay countdown calls `Middle()` when the decremented value reaches zero and then returns.**  
   Active in YR: Yes. `AnimClass::AI` checks `AnimClass+0x184 != 0`, decrements it, returns if still nonzero, otherwise calls `Middle()` and returns. No frame advancement occurs in that same visit after `Middle()`.

10. **`Middle()` uses the same `StartSound/Report` field and the start-sound handle.**  
    Active in YR: Yes/Conditional. `AnimClass::Middle @ 0x00424CE0` checks not-invisible (`AnimClass+0x198` false) and `Type+0x2F8 != -1`, gets object coordinates through vtable `+0x48`, and calls `VocClass::PlayAt @ 0x007509E0` with the start/report sound field and the handle at `AnimClass+0x1A0` (`param_1 + 0x68`).

11. **`Middle()` releases/detaches sound handles on the non-start branch and always follows with a second detach/release call.**  
    Active in YR: Yes. The decompile shows an else branch when invisible or `Type+0x2F8 == -1`, followed by another detach/release call before the `Start` check. Exact helper naming is decompiler-noisy, but the scoped timing is clear: this cleanup happens inside `Middle()` before `AnimClass::Start()`.

12. **`Middle()` calls `AnimClass::Start()` only when type `Start` is zero.**  
    Active in YR: Yes. `Middle()` tests `*(int *)(Type+0x298) == 0` and calls `AnimClass::Start @ 0x00424F00` only in that case. Nonzero `Start=` delays `Start()` side effects until later frame logic reaches the start frame.

13. **`AnimClass::Start()` owns particle/scorch/crater/start-damage side effects, not the constructor itself.**  
    Active in YR: Conditional. `AnimClass::Start @ 0x00424F00` reads `SpawnsParticle`, `NumParticles`, `Scorch`, `Crater`, and related type flags after coordinate lookup. Constructor `delay=1` therefore delays these `Start()` side effects at least until `Middle()` runs and the `Start==0` condition passes.

14. **`Destroy()` releases active sound state before optional `StopSound`.**  
    Active in YR: Yes/Conditional. `AnimClass::Destroy @ 0x004255B0` detaches owner, calls `AnimClass::SetOwnerObject(NULL)`, calls a sound-release helper, then checks not-invisible, non-null type, and `Type+0x2FC != -1` before calling `VocClass::PlayAt` with the stop-sound handle at `AnimClass+0x1B4` (`param_1 + 0x6D`).

15. **`StopSound` is tied to `Destroy()`, not `Next=`.**  
    Active in YR: Yes. The `Next=` branch in `AnimClass::AI` mutates `AnimClass+0xC8`, resets playback fields, calls `Middle()`, and returns; it does not call `AnimClass::Destroy`, `ObjectClass::UnInit`, or the `StopSound` check. Stop sound plays when the anim object is destroyed/cleaned up, not when it morphs to `Next`.

16. **`Next=` calls `Middle()` immediately and does not re-run constructor delay or first guard.**  
    Active in YR: Conditional on non-null `Next`. The `Next=` branch writes the new type pointer, fills `End`/frame-count defaults as needed, clears inactive byte `+0x19B`, resets damage accumulator/timing fields, writes current frame from the new type `Start`, then calls `Middle()` directly. No `operator_new`, no constructor call, no write setting `+0x19C = 1`, and no constructor-delay argument is involved.

17. **`Next=` can immediately start the new type's sound.**  
    Active in YR: Conditional. Because `Next=` calls `Middle()` after replacing `AnimClass+0xC8`, `Middle()` reads the new type's `+0x2F8` field. If the new type has `StartSound`/`Report`, the start/report sound is eligible in the same AI visit as the in-place transition.

18. **`UpdateLoopingSound` may allocate or maintain a loop handle before `Middle()` for delayed children.**  
    Active in YR: Conditional. `AnimClass::UpdateLoopingSound @ 0x00750D40` validates the handle, obtains the sound event, computes volume/pan, may allocate a sound event for loopable audio when audio is enabled, and sets the loop handle. Because `AnimClass::AI` calls it before first guard and delay countdown, delayed children with valid `Type+0x2F8` are not simply audio-silent until `Middle()`. Exact audible output depends on whether the resolved audio event is loopable and in range; this report does not claim full mixer parity.

## Delay Timing Matrix

| Constructor row | Constructor result | First AI visit | Second AI visit | Sound/lifecycle consequence |
|---|---|---|---|---|
| `delay=0` | Calls `Middle()` immediately; if `Start==0`, may call `Start()` immediately. | Top looping-sound update may run; first-AI guard clears and returns. | Normal delay block skipped; frame timer/lifecycle can proceed. | Start/report behavior can occur at construction and again through top maintenance on AI visits. |
| `delay=1` | Does not call `Middle()`; first guard remains set. | Top looping-sound update may run, then first guard clears and returns before delay. | Top looping-sound update may run, delay decrements to zero, `Middle()` runs and returns. | Trailer child `Middle()`/`Start()` is delayed until the second AI visit; looping-sound maintenance can still be touched earlier. |
| `Next=` | No constructor; no delay arg. | On loop exhaustion, same object swaps type and calls `Middle()` immediately. | Subsequent visit uses new type without first guard. | No old-type stop sound; new-type start/report eligible immediately through `Middle()`. |

## Current Rust Surface

- `src/sim/components.rs` `WorldEffect::tick_with_start_sound` has a single `start_sound_emitted` edge after `delay_ms` reaches zero; it has no first-AI guard, no top-of-AI looping-sound maintenance, no generic `Next`, and no `StopSound`.
- `src/sim/components.rs` converts `AnimClassSpawnDescriptor.delay` to milliseconds for `WorldEffect`, which cannot represent native first-AI-visit plus native delay-countdown ordering.
- `src/app_building_anim.rs` has an app-side `AnimRuntime` with first-AI guard, delay frames, and in-place `Next` for selected surfaces, but it does not model `StartSound`/`Report`/`StopSound` timing.
- `src/rules/art_data.rs` parses `start_sound` and `report_sound`, but generic runtime surfaces need to preserve the shared native field priority: `StartSound` first, `Report` fallback only if absent/unresolved.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Constructor `delay=0` calls `Middle()` immediately; `delay=1` skips constructor `Middle()`, first AI clears guard, second AI decrements delay to zero and calls `Middle()`. | Replace millisecond-only delayed start semantics with native first-guard plus logic-frame delay semantics for generic anim runtime. | `src/sim/components.rs`, future generic `AnimClass` runtime, `src/app_building_anim.rs` if reused. | A trailer child row with `delay=1` and `Start=0` does not run `Start()` on construction or first AI visit, but does run `Middle()` on the second AI visit. | `anim_constructor_delay_one_middle_runs_after_first_guard_then_countdown` | High: current `WorldEffect` can start one fixed tick early/late and lacks guard ordering. |
| `StartSound` and `Report` share `Type+0x2F8`; `StartSound` has lookup priority and `Report` is fallback only. | Store one resolved start/report sound field for native anim metadata, with fallback priority matching `ReadINI`. | `src/rules/art_data.rs`, audio event bridge. | Type with both keys plays `StartSound`; type with missing/unresolved `StartSound` but valid `Report` uses `Report`. | `anim_type_startsound_priority_report_fallback` | Medium: separate fields can double-play or choose the wrong sound. |
| Top-of-AI `UpdateLoopingSound` runs before first guard and delay countdown. | Do not model all anim audio as a one-shot delayed edge; preserve a looping-sound maintenance hook before lifecycle guards for sounds that are loopable/native-maintained. | Generic anim runtime, `src/audio/events.rs`. | A delayed trailer child with loopable `StartSound` gets pre-Middle sound-maintenance visits, while non-loopable one-shot start remains tied to `Middle()` eligibility. | `anim_looping_sound_maintenance_precedes_first_guard_and_delay` | High for loop sounds such as persistent fires/waterfalls; exact mixer output still needs audio-specific tests. |
| `Next=` is in-place and calls `Middle()` immediately on the new type without constructor delay, first guard, allocation, or old-type stop sound. | Implement generic `Next` as an in-place type swap with immediate `Middle()` sound/start handling and no destroy/stop-sound event. | `src/app_building_anim.rs`, generic anim runtime, `WorldEffect` replacement path. | Type `A` with `Next=B`; `A` has `StopSound`, `B` has `StartSound`; transition emits/maintains `B` start behavior and does not emit `A` stop behavior. | `anim_next_middle_starts_new_type_without_old_stop_sound` | High: treating `Next` as destroy+spawn changes sound and lifecycle bytes. |
| `Destroy()` releases current sound and optionally plays `StopSound` from `Type+0x2FC` at object coords. | Add stop-sound edge on real generic anim destruction, not on `Next`, not on visual allocation failure, and not on ordinary delay expiry. | Generic anim runtime, audio event bridge. | Finished anim with `StopSound` emits it once at destroy; same anim transitioning through `Next` does not emit old stop. | `anim_destroy_plays_stopsound_once_next_does_not` | Medium-high: stop sounds are sparse but obvious where authored. |

## Negative Facts / Do Not Do

- Do not treat constructor `delay=1` as "start on the next AI visit"; first-AI guard consumes that visit before delay countdown.
- Do not call `Middle()` for trailer children in the constructor; their row explicitly passes `delay=1`.
- Do not model `Next=` as a new constructor, allocation, or destroy/spawn pair.
- Do not play the old type's `StopSound` when `Next=` transitions in place.
- Do not store `StartSound` and `Report` as two independently playable native anim sounds.
- Do not claim delayed children are completely untouched by sound code before `Middle()`; top-of-AI looping-sound maintenance can run before first guard and delay countdown.
- Do not reduce native anim audio to a single `start_sound_emitted` boolean if the type's sound is loopable/maintained through `UpdateLoopingSound`.

## Remaining Uncertainty

- Exact mixer-visible output for the pre-Middle `UpdateLoopingSound` path was not live-captured. The call timing and allocation/maintenance gate are verified; full audible parity belongs to a focused audio queue/mixer slot.
- The decompiler labels one sound-handle release helper noisily as `AnimClass__Detach`; this report relies only on call order, handle offsets, and surrounding predicates, not on that helper name.
- Same-pass scheduler position for newly appended children belongs to the global registration/scheduler slot; this report only proves what the child does on its first and later AI visits once visited.

## Stale Docs / Replacement Wording

- `docs/research/ANIMATION_SOUNDS_GHIDRA_REPORT.md` is directionally useful but over-broad where it says start/report sound is simply "played continuously in AnimClass::AI + on Middle." Replacement wording: "`AnimClass::AI` performs a top-of-function `UpdateLoopingSound` maintenance call for non-invisible anims with `Type+0x2F8 != -1` before first-AI guard and delay countdown. `Middle()` separately handles the start/report sound at animation start or `Next=` transition. Exact audible output depends on the resolved audio event and loop/handle state."
- Any doc or trace claiming a `delay=1` trailer child starts immediately should be replaced with: "`delay=1` skips constructor `Middle()`, first AI clears `+0x19C` and returns, and the next AI visit decrements delay to zero and calls `Middle()`."
- Any doc claiming `Next=` allocates or destroys should be replaced with: "`Next=` writes the new type pointer into the same `AnimClass`, resets playback fields, calls `Middle()` immediately, and does not emit the old type's `StopSound`."

## Open Questions Log

- `[RESOLVED] OQ-01 - Does constructor delay zero call Middle immediately? -> Yes.` Evidence: `AnimClass::Constructor @ 0x00421EA0`, `if +0x184 == 0 call Middle`.
- `[RESOLVED] OQ-02 - Does constructor delay one suppress immediate Middle? -> Yes.` Evidence: same constructor branch has no nonzero-delay alternate call.
- `[RESOLVED] OQ-03 - Does first-AI guard run before delay countdown? -> Yes.` Evidence: `AnimClass::AI @ 0x00423AC0` clears `+0x19C` and returns before `+0x184` countdown.
- `[RESOLVED] OQ-04 - Is top-of-AI sound maintenance before the first guard? -> Yes.` Evidence: first branch in `AnimClass::AI` checks `+0x198` and `Type+0x2F8`, then calls `UpdateLoopingSound`.
- `[RESOLVED] OQ-05 - Does Next call Middle? -> Yes.` Evidence: `Next=` branch in `AnimClass::AI` resets fields and calls `AnimClass::Middle()`.
- `[RESOLVED] OQ-06 - Does Next call constructor or set first guard? -> No.` Evidence: no allocation/constructor call and no write setting `+0x19C=1` in the branch.
- `[RESOLVED] OQ-07 - When does StopSound play? -> During Destroy when visible/non-null type and `Type+0x2FC != -1`; not during Next.`
- `[DEFERRED] OQ-08 - What exact waveform/mixer effect occurs for pre-Middle `UpdateLoopingSound` on every sound event type?` Category: audio-mixer; reason: outside scoped call timing; requires focused live/audio queue investigation.

## Sources

- Ghidra read-only decompile: `AnimClass::Constructor @ 0x00421EA0`.
- Ghidra read-only decompile: `AnimClass::AI @ 0x00423AC0`.
- Ghidra read-only decompile: `AnimClass::Middle @ 0x00424CE0`.
- Ghidra read-only decompile: `AnimClass::Start @ 0x00424F00`.
- Ghidra read-only decompile: `AnimClass::Destroy @ 0x004255B0`.
- Ghidra read-only decompile: `AnimClass::UpdateLoopingSound @ 0x00750D40`.
- Ghidra read-only decompile: `VocClass::PlayAt @ 0x007509E0`.
- Ghidra read-only decompile: `AnimTypeClass::Constructor @ 0x00427530`; `AnimTypeClass::ReadINI @ 0x00427D00`.
- Rust surface scan: `src/sim/components.rs`, `src/app_building_anim.rs`, `src/rules/art_data.rs`, `src/audio/events.rs`.

## Status

COMPLETE for the scoped constructor/Middle/start-stop sound timing slice.
