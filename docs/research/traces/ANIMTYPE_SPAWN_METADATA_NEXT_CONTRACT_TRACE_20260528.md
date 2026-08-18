# AnimType Spawn Metadata and Next Contract Trace - 2026-05-28

## Scope

Subagent slot: 4 of `/trace-swarm`.

Concrete mechanic: parse one AnimType/art entry carrying `TrailerAnim`, `TrailerSeperation`, `BounceAnim`, `ExpireAnim`, and `Next`; verify whether current Rust preserves those metadata fields with uppercase animation references and whether `Next` remains in-place transition metadata rather than a new `AnimClass` spawn.

Concrete fixture used for Rust-side value computation:

```ini
[TRACEANIM]
TrailerAnim=smokey2
TrailerSeperation=-2
BounceAnim=twlt026
ExpireAnim=twlt036
Next=smokey
```

Retail-content note: no stock `ini/artmd.ini` section found with all five keys in one section. Stock standard-YR entries prove the individual key families are active: `[DBRIS1LG]` has `ExpireAnim=TWLT036`, `TrailerAnim=SMOKEY2`, `TrailerSeperation=2`; `[METSMALL]` has `ExpireAnim=TWLT100`, `TrailerAnim=METSTRAL`, `TrailerSeperation=1`; `[METSTRAL]` has `Next=SMOKEY`.

No Ghidra MCP calls were needed; the trace relies on verified read-only Ghidra research reports. No Cargo test command was run because this swarm slot may write exactly one file and Cargo would write under `target/`.

## Evidence Read

- `docs/research/ANIMCLASS_AI_TRAILERANIM_PERIODIC_SPAWNS_GHIDRA_REPORT.md`: verifies `TrailerAnim` at `AnimType+0x308`, signed `TrailerSeperation` at `+0x30C`, active YR trailer branch, global-frame modulo, and constructor row `(delay=1, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)`.
- `docs/research/ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md`: verifies `BounceAnim` at `+0x300`, `ExpireAnim` at `+0x304`, active YR bouncer/meteor path, and distinct constructor rows.
- `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`: verifies `Next` is an in-place type-pointer replacement and does not allocate or destroy an `AnimClass`.
- `src/rules/art_data.rs`: current parser and parser tests.
- `src/app_building_anim.rs`, `src/sim/components.rs`: current runtime consumers.

## Pipeline

`INI art section` -> `ArtRegistry::from_ini` -> `AnimTypeRuntimeConfig` metadata -> runtime consumers:

- generic spawn metadata (`TrailerAnim`, `TrailerSeperation`, `BounceAnim`, `ExpireAnim`) has parsed storage but no generic `AnimClass` / bouncer / trailer runtime consumer.
- `Next` has parsed storage and one app-side runtime consumer in `app_building_anim.rs` that mutates the same `AnimRuntime`.
- `WorldEffect` one-shot runtime does not consult `AnimTypeRuntimeConfig::next`.

## Stage Trace

| Stage | Our computed output | gamemd expected output | Verdict |
|---|---|---|---|
| 1. Store AnimType metadata record | `ArtRegistry::from_ini` calls `parse_anim_runtime_config` for every section and inserts under uppercase section key at `src/rules/art_data.rs:499`, `src/rules/art_data.rs:571`, `src/rules/art_data.rs:576`; concrete key `TRACEANIM` would have one metadata record. | `AnimTypeClass::ReadINI @ 0x00427D00` fills the existing `AnimTypeClass`; active for standard YR art parsing per cited reports. Numeric object/table identity was not computed. | UNCHECKED |
| 2. Parse `TrailerSeperation=-2` | `section.get_i32("TrailerSeperation").unwrap_or(0)` stores `-2` as `i32` at `src/rules/art_data.rs:289`; focused parser test asserts `-2` at `src/rules/art_data.rs:1269`. | `ReadINI` stores signed int at `AnimType+0x30C`; negative values are not clamped. Concrete numeric value for the fixture is `-2`. | PASS |
| 3. Preserve `TrailerAnim`, `BounceAnim`, `ExpireAnim`, `Next` refs | `Next` uppercases via `src/rules/art_data.rs:285`; `parse_anim_ref` trims non-empty values and uppercases at `src/rules/art_data.rs:305..310`; concrete Rust refs are `SMOKEY2`, `TWLT026`, `TWLT036`, `SMOKEY`. | gamemd stores `AnimTypeClass*` pointers, not strings: `Next +0x2C8`, `BounceAnim +0x300`, `ExpireAnim +0x304`, `TrailerAnim +0x308`. Pointer numeric values for this synthetic fixture were not computed. | UNCHECKED |
| 4. `TrailerAnim` runtime spawn | Parsed fields exist at `src/rules/art_data.rs:288..289`, but `rg` found no runtime consumer of `trailer_anim`/`trailer_seperation` outside parser tests. No new `WorldEffect`/`AnimClass` row is created for the concrete fixture. | Active YR `AnimClass::AI @ 0x004242A6..0x00424322` spawns independent trailer `AnimClass` when active, not inactive, `TrailerAnim != null`, and signed global-frame modulo passes. For `TrailerSeperation=-2`, active gamemd would use signed `IDIV`; exact frame sample was not run. | NOT-IMPLEMENTED |
| 5. `BounceAnim` runtime spawn | Parsed field exists at `src/rules/art_data.rs:286`, but no generic SHP bouncer `AnimClass::ProcessBounceResult` equivalent was found. | Active YR bouncer path spawns `BounceAnim` on bounce result `1` with `(delay=0, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)` at `0x00423981..0x004239CE`. | NOT-IMPLEMENTED |
| 6. `ExpireAnim` runtime spawn | Parsed field exists at `src/rules/art_data.rs:287`, but no generic bouncer/meteor accepted-impact runtime was found. | Active YR AI spawns `ExpireAnim` on accepted bouncer/meteor impact with `(delay=0, loopCount=1, drawFlags=0x2600, zAdjust=-30, reverse=0)` at `0x00423DE7..0x00423E70`; normal destroy does not spawn it. | NOT-IMPLEMENTED |
| 7. App-side `Next` allocation contract | `advance_anim_runtime_visit` calls `switch_anim_runtime_type` at `src/app_building_anim.rs:936..937`; `switch_anim_runtime_type` mutates `runtime.type_name` at `src/app_building_anim.rs:994` and does not push a new effect or entity. New-spawn count for the switch is `0`. | gamemd `Next` changes the same `AnimClass.Type` pointer and does not allocate/destroy an `AnimClass`; new-spawn count is `0`. | PASS |
| 8. Generic `Next` playback for the concrete spawn-metadata AnimType | `WorldEffect::tick_with_start_sound` advances frames and finishes at `frame >= total_frames` without consulting `AnimTypeRuntimeConfig::next` at `src/sim/components.rs:914..920`. The generic `AnimClass` runtime is absent. | Active gamemd evaluates `Next` after loop exhaustion and mutates the same `AnimClass` in place. | NOT-IMPLEMENTED |

