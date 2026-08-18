# GSI-04.03B Merged-Rules Overlay-Registry Authority Design

Date: 2026-07-24
Status: **APPROVED — AUTONOMOUS EVIDENCE GATE PASSED**
Contract:
`docs/contracts/2026-07-24-gsi-04-03b-merged-rules-overlay-registry-authority-implementation-contract.md`

## Goal

Make the successfully merged rules source the single transient construction
authority for the match `RuleSet`, production `OverlayTypeRegistry`, and overlay
atlas inputs, without touching parser/type ownership or changing merge
semantics.

## Architecture Context

`src/app_init_helpers.rs::load_rules_ini` currently owns the four-layer rules
load:

```text
rules.ini
  -> merge rulesmd.ini
  -> merge selected MP mode payload
  -> apply bounded map value overrides
  -> RuleSet::from_ini
```

`RuleSet::from_ini` records the input `IniFile::content_hash()` as
`source_ini_hash`. The loader then discards the `IniFile` and returns only the
parsed `RuleSet`.

The real match loader in `src/app_init.rs::load_map_from_initial` calls that
merged loader, but later performs an independent raw lookup:

```text
rulesmd.ini OR rules.ini
  -> OverlayTypeRegistry::from_ini
  -> ResolvedTerrainGrid
  -> overlay grid / ore queues
  -> build_overlay_atlas_from_map
```

That raw `IniFile` also reaches `src/app_skirmish.rs`, where another
`OverlayTypeRegistry` is parsed for wall connectivity and the same source is
used for overlay and bridge asset resolution. No `app_skirmish.rs` interface
change is needed; it already accepts `&IniFile`.

Startup in `src/app.rs` is different: no map or mode is selected, and the
caller only needs a `RuleSet`. A compatibility wrapper can preserve that
surface.

Native YR has one load-time rules/type authority. `ScenarioClass::Full_Init @
0x00686B20` invokes `RulesClass::Process @ 0x006686C0` for reset/main rules,
then later sends the active map to the same section/type reader `0x00668BF0`.
Rust-native ownership need not imitate global native objects, but both parsed
views must originate from the same ordered source.

## Operator Questions Resolved Autonomously

- **Target:** the production match-load path, not every helper that happens to
  read raw rules.
- **Behavior:** source coherence and native load ordering; no tick-time
  behavior is added.
- **Compatibility:** preserve `load_rules_ini` for startup callers.
- **Error policy:** a merged `IniFile` is exposed only when `RuleSet::from_ini`
  succeeded from that exact content.
- **Mode ordering:** preserve the current Rust base/YR/mode/map order without
  claiming the existing mode reports prove full native rules application.
- **Approval method:** the operator specification replaces interactive
  questions/selection with evidence, independent challenge, repair, and
  explicit autonomous approval.
- **Retail correction:** `EB2`'s `[SpazWH] Tiberium=yes` is a warhead setting,
  not an overlay flag. It is excluded from the design evidence.

## Impact Analysis

Owned implementation paths:

- `src/app_init_helpers.rs`
  - add the paired transient result;
  - factor the existing merge sequence into a private pure composer;
  - preserve the compatibility wrapper;
  - add synthetic and retail-backed tests.
- `src/app_init.rs`
  - consume the paired result once;
  - remove the raw rules reload;
  - pass the retained merged source to both registry construction and the
    existing atlas builder.

Read-only dependencies:

- `src/map/overlay_types.rs`;
- `src/app_skirmish.rs`;
- `src/rules/ini_parser.rs`;
- `src/rules/ruleset.rs`;
- `src/app.rs`.

Explicitly excluded independent readers:

- startup trackbar/options loaders;
- random-map preview/RMG inputs;
- cooperative roster helpers;
- other UI data readers that do not construct the match
  `RuleSet`/overlay-registry pair.

Blast radius:

- Map and selected-mode overlay section values begin reaching resolved terrain,
  overlay grids, ore classification, wall connectivity, overlay atlas lookup,
  and bridge overlay asset lookup through the source those consumers already
  accept.
- Base-only and stock-equivalent map values retain the same parsed result.
- `RuleSet` itself is unchanged.
- No state layout, snapshot, replay, hash format, RNG cursor, scheduler order,
  tick timing, render algorithm, or public crate API changes.
