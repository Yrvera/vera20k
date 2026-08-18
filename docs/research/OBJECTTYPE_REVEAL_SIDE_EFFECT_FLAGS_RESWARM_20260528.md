# ObjectType Reveal Side-Effect Flags - Reswarm Report

**Address(es):** `ObjectClass::Reveal @ 0x005F4EC0`, `ObjectTypeClass::Constructor @ 0x005F7090`, `ObjectTypeClass::ReadINI @ 0x005F92D0`, `RulesClass::ReadAudioVisual @ 0x0066B700`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** parser/default owners and common stock-YR values for `ObjectTypeClass+0x234`, `+0xAC`, `+0x23A`, `+0x23B..+0x23D`, and `+0x240` only as consumed by `ObjectClass::Reveal`.
**Non-Scope:** all other ObjectType fields, full Reveal ordering, line-trail update/draw, alpha-shape draw lifecycle, all stock object fields, and patching stale docs.
**Confidence:** High for offsets/defaults/parser owners and representative stock enabled entries; Medium for complete stock matrix because only relevant stock key occurrences and common type constructors were audited.
**Active in YR:** Yes, conditional per object type and stock INI/art entry.

## 0. Investigation Contract

**Target question:** Which parser/default owners set the Reveal-consumed ObjectType fields `+0x234`, `+0xAC`, `+0x23A`, `+0x23B..+0x23D`, and `+0x240`, and which common stock YR entries enable the alpha and line-trail side effects?

**Non-goals:** Do not re-prove full `ObjectClass::Reveal` order, do not audit all ObjectType fields, do not audit all alpha/line-trail runtime behavior after creation, and do not patch Rust or stale docs.

**Evidence needed to mark COMPLETE:** binary constructor/default evidence for each offset; binary `ReadINI`/global rules reader evidence for each parser owner; INI scan for stock YR key occurrences; representative type-class constructor evidence for common stock object categories; current Rust parser scan for handoff deltas.

**Stop conditions:** Stop once the target offsets have default/parser ownership, Reveal reader addresses, stock key occurrences, and Rust deltas recorded; defer only full all-type/all-map matrix or asset-existence checks outside this narrow slice.

## 1. Overview

`ObjectClass::Reveal` does not own the values for its logic-registration, alpha, or line-trail gates. It only consumes fields already initialized on the object's type: `+0x234` for logic registration, `+0xAC` for alpha SHP pointer, `+0x23A` for line trail enable, `+0x23B..+0x23D` for fallback RGB, and `+0x240` for line-trail decrement.

The base `ObjectTypeClass` constructor defaults logic registration and line trail off, alpha pointer null, line-trail RGB to `128,128,128`, and decrement to `16`. Common stock subclasses then choose logic-registration eligibility by constructor, while `ObjectTypeClass::ReadINI` parses alpha and line-trail keys from different owner sections.

## 2. Key Offsets / Defaults

| Offset | Meaning in Reveal slice | Default owner/value | Parser owner | Reveal consumer | Active in YR |
|---|---|---|---|---|---|
| `+0xAC` | AlphaImage SHP pointer | `ObjectTypeClass::Constructor` stores `0` at `param_1[0x2B]` | `ObjectTypeClass::ReadINI` reads `AlphaImage` from rules/object section into name buffer `+0x213`; if nonempty, builds `<name>.SHP` and stores loaded pointer at `+0xAC` | `0x005F5045..0x005F5053` gates alpha-shape allocation | Conditional on successful asset load |
| `+0x234` | Logic registration eligibility | base `ObjectTypeClass::Constructor` stores `0` at `0x005F7194`; subclasses override per type | No base `ObjectTypeClass::ReadINI` key found in this slice; Terrain `IsVeinhole=yes` can clear it | `0x005F4FEF..0x005F4FF7` before `FUN_0055BAA0` | Conditional by type class |
| `+0x23A` | UseLineTrail | base constructor stores `0` at `0x005F71B2` | `ObjectTypeClass::ReadINI` reads `UseLineTrail` from art/image section `+0x1F8` at `0x005F9581..0x005F959D` | `0x005F514B..0x005F515D` | Conditional on art image |
| `+0x23B..+0x23D` | Type fallback line-trail RGB | base constructor stores `0x80,0x80,0x80` at `0x005F71B8..0x005F71C4` | `ObjectTypeClass::ReadINI` reads `LineTrailColor` from art/image section at `0x005F95A4..0x005F95C4` | `0x005F51BA..0x005F51DD`, only when global override is all zero | Conditional on line trail and global override |
| `+0x240` | LineTrailColorDecrement | base constructor stores `0x10` at `0x005F71CA` | `ObjectTypeClass::ReadINI` reads `LineTrailColorDecrement` from art/image section at `0x005F95CE..0x005F95E2` | `0x005F51EB..0x005F5202` | Conditional on line trail |
| `Rules+0x1863..0x1865` | Global line-trail RGB override | rules defaults to zero in stock INI | `RulesClass::ReadAudioVisual` reads `LineTrailColorOverride` from `[AudioVisual]` at `0x0066B789..0x0066B7A4` | `0x005F518D..0x005F51E8`; any nonzero component makes global color win | Yes; stock YR value is `0,0,0` |

