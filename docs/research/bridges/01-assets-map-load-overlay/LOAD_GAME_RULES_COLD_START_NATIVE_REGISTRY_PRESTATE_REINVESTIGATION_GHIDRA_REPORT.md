# Load_Game_Rules cold-start native registry prestate — Ghidra re-investigation

**Address(es):** `Main_Game` call site `0x0048CCCF`, `Init_Game @ 0x0052BA60`,
`Load_Game_Rules @ 0x0052CD70`, `RulesClass::ReadAudioVisual @ 0x006691E0`,
`RulesClass::ReadAnimations @ 0x006728B0`, `RulesClass::ReadBuildingTypes @ 0x00672660`,
`ScenarioClass::Full_Init @ 0x00686B20`
**Investigation mode:** exhaustive slice / contradiction extension
**Binary authority:** active-retail Yuri's Revenge `gamemd.exe`
**Data authority:** active-retail MIX winners extracted to `ini/rulesmd.ini`,
`ini/artmd.ini`, and `ini/aimd.ini`
**Rust changes:** none; this investigation is documentation-only
**Confidence:** high for the active stock success path, constructor-bearing call graph,
phase boundaries, startup oracle, and process-owner handoff

## 1. Verdict

Cold active YR does **not** enter the first scenario with empty Type registries, and it
does **not** run full `RULESMD.INI` through `RulesClass::Process` during startup.
Instead, `Init_Game` builds a bounded startup registry in this exact order:

```text
Load_Game_Rules:
  selected RULESMD [AudioVisual]
    -> constructor-capable Anim lookups
  optional LANGRULE.INI full Process                 // inactive in stock retail

Init_Game, immediately after Load_Game_Rules succeeds:
  selected RULESMD [Animations] master
  -> live Anim body loop using fixed global ARTMD
  -> selected RULESMD [BuildingTypes] master
  -> live Building body loop using selected RULESMD + fixed global ARTMD
```

On the active stock corpus this creates exactly **1,070 ID-bearing Types** before the
first scenario `Full_Init`. The ordered-event FNV-1a oracle is
`0x408b802af3a4cfce`. The final stock startup registries contain 613 Anims,
402 Buildings, 31 Weapons, 9 Overlays, 7 Units, 5 ParticleSystems, 2 Warheads,
and 1 Infantry; all other ID-bearing families, including HouseType, Side, and
SuperWeaponType, remain empty. One non-ID-bearing Particle also exists.

The stale claim that call `0x0052D317` processes the selected `RULESMD.INI` is false.
Assembly proves its argument is the stack-local optional `LANGRULE.INI` object created
at `0x0052D24F`. The active retail corpus contains neither `LANGRULE.INI` nor
`LANGRULEMD.INI`, and the binary contains no `LANGRULEMD.INI` filename string.

For a noncampaign scenario, `ScenarioClass::Full_Init` then performs a pre-reset pass
against this live startup registry:

```text
[Countries] master -> ReadGeneral -> live HouseType body loop -> Create_Houses
```

That pass must move the startup registry state in and return the mutated pre-reset
state. Its event vector is a new `E_multi` segment: the 1,070 startup events occurred
before the scenario's numeric-ID reset and must not be charged again to the saved
scenario prefix. The later destructive rules reset discards the pre-reset registry
objects, and the full Process stack starts from empty registries. The post-reset state,
not the compatibility startup receipt, becomes the process owner for subsequent
preview/scenario work.

## 2. Scope and prior-work gate

### 2.1 Bounded scope

In scope:

- the cold process state before `Load_Game_Rules`;
- every constructor-capable call in `Load_Game_Rules` and the immediate successful
  `Init_Game` continuation before any scenario;
- active file/layer selection for RULESMD, ARTMD, AIMD, and language rules;
- exact stock phase boundaries, ordered constructor projection, registry counts, and
  SuperWeaponType count;
- repeated invocation, failure, and campaign/noncampaign ownership consequences;
- the first noncampaign pre-reset Countries/General/House-body consumer;
- the Rust process-owner and event-drain handoff required by those facts.

Out of scope:

- re-proving every constructor site inside full `RulesClass::Process`; that already
  has an exhaustive verified report and is used only where the exact same native
  reader body is called here;
- re-proving the later House/Super, Cell resize, map Process, Tube, Overlay, child,
  or preview formula;
- AI Script/TaskForce/Team/AITrigger registries, which do not call
  `AbstractClass::AssignUniqueID`;
- TS/Firestorm selection and composition behavior;
- malformed custom INI allocation-failure emulation beyond preserving native
  no-rollback boundaries.

### 2.2 Prior reports read end to end

Before binary work, these reports were read completely:

- `RULES_PROCESS_NATIVE_ID_CONSTRUCTOR_CHRONOLOGY_REINVESTIGATION_GHIDRA_REPORT.md`;
- `FULL_INIT_AND_PREVIEW_NATIVE_ID_PREFIX_REINVESTIGATION_GHIDRA_REPORT.md`.

They correctly establish the later Process chronology and prefix skeleton, but treat
the incoming pre-reset registry state as an input. The chronology report also labels
`0x0052D317` as a base Process call. This investigation reopens and replaces only
that cold-start boundary plus the newly exposed pre-reset House-body dependency.

