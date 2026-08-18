# AnimClass::AI TrailerAnim Periodic Spawns - Ghidra Research Report

**Address(es):** `AnimClass::AI @ 0x00423AC0`, trailer branch `0x004242A6..0x00424322`; `AnimClass::Constructor @ 0x00421EA0`; `AnimClass::GetCoords_WithOwnerOffset @ 0x00422BE0`; `AnimTypeClass::ReadINI @ 0x00427D00`; `AnimTypeClass::Constructor @ 0x00427530`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Only the periodic `TrailerAnim=` spawn path inside `AnimClass::AI @ 0x00423AC0`.  
**Non-Scope:** `Next=`, bouncer impact `ExpireAnim=`, warhead/debris caller taxonomy, draw traversal, and Rust implementation edits.  
**Confidence:** High for branch mechanics and INI parse/defaults; Medium for standard-content reachability because this report did not trace every stock constructor caller for debris/meteor parents.  
**Active in YR:** Conditional. The code path is live in `gamemd.exe` and consumes YR `artmd.ini` `TrailerAnim`/`TrailerSeperation`; it fires for active, non-inactive `AnimClass` instances whose type has non-null `TrailerAnim` and whose signed global-frame modulo test passes.

## Working Notes Gate

- `Target question` - Verify exact gate conditions, global-frame separation behavior, constructor argument row, delay value, position source, active-YR liveness, RNG/no-RNG behavior, and Rust-facing implications for `AnimClass::AI` periodic `TrailerAnim=` spawns.
- `Non-goals` - Do not re-investigate `Next=`, global registration/destruction, draw traversal, bouncer impact spawn families, or Rust code changes.
- `Evidence needed to mark COMPLETE` - Decompile plus assembly context for trailer branch and constructor args; decompile plus string-reader assembly for `TrailerAnim`/`TrailerSeperation`; INI defaults/stock examples; Rust surface scan.
- `Stop conditions` - All scoped questions resolved/deferred; zero-add re-read adds no new material questions; exactly this report and the shared claims file may be written.

## 1. Overview

`TrailerAnim=` is consumed directly inside `AnimClass::AI`, before the normal first-AI guard, delay countdown, timer advancement, frame advancement, damage application, and loop/`Next=` end handling. When the branch fires, `gamemd.exe` allocates a new independent `AnimClass` at the parent animation's current effective coordinates using constructor args `(type=TrailerAnim, coords=parent.GetCoords(), delay=1, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

The separation mechanism is not a per-instance countdown. It uses signed division of the global frame counter by `TrailerSeperation`; all parents with the same separation value are synchronized to the same absolute frames.

## 2. Class Layout / Key Offsets

| Object | Offset | Type | Meaning | Active in YR |
|---|---:|---|---|---|
| `AnimClass` | `+0x90` | active byte | Branch requires nonzero before any trailer work. | Yes; constructor sets active at `0x00421EA0`. |
| `AnimClass` | `+0xC8` | `AnimTypeClass*` | Parent type pointer. | Yes. |
| `AnimClass` | `+0xCC` | owner object pointer | Read by `GetCoords_WithOwnerOffset`, not transferred to trailer. | Conditional; only attached anims use it. |
| `AnimClass` | `+0x184` | delay frames | Trailer child receives constructor delay `1`. | Yes. |
| `AnimClass` | `+0x190` | draw flags | Trailer child receives `0x600`. | Yes. |
| `AnimClass` | `+0x195` | loop remaining byte | Child constructor derives it from child type `LoopCount * 1`, clamped to at least `1`. | Yes. |
| `AnimClass` | `+0x19B` | inactive/hidden byte | Branch requires zero; nonzero skips trailer spawn. | Yes. |
| `AnimClass` | `+0x19C` | first-AI guard | Checked after trailer branch, so it does not suppress parent trailer spawning. | Yes. |
| `AnimTypeClass` | `+0x308` | `TrailerAnim` pointer | Non-null gate for branch. | Conditional; default null, parsed from YR art. |
| `AnimTypeClass` | `+0x30C` | signed int `TrailerSeperation` | Signed divisor for global-frame modulo. | Conditional; default zero, parsed from YR art. |

