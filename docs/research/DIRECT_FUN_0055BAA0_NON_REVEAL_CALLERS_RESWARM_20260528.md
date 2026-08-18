# Direct `FUN_0055BAA0` Non-Reveal Callers - Reswarm 2026-05-28

**Address(es):** `FUN_0055BAA0 @ 0x0055BAA0`, remover `FUN_0055BAE0 @ 0x0055BAE0`, direct non-`ObjectClass::Reveal` registration call sites `0x00435B01`, `0x00437070`, `0x00710492`, `0x0075F95F`, paired remover sites `0x00435B7E`, `0x00437042`, `0x004370EE`, `0x0075F9BD`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Classify the direct non-`ObjectClass::Reveal` callers named by the parent swarm, identify owner class/path, active-YR condition, and Rust-facing lifecycle implication.  
**Non-Scope:** Re-proving ordinary `ObjectClass::Reveal` registration, `ObjectClass::Conceal`/destructor unregistration, helper/remover internals, full BuildingLight/WaveClass rendering, full BFRT combat behavior, save/load active-vector rebuild ownership, or Rust implementation.  
**Confidence:** High for caller identity, call shape, and standard-YR activation classification; Medium for Rust deltas because Rust was scanned statically.  
**Active in YR:** Conditional overall. OpenTopped passenger registration and WaveClass registration are stock-live in standard YR data. BuildingLight registration is a live engine path but requires `HasSpotlight=yes`; no repo stock assignment was found.

## 0. Working Notes Gate

- **Target question:** Which direct non-`ObjectClass::Reveal` callers of `FUN_0055BAA0` / `FUN_0055BAE0` are active in standard YR, and what lifecycle meaning should Rust preserve?
- **Non-goals:** Do not re-prove ordinary `ObjectClass::Reveal`, `ObjectClass::Conceal`, destructor removal, helper internals, or generic active-vector semantics unless contradiction appears.
- **Evidence needed to mark COMPLETE:** For the named callers and paired removers, decompile plus assembly/xref evidence identifying class/path, activation condition, and Rust-facing lifecycle implication.
- **Stop conditions:** Stop after the named direct caller slice is classified, extra callers are listed as open/deferred if not covered, and this allowed report is written.

## 1. Overview

The direct non-`Reveal` callers are not a hidden map-load ordering source. They are three class-specific lifecycle families:

1. `BuildingLightClass` constructor and reveal wrapper call `ObjectClass::Reveal` first, then directly call `FUN_0055BAA0` on success.
2. `TechnoClass::SetInOpenTransport` marks a passenger as inside an open-topped transport, calls a passenger virtual, then directly registers that passenger in the `LogicClass` active vector.
3. `WaveClass` reveal wrapper submits the wave to display, then directly registers the wave in the same active vector.

All four verified registration sites pass `unique_scan_flag=0` and use `ECX=0x87F778`, the same `LogicClass` singleton as ordinary reveal registration. Duplicate prevention remains the object-local `Object+0x98` membership byte from the helper report.

## 2. Key Fields / Gates

| Field / global | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `ObjectClass+0x98` | all object-derived | Active-vector membership byte used by add/remove helpers | helper report; all direct sites pass `this`/passenger to `0x0055BAA0` or `0x0055BAE0` | Yes |
| `0x87F778` | `LogicClass` singleton | Target active vector for all direct calls | assembly at `0x00435AFC`, `0x0043706B`, `0x0071048D`, `0x0075F95A` | Yes |
| `BuildingTypeClass+0x154B` | building type | `HasSpotlight=` allocation gate | `BuildingClass::Unlimbo @ 0x00441169..0x00441190`; BuildingLight report | Conditional |
| `BuildingClass+0x600` | building instance | `BuildingLightClass*` storage | `0x00441190` writes returned constructor pointer | Conditional |
| `TechnoClass+0x82` | passenger techno | in-open-topped-transport byte set before direct registration | `TechnoClass::SetInOpenTransport @ 0x00710470`; assembly `0x0071047D` | Yes, conditional on OpenTopped entry |
| `TechnoTypeClass+0x5E4` | transport type | `OpenTopped=` gate before `SetInOpenTransport` callers | `0x0051A451..0x0051A45E`, `0x0073A750..0x0073A75D`; `rulesmd.ini:[BFRT] OpenTopped=yes` | Yes |
| `WeaponTypeClass+0x130` | weapon type | `IsSonic=` WaveClass type-0 construction gate | `TechnoClass::Fire_At @ 0x006FF43F..0x006FF470`; `rulesmd.ini:[SonicZap]`, `[SonicZapE]` | Yes |
| `WeaponTypeClass+0x15C` | weapon type | `IsMagBeam=` WaveClass type-3 construction gate | `TechnoClass::Fire_At @ 0x006FF5F5..0x006FF647`; `rulesmd.ini:[MagneticBeam]` rows | Yes |