### 2.3 OpenTS navigation lead and exclusions

`C:\Users\enok\Documents\OpenTS\code\init.cpp:849-975` separates startup partial
rule reads from optional language `Addition`, while
`C:\Users\enok\Documents\OpenTS\code\rules.cpp:616-717` performs the later
destructive registry initialization and `rules.cpp:735-775` provides the structural
full-Addition lead. `skirmish.cpp:223-227` also led directly to the HouseTypes,
Sides, then HouseType-body pattern.

Those correspondences were used only to navigate `gamemd.exe`. OpenTS's RULES chooser,
base `RULES.INI` composition, Firestorm files, and `ARTFS`/`AIFS`/`LANGFS` paths are
TS-only and excluded. No conclusion below is closed from OpenTS.

## 3. Active-retail data selection

The active install's asset resolver establishes these winners:

| Logical file | Active source | Size | SHA-256 of extracted authority |
|---|---|---:|---|
| `RULESMD.INI` | `expandmd01.mix` | 743,215 | `3D341EF8A13A4B5AB24AF2EEF48AC94931AC2BB87D950FE3330A07E2D25672EF` |
| `ARTMD.INI` | `ra2md.mix -> localmd.mix` | 336,535 | `E1F0378394313C04EBBD5073F47785EE3E46F1B3C62D65724E8F3C310EE7BA31` |
| `AIMD.INI` | `ra2md.mix -> localmd.mix` | 138,538 | `5DF41EAEC00A78D0760EF5EECDF27D65AE1CD537309C7EAC973318266986F89D` |

The active `RULESMD.INI` shadows an older `ra2md.mix -> localmd.mix` member. There
are no loose `RULEMD*.INI` candidates in the active install, so the wildcard chooser
falls through to the canonical MIX-backed `RULESMD.INI`. Neither `LANGRULE.INI` nor
`LANGRULEMD.INI` resolves from any active MIX layer or loose file.

The binary root literals are:

| Address | Literal | Role |
|---:|---|---|
| `0x0082626C` | `RULEMD*.INI` | optional loose multi-rules chooser |
| `0x00826260` | `RULESMD.INI` | canonical selected root/fallback |
| `0x00826254` | `ARTMD.INI` | fixed global Art object |
| `0x00826228` | `LANGRULE.INI` | optional language full Process |
| `0x0082621C` | `AIMD.INI` | AI definition INI |

String search finds `LANGRULE.INI` only; there is no `LANGRULEMD.INI` string in the
active binary. Active YR does not compose RA2 `RULES.INI`, `ART.INI`, or `AI.INI`
beneath these MD roots.

## 4. Cold entry and call frequency

`Init_Game @ 0x0052BA60` is called once by `Main_Game` at the sole direct xref
`0x0048CCCF`, before the main session loop. Its only `Load_Game_Rules` call is the
unconditional call at `0x0052C95C`; `Load_Game_Rules @ 0x0052CD70` has no other
caller.

The active image initializes the located registry count globals to zero. Fresh
decompilation of `RulesClass::Constructor @ 0x00665650` shows no Type factory call.
Direct xref enumeration for the 16 ID-bearing factory families found no reachable
pre-`Load_Game_Rules` allocator in the startup corridor. The first reachable factory
site is therefore the `ReadAudioVisual` call after the root/Art loads succeed.

This is a demonstrated empty cold registry, not an inference from the missing
language file. It also proves initial SuperWeaponType count zero.

Neither `Load_Game_Rules` nor the two following master/body sweeps clears a Type
registry or resets the shared numeric-ID cursor. A hypothetical repeat before a rules
reset therefore performs case-insensitive lookups against retained state and emits
only genuinely new names. The active call graph does not repeat this sequence: its
single `Init_Game` owner runs once per process.

## 5. Exact `Load_Game_Rules` choreography

### 5.1 Selected root and fixed inputs

`Load_Game_Rules` first scans loose `RULEMD*.INI`, selects a compatible candidate if
present, otherwise inserts a canonical `RULESMD.INI` INI object, and stores the
selected INI pointer in `g_SelectedRulesINI @ 0x00887048`. The canonical fallback
object is inserted without using physical-file absence as its rejection test; the
explicit `candidate_count < 1` branch is therefore a container/allocation guard. It
then loads `ARTMD.INI` into fixed global `g_ArtINI @ 0x00887180`. An empty candidate
vector or ARTMD load failure returns false before constructor-capable partial reads.
Active stock resolves both files.

After successful Art loading, the exact partial call order is:

| Call site | Callee / input | ID-bearing effect |
|---:|---|---|
| `0x0052D0FF` | `Init_Color_Schemes_INI(selected root)` | none; color registry only |
| `0x0052D111` | `RulesClass::ReadColorAdd(selected root)` | none |
| `0x0052D121` | `[Movies]` reader with fixed Art INI | none; movie strings/objects, no Type factory |
| `0x0052D132` | `RulesClass::ReadAudioVisual(selected root)` | Anim FindOrAllocate sites |
| `0x0052D144` | `ReadMultiplayerDialogSettings(selected root)` | none |

The later `AIMD.INI` load constructs AI definition data but does not call any
ID-bearing Type factory. No Countries, General, HouseType-body, explicit Type-master,
or other full Process call receives the selected root inside `Load_Game_Rules`.

