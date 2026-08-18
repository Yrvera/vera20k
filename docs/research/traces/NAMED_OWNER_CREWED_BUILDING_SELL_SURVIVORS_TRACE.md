# Named Owner Crewed Building Sell Survivors Trace

Scenario: skirmish-style local human owner is named `Commander`, selected country is Soviet (`Russians`, side index 1), owns a full-health `NACNST` Soviet Construction Yard, and sells it.

Verdict: FAIL. Rust ejects zero crew survivors because survivor infantry type selection still checks infantry `Owner=` against the literal owner name `Commander`. gamemd selects survivor infantry from the owning house side/country, not the literal player display name.

## Pipeline

`sell_building` -> `eject_sell_survivors` -> `sell_survivor_type` -> `sell_survivor_limit` -> edge-cell placement -> infantry spawn -> sold building removal/refund.

## Entry Points

1. `sell_building` - `src/sim/production/production_sell.rs:441` - player sell command removes a player-owned structure.
2. `eject_sell_survivors` - `src/sim/production/production_sell.rs:139` - sell-only crew survivor path, called before garrison ejection and building removal.
3. `eject_destruction_survivors` - `src/sim/production/production_sell.rs:186` - combat destruction path shares the same `sell_survivor_type` helper, so the named-owner type bug also affects destroyed crewed buildings; this is adjacent, not the traced sell scenario.

Coverage: sell path is traced. Destruction path is adjacent because the requested scenario is selling.

## Concrete Data

Skirmish launch stores player name separately from selected country:

- `src/app_skirmish.rs:298-307` interns slot owner name (`Commander`) as the house key, stores `country=Russians`, and stores `side_index=1`.
- `src/skirmish_launch.rs:56-75` maps `LaunchCountry::Russia` to `country_name=Russians`, `side_index=1`.

Retail/YR INI data:

- `ini/rulesmd.ini:48-50`: `AlliedSurvivorDivisor=500`, `SovietSurvivorDivisor=250`, `ThirdSurvivorDivisor=750`.
- `ini/rulesmd.ini:212-216`: `AlliedCrew=E1`, `SovietCrew=E2`, `ThirdCrew=INIT`, `Technician=CTECH`, `Engineer=ENGINEER`.
- `ini/rulesmd.ini:4327`: `[E2]` Conscript, `Owner=Russians,Confederation,Africans,Arabs`, `Cost=100`.
- `ini/rulesmd.ini:4870`: `[INIT]` Yuri Initiate, `Owner=YuriCountry`.
- `ini/rulesmd.ini:12418`: `[NACNST]` Soviet Construction Yard, `Cost=3000`, `Crewed=yes`, `Owner=Russians,Confederation,Africans,Arabs`.

## gamemd Evidence

Research doc:

- `BUILDINGCLASS_MASTER_GHIDRA_REPORT.md:604-614` identifies active `BuildingClass::Sell` state 1 as eject+animate, with survivor count via vtable+0x2D0 and survivor infantry type via vtable+0x30C.
- Same doc states count is `clamp(Cost / SurvivorDivisor[side], 1, 5)` and type is `AlliedCrew/SovietCrew/ThirdCrew` by side, with Soviet engineer and technician random overrides.

Readonly Ghidra spot-check:

- `BuildingClass__Sell @ 00449c30` state 1 calls vtable+0x2D0 for survivor count, then vtable+0x30C for survivor type, then constructs `InfantryClass(type, this->Owner)`. This is the active standard YR sell mission path; no TS-only gate was found on these calls.
- `FUN_00451330 @ 00451330` reads `this->Owner + 0x1e8` as side index, uses `Rules+0x14f8/+0x14fc/+0x1500` for Allied/Soviet/Third survivor divisors, divides building cost by divisor, clamps to 1..5, and returns 0 only if crewed/other eligibility fails.
- `FUN_0044eb10 @ 0044eb10` gives a 25% Soviet-side engineer chance when not bio-reactor, then delegates to `FUN_00707d20`.
- `FUN_00707d20 @ 00707d20` reads `param_1[0x87] + 0x1e8` as side index and returns `Rules+0xf78/+0xf7c/+0xf80` for Allied/Soviet/Third crew, with technician override rules. It does not compare the infantry type's `Owner=` list to the player's name string.

For this scenario, gamemd expected count for full-health `NACNST`:

- Side index: 1 (Soviet).
- Divisor: 250.
- Building cost: 3000.
- Raw count: `3000 / 250 = 12`.
- Clamp: 5.
- Type: usually `E2`; 25% chance `ENGINEER` on Soviet-side non-bio-reactor path; possible technician override if the building's weapon-equipped predicate triggers. All are spawned for `this->Owner`, i.e. the `Commander` house object.

## Rust Stage Results

### Stage 1 - House identity setup

Rust input: skirmish slot owner name `Commander`, country `Russians`.