## 3. Direct Registration Callers

| Site | Owner/path | Direct call shape | Active in YR |
|---|---|---|---|
| `0x00435B01` | `BuildingLightClass::Constructor @ 0x00435820` | After `ObjectClass::Reveal(&coords, 0)` succeeds: `PUSH 0`, `PUSH ESI`, `MOV ECX,0x87F778`, `CALL 0x0055BAA0`. | Conditional. `BuildingClass::Unlimbo` creates the object only when `BuildingType+0x154B HasSpotlight` is nonzero. Repo `ini/rules*.ini` and `ini/art*.ini` have no `HasSpotlight=` assignment. |
| `0x00437070` | `BuildingLightClass` reveal wrapper `FUN_00437050` | Calls `ObjectClass::Reveal(param_2, param_3)`; on success calls `FUN_0055BAA0(this, 0)` and returns `1`. | Conditional. Same `BuildingLightClass` object condition as above; vtable data xref `0x007E3BA8` proves wrapper ownership. |
| `0x00710492` | `TechnoClass::SetInOpenTransport @ 0x00710470` | Null guard; write passenger `+0x82=1`; call passenger vtable `+0x3D0`; then direct `FUN_0055BAA0(passenger, 0)`. | Yes, conditional on entering an `OpenTopped` transport. Infantry and Unit entry paths test target type `+0x5E4`; stock `[BFRT] OpenTopped=yes` and passenger `OpenTransportWeapon=` rows make this standard YR gameplay. |
| `0x0075F95F` | WaveClass reveal wrapper `FUN_0075F8B0` | After game-active, in-limbo, display/layer, and visibility gates pass, it submits display object if layer lookup succeeds, then calls `FUN_0055BAA0(this, 0)`. | Yes. `TechnoClass::Fire_At` constructs WaveClass for stock `IsSonic=Yes` and `IsMagBeam=yes` weapons. |

Material shared fact: Active in YR: Yes/Conditional per row. Evidence: `get_bulk_xrefs(0x0055BAA0)` returned direct call xrefs `0x005F5040`, `0x0075F95F`, `0x00435B01`, `0x00437070`, `0x00710492` plus data xref `0x007E1918`; call-site assembly proves all four direct non-Reveal calls pass flag `0` and target `0x87F778`.

## 4. Paired Direct Removers

| Site | Owner/path | Direct remover shape | Active in YR |
|---|---|---|---|
| `0x00437042` | `BuildingLightClass` conceal wrapper `FUN_00437030` | Calls `ObjectClass::Conceal`; if it returns nonzero, calls `FUN_0055BAE0(this)` and returns `1`. | Conditional on `BuildingLightClass` existence. |
| `0x004370EE` | `BuildingLightClass::Destructor @ 0x004370C0` | Installs BuildingLight vtables, calls `ObjectClass::Conceal`; on success calls remover, then removes from `DAT_008B4194` BuildingLight vector and calls `ObjectClass::Destructor`. | Conditional on `BuildingLightClass` existence. |
| `0x00435B7E` | Bad/overlapping BuildingLight-region xref | Assembly context after a bad boundary shows a direct remover followed by BuildingLight vector removal pattern. | Conditional/uncertain boundary. Clean destructor evidence is `0x004370EE`; read-only rules forbade function-boundary repair. |
| `0x0075F9BD` | WaveClass conceal/unreveal wrapper following `FUN_0075F8B0` | Calls display/layer remove helper, then `FUN_0055BAE0(this)`, then vtable `+0x11C`, writes `Object+0x81=1`, clears byte at `+0x80`, returns `1`. | Yes for WaveClass lifecycle. |