### 5.2 `ReadAudioVisual` constructor coverage

Fresh decompilation of `RulesClass::ReadAudioVisual @ 0x006691E0` accounts for ten
constructor-capable Anim lookup shapes in native call order:

```text
DropPodPuff -> VeinAttack -> Dig -> AtmosphereEntry
-> TreeFire tokens -> OnFire tokens
-> Smoke -> Smoke -> SmallFire -> LargeFire
```

All other fields in this large reader use scalar, color, sound, movie, or lookup-only
parsers. In active stock RULESMD, every constructor-capable value above is absent or
commented except `Smoke=xxxx`. The first Smoke site creates Anim `xxxx`; the second
finds the same case-insensitive stored ID and emits no event.

Thus the exact stock boundary after startup AudioVisual is:

```text
ID-bearing events = 1
events             = [Anim xxxx]
Anim registry       = 1
SuperWeapon registry = 0
```

### 5.3 What `0x0052D317` actually processes

The disputed assembly is decisive:

```text
0x0052D231  construct CCFileClass("LANGRULE.INI")
0x0052D23E  call IsAvailable_MixThenRaw
0x0052D245  jnz 0x0052D325                 // skip whole block when absent
0x0052D24F  construct stack-local CCINIClass at [esp+0x2c]
0x0052D263  load LANGRULE into that local
0x0052D274  reject load result > 1
0x0052D312  lea edx,[esp+0x2c]
0x0052D316  push edx
0x0052D317  call RulesClass::Process @ 0x00668BF0
```

The call target is indeed full `RulesClass::Process`; the corrected fact is its
argument. It receives only the local optional LANGRULE object, never
`g_SelectedRulesINI`. Since active retail has no LANGRULE, the whole block is skipped
and contributes zero stock events.

If a custom installation supplies `LANGRULE.INI`, that Process is active and must
extend the live startup registry after AudioVisual using the already verified full
Process chronology and fixed `g_ArtINI`. This is a custom-data branch, not dormant
code. `LANGRULEMD.INI` is neither requested nor reachable.

## 6. Immediate successful `Init_Game` continuation

The initial investigation boundary had to expand when disassembly showed additional
constructor work immediately after `Load_Game_Rules` returns true. Omitting it would
still leave the first scenario's incoming state wrong.

### 6.1 Animation master and live bodies

At `0x0052C9D1`, `Init_Game` calls
`RulesClass::ReadAnimations @ 0x006728B0` with `g_SelectedRulesINI`. This enumerates
the source-order `[Animations]` values and calls Anim FindOrAllocate.

The loop at `0x0052C9D6..0x0052C9FB` then:

- starts at Anim index zero;
- fetches the current Anim pointer array;
- pushes fixed `g_ArtINI @ 0x00887180`;
- calls each Anim's virtual `ReadINI` slot `+0x64`;
- reloads the live Anim count at `0x0052C9F3` after every body.

Therefore an Anim appended by an earlier Anim body receives its own body later in
this same loop. The exact constructor-capable Anim-body order is:

```text
Next -> Spawns -> TiberiumSpawnType -> BounceAnim -> ExpireAnim
-> TrailerAnim -> Warhead -> SpawnsParticle
```

The first two, Bounce, Expire, and Trailer target Anim; `TiberiumSpawnType` targets
Overlay; `Warhead` targets Warhead; `SpawnsParticle` creates a non-ID-bearing
Particle. All Art reads use the one fixed ARTMD snapshot.

Stock `[Animations]` contains 611 authored nonempty rows but only 607 distinct new
values after `xxxx`; the repeated values are `TWLT100`, `GAWEAP_1`, `GAWEAP_2`, and
`GAWEAP_A`. The phase boundaries are:

| Boundary | Cumulative ID events | Relevant new work |
|---|---:|---|
| after AudioVisual | 1 | `Anim xxxx` |
| after `[Animations]` master | 608 | 607 new Anims |
| after live Anim bodies | 612 | `Anim SMOKEY2`, `Warhead HE`, `Overlay TIB2_01`, `Warhead TankOGas` |

The four Anim-body ID events occur in exactly the order shown. The bodies also create
non-ID Particle `VirusCloud1`. At this boundary the ID-bearing registries contain
609 Anims, 1 Overlay, and 2 Warheads.

### 6.2 Building master and live bodies

At `0x0052CA09`, `Init_Game` calls
`RulesClass::ReadBuildingTypes @ 0x00672660` with `g_SelectedRulesINI`. It enumerates
the source-order `[BuildingTypes]` values and calls Building FindOrAllocate.

The loop at `0x0052CA0E..0x0052CA35` then:

- starts at Building index zero;
- passes `g_SelectedRulesINI` to each virtual `ReadINI` slot `+0x64`;
- uses the same fixed global Art object through inherited Techno/Building readers;
- reloads the live Building count at `0x0052CA2D` after every body.

An appended Building would therefore receive a same-loop body. A Building body that
creates another family does **not** trigger that other family's body loop here; only
the Building registry is walked. Constructor order within each body is the verified
inherited Object/Techno/Building order from the Process chronology report, including
the Building Art `ToOverlay` lookup.

Stock `[BuildingTypes]` has 403 authored nonempty rows and 402 distinct new values;
`NAPSYA` is repeated. The master boundary is 1,014 cumulative events. Stock Building
bodies then add 56 ID-bearing events:

| Family | New events in Building bodies |
|---|---:|
| Weapon | 31 |
| Overlay | 8 |
| Unit | 7 |
| ParticleSystem | 5 |
| Anim | 4 |
| Infantry | 1 |
| **Total** | **56** |

The final startup boundary is therefore 1,070 ID-bearing events.

The exact ordered 56-event Building-body suffix is:

```text
01 Anim gtpowexp                         GAPOWR.Explosion#6
02 ParticleSystem SmallGreySSys          GAREFN.RefinerySmokeParticleSystem
03 Unit CMIN                             GAREFN.FreeUnit
04 Unit AMCV                             GACNST.UndeploysInto
05 ParticleSystem SparkSys               GAPILE.DamageParticleSystems#1
06 ParticleSystem BigGreySmokeSys        GAPILE.DamageParticleSystems#3
07 Overlay GASAND                        GASAND(Image=GASAND).ToOverlay
08 Anim tstlexp                          NAPOWR.Explosion#6
09 Overlay GAWALL                        GAWALL(Image=GAWALL).ToOverlay
10 Unit HARV                             NAREFN.FreeUnit
11 Overlay NAWALL                        NAWALL(Image=NAWALL).ToOverlay
12 Weapon Vulcan                         NALASR.Primary
13 Weapon RedEye2                        NASAM.Primary
14 Unit SMCV                             NACNST.UndeploysInto
15 ParticleSystem LGSparkSys             GALITE.DamageParticleSystems#2
16 Weapon CoilBolt                       TESLA.Primary
17 Weapon OPCoilBolt                     TESLA.Secondary
18 Weapon PrismShot                      ATESLA.Primary
19 Weapon PrismSupport                   ATESLA.Secondary
20 Weapon BarrelExplosion                AMMOCRAT.DeathWeapon
21 Weapon GrandCannonWeapon              GTGCAN.Primary
22 Weapon NukePayload                    NANRCT.DeathWeapon
23 Weapon Vulcan2                        GAPILL.Primary
24 Weapon FlakWeapon                     NAFLAK.Primary
25 Weapon HoverMissile                   CAOUTP.Primary
26 Weapon OilExplosion                   CAOILD.DeathWeapon
27 ParticleSystem SmallGreySmokeSys      NACLON.DamageParticleSystems#2
28 Weapon EiffelBolt                     CAPARS01.Primary
29 Weapon MayanPrism                     MAYAN.Primary
30 Anim CAWA15DM                         CAWASH15.DestroyAnim#1
31 Overlay CAFNCB                        CAFNCB(Image=CAFNCB).ToOverlay
32 Overlay CAFNCW                        CAFNCW(Image=CAFNCW).ToOverlay
33 Anim CACH06DM                         CACHIG06.DestroyAnim#1
34 Overlay CAKRMW                        CAKRMW(Image=CAKRMW).ToOverlay
35 Overlay CAFNCP                        CAFNCP(Image=CAFNCP).ToOverlay
36 Unit PCV                              YACNST.UndeploysInto
37 Overlay GAFWLL                        GAFWLL(Image=GAFWLL).ToOverlay
38 Weapon AGGattling                     YAGGUN.Weapon1
39 Weapon AGGattlingE                    YAGGUN.EliteWeapon1
40 Weapon AAGattCann                     YAGGUN.Weapon2
41 Weapon AAGattlingE                    YAGGUN.EliteWeapon2
42 Weapon AGGattling2                    YAGGUN.Weapon3
43 Weapon AGGattling2E                   YAGGUN.EliteWeapon3
44 Weapon AAGattCann2                    YAGGUN.Weapon4
45 Weapon AAGattling2E                   YAGGUN.EliteWeapon4
46 Weapon AGGattling3                    YAGGUN.Weapon5
47 Weapon AGGattling3E                   YAGGUN.EliteWeapon5
48 Weapon AAGattCann3                    YAGGUN.Weapon6
49 Weapon AAGattling3E                   YAGGUN.EliteWeapon6
50 Weapon MultipleMindControlTower       YAPSYT.Primary
51 Weapon BlimpBombEffect                YAPPPT.DeathWeapon
52 Unit ROBO                             GAROBO.PowersUnit
53 Weapon 20mmRapid                      YAREFN.Primary
54 Weapon 20mmRapidE                     YAREFN.ElitePrimary
55 Unit SMIN                             YAREFN.UndeploysInto
56 Infantry SLAV                         YAREFN.Enslaves
```

## 7. Exact stock startup oracle

Two independent data projections reproduced the binary choreography using source
order, live count reloads, case-insensitive family lookup, 24-byte native stored IDs,
native caller string capacities, comma tokenization, and none-sentinel behavior.
Neither projection used final registry lengths as a substitute for event order.

Hash convention: start with 64-bit FNV-1a offset basis; for every successful
first-new ID-bearing event fold `family_code || ASCII(native_stored_id) || 0xFF`.
Family codes are the existing Rust oracle mapping House=0, Side=1, Overlay=2,
Super=3, Warhead=4, Smudge=5, Terrain=6, Building=7, Unit=8, Aircraft=9,
Infantry=10, Anim=11, VoxelAnim=12, ParticleSystem=13, Weapon=14, Bullet=15.

