# Direct Non-Reveal FUN_0055BAA0 Callers - Re-swarm Research Report

**Address(es):** `FUN_0055BAA0 @ 0x0055BAA0`; non-`ObjectClass::Reveal` call sites `0x00435B01`, `0x00437070`, `0x00710492`, `0x0075F95F`; matching removers `0x00435B7E`, `0x00437042`, `0x004370EE`, `0x0075F9BD`.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Map the four specified direct non-`ObjectClass::Reveal` registration callers and needed remover siblings to concrete class/function contexts and standard YR activation conditions.
**Non-Scope:** Helper internals, `ObjectClass::Reveal` branch ordering, save/load active-vector reconstruction, and full AI/render semantics of BuildingLight, OpenTopped passengers, or WaveClass.
**Confidence:** High for caller identity, call shape, and YR activity classification from live Ghidra decompile/assembly plus repo INI checks; Medium for Rust deltas because this slot did static scans only.
**Active in YR:** Conditional overall. Two caller families are stock-live in standard YR content (`TechnoClass::SetInOpenTransport`, `WaveClass` reveal); `BuildingLightClass` callers are live engine paths but require `HasSpotlight=yes`, which repo stock INI does not set.

## Working Notes Gate

- **Target question:** Which concrete classes/functions own direct non-`ObjectClass::Reveal` calls to `FUN_0055BAA0` at `0x00435B01`, `0x00437070`, `0x00710492`, and `0x0075F95F`, and when are they active in standard YR?
- **Non-goals:** Do not re-prove `FUN_0055BAA0` internals, do not implement Rust, do not expand into full BuildingLight/OpenTopped/WaveClass mechanics, and do not mutate Ghidra.
- **Evidence needed to mark COMPLETE:** For each specified call site, cite Ghidra decompile plus call-site assembly/xref evidence, name the concrete class/function owner, classify Active in YR Yes/No/Conditional with INI/default or caller evidence, and provide Rust-facing handoff items.
- **Stop conditions:** Stop after all four registration sites and needed remover siblings are mapped, every open question is resolved/deferred, and at least one implementation handoff item exists.

## 1. Overview

The non-`ObjectClass::Reveal` registration callers are not miscellaneous load-order sources. They are class-specific lifecycle wrappers:

1. `BuildingLightClass` constructor and virtual reveal wrapper register the spotlight object after successful `ObjectClass::Reveal`.
2. `TechnoClass::SetInOpenTransport` registers a passenger object after setting `TechnoClass+0x82` and calling the passenger hide/notification virtual.
3. `WaveClass` reveal wrapper registers the wave object after successful placement/display submission.

All verified calls pass unique flag `0` to `FUN_0055BAA0` and target the same `LogicClass` singleton at `0x87F778`. The helper's `Object+0x98` membership mechanics remain inherited from `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md` and were not re-proved here.

## 2. Class Layout / Key Offsets