Settled ordinary removers `ObjectClass::Conceal @ 0x005F4DD3` and destructor fallback `0x005F3D75` remain outside this slice except as pairing context.

## 5. Activation Details

### 5.1 BuildingLight

`BuildingClass::Unlimbo @ 0x00441169..0x00441190` reads byte `[type+0x154B]`, skips allocation when zero, otherwise allocates `0xE8`, calls `BuildingLightClass::Constructor`, and stores the result at `BuildingClass+0x600`. The constructor appends to the separate BuildingLight global vector first, computes initial coordinates, calls `ObjectClass::Reveal`, and only then calls `FUN_0055BAA0`.

Active in YR: Conditional. The parser/runtime path is live, but standard repo data did not contain `HasSpotlight=`. A mod/map/rules override can activate it. Evidence: decompile `0x00435820`, assembly `0x00435AF0..0x00435B01`, `0x00441169..0x00441190`, and repo `rg HasSpotlight` checks.

### 5.2 OpenTopped Passengers

`TechnoClass::SetInOpenTransport @ 0x00710470` has one null guard. For a non-null passenger it writes `Techno+0x82=1`, calls vtable `+0x3D0`, and appends the passenger object to the active vector.

Known ordinary caller gates:

- `InfantryClass` entry path `0x0051A451..0x0051A45E`: calls target `GetType`, tests `type+0x5E4`, and calls `SetInOpenTransport` only when nonzero.
- `UnitClass` entry path `0x0073A750..0x0073A75D`: same target `OpenTopped` gate and call shape.

Active in YR: Yes, conditional on entering an open-topped transport. Stock `rulesmd.ini` has `[BFRT] OpenTopped=yes`, `Passengers=5`, and multiple infantry `OpenTransportWeapon=` values.

### 5.3 WaveClass

`TechnoClass::Fire_At` constructs WaveClass via `WaveClass::Constructor @ 0x0075E950` when:

- `WeaponType+0x130` is nonzero (`IsSonic=`): assembly `0x006FF43F..0x006FF470`.
- `WeaponType+0x15C` is nonzero (`IsMagBeam=`) and the no-existing-wave gate passes: assembly `0x006FF5F5..0x006FF647`.

The constructor calls `FUN_0075F8B0` at `0x0075EB57`. The reveal wrapper registers in the active vector after display/layer submission. Active in YR: Yes. Repo `rulesmd.ini` contains `[SonicZap] IsSonic=Yes`, `[SonicZapE] IsSonic=Yes`, and `[MagneticBeam] IsMagBeam=yes` rows.

## 6. INI Keys

| Key | Scope | Stock/default status | Effect on this slice | Active in YR |
|---|---|---|---|---|
| `HasSpotlight=` | building type | default false; no repo stock assignment found | enables BuildingLight allocation and its direct registration/removal lifecycle | Conditional |
| `OpenTopped=` | transport TechnoType | stock `[BFRT] OpenTopped=yes` | gates passenger `SetInOpenTransport` caller reachability | Yes |
| `OpenTransportWeapon=` | passenger TechnoType | stock infantry rows set `0` or `1`; default `-1` | makes open-topped passenger logic useful after `+0x82` and registration | Yes |
| `IsSonic=` | WeaponType | stock `[SonicZap]` and `[SonicZapE]` set `Yes` | triggers WaveClass type 0 construction | Yes |
| `IsMagBeam=` | WeaponType | stock `[MagneticBeam]` rows set `yes` | triggers WaveClass type 3 construction | Yes |

No INI key changes `FUN_0055BAA0` insertion semantics. All verified direct callers pass helper flag `0`, so ordinary duplicate prevention is still `Object+0x98`.

## 7. Current Rust Implementation Status

Static scan only:

| Rust surface | Observed status | Rust-facing implication |
|---|---|---|
| `src/sim/world/mod.rs` | `live_object_order: Vec<u64>`, `register_live_object` uses `Vec::contains`, `unregister_live_object` uses `retain`, and `live_object_order_snapshot` appends sorted missing entity IDs. | Future active-list APIs must cover these non-Reveal registration paths too; sorted fallback is not native active-vector behavior. |
| `src/sim/game_entity.rs` | No audited native `Object+0x98` style membership byte or `Techno+0x82` exact open-transport field was found in this slice. | OpenTopped passengers need contained/in-open-transport state distinct from storage and active membership. |
| `src/sim/passenger.rs` | Handles `open_topped` / `open_transport_weapon` weapon selection surfaces and uses `live_object_order_snapshot`, but this slice did not find native-order `SetInOpenTransport` register semantics. | BFRT passenger entry must not be treated as transport-only weapon override. |
| `src/rules/object_type.rs` | Parses `open_topped` and `open_transport_weapon`; no `HasSpotlight` parser found. | BuildingLight activation gate is missing. |
| `src/rules/weapon_type.rs` | Parses `is_sonic`; `is_magbeam` was not confirmed in the static scan output. | WaveClass construction needs exact weapon flag mapping before implementation. |
| `src/map/lighting.rs` | Point-light ambience exists; no `BuildingLightClass` object lifecycle found. | `HasSpotlight` should not be folded into point-light ambience. |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Direct xref set to `FUN_0055BAA0` | verified | `get_bulk_xrefs(0x0055BAA0)` | none for named static xrefs |
| Direct xref set to `FUN_0055BAE0` | verified | `get_bulk_xrefs(0x0055BAE0)` | none for named static xrefs |
| `0x00435B01` BuildingLight constructor caller | verified | decompile `0x00435820`; assembly `0x00435AF0..0x00435B01` | none for identity/activation |
| `0x00437070` BuildingLight reveal wrapper caller | verified | decompile `0x00437050`; assembly `0x0043705F..0x00437075`; vtable xref from prior report | none |
| BuildingLight allocation gate | verified | `BuildingClass::Unlimbo @ 0x00441169..0x00441190`; repo INI scan | packed retail map override scan deferred |
| `0x00710492` OpenTopped passenger caller | verified | decompile `0x00710470`; assembly `0x00710477..0x00710498`; caller gates `0x0051A451`, `0x0073A750` | exact vtable `+0x3D0` body out of scope |
| `0x0075F95F` WaveClass reveal caller | verified | decompile `0x0075F8B0`; assembly `0x0075F947..0x0075F95F`; ctor call `0x0075EB57` | full WaveClass AI/render out of scope |
| WaveClass stock activation | verified | Fire_At gates `0x006FF43F..0x006FF470`, `0x006FF5F5..0x006FF647`; repo INI rows | exact visual pixels out of scope |
| BuildingLight removers | verified/touched | `0x00437042`, `0x004370EE`; ambiguous `0x00435B7E` touched | bad function boundary at `0x00435B7E` unresolved by design |
| WaveClass remover | verified | assembly `0x0075F9A2..0x0075F9DD` | none for direct remove path |
| Rust surfaces | touched-not-exhausted | `rg` over `src` | implementation design belongs to parent swarm |

## 9. Open Questions - Final State