## Entry Point Coverage

- Parser entry point: `ArtRegistry::from_ini` processes all sections and caches `AnimTypeRuntimeConfig` (`src/rules/art_data.rs:385`, `src/rules/art_data.rs:499`, `src/rules/art_data.rs:576`).
- Runtime entry point found for `Next`: app-side building/garrison animation runtime (`src/app_building_anim.rs:892`, `src/app_building_anim.rs:936`, `src/app_building_anim.rs:980`).
- Generic runtime entry point missing: no global `AnimClass` AI scheduler or generic bouncer/trailer runtime was found; `WorldEffect` is a one-shot frame ticker.

## Findings

### NOT-IMPLEMENTED - TrailerAnim spawn metadata is parsed but inert

Player-visible difference: debris/meteor-style animations with `TrailerAnim` will not emit smoke/trail child animations from the generic runtime. Current Rust preserves `trailer_anim` and `trailer_seperation` in metadata, but no consumer creates the gamemd child `AnimClass` row.

Rust evidence: `src/rules/art_data.rs:288`, `src/rules/art_data.rs:289`; no runtime usages found outside parser tests.

gamemd evidence: `AnimClass::AI @ 0x004242A6..0x00424322`; active in standard YR when stock or modded live AnimTypes with non-null `TrailerAnim` reach the branch.

### NOT-IMPLEMENTED - BounceAnim and ExpireAnim runtime split is absent

Player-visible difference: bouncer impacts cannot show the separate bounce tick animation and accepted-impact explosion animation with native constructor flags/order. The parser carries both names, but no generic bouncer path consumes them.

Rust evidence: `src/rules/art_data.rs:286`, `src/rules/art_data.rs:287`; no generic bouncer runtime found.

gamemd evidence: `BounceAnim` constructor in `AnimClass::ProcessBounceResult @ 0x00423981..0x004239CE`; `ExpireAnim` constructor in `AnimClass::AI @ 0x00423DE7..0x00423E70`; active for standard YR bouncer/meteor AnimTypes.

### NOT-IMPLEMENTED - Generic Next chain is absent outside the app-side runtime slice

Player-visible difference: a generic world effect / future generic `AnimClass` created from the concrete entry would end instead of morphing in-place to `SMOKEY`. The garrison/building app slice mutates in place correctly, but `WorldEffect` one-shots do not consult `Next`.

Rust evidence: `src/app_building_anim.rs:936..1002` is in-place for that app slice; `src/sim/components.rs:914..920` finishes one-shot effects without `Next`.

gamemd evidence: `ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md` verifies `Next` replaces the current `AnimClass.Type` pointer and allocates `0` new `AnimClass` instances.

## Adjacent Findings

- Existing research reports still contain older implementation-status text saying `art_data.rs` does not parse some of these metadata fields. Current source now parses them; the stale research wording should be updated in a separate doc-maintenance pass, not in this trace slot.
- The parser accepts negative `TrailerSeperation` as gamemd does. Runtime semantics for non-null `TrailerAnim` with zero separation remain a future hazard: gamemd would reach signed divide-by-zero rather than silently disabling trailers.

## Verdict Tally

PASS: 2 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 4

## Status

COMPLETE