| Boundary | Cumulative ID-bearing events |
|---|---:|
| root AudioVisual sites | 1 |
| root Animations master | 608 |
| live Anim ARTMD bodies | 612 |
| root BuildingTypes master | 1,014 |
| live Building bodies | **1,070** |

```text
startup trace FNV-1a = 0x408b802af3a4cfce
first events          = Anim xxxx, Anim TWLT100, Anim ELECTRO
startup Super count   = 0
```

Final registry counts:

| Family | Count |
|---|---:|
| Anim | 613 |
| Building | 402 |
| Weapon | 31 |
| Overlay | 9 |
| Unit | 7 |
| ParticleSystem | 5 |
| Warhead | 2 |
| Infantry | 1 |
| House, Side, SuperWeapon, Smudge, Terrain, Aircraft, VoxelAnim, Bullet | 0 each |
| Particle | 1, non-ID-bearing |

All stock identifiers exercised here are at most 24 bytes. Long-name truncation and
post-truncation duplicate behavior remain required production semantics but are not
exercised by this stock oracle. ARTMD has two `PARABOMB` sections; the first is empty
and the later body contains only Rate/loop keys, so either native section-resolution
choice contributes no constructor event to this oracle.

## 8. First noncampaign consumer and reset boundary

Fresh `ScenarioClass::Full_Init @ 0x00686B20` inspection corrects another inherited
shorthand. Under `g_GameMode != 0`, the pre-reset block is:

```text
0x0068741D  FUN_006722F0(selected root)       // [Countries] master
0x0068742E  RulesClass::ReadGeneral(root)
0x00687433..0x0068745C
            live HouseType loop, vslot +0x64, root input, count reloaded
0x0068745E  ScenarioClass::Create_Houses
```

`FUN_006722F0` reads `[Countries]` values with the 32-byte caller buffer and calls
HouseType FindOrAllocate in source order. `HouseTypeClass::ReadINI @ 0x00511850`
contains the exact constructor sequence:

```text
VeteranInfantry tokens -> Infantry
VeteranUnits tokens     -> Unit
VeteranAircraft tokens  -> Aircraft
Side scalar             -> Side
```

The House loop reloads its live count. House bodies do not contain a HouseType factory,
so they cannot extend that same family, but the live-loop contract must still be
preserved rather than replaced with a frozen snapshot.

This corrects `E_multi = Countries/General` to:

```text
E_multi = actual first-new ID-bearing events from
          Countries -> General -> live HouseType bodies
          against the retained startup registry state
```

The stock `E_multi` event oracle is recorded in section 12. Its essential ownership
boundary is fixed: startup's 1,070-event vector is drained before the scenario prefix
begins, while the registry state is retained for duplicate suppression and House-body
lookup.

The subsequent outer rules reset destructs the old Type objects. It does not make the
already consumed native IDs reappear. The inner full Process starts with empty Type
registries and constructs the authoritative post-reset state. Consequently:

- pre-reset House/Side/etc. objects are disposable but their Assign events remain in
  `E_multi`;
- startup Assign events are older than the scenario's reset and are not part of
  `E_multi`;
- post-reset Process state must replace the pre-reset registry owner;
- later passes must continue from that post-reset owner until the next explicit reset.

Campaign uses the same shared cold `Init_Game` startup state, but it does not execute
the noncampaign Countries/General/House-body block. Its later campaign prefix remains
the separately verified branch in the Full_Init report.

## 9. Failure and repeat semantics

There is no transactional rollback of Type registry allocations in this startup
path:

- the explicit empty-candidate-vector guard, or ARTMD load failure: return false before
  AudioVisual; registry state remains as it entered. The canonical fallback object is
  inserted without using physical-file absence as that guard;
- optional LANGRULE absent: skip its block with the AudioVisual state retained;
- malformed/failed LANGRULE load after AudioVisual: the function can return false,
  but earlier partial allocations are not rolled back;
- allocation failure inside a factory does not emit a successful event; a safe Rust
  implementation may fail the load instead of preserving a degraded partial object,
  but may not invent an event.

The successful active stock path reaches both immediate Init_Game master/body sweeps.
A failed `Load_Game_Rules` returns before them. A direct repeat without a reset retains
all family registries; duplicate names are lookups. No active caller performs such a
repeat, so this is a custom harness/lifecycle rule rather than an ordinary stock loop.

## 10. Current Rust ownership mismatch

The current Rust app has no process-lifetime native rules/registry owner:

- `initialize()` calls `load_rules_ini` for frontend rules;
- `load_rules_ini` immediately calls `into_rules_discarding_native_receipt`, retaining
  only `RuleSet`;
- `AppState` owns `ProcessAssets`, but no `NativeRulesRegistryState` equivalent;
- the fresh-scenario descriptor carries no rules construction receipt/state;
- interactive match loading destructures `LoadedRules` into
  `_native_type_construction_trace` and drops it;
- headless loading independently drops the same receipt;
- `SimRuntime` receives only the compatibility `RuleSet`.

The recently implemented compatibility loader also performs the full Rules stack at
startup. That receipt cannot be promoted into the native owner: native cold startup is
the partial 1,070-event choreography above, not full RULESMD Process. The RuleSet may
remain a downstream compatibility projection, but its discarded trace must remain
explicitly non-authoritative.