- `[RESOLVED] DNR-001 - Which xrefs are direct non-Reveal callers? -> `0x00435B01`, `0x00437070`, `0x00710492`, `0x0075F95F`.` (evidence: `get_bulk_xrefs(0x0055BAA0)`)
- `[RESOLVED] DNR-002 - Do these direct callers target the same LogicClass singleton? -> Yes, all set `ECX=0x87F778`.` (evidence: call-site assembly)
- `[RESOLVED] DNR-003 - Do these direct callers use unique-scan insertion? -> No, all push `0` before calling `FUN_0055BAA0`.` (evidence: `0x00435AF9`, `0x00437068`, `0x0071048A`, `0x0075F957`)
- `[RESOLVED] DNR-004 - What owns `0x00435B01`? -> `BuildingLightClass::Constructor`.` (evidence: decompile `0x00435820`; assembly `0x00435AF0..0x00435B01`)
- `[RESOLVED] DNR-005 - Is BuildingLight stock-active? -> Conditional only; engine path is live, but no repo stock `HasSpotlight=` assignment was found.` (evidence: `0x00441169..0x00441190`; repo INI scan)
- `[RESOLVED] DNR-006 - What owns `0x00437070`? -> `BuildingLightClass` reveal wrapper `FUN_00437050`.` (evidence: decompile `0x00437050`; assembly `0x0043705F..0x00437075`)
- `[RESOLVED] DNR-007 - Which removers pair with BuildingLight? -> `FUN_00437030` and `BuildingLightClass::Destructor @ 0x004370C0`; `0x00435B7E` is a bad-boundary BuildingLight-region xref.` (evidence: `0x00437042`, `0x004370EE`, `0x00435B7E` assembly)
- `[RESOLVED] DNR-008 - What owns `0x00710492`? -> `TechnoClass::SetInOpenTransport`.` (evidence: decompile `0x00710470`; assembly `0x00710477..0x00710498`)
- `[RESOLVED] DNR-009 - Is SetInOpenTransport stock-active? -> Yes for passengers entering OpenTopped transports; stock BFRT enables it.` (evidence: `0x0051A451..0x0051A45E`, `0x0073A750..0x0073A75D`, `rulesmd.ini:[BFRT]`)
- `[RESOLVED] DNR-010 - What owns `0x0075F95F`? -> WaveClass reveal wrapper `FUN_0075F8B0`.` (evidence: decompile `0x0075F8B0`; constructor call `0x0075EB57`)
- `[RESOLVED] DNR-011 - Is WaveClass stock-active? -> Yes for stock `IsSonic` and `IsMagBeam` weapons.` (evidence: `0x006FF43F`, `0x006FF5F5`; repo INI rows)
- `[RESOLVED] DNR-012 - Which direct remover pairs with WaveClass? -> `0x0075F9BD` in the WaveClass conceal/unreveal wrapper.` (evidence: assembly `0x0075F9A2..0x0075F9DD`)
- `[DEFERRED] DNR-013 - What exact body is passenger vtable `+0x3D0` inside SetInOpenTransport?` (category: `out-of-scope`; reason: not needed to classify helper caller; next-step-if-pursued: trace passenger hide/remove-from-cell notification virtual)
- `[DEFERRED] DNR-014 - Do extracted retail mission/map payloads set `HasSpotlight=`?` (category: `out-of-scope`; reason: repo INI was enough for standard project data; next-step-if-pursued: scan extracted MIX map/mission INI payloads)
- `[DEFERRED] DNR-015 - How should the final live-vector scheduler implement all three families?` (category: `requires-different-system-context`; reason: parent scheduler/lifecycle design owns Rust implementation; next-step-if-pursued: merge with active-vector scheduler contract)