| Offset / global | Class | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `ObjectClass+0x81` | Object-derived | InLimbo gate read by WaveClass reveal wrapper | `0x0075F8C1..0x0075F8C9` | Yes for WaveClass reveal conditions |
| `ObjectClass+0x98` | Object-derived | Logic membership flag set/cleared by helper/remover | prior helper report; all callers pass object ptr to `0x0055BAA0`/`0x0055BAE0` | Yes |
| `TechnoClass+0x82` | Techno-derived | InOpenToppedTransport passenger flag, set before direct logic registration | decompile `TechnoClass__SetInOpenTransport @ 0x00710470`; assembly `0x0071047D..0x00710492` | Yes, conditional on entering OpenTopped transport |
| `TechnoTypeClass+0x5E4` | TechnoType | `OpenTopped=` vehicle flag gating `SetInOpenTransport` callers | `InfantryClass__PerCellProcess @ 0x0051A451..0x0051A45E`; `UnitClass__PerCellProcess @ 0x0073A750..0x0073A75D`; `rulesmd.ini:[BFRT] OpenTopped=yes` | Yes in stock YR for BFRT |
| `BuildingTypeClass+0x154B` | BuildingType | `HasSpotlight=` allocation gate for `BuildingLightClass` | `BuildingClass__Unlimbo @ 0x00441171..0x00441190`; prior BuildingLight report; repo INI no `HasSpotlight=` assignment | Conditional; live parser, no stock repo activation found |
| `BuildingClass+0x600` | BuildingClass | Stored `BuildingLightClass*` spotlight pointer | `0x00441187` constructor call, `0x00441190` store | Conditional |
| `WeaponTypeClass+0x130` | WeaponType | `IsSonic=` triggers WaveClass type 0 | `TechnoClass::Fire_At @ 0x006FF43F..0x006FF470`; `rulesmd.ini:[SonicZap]/[SonicZapE] IsSonic=Yes` | Yes in stock YR |
| `WeaponTypeClass+0x15C` | WeaponType | `IsMagBeam=` triggers WaveClass type 3 | `TechnoClass::Fire_At @ 0x006FF5F5..0x006FF647`; `rulesmd.ini:[MagneticBeam] IsMagBeam=yes` | Yes in stock YR |
| `DAT_00A8EC3C/48` | WaveClass globals | Global WaveClass list independent from LogicClass registration | `WaveClass__Constructor @ 0x0075E950` / prior WaveClass docs | Yes for WaveClass lifecycle |
| `0x87F778` | LogicClass singleton | Target vector for all four direct registration calls | assembly at `0x00435AFC`, `0x0043706B`, `0x0071048D`, `0x0075F95A` | Yes |

## 3. Core Logic

### 3.1 Caller Map

| Site | Concrete owner | Direct call shape | Gameplay condition | Active in YR |
|---|---|---|---|---|
| `0x00435B01` | `BuildingLightClass__Constructor @ 0x00435820` | After `ObjectClass__Reveal(&coords, 0)` returns nonzero, pushes `0`, pushes `ESI` (`this`), sets `ECX=0x87F778`, calls `FUN_0055BAA0` | Constructor is called by `BuildingClass::Unlimbo` only when `BuildingType+0x154B HasSpotlight` is nonzero; repo stock INI has no `HasSpotlight=` assignment | Conditional; live mod/map/rules path, no stock repo activation found |
| `0x00437070` | `BuildingLightClass` virtual reveal wrapper `FUN_00437050` | Calls `ObjectClass__Reveal(param_2, param_3)`; if success, pushes `0`, `this`, `ECX=0x87F778`, calls helper | Used through `BuildingLightClass` vtable data xref `0x007E3BA8`; same spotlight allocation condition as above | Conditional; live for BuildingLight objects |
| `0x00710492` | `TechnoClass__SetInOpenTransport @ 0x00710470` | If passenger pointer non-null: write byte `+0x82=1`, call vtable `+0x3D0`, then push `0`, passenger, `ECX=0x87F778`, call helper | Called from Infantry and Unit per-cell passenger-entry paths only after target type `OpenTopped` byte `+0x5E4` is true | Yes; stock `[BFRT] OpenTopped=yes`, passengers with `OpenTransportWeapon=` exist |
| `0x0075F95F` | WaveClass reveal wrapper `FUN_0075F8B0` | After game/in-limbo/map-editor/display/reveal gates, display submit if layer valid, then push `0`, `this`, `ECX=0x87F778`, call helper | Called from `WaveClass__Constructor @ 0x0075E950` after geometry/list init; WaveClass constructed by `TechnoClass::Fire_At` for `IsSonic` type 0 and `IsMagBeam` type 3 | Yes; stock `[SonicZap]`, `[SonicZapE]`, and `[MagneticBeam]` set these gates |

### 3.2 Matching remover callers