Tiny detail: line-trail keys are not read from the rules/object section after `Image=` is resolved. `ObjectTypeClass::ReadINI` switches its section pointer to `this+0x1F8`, so stock `[MEDUSA]` and `[DRAGON]` in `artmd.ini`, not weapon or projectile rules sections, own the line-trail values consumed by Reveal.

## 3. `+0x234` Common Type Defaults

| Type class / common stock surface | `+0x234` value after constructor/read | Evidence | Active in YR |
|---|---:|---|---|
| Base `ObjectTypeClass` | `0` | constructor write `0x005F7194` | Yes as inherited base default |
| `TechnoTypeClass` and derived infantry/unit/aircraft/building types | `1` | constructor write at `0x007116AD`; derived constructors call `TechnoTypeClass::Constructor` | Yes for ordinary stock units/buildings |
| `AnimTypeClass` | `1` | assembly write `0x00427750`; constructor decompile shows inherited base then type overrides | Yes for stock animations that reveal |
| `BulletTypeClass` | `1` | constructor region with vtable setup and write `0x0046BD05`; Ghidra missed a clean constructor symbol boundary, but the disassembly context is stable | Yes for projectile/bullet types |
| `TerrainTypeClass` | default `1`; `IsVeinhole=yes` clears to `0` and sets `LegalTarget` | constructor decompile `0x0071DA80`; read path `0x0071DEA0` around `IsVeinhole` branch/write `0x0071DF05` | Yes; standard YR trees default eligible, veinhole path is TS-legacy/conditional |
| `VoxelAnimTypeClass` | `1` | constructor decompile `0x0074AD80`, write at `0x0074AE92` | Yes for voxel anim types |
| `ParticleSystemTypeClass` | `1` | constructor decompile `0x006440A0`, write at `0x0064420E` | Yes for particle system types |
| `ParticleTypeClass` | `0` | constructor decompile `0x00644BE0`, write at `0x00644DB5` with zero register | Yes for particle type definitions |
| `OverlayTypeClass` | `0` | constructor `0x005FE250` calls base and does not override `+0x234` | Yes; stock overlay types are not Reveal-registered through this gate |

## 4. Stock YR Key Occurrences