- The retained merged `IniFile` lives only during map construction and is
  dropped with the rest of the loader locals.

Ownership risk:

- The preserved dirty damage-authority worktree owns `src/rules/*` and several
  sim/world paths. This feature must not edit, stage, format, or otherwise
  claim those files.
- `src/map/overlay_types.rs` is reserved for the dependent GSI-04.03A feature
  and stays read-only here.

## Tiny-Detail Ledger

- Base `rules.ini` is required and read first. Missing/unparseable base returns
  no load result. [Rust: `app_init_helpers.rs::load_rules_ini`]
- `rulesmd.ini` is an optional YR patch and merges over base when parseable.
  [Rust loader; native main rules layering]
- Current Rust merges the selected mode payload after rulesmd. Full native
  mode rules application/order remains `UNCHECKED`; this feature preserves the
  Rust order. [Rust loader; bounded MPModes reports]
- The map pass follows the mode and uses current
  `merge_rules_overrides` value semantics. [GHIDRA `0x006686C0`,
  `0x00668BF0`; Rust loader]
- Empty map values preserve the prior field. [doc:
  `RULESCLASS_GHIDRA_REPORT.md` §9.3; Rust `merge_rules_overrides`]
- Current Rust deliberately excludes map numbered registry lists/new registry
  allocations. This remains a named residual, not a hidden parity claim.
  [doc: `RULESCLASS_GHIDRA_REPORT.md` §9.3]
- `RuleSet::from_ini` is called once on the final merged source.
  [Rust loader]
- The paired source invariant is exact:
  `rules.source_ini_hash() == merged_ini.content_hash()`. [Rust:
  `ruleset.rs:2131,2247`; `ini_parser.rs:410`]
- The merged source represents rules state only. Later
  `RuleSet::merge_art_data` does not mutate or invalidate its rules-source
  hash. [Rust `app_init.rs`]
- `OverlayTypeRegistry::from_ini` reads the ordered `[OverlayTypes]` registry
  and every named type's flag/value section from its supplied source. [Rust:
  `overlay_types.rs:169..278`]
- The same source also reaches `build_overlay_atlas_from_map`, which reparses a
  registry and performs overlay/bridge asset resolution. [Rust:
  `app_init.rs:941`; `app_skirmish.rs:1838..2057`]
- Art remains separately merged as `art.ini < artmd.ini` and is passed to
  registry construction exactly as before. [Rust `load_art_ini`,
  `app_init.rs`]
- The change consumes no RNG and writes no simulation state independently.
  Downstream differences are the intended consequence of corrected rule
  inputs. [Architecture trace]
- `EB2` / `XEB2` is not an overlay-registry proof: `SpazWH` is a warhead
  record. [retail `EB2.mmx` raw section context]
- Stock selected `MountMoras.map` contains no overlay registry/type sections.
  Its real `GAYARD.TechLevel=11` overrides merged retail value 4, so it proves
  the map rules pass is active while providing an overlay no-op preservation
  fixture. [retail census]
- The MPModes reports verify archive-backed mode payloads and common mode
  fields, but explicitly defer full rules application. [mode reports]
- Stock selected `XEB2.MAP` defines map-only warhead `SpazWH`. The bounded Rust
  merge drops new type sections; this stock-live allocation residual is
  protected and excluded from this source-unification feature. [retail
  `XEB2.MAP`; corrected `RULESCLASS_GHIDRA_REPORT.md` §9.3]
- Native per-type readers are stateful across successive INI passes.
  `Tiberium=yes` can force `Armor=Special` and, when land is zero,
  `Land=Tiberium`; a later pass does not inherently reverse those writes.
  Parsing one final composed INI is therefore not a proof of generic native
  reread equivalence. This feature fixes routing only. [doc:
  `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md` lines 156–163]

## Approaches Considered

### A. Transient paired load result — chosen

Shape:

```text
LoadedRules {
    rules: RuleSet,
    merged_ini: IniFile,
}
```

`load_rules_with_merged_ini` loads/composes once, parses `RuleSet`, proves the
hash invariant, and returns both owners. Existing `load_rules_ini` delegates
and returns only `rules` for startup.