| Site | Concrete owner | Direct remover shape | Active in YR |
|---|---|---|---|
| `0x00435B7E` | `BuildingLightClass__Destructor @ 0x004370C0` as decompiled from stale/overlapping function context in constructor-area xref list | Removes after successful `ObjectClass__Conceal`, then removes from `DAT_008B4194` global BuildingLight vector and calls `ObjectClass__Destructor` | Conditional on `BuildingLightClass` existence |
| `0x00437042` | `BuildingLightClass` virtual conceal wrapper `FUN_00437030` | Calls `ObjectClass__Conceal`; if success, calls `FUN_0055BAE0(this)` through `ECX=0x87F778` | Conditional on `BuildingLightClass` existence |
| `0x004370EE` | `BuildingLightClass__Destructor @ 0x004370C0` | Destructor installs BuildingLight vtables, calls `ObjectClass__Conceal`, then calls remover on success before removing from global BuildingLight vector | Conditional on `BuildingLightClass` existence |
| `0x0075F9BD` | WaveClass conceal/unreveal wrapper immediately following `FUN_0075F8B0` | Removes from display/layer vector, calls `FUN_0055BAE0(this)`, then calls vtable `+0x11C`, sets `+0x81=1`, clears byte at `+0x80` | Yes for WaveClass fade/destruction lifecycle; WaveClass is stock-live |

Note on `0x00435B7E`: Ghidra's function boundary/xref context overlaps constructor-area bytes; the unambiguous high-confidence BuildingLight destructor remover is `0x004370EE` in `BuildingLightClass__Destructor @ 0x004370C0`. This report does not create or repair function boundaries because the swarm rules require read-only Ghidra use.

### 3.3 BuildingLightClass activation

`BuildingClass__Unlimbo @ 0x00441171..0x00441190` checks the building type byte at `+0x154B`, allocates `0xE8`, calls `BuildingLightClass__Constructor @ 0x00435820`, and stores the result at `BuildingClass+0x600`. The constructor first calls `ObjectClass__Constructor`, installs the BuildingLight vtables, appends to the BuildingLight global vector, computes initial endpoint coordinates, calls `ObjectClass__Reveal`, and only then directly registers the object with `FUN_0055BAA0`.