## 11. Open Questions Log — resolved state

| ID | Question | Resolution |
|---|---|---|
| Q01 | Exact `Load_Game_Rules` boundary? | entry `0x0052CD70`; success returns after AIMD load; false paths identified in sections 5/9. |
| Q02 | Caller and frequency? | sole call `Init_Game+0xEFC @ 0x0052C95C`; `Init_Game` sole call from `Main_Game` call site `0x0048CCCF`; once per process. |
| Q03 | Full Process on selected RULESMD at startup? | no. No selected-root Process call exists in `Load_Game_Rules`. |
| Q04 | Call at `0x0052D317`? | `RulesClass::Process` on stack-local optional `LANGRULE.INI`. |
| Q05 | Decompiler misassociation? | target was right; inherited argument attribution was wrong. Assembly `lea/push [esp+0x2c]` proves the local. |
| Q06 | LANGRULE or LANGRULEMD? | only `LANGRULE.INI`. |
| Q07 | Language gate? | `CCFileClass::IsAvailable_MixThenRaw`; then local load must not return `>1`. No game-mode/language-index gate around the filename. |
| Q08 | Active filename literal? | `LANGRULE.INI @ 0x00826228`. |
| Q09 | Stock language file present? | no active loose or MIX member for either spelling. |
| Q10 | ARTMD role? | fixed global Art loaded before partial readers; used by startup Anim and inherited Building bodies; not a Rules layer. |
| Q11 | Other constructor-capable startup callees? | AudioVisual plus post-return Anim/Building master/body sweeps; no Countries/General until Full_Init. |
| Q12 | Initial registries? | all 16 ID-bearing family counts zero at cold image entry. |
| Q13 | Static/global pre-allocation? | none found; Rules constructor has no Type factory and xref walk finds no earlier reachable allocator. |
| Q14 | Exact cold prefix and Super count? | 1,070 events, hash `0x408b802af3a4cfce`, Super count 0. |
| Q15 | Empty prestate demonstrated? | yes, image bytes + constructor + startup call-graph/factory-xref evidence. |
| Q16 | Repeat behavior? | no reset; retained family state, only first-new names emit. Active caller does not repeat. |
| Q17 | Startup registry/counter reset? | none in Load_Game_Rules or its immediate Init_Game continuation. |
| Q18 | Failure rollback? | none; paths preserve whatever succeeded before return. Active stock success reaches all sweeps. |
| Q19 | Campaign difference? | shared startup identical; campaign excludes only later noncampaign prepass. |
| Q20 | TS/dormant language behavior? | optional YR LANGRULE is active custom-data code; TS chooser/Firestorm composition excluded. |
| Q21 | Rust retained state? | live startup registry state; startup event vector drained at scenario reset; fixed Art and selected root available to exact consumers. |
| Q22 | Prepass input/output? | move in startup registry, run Countries -> General -> live House bodies, return mutated state plus E_multi-only events and current Super count. |
| Q23 | Compatibility discard sites? | frontend `into_rules_discarding_native_receipt` and interactive/headless underscore traces are non-authoritative and must not remain the only owner. |
| Q24 | Empty versus hidden prefix discriminator? | five phase counts, first events, full hash, final family counts, and Super zero. |
| Q25 | Are startup master body loops frozen? | no; both reload their own family's live count after every body. |
| Q26 | Do Building-created other families get startup bodies? | no; only the Building family is walked after the Building master. |
| Q27 | Does AIMD construct Type IDs here? | no; its definition registries do not call AssignUniqueID. |
| Q28 | Does the Movie reader construct Type IDs? | no; it creates movie/string table entries only. |
| Q29 | Is startup state part of scenario E_multi? | state yes, historical event vector no. |
| Q30 | Can the first noncampaign pass omit House bodies? | no; Full_Init explicitly walks every live HouseType before Create_Houses. |
| Q31 | Which state survives the later reset? | consumed numeric-ID history survives; pre-reset Type objects do not; post-reset Process registry becomes owner. |

## 12. Stock noncampaign pre-reset oracle

An independent active-data projection started from the verified 1,070-event startup
registry and ran the exact native prepass. It produced:

```text
E_multi events             = 51
E_multi-only FNV-1a        = 0x45b8b69cd005937d
startup + E event count    = 1,121
startup + E cumulative FNV = 0x026859d66424f324
Countries / General / House bodies = 14 / 32 / 5
SuperWeaponType count      = 0
```

The E-only hash restarts at the FNV offset basis. The cumulative hash continues the
startup hash state through E. There are 112 duplicate-suppressed existing lookups and
zero none-sentinel attempts.

Exact ordered `E_multi` events:

```text
001 House Americans                 Countries.0
002 House Alliance                  Countries.1
003 House French                    Countries.2
004 House Germans                   Countries.3
005 House British                   Countries.4
006 House Africans                  Countries.5
007 House Arabs                     Countries.6
008 House Confederation             Countries.7
009 House Russians                  Countries.8
010 House YuriCountry               Countries.9
011 House GDI                       Countries.10
012 House Nod                       Countries.11
013 House Neutral                   Countries.12
014 House Special                   Countries.13
015 VoxelAnim GASTANK               General.BarrelDebris#1
016 VoxelAnim PIECE                 General.BarrelDebris#2
017 Anim D                          General.MetallicDebris#15
018 Anim WCLBOLT2                   General.WeatherConBolts#2
019 Warhead DominatorWH             General.DominatorWarhead
020 Unit VISC_LRG                   General.LargeVisceroid
021 Unit VISC_SML                   General.SmallVisceroid
022 VoxelAnim TIRE                  General.TireVoxelDebris
023 Aircraft ORCA                   General.PadAircraft#1
024 Aircraft BEAG                   General.PadAircraft#2
025 Infantry SNIPE                  General.SecretInfantry#1
026 Infantry TERROR                 General.SecretInfantry#2
027 Infantry DESO                   General.SecretInfantry#3
028 Infantry YURI                   General.SecretInfantry#4
029 Unit TNKD                       General.SecretUnits#1
030 Unit TTNK                       General.SecretUnits#2
031 Unit DTRUCK                     General.SecretUnits#3
032 Infantry E1                     General.AlliedDisguise
033 Infantry E2                     General.SovietDisguise
034 Infantry INIT                   General.ThirdDisguise
035 Infantry ENGINEER               General.Engineer
036 Infantry CTECH                  General.Technician
037 Infantry BRUTE                  General.AnimToInfantry#1
038 Warhead IonWH                   General.LightningWarhead
039 Aircraft V3ROCKET               General.V3RocketType
040 Aircraft DMISL                  General.DMislType
041 Aircraft CMISL                  General.CMislType
042 Terrain VEINTREE                General.VeinholeTypeClass
043 Terrain TREE01                  General.DefaultMirageDisguises#1
044 Terrain TREE02                  General.DefaultMirageDisguises#2
045 Terrain TREE03                  General.DefaultMirageDisguises#3
046 Terrain TREE04                  General.DefaultMirageDisguises#4
047 Side GDI                        Americans.Side
048 Side Nod                        Africans.Side
049 Side ThirdSide                  YuriCountry.Side
050 Side Civilian                   Neutral.Side
051 Side Mutant                     Special.Side
```

Final registry counts after E:

| Family | Count |
|---|---:|
| Anim | 615 |
| Building | 402 |
| Weapon | 31 |
| House | 14 |
| Unit | 12 |
| Infantry | 11 |
| Overlay | 9 |
| Aircraft | 5 |
| Side | 5 |
| Terrain | 5 |
| ParticleSystem | 5 |
| Warhead | 4 |
| VoxelAnim | 3 |
| SuperWeapon, Smudge, Bullet | 0 each |
| Particle | 1, non-ID-bearing |

House count remains 14 throughout the live body loop. Stock defines no
`VeteranInfantry`, `VeteranUnits`, or `VeteranAircraft` key in those House sections;
only five first-new Side identities allocate. Earlier House bodies create `GDI` and
`Nod`, suppressing those lookups in later bodies. `ThirdSide`, `Civilian`, and `Mutant`
similarly suppress repeats after their first allocating House. The other suppressed
lookups are existing General references and same-prepass repeats; notably `GASTANK`
and `PIECE` allocate from `BarrelDebris` and repeat under later Voxel-debris keys.

## 13. Constructor-capable coverage ledger

| Corridor | Input | Constructor-capable? | Covered result |
|---|---|---:|---|
| cold globals / Rules ctor | image/defaults | possible hypothesis | zero counts; no factory call |
| root chooser | `RULEMD*.INI` / `RULESMD.INI` | no | selected-root identity only |
| Colors | selected root | no Type | excluded |
| ColorAdd | selected root | no Type | excluded |
| Movies | fixed Art | no Type | excluded |
| AudioVisual | selected root | yes, Anim | all ten shapes; stock one event |
| MultiplayerDialogSettings | selected root | no Type | excluded |
| optional LANGRULE | local INI | yes, full Process | active custom branch; stock absent |
| AIMD load | `AIMD.INI` | no ID-bearing Type | excluded from prefix |
| Animations master | selected root | yes, Anim | 607 stock new events after `xxxx` |
| live Anim bodies | fixed Art | yes | four ID + one non-ID Particle stock additions |
| BuildingTypes master | selected root | yes, Building | 402 stock new events |
| live Building bodies | root + fixed Art | yes | 56 stock new events; family breakdown verified |
| later Init_Game tail | startup globals | audited for Type factories | no additional covered Type factory before return |
| Full_Init Countries | selected root | yes, House | included in E_multi |
| Full_Init General | selected root | yes, mixed | included in E_multi |
| live HouseType bodies | selected root | yes, Infantry/Unit/Aircraft/Side | included in E_multi |
| Create_Houses | current HouseTypes | instance Assigns | downstream House/Super prefix owner, not a Type event |
| destructive rules reset | live registries | destruction | consumes no new Type ID; replaces state boundary |

No direct or transitive constructor-capable call in the bounded startup corridor is
unaccounted for.

## 14. Adversarial checks

1. **Could `0x0052D317` still receive RULESMD through a hidden global?** No. The
   immediate `lea/push` passes the local `CCINIClass` at `[esp+0x2c]`, constructed
   only inside the LANGRULE availability branch.
2. **Could missing LANGRULE imply an empty startup registry?** No. AudioVisual runs
   before the language branch, and the successful caller then performs Anim and
   Building master/body sweeps.