Architectural fit:

- Extends the current app-layer owner instead of moving rules data into sim or
  duplicating it inside `RuleSet`.
- Keeps the existing startup API.
- Makes successful parse and exact source lifetime one result.
- Lets `app_init` destructure once and eliminate the alternate raw source.

Tiny-detail coverage:

- The private composer owns base/YR/mode/map order and current empty/registry
  policy.
- The paired constructor owns parse/failure behavior and hash identity.
- `app_init` owns one-time destructuring and downstream routing.
- Existing art flow stays after rules construction.
- No cache or persistent second authority is introduced.

Trade-offs:

- Carries the parsed `IniFile` until map construction completes.
- A caller could theoretically ignore `merged_ini`; removing the raw reload,
  keeping the result transient, and tests around the production callsite make
  that regression visible.

### B. Return only a merged `IniFile`; parse at callers — rejected

How it works:

- A helper returns the four-layer `IniFile`.
- Each caller decides when/how to run `RuleSet::from_ini`.

Architectural issue:

- Centralizes merge ordering but separates successful parsing from the source
  contract.
- Repeats error/logging policy at callers.
- Makes it easier for a caller to parse `RuleSet` from one source and construct
  a registry from another.

Parity assessment:

- Can be correct, but the invariant becomes callsite convention rather than a
  type-level result. It is weaker than A without reducing owned scope.

### C. Duplicate the merge inside `app_init` — rejected

How it works:

- Keep `load_rules_ini` unchanged.
- Independently reload and re-merge base, YR, mode, and map for the registry.

Parity failure:

- Two archive reads, parse branches, warnings, and merge implementations can
  diverge.
- Any later merge-order or map-policy correction can update one path and leave
  the other stale, reproducing the exact defect.
- Equality tests would detect some drift but would not remove duplicate
  authority.

Verdict: architectural and parity DRIFT.

### D. Store `OverlayTypeRegistry` inside `RuleSet` — rejected

How it works:

- Parse and retain the overlay registry as a new `RuleSet` field.

Why rejected:

- Requires edits in the protected dirty `ruleset.rs`.
- Couples art-enriched overlay presentation data to a gameplay rules aggregate.
- Broadens construction, clone, hash, and downstream interfaces when the
  registry is already an app/map-layer object.
- Does not help the atlas functions that still need the merged `IniFile`.

## Chosen Approach

Choose A.

The private pure composer receives already parsed layers and owns only the
existing order. The archive-facing function loads base/optional YR data, calls
the composer, parses `RuleSet`, checks exact content-hash identity, and returns
the transient pair. The compatibility wrapper delegates.

`load_map_from_initial` calls the paired function once:

```text
LoadedRules
  ├─ rules ───────────────> existing RuleSet/art/sim flow
  └─ merged_ini
       ├──────────────────> OverlayTypeRegistry::from_ini
       └──────────────────> build_overlay_atlas_from_map
```

No caller recomposes or reparses rules.

## Design

### Components

`LoadedRules` (app-init helper boundary)

- owns `RuleSet`;
- owns the exact merged `IniFile`;
- constructs the pair through one private `from_merged_ini` boundary that
  parses `RuleSet` and checks source-hash identity;
- remains crate-private and transient.

`compose_rules_layers` (private pure helper)

- consumes base `IniFile`;
- applies optional parsed YR patch, mode, then map;
- reuses `IniFile::merge` and `merge_rules_overrides`;
- contains no archive access or parsing.

`load_rules_with_merged_ini`

- preserves current archive lookup/log/failure behavior;
- calls the composer;
- delegates parsing to `LoadedRules::from_merged_ini`;
- returns the paired result only on success.

`load_rules_ini`

- compatibility wrapper returning only `.rules`.

### Interfaces / Contracts

- The pair's two fields always satisfy source hash equality.
- Neither field is optional after successful return.
- `app_init` must not perform another `rules.ini`/`rulesmd.ini` lookup after
  accepting the pair.
- Art enrichment may mutate `RuleSet` after the pair is destructured; it does
  not redefine `merged_ini`.
- `app_skirmish` continues to receive `&IniFile` without interface changes.

### Data Flow