Active in YR: Conditional. The parser/runtime path is live, but repo `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, and `ini/artmd.ini` contain no `HasSpotlight=` assignments. A mod/map/rules override can activate it.

### 3.4 OpenTopped passenger activation

`TechnoClass__SetInOpenTransport @ 0x00710470` has one null guard. For non-null passengers it writes `TechnoClass+0x82 = 1`, calls vtable slot `+0x3D0`, and registers the passenger in the live LogicClass vector. Caller assembly proves both found ordinary entry sites are gated by the target type's `OpenTopped` byte:

- `InfantryClass__PerCellProcess @ 0x0051A451..0x0051A45E`: loads `target.GetType()+0x5E4`, tests it, calls `SetInOpenTransport` only when nonzero.
- `UnitClass__PerCellProcess @ 0x0073A750..0x0073A75D`: same gate and call shape.

Active in YR: Yes, conditional per transport type. Stock `rulesmd.ini:[BFRT]` sets `OpenTopped=yes` and `Passengers=5`; several stock infantry set `OpenTransportWeapon=0/1`, so the mechanism is standard YR gameplay.

### 3.5 WaveClass activation

WaveClass construction is stock-live. `TechnoClass::Fire_At` constructs WaveClass type 0 when `WeaponType+0x130 IsSonic` is set (`0x006FF43F..0x006FF470`) and type 3 when `WeaponType+0x15C IsMagBeam` is set (`0x006FF5F5..0x006FF647`). Repo INI confirms:

- `rulesmd.ini:[SonicZap]` and `[SonicZapE]` set `IsSonic=Yes`.
- `rulesmd.ini:[MagneticBeam]` includes `IsMagBeam=yes`.

The WaveClass constructor `0x0075E950` registers in its global wave vector, initializes geometry, then calls `FUN_0075F8B0(this+0xB4, 0)`. That reveal wrapper runs only when `g_GameActive != 0`, `Object+0x81` is nonzero, byte `+0x74` is zero, map editor restrictions pass or are bypassed, `ObjectClass` display/visibility virtuals succeed, and display layer lookup is valid. On success it submits the object to display and calls `FUN_0055BAA0`.

Active in YR: Yes for Sonic Tank and Magnetron visuals. WaveClass is visual-only per existing WaveClass AI report; this slot only maps its logic-vector registration.

## 4. INI Keys

| Key | Scope | Default / stock value | Effect on this slice | Evidence | Active in YR |
|---|---|---|---|---|---|
| `HasSpotlight=` | BuildingType | default false; no repo stock assignment found | Enables BuildingLight allocation and therefore `0x00435B01`/`0x00437070` paths | `0x00441171..0x00441190`; BuildingLight report; repo `rg` | Conditional |
| `OpenTopped=` | TechnoType vehicle | stock `[BFRT] OpenTopped=yes` | Gates `SetInOpenTransport` caller reachability | `0x0051A451..0x0051A45E`, `0x0073A750..0x0073A75D`; `rulesmd.ini:6932` | Yes |
| `OpenTransportWeapon=` | TechnoType passenger | stock infantry rows set `0` or `1`; default `-1` | Makes `+0x82` passenger fire behavior useful after registration | prior IFV/OpenTopped report; repo `rulesmd.ini` rows | Yes |
| `IsSonic=` | WeaponType | stock `[SonicZap]` and `[SonicZapE]` set `Yes` | Triggers WaveClass type 0 construction | `0x006FF43F..0x006FF470`; `rulesmd.ini:23688`, `25107` | Yes |
| `IsMagBeam=` | WeaponType | stock `MagneticBeam` rows set `yes` | Triggers WaveClass type 3 construction | `0x006FF5F5..0x006FF647`; `rulesmd.ini` matches | Yes |

## 5. Integration Points

| Integration point | Status | Evidence | Active in YR |
|---|---|---|---|
| Ghidra xrefs to `FUN_0055BAA0` | Verified direct xrefs are `0x005F5040`, `0x0075F95F`, `0x00435B01`, `0x00437070`, `0x00710492` plus data `0x007E1918` | bulk xrefs to `0x0055BAA0` | Yes/Conditional per caller |
| Ghidra xrefs to `FUN_0055BAE0` | Verified remover xrefs are `0x005F3D75`, `0x005F4DD3`, `0x00437042`, `0x004370EE`, `0x0075F9BD`, `0x00435B7E` | bulk xrefs to `0x0055BAE0` | Yes/Conditional per caller |
| BuildingLight allocation | `BuildingClass::Unlimbo` constructs `BuildingLightClass` only if `HasSpotlight` byte set | `0x00441171..0x00441190` | Conditional |
| OpenTopped passenger entry | Infantry/Unit per-cell process gate on `OpenTopped` then call `SetInOpenTransport` | `0x0051A451..0x0051A45E`; `0x0073A750..0x0073A75D` | Yes for BFRT |
| WaveClass construction | `TechnoClass::Fire_At` constructs WaveClass for `IsSonic`/`IsMagBeam` | `0x006FF43F..0x006FF470`; `0x006FF5F5..0x006FF647` | Yes |

## 6. Current Rust Implementation Status

Static scan only:

| Rust surface | Observed status | Rust-facing implication |
|---|---|---|
| `src/sim/entity_store.rs`, `src/sim/world/mod.rs` | No native-style separate live LogicClass vector or object-local `+0x98` equivalent found in prior/live-vector reports | Direct non-Reveal callers also need to append to the same future live-logic membership list, not only ordinary reveal |
| `src/sim/passenger.rs` | Parses/uses `open_topped` and `open_transport_weapon` in passenger boarding/combat override areas, but current model appears to assign weapon override state rather than model `+0x82` plus direct live-logic registration | OpenTopped entry should set passenger-contained flag and logic membership in native order |
| `src/rules/object_type.rs` | `open_topped` and `open_transport_weapon` are parsed; no `HasSpotlight` parse found | BuildingLight allocation gate missing |
| `src/map/lighting.rs` | Point-light system exists; no directional `BuildingLightClass` object path | Do not fold BuildingLight into point-light ambience |
| `src/sim` / `src/render` scan | No explicit WaveClass runtime object found | WaveClass visual lifecycle and its live logic membership are missing/unchecked |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00435B01` caller | verified | decompile `0x00435820`; assembly `0x00435AF0..0x00435B0A` | none for identity/activation |
| `0x00437070` caller | verified | decompile `0x00437050`; assembly `0x00437050..0x00437078`; xref data `0x007E3BA8` | none for identity |
| `0x00710492` caller | verified | decompile `0x00710470`; assembly `0x00710477..0x00710498`; caller gates `0x0051A451`, `0x0073A750` | exact vtable `+0x3D0` body out of scope |
| `0x0075F95F` caller | verified | decompile `0x0075F8B0`; assembly `0x0075F8B0..0x0075F968`; WaveClass ctor call `0x0075EB57` | full WaveClass visual semantics out of scope |
| BuildingLight remover `0x00437042` | verified | decompile `0x00437030`; assembly `0x00437030..0x0043704E` | none |
| BuildingLight remover `0x004370EE` | verified | decompile `0x004370C0`; assembly `0x004370C0..0x0043710D` | none |
| Ambiguous stale xref `0x00435B7E` | touched-not-exhausted | bulk xref/context shows overlap near BuildingLight constructor/destructor area | Ghidra boundary repair would be needed, but mutation is disallowed |
| WaveClass remover `0x0075F9BD` | verified | assembly `0x0075F9A2..0x0075F9DD`; paired with reveal wrapper | none |
| Standard YR activity filter | verified | repo INI checks for `OpenTopped`, `IsSonic`, `IsMagBeam`, `HasSpotlight` | retail map override scan for `HasSpotlight=` outside repo deferred |
| Current Rust touchpoints | touched-not-exhausted | `rg` over `src/` | implementation audit not performed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-DNRC-001 - Which class owns 0x00435B01? -> BuildingLightClass__Constructor calls ObjectClass__Reveal then FUN_0055BAA0.` (evidence: `0x00435820`; assembly `0x00435AF0..0x00435B0A`)
- `[RESOLVED] OQ-DNRC-002 - Is BuildingLight stock-active in standard repo YR data? -> Conditional only; parser/runtime live, but no repo stock HasSpotlight assignment was found.` (evidence: `0x00441171..0x00441190`; repo INI `rg`)
- `[RESOLVED] OQ-DNRC-003 - Which class owns 0x00437070? -> BuildingLightClass virtual reveal wrapper FUN_00437050.` (evidence: decompile `0x00437050`; vtable data xref `0x007E3BA8`)
- `[RESOLVED] OQ-DNRC-004 - Which removers pair with BuildingLight? -> FUN_00437030 virtual conceal and BuildingLightClass__Destructor @ 0x004370C0 call FUN_0055BAE0 after successful ObjectClass__Conceal.` (evidence: `0x00437042`, `0x004370EE`)
- `[RESOLVED] OQ-DNRC-005 - Which class owns 0x00710492? -> TechnoClass__SetInOpenTransport.` (evidence: decompile `0x00710470`; assembly `0x00710477..0x00710498`)
- `[RESOLVED] OQ-DNRC-006 - Is SetInOpenTransport stock-active? -> Yes, when infantry/unit enters OpenTopped transport; BFRT sets OpenTopped=yes.` (evidence: `0x0051A451..0x0051A45E`, `0x0073A750..0x0073A75D`, `rulesmd.ini:[BFRT]`)
- `[RESOLVED] OQ-DNRC-007 - Which class owns 0x0075F95F? -> WaveClass reveal wrapper FUN_0075F8B0.` (evidence: decompile `0x0075F8B0`; ctor call from `0x0075EB57`)
- `[RESOLVED] OQ-DNRC-008 - Is WaveClass stock-active? -> Yes for IsSonic and IsMagBeam weapons in stock rulesmd.ini.` (evidence: `0x006FF43F..0x006FF470`, `0x006FF5F5..0x006FF647`, repo INI lines)
- `[RESOLVED] OQ-DNRC-009 - Do the direct callers pass unique_scan_flag nonzero? -> No, all mapped call sites push 0 before calling FUN_0055BAA0.` (evidence: assembly `0x00435AF9`, `0x00437068`, `0x0071048A`, `0x0075F957`)
- `[RESOLVED] OQ-DNRC-010 - Do the direct callers target the same LogicClass singleton as Reveal? -> Yes, all set ECX=0x87F778.` (evidence: assembly at all four call sites)
- `[DEFERRED] OQ-DNRC-011 - What exact body is vtable +0x3D0 in SetInOpenTransport?` (category: `out-of-scope`; reason: not needed to map direct helper caller; next-step-if-pursued: trace passenger hide/remove-from-cell virtual)
- `[DEFERRED] OQ-DNRC-012 - Are retail mission maps outside repo setting HasSpotlight?` (category: `out-of-scope`; reason: repo INI was checked, extracted retail map archive scan was not requested; next-step-if-pursued: scan extracted `.map/.yrm/.mpr` files)
- `[DEFERRED] OQ-DNRC-013 - How should all three families integrate with final Rust live-vector scheduler?` (category: `requires-different-system-context`; reason: implementation design belongs to parent lifecycle swarm; next-step-if-pursued: fold this report into live LogicClass scheduler contract)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| OpenTopped passenger entry sets `Techno+0x82=1`, calls passenger virtual `+0x3D0`, then appends passenger to LogicClass via `FUN_0055BAA0`; callers are gated by target `OpenTopped`. | `0x00710470`; call sites `0x0051A451..0x0051A45E`, `0x0073A750..0x0073A75D`; `rulesmd.ini:[BFRT] OpenTopped=yes` | likely mismatch/partial: Rust has `open_topped`/`open_transport_weapon` handling but no native `+0x82` plus logic-list membership sequence | `src/sim/passenger.rs`, `src/sim/game_entity.rs`, future live logic scheduler | Boarding BFRT must mark passenger as in-open-transport and ensure it remains/enters live logic membership after hide/removal from map, in native order | Load GI into BFRT; passenger is hidden/contained but still eligible for own AI/weapon selection from cargo, and duplicate entry does not double-register | `open_topped_boarding_sets_passenger_flag_and_logic_membership_once` | Do not model BFRT fire as only a transport-level weapon override; gamemd marks/registers the passenger object |
| WaveClass reveal wrapper appends wave objects to the same LogicClass vector after display/layer submission; WaveClass is stock-live for `IsSonic` and `IsMagBeam`. | `0x0075F8B0..0x0075F95F`; Fire_At gates `0x006FF43F`, `0x006FF5F5`; `rulesmd.ini` Sonic/MagBeam keys | missing/unchecked: no explicit WaveClass object runtime found | future WaveClass visual/effect system plus live logic scheduler | Sonic/Magnetron wave visual objects must have reveal/register lifecycle and tick through native live-object ordering, not only render-time interpolation | Fire Sonic Tank and Magnetron; each creates a WaveClass-equivalent object that registers once, ticks, and unregisters on fade/destruction | `waveclass_reveal_registers_logic_object_for_sonic_and_magbeam` | Do not treat WaveClass as TS-dead or purely static render artifact |
| BuildingLight constructor and virtual reveal wrapper directly register the spotlight object only after successful `ObjectClass::Reveal`; activation requires `HasSpotlight=yes`. | `0x00435B01`, `0x00437070`; allocation gate `0x00441171..0x00441190`; no repo stock `HasSpotlight=` | missing: no `HasSpotlight` parse/spotlight object path found | `src/rules/object_type.rs`, `src/map/lighting.rs`, future BuildingLight object/render path | If mods/maps set `HasSpotlight=yes`, allocate a separate BuildingLight object and register it after successful reveal; keep it separate from point-light ambience | Custom rules fixture with one `HasSpotlight=yes` building creates one spotlight object, registers it once, and removing building unregisters it | `buildinglight_has_spotlight_reveal_registers_after_successful_unlimbo` | Do not infer spotlights from building names or LightVisibility; do not implement as PointLight ambience |

### Negative Facts / Do Not Do

- Do not classify every non-Reveal direct `FUN_0055BAA0` caller as an ordinary map-load order source. Evidence: callers are BuildingLight, OpenTopped passenger, and WaveClass lifecycle wrappers.
- Do not treat `BuildingLightClass` as stock-active merely because the engine path exists. Evidence: `HasSpotlight=` gate at `0x00441171..0x00441190`, no repo stock assignment found.
- Do not treat BFRT/OpenTopped passenger fire as transport-only state. Evidence: `TechnoClass__SetInOpenTransport` writes passenger `+0x82` and registers the passenger object at `0x00710492`.
- Do not mark WaveClass type 0/sonic as TS-dead in current project docs. Evidence: repo `rulesmd.ini` has `IsSonic=Yes` for `[SonicZap]` and `[SonicZapE]`; Fire_At reads `weapon+0x130` at `0x006FF43F`.
- Do not implement these direct registration paths with unique-vector scanning as the primary duplicate guard. Evidence: all four mapped callers push `0`; duplicate protection remains object-local `+0x98` from the helper report.

### Remaining Uncertainty

- The xref at `0x00435B7E` appears inside an overlapping/misbounded BuildingLight function region; `0x004370EE` is the clean BuildingLight destructor remover. Boundary repair was not performed due to read-only rules.
- Exact effect of the vtable `+0x3D0` call inside `SetInOpenTransport` was not traced.
- Retail map overrides outside repo INI were not scanned for `HasSpotlight=`.
- Rust live-vector design remains parent-swarm scope.

### Stale Docs / Follow-up Docs

- `docs/research/WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md` section "CRITICAL: IsSonic is TS-LEGACY DEAD CODE IN YR" is stale. Replacement wording: "`IsSonic=` is stock-live in current repo YR data: `rulesmd.ini` sets `IsSonic=Yes` on `[SonicZap]` and `[SonicZapE]`, and `TechnoClass::Fire_At @ 0x006FF43F..0x006FF470` gates WaveClass type 0 construction on `WeaponTypeClass+0x130`. The earlier zero-match result came from a case-sensitive grep; the INI parser accepts capitalized `Yes`."
- `docs/research/WAVECLASS_GHIDRA_REPORT.md` section 8 overstates WaveClass triggers by grouping laser/radbeam flags under `+0x130`. Replacement wording: "`WeaponTypeClass+0x130 IsSonic` triggers WaveClass type 0; `WeaponTypeClass+0x15C IsMagBeam` triggers WaveClass type 3. `IsLaser`, `DiskLaser`, `IsBigLaser`, and `IsRadBeam` route to other beam classes per `WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md`."

## Sources

- Ghidra read-only decompile/assembly:
  - `BuildingLightClass__Constructor @ 0x00435820`, call site `0x00435B01`
  - `BuildingLightClass` conceal/reveal wrappers `0x00437030`, `0x00437050`
  - `BuildingLightClass__Destructor @ 0x004370C0`, remover `0x004370EE`
  - `BuildingClass__Unlimbo @ 0x00440580`, allocation call `0x00441187`
  - `TechnoClass__SetInOpenTransport @ 0x00710470`, call site `0x00710492`
  - `InfantryClass__PerCellProcess @ 0x0051A45E`, `UnitClass__PerCellProcess @ 0x0073A75D`
  - `WaveClass__Constructor @ 0x0075E950`, call into reveal wrapper `0x0075EB57`
  - WaveClass reveal wrapper `0x0075F8B0`, call site `0x0075F95F`, remover `0x0075F9BD`
  - `TechnoClass::Fire_At` WaveClass gates `0x006FF43F..0x006FF470`, `0x006FF5F5..0x006FF647`
- Research docs:
  - `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
  - `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`
  - `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md`
  - `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`
  - `WAVECLASS_GHIDRA_REPORT.md`
  - `WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md`
  - `combat/systems/sonic.md`
- Repo INI:
  - `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
- Rust static scan:
  - `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, `src/sim/passenger.rs`, `src/sim/game_entity.rs`, `src/rules/object_type.rs`, `src/map/lighting.rs`