3. **Could the post-return sweeps be scenario-only or dormant?** No. They are the
   success fallthrough of the sole `Init_Game` startup call, before Init_Game returns
   and before any ScenarioClass::Full_Init caller.
4. **Could final registry counts substitute for event order?** No. duplicate master
   rows and live same-family body growth make chronology observable. The full ordered
   FNV oracle independently constrains it.
5. **Could the Movie, AIMD, Particle, or sound readers secretly spend native IDs?**
   No. Fresh callees show movie/string, AI-definition, non-ID Particle, or Voc lookups;
   none reaches an ID-bearing Type constructor/AssignUniqueID.
6. **Could Building-body-created Anims receive bodies during startup?** No. The Anim
   live loop has already finished; the later live loop reloads only Building count.
7. **Could the first Full_Init safely reconstruct startup state from the 1,070 events?**
   Not as architecture. Native owns live family registries, not only a log; duplicate
   suppression and body lookup require moving the state itself.
8. **Could startup's 1,070 events be added to E_multi?** No. the scenario resets the
   numeric cursor after startup. They determine lookup state but are outside the
   post-reset saved-prefix arithmetic.
9. **Could campaign use an empty owner because it skips E_multi?** No. campaign shares
   the same Init_Game startup registry; only the later Full_Init branch differs.
10. **Could OpenTS base layering justify `RULES.INI -> RULESMD.INI` composition?** No.
    active YR literals, data resolution, and call arguments establish standalone MD
    roots.

## 15. Zero-add pass

After closing the initial question ledger, a second pass rechecked:

- every direct call between successful ARTMD load and `Load_Game_Rules` return;
- every constructor-capable call between the caller's success edge and the end of
  `Init_Game`;
- all factory xrefs reachable before the first AudioVisual site;
- optional filename strings and active asset winners;
- live-count reloads in both startup body loops;
- the noncampaign Full_Init corridor through `Create_Houses`;
- current Rust receipt discard and ownership sites.

This pass added no new startup constructor stage. It did correct the prepass scope by
adding the live HouseType body loop and exposed the required separation between
retained registry state and drained startup event history.

## 16. Implementation handoff

### 16.1 Required ownership and choreography

1. Add an exact startup choreography over the existing constructor processor:
   `AudioVisual(root) -> Animations master(root) -> live Anim bodies(fixed Art) ->
   BuildingTypes master(root) -> live Building bodies(root,fixed Art)`. Run optional
   LANGRULE full Process between AudioVisual and the caller sweeps when present.
2. Introduce one move-only process owner for `NativeRulesRegistryState`; it must not be
   reconstructed from compatibility `RuleSet` or final counts.
3. At scenario prefix reset, drain/discard startup events but retain the registry state.
4. Run exact noncampaign prepass
   `Countries -> General -> live HouseType bodies`, returning E_multi-only events and
   the current SuperWeaponType count.
5. At the destructive rules reset, replace pre-reset registry state with default-empty
   state; then run the verified full source stack. That post-reset state becomes the
   process owner for preview, scenario load, and subsequent passes.
6. Thread the same owner through interactive, headless, and preview paths. Remove the
   underscore/discard seams as authoritative production behavior; compatibility-only
   discards must be named and tested as such.

### 16.2 Acceptance tests

- stock startup boundaries are exactly `1`, `608`, `612`, `1,014`, `1,070`;
- stock startup trace hash is `0x408b802af3a4cfce`;
- stock startup first events are `Anim xxxx`, `Anim TWLT100`, `Anim ELECTRO`;
- final startup family counts and Super zero match section 7;
- Anim body additions occur exactly
  `Anim SMOKEY2 -> Warhead HE -> Overlay TIB2_01 -> Warhead TankOGas`;
- two stock Smoke reads emit one construction;
- both startup same-family loops reload live length;
- Building-created other-family objects do not receive bodies in startup;
- E_multi excludes the 1,070 startup events but receives the retained lookup state;
- stock E_multi is 51 events with E-only hash `0x45b8b69cd005937d` and cumulative
  startup-plus-E hash `0x026859d66424f324`;
- House bodies construct in Infantry -> Unit -> Aircraft -> Side order;
- destructive reset drops pre-reset objects, not already spent numeric-ID history;
- a second scenario continues from the post-reset owner rather than replaying cold
  startup or using the compatibility frontend receipt;
- interactive and headless paths retain the same native state contract.

### 16.3 Correction bundle for inherited docs/design

- Replace “`0x0052D317` processes base RULESMD” with “`0x0052D317` processes the
  availability-gated stack-local `LANGRULE.INI`; stock retail skips it.”
- Replace “cold startup registry is empty/unknown” with the exact 1,070-event startup
  choreography and oracle in this report.
- Replace “`E_multi` is Countries/General” with
  “Countries -> General -> live HouseType bodies against retained startup state.”
- Preserve standalone MD roots; reject TS/RA2 base-layer composition.

## 17. Residuals

- Runtime allocation exhaustion and malformed custom files are not stock-corpus
  exercised. Rust may fail safely, but must preserve successful-event/no-rollback
  semantics up to the failure.
- Long stored-ID truncation remains governed by the full Process chronology report;
  active stock startup names do not exercise it.