| Key / section owner | Stock YR entries | Effective Reveal-side value | Evidence | Active in YR |
|---|---|---|---|---|
| `[AudioVisual] LineTrailColorOverride` | `rulesmd.ini:600`, also base `rules.ini:456` | `0,0,0`, so Reveal uses per-type `+0x23B..+0x23D` when line trail is enabled | INI scan; global reader `0x0066B789..0x0066B7A4` | Yes |
| rules/object `AlphaImage` | `TSTLAMP=ALPHATST`; `GALITE`, `REDLAMP`, `GRENLAMP`, `BLUELAMP`, `PURPLAMP` use `NONE` in rulesmd | Nonempty string triggers load attempt into `+0xAC`; Reveal sees only the resulting pointer | INI scan; `ObjectTypeClass::ReadINI @ 0x005F937E`, load block after first char test | Conditional on asset load |
| art/image `UseLineTrail` | `[MEDUSA] yes`, `[DRAGON] yes` in `artmd.ini` | `+0x23A=1` for types whose resolved image section is `MEDUSA` or `DRAGON` | INI scan; `0x005F9581..0x005F959D` | Yes for matching projectile images |
| art/image `LineTrailColor` | `[MEDUSA] 208,208,208`; `[DRAGON] 216,216,255` | Stored at `+0x23B..+0x23D`; used because global override is zero | INI scan; `0x005F95A4..0x005F95C4`; Reveal `0x005F51BA..0x005F51DD` | Yes |
| art/image `LineTrailColorDecrement` | `[MEDUSA] 12`; `[DRAGON] 16` | Stored at `+0x240`; passed to `LineTrail__SetColorDecrement` | INI scan; `0x005F95CE..0x005F95E2`; Reveal `0x005F51EB..0x005F5202` | Yes |

Representative stock projectile image users in `rulesmd.ini`: `MedusaProjectile` uses `Image=MEDUSA`; `HeatSeeker`, `ClusterBits`, `NormalBomb`, `DepthCharge`, `AAHeatSeeker`, `AAHeatSeeker2`, `AirToGroundMissile`, `AAHeatSeeker3`, and `NavalToGroundSeeker` use `Image=DRAGON`.

## 5. Current Rust Implementation Status