Adversarial corner cases answered: null OpenTopped passenger is a no-op (`0x00710470`); failed BuildingLight reveal skips direct registration (`0x00435AF5..0x00435B01`, `0x00437064..0x00437070`); failed WaveClass display/visibility path skips registration (`0x0075F8B0`); all direct duplicate protection is still `Object+0x98`; missing `HasSpotlight=` leaves no BuildingLight object even though `[General]` spotlight parameters may exist.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| OpenTopped passenger entry writes passenger `Techno+0x82=1`, calls passenger virtual `+0x3D0`, then directly appends passenger to the `LogicClass` active vector. | `0x00710470`; assembly `0x00710477..0x00710498`; caller gates `0x0051A451`, `0x0073A750`; `rulesmd.ini:[BFRT] OpenTopped=yes`; Active in YR: Yes | partial/unchecked: Rust parses and uses OpenTopped weapon surfaces but does not model this exact lifecycle registration sequence | `src/sim/passenger.rs`, `src/sim/game_entity.rs`, future live active-list API | Boarding an open-topped transport must mark the passenger as in-open-transport and ensure active-list membership in native order, guarded by the membership byte equivalent. | GI enters BFRT; passenger is hidden/contained but still live for passenger logic/weapon selection, and repeated entry notifications do not double-register. | `open_topped_boarding_sets_passenger_flag_and_logic_membership_once` | Do not model BFRT fire as only a transport-level weapon override. |
| WaveClass reveal wrapper registers wave objects after display/layer submission, and stock YR creates WaveClass for `IsSonic` and `IsMagBeam`. | `0x0075F8B0..0x0075F95F`; `0x0075EB57`; Fire_At gates `0x006FF43F..0x006FF470`, `0x006FF5F5..0x006FF647`; Active in YR: Yes | missing/unchecked: no explicit WaveClass runtime found in static scan | future WaveClass visual/effect system plus live active-list API | Sonic/Magnetron wave objects need reveal/register/unregister lifecycle in the same active vector, not only render interpolation. | Fire SonicZap and MagneticBeam; each creates one WaveClass-equivalent object, registers once, ticks, then unregisters on conceal/destruction. | `waveclass_reveal_registers_logic_object_for_sonic_and_magbeam` | Do not treat WaveClass type 0/sonic as TS-dead or purely static render. |
| BuildingLight constructor and reveal wrapper directly register a spotlight object after successful `ObjectClass::Reveal`; activation requires `HasSpotlight=yes`. | `0x00435B01`, `0x00437070`; allocation gate `0x00441169..0x00441190`; Active in YR: Conditional | missing: no `HasSpotlight` parse or separate BuildingLight object path found | `src/rules/object_type.rs`, `src/map/lighting.rs`, future building lifecycle and spotlight render path | Mods/maps with `HasSpotlight=yes` must allocate a separate BuildingLight object at the native unlimbo point and register it after successful reveal. | Custom rules fixture with one `HasSpotlight=yes` building creates one spotlight object, active membership appends once, and building teardown unregisters it. | `buildinglight_has_spotlight_reveal_registers_after_successful_unlimbo` | Do not infer spotlights from point-light fields or building names. |
| Direct removers are class-specific lifecycle complements: BuildingLight conceal/destructor and WaveClass conceal wrapper call `FUN_0055BAE0`. | `0x00437042`, `0x004370EE`, `0x0075F9BD`; Active in YR: Conditional/Yes by family | missing/unchecked: Rust unregister is `retain` by stable ID and lacks native membership-byte gate | `src/sim/world/mod.rs`, future active-list unregister | Non-Reveal direct registration families also need paired unregister with native stable-compaction semantics from the remover report. | Register A, direct-register wave B, unregister B through wave conceal; active order remains A with no sorted fallback or duplicate cleanup side effects. | `direct_registered_wave_unregisters_via_active_vector_remover` | Do not collapse these into ordinary ObjectClass conceal only; direct wrappers call the remover explicitly. |

## 11. Negative Facts / Do Not Do

- Do not say only `ObjectClass::Reveal` registers objects into the active vector. Active in YR: Yes/Conditional; evidence: direct calls at `0x00435B01`, `0x00437070`, `0x00710492`, `0x0075F95F`.
- Do not classify all direct non-Reveal calls as irrelevant or map-load-only. Active in YR: Yes for OpenTopped and WaveClass, Conditional for BuildingLight; evidence above.
- Do not treat BuildingLight as stock-active in unmodified repo data. Active in YR: Conditional; evidence: `HasSpotlight=` gate, no repo stock assignment.
- Do not treat BFRT/OpenTopped passenger fire as transport-only state. Active in YR: Yes; evidence: passenger `+0x82` write and direct passenger registration at `0x00710492`.
- Do not mark WaveClass `IsSonic` as TS-dead in current repo YR data. Active in YR: Yes; evidence: `rulesmd.ini` has `IsSonic=Yes` on `[SonicZap]` and `[SonicZapE]`, and `Fire_At` reads `WeaponType+0x130`.
- Do not implement these direct registrations with unique-vector scan semantics. Active in YR: Yes/Conditional; evidence: every direct registration site pushes `0`; the duplicate guard is `Object+0x98`.

## 12. Remaining Uncertainty

