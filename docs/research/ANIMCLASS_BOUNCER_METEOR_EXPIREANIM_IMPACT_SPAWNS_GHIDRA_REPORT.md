# AnimClass Bouncer/Meteor ExpireAnim Impact Spawns - Ghidra Research Report

**Address(es):** `0x00423930` (`AnimClass::ProcessBounceResult`), `0x00423AC0` (`AnimClass::AI`), `0x004255B0` (`AnimClass::Destroy`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `AnimClass` bouncer/meteor `ExpireAnim=` impact-spawn behavior and the normal-destroy negative fact.
**Non-Scope:** full AnimClass constructor caller taxonomy, trailer spawns, chrono/warp spawns, building damage spawns, full draw composition, full BounceClass physics math.
**Confidence:** High
**Active in YR:** Conditional. The code is live for any `AnimClass` constructed from an `AnimTypeClass` with `Bouncer=yes` or `IsMeteor=yes`. Retail `artmd.ini` defines many `Bouncer=yes` debris anims and `IsMeteor=yes` meteor anims; individual runtime liveness depends on those anims being spawned by a live YR event or modded content.

Working notes:
- `Target question`: Does `AnimClass::ProcessBounceResult @ 0x00423930` or normal `AnimClass::Destroy` spawn `ExpireAnim=`, and what are exact impact-spawn arguments/gates?
- `Non-goals`: Do not re-map global AnimClass callers, TrailerAnim, chrono/warp, building/object damage, or full BounceClass physics.
- `Evidence needed to mark COMPLETE`: fresh Ghidra evidence for ProcessBounceResult, AI impact branch, Destroy, ReadINI/default fields, retail INI liveness, current Rust surfaces.
- `Stop conditions`: stop after all `ExpireAnim` impact gates/args and normal-destroy negative fact are resolved; list any broader spawn-path questions as out-of-scope.

## 1. Overview

`ExpireAnim=` is not a generic AnimClass death hook. For `Bouncer=yes` / `IsMeteor=yes` AnimTypes it is an impact animation spawned by `AnimClass::AI` after the embedded BounceClass path reports a bounce or stop and after the terrain/water gate accepts the landing. Normal `AnimClass::Destroy @ 0x004255B0` plays `StopSound=` only and never constructs `ExpireAnim=`.

The older high-level claim "ExpireAnim is impact-only for bouncers and not normal destroy" is verified, but the ownership wording needs one refinement: `ProcessBounceResult @ 0x00423930` itself spawns `BounceAnim=` on return `1`; the `ExpireAnim=` constructor call lives in `AnimClass::AI @ 0x00423DE7..0x00423E70`.

## 2. Class Layout / Key Offsets

| Field | Offset | Source | Meaning | Active in YR |
|---|---:|---|---|---|
| `AnimClass.Type` | `+0x0C8` | constructor/AI decompile | current `AnimTypeClass*` | Yes |
| `AnimClass.IsBouncer` | `+0x194` | constructor `0x00421EA0`, AI `0x00423C24` | per-instance embedded BounceClass driver gate | Conditional: set only for `Bouncer=yes` or `IsMeteor=yes` types |
| `AnimType.BounceAnim` | `+0x300` | ReadINI `0x00428415..0x004284C9`; ProcessBounceResult `0x00423981..0x004239CE` | bounce-tick animation, not ExpireAnim | Conditional |
| `AnimType.ExpireAnim` | `+0x304` | ReadINI `0x004284B5..0x00428573`; AI `0x00423DE7..0x00423E70` | impact animation for accepted bouncer/meteor landings | Conditional |
| `AnimType.Warhead` | `+0x330` | ReadINI `0x00428665..0x004286AD`; AI `0x00423E75..0x00423EAB` | impact damage warhead | Conditional |
| `AnimType.DamageRadius` | `+0x334` | ReadINI `0x00428651..0x00428682`; ProcessBounceResult/AI | impact radius/nearby-object gate | Conditional |
| `AnimType.IsMeteor` | `+0x356` | ReadINI decompile; ProcessBounceResult `0x00423948..0x00423962`; AI `0x00423CE0` | meteor variant gate | Conditional |
| `AnimType.Bouncer` | `+0x35A` | constructor `0x00421EA0`, ReadINI `0x0042869E..0x004286B8` | enables embedded bounce physics | Conditional |
| `AnimType.StopSound` | `+0x2FC` | Destroy `0x004255E4..0x00425618` | normal destruction sound | Conditional |

## 3. Core Logic

### 3.1 Entry Gate

`AnimClass::Constructor @ 0x00421EA0` sets `AnimClass+0x194 = 1` only when `AnimType+0x35A Bouncer` or `AnimType+0x356 IsMeteor` is nonzero, then initializes embedded BounceClass with gravity `1.4` (`0x3FF66666_60000000`). `AnimClass::AI @ 0x00423C24..0x00423C44` checks that instance byte and calls vtable slot `+0x1E8`, whose AnimClass vtable entry at `0x007E353C` points to `0x00423930`.

Active in YR: Conditional. Retail `artmd.ini` has `Bouncer=yes` debris and `IsMeteor=yes` meteor AnimTypes, but this branch only runs for constructed instances of those sections.

### 3.2 ProcessBounceResult

`AnimClass::ProcessBounceResult @ 0x00423930` calls `BounceClass::Update @ 0x00439B00` and returns the update code:

- `0`: no bounce/stop; no impact side effects in this function.
- `1`: bounce this tick. If `AnimType+0x300 BounceAnim` is non-null and allocation succeeds, it constructs `BounceAnim` with args: `delay=0`, `loopCount=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0` (`0x00423981..0x004239CE`). It then scans objects in the current cell and applies impact damage to objects within `DamageRadius` using `Damage`/`Warhead` fields (`0x004239D3..0x00423A83`).
- `2`: stopped. It calls vtable `+0xF8` (`AnimClass::Destroy`) at `0x00423971..0x00423976`, then returns `2`. The caller still runs the AI impact branch for return `2`.

Active in YR: Conditional. This function is live through the instance bouncer gate; it does not itself read or construct `ExpireAnim=`.

### 3.3 ExpireAnim Impact Branch in AI

When `AI` receives return `1` or `2` from vtable `+0x1E8`, it computes two terrain gates:

- `water_cell = (CellClass+0xEC == 2)` from `0x00423C70..0x00423CAA`.
- `above_or_at_ground_gate = (AnimZ >= CellGroundHeight + DAT_0089A1B4)` from `0x00423CB1..0x00423CBE`.

The `ExpireAnim` impact branch runs when `!water_cell || above_or_at_ground_gate` (`0x00423CC1..0x00423CCF` routes accepted cases to `0x00423DE7`). If both are false, the code takes the water/splash branch and does not construct `ExpireAnim`.

At `0x00423DE7..0x00423E70`, if `AnimType+0x304 ExpireAnim` is non-null and `operator_new(0x1C8)` succeeds, `AI` converts the embedded BounceClass float position to integer coords with three `Math::ftol` calls and constructs:

`AnimClass(type->ExpireAnim, impact_coords, delay=0, loopCount=1, drawFlags=0x2600, zAdjust=-30, reverse=0)`.

Load-bearing argument evidence:

- `ExpireAnim` pointer read: `MOV EAX,[EDX+0x304]` at `0x00423DED`; null check jumps to `0x00423EFD`.
- `reverse=0`: `PUSH 0x0` at `0x00423E51`.
- `zAdjust=-30`: `PUSH -0x1e` at `0x00423E53`.
- `drawFlags=0x2600`: `PUSH 0x2600` at `0x00423E55`.
- `loopCount=1`: `PUSH 0x1` at `0x00423E64`.
- `delay=0`: `PUSH 0x0` at `0x00423E6A`.
- constructor call: `CALL 0x00421EA0` at `0x00423E70`.

The same non-null `ExpireAnim` gate also encloses the subsequent impact damage and debris-smoke helper in this branch (`0x00423E75..0x00423EF8`). If `ExpireAnim` is null, the code jumps to `0x00423EFD`, skipping that damage/smoke block.

Active in YR: Conditional. Retail `artmd.ini` examples include `DBRIS1LG` with `Bouncer=yes`, `ExpireAnim=TWLT036`, `Damage=20`, `DamageRadius=80`, `Warhead=HE`; `METDEBRI` with `Bouncer=yes`, `ExpireAnim=TWLT070`; and `METLARGE`/`METSMALL` with `IsMeteor=true`.

### 3.4 Normal Destroy Negative Fact

`AnimClass::Destroy @ 0x004255B0` does not call `AnimClass::Constructor`, does not read `AnimType+0x304`, and does not branch on `ExpireAnim`. Its side effects are:

1. detach from owner object if `AnimClass+0x0CC` is non-null (`0x004255BC..0x004255C3`);
2. `SetOwnerObject(NULL)` and sound release (`0x004255C6..0x004255D5`);
3. if not suppressed, `Type` is non-null, and `StopSound != -1`, play `StopSound` at `SparkleCoords` (`0x004255DA..0x00425618`);
4. call `ObjectClass::UnInit` (`0x0042561D..0x0042561F`).

Active in YR: Yes. This is the vtable `+0xF8` destruction path for normal AnimClass cleanup. The Ghidra plate comment on this function is stale/misleading because it says ExpireAnim is spawned, but the function body refutes that.

## 4. INI Keys

| Key | File/source | Parsed field | Default | Effect in this slice | Active in YR |
|---|---|---:|---:|---|---|
| `Bouncer=` | `art(md).ini`; ReadINI `0x0042869E..0x004286B8` | `+0x35A` | false | constructor sets `AnimClass+0x194`; AI drives bounce/impact | Conditional; retail debris sections use it |
| `IsMeteor=` | `art(md).ini`; ReadINI decompile | `+0x356` | false | constructor uses meteor variant; ProcessBounceResult adjusts z | Conditional; retail `METLARGE`/`METSMALL` use it |
| `BounceAnim=` | ReadINI `0x00428415..0x004284C9` | `+0x300` | null | spawned by `ProcessBounceResult` on return `1`, flags `0x600` | Conditional |
| `ExpireAnim=` | ReadINI `0x004284B5..0x00428573` | `+0x304` | null | impact-only spawn in `AI`, flags `0x2600`, zAdjust `-30` | Conditional |
| `Damage=` | ReadINI decompile | `+0x2A8` | `0.0` | impact damage amount in this branch | Conditional |
| `DamageRadius=` | ReadINI `0x00428651..0x00428682` | `+0x334` | `0` | impact radius / nearby-object gate | Conditional |
| `Warhead=` | ReadINI `0x00428665..0x004286AD` | `+0x330` | null | impact damage warhead | Conditional |
| `StopSound=` | ReadINI decompile; Destroy `0x004255EE..0x00425618` | `+0x2FC` | `-1` | normal destroy sound only | Conditional |

## 5. Integration Points

`ProcessBounceResult` has no ordinary static caller because it is reached through the AnimClass vtable. The chain verified here is:

`AnimClass::Constructor` (`Bouncer`/`IsMeteor` -> set `+0x194`) -> `LogicClass`/object AI tick -> `AnimClass::AI @ 0x00423C24..0x00423C44` -> vtable `+0x1E8` -> `AnimClass::ProcessBounceResult @ 0x00423930` -> AI impact branch -> final `AnimClass::Destroy`.

The final `AI` branch always calls vtable `+0xF8` after the accepted impact handling, so impact-spawned `ExpireAnim` is emitted before the parent bouncer/meteor is destroyed by AI.

## 6. Current Rust Implementation Status

Rust has app-side `AnimRuntime`, `GarrisonMuzzleFlash`, `WeaponMuzzleFlash`, and `WorldEffect` in `src/sim/components.rs`, plus ad-hoc world-effect spawning in combat, bridge, and superweapon code. `src/rules/art_data.rs` has `AnimTypeRuntimeConfig` for lifecycle/draw fields but does not carry `Bouncer`, `IsMeteor`, `BounceAnim`, `ExpireAnim`, `Damage`, `DamageRadius`, or `Warhead`.

No generic `AnimClass` / embedded BounceClass runtime was found for SHP debris/bouncers. `VoxelAnimation` exists for HVA frame cycling only and does not model BounceClass physics or bouncer impact side effects.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AnimClass::ProcessBounceResult @ 0x00423930` | verified | decompile plus disassembly `0x00423930..0x00423AB7` | none for this slice |
| `AI` bouncer call gate | verified | `0x00423C24..0x00423C44`; vtable xref `0x007E353C -> 0x00423930` | none |
| `AI` terrain/water impact gate | verified | `0x00423C70..0x00423CCF` | exact enum meaning of `Cell+0xEC==2` accepted from existing cell-terrain docs, not re-derived here |
| `AI` ExpireAnim constructor args | verified | `0x00423DE7..0x00423E70` | none |
| normal `AnimClass::Destroy` | verified | `0x004255B0..0x00425628` | none |
| `AnimTypeClass::ReadINI` key offsets | verified | `0x00427D00`; key-specific ranges above | none |
| full runtime caller taxonomy for constructing bouncers | deferred | out-of-scope parent slot 1 | dispatch constructor-caller taxonomy slot |
| full water/splash visual behavior | deferred | AI branch touched, not expanded | separate water-impact visual investigation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Where is the bouncer/meteor entry point? -> AnimClass constructor sets +0x194 from type +0x35A/+0x356, then AI calls vtable +0x1E8 when +0x194 is set.` (evidence: `0x00421EA0`; `0x00423C24..0x00423C44`)
- `[RESOLVED] OQ-02 - Does ProcessBounceResult spawn ExpireAnim? -> No. It spawns BounceAnim on return 1; ExpireAnim is spawned by AI after the vtable result.` (evidence: `0x00423981..0x004239CE`; `0x00423DE7..0x00423E70`)
- `[RESOLVED] OQ-03 - Does normal Destroy spawn ExpireAnim? -> No. Destroy reads StopSound +0x2FC and has no constructor call or +0x304 read.` (evidence: `0x004255B0..0x00425628`)
- `[RESOLVED] OQ-04 - Exact impact constructor arguments? -> `delay=0`, `loopCount=1`, `drawFlags=0x2600`, `zAdjust=-30`, `reverse=0`, coords from BounceClass floats via ftol.` (evidence: `0x00423E28..0x00423E70`)
- `[RESOLVED] OQ-05 - What gates ExpireAnim impact spawn? -> return 1/2 from ProcessBounceResult, accepted terrain/water gate, non-null +0x304, and allocation success.` (evidence: `0x00423C24..0x00423CCF`; `0x00423DE7..0x00423E70`)
- `[RESOLVED] OQ-06 - Is bouncer/meteor path active in YR? -> Conditional; retail artmd defines Bouncer/IsMeteor AnimTypes and binary parses/uses the keys.` (evidence: `ini/artmd.ini` sections `DBRIS*`, `METLARGE`, `METSMALL`, `METDEBRI`, `CRYSTAL1..4`; ReadINI `0x00427D00`)
- `[RESOLVED] OQ-07 - What happens when ExpireAnim is null? -> AI jumps past the ExpireAnim constructor and the immediately enclosed impact damage/smoke block.` (evidence: `0x00423DED..0x00423EFD`)
- `[DEFERRED] OQ-08 - Which standard gameplay caller constructs every retail bouncer AnimType?` (category: `out-of-scope`; reason: parent slot 1 owns constructor caller taxonomy; next-step-if-pursued: classify callers of `0x00421EA0` by AnimType source)
- `[DEFERRED] OQ-09 - Exact water splash branch assets/order?` (category: `out-of-scope`; reason: this slot only verifies ExpireAnim impact spawns; next-step-if-pursued: trace `0x00423CD5..0x00423DE2`)
- `[RESOLVED] OQ-10 - Current Rust surface? -> No generic SHP bouncer/meteor AnimClass runtime or parser fields for this key set were found.` (evidence: `rg` over `src`; `src/rules/art_data.rs`; `src/sim/components.rs`)

## 9. Visual/UI Composition Ledger

This report does not claim draw composition. It verifies creation of a visual object:

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `AnimClass::AI @ 0x00423DE7..0x00423E70` | `+0x194` set; ProcessBounceResult return `1/2`; terrain/water gate; `ExpireAnim != null` | `AnimType.ExpireAnim` | impact coords from BounceClass floats, ftol to CoordStruct | deferred to draw path | Conditional | impact overlay |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `ExpireAnim=` target, e.g. `TWLT036/TWLT070/TWLT100` | conditional | deferred | conditional | no | no | yes | no | no | constructor call at `0x00423E70`; retail `artmd.ini` examples |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `ExpireAnim=` spawns only from accepted bouncer/meteor impact branch, not from normal Destroy | AI `0x00423DE7..0x00423E70`; Destroy `0x004255B0..0x00425628` | missing | `src/rules/art_data.rs`; future generic `AnimClass`/SHP bouncer runtime; `src/sim/components.rs` / world effect spawn surface | parse the keys and spawn `ExpireAnim` only when bouncer physics returns 1/2 and impact gate accepts | Create a `Bouncer=yes` test anim with `ExpireAnim=TWLT036`, force landing on accepted ground, assert exactly one `TWLT036` effect before parent deletion | Do not hook `ExpireAnim` into all `WorldEffect` expiry/destruction |
| Impact constructor args are fixed: delay 0, loop arg 1, flags `0x2600`, zAdjust `-30`, reverse 0 | `0x00423E51..0x00423E70` | missing | future `AnimRuntime`/render submission metadata | carry draw flags and z-adjust into the spawned visual; do not fall back to type default zAdjust because constructor param is nonzero | Spawn bouncer impact `ExpireAnim` and assert effect metadata has `draw_flags=0x2600`, `z_adjust=-30`, `delay=0`, `reverse=false` | Using generic `WorldEffect` defaults will lose depth/order parity |
| `BounceAnim=` and `ExpireAnim=` are distinct: ProcessBounceResult uses `+0x300` on return 1; AI uses `+0x304` on accepted impact | ProcessBounceResult `0x00423981..0x004239CE`; AI `0x00423DED..0x00423E70` | missing | parser/runtime for AnimType bouncer fields | model both keys separately and allow both to spawn in the same tick when return 1 plus accepted impact conditions apply | Test anim with both `BounceAnim=BNC` and `ExpireAnim=EXP`, force return 1 accepted landing, assert spawn order `BounceAnim` then `ExpireAnim` | Collapsing both keys into one "impact animation" loses same-tick ordering |

Proposed Rust test names:

- `anim_bouncer_impact_spawns_expireanim_only_on_accepted_ground`
- `anim_bouncer_expireanim_constructor_args_match_native`
- `anim_bounceanim_and_expireanim_same_tick_order_is_preserved`
- `anim_destroy_does_not_spawn_expireanim`

### Negative Facts / Do Not Do

- Do not spawn `ExpireAnim` from normal `AnimClass::Destroy`; `Destroy` reads `StopSound +0x2FC`, calls `VocClass__PlayAt`, and calls `ObjectClass::UnInit`, with no `+0x304` read or constructor call (`0x004255DA..0x0042561F`).
- Do not say `ProcessBounceResult` owns the `ExpireAnim` constructor; it owns `BounceAnim +0x300` and returns a code. The `ExpireAnim +0x304` constructor is in `AI` (`0x00423981..0x004239CE`; `0x00423DE7..0x00423E70`).
- Do not implement `ExpireAnim` as always-on bouncer damage. The impact damage/smoke block in this AI branch is skipped when `ExpireAnim` is null (`0x00423DED..0x00423EFD`).
- Do not treat water impacts below the ground gate as `ExpireAnim` impacts. The branch goes to the water/splash path when `Cell+0xEC == 2` and `AnimZ < GroundHeight + DAT_0089A1B4` (`0x00423C70..0x00423CCF`).
- Do not rely on the stale Ghidra plate comment for `AnimClass::Destroy`; the actual disassembly/decompile body is the source of truth.

### Stale Docs / Follow-up Docs

- `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md` replacement wording for section "BounceAnim= and ExpireAnim=":
  "For `Bouncer=yes` / `IsMeteor=yes` AnimClass instances, `AnimClass::ProcessBounceResult @ 0x00423930` drives BounceClass and may spawn `BounceAnim=` (`AnimType+0x300`) on update return `1` with constructor args `delay=0`, `loopCount=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`. The `ExpireAnim=` (`AnimType+0x304`) impact spawn is not inside `ProcessBounceResult`; it is in `AnimClass::AI @ 0x00423DE7..0x00423E70` after the vtable `+0x1E8` call returns `1` or `2`, when the terrain/water gate accepts the landing and `ExpireAnim` is non-null. The constructor args are `delay=0`, `loopCount=1`, `drawFlags=0x2600`, `zAdjust=-30`, `reverse=0`."
- `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md` replacement wording for section "ExpireAnim on Normal Anim Destruction":
  "`AnimClass::Destroy @ 0x004255B0` does not spawn `ExpireAnim=`. It detaches owner links, calls `SetOwnerObject(NULL)`, releases sound, optionally plays `StopSound=` (`AnimType+0x2FC`) at `SparkleCoords`, then calls `ObjectClass::UnInit`. There is no `AnimType+0x304` read and no `AnimClass::Constructor` call in this function."
- `docs/research/ADDRESS_MAP.md` line for `0x004255B0` should be replaced with:
  "`0x004255B0` | `AnimClass::Destroy` (detach, clear owner, release sound, optional StopSound, deferred delete; does NOT spawn ExpireAnim) | - | `ANIM_CLASS`"

## Sources

- Ghidra decompiled/read-only: `0x00421EA0`, `0x00423930`, `0x00423AC0`, `0x004255B0`, `0x00427530`, `0x00427D00`
- Ghidra disassembly/address ranges: `0x00423971..0x004239CE`, `0x00423C24..0x00423CCF`, `0x00423DE7..0x00423EFD`, `0x004255B0..0x00425628`, `0x004284B5..0x00428573`, `0x00428651..0x004286B8`
- Prior docs checked: `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`, `docs/research/BOUNCE_CLASS_GHIDRA_REPORT.md`, `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`
- INI checked: `ini/artmd.ini`, `ini/art.ini`, `ini/rulesmd.ini`, `ini/rules.ini`
- Rust scan: `src/rules/art_data.rs`, `src/sim/components.rs`, `src/sim/animation.rs`, `src/sim/combat/mod.rs`, `src/sim/world/bridge_orchestrator.rs`