```text
AssetManager
  -> parse base
  -> parse optional YR patch
  -> compose(base, YR, mode, map)
  -> RuleSet::from_ini(&merged)
  -> LoadedRules { rules, merged }
  -> app map construction
```

### Error Handling

- Missing/unparseable base: return `None`, as now.
- Missing YR patch: continue with base, as now.
- Unparseable YR patch: skip it, as now.
- Mode/map inputs are already parsed; retain their current merge behavior.
- `RuleSet::from_ini` failure: log and return `None`; do not expose a merged
  source with no accepted rules authority.
- Hash mismatch is a construction invariant violation; use a debug assertion
  and an unconditional test, not a recoverable fallback to raw rules.

### Testing Strategy

1. Synthetic composer-and-pair test:
   - base registers `TIB01` and Riparius;
   - YR/mode/map set distinguishable values;
   - map attempts registry-list replacement but overrides existing per-type
     values;
   - pass the composed source through the private pair constructor;
   - construct the registry from the pair's source and assert order, list
     exclusion, final flag/image, and source hash identity.
2. Retail-backed loader test:
   - use installed `rules.ini`/`rulesmd.ini` through `AssetManager`;
   - build the raw comparison using exactly `rulesmd.ini` when present,
     otherwise `rules.ini`;
   - assert ID `0` is `GASAND` with raw `Tiberium=false`;
   - apply `[GASAND] Tiberium=yes` through the real loader;
   - assert merged ID/name/flag, raw-versus-merged difference, and exact pair
     hash.
   - treat the flag flip as a routing oracle only; do not claim Land/Armor
     reread parity.
3. Retail `MountMoras.map` no-op test:
   - require source `expandmd01.mix`, 103,241 bytes, and no overlay registry or
     type sections;
   - compare no-map and map paired results;
   - assert `GAYARD.TechLevel` changes 4 to 11, pair hashes differ and match
     their own sources, while exact `GASAND`/`TIB01` registry identity and
     flags are preserved.
4. Existing merge-order/map tests remain green.
5. Existing overlay parser tests remain green.
6. Impact search proves the raw reload is gone from the match loader.
7. Format only the two edited Rust files.
8. Run final `cargo check -q`.

`MountMoras.map` proves the stock map rules pass through `GAYARD`, not an
overlay override; the synthetic retail-backed flag flip is the non-vacuous
overlay routing oracle.

## Architectural Decisions

- Follow the existing app-layer rules loader rather than create a new rules
  service.
- Preserve the startup wrapper and narrow production callsite churn.
- Use source hashing already present in `RuleSet`; do not add fields to
  protected rules code.
- Keep overlay registry as derived map/app data.
- Keep map allocation drift explicit and untouched.
- Introduce no dependency or module.

## Strongest Rejection Argument

The paired result fixes only one callsite and cannot prevent future callers
from discarding the merged source; it may therefore look like a soft convention
rather than a real authority cutover.

Response:

The scoped authority split exists only in the match loader. A returns the
accepted parse and exact input in one value, removes the only alternate raw
reload from that loop, and passes the retained source to both downstream
registry constructors. Exact `source_ini_hash == content_hash` tests pin the
pair. Making the source persistent inside `RuleSet` would broaden protected
rules ownership and still not satisfy atlas callers. A is the smallest
Rust-native boundary that eliminates the duplicate source rather than merely
checking it.

## Autonomous Approval Gate

Approved on 2026-07-24 after independent challenge and repair:

- the raw `MountMoras.map` payload was rechecked directly and the false
  overlay-section census was removed;
- full native mode-payload rules application remains `UNCHECKED`;
- stateful multi-pass `ReadINI` equivalence is an explicit residual;
- `SpazWH` is classified only as a separate stock-live map-type allocation
  residual;
- the paired-construction tests now pin the production parse/hash boundary;
- the retail routing oracle names exact ID `0`/`GASAND`, raw false, map true,
  and does not claim Land/Armor parity;
- caller audit found no second hidden production match registry source;
- owned paths do not overlap the dirty damage-authority worktree.

Approach A is the smallest Rust-native cutover that removes the duplicate
source while preserving the current loader's behavior. Implementation is
authorized only within the two owned Rust paths and the documented tests.