## 3. Core Logic

Verified branch order:

1. If parent active byte `+0x90` is zero, skip the trailer branch.
2. If parent inactive byte `+0x19B` is nonzero, skip the trailer branch.
3. Load parent type pointer from `AnimClass+0xC8`.
4. If `AnimType+0x308 TrailerAnim` is null, skip.
5. Load signed `AnimType+0x30C TrailerSeperation`.
6. If separation is exactly `1`, spawn without division.
7. Otherwise perform signed `g_CurrentFrameCounter / TrailerSeperation` and spawn only when the signed remainder is zero.
8. Allocate `0x1C8` bytes for a new `AnimClass`; if allocation returns null, skip with no coordinate query.
9. Call parent virtual `+0x48` (`AnimClass::GetCoords_WithOwnerOffset`) to fill a local coordinate.
10. Call `AnimClass::Constructor` on the allocated object with type pointer from parent `TrailerAnim`, the returned coordinate pointer, `delay=1`, `loopCount=1`, `drawFlags=0x600`, `zAdjust=0`, and `reverse=0`.

Load-bearing edge cases:

- There is no separate nonzero check for `TrailerSeperation` after `TrailerAnim` is known non-null. With `TrailerAnim != null` and `TrailerSeperation == 0`, the binary reaches `IDIV ECX` with `ECX=0`. Active in YR: Conditional. Standard YR entries that set `TrailerAnim` also set `TrailerSeperation`, but mods must not assume zero disables a non-null trailer.
- Negative `TrailerSeperation` is not clamped by `ReadINI`; the AI branch uses signed `IDIV`. Active in YR: Conditional. Stock YR uses positive values; modded negative values would use signed remainder behavior.
- `TrailerSeperation=1` uses a special equality branch and never divides. Active in YR: Yes for stock `[METLARGE]` and `[METSMALL]`.
- The branch runs before the parent first-AI guard at `+0x19C` and before constructor-delay countdown. Active in YR: Yes. A parent anim can emit a trailer on its first AI visit if all trailer gates pass.
- The spawned child has delay value `1`. Constructor `delay != 0` prevents immediate `Middle()` during construction. On the child's own first AI visit, the first-AI guard clears before delay countdown; `Middle()` is therefore not eligible until a later AI visit. Active in YR: Yes.
- The trailer branch itself makes no RNG calls. Constructor-side RNG may occur only because of the child type's own settings, such as `RandomRate`, `Normalized`, bouncer/meteor initialization, or other constructor behavior. Active in YR: Conditional on child type.

## 4. INI Keys

| Key | Parser / binary evidence | Default | Effect | Active in YR |
|---|---|---:|---|---|
| `TrailerAnim` | `AnimTypeClass::ReadINI @ 0x00427D00`, string xref `0x00428588`, stores pointer at `+0x308` near `0x00428640`; constructor default `+0x308 = 0` at `0x00427530`. | null | Names the child `AnimTypeClass` allocated periodically by parent AI. | Conditional; stock YR uses it on debris/meteor anims. |
| `TrailerSeperation` | `ReadINI` string xref `0x0042863A`, `CCINIClass::ReadInt`, stores signed int at `+0x30C` near `0x0042864B`; constructor default `+0x30C = 0`. | `0` | Signed divisor for `g_CurrentFrameCounter % TrailerSeperation`. Misspelling is the binary key. | Conditional; stock YR uses positive `1` and `2`. |

Stock YR examples checked in `ini/artmd.ini`:

- `[DBRIS1LG]`, `[DBRIS5LG]`, `[DBRIS8LG]`: `TrailerAnim=SMOKEY2`, `TrailerSeperation=2`, bouncer debris with `RandomRate=220,600`.
- `[METLARGE]`: `TrailerAnim=SMOKEY2`, `TrailerSeperation=1`.
- `[METSMALL]`: `TrailerAnim=METSTRAL`, `TrailerSeperation=1`.
- `[SMOKEY2]`: no explicit `Rate`, no `RandomRate`, no `TrailerAnim`; constructor default `Rate` is one logic frame, so this stock trailer child does not consume constructor RNG through `RandomRate`.
- `[METSTRAL]`: `LoopStart=0`, `LoopEnd=8`, `LoopCount=1`, `Rate=600`, `Next=SMOKEY`; no `RandomRate` in the checked section.

## 5. Integration Points

| Integration point | Evidence | Behavior | Active in YR |
|---|---|---|---|
| Per-tick dispatch | Existing global registration/lifetime report verifies `AnimClass` registration and live object-vector AI scheduling; vtable xref to `AnimClass::AI` at `0x007E33B0`. | `AnimClass::AI` is the live update owner for active anim objects. | Yes. |
| Branch location inside AI | Decompile `0x00423AC0`; assembly context `0x004242A6..0x00424322`. | Trailer spawn runs after earlier special bouncer/visibility handling and before `HideIfNoOre`, first-AI guard, delay countdown, timer, frame advance, damage, and loop/`Next=`. | Yes when an anim reaches this branch. |
| Position source | Trailer branch calls vtable `+0x48` at `0x0042430A`; `AnimClass::GetCoords_WithOwnerOffset @ 0x00422BE0`. | Parent coordinates include owner-relative offset when `AnimClass+0xCC` is non-null; otherwise parent object coords. | Conditional for attached parents. |
| Child allocation/registration | `operator_new(0x1C8)` before `AnimClass::Constructor @ 0x0042431D`; constructor decompile at `0x00421EA0`. | Child is a real globally registered `AnimClass`, independent of parent identity. | Yes if allocation succeeds. |

## 6. Current Rust Implementation Status

Rust scan result:

- `src/rules/art_data.rs` parses generic anim lifecycle keys (`Rate`, `LoopStart`, `LoopEnd`, `LoopCount`, `Next`, random rate/delay, render flags) but does not parse `TrailerAnim` or `TrailerSeperation`.
- `src/sim/components.rs` has `AnimRuntime` only for app-side normal SHP animation state and `WorldEffect` for temporary effects; there is no general globally registered `AnimClass` entity with constructor args/order.
- `src/app_building_anim.rs` embeds an `AnimRuntime` for garrison muzzle flashes and implements first-AI guard, lifecycle, and `Next` for that app surface. It has no general periodic trailer spawn path and no global-frame modulo scheduling.
- Existing `WorldEffect` one-shots tick by milliseconds and frame counts; that surface cannot reproduce absolute global-frame synchronized trailer spawns, `delay=1` constructor semantics, owner-relative parent coordinate sampling, or real `AnimClass` registration/order without expansion.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Trailer branch gates | verified | `AnimClass::AI` decompile; assembly `0x004242A6..0x004242C8` | none |
| Signed global-frame modulo | verified | assembly `0x004242CA..0x004242DF` (`CMP ECX,1`, `CDQ`, `IDIV ECX`, `TEST EDX`) | none |
| Constructor argument row | verified | assembly `0x004242E1..0x0042431D`; constructor signature decompile `0x00421EA0` | none |
| Position source and owner-relative behavior | verified | call at `0x0042430A`; `AnimClass::GetCoords_WithOwnerOffset @ 0x00422BE0` | exact owner attachment producers are out of scope |
| No explicit owner transfer | verified | trailer branch call row lacks `SetOwnerObject`; constructor initializes owner fields to zero/null at `0x00421EA0` | none |
| INI parse and defaults | verified | `AnimTypeClass::Constructor @ 0x00427530`; `ReadINI @ 0x00427D00`, string xrefs `0x00428588`, `0x0042863A` | none |
| Standard YR content examples | verified | `ini/artmd.ini` sections named in Section 4 | full spawn-caller taxonomy for debris/meteor parents is other-slot scope |
| Child `Middle()` timing after `delay=1` | verified | constructor `delay` handling at `0x00421EA0`; AI first guard/delay order `0x0042436D..0x004243A1` | scheduler same-pass visit frequency remains covered by global registration doc |
| `Next=` interaction | deferred | not needed beyond branch order | out-of-scope; settled sibling doc says in-place transition |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is the branch gated by parent active state? -> Yes, byte `AnimClass+0x90` must be nonzero before the branch continues.` (evidence: `0x004242A6..0x004242AE`)
- `[RESOLVED] OQ-02 - Is the branch gated by parent inactive/hidden byte? -> Yes, byte `AnimClass+0x19B` must be zero.` (evidence: `0x004242B0..0x004242B8`)
- `[RESOLVED] OQ-03 - Is `TrailerAnim` null checked? -> Yes, `AnimType+0x308` is tested and null skips.` (evidence: `0x004242BA..0x004242C8`)
- `[RESOLVED] OQ-04 - Is `TrailerSeperation=0` a disable gate? -> No; with non-null `TrailerAnim`, zero reaches signed divide-by-zero because only `TrailerAnim` is null-checked and `TrailerSeperation==1` is special-cased.` (evidence: `0x004242CA..0x004242DD`)
- `[RESOLVED] OQ-05 - Is separation per-instance? -> No; it is `g_CurrentFrameCounter % TrailerSeperation`, not an object counter.` (evidence: `0x004242D5..0x004242DD`)
- `[RESOLVED] OQ-06 - Is division signed? -> Yes; assembly uses `CDQ` then `IDIV ECX`.` (evidence: `0x004242D5..0x004242DD`)
- `[RESOLVED] OQ-07 - What happens at separation `1`? -> Special equality branch jumps directly to allocation and avoids division.` (evidence: `0x004242CA..0x004242D3`)
- `[RESOLVED] OQ-08 - What are constructor arguments? -> `(TrailerAnim, parent.GetCoords(), delay=1, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)`.` (evidence: `0x004242F6..0x0042431D`; constructor signature `0x00421EA0`)
- `[RESOLVED] OQ-09 - Is parent position sampled before allocation? -> No; allocation succeeds first, then parent `GetCoords` is called.` (evidence: `0x004242E1..0x0042430A`)
- `[RESOLVED] OQ-10 - Does parent `GetCoords` account for owner-relative attachment? -> Yes when `AnimClass+0xCC` is non-null, `GetCoords_WithOwnerOffset` adds owner object coords to anim offset.` (evidence: `0x00422BE0`)
- `[RESOLVED] OQ-11 - Does the child inherit parent owner object/house? -> No transfer call occurs in this branch; constructor starts owner object and owner house fields as zero/null.` (evidence: `0x004242F6..0x0042431D`; `0x00421EA0`)
- `[RESOLVED] OQ-12 - Does first-AI guard block parent trailer spawning? -> No; trailer branch precedes first-AI guard check at `+0x19C`.` (evidence: `0x004242A6..0x00424375`)
- `[RESOLVED] OQ-13 - Does constructor delay zero/start immediately? -> Trailer child uses `delay=1`, so constructor does not call `Middle()` immediately.` (evidence: `0x00424305`, constructor `0x00421EA0`)
- `[RESOLVED] OQ-14 - Does the trailer branch consume RNG? -> No RNG call in branch; constructor RNG remains child-type dependent.` (evidence: `0x004242A6..0x00424322`; constructor `0x00421EA0`)
- `[RESOLVED] OQ-15 - Where are keys parsed and what are defaults? -> constructor defaults `TrailerAnim=null`, `TrailerSeperation=0`; `ReadINI` reads misspelled key and stores signed int.` (evidence: `0x00427530`, `0x00428588`, `0x0042863A`)
- `[RESOLVED] OQ-16 - Is the path standard-YR relevant? -> Conditional yes; YR `artmd.ini` contains stock positive-separation `TrailerAnim` on debris/meteor anims, and AI consumes it without a TS-only global flag.` (evidence: `ini/artmd.ini`; `0x00423AC0`)
- `[DEFERRED] OQ-17 - Which exact stock gameplay events instantiate every parent trailer type?` (category: `out-of-scope`; reason: caller taxonomy belongs to other swarm slots; next-step-if-pursued: trace debris/meteor parent constructor callers by type.)
- `[DEFERRED] OQ-18 - How often does a trailer child receive same-pass AI after append in every scheduler cursor position?` (category: `requires-different-system-context`; reason: global registration/lifetime report covers append/scheduler shape; next-step-if-pursued: scheduler-order parity trace.)

Zero-add pass: a final cold re-read of `0x004242A6..0x00424322`, `0x00421EA0`, `0x00422BE0`, and `0x00428588..0x0042864B` added no new scoped open questions.

## 9. Visual/UI Composition Ledger

This report verifies visual-effect spawn semantics, not draw composition. Draw traversal, layer ordering, flag expansion, and `DrawIt` translucency are covered by sibling reports.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `AnimClass::AI @ 0x004242A6..0x00424322` | active parent, not inactive, non-null `TrailerAnim`, global-frame modulo pass | child type from `TrailerAnim`; frame initialized by constructor/type lifecycle | parent `GetCoords` result, owner-relative if parent attached | later draw path, not investigated here | Conditional | spawn/overlay effect |

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `SMOKEY2` | Yes when referenced by `TrailerAnim` | Later by normal `AnimClass` draw path | Conditional on parent spawn | no | no | yes | no | no | `ini/artmd.ini`; trailer constructor branch |
| `METSTRAL` | Yes when referenced by `TrailerAnim` | Later by normal `AnimClass` draw path | Conditional on `METSMALL` spawn | no | no | yes | no | no | `ini/artmd.ini`; trailer constructor branch |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Parent trailer spawn is gated by active byte, inactive byte zero, non-null `TrailerAnim`, and signed global-frame modulo of `TrailerSeperation`; there is no per-instance counter. | `0x004242A6..0x004242DF` | Missing; Rust parses no `TrailerAnim` or `TrailerSeperation` and has no generic `AnimClass` runtime. | `src/rules/art_data.rs`, `src/sim/components.rs`, future generic anim tick surface or app effect bridge. | Carry `TrailerAnim` and signed `TrailerSeperation`, test against absolute sim frame, and spawn in native AI order. | Two active parents with `TrailerSeperation=2` both spawn on the same even global frame and not on adjacent odd frames. Proposed test: `anim_trailer_spawn_uses_global_frame_modulo_not_instance_counter`. | Do not implement a countdown reset per parent; it desynchronizes stock global-frame behavior. |
| `TrailerSeperation=0` does not disable a non-null `TrailerAnim`; binary would divide by zero after the non-null trailer gate. | `0x004242C0..0x004242DD`; defaults `0x00427530`; parser `0x0042863A` | Missing/unchecked; current Rust has no field. | INI/rules validation and generic runtime. | Preserve exact semantics or explicitly mark mod data invalid at load with a parity note; do not silently treat zero as disabled when `TrailerAnim` is present. | A mod fixture with `TrailerAnim=SMOKEY2` and omitted/zero `TrailerSeperation` must not silently produce zero spawns. Proposed test: `anim_trailer_zero_seperation_is_not_silent_disable`. | Old docs claim zero disables; that wording is stale and would hide crash/invalid-data parity. |
| Spawned trailer constructor args are `(type, parent.GetCoords(), delay=1, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)`, and spawn occurs before parent first-AI guard/delay/timer/frame logic. | `0x004242F6..0x0042431D`; first guard begins `0x0042436D`; constructor `0x00421EA0`; GetCoords `0x00422BE0` | Missing; `WorldEffect` and garrison `AnimRuntime` cannot represent global constructor order, delay visits, or owner-relative parent coordinates generally. | `src/sim/components.rs`, `src/app_building_anim.rs`, generic world-effect/anim spawn pipeline. | Parent with first-AI guard still emits trailer if global modulo passes; child is inserted with delay `1` and does not call `Middle()` immediately. Proposed test: `anim_trailer_spawns_before_parent_first_ai_guard_with_delay_one_child`. | Do not let first-AI guard or parent delay suppress trailer emission; do not start the child immediately as if `delay=0`. |

### Negative Facts / Do Not Do

- Do not model `TrailerSeperation` as a per-animation countdown. Evidence: branch uses `g_CurrentFrameCounter` and signed `IDIV` at `0x004242D5..0x004242DD`.
- Do not treat `TrailerSeperation=0` as a safe disabled state when `TrailerAnim` is non-null. Evidence: no zero test after loading `+0x30C`; only `TrailerAnim+0x308` null is checked before `IDIV`.
- Do not assume the parent first-AI guard prevents trailer spawning. Evidence: trailer constructor call range `0x004242A6..0x00424322` precedes first-guard check at `0x0042436D`.
- Do not copy parent owner object or owner house onto the trailer child. Evidence: constructor row lacks `SetOwnerObject`; full constructor initializes owner fields to zero/null at `0x00421EA0`.
- Do not call `Middle()`/start sound immediately for trailer children. Evidence: constructor argument `delay=1` at `0x00424305`; constructor calls `Middle()` only when delay field is zero.

### Stale Docs / Follow-up Docs

- `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`: replace "If `TrailerSeperation=0`, no trailers are spawned (the `!= 0` check gates the whole block)" with "If `TrailerAnim` is non-null, `AnimClass::AI` does not test `TrailerSeperation` for nonzero before signed division. It special-cases `TrailerSeperation==1`; otherwise it evaluates `g_CurrentFrameCounter % TrailerSeperation == 0`. Stock YR entries that set `TrailerAnim` also set positive `TrailerSeperation`, but a non-null trailer with zero separation would reach divide-by-zero rather than silently disabling trailers."
- `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`: replace "if `separation != 0`" pseudocode with "if `TrailerAnim != null` and (`TrailerSeperation == 1` or signed remainder of `g_CurrentFrameCounter / TrailerSeperation` is zero)".
- `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`: add "The branch runs before the parent first-AI guard and constructor delay countdown; first-AI guard prevents frame advancement/start, not parent trailer emission."

## Sources

- Ghidra decompiled/read-only: `AnimClass::AI @ 0x00423AC0`.
- Ghidra assembly context/read-only: trailer branch and constructor call `0x004242A6..0x00424322`.
- Ghidra decompiled/read-only: `AnimClass::Constructor @ 0x00421EA0`.
- Ghidra decompiled/read-only: `AnimClass::GetCoords_WithOwnerOffset @ 0x00422BE0`.
- Ghidra decompiled/read-only: `AnimTypeClass::Constructor @ 0x00427530`.
- Ghidra decompiled/read-only: `AnimTypeClass::ReadINI @ 0x00427D00`; string/data xrefs `TrailerAnim @ 0x008183E0`, `TrailerSeperation @ 0x008183CC`.
- Repo INI checked: `ini/artmd.ini`, `ini/art.ini`.
- Prior docs used as leads only: `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`, `docs/research/ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md`, `docs/research/ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`, `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`.
- Rust scan: `src/rules/art_data.rs`, `src/sim/components.rs`, `src/app_building_anim.rs`.