| Rust surface | Current shape | Delta |
|---|---|---|
| `src/rules/object_type.rs` | parses many rules/object keys, including `OpenTopped`; no native fields for Reveal `logic_registration_eligible`, `alpha_image`, `use_line_trail`, line-trail RGB, or decrement | Missing parser/default model for all target offsets |
| `src/rules/ruleset.rs` / `src/rules/art_data.rs` | no `LineTrailColorOverride`, `UseLineTrail`, `LineTrailColor`, `LineTrailColorDecrement`, or `AlphaImage` parse hits in the scanned files | Missing rules/art merge surface for target keys |
| `src/sim/world/mod.rs` and `src/sim/world/world_spawn.rs` | `register_live_object` is called directly by spawn paths | Missing native `+0x234` gate and Reveal-ordered side-effect dispatch |
| render/app surfaces | generic alpha/visual alpha fields exist, but no native Reveal-created AlphaShape/LineTrail bridge was found in this scan | Missing Reveal-side visual event surface tied to parsed type flags |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Base defaults for target offsets | verified | `ObjectTypeClass::Constructor @ 0x005F7090`; assembly `0x005F7194..0x005F71CA` | none |
| `AlphaImage` parser and pointer load owner | verified | `ObjectTypeClass::ReadINI @ 0x005F937E`; load block for nonempty `+0x213` | asset-existence result for `NONE.SHP` deferred |
| line-trail parser section owner | verified | `0x005F957A` switches to `+0x1F8`; reads at `0x005F9581`, `0x005F95A4`, `0x005F95CE` | none |
| global line-trail override parser | verified | `RulesClass::ReadAudioVisual @ 0x0066B789..0x0066B7A4`; stock INI `0,0,0` | map override behavior not audited |
| common `+0x234` type defaults | verified for listed type classes | constructors/readers in section 3 | exhaustive all-class table deferred |
| stock alpha/line-trail key occurrences | verified for repo INI corpus | `rg`/Python scan of `ini/rules*.ini`, `ini/art*.ini` | asset load success for alpha names deferred |
| current Rust parser/status scan | touched-not-exhausted | `rg` over `src/rules`, `src/sim`, `src/render`, `src/app_instances` | implementation design/tests |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-OTREVEAL-001 - Who owns base defaults? -> ObjectTypeClass constructor owns all target base defaults.` (evidence: `0x005F7090`; assembly `0x005F7194..0x005F71CA`)
- `[RESOLVED] OQ-OTREVEAL-002 - Is +0x234 parsed by base ObjectType ReadINI? -> No target key read was found in `ObjectTypeClass::ReadINI`; common classes set it in constructors or terrain veinhole read branch.` (evidence: `0x005F92D0`; offset pattern hits)
- `[RESOLVED] OQ-OTREVEAL-003 - Where is AlphaImage read? -> rules/object section, into `+0x213`, then loaded to pointer `+0xAC` if nonempty.` (evidence: `0x005F937E`; `0x005F9640`-tail load block in function decompile)
- `[RESOLVED] OQ-OTREVEAL-004 - Where are line-trail keys read? -> art/image section `+0x1F8`, not the rules/object section.` (evidence: `0x005F957A..0x005F95E2`)
- `[RESOLVED] OQ-OTREVEAL-005 - What are stock enabled line-trail entries? -> `[MEDUSA]` and `[DRAGON]` in artmd/art with colors and decrements.` (evidence: `ini/artmd.ini:14749..14759`)
- `[RESOLVED] OQ-OTREVEAL-006 - Does stock global override suppress per-type line-trail colors? -> No; stock value is all zero, so Reveal uses type RGB.` (evidence: `ini/rulesmd.ini:600`; Reveal `0x005F518D..0x005F51E8`)
- `[RESOLVED] OQ-OTREVEAL-007 - Which common stock categories default +0x234 true? -> Techno-derived, AnimType, BulletType, TerrainType except veinhole, VoxelAnimType, ParticleSystemType.` (evidence: constructors listed in section 3)
- `[RESOLVED] OQ-OTREVEAL-008 - Which common stock categories default +0x234 false? -> base ObjectType, OverlayType, ParticleType.` (evidence: base/overlay/particle constructors)
- `[DEFERRED] OQ-OTREVEAL-009 - Does every `AlphaImage=NONE` stock entry produce a null pointer?` (category: `requires-different-system-context`; reason: binary load path is verified but this slot did not inspect retail MIX asset inventory or `LoadFileFromMIX` failure result for `NONE.SHP`; next-step-if-pursued: verify asset lookup result at runtime or by MIX inventory.)
- `[DEFERRED] OQ-OTREVEAL-010 - Full all-stock type matrix for +0x234?` (category: `bounded-cost-too-high`; reason: common constructor-level values are enough for handoff, but every stock type instance was not enumerated; next-step-if-pursued: build a loader dump after all TypeClass constructors/readers run.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| Logic registration eligibility is type-constructor/default data at `ObjectType+0x234`, not a successful-reveal synonym: base/overlay/particle false, techno/anim/bullet/terrain/voxelanim/particle-system true, with terrain veinhole clearing it. | `0x005F7194`, `0x007116AD`, `0x00427750`, `0x0046BD05`, `0x0071DA80`, `0x0071DF05`; Reveal `0x005F4FEF` | Missing native field and spawn paths register directly | `src/rules/object_type.rs`; `src/sim/world/mod.rs`; future reveal API | Add data-driven/type-category reveal registration eligibility and gate live-order insertion after native Reveal success. | Reveal representative alive objects for unit, overlay-like/object false, particle type false, and terrain veinhole false; assert only `+0x234` true cases enter live order. | `reveal_logic_registration_uses_type_eligibility_flag` | Do not infer logic membership from object category, non-limbo state, or Mark success alone. |
| `AlphaImage` is a rules/object-section string that only affects Reveal through loaded pointer `ObjectType+0xAC`; nonempty string triggers `<name>.SHP` load, while Reveal tests the pointer. | `ObjectTypeClass::ReadINI @ 0x005F937E`; constructor `+0xAC=0`; Reveal `0x005F5045` | Rust lacks `AlphaImage` parse/load and Reveal-ordered alpha-shape event | `src/rules/object_type.rs`; asset loader; render/app reveal event surface | Parse/load alpha image into type data and emit alpha creation only after Reveal mark/display/logic branch reaches the native alpha slot. | `TSTLAMP` with `AlphaImage=ALPHATST` emits an alpha event after registration branch; a missing/null alpha pointer emits none. | `reveal_alpha_image_event_requires_loaded_type_pointer` | Do not treat the literal `AlphaImage` string as sufficient at Reveal time; native tests pointer, not string. |
| Line trail is art-image data: `UseLineTrail` gates allocation, `[MEDUSA]` uses `208,208,208/12`, `[DRAGON]` uses `216,216,255/16`, and stock global override `0,0,0` leaves per-type RGB active. | `0x005F957A..0x005F95E2`; `RulesClass @ 0x0066B789`; `artmd.ini:14749..14759`; Reveal `0x005F514B..0x005F520D` | Rust lacks target art/rules parser fields and Reveal-ordered line-trail event | `src/rules/art_data.rs`; `src/rules/ruleset.rs`; future reveal visual event bridge | Parse line-trail art data and global override; create line trail after alpha slot using native color precedence and decrement. | Reveal a `MedusaProjectile` and a `DRAGON` image projectile with stock rules; assert events use per-type colors/decrements and appear after alpha branch. | `reveal_line_trail_uses_art_image_data_after_alpha_slot` | Do not read line-trail keys from weapon/projectile rules sections or apply global override when it is all zero. |

## 9. Negative Facts / Do Not Do

- Do not add a generic `LogicEligible=` INI parser for `+0x234`; no such base ObjectType key was found in the verified reader. Active in YR: Yes, because constructors/read branches own it.
- Do not parse `UseLineTrail`, `LineTrailColor`, or `LineTrailColorDecrement` from `[Projectile]`, `[Weapon]`, or rules object sections; the verified reader uses the resolved art image section at `+0x1F8`.
- Do not use `AlphaImage` as a render event just because the string is nonempty; Reveal gates on `ObjectType+0xAC` pointer.
- Do not overwrite per-type line-trail RGB with stock `LineTrailColorOverride=0,0,0`; Reveal only uses the global color when at least one global component is nonzero.
- Do not give overlays or particle types live logic registration by default; their common constructors leave `+0x234` false.

## 10. Remaining Uncertainty

- Full all-stock type instance matrix for `+0x234` is not dumped; this report verifies common constructor/read owners and representative stock enabled entries.
- `AlphaImage=NONE` pointer result is not proven by asset inventory/runtime; binary proves only that nonempty names trigger a load attempt and Reveal consumes the resulting pointer.
- Map override behavior for `LineTrailColorOverride` was not audited beyond the global reader and stock rules default.

## 11. Stale Docs / Follow-up Wording

- `ALPHA_SHAPE_CLASS_LIFECYCLE.md` should add/replace its parser wording with: "`AlphaImage=` is read by `ObjectTypeClass::ReadINI @ 0x005F92D0` from the rules/object section into the type's alpha-image name buffer; if the resulting string is nonempty, gamemd attempts to load `<AlphaImage>.SHP` and stores the resulting SHP pointer at `ObjectTypeClass+0xAC`. `ObjectClass::Reveal` tests that pointer, not the string value."
- No new replacement wording is required for `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md` beyond the prior Reveal-order wording from `OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`.

## Sources

- Ghidra read-only decompile/assembly:
  - `ObjectClass::Reveal @ 0x005F4EC0`
  - `ObjectTypeClass::Constructor @ 0x005F7090`
  - `ObjectTypeClass::ReadINI @ 0x005F92D0`
  - `RulesClass::ReadAudioVisual @ 0x0066B700`
  - `AnimTypeClass::Constructor @ 0x00427530`
  - BulletType constructor region around `0x0046BD05` (boundary missed by Ghidra)
  - `OverlayTypeClass::Constructor @ 0x005FE250`
  - `ParticleSystemTypeClass::Constructor @ 0x006440A0`
  - `ParticleTypeClass::Constructor @ 0x00644BE0`
  - `TechnoTypeClass::Constructor @ 0x00710AF0`
  - `TerrainTypeClass::Constructor @ 0x0071DA80`
  - `TerrainTypeClass::ReadINI_Full @ 0x0071DEA0`
  - `VoxelAnimTypeClass::Constructor @ 0x0074AD80`
- INI files checked:
  - `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`
- Prior research:
  - `docs/research/OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`
  - `docs/research/ALPHA_SHAPE_CLASS_LIFECYCLE.md`
  - `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`
- Rust scan:
  - `src/rules/object_type.rs`, `src/rules/ruleset.rs`, `src/rules/art_data.rs`
  - `src/sim/world/mod.rs`, `src/sim/world/world_spawn.rs`