Rust output: `HouseState { name=Commander, country=Russians, side_index=1 }`.

gamemd expected: player house identity can have a display/player name while rules country/side drive side-specific rules.

Verdict: PASS for data availability. The required side/country fields exist.

### Stage 2 - Sell trigger and dispatch

Rust: `sell_building` resolves the building owner string from entity owner, then calls `eject_sell_survivors(sim, rules, &owner_name, obj, position, health)` before garrison ejection and removal (`src/sim/production/production_sell.rs:441-481`).

gamemd: active `BuildingClass__Sell` state 1 computes survivor count/type and spawns survivors before final cleanup.

Verdict: UNCHECKED for exact tick/animation timing. Dispatch order is directionally similar, but exact sell animation state timing was not recomputed.

### Stage 3 - Survivor type selection

Rust computation for `Commander` Soviet:

- `sell_survivor_type` reads `HouseState.side_index=1`, builds preferred list `["E2", "E1", "INIT"]`.
- For `E2`, `obj.owner = ["Russians", "Confederation", "Africans", "Arabs"]`.
- Current check at `src/sim/production/production_sell.rs:102` tests whether any `obj.owner` equals literal `owner`, i.e. `Commander`.
- `Commander` does not match `Russians`; `E2` rejected.
- `E1` rejected for the same reason.
- `INIT` rejected for the same reason.
- No infantry type is returned; `eject_sell_survivors` returns 0 immediately at `src/sim/production/production_sell.rs:147-148`.

gamemd computation:

- Reads owner house side index 1.
- Returns `SovietCrew=E2` from Rules, with a 25% Soviet engineer chance on the checked path.
- Does not compare `E2.Owner` to display name `Commander`.

Verdict: FAIL. Player-visible difference: selling the named Soviet player's crewed building produces no crew survivor, while gamemd produces Soviet-side survivor infantry.

### Stage 4 - Survivor count

Rust latent computation for full-health `NACNST`, if Stage 3 returned a type:

- `sell_refund_for_building`: `3000 * 50 * 100 / 10000 = 1500`.
- `survivor_divisor_for_owner`: side index 1 -> `SovietSurvivorDivisor=250`.
- `sell_survivor_limit`: `1500 / 250 = 6`.
- No clamp to 5 in current Rust.

Actual Rust output for the traced scenario: 0, because Stage 3 returns `None` before count is used.

gamemd computation:

- `Cost / SovietSurvivorDivisor = 3000 / 250 = 12`, then clamp to 5.
- No sell-refund or health scaling in the verified sell survivor count path.

Verdict: FAIL. Current visible output is 0 vs gamemd 5 for full-health `NACNST`. After fixing the named-owner type gate, Rust would still produce 6 unless the count formula is fixed to cost/divisor clamped 1..5.

### Stage 5 - Spawn ownership

Rust: if spawning occurs, `spawn_object_at_height(&infantry_type, owner, ...)` uses literal owner `Commander`, which is correct for object ownership.

gamemd: `InfantryClass__Constructor(type, this->Owner)` uses the same owner house pointer as the sold building.

Verdict: PASS for ownership value if a survivor spawns.

### Stage 6 - Placement and timing

Rust: `sell_survivor_positions` scans sorted foundation-edge cells and spawns up to `survivor_limit`.

gamemd: sell path uses foundation exit-cell/edge placement and `CellClass::PlaceInfantryInCell`, with random survivor placement choices in the decompiled path.

Verdict: UNCHECKED. This trace did not compute exact cell equality or random sequence equality.

## Findings

1. FAIL - Named-owner type gate rejects valid Soviet/Yuri survivor infantry.
   - Rust: `src/sim/production/production_sell.rs:102`.
   - gamemd: `FUN_00707d20 @ 00707d20` selects `SovietCrew/ThirdCrew` by owner house side, not display name.
   - Player-visible effect: selling a crewed building as `Commander` with Soviet/Yuri country ejects no crew.

2. FAIL - Sell survivor count formula differs from gamemd.
   - Rust: `src/sim/production/production_sell.rs:29-40` health-scaled half-refund, then `src/sim/production/production_sell.rs:65-80` divides by side divisor without 1..5 clamp.
   - gamemd: `FUN_00451330 @ 00451330` uses full building cost divided by side divisor, clamps 1..5.
   - Player-visible effect: after type selection is fixed, many sold buildings eject too few or too many survivors; full-health `NACNST` would be 6 in Rust vs 5 in gamemd.

## Adjacent Findings

- `eject_destruction_survivors` also calls `sell_survivor_type`, so named-owner destroyed crewed buildings likely share the zero-survivor type failure. Not traced here because the requested scenario is selling.
- Existing Rust tests cover named country production and literal `Russians` sell survivors, but not `Commander` with `country=Russians`.

## Verdict Tally

PASS: 2
FAIL: 2
UNCHECKED: 2
NOT-IMPLEMENTED: 0