- Exact function boundary/name for raw BuildingLight-region remover xref `0x00435B7E` remains unresolved because Ghidra shows bad/overlapping context and swarm rules forbid mutating function boundaries.
- Passenger vtable `+0x3D0` inside `SetInOpenTransport` was not traced; the direct registration classification does not depend on it.
- Retail packed map/mission overrides outside repo INI were not scanned for `HasSpotlight=`.
- Rust live-vector scheduler design remains parent-swarm scope.

## 13. Stale Docs / Follow-up Wording

- Any wording that says "only `ObjectClass::Reveal` registers objects" should be replaced with: "`ObjectClass::Reveal` is the ordinary registration path, but BuildingLightClass, TechnoClass::SetInOpenTransport, and WaveClass also contain direct non-Reveal calls to `FUN_0055BAA0` that append to the same `LogicClass` active vector under their own lifecycle gates."
- Any wording that says "all direct non-Reveal callers are irrelevant" should be replaced with: "Direct non-Reveal callers are active for stock OpenTopped passengers and WaveClass, and conditional for BuildingLight through `HasSpotlight=yes`."
- `WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md` stale section "IsSonic is TS-LEGACY DEAD CODE IN YR" should be replaced with: "`IsSonic=` is stock-live in current repo YR data: `rulesmd.ini` sets `IsSonic=Yes` on `[SonicZap]` and `[SonicZapE]`, and `TechnoClass::Fire_At @ 0x006FF43F..0x006FF470` gates WaveClass type 0 construction on `WeaponTypeClass+0x130`. Earlier zero-match results were caused by case-sensitive matching."
- `WAVECLASS_GHIDRA_REPORT.md` wording that groups laser/radbeam under the same `+0x130` trigger should be narrowed to: "`WeaponTypeClass+0x130 IsSonic` triggers WaveClass type 0; `WeaponTypeClass+0x15C IsMagBeam` triggers WaveClass type 3. Laser/radbeam-style flags route through other beam systems per later corrections."

## Sources

- Ghidra read-only evidence:
  - `get_bulk_xrefs(0x0055BAA0)` -> `0x005F5040`, `0x0075F95F`, `0x00435B01`, `0x00437070`, `0x00710492`, data `0x007E1918`
  - `get_bulk_xrefs(0x0055BAE0)` -> `0x005F3D75`, `0x005F4DD3`, `0x00437042`, `0x004370EE`, `0x0075F9BD`, `0x00435B7E`
  - decompile `BuildingLightClass::Constructor @ 0x00435820`
  - decompile `FUN_00437050`, `FUN_00437030`, `BuildingLightClass::Destructor @ 0x004370C0`
  - decompile `TechnoClass::SetInOpenTransport @ 0x00710470`
  - decompile `FUN_0075F8B0` and constructor call context `0x0075EB57`
  - decompile `BuildingClass::Unlimbo @ 0x00440580`
  - assembly contexts for `0x00435B01`, `0x00437070`, `0x00710492`, `0x0075F95F`, `0x00437042`, `0x004370EE`, `0x0075F9BD`, `0x00435B7E`
  - assembly contexts for caller gates `0x0051A45E`, `0x0073A75D`, `0x006FF43F`, `0x006FF470`, `0x006FF5F5`, `0x006FF647`, `0x00441187`, `0x00441190`
- Prior docs:
  - `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`
  - `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
  - `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`
  - `DIRECT_NON_REVEAL_FUN_0055BAA0_CALLERS_RESWARM_20260528.md`
  - `BUILDINGLIGHT_HASSPOTLIGHT_REGISTRATION_RESWARM_20260528.md`
  - `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`
  - `WAVECLASS_GHIDRA_REPORT.md`
  - `WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md`
- INI scans:
  - `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
- Rust static scan:
  - `src/sim/world/mod.rs`
  - `src/sim/world/world_spawn.rs`
  - `src/sim/game_entity.rs`
  - `src/sim/passenger.rs`
  - `src/rules/object_type.rs`
  - `src/rules/weapon_type.rs`
  - `src/map/lighting.rs`

Status: COMPLETE.
