# Alternate Base Cell And Waypoint Action Design

**Scope:** Close the stock-retail waypoint-code and `HouseClass+0x5494` alternate-base-cell
mechanism needed by Phase 3 base placement. This is a prerequisite, not closure of the naval or
ordinary placement selector.

## Evidence and required behavior

- `TActionClass::Read @ 0x006DD5B0` sends action token 8 to `FUN_00763690` except for the
  explicitly numeric parameter types 5, 9, and 11. The decoder examines at most two bytes,
  accepts ASCII letter case through CRT `isalpha`/`toupper`, maps `A..Z` to `0..25`, and maps a
  second letter as `26 * first + second + 26`. Thus `P=15`, `AA=26`, `NZ=389`, and `ZZ=701`.
  A non-letter first byte yields `-1`; trailing bytes after the first two do not participate.
  The `TActionClass` constructor first initializes the destination dword at `+0x44` to zero, and
  `Read` calls the decoder only when `strtok` returns token 8. Therefore an absent or empty-at-end
  token 8 retains waypoint index 0, while a present whitespace/non-letter token decodes to `-1`.
- The existing Rust action-48/action-112 camera path reads token 8 as a decimal integer. That is
  wrong for the native reader and must consume the same alphabetic decoder.
- `TriggerTypeClass::Read` resolves trigger token 1 case-insensitively through
  `HouseTypeClass__FindIndexOfName @ 0x005117D0`. In source registration order it checks each
  HouseType's `Name=` alias (`+0x64`) before its registry ID (`+0x24`); `<none>` resolves to the
  first registered HouseType. `TriggerClass::Spring @ 0x007265C0` then passes that canonical
  HouseType self-index (`+0xB4`) to `HouseClass__Find_By_Country_Index @ 0x00502D30`. The latter
  scans the global House array in registration order and returns the first House whose Type
  index (`House->Type+0xB8`) matches. Rust must therefore canonicalize the trigger owner through
  the source-ordered `RuleSet` country registry, then scan `ScenarioSession.house_order` and
  compare canonical indices through `HouseState.country`; trigger text is not an arbitrary
  entity-owner name, and the event owner is irrelevant.
- `TriggerAction__Execute @ 0x006DD8B0` cases 137/138 call `FUN_006E44E0`/`FUN_006E4540` with that
  House pointer. Action 137 returns without mutation for a null House or a waypoint whose packed
  cell is the zero invalid sentinel; otherwise it writes only `House+0x5494`. Action 138 returns
  without mutation for a null House; otherwise it restores packed zero through `FUN_0050DFF0`.
- `ScenarioClass__Read_Waypoints @ 0x0068BDC0` owns exactly 702 entries, indices `0..=701`.
  Missing/zero entries contain the same packed-zero invalid cell; nonzero entries use signed
  `/1000` and `%1000`. The Rust map parser already retains all valid entries in this range.
- `HouseClass` construction initializes `+0x5494` to packed `(0,0)`. The exhaustive retail census
  found action 137 in `all01umd.map` (`P -> (93,106)`), `all03umd.map`
  (`NZ -> (122,135)`), and `all07smd.map` (`AA -> (105,194)`), and no action 138 in the scanned
  shipped corpus. Action 138 is nevertheless exact, trivial, and shares the same live dispatch,
  so implementing its clearer avoids a custom-map residual.

The primary/launch cell `HouseState::base_center` remains the distinct `House+0x5490` authority.
The `House+0x5750` base-plan center and all of its writers remain a separately reviewed mechanism.

## Ownership and data flow

1. Add a single ASCII waypoint-token decoder beside map action parsing. Return `Option<u32>` as
   the deterministic Rust translation of native `-1`; accept no numeric substitute. Preserve the
   constructor/read distinction: absent or empty-at-end token 8 retains index 0, but a present
   whitespace/non-letter token is invalid.
2. Keep the complete parsed waypoint table in `SimResources`. It is immutable map input, just like
   trigger definitions, and survives an in-scenario load through `SimRuntime::rebind_restored`.
   It must not be duplicated into mutable `Simulation` state merely to serve one trigger action.
3. Borrow that table through `TriggerInputs` into `TriggerRuntime::advance_at_frame` and action
   dispatch. Camera actions emit the decoded index. Action 137 resolves the decoded index to a
   valid map waypoint before it mutates a House.
4. Add `HouseState::alternate_base_center: (u16,u16)`, default/constructor `(0,0)`. It is mutable,
   future-affecting selector state and therefore participates in both snapshots and `state_hash`.
   Bump the bincode snapshot version because adding a field changes every encoded `HouseState`.
5. Resolve `MapTrigger.owner` through the bound `RuleSet` in country source order, testing each
   `CountryRules.name` alias before that entry's registry ID and mapping `<none>` to index 0. Then
   scan only `session.house_order` and select the first state whose `HouseState.country` maps to
   that same canonical index. Missing rules, owner/type resolution, country, registered House,
   decoded waypoint, or waypoint entry performs no write.

## Explicit exclusions

- Malformed action records and non-alphabetic token-8 values do not occur in the exhaustive retail
  action census. Native can index before its waypoint array after a decoded `-1`; Rust must not
  emulate adjacent host-memory reads. Treat the decoded sentinel as invalid/no mutation.
- The fresh shipped-content census covered 310 maps and 21,374 action chunks: exactly three action
  137 records and zero action 138 records. No shipped record depends on malformed interior-empty
  `strtok` collapsing. Deterministic ASCII-only decoding is evidence-backed; do not add numeric
  fallback, clamping, RNG use, event-owner resolution, or writes to `+0x5490`/`+0x5750`.
- This slice does not make the incomplete trigger runtime globally exact, add unimplemented trigger
  actions, change trigger scheduling, or reinterpret action return values. All three stock action-137
  records have a valid resolved House and waypoint, so those unrelated trigger-runtime differences
  are not prerequisites for this state writer.
- Static waypoint data is not state-hashed here. The map identity/hash and bound-resource restore
  contract already own immutable match inputs; only the mutable alternate cell joins lockstep state.

## Acceptance tests

- Decoder vectors: `A`, `Z`, `a`, `P`, `AA`, `aa`, `NZ`, `ZZ`, present first-byte nonletter,
  present whitespace, a letter followed by a nonletter, and ignored third/trailing bytes. Action
  reading separately proves absent and empty-at-end token 8 retain constructor index 0.
- Existing camera actions 48/112 use alphabetic token 8 and produce the decoded index; numeric
  token 8 no longer masquerades as a native waypoint.
- Action 137 writes only the first registration-order House with matching canonical HouseType and
  only for a present waypoint whose packed cell is exactly nonzero. Prove `Name=` alias and
  `<none>` resolution (including duplicate-alias order), missing rules/owner/country/House, invalid
  token, absent/exact-`(0,0)` waypoint, and a same-name/wrong-country House. `(0,y)` and `(x,0)`
  remain valid. No exclusion may mutate either base cell.
- Replay the three retail fixtures `P`, `NZ`, and `AA` to their exact cells.
- Action 138 clears only the matching House's alternate cell and is a no-op for a null resolution.
- Primary `base_center` remains unchanged across 137/138.
- A snapshot round trip preserves a nonzero alternate cell; changing only the alternate cell changes
  `state_hash`; the current snapshot version assertion reflects the layout bump.
- `SimRuntime::rebind_restored` retains the full waypoint table.

Focused validation is `cargo test -p vera20k --lib` with filters for map action parsing, trigger
runtime, runtime resource rebinding, house hash, and snapshot tests. The phase-wide full `--lib`
suite remains reserved for final Phase 3 certification.
