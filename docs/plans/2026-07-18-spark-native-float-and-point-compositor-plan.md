# Spark Native-Float and Tactical Point Foundation Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Stop after the final verification task; do not remove the public Spark rejection or add a production point pass from this plan.

**Goal:** Add the deterministic, integer-backed arithmetic, behavior-specific Spark state, injected-input Spark tick kernel, and pure tactical u16 A/Z point resolver that are safe to build before the remaining bridge, parser, runtime-display, A-buffer, snapshot, and frame-order prerequisites are proved.

**Architecture:** `util/native_x87.rs` owns the new bounded software arithmetic surface so both simulation and rendering can name every native operation and memory-store boundary without using hardware floating point in `sim/`. `sim/particles/spark.rs` owns only pure behavior-3 state transitions over injected collision facts; `render/tactical_compat.rs` owns immutable point commands and a pure resolver over injected u16 A/Z frame data. Public Spark system creation, `system_ai` dispatch, snapshots, map-query wiring, render extraction, and physical pixel commit remain unchanged and unavailable.

**Design Doc:** `docs/plans/2026-07-18-spark-native-float-and-point-compositor-design.md`

---

## Grounding Summary

- `PARTICLE_SPARK_COLLISION_AND_PIXEL_COMPOSITOR_GHIDRA_REPORT.md` is the verified primary report for the active behavior-3 roots `0x0062C6E0` and `0x0062CEC0`; it proves field widths, x87 store boundaries, collision predicates, point projection, u16 A/Z rules, interpolation, packing, and draw order.
- `PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md` proves forward particle AI followed by reverse dead cleanup and supplies the color-progression state machine.
- `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` proves the Spark color jitter consumes the gameplay/scenario stream, not render RNG.
- Live Ghidra verification confirms process control word `0x0E7F`: 53-bit precision and truncate toward zero; `Math__ftol @ 0x007C5F00` performs `FISTP qword` without restoring another mode.
- Live `disassemble_function(0x005AF4D0)` confirms the exact three matrix-vector dot-product orders and one `f32` store per output component. Live decompile/disassembly of `0x005AFC20` confirms raw transpose copies plus one-store negative-dot translations. Root disassembly fixes the surrounding order as inverse multiply, scalar-by-one of that local result, local-Z negate, forward multiply, final-Y negate.
- Movement-root disassembly proves that cell and bridge work consumes the earlier integer candidate while ground/building/clamp predicates retain candidate `f32` and the final selected coordinate crosses `Math__ftol` again. The kernel therefore carries both candidate forms.
- Projection-root disassembly proves that each wrapping `60` or `30` multiplication crosses its own signed `/2` before term addition and the final `/256`; folding those expressions changes overflow behavior. Draw-root disassembly also proves that `1.0 - accumulator` remains on the x87 stack across all three color channels.
- The current Rust particle owner pattern is `BTreeMap<u64, ParticleSystem>` plus ordered `Vec<Particle>`, with remove/tick/reinsert ownership in `system_ai.rs`; this plan preserves those types but does not activate Spark dispatch.
- Current Spark/Railgun public spawn rejection in `spawn.rs:26-49`, Spark no-op dispatch in `system_ai.rs:103-112`, and SHP-builder skip in `app_instances/particles.rs:55-61` are deliberate activation guards and must remain.
- Current `Particle` has generic fixed-point direction/scalar velocity, `u8` index, and `SimFixed` accumulator. This plan adds an optional raw-bit `SparkRuntimeState` without changing Smoke/Gas/Fire semantics.
- Current particle hashing in `world_hash.rs:130-155` omits Spark motion/color state. This plan adds raw-bit hashing, but snapshot persistence stays outside this conflict-free foundation because `snapshot.rs`, `world/mod.rs`, and `world_hash.rs` already contain another session's work and particle systems are globally `#[serde(skip)]`.
- `rulesmd.ini:26075-26105,26345-26353` and base fallback `rules.ini:18874-18904,19055-19063` provide behavior 3, integer velocity bounds, RGB lists/start colors, and textual `ColorSpeed=.13`; `[General] Gravity=6` is at `rulesmd.ini:756` and `rules.ini:615`.
- The exact native decimal-to-double result for `ColorSpeed=.13` is not yet captured. Therefore this plan accepts `NativeF64Bits` as an injected kernel input and does not change the current rule parser or `ParticleType.color_speed`.
- The current R8 shroud texture and `Depth32Float` scene depth are not equivalent to the native u16 A/Z consumer domain. The pure point resolver accepts exact external words and has no R8/float-depth fallback.
- Static bridge facts are not proved equivalent to live `CellClass+0x140 & 0x100` after collapse/repair. The collision kernel accepts already-resolved old/new structural facts and does not choose a Rust producer.
- Runtime `AdjustForZ`, DirectDraw loss/shift values, the exact A-buffer producer, common performance latch ownership, physical point commit position, and final retail pixel oracle remain activation gates.

Research evidence inventory:

| Report | Stated confidence/status | Active-YR / caution |
|---|---|---|
| `PARTICLE_SPARK_COLLISION_AND_PIXEL_COMPOSITOR_GHIDRA_REPORT.md` | Verified primary mechanism report | Both roots and stock behavior-3 types active in YR |
| `PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md` | High for decompiled functions; Medium for unrecovered semantic names | Stock Spark/Railgun systems active in YR |
| `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` | High for scoped RNG bounds/order; COMPLETE for classified callsites | Particle owner/RNG paths active; the report's Spark non-scope is supplemented by the primary report |
| `BSURFACE_CIRCBUF_ABUFFER_REPORT.md` | HIGH for constructor offsets and multiple usage sites | Shared A/Z circular substrate is active |
| `ZBUFFER_DEPTH_SYSTEM.md` | Ghidra-backed context document with no explicit confidence declaration | Use only where corroborated by the fresh Spark report and bracket reports |
| `TACTICAL_ABUFFER_SHROUD_VALUES_FOR_BRACKET_LINES_GHIDRA_REPORT.md` | High for writers/constants/gates; Medium for asset-internal edge distribution | Reset/shroud/gap paths active; fog conditional/off by default |
| `SURFACE_DRAWLINE_ABUFFER_ZTEST_PIXEL_CONTRACT_GHIDRA_REPORT.md` | High for predicate/order/write flag; Medium for decompiler-local names | Selected-building consumer active; fog conditional/off by default |
| `PRIMARY_SURFACE_ZBUFFER_BRACKET_OWNERSHIP_GHIDRA_REPORT.md` | High for normal bracket buffer ownership/timing; Medium for exotic VXL states | Normal bracket path active; exotic visual matrix remains conditional |

No selected report identifies the scoped Spark roots as dormant Tiberian Sun legacy. Standard YR fog remains conditional/off by default; this plan therefore accepts a pre-resolved fog gate and makes no TS-style fog mechanism part of Spark simulation.

## Key Technical Decisions

- **Integer-backed, finite-domain x87 subset:** store 53-bit normalized significands and signed exponents, use explicit guard/round/sticky alignment, and chop after every operation. — **Confidence: high**
  - **Source:** approved design; Ghidra startup chain `0x007CD80F` through `0x007C5F00`; movement stores `0x0062C705..0x0062C76A`.
- **Reject unsupported exceptional values:** signed zero is preserved, while NaN, infinity, subnormal input/result, overflow, and invalid integer conversion return typed errors instead of silently claiming native equivalence. — **Confidence: high**
  - **Source:** approved design safe-domain boundary; exceptional behavior remains explicitly unverified.
- **Inject facts into the Spark kernel:** the pure tick receives gravity, color speed, one already-consumed RNG sample, color count, slope matrix, ground, structural, building, and overlay facts. — **Confidence: high**
  - **Source:** approved design activation gates; current map/bridge files are concurrent-session surfaces and P6 remains blocked.
- **Keep public activation guards:** no task changes `spawn_particle_system`, Spark `system_ai` dispatch, or the SHP builder's behavior-3 skip. — **Confidence: high**
  - **Source:** design Simulation/Render contracts and contract P2/P19.
- **Hash raw Spark state now, defer particle snapshot ownership:** raw bits affect deterministic future state and must hash; serialization/version ownership waits until the pre-existing global `#[serde(skip)]` particle-store issue and active snapshot work are coordinated. — **Confidence: high**
  - **Source:** contract P20; current `git status`; `world/mod.rs` particle store serialization attribute.
- **Pure renderer contract, no physical point pass:** `render/tactical_compat.rs` returns an ordered packed write or typed rejection but never writes GPU/surface state and never selects R8 or float depth. — **Confidence: high**
  - **Source:** verified point root `0x0062CEC0`; approved design Render contract.
- **Preserve helper scalar order:** slope transform dot products use the exact `0x005AF4D0` operand order and only one `f32` quantization per output; inverse translation calculations are performed even though the later 3x3 multiply ignores them. — **Confidence: high**
  - **Source:** live Ghidra `disassemble_function(0x005AF4D0)`, decompile/disassembly `0x005AFC20`, and root sequence `0x0062C9C8..0x0062CA26` recorded in the research report.
- **Retain native dual candidate forms:** integer candidates select the cell and drive bridge comparisons, while raw stored-`f32` candidates drive ground/contact/clamp predicates and the final commit conversion. — **Confidence: high**
  - **Source:** movement root `0x0062C76E..0x0062C7C5`, `0x0062C85B..0x0062C94B`, and final `Math__ftol` sequence `0x0062CA3B..0x0062CA5D`.
- **Preserve projection overflow staging and color stack lifetime:** each `60`/`30` product is wrapping and divided by two before addition, and one retained `1.0 - accumulator` value serves all three channel calculations. — **Confidence: high**
  - **Source:** projection helper `0x006D2140`; point root `0x0062D164..0x0062D215`.
- **Synthetic display tuples are regression fixtures only:** unit tests may inject an RGB565-like tuple, but production code has no default tuple and the tests make no retail pixel-parity claim. — **Confidence: high**
  - **Source:** contract P18 and AT-16.

## Open Questions

### Resolved During Planning

- **Which x87 precision and rounding mode is active?** 53-bit precision and truncation toward zero, live control word `0x0E7F`.
- **Where are movement memory boundaries?** Persistent Z, probe Z, converted coordinates, candidate components, and the accumulator have explicit `f32`/`f64` stores.
- **What is the slope helper's scalar order?** `(m01*v1 + m02*v2) + m00*v0`, `(m11*v1 + m10*v0) + m12*v2`, and `(m21*v1 + m20*v0) + m22*v2`, with one final `f32` store per component.
- **Which candidate representation drives collision?** Integer X/Y/Z drives the cell and bridge path; retained stored-`f32` candidate Z drives ground, contact-band, clamp, and final-selection logic before a second final `Math__ftol` commit.
- **Can projection fold `60/2` to `30` or `30/2` to `15`?** No. Each native wrapping multiplication is divided separately before wrapping addition and final `/256`.
- **Is `1.0 - accumulator` recomputed per RGB channel?** No. The point root retains one x87 value across all three channel calculations.
- **Does reflected slope velocity persist?** No. It is stack-local; only coordinate, stored Z velocity, and deletion state persist.
- **Can the existing SHP/R8/Depth32 paths be reused?** No equivalence is proved; the foundation exposes a distinct point consumer contract.

### Deferred Activation Gates

- **Exact `ColorSpeed=.13` raw double:** capture retail memory or verify the native INI reader before changing `ParticleType.color_speed` or initializing live Spark state.
- **Live structural bridge bit:** trace native bit `0x100` across intact, collapsed, and repaired bridge states before wiring `SparkCollisionFacts` to Rust map state.
- **Ground/slope/occupancy producer:** map the injected facts to the current resolved terrain and ordered occupancy without changing invalid-cell or first-building semantics.
- **Snapshot version ownership:** after other snapshot work lands, serialize the complete particle store, increment the then-current version exactly once, and rebaseline under one owner.
- **Runtime display values:** capture `g_AdjustForZ_Multiplier` and the six DirectDraw loss/shift globals from a running standard retail session.
- **Exact A/Z production:** prove the complete u16 A producer and the integer tactical Z producer/frame timing; no R8 or `Depth32Float` substitution is authorized.
- **Common performance latch and object-render order:** identify the shared owner and preserve mixed-particle traversal before command extraction/commit is wired.
- **Retail certification:** execute contract AT-16 and require raw predicate, packed value, touched byte(s), and destination pixel equality.
- **Upstream burst/lights:** finish the separate Spark particle-system and lighting design before public behavior-3 activation.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/util/native_x87.rs` | Deterministic finite-domain x87 PC=53/RC=chop values, operations, stores, comparison, and `ftol` |
| Modify | `src/util/mod.rs` | Export the compatibility module |
| Modify | `src/sim/particles/mod.rs` | Define `SparkRuntimeState`, add `Particle.spark`, and expose the Spark kernel |
| Modify | `src/sim/particles/fire.rs` | Initialize non-Spark particles with `spark: None` |
| Modify | `src/sim/particles/gas.rs` | Initialize non-Spark particles with `spark: None` |
| Modify | `src/sim/particles/smoke.rs` | Initialize non-Spark particles with `spark: None` |
| Modify | `src/sim/particles/spawn.rs` | Initialize generic particles with `spark: None`; retain public Spark rejection |
| Modify | `src/sim/particles/system_ai.rs` | Update test literals only; retain Spark no-op dispatch |
| Modify | `src/sim/world/world_hash.rs` | Hash optional Spark raw state in stable particle order and add sensitivity tests |
| Create | `src/sim/particles/spark.rs` | Pure injected-input motion, slope, collision, color, lifetime, and one-particle tick kernel |
| Modify | `src/render/mod.rs` | Export the tactical compatibility module |
| Create | `src/render/tactical_compat.rs` | Immutable command/frame types and pure gate/projection/A/Z/color/packing resolver |

## Interface Changes

- `util::native_x87` adds `NativeF32Bits`, `NativeF64Bits`, opaque `X87Value`, `X87Chop53`, `X87Ordering`, and `NativeX87Error`. Callers must use named methods; no arithmetic operator traits are implemented.
- `Particle` gains `pub spark: Option<SparkRuntimeState>`. Every existing constructor initializes `None`; only tests in this plan construct `Some`.
- `sim::particles::spark` adds pure types `SparkCollisionFacts`, `SparkCollisionKind`, `SparkMotionStep`, `SparkTickInputs`, `SparkTickResult`, and pure functions. It imports no presentation layer.
- `render::tactical_compat` adds `SparkPointCommand`, `WordPlane`, `TacticalCompatFrame`, `TacticalRect`, `DirectDrawPixelFormat`, `PointResolution`, `PackedPointWrite`, typed rejection/error enums, and pure helpers.
- No existing public spawn, tick-dispatch, map, rules-parser, snapshot, app-extraction, GPU-resource, or draw-pass interface changes in this plan.

## Sim Checklist

- [ ] `src/sim/` contains no `f32` or `f64` arithmetic or values; native floats are raw `u32`/`u64` wrappers and integer operations.
- [ ] Optional Spark state is hashed by raw bits, signed index, RGB, and presence tag in existing BTreeMap/Vec order.
- [ ] `sim/` adds no dependency on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.
- [ ] Production tick ordering is unchanged because Spark `system_ai` remains a no-op; the pure kernel internally preserves movement/collision, color, then lifetime order.
- [ ] Existing BTreeMap system order and Vec particle order remain unchanged.
- [ ] No per-tick allocation exists inside the x87 kernel, Spark kernel, or point resolver.
- [ ] Snapshot persistence is explicitly still unavailable, so public Spark activation remains blocked.

## Risk Areas

- A one-bit arithmetic error would contaminate coordinates, collision branches, color thresholds, packed pixels, and deterministic hashes. Tests must assert raw bits at every store boundary and later be checked against a native oracle.
- Subtraction alignment and negative truncation are easy to implement as round-to-nearest accidentally. The software kernel uses chop semantics and includes values around f32 and 53-bit precision boundaries.
- Adding `Particle.spark` touches every struct literal. `rg -n "Particle\\s*\\{" src` must be empty of unreviewed constructors after the edit.
- `world_hash.rs` currently contains another session's changes. The executor must re-read and merge only the narrow particle hash block; no unrelated hunk may be replaced.
- The collision facts are deliberately already-resolved inputs. Turning them into booleans from static bridge/deck presence would bypass the P6 gate and create silent drift.
- Collision must not discard candidate `f32` after creating the integer cell/bridge coordinate. Fractional values around `G`, `G-100`, and `G+150` can select different native branches even when the earlier `Math__ftol` candidate equals the integer boundary.
- The point resolver must sample A before Z, never write Z, apply A once, preserve signed wrapping, and retain the draw ordinal. A generic GPU depth or shroud path is not an acceptable verification substitute.
- Projection factors must remain separate wrapping multiply-then-`/2` terms; algebraic constant folding is invalid over the full `i32` input domain. Color interpolation must retain one `1.0-a` x87 value and the native next-term-plus-current-term order.
- Synthetic display-format tests can prove the formula only. They cannot establish which tuple retail uses.
- These foundation tests are regression evidence. They do not certify parity until gamemd/retail-derived fixtures or exhaustive proof are named.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---:|---|---|---|
| 1 | PC=53 chop arithmetic and explicit stores | One bit changes later collision/color/pixel state | Raw-bit unit tests plus later x87 differential oracle; Ghidra startup/control chain |
| 1 | Signed zero and exceptional boundary | Silent canonicalization would falsify raw-state claims | Signed-zero store tests; typed error tests for unverified domains |
| 2 | Integer/f32/f64 and `ftol` boundaries | Native converts coordinates and channels through these exact widths | Precision-boundary and positive/negative chop tests |
| 3 | `SparkRuntimeState` widths and presence | Generic direction/scalar/index state cannot encode gamemd behavior 3 | Compile-time types; raw-hash sensitivity including `+0` versus `-0` |
| 3 | Stable raw hashing | Hidden Spark points still affect future RNG/state | One-field-at-a-time state hash tests in BTreeMap/Vec order |
| 4 | Double-gravity asymmetry | Every Spark trajectory uses stored `old_vz-g` but candidate `old_vz-2g` | Flat-ground trace `(2560,2560,10)` -> stored `-6`, candidate `-2` |
| 4 | Signed coordinate conversion | Negative and >24-bit coordinates can differ by one or more leptons | Raw f32-store/ftol tests and `/256` truncation table |
| 4 | Bridge integer form versus ground/contact raw-`f32` form | Early `Math__ftol` can hide a fractional crossing and change deletion | Integer boundary table plus raw `-0.5` candidate at `G=0`; final commit must still be `0` |
| 4 | Bridge, ground, building, wall boundaries | Equality sides and 20/100/150-lepton boundaries change deletion coordinates | Table tests for `P`, `P-20`, `G-99/G-100`, `G+149/G+150`, wall IDs |
| 4 | Slope operation order and discarded result | Reordering changes the native helper trace; reassociated dot products can change bits; persisting it would invent bounce | Static inverse/scale/negate/forward order audit; non-associative `2^100 + -2^100 + 1` dot fixture; identity reflection fixture |
| 4 | Color RNG formula and lifetime order | One missed draw desynchronizes gameplay; `>1` and signed-i16 behavior are exact | Injected sample formula tests, strict threshold/index tests, lifetime `0 -> -1` |
| 5 | Projection wrapping/clip | Folding separate wrapping products changes pixels at large signed coordinates | Native fixtures `(256,0,0)`, `(-1,0,0)`, `(50_000_000,0,0)`, and `(i32::MAX,1,0)` plus all clip edges |
| 5 | Complete u16 A domain | Native discontinuity at `126 -> 127` cannot fit an assumed opacity interpretation | A values `0,1,126,127,128,65535` |
| 5 | Wrapped u16 Z and strict comparison | Equality rejects and Spark does not update depth | candidate-1/equal/+1, negative/above-u16/wrap tests; unchanged input slice |
| 5 | Color source/interpolation | Index zero targets list entry 1, not entry 0; native retains one `1.0-a` value across RGB | Index-zero/nonzero tests, raw x87 interpolation, and Task 7 stack-lifetime audit |
| 5 | Runtime packing/byte width | Retail may use different masks; non-2-byte surface writes low byte | Injected tuple formula tests; no production defaults |
| 6 | Gate order and draw ordinal | Render hiding must not stop simulation and batching must not reorder pixels | Typed first-rejection tests and ordinal round-trip |
| 7 | Activation guards | Partial Spark activation would run missing producer/query/render behavior | Source assertions and existing focused tests; inspect no production wiring diff |

---

## Tasks

### Task 1: Add the bounded software x87 interface and implementation

**Why:** Every later Spark simulation and point-color operation depends on an explicit, deterministic PC=53/truncate arithmetic surface.

**Files:**

- Create: `src/util/native_x87.rs`
- Modify: `src/util/mod.rs:1-23`

**Pattern:** New low-level pattern approved by the design. It follows the existing `util` rule of having no gameplay or presentation ownership and uses `thiserror` for library errors.

**Step 1: Export the module**

Add this declaration to `src/util/mod.rs` in alphabetical order:

```rust
pub mod native_x87;
```

**Step 2: Create the complete finite-domain compatibility kernel**

Create `src/util/native_x87.rs` with the following code:

```rust
//! Deterministic integer implementation of the finite x87 subset used by gamemd.
//!
//! The active process uses 53-bit precision and truncate-toward-zero rounding.
//! Callers name every operation and memory store so evaluation order stays visible.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const SIGNIFICAND_TOP: u64 = 1_u64 << 52;
const EXTENDED_TOP: u64 = 1_u64 << 55;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NativeF32Bits(u32);

impl NativeF32Bits {
    pub const POSITIVE_ZERO: Self = Self(0x0000_0000);
    pub const NEGATIVE_ZERO: Self = Self(0x8000_0000);
    pub const ONE: Self = Self(0x3f80_0000);

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NativeF64Bits(u64);

impl NativeF64Bits {
    pub const POSITIVE_ZERO: Self = Self(0x0000_0000_0000_0000);
    pub const NEGATIVE_ZERO: Self = Self(0x8000_0000_0000_0000);
    pub const HALF: Self = Self(0x3fe0_0000_0000_0000);
    pub const ONE: Self = Self(0x3ff0_0000_0000_0000);

    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X87Ordering {
    Less,
    Equal,
    Greater,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum NativeX87Error {
    #[error("{format} NaN or infinity is outside the verified x87 domain")]
    NonFiniteInput { format: &'static str },
    #[error("{format} subnormal input is outside the verified x87 domain")]
    SubnormalInput { format: &'static str },
    #[error("{format} subnormal result is outside the verified x87 domain")]
    SubnormalResult { format: &'static str },
    #[error("{format} overflow is outside the verified x87 domain")]
    StoreOverflow { format: &'static str },
    #[error("x87 integer conversion is outside the verified signed 64-bit domain")]
    IntegerConversion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X87Value {
    sign: bool,
    exponent: i32,
    significand: u64,
}

impl X87Value {
    const fn zero(sign: bool) -> Self {
        Self {
            sign,
            exponent: 0,
            significand: 0,
        }
    }

    const fn is_zero(self) -> bool {
        self.significand == 0
    }

    fn magnitude_cmp(self, rhs: Self) -> Ordering {
        self.exponent
            .cmp(&rhs.exponent)
            .then_with(|| self.significand.cmp(&rhs.significand))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct X87Chop53;

impl X87Chop53 {
    pub fn load_i32(value: i32) -> X87Value {
        if value == 0 {
            return X87Value::zero(false);
        }
        let sign = value.is_negative();
        let magnitude = value.unsigned_abs() as u64;
        let top = 63 - magnitude.leading_zeros();
        X87Value {
            sign,
            exponent: top as i32,
            significand: magnitude << (52 - top),
        }
    }

    pub fn load_f32(bits: NativeF32Bits) -> Result<X87Value, NativeX87Error> {
        let raw = bits.bits();
        let sign = raw >> 31 != 0;
        let exponent = (raw >> 23) & 0xff;
        let fraction = raw & 0x007f_ffff;
        if exponent == 0xff {
            return Err(NativeX87Error::NonFiniteInput { format: "f32" });
        }
        if exponent == 0 {
            if fraction == 0 {
                return Ok(X87Value::zero(sign));
            }
            return Err(NativeX87Error::SubnormalInput { format: "f32" });
        }
        Ok(X87Value {
            sign,
            exponent: exponent as i32 - 127,
            significand: ((1_u64 << 23) | fraction as u64) << 29,
        })
    }

    pub fn load_f64(bits: NativeF64Bits) -> Result<X87Value, NativeX87Error> {
        let raw = bits.bits();
        let sign = raw >> 63 != 0;
        let exponent = (raw >> 52) & 0x7ff;
        let fraction = raw & 0x000f_ffff_ffff_ffff;
        if exponent == 0x7ff {
            return Err(NativeX87Error::NonFiniteInput { format: "f64" });
        }
        if exponent == 0 {
            if fraction == 0 {
                return Ok(X87Value::zero(sign));
            }
            return Err(NativeX87Error::SubnormalInput { format: "f64" });
        }
        Ok(X87Value {
            sign,
            exponent: exponent as i32 - 1023,
            significand: (1_u64 << 52) | fraction,
        })
    }

    pub fn neg(value: X87Value) -> X87Value {
        X87Value {
            sign: !value.sign,
            ..value
        }
    }

    pub fn add(lhs: X87Value, rhs: X87Value) -> X87Value {
        if lhs.is_zero() && rhs.is_zero() {
            return X87Value::zero(lhs.sign && rhs.sign);
        }
        if lhs.is_zero() {
            return rhs;
        }
        if rhs.is_zero() {
            return lhs;
        }

        let mut high = lhs;
        let mut low = rhs;
        if high.exponent < low.exponent {
            std::mem::swap(&mut high, &mut low);
        }
        let exponent_gap = (high.exponent - low.exponent) as u32;
        let high_extended = high.significand << 3;
        let low_extended = shift_right_jam_u64(low.significand << 3, exponent_gap);

        if high.sign == low.sign {
            let mut sum = high_extended + low_extended;
            let mut exponent = high.exponent;
            if sum & (EXTENDED_TOP << 1) != 0 {
                sum = shift_right_jam_u64(sum, 1);
                exponent += 1;
            }
            return chop_extended(high.sign, exponent, sum);
        }

        if high_extended == low_extended {
            return X87Value::zero(false);
        }
        let (sign, mut difference) = if high_extended > low_extended {
            (high.sign, high_extended - low_extended)
        } else {
            (low.sign, low_extended - high_extended)
        };
        let top = 63 - difference.leading_zeros();
        let normalize = 55 - top;
        difference <<= normalize;
        chop_extended(sign, high.exponent - normalize as i32, difference)
    }

    pub fn sub(lhs: X87Value, rhs: X87Value) -> X87Value {
        Self::add(lhs, Self::neg(rhs))
    }

    pub fn mul(lhs: X87Value, rhs: X87Value) -> X87Value {
        if lhs.is_zero() || rhs.is_zero() {
            return X87Value::zero(lhs.sign ^ rhs.sign);
        }
        let product = lhs.significand as u128 * rhs.significand as u128;
        let top = 127 - product.leading_zeros();
        let shift = top - 55;
        let extended = shift_right_jam_u128(product, shift);
        let exponent = lhs.exponent + rhs.exponent + (top as i32 - 104);
        chop_extended(lhs.sign ^ rhs.sign, exponent, extended)
    }

    pub fn compare(lhs: X87Value, rhs: X87Value) -> X87Ordering {
        if lhs.is_zero() && rhs.is_zero() {
            return X87Ordering::Equal;
        }
        let ordering = if lhs.sign != rhs.sign {
            if lhs.sign {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        } else {
            let magnitude = lhs.magnitude_cmp(rhs);
            if lhs.sign { magnitude.reverse() } else { magnitude }
        };
        match ordering {
            Ordering::Less => X87Ordering::Less,
            Ordering::Equal => X87Ordering::Equal,
            Ordering::Greater => X87Ordering::Greater,
        }
    }

    pub fn store_f32(value: X87Value) -> Result<NativeF32Bits, NativeX87Error> {
        let sign = u32::from(value.sign) << 31;
        if value.is_zero() {
            return Ok(NativeF32Bits::from_bits(sign));
        }
        if value.exponent > 127 {
            return Err(NativeX87Error::StoreOverflow { format: "f32" });
        }
        if value.exponent < -126 {
            return Err(NativeX87Error::SubnormalResult { format: "f32" });
        }
        let exponent = ((value.exponent + 127) as u32) << 23;
        let fraction = ((value.significand >> 29) as u32) & 0x007f_ffff;
        Ok(NativeF32Bits::from_bits(sign | exponent | fraction))
    }

    pub fn store_f64(value: X87Value) -> Result<NativeF64Bits, NativeX87Error> {
        let sign = u64::from(value.sign) << 63;
        if value.is_zero() {
            return Ok(NativeF64Bits::from_bits(sign));
        }
        if value.exponent > 1023 {
            return Err(NativeX87Error::StoreOverflow { format: "f64" });
        }
        if value.exponent < -1022 {
            return Err(NativeX87Error::SubnormalResult { format: "f64" });
        }
        let exponent = ((value.exponent + 1023) as u64) << 52;
        let fraction = value.significand & 0x000f_ffff_ffff_ffff;
        Ok(NativeF64Bits::from_bits(sign | exponent | fraction))
    }

    pub fn ftol_i64(value: X87Value) -> Result<i64, NativeX87Error> {
        if value.is_zero() || value.exponent < 0 {
            return Ok(0);
        }
        if value.exponent > 63 {
            return Err(NativeX87Error::IntegerConversion);
        }
        let magnitude = if value.exponent <= 52 {
            (value.significand >> (52 - value.exponent)) as u128
        } else {
            let shift = (value.exponent - 52) as u32;
            (value.significand as u128) << shift
        };
        if value.sign {
            if magnitude > (1_u128 << 63) {
                return Err(NativeX87Error::IntegerConversion);
            }
            if magnitude == 1_u128 << 63 {
                return Ok(i64::MIN);
            }
            Ok(-(magnitude as i64))
        } else {
            if magnitude > i64::MAX as u128 {
                return Err(NativeX87Error::IntegerConversion);
            }
            Ok(magnitude as i64)
        }
    }
}

fn chop_extended(sign: bool, exponent: i32, extended: u64) -> X87Value {
    let significand = extended >> 3;
    debug_assert!(significand == 0 || significand & SIGNIFICAND_TOP != 0);
    X87Value {
        sign,
        exponent,
        significand,
    }
}

fn shift_right_jam_u64(value: u64, distance: u32) -> u64 {
    if distance == 0 {
        value
    } else if distance < 64 {
        (value >> distance) | u64::from(value << (64 - distance) != 0)
    } else {
        u64::from(value != 0)
    }
}

fn shift_right_jam_u128(value: u128, distance: u32) -> u64 {
    if distance == 0 {
        value as u64
    } else if distance < 128 {
        ((value >> distance) as u64) | u64::from(value << (128 - distance) != 0)
    } else {
        u64::from(value != 0)
    }
}
```

**Step 3: Check the interface constraints**

Run:

```powershell
rg -n "impl (Add|Sub|Mul)|\bf32\b|\bf64\b" src/util/native_x87.rs
```

Expected: `f32`/`f64` appear only in documentation/error-format strings and type names such as `NativeF32Bits`; there are no arithmetic trait implementations and no Rust floating-point fields or literals.

### Task 2: Add raw-bit arithmetic regression tests

**Why:** The x87 module is a high-risk foundation; raw store boundaries, negative chop, cancellation, signed zero, and unsupported domains must be pinned before any Spark logic consumes it.

**Files:**

- Modify: `src/util/native_x87.rs` after the helpers from Task 1

**Pattern:** Same-file Rust unit tests for pure low-level transforms. These are regression fixtures, not gamemd parity certification.

**Step 1: Append the complete test module**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn f32_value(bits: u32) -> X87Value {
        X87Chop53::load_f32(NativeF32Bits::from_bits(bits)).unwrap()
    }

    fn f64_value(bits: u64) -> X87Value {
        X87Chop53::load_f64(NativeF64Bits::from_bits(bits)).unwrap()
    }

    #[test]
    fn signed_zero_round_trips_without_canonicalization() {
        let positive = X87Chop53::load_f32(NativeF32Bits::POSITIVE_ZERO).unwrap();
        let negative = X87Chop53::load_f32(NativeF32Bits::NEGATIVE_ZERO).unwrap();
        assert_eq!(X87Chop53::store_f32(positive).unwrap().bits(), 0x0000_0000);
        assert_eq!(X87Chop53::store_f32(negative).unwrap().bits(), 0x8000_0000);
        assert_eq!(X87Chop53::compare(positive, negative), X87Ordering::Equal);
        assert_eq!(
            X87Chop53::store_f32(X87Chop53::sub(positive, positive))
                .unwrap()
                .bits(),
            0x0000_0000,
        );
    }

    #[test]
    fn i32_to_f32_store_chops_at_the_24_bit_boundary() {
        let positive = X87Chop53::load_i32(16_777_217);
        let negative = X87Chop53::load_i32(-16_777_217);
        assert_eq!(X87Chop53::store_f32(positive).unwrap().bits(), 0x4b80_0000);
        assert_eq!(X87Chop53::store_f32(negative).unwrap().bits(), 0xcb80_0000);
    }

    #[test]
    fn pc53_addition_chops_half_ulp_and_keeps_full_ulp() {
        let one = f64_value(0x3ff0_0000_0000_0000);
        let half_ulp = f64_value(0x3ca0_0000_0000_0000);
        let full_ulp = f64_value(0x3cb0_0000_0000_0000);
        assert_eq!(
            X87Chop53::store_f64(X87Chop53::add(one, half_ulp))
                .unwrap()
                .bits(),
            0x3ff0_0000_0000_0000,
        );
        assert_eq!(
            X87Chop53::store_f64(X87Chop53::add(one, full_ulp))
                .unwrap()
                .bits(),
            0x3ff0_0000_0000_0001,
        );
    }

    #[test]
    fn subtraction_and_double_gravity_have_explicit_f32_boundaries() {
        let zero = f32_value(0x0000_0000);
        let gravity = f32_value(0x40c0_0000);
        let persistent = X87Chop53::sub(zero, gravity);
        let persistent_bits = X87Chop53::store_f32(persistent).unwrap();
        let probe = X87Chop53::sub(X87Chop53::load_f32(persistent_bits).unwrap(), gravity);
        assert_eq!(persistent_bits.bits(), 0xc0c0_0000);
        assert_eq!(X87Chop53::store_f32(probe).unwrap().bits(), 0xc140_0000);
    }

    #[test]
    fn multiplication_and_compare_use_chopped_53_bit_values() {
        let half = f64_value(0x3fe0_0000_0000_0000);
        let quarter = X87Chop53::mul(half, half);
        assert_eq!(
            X87Chop53::store_f64(quarter).unwrap().bits(),
            0x3fd0_0000_0000_0000,
        );
        assert_eq!(X87Chop53::compare(quarter, half), X87Ordering::Less);
    }

    #[test]
    fn ftol_chops_positive_and_negative_values_toward_zero() {
        assert_eq!(X87Chop53::ftol_i64(f64_value(0x400e_0000_0000_0000)).unwrap(), 3);
        assert_eq!(X87Chop53::ftol_i64(f64_value(0xc00e_0000_0000_0000)).unwrap(), -3);
    }

    #[test]
    fn unverified_exceptional_domains_return_typed_errors() {
        assert_eq!(
            X87Chop53::load_f32(NativeF32Bits::from_bits(0x7f80_0000)),
            Err(NativeX87Error::NonFiniteInput { format: "f32" }),
        );
        assert_eq!(
            X87Chop53::load_f64(NativeF64Bits::from_bits(0x0000_0000_0000_0001)),
            Err(NativeX87Error::SubnormalInput { format: "f64" }),
        );
    }
}
```

**Step 2: Verify the focused arithmetic suite**

Before Cargo, run:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
```

If another session owns Cargo, wait for it to finish. Then run:

```powershell
cargo test -p vera20k native_x87 -- --nocapture
```

Expected literal result: `test result: ok.` with all seven `native_x87` tests passing.

### Task 3: Add behavior-specific Spark state and raw deterministic hashing

**Why:** Spark needs three independent raw velocities, a signed index, start RGB, and a raw accumulator without changing the generic fields used by Smoke/Gas/Fire.

**Files:**

- Modify: `src/sim/particles/mod.rs:1-86`
- Modify: `src/sim/particles/fire.rs:67-93`
- Modify: `src/sim/particles/gas.rs:208-234`
- Modify: `src/sim/particles/smoke.rs:180-206`
- Modify: `src/sim/particles/spawn.rs:103-125`
- Modify: `src/sim/particles/system_ai.rs:239-265` (test literal only)
- Modify: `src/sim/world/world_hash.rs:130-155,866-953`

**Pattern:** Optional behavior-specific state on the existing `Particle`, preserving the BTreeMap system owner and Vec particle order. This modifies a shared struct interface; every `Particle { ... }` literal depends on it.

**Step 1: Re-read the shared-file diff before editing**

Run:

```powershell
git diff -- src/sim/world/world_hash.rs
rg -n "Particle\s*\{" src
```

Preserve every unrelated hunk. If another session has changed `hash_particle_systems` itself, stop this task until that owner finishes; do not overwrite or restyle its work.

**Step 2: Define the state and field**

In `src/sim/particles/mod.rs`, add imports and the module declaration:

```rust
use crate::util::native_x87::{NativeF32Bits, NativeF64Bits};
use serde::{Deserialize, Serialize};

pub mod spark;
```

Replace the stale Tier-3 module comment with:

```rust
//! Spark compatibility state and pure kernels exist, but public Spark/Railgun
//! spawn and production dispatch remain unavailable until their activation gates close.
```

Define this type immediately before `Particle`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SparkRuntimeState {
    pub velocity_x: NativeF32Bits,
    pub velocity_y: NativeF32Bits,
    pub velocity_z: NativeF32Bits,
    pub start_rgb: [u8; 3],
    pub color_index: i32,
    pub color_accumulator: NativeF64Bits,
}
```

Add this field to `Particle` immediately after the existing generic color fields:

```rust
    /// Authoritative behavior-3 state. Generic direction/velocity/color fields
    /// remain authoritative for the existing Smoke/Gas/Fire implementations only.
    pub spark: Option<SparkRuntimeState>,
```

**Step 3: Update every existing constructor without activating Spark**

Add the following initializer next to `color_accumulator` in each literal reported by `rg -n "Particle\s*\{" src` in `fire.rs`, `gas.rs`, `smoke.rs`, `spawn.rs`, `system_ai.rs`, and `world_hash.rs`:

```rust
        spark: None,
```

Do not change the rejection match in `spawn_particle_system` or the Spark/Railgun no-op match in `system_ai.rs`.

**Step 4: Hash raw Spark state after the existing particle fields**

Insert this code inside `hash_particle_systems`, after `p.marked_for_deletion.hash(hasher);`:

```rust
                match p.spark {
                    None => 0_u8.hash(hasher),
                    Some(spark) => {
                        1_u8.hash(hasher);
                        spark.velocity_x.bits().hash(hasher);
                        spark.velocity_y.bits().hash(hasher);
                        spark.velocity_z.bits().hash(hasher);
                        spark.start_rgb.hash(hasher);
                        spark.color_index.hash(hasher);
                        spark.color_accumulator.bits().hash(hasher);
                    }
                }
```

This hashes a presence tag so `None` cannot collide structurally with a zero-valued `Some` state. Do not numerically normalize signed zero.

**Step 5: Add complete raw-state sensitivity tests**

Extend `particle_hash_tests` imports:

```rust
    use crate::rules::particle_type::ParticleTypeId;
    use crate::sim::particles::{Particle, SparkRuntimeState};
    use crate::util::native_x87::{NativeF32Bits, NativeF64Bits};
```

Add these helpers and tests inside `particle_hash_tests`:

```rust
    fn particle_with_spark(spark: Option<SparkRuntimeState>) -> Particle {
        Particle {
            type_id: ParticleTypeId(0),
            coords: IVec3::new(-1, 2, 3),
            previous_coords: IVec3::ZERO,
            origin: IVec3::ZERO,
            direction: [SimFixed::from_num(0); 3],
            velocity: SimFixed::from_num(0),
            lifetime_remaining: 9,
            damage_counter: 0,
            state_ai_advance: 0,
            animation_state: 0,
            translucency: 0,
            hit_ground: false,
            marked_for_deletion: false,
            drift_x: 0,
            drift_y: 0,
            drift_z: 0,
            current_color: [0; 3],
            color_index: 0,
            color_accumulator: SimFixed::from_num(0),
            spark,
            prev_delta: [SimFixed::from_num(0); 3],
            state_advance_counter: 0,
        }
    }

    fn hash_with_particle(particle: Particle) -> u64 {
        let mut sim = Simulation::new();
        let mut system = fake_system(IVec3::ZERO);
        system.particles.push(particle);
        sim.particle_systems.insert(system);
        sim.state_hash()
    }

    #[test]
    fn every_raw_spark_field_changes_the_state_hash() {
        let base = SparkRuntimeState {
            velocity_x: NativeF32Bits::from_bits(0x0000_0000),
            velocity_y: NativeF32Bits::from_bits(0x3f80_0000),
            velocity_z: NativeF32Bits::from_bits(0xc0c0_0000),
            start_rgb: [80, 255, 255],
            color_index: 0,
            color_accumulator: NativeF64Bits::POSITIVE_ZERO,
        };
        let base_hash = hash_with_particle(particle_with_spark(Some(base)));
        let variants = [
            SparkRuntimeState {
                velocity_x: NativeF32Bits::NEGATIVE_ZERO,
                ..base
            },
            SparkRuntimeState {
                velocity_y: NativeF32Bits::from_bits(0x4000_0000),
                ..base
            },
            SparkRuntimeState {
                velocity_z: NativeF32Bits::from_bits(0xc100_0000),
                ..base
            },
            SparkRuntimeState {
                start_rgb: [255, 255, 100],
                ..base
            },
            SparkRuntimeState {
                color_index: -1,
                ..base
            },
            SparkRuntimeState {
                color_accumulator: NativeF64Bits::NEGATIVE_ZERO,
                ..base
            },
        ];
        for variant in variants {
            assert_ne!(base_hash, hash_with_particle(particle_with_spark(Some(variant))));
        }
        assert_ne!(base_hash, hash_with_particle(particle_with_spark(None)));
    }

    #[test]
    fn spark_coordinate_lifetime_and_delete_state_remain_hashed() {
        let state = SparkRuntimeState {
            velocity_x: NativeF32Bits::POSITIVE_ZERO,
            velocity_y: NativeF32Bits::POSITIVE_ZERO,
            velocity_z: NativeF32Bits::POSITIVE_ZERO,
            start_rgb: [0; 3],
            color_index: 0,
            color_accumulator: NativeF64Bits::POSITIVE_ZERO,
        };
        let base = particle_with_spark(Some(state));
        let base_hash = hash_with_particle(base.clone());

        let mut changed = base.clone();
        changed.coords.x = 0;
        assert_ne!(base_hash, hash_with_particle(changed));

        let mut changed = base.clone();
        changed.lifetime_remaining = 8;
        assert_ne!(base_hash, hash_with_particle(changed));

        let mut changed = base;
        changed.marked_for_deletion = true;
        assert_ne!(base_hash, hash_with_particle(changed));
    }
```

Also add `spark: None,` to the existing `make_p` closure in `state_advance_counter_changes_hash`.
Remove that test's now-redundant function-local imports of `ParticleTypeId` and `Particle`; the module-level imports above serve both existing and new tests.

**Step 6: Verify constructor coverage and hashing**

Run:

```powershell
rg -n -U "Particle\s*\{(?s:.*?)state_advance_counter:" src/sim/particles src/sim/world/world_hash.rs
cargo test -p vera20k particle_hash_tests -- --nocapture
```

Inspect every matched literal and require one explicit `spark:` initializer. Expected literal Cargo result: `test result: ok.`

### Task 4: Implement the pure injected-input Spark tick kernel

**Why:** This isolates the verified behavior-3 arithmetic and branch table from the still-unproved map, bridge, parser, RNG-owner, and production-dispatch adapters.

**Files:**

- Create: `src/sim/particles/spark.rs`

**Pattern:** Pure subsystem helper over authoritative state plus immutable facts, similar to other bounded `sim` mechanism modules. The injected-fact boundary is deliberate; it is not a new world-query abstraction and it allocates nothing.

**Step 1: Create the complete kernel**

Create `src/sim/particles/spark.rs`:

```rust
//! Pure behavior-3 Spark arithmetic and collision kernel.
//!
//! Production dispatch remains disabled. Callers must supply already-resolved
//! native-frame collision facts and the one gameplay RNG sample for this tick.

use glam::IVec3;
use thiserror::Error;

use super::{Particle, SparkRuntimeState};
use crate::util::native_x87::{
    NativeF32Bits, NativeF64Bits, NativeX87Error, X87Chop53, X87Ordering, X87Value,
};

const STRUCTURAL_BRIDGE_HEIGHT: i32 = 360;
const ASCENDING_BRIDGE_DELETE_OFFSET: i32 = 20;
const GROUND_CLAMP_DEPTH: i32 = 100;
const BUILDING_CONTACT_HEIGHT_F32: NativeF32Bits =
    NativeF32Bits::from_bits(0x4316_0000);
const MAX_COLOR_RNG_SAMPLE: u32 = 0x7fff_fffe;
const COLOR_RNG_RECIPROCAL: NativeF64Bits =
    NativeF64Bits::from_bits(0x3e00_0000_0040_0000);
const COLOR_JITTER_SCALE: NativeF64Bits =
    NativeF64Bits::from_bits(0x3fa9_9999_9999_999a);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparkCollisionFacts {
    pub ground_z: i32,
    pub slope_matrix: [NativeF32Bits; 12],
    pub old_has_structural_bridge: bool,
    pub candidate_has_structural_bridge: bool,
    pub accepted_building: bool,
    pub wall_overlay_id: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparkCollisionKind {
    DescendingBridge,
    AscendingBridge,
    BelowGroundNear,
    BelowGroundDeep,
    Building,
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparkMotionStep {
    pub old_coords: IVec3,
    pub candidate_coords: IVec3,
    pub candidate_f32: [NativeF32Bits; 3],
    pub persistent_velocity: [NativeF32Bits; 3],
    pub probe_velocity: [NativeF32Bits; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparkCollisionResolution {
    pub committed_coords: IVec3,
    pub kind: Option<SparkCollisionKind>,
    pub transient_reflection: Option<[NativeF32Bits; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparkTickInputs {
    pub gravity: NativeF32Bits,
    pub color_speed: NativeF64Bits,
    pub color_rng_sample: u32,
    pub color_count: usize,
    pub collision: SparkCollisionFacts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparkTickResult {
    pub motion: SparkMotionStep,
    pub collision_kind: Option<SparkCollisionKind>,
    pub transient_reflection: Option<[NativeF32Bits; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SparkKernelError {
    #[error("behavior-3 tick requires SparkRuntimeState")]
    MissingRuntimeState,
    #[error("Spark ColorList count {0} is outside the safe valid-state boundary")]
    InvalidColorCount(usize),
    #[error("Spark color RNG sample {0:#x} is outside 0..=0x7ffffffe")]
    InvalidColorRngSample(u32),
    #[error(transparent)]
    NativeX87(#[from] NativeX87Error),
}

pub fn lepton_to_cell_trunc(value: i32) -> i32 {
    value / 256
}

pub fn integrate_motion(
    coords: IVec3,
    spark: SparkRuntimeState,
    gravity: NativeF32Bits,
) -> Result<SparkMotionStep, SparkKernelError> {
    let gravity_value = X87Chop53::load_f32(gravity)?;
    let old_vz = X87Chop53::load_f32(spark.velocity_z)?;
    let stored_vz = X87Chop53::store_f32(X87Chop53::sub(old_vz, gravity_value))?;
    let probe_z = X87Chop53::store_f32(X87Chop53::sub(
        X87Chop53::load_f32(stored_vz)?,
        gravity_value,
    ))?;

    let probe_velocity = [spark.velocity_x, spark.velocity_y, probe_z];
    let (old_x, candidate_x, candidate_x_f32) =
        motion_component(coords.x, probe_velocity[0])?;
    let (old_y, candidate_y, candidate_y_f32) =
        motion_component(coords.y, probe_velocity[1])?;
    let (old_z, candidate_z, candidate_z_f32) =
        motion_component(coords.z, probe_velocity[2])?;

    Ok(SparkMotionStep {
        old_coords: IVec3::new(old_x, old_y, old_z),
        candidate_coords: IVec3::new(candidate_x, candidate_y, candidate_z),
        candidate_f32: [candidate_x_f32, candidate_y_f32, candidate_z_f32],
        persistent_velocity: [spark.velocity_x, spark.velocity_y, stored_vz],
        probe_velocity,
    })
}

fn motion_component(
    old_coordinate: i32,
    displacement: NativeF32Bits,
) -> Result<(i32, i32, NativeF32Bits), SparkKernelError> {
    let old_stored = X87Chop53::store_f32(X87Chop53::load_i32(old_coordinate))?;
    let old_value = X87Chop53::load_f32(old_stored)?;
    let old_round_trip = X87Chop53::ftol_i64(old_value)? as i32;
    let candidate = X87Chop53::add(old_value, X87Chop53::load_f32(displacement)?);
    let candidate_stored = X87Chop53::store_f32(candidate)?;
    let candidate_round_trip =
        X87Chop53::ftol_i64(X87Chop53::load_f32(candidate_stored)?)? as i32;
    Ok((old_round_trip, candidate_round_trip, candidate_stored))
}

pub fn resolve_collision(
    motion: SparkMotionStep,
    facts: SparkCollisionFacts,
) -> Result<SparkCollisionResolution, SparkKernelError> {
    let old_z = motion.old_coords.z;
    let candidate_z_integer = motion.candidate_coords.z;
    let candidate_z = X87Chop53::load_f32(motion.candidate_f32[2])?;
    let ground_z = facts.ground_z;
    let ground_exact = X87Chop53::load_i32(ground_z);
    let bridge_plane = ground_z.wrapping_add(STRUCTURAL_BRIDGE_HEIGHT);
    let structural = facts.old_has_structural_bridge || facts.candidate_has_structural_bridge;

    let bridge_kind = if structural
        && candidate_z_integer < bridge_plane
        && old_z >= bridge_plane
    {
        Some(SparkCollisionKind::DescendingBridge)
    } else if structural && candidate_z_integer >= bridge_plane && old_z < bridge_plane {
        Some(SparkCollisionKind::AscendingBridge)
    } else {
        None
    };

    // Native performs the contact-band gate against the retained raw candidate
    // and an exact FILD ground value before its final collision-selection block.
    let contact_kind = if bridge_kind.is_none()
        && X87Chop53::compare(candidate_z, ground_exact) != X87Ordering::Less
    {
        let contact_floor = X87Chop53::sub(
            candidate_z,
            X87Chop53::load_f32(BUILDING_CONTACT_HEIGHT_F32)?,
        );
        if X87Chop53::compare(contact_floor, ground_exact) == X87Ordering::Less {
            if facts.accepted_building {
                Some(SparkCollisionKind::Building)
            } else if matches!(
                facts.wall_overlay_id,
                Some(0x02) | Some(0x1a) | Some(0xf3)
            ) {
                Some(SparkCollisionKind::Wall)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let ground_stored_bits = X87Chop53::store_f32(ground_exact)?;
    let ground_stored = X87Chop53::load_f32(ground_stored_bits)?;

    let (committed_z_bits, kind) = match bridge_kind {
        Some(SparkCollisionKind::DescendingBridge) => (
            integer_as_stored_f32(bridge_plane)?,
            Some(SparkCollisionKind::DescendingBridge),
        ),
        Some(SparkCollisionKind::AscendingBridge) => (
            integer_as_stored_f32(
                bridge_plane.wrapping_sub(ASCENDING_BRIDGE_DELETE_OFFSET),
            )?,
            Some(SparkCollisionKind::AscendingBridge),
        ),
        _ if X87Chop53::compare(candidate_z, ground_stored) == X87Ordering::Less => {
            let clamp_boundary =
                X87Chop53::load_i32(ground_z.wrapping_sub(GROUND_CLAMP_DEPTH));
            if X87Chop53::compare(clamp_boundary, candidate_z) == X87Ordering::Less {
                (
                    ground_stored_bits,
                    Some(SparkCollisionKind::BelowGroundNear),
                )
            } else {
                (
                    motion.candidate_f32[2],
                    Some(SparkCollisionKind::BelowGroundDeep),
                )
            }
        }
        _ if contact_kind.is_some() => (ground_stored_bits, contact_kind),
        _ => (motion.candidate_f32[2], None),
    };

    let transient_reflection = if kind.is_some() {
        Some(reflect_slope_vector(
            motion.probe_velocity,
            facts.slope_matrix,
        )?)
    } else {
        None
    };

    Ok(SparkCollisionResolution {
        committed_coords: IVec3::new(
            ftol_f32_to_i32(motion.candidate_f32[0])?,
            ftol_f32_to_i32(motion.candidate_f32[1])?,
            ftol_f32_to_i32(committed_z_bits)?,
        ),
        kind,
        transient_reflection,
    })
}

fn integer_as_stored_f32(value: i32) -> Result<NativeF32Bits, NativeX87Error> {
    X87Chop53::store_f32(X87Chop53::load_i32(value))
}

fn ftol_f32_to_i32(value: NativeF32Bits) -> Result<i32, NativeX87Error> {
    Ok(X87Chop53::ftol_i64(X87Chop53::load_f32(value)?)? as i32)
}

pub fn reflect_slope_vector(
    probe_velocity: [NativeF32Bits; 3],
    slope_matrix: [NativeF32Bits; 12],
) -> Result<[NativeF32Bits; 3], SparkKernelError> {
    let axis_probe = [
        probe_velocity[0],
        negate_f32(probe_velocity[1])?,
        probe_velocity[2],
    ];
    let inverse = inverse_orthonormal_matrix(slope_matrix)?;
    let inverse_result = matrix_vector(inverse, axis_probe)?;
    let mut local = [
        multiply_store_f32(inverse_result[0], NativeF32Bits::ONE)?,
        multiply_store_f32(inverse_result[1], NativeF32Bits::ONE)?,
        multiply_store_f32(inverse_result[2], NativeF32Bits::ONE)?,
    ];
    local[2] = negate_f32(local[2])?;
    let mut reflected = matrix_vector(slope_matrix, local)?;
    reflected[1] = negate_f32(reflected[1])?;
    Ok(reflected)
}

fn inverse_orthonormal_matrix(
    matrix: [NativeF32Bits; 12],
) -> Result<[NativeF32Bits; 12], SparkKernelError> {
    let zero = NativeF32Bits::POSITIVE_ZERO;
    let mut inverse = [zero; 12];
    inverse[0] = matrix[0];
    inverse[1] = matrix[4];
    inverse[2] = matrix[8];
    inverse[4] = matrix[1];
    inverse[5] = matrix[5];
    inverse[6] = matrix[9];
    inverse[8] = matrix[2];
    inverse[9] = matrix[6];
    inverse[10] = matrix[10];
    inverse[3] = negative_ordered_product_sum([
        (matrix[0], matrix[3]),
        (matrix[8], matrix[11]),
        (matrix[4], matrix[7]),
    ])?;
    inverse[7] = negative_ordered_product_sum([
        (matrix[1], matrix[3]),
        (matrix[9], matrix[11]),
        (matrix[5], matrix[7]),
    ])?;
    inverse[11] = negative_ordered_product_sum([
        (matrix[2], matrix[3]),
        (matrix[10], matrix[11]),
        (matrix[6], matrix[7]),
    ])?;
    Ok(inverse)
}

fn negative_ordered_product_sum(
    pairs: [(NativeF32Bits, NativeF32Bits); 3],
) -> Result<NativeF32Bits, SparkKernelError> {
    let first = multiply_value(pairs[0].0, pairs[0].1)?;
    let second = multiply_value(pairs[1].0, pairs[1].1)?;
    let third = multiply_value(pairs[2].0, pairs[2].1)?;
    let partial = X87Chop53::add(first, second);
    X87Chop53::store_f32(X87Chop53::neg(X87Chop53::add(partial, third)))
        .map_err(SparkKernelError::from)
}

fn matrix_vector(
    matrix: [NativeF32Bits; 12],
    vector: [NativeF32Bits; 3],
) -> Result<[NativeF32Bits; 3], SparkKernelError> {
    Ok([
        ordered_product_sum([
            (matrix[1], vector[1]),
            (matrix[2], vector[2]),
            (matrix[0], vector[0]),
        ])?,
        ordered_product_sum([
            (matrix[5], vector[1]),
            (matrix[4], vector[0]),
            (matrix[6], vector[2]),
        ])?,
        ordered_product_sum([
            (matrix[9], vector[1]),
            (matrix[8], vector[0]),
            (matrix[10], vector[2]),
        ])?,
    ])
}

fn ordered_product_sum(
    pairs: [(NativeF32Bits, NativeF32Bits); 3],
) -> Result<NativeF32Bits, SparkKernelError> {
    let first = multiply_value(pairs[0].0, pairs[0].1)?;
    let second = multiply_value(pairs[1].0, pairs[1].1)?;
    let third = multiply_value(pairs[2].0, pairs[2].1)?;
    let partial = X87Chop53::add(first, second);
    X87Chop53::store_f32(X87Chop53::add(partial, third))
        .map_err(SparkKernelError::from)
}

fn multiply_value(
    lhs: NativeF32Bits,
    rhs: NativeF32Bits,
) -> Result<X87Value, SparkKernelError> {
    Ok(X87Chop53::mul(
        X87Chop53::load_f32(lhs)?,
        X87Chop53::load_f32(rhs)?,
    ))
}

fn multiply_store_f32(
    lhs: NativeF32Bits,
    rhs: NativeF32Bits,
) -> Result<NativeF32Bits, SparkKernelError> {
    X87Chop53::store_f32(multiply_value(lhs, rhs)?).map_err(SparkKernelError::from)
}

fn negate_f32(value: NativeF32Bits) -> Result<NativeF32Bits, SparkKernelError> {
    X87Chop53::store_f32(X87Chop53::neg(X87Chop53::load_f32(value)?))
        .map_err(SparkKernelError::from)
}

pub fn advance_color(
    spark: &mut SparkRuntimeState,
    color_speed: NativeF64Bits,
    color_rng_sample: u32,
    color_count: usize,
) -> Result<(), SparkKernelError> {
    if color_rng_sample > MAX_COLOR_RNG_SAMPLE {
        return Err(SparkKernelError::InvalidColorRngSample(color_rng_sample));
    }
    let Ok(color_count_i32) = i32::try_from(color_count) else {
        return Err(SparkKernelError::InvalidColorCount(color_count));
    };
    if color_count_i32 < 2 {
        return Err(SparkKernelError::InvalidColorCount(color_count));
    }

    let scaled_rng = X87Chop53::mul(
        X87Chop53::load_i32(color_rng_sample as i32),
        X87Chop53::load_f64(COLOR_RNG_RECIPROCAL)?,
    );
    let jitter = X87Chop53::mul(
        scaled_rng,
        X87Chop53::load_f64(COLOR_JITTER_SCALE)?,
    );
    let with_speed = X87Chop53::add(jitter, X87Chop53::load_f64(color_speed)?);
    let accumulated = X87Chop53::add(
        with_speed,
        X87Chop53::load_f64(spark.color_accumulator)?,
    );
    spark.color_accumulator = X87Chop53::store_f64(accumulated)?;

    let stored = X87Chop53::load_f64(spark.color_accumulator)?;
    let one = X87Chop53::load_f64(NativeF64Bits::ONE)?;
    if X87Chop53::compare(stored, one) == X87Ordering::Greater {
        if spark.color_index < color_count_i32.wrapping_sub(2) {
            spark.color_index = spark.color_index.wrapping_add(1);
            spark.color_accumulator = NativeF64Bits::POSITIVE_ZERO;
        } else {
            spark.color_accumulator = NativeF64Bits::ONE;
        }
    }
    Ok(())
}

pub fn tick_particle_with_facts(
    particle: &mut Particle,
    inputs: SparkTickInputs,
) -> Result<SparkTickResult, SparkKernelError> {
    let mut spark = particle.spark.ok_or(SparkKernelError::MissingRuntimeState)?;
    let motion = integrate_motion(particle.coords, spark, inputs.gravity)?;
    let collision = resolve_collision(motion, inputs.collision)?;
    spark.velocity_x = motion.persistent_velocity[0];
    spark.velocity_y = motion.persistent_velocity[1];
    spark.velocity_z = motion.persistent_velocity[2];
    advance_color(
        &mut spark,
        inputs.color_speed,
        inputs.color_rng_sample,
        inputs.color_count,
    )?;
    let lifetime = particle.lifetime_remaining.wrapping_sub(1);

    particle.coords = collision.committed_coords;
    particle.spark = Some(spark);
    particle.lifetime_remaining = lifetime;
    particle.marked_for_deletion = particle.marked_for_deletion
        || collision.kind.is_some()
        || lifetime == 0;

    Ok(SparkTickResult {
        motion,
        collision_kind: collision.kind,
        transient_reflection: collision.transient_reflection,
    })
}
```

**Step 2: Add complete mechanism tests in the same file**

Append:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::particle_type::ParticleTypeId;
    use crate::util::fixed_math::SimFixed;

    const F32_ZERO: NativeF32Bits = NativeF32Bits::POSITIVE_ZERO;
    const F32_ONE: NativeF32Bits = NativeF32Bits::ONE;
    const F32_SIX: NativeF32Bits = NativeF32Bits::from_bits(0x40c0_0000);
    const F64_ZERO: NativeF64Bits = NativeF64Bits::POSITIVE_ZERO;
    const F64_HALF: NativeF64Bits = NativeF64Bits::HALF;

    fn identity_matrix() -> [NativeF32Bits; 12] {
        [
            F32_ONE, F32_ZERO, F32_ZERO, F32_ZERO,
            F32_ZERO, F32_ONE, F32_ZERO, F32_ZERO,
            F32_ZERO, F32_ZERO, F32_ONE, F32_ZERO,
        ]
    }

    fn facts(ground_z: i32) -> SparkCollisionFacts {
        SparkCollisionFacts {
            ground_z,
            slope_matrix: identity_matrix(),
            old_has_structural_bridge: false,
            candidate_has_structural_bridge: false,
            accepted_building: false,
            wall_overlay_id: None,
        }
    }

    fn spark_state(vz: NativeF32Bits) -> SparkRuntimeState {
        SparkRuntimeState {
            velocity_x: F32_ZERO,
            velocity_y: F32_ZERO,
            velocity_z: vz,
            start_rgb: [80, 255, 255],
            color_index: 0,
            color_accumulator: F64_ZERO,
        }
    }

    fn particle(coords: IVec3, vz: NativeF32Bits, lifetime: i16) -> Particle {
        Particle {
            type_id: ParticleTypeId(0),
            coords,
            previous_coords: coords,
            origin: coords,
            direction: [SimFixed::from_num(0); 3],
            velocity: SimFixed::from_num(0),
            lifetime_remaining: lifetime,
            damage_counter: 0,
            state_ai_advance: 0,
            animation_state: 0,
            translucency: 0,
            hit_ground: false,
            marked_for_deletion: false,
            drift_x: 0,
            drift_y: 0,
            drift_z: 0,
            current_color: [0; 3],
            color_index: 0,
            color_accumulator: SimFixed::from_num(0),
            spark: Some(spark_state(vz)),
            prev_delta: [SimFixed::from_num(0); 3],
            state_advance_counter: 0,
        }
    }

    fn stored_f32(value: i32) -> NativeF32Bits {
        X87Chop53::store_f32(X87Chop53::load_i32(value)).unwrap()
    }

    fn motion_with_candidate_f32(
        old_z: i32,
        candidate_z: i32,
        candidate_z_f32: NativeF32Bits,
    ) -> SparkMotionStep {
        SparkMotionStep {
            old_coords: IVec3::new(0, 0, old_z),
            candidate_coords: IVec3::new(0, 0, candidate_z),
            candidate_f32: [F32_ZERO, F32_ZERO, candidate_z_f32],
            persistent_velocity: [F32_ZERO; 3],
            probe_velocity: [F32_ZERO; 3],
        }
    }

    fn motion(old_z: i32, candidate_z: i32) -> SparkMotionStep {
        motion_with_candidate_f32(old_z, candidate_z, stored_f32(candidate_z))
    }

    #[test]
    fn flat_ground_trace_preserves_double_gravity_and_commit_order() {
        let mut particle = particle(IVec3::new(2560, 2560, 10), F32_ZERO, 2);
        let result = tick_particle_with_facts(
            &mut particle,
            SparkTickInputs {
                gravity: F32_SIX,
                color_speed: F64_ZERO,
                color_rng_sample: 0,
                color_count: 5,
                collision: facts(0),
            },
        )
        .unwrap();
        assert_eq!(result.motion.persistent_velocity[2].bits(), 0xc0c0_0000);
        assert_eq!(result.motion.probe_velocity[2].bits(), 0xc140_0000);
        assert_eq!(result.motion.candidate_coords, IVec3::new(2560, 2560, -2));
        assert_eq!(particle.coords, IVec3::new(2560, 2560, 0));
        assert_eq!(result.collision_kind, Some(SparkCollisionKind::BelowGroundNear));
        assert!(particle.marked_for_deletion);
        assert_eq!(particle.lifetime_remaining, 1);
    }

    #[test]
    fn signed_leptons_truncate_toward_zero_at_cell_boundaries() {
        assert_eq!(lepton_to_cell_trunc(-1), 0);
        assert_eq!(lepton_to_cell_trunc(-255), 0);
        assert_eq!(lepton_to_cell_trunc(-256), -1);
        assert_eq!(lepton_to_cell_trunc(255), 0);
        assert_eq!(lepton_to_cell_trunc(256), 1);
    }

    #[test]
    fn structural_bridge_predicates_keep_their_equality_sides() {
        let mut structural = facts(0);
        structural.old_has_structural_bridge = true;
        let descending = resolve_collision(motion(370, 358), structural).unwrap();
        assert_eq!(descending.committed_coords.z, 360);
        assert_eq!(descending.kind, Some(SparkCollisionKind::DescendingBridge));

        let ascending = resolve_collision(motion(350, 368), structural).unwrap();
        assert_eq!(ascending.committed_coords.z, 340);
        assert_eq!(ascending.kind, Some(SparkCollisionKind::AscendingBridge));

        let equality = resolve_collision(motion(370, 360), structural).unwrap();
        assert_eq!(equality.kind, None);
        assert_eq!(equality.committed_coords.z, 360);
    }

    #[test]
    fn ground_and_contact_height_boundaries_are_strict() {
        let near = resolve_collision(motion(0, -99), facts(0)).unwrap();
        assert_eq!(near.committed_coords.z, 0);
        assert_eq!(near.kind, Some(SparkCollisionKind::BelowGroundNear));

        let exact_deep = resolve_collision(motion(0, -100), facts(0)).unwrap();
        assert_eq!(exact_deep.committed_coords.z, -100);
        assert_eq!(exact_deep.kind, Some(SparkCollisionKind::BelowGroundDeep));

        let mut building = facts(0);
        building.accepted_building = true;
        assert_eq!(
            resolve_collision(motion(0, 149), building).unwrap().kind,
            Some(SparkCollisionKind::Building),
        );
        assert_eq!(resolve_collision(motion(0, 150), building).unwrap().kind, None);
    }

    #[test]
    fn fractional_candidate_remains_below_ground_until_final_ftol() {
        let result = resolve_collision(
            motion_with_candidate_f32(
                0,
                0,
                NativeF32Bits::from_bits(0xbf00_0000),
            ),
            facts(0),
        )
        .unwrap();
        assert_eq!(result.kind, Some(SparkCollisionKind::BelowGroundNear));
        assert_eq!(result.committed_coords.z, 0);
    }

    #[test]
    fn wall_fallback_accepts_only_the_three_native_overlay_ids() {
        for overlay in [0x02, 0x1a, 0xf3] {
            let mut wall = facts(0);
            wall.wall_overlay_id = Some(overlay);
            assert_eq!(
                resolve_collision(motion(0, 100), wall).unwrap().kind,
                Some(SparkCollisionKind::Wall),
            );
        }
        for overlay in [0x01, 0x03, 0x19, 0x1b, 0xf2, 0xf4] {
            let mut wall = facts(0);
            wall.wall_overlay_id = Some(overlay);
            assert_eq!(resolve_collision(motion(0, 100), wall).unwrap().kind, None);
        }
    }

    #[test]
    fn identity_slope_reflects_probe_z_but_never_replaces_persistent_velocity() {
        let reflected = reflect_slope_vector(
            [F32_ZERO, F32_ZERO, NativeF32Bits::from_bits(0xc140_0000)],
            identity_matrix(),
        )
        .unwrap();
        assert_eq!(reflected[0].bits(), 0x0000_0000);
        assert_eq!(reflected[1].bits(), 0x8000_0000);
        assert_eq!(reflected[2].bits(), 0x4140_0000);
    }

    #[test]
    fn matrix_vector_keeps_native_non_associative_dot_order() {
        let matrix = [
            NativeF32Bits::ONE,
            NativeF32Bits::from_bits(0x7180_0000),
            NativeF32Bits::from_bits(0xf180_0000),
            F32_ZERO,
            F32_ZERO,
            F32_ZERO,
            F32_ZERO,
            F32_ZERO,
            F32_ZERO,
            F32_ZERO,
            F32_ZERO,
            F32_ZERO,
        ];
        let result = matrix_vector(matrix, [NativeF32Bits::ONE; 3]).unwrap();
        assert_eq!(result[0], NativeF32Bits::ONE);
    }

    #[test]
    fn color_progression_uses_strict_greater_than_and_count_minus_two() {
        let mut state = spark_state(F32_ZERO);
        state.color_accumulator = NativeF64Bits::ONE;
        advance_color(&mut state, F64_ZERO, 0, 5).unwrap();
        assert_eq!(state.color_index, 0);
        assert_eq!(state.color_accumulator, NativeF64Bits::ONE);

        advance_color(&mut state, F64_HALF, 0, 5).unwrap();
        assert_eq!(state.color_index, 1);
        assert_eq!(state.color_accumulator, NativeF64Bits::POSITIVE_ZERO);

        state.color_index = 3;
        state.color_accumulator = NativeF64Bits::ONE;
        advance_color(&mut state, F64_HALF, 0, 5).unwrap();
        assert_eq!(state.color_index, 3);
        assert_eq!(state.color_accumulator, NativeF64Bits::ONE);
    }

    #[test]
    fn lifetime_zero_wraps_to_negative_one_without_lifetime_deletion() {
        let mut particle = particle(IVec3::new(0, 0, 1000), F32_ZERO, 0);
        tick_particle_with_facts(
            &mut particle,
            SparkTickInputs {
                gravity: F32_ZERO,
                color_speed: F64_ZERO,
                color_rng_sample: 0,
                color_count: 5,
                collision: facts(0),
            },
        )
        .unwrap();
        assert_eq!(particle.lifetime_remaining, -1);
        assert!(!particle.marked_for_deletion);
    }
}
```

**Step 3: Verify the simulation boundary and focused tests**

Run:

```powershell
rg -n "use crate::(render|ui|sidebar|audio|net)|\bf32\b|\bf64\b|Vec::new|\.collect" src/sim/particles/spark.rs
cargo test -p vera20k particles::spark::tests -- --nocapture
```

Expected: no forbidden dependency, no hardware float type/value, no hot-path allocation, and literal `test result: ok.` for all ten Spark kernel tests. The `-0.5` fixture must prove that collision predicates retain candidate `f32` after the earlier integer cell/bridge conversion, and the `2^100 + -2^100 + 1` fixture must fail if the matrix dot products are reassociated.

**Step 4: Confirm production remains disabled**

Run:

```powershell
rg -n -C 3 "ParticleSystemBehavesLike::Spark|ParticleSystemBehavesLike::Railgun" src/sim/particles/spawn.rs src/sim/particles/system_ai.rs
```

Expected: public spawn still returns `None` for both behaviors and `system_ai` still does no Spark/Railgun tick dispatch.

### Task 5: Add the immutable tactical point contract and pure resolver

**Why:** The exact behavior-3 point consumer can be implemented and tested over injected u16 A/Z words and runtime values without guessing their producers or physically changing the current framebuffer.

**Files:**

- Create: `src/render/tactical_compat.rs`
- Modify: `src/render/mod.rs:1-54`

**Pattern:** Pure render data transform above `sim`, similar to other renderer-side CPU preparation helpers. It creates a new shared tactical compatibility interface but owns no GPU resources and performs no destination write.

**Step 1: Export the module**

Add to `src/render/mod.rs` in alphabetical order:

```rust
pub mod tactical_compat;
```

**Step 2: Create the complete command/frame/resolver implementation**

Create `src/render/tactical_compat.rs`:

```rust
//! Pure native-compatible tactical point consumer.
//!
//! This module consumes externally produced u16 A/Z words and runtime display
//! values. It never substitutes the R8 shroud texture or floating scene depth.

use glam::{IVec2, IVec3};
use thiserror::Error;

use crate::util::native_x87::{
    NativeF64Bits, NativeX87Error, X87Chop53, X87Value,
};

const Z_CORRECTION_THRESHOLD: i32 = 728;
const PARTICLE_Z_BIAS: i32 = 0x32;
const A_PASSTHROUGH_THRESHOLD: u16 = 127;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparkDrawGates {
    pub performance_passed: bool,
    pub extra_animations_enabled: bool,
    pub fog_passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparkPointCommand {
    pub world: IVec3,
    pub start_rgb: [u8; 3],
    pub color_index: i32,
    pub color_accumulator: NativeF64Bits,
    pub damage: i32,
    pub gates: SparkDrawGates,
    pub draw_ordinal: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct WordPlane<'a> {
    pub words: &'a [u16],
    pub width: usize,
    pub height: usize,
    pub pitch_words: usize,
    pub row_origin: i32,
}

impl WordPlane<'_> {
    pub fn sample(self, x: i32, screen_y: i32) -> Option<u16> {
        let row = screen_y.checked_sub(self.row_origin)?;
        let x = usize::try_from(x).ok()?;
        let row = usize::try_from(row).ok()?;
        if x >= self.width || row >= self.height || self.pitch_words < self.width {
            return None;
        }
        let index = row.checked_mul(self.pitch_words)?.checked_add(x)?;
        self.words.get(index).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl TacticalRect {
    pub fn contains(self, point: IVec2) -> bool {
        if self.width <= 0 || self.height <= 0 {
            return false;
        }
        let right = self.left.wrapping_add(self.width);
        let bottom = self.top.wrapping_add(self.height);
        point.x >= self.left && point.x < right && point.y >= self.top && point.y < bottom
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectDrawPixelFormat {
    pub red_loss: u32,
    pub red_shift: u32,
    pub green_loss: u32,
    pub green_shift: u32,
    pub blue_loss: u32,
    pub blue_shift: u32,
    pub destination_bytes_per_pixel: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct TacticalCompatFrame<'a> {
    pub a_plane: WordPlane<'a>,
    pub z_plane: WordPlane<'a>,
    pub clip: TacticalRect,
    pub tactical_offset_x: i32,
    pub tactical_offset_y: i32,
    pub radar_viewport_offset_y: i32,
    pub adjust_for_z_multiplier: NativeF64Bits,
    pub z_origin_term: i16,
    pub z_bottom_term: i16,
    pub pixel_format: DirectDrawPixelFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointRejectReason {
    Performance,
    ExtraAnimations,
    Fog,
    OutsideClip,
    AZero,
    ZTest,
    InvalidColorIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedPointWrite {
    pub screen: IVec2,
    pub packed_value: u16,
    pub byte_width: u8,
    pub draw_ordinal: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointResolution {
    Rejected(PointRejectReason),
    Write(PackedPointWrite),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TacticalCompatError {
    #[error("native-compatible A word is unavailable at ({x},{y})")]
    MissingAWord { x: i32, y: i32 },
    #[error("native-compatible Z word is unavailable at ({x},{y})")]
    MissingZWord { x: i32, y: i32 },
    #[error(transparent)]
    NativeX87(#[from] NativeX87Error),
}

pub fn resolve_spark_point(
    command: SparkPointCommand,
    color_list: &[[u8; 3]],
    frame: TacticalCompatFrame<'_>,
) -> Result<PointResolution, TacticalCompatError> {
    if !command.gates.performance_passed && command.damage == 0 {
        return Ok(PointResolution::Rejected(PointRejectReason::Performance));
    }
    if !command.gates.extra_animations_enabled {
        return Ok(PointResolution::Rejected(PointRejectReason::ExtraAnimations));
    }
    if !command.gates.fog_passed {
        return Ok(PointResolution::Rejected(PointRejectReason::Fog));
    }

    let screen = project_spark_point(command.world, frame)?;
    if !frame.clip.contains(screen) {
        return Ok(PointResolution::Rejected(PointRejectReason::OutsideClip));
    }

    let a_word = frame
        .a_plane
        .sample(screen.x, screen.y)
        .ok_or(TacticalCompatError::MissingAWord {
            x: screen.x,
            y: screen.y,
        })?;
    if a_word == 0 {
        return Ok(PointResolution::Rejected(PointRejectReason::AZero));
    }

    let z_word = frame
        .z_plane
        .sample(screen.x, screen.y)
        .ok_or(TacticalCompatError::MissingZWord {
            x: screen.x,
            y: screen.y,
        })?;
    let adjust_for_z = adjust_for_z(command.world.z, frame.adjust_for_z_multiplier)?;
    let candidate = z_candidate(
        frame.z_origin_term,
        frame.z_bottom_term,
        screen.y,
        adjust_for_z,
    );
    if !z_passes(candidate, z_word) {
        return Ok(PointResolution::Rejected(PointRejectReason::ZTest));
    }

    let Some((current, next)) = select_color_pair(command, color_list) else {
        return Ok(PointResolution::Rejected(PointRejectReason::InvalidColorIndex));
    };
    let interpolated = interpolate_rgb(current, next, command.color_accumulator)?;
    let modulated = modulate_rgb(interpolated, a_word);
    let packed_value = pack_rgb(modulated, frame.pixel_format);
    let byte_width = if frame.pixel_format.destination_bytes_per_pixel == 2 {
        2
    } else {
        1
    };

    Ok(PointResolution::Write(PackedPointWrite {
        screen,
        packed_value,
        byte_width,
        draw_ordinal: command.draw_ordinal,
    }))
}

pub fn project_spark_point(
    world: IVec3,
    frame: TacticalCompatFrame<'_>,
) -> Result<IVec2, TacticalCompatError> {
    let planar_x = projection_half_term(world.x, 60)
        .wrapping_add(projection_half_term(world.y, -60))
        / 256;
    let planar_y = projection_half_term(world.x, 30)
        .wrapping_add(projection_half_term(world.y, 30))
        / 256;
    let z_adjustment = adjust_for_z(world.z, frame.adjust_for_z_multiplier)?;
    Ok(IVec2::new(
        planar_x.wrapping_sub(frame.tactical_offset_x),
        planar_y
            .wrapping_sub(z_adjustment)
            .wrapping_sub(frame.tactical_offset_y)
            .wrapping_add(frame.radar_viewport_offset_y),
    ))
}

fn projection_half_term(value: i32, factor: i32) -> i32 {
    value.wrapping_mul(factor) / 2
}

pub fn adjust_for_z(
    world_z: i32,
    multiplier: NativeF64Bits,
) -> Result<i32, NativeX87Error> {
    let product = X87Chop53::mul(
        X87Chop53::load_i32(world_z),
        X87Chop53::load_f64(multiplier)?,
    );
    let correction = X87Chop53::load_i32(if world_z >= Z_CORRECTION_THRESHOLD {
        1
    } else {
        0
    });
    let corrected = X87Chop53::add(product, correction);
    let biased = X87Chop53::add(
        corrected,
        X87Chop53::load_f64(NativeF64Bits::HALF)?,
    );
    Ok(X87Chop53::ftol_i64(biased)? as i32)
}

pub fn z_candidate(
    origin_term: i16,
    bottom_term: i16,
    screen_y: i32,
    adjust_for_z: i32,
) -> i32 {
    let base = i32::from(origin_term)
        .wrapping_add(i32::from(bottom_term))
        .wrapping_sub(screen_y) as u16;
    (base as i32)
        .wrapping_sub(adjust_for_z)
        .wrapping_sub(PARTICLE_Z_BIAS)
}

pub const fn z_passes(candidate: i32, stored: u16) -> bool {
    candidate < stored as i32
}

fn select_color_pair(
    command: SparkPointCommand,
    color_list: &[[u8; 3]],
) -> Option<([u8; 3], [u8; 3])> {
    let index = usize::try_from(command.color_index).ok()?;
    if index == 0 {
        Some((command.start_rgb, *color_list.get(1)?))
    } else {
        Some((*color_list.get(index)?, *color_list.get(index.checked_add(1)?)?))
    }
}

pub fn interpolate_rgb(
    current: [u8; 3],
    next: [u8; 3],
    accumulator: NativeF64Bits,
) -> Result<[i32; 3], NativeX87Error> {
    let one_minus_a = X87Chop53::sub(
        X87Chop53::load_f64(NativeF64Bits::ONE)?,
        X87Chop53::load_f64(accumulator)?,
    );
    Ok([
        interpolate_channel(current[0], next[0], accumulator, one_minus_a)?,
        interpolate_channel(current[1], next[1], accumulator, one_minus_a)?,
        interpolate_channel(current[2], next[2], accumulator, one_minus_a)?,
    ])
}

fn interpolate_channel(
    current: u8,
    next: u8,
    accumulator: NativeF64Bits,
    one_minus_a: X87Value,
) -> Result<i32, NativeX87Error> {
    let next_term = X87Chop53::mul(
        X87Chop53::load_i32(next as i32),
        X87Chop53::load_f64(accumulator)?,
    );
    let current_term = X87Chop53::mul(X87Chop53::load_i32(current as i32), one_minus_a);
    Ok(X87Chop53::ftol_i64(X87Chop53::add(next_term, current_term))? as i32)
}

pub fn modulate_rgb(rgb: [i32; 3], a_word: u16) -> [i32; 3] {
    if a_word >= A_PASSTHROUGH_THRESHOLD {
        return rgb;
    }
    let a = a_word as i32;
    [
        rgb[0].wrapping_mul(a) >> 7,
        rgb[1].wrapping_mul(a) >> 7,
        rgb[2].wrapping_mul(a) >> 7,
    ]
}

pub fn pack_rgb(rgb: [i32; 3], format: DirectDrawPixelFormat) -> u16 {
    pack_channel(rgb[0], format.red_loss, format.red_shift)
        | pack_channel(rgb[1], format.green_loss, format.green_shift)
        | pack_channel(rgb[2], format.blue_loss, format.blue_shift)
}

fn pack_channel(channel: i32, loss: u32, shift: u32) -> u16 {
    let reduced = channel >> (loss & 31);
    (reduced.wrapping_shl(shift & 31) as u32 & 0xffff) as u16
}
```

**Step 3: Verify the resolver has no approximate resource dependency**

Run:

```powershell
rg -n "ShroudBuffer|Depth32Float|SpriteInstance|wgpu|f32|f64|RGB565|Vec<|Vec::new" src/render/tactical_compat.rs
```

Expected: no match. `NativeF64Bits` is an integer-backed raw wrapper; there is no hardware float, allocation, GPU type, assumed display tuple, or current shroud/depth adapter.

### Task 6: Add exhaustive pure point-kernel regression tables

**Why:** Projection—including native pre-division overflow staging—clip, A discontinuity, wrapped Z, color-source selection, shared x87 interpolation state, packing, gate order, and ordinal preservation are independently testable before a production compositor exists.

**Files:**

- Modify: `src/render/tactical_compat.rs` after Task 5 implementation

**Pattern:** Same-file unit tests for pure renderer transforms. Synthetic display values test data flow only and are labeled non-certifying.

**Step 1: Append the complete test module**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const ZERO_MULTIPLIER: NativeF64Bits = NativeF64Bits::POSITIVE_ZERO;
    const SYNTHETIC_565: DirectDrawPixelFormat = DirectDrawPixelFormat {
        red_loss: 3,
        red_shift: 11,
        green_loss: 2,
        green_shift: 5,
        blue_loss: 3,
        blue_shift: 0,
        destination_bytes_per_pixel: 2,
    };

    fn plane(words: &[u16], width: usize, height: usize) -> WordPlane<'_> {
        WordPlane {
            words,
            width,
            height,
            pitch_words: width,
            row_origin: 0,
        }
    }

    fn frame<'a>(a_words: &'a [u16], z_words: &'a [u16]) -> TacticalCompatFrame<'a> {
        TacticalCompatFrame {
            a_plane: plane(a_words, 64, 64),
            z_plane: plane(z_words, 64, 64),
            clip: TacticalRect {
                left: 0,
                top: 0,
                width: 64,
                height: 64,
            },
            tactical_offset_x: 0,
            tactical_offset_y: 0,
            radar_viewport_offset_y: 0,
            adjust_for_z_multiplier: ZERO_MULTIPLIER,
            z_origin_term: 0,
            z_bottom_term: 100,
            pixel_format: SYNTHETIC_565,
        }
    }

    fn command(world: IVec3) -> SparkPointCommand {
        SparkPointCommand {
            world,
            start_rgb: [80, 255, 255],
            color_index: 0,
            color_accumulator: NativeF64Bits::POSITIVE_ZERO,
            damage: 0,
            gates: SparkDrawGates {
                performance_passed: true,
                extra_animations_enabled: true,
                fog_passed: true,
            },
            draw_ordinal: 42,
        }
    }

    #[test]
    fn projection_matches_verified_signed_fixtures() {
        let a = vec![127; 64 * 64];
        let z = vec![u16::MAX; 64 * 64];
        let frame = frame(&a, &z);
        assert_eq!(
            project_spark_point(IVec3::new(256, 0, 0), frame).unwrap(),
            IVec2::new(30, 15),
        );
        assert_eq!(
            project_spark_point(IVec3::new(-1, 0, 0), frame).unwrap(),
            IVec2::new(0, 0),
        );
    }

    #[test]
    fn projection_preserves_native_wrap_before_each_half_term() {
        let a = vec![127; 64 * 64];
        let z = vec![u16::MAX; 64 * 64];
        let frame = frame(&a, &z);
        assert_eq!(
            project_spark_point(IVec3::new(50_000_000, 0, 0), frame).unwrap(),
            IVec2::new(-2_529_233, 2_929_687),
        );
        assert_eq!(
            project_spark_point(IVec3::new(i32::MAX, 1, 0), frame).unwrap(),
            IVec2::ZERO,
        );
    }

    #[test]
    fn clip_is_inclusive_left_top_and_exclusive_right_bottom() {
        let clip = TacticalRect {
            left: 10,
            top: 20,
            width: 4,
            height: 3,
        };
        assert!(clip.contains(IVec2::new(10, 20)));
        assert!(clip.contains(IVec2::new(13, 22)));
        assert!(!clip.contains(IVec2::new(14, 22)));
        assert!(!clip.contains(IVec2::new(13, 23)));
        assert!(!clip.contains(IVec2::new(9, 20)));
        assert!(!clip.contains(IVec2::new(10, 19)));
    }

    #[test]
    fn complete_a_word_threshold_domain_matches_native_discontinuity() {
        let rgb = [80, 255, 255];
        assert_eq!(modulate_rgb(rgb, 1), [0, 1, 1]);
        assert_eq!(modulate_rgb(rgb, 126), [78, 251, 251]);
        assert_eq!(modulate_rgb(rgb, 127), rgb);
        assert_eq!(modulate_rgb(rgb, 128), rgb);
        assert_eq!(modulate_rgb(rgb, 65_535), rgb);
        assert_eq!(modulate_rgb([-1, -128, 255], 1), [-1, -1, 1]);
    }

    #[test]
    fn z_uses_wrapped_u16_base_and_strict_signed_comparison() {
        let candidate = z_candidate(i16::MAX, i16::MAX, -10, 0);
        let expected_base = (i32::from(i16::MAX)
            .wrapping_add(i32::from(i16::MAX))
            .wrapping_sub(-10) as u16) as i32;
        assert_eq!(candidate, expected_base - 50);
        assert!(!z_passes(100, 99));
        assert!(!z_passes(100, 100));
        assert!(z_passes(100, 101));
        assert!(z_passes(-1, 0));
        assert!(!z_passes(65_536, u16::MAX));
    }

    #[test]
    fn interpolation_uses_start_to_list_one_then_list_pairs() {
        let colors = [
            [0, 128, 255],
            [255, 255, 255],
            [200, 200, 150],
        ];
        let zero = command(IVec3::ZERO);
        assert_eq!(
            select_color_pair(zero, &colors),
            Some(([80, 255, 255], [255, 255, 255])),
        );
        let one = SparkPointCommand {
            color_index: 1,
            ..zero
        };
        assert_eq!(
            select_color_pair(one, &colors),
            Some(([255, 255, 255], [200, 200, 150])),
        );
        assert_eq!(
            interpolate_rgb([80, 255, 255], [255, 255, 255], NativeF64Bits::POSITIVE_ZERO)
                .unwrap(),
            [80, 255, 255],
        );
        assert_eq!(
            interpolate_rgb([80, 255, 255], [255, 255, 255], NativeF64Bits::ONE).unwrap(),
            [255, 255, 255],
        );
    }

    #[test]
    fn resolver_reads_a_then_z_and_preserves_z_and_ordinal() {
        let mut a = vec![127; 64 * 64];
        let mut z = vec![u16::MAX; 64 * 64];
        let point = IVec2::new(30, 15);
        let index = point.y as usize * 64 + point.x as usize;
        a[index] = 127;
        z[index] = u16::MAX;
        let z_before = z.clone();
        let colors = [[0, 128, 255], [255, 255, 255]];
        let result = resolve_spark_point(command(IVec3::new(256, 0, 0)), &colors, frame(&a, &z))
            .unwrap();
        assert_eq!(
            result,
            PointResolution::Write(PackedPointWrite {
                screen: point,
                packed_value: 0x57ff,
                byte_width: 2,
                draw_ordinal: 42,
            }),
        );
        assert_eq!(z, z_before);
    }

    #[test]
    fn zero_a_rejects_before_color_and_no_runtime_default_is_needed() {
        let a = vec![0; 64 * 64];
        let z = vec![u16::MAX; 64 * 64];
        let invalid_colors: [[u8; 3]; 0] = [];
        assert_eq!(
            resolve_spark_point(command(IVec3::ZERO), &invalid_colors, frame(&a, &z)).unwrap(),
            PointResolution::Rejected(PointRejectReason::AZero),
        );
    }

    #[test]
    fn gate_order_and_damage_override_are_explicit() {
        let a = vec![127; 64 * 64];
        let z = vec![u16::MAX; 64 * 64];
        let colors = [[0, 0, 0], [255, 255, 255]];
        let base = command(IVec3::ZERO);

        let performance = SparkPointCommand {
            gates: SparkDrawGates {
                performance_passed: false,
                ..base.gates
            },
            ..base
        };
        assert_eq!(
            resolve_spark_point(performance, &colors, frame(&a, &z)).unwrap(),
            PointResolution::Rejected(PointRejectReason::Performance),
        );

        let detail = SparkPointCommand {
            damage: 1,
            gates: SparkDrawGates {
                performance_passed: false,
                extra_animations_enabled: false,
                fog_passed: false,
            },
            ..base
        };
        assert_eq!(
            resolve_spark_point(detail, &colors, frame(&a, &z)).unwrap(),
            PointResolution::Rejected(PointRejectReason::ExtraAnimations),
        );
    }

    #[test]
    fn non_two_byte_destination_contract_keeps_only_one_output_byte() {
        let mut format = SYNTHETIC_565;
        format.destination_bytes_per_pixel = 4;
        let a = vec![127; 64 * 64];
        let z = vec![u16::MAX; 64 * 64];
        let colors = [[0, 0, 0], [255, 255, 255]];
        let mut frame = frame(&a, &z);
        frame.pixel_format = format;
        let PointResolution::Write(write) =
            resolve_spark_point(command(IVec3::ZERO), &colors, frame).unwrap()
        else {
            panic!("passing point must resolve to a write");
        };
        assert_eq!(write.byte_width, 1);
        assert_eq!(write.packed_value as u8, 0xff);
    }
}
```

**Step 2: Verify the point-kernel suite**

Run:

```powershell
cargo test -p vera20k tactical_compat -- --nocapture
```

Expected literal result: `test result: ok.` with all ten point-kernel tests passing. The large-coordinate fixtures must fail if `60/2` is folded into `30`, if `30/2` is folded into `15`, or if X/Y terms are combined before their separate signed `/2` operations. Treat the synthetic packing tuple as formula regression only.

### Task 7: Re-verify the load-bearing binary anchors against the implemented operation order

**Why:** The implementation is reverse-engineering-driven; a final read-only binary comparison prevents a cleanly compiling transcription from drifting away from the active executable.

**Files:**

- Read: `src/util/native_x87.rs`
- Read: `src/sim/particles/spark.rs`
- Read: `src/render/tactical_compat.rs`
- Read: `docs/research/PARTICLE_SPARK_COLLISION_AND_PIXEL_COMPOSITOR_GHIDRA_REPORT.md`

**Pattern:** Project verification discipline: decompile plus disassembly and callsite context; local labels are navigation hints, not authority.

**Step 1: Re-open the movement and point roots**

Using the live `gamemd.exe` program, decompile and disassemble:

- `0x0062C6E0` — behavior-3 movement/collision/color root.
- `0x0062CEC0` — behavior-3/4 point-draw root.
- `0x007C5F00` — `Math__ftol` conversion body.
- `0x005AF4D0` — 3x3 matrix-vector helper.
- `0x005AFC20` — orthonormal 3x4 inverse helper.

**Step 2: Compare the implementation line by line against these required observations**

- Persistent Z is stored after one gravity subtraction; probe Z is stored after the second.
- Coordinate conversion and candidate conversion each cross explicit `f32` memory boundaries before `Math__ftol`.
- The integer candidate remains available for cell/bridge work, but ground/contact/clamp comparisons consume the retained candidate `f32`; X/Y/Z cross final `Math__ftol` again when committed.
- Color order is `rng * reciprocal`, then `* 0.05`, then `+ ColorSpeed`, then `+ old accumulator`, followed by one `f64` store.
- The strict accumulator branch is `> 1.0`; the index predicate is signed `index < count - 2`.
- Matrix dot-product orders and final-only `f32` stores match Task 4 exactly; the surrounding call sequence is inverse multiply, scalar-by-one on that result, local-Z negate, forward multiply, final-Y negate.
- Collision coordinates/deletion precede color; lifetime follows color.
- Projection retains each wrapping `60`/`30` multiply followed by its own signed `/2` before term addition and final `/256`.
- Projection, clip, A load/reject, Z load/strict predicate, interpolation, A modulation, packing, and write preparation stay in native order; interpolation retains one `1.0-a` x87 value across RGB and adds each next term before its current term.
- A and Z are zero-extended words; Z base narrows/wraps before signed subtraction; no point path writes Z.

If any active instruction contradicts a task, stop execution, update the research report with the exact Ghidra call evidence, revise this plan, and run `/review-plan` again. Do not adapt code to a decompiler label alone.

**Step 3: Record the verification status honestly**

The acceptable post-task status is:

- **Static mechanism:** verified against the named active roots.
- **Rust regression behavior:** passing focused tests.
- **Whole-domain arithmetic parity:** unverified until native differential fixtures or exhaustive proof exist.
- **End-to-end pixel parity:** blocked until contract AT-16 and exact A/Z/frame integration are complete.

### Task 8: Format narrowly and run the serial foundation regression

**Why:** This validates all new interfaces together without racing other Cargo owners or formatting unrelated concurrent work.

**Files:**

- Format only files edited by Tasks 1-6.
- Inspect all diffs, especially shared particle/hash files.

**Pattern:** Repository Cargo/test coordination and narrow-format policy.

**Step 1: Confirm no other session owns Cargo**

Run:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
```

Do not start a build while another session owns these processes.

**Step 2: Format only edited Rust files**

Run:

```powershell
rustfmt --edition 2024 src/util/native_x87.rs src/util/mod.rs src/sim/particles/mod.rs src/sim/particles/fire.rs src/sim/particles/gas.rs src/sim/particles/smoke.rs src/sim/particles/spawn.rs src/sim/particles/system_ai.rs src/sim/particles/spark.rs src/sim/world/world_hash.rs src/render/mod.rs src/render/tactical_compat.rs
git diff --check
```

Inspect the diff and discard no unrelated user changes. If `rustfmt` changes unrelated regions of a shared file, restore those formatting-only hunks with a narrow patch while retaining functional edits.

**Step 3: Run focused tests serially**

Run one command at a time:

```powershell
cargo test -p vera20k native_x87 -- --nocapture
cargo test -p vera20k particle_hash_tests -- --nocapture
cargo test -p vera20k particles::spark::tests -- --nocapture
cargo test -p vera20k tactical_compat -- --nocapture
```

Expected for every command: a literal `test result: ok.` line.

**Step 4: Run the final compile check**

Run:

```powershell
cargo check -q -p vera20k
```

Expected: exit code 0 and no compiler diagnostics.

**Step 5: Audit activation guards and scope**

Run:

```powershell
rg -n -C 3 "ParticleSystemBehavesLike::Spark|ParticleSystemBehavesLike::Railgun" src/sim/particles/spawn.rs src/sim/particles/system_ai.rs src/app_instances/particles.rs
rg -n "tactical_compat|SparkPointCommand" src/app src/app_instances src/app_render
git diff -- src/sim/snapshot.rs src/sim/world/mod.rs src/rules/particle_type.rs src/map src/app_render/draw_passes.rs src/render/shroud_buffer.rs
```

Expected:

- Public Spark/Railgun spawn remains rejected.
- Production Spark/Railgun system dispatch remains a no-op.
- Behavior 3 remains excluded from the SHP instance path.
- There is no app/render extraction or draw-pass caller of the pure resolver.
- This plan contributes no diff to snapshots, rule parsing, map/bridge producers, draw passes, or shroud resources. Pre-existing unrelated diffs may be present and must remain untouched.

## Explicit Exclusions and Activation Conditions

This plan is complete when Tasks 1-8 pass while production behavior 3 remains unavailable. The following changes require new evidence and a revised/next plan:

| Excluded change | Required condition before authorization |
|---|---|
| Replace `ParticleType.color_speed` / parse `.13` | Retail raw-value capture or verified native INI decimal conversion |
| Parse/wire global `Gravity` into live Spark | Exact live Spark construction and public dispatch plan, using merged INI authority |
| Map `SparkCollisionFacts` from terrain/occupancy | Exact invalid-cell, slope/ground, first-building, LaserFence/undeploy, and wall query adapter design |
| Choose structural bridge producer | Native bit `0x100` lifecycle through collapse and repair mapped to Rust state |
| Consume gameplay RNG inside Spark system AI | Upstream burst owner plus forward/reverse tick integration plan proving one draw per active particle |
| Serialize particle systems or bump snapshot version | Exclusive snapshot ownership after current work lands; complete store serialization and versioned tests |
| Build commands in `app_instances` | Exact common performance latch owner and native-equivalent traversal/draw ordinal |
| Allocate/populate u16 A/Z buffers | Exact producer and frame-timing proof shared by all tactical consumers |
| Supply runtime multiplier/loss/shift defaults | Captured standard-session retail values and supported display-mode contract |
| Commit pixels in `draw_passes` | Exact overlap/order, single-A composition, later-light order, and retail final-pixel oracle |
| Remove Spark spawn/no-op/SHP guards | All upstream state/query/snapshot/render prerequisites close atomically |

## Sources & References

- **Approved design:** `docs/plans/2026-07-18-spark-native-float-and-point-compositor-design.md`
- **Implementation contract:** `docs/contracts/2026-07-18-spark-collision-pixel-compositor-implementation-contract.md`
- **Primary verified report:** `docs/research/PARTICLE_SPARK_COLLISION_AND_PIXEL_COMPOSITOR_GHIDRA_REPORT.md`
- **Timing/color report:** `docs/research/PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md`
- **RNG report:** `docs/research/PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md`
- **A-buffer context:** `docs/research/BSURFACE_CIRCBUF_ABUFFER_REPORT.md`
- **Z-buffer context:** `docs/research/ZBUFFER_DEPTH_SYSTEM.md`
- **Shared A/Z consumer context:** `docs/research/building-selection-brackets/TACTICAL_ABUFFER_SHROUD_VALUES_FOR_BRACKET_LINES_GHIDRA_REPORT.md`, `docs/research/building-selection-brackets/SURFACE_DRAWLINE_ABUFFER_ZTEST_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `docs/research/building-selection-brackets/PRIMARY_SURFACE_ZBUFFER_BRACKET_OWNERSHIP_GHIDRA_REPORT.md`
- **Separate upstream design:** `docs/plans/2026-07-18-spark-particle-system-and-lighting-design.md`
- **Active binary roots:** `0x0062C6E0` Spark AI; `0x0062CEC0` point draw; `0x0062CE40` particle AI dispatch; `0x0062E840` Spark system owner; `0x007C5F00` `Math__ftol`; `0x005AF4D0` matrix-vector; `0x005AFC20` inverse matrix.
- **x87 startup/control anchors:** `0x007CD80F`, `0x007CBDAF`, `0x007C8F46`, `0x007CEAAF`, `0x007CBF14`, `0x007CC01C`, `0x006BBFC1`, `0x006BBFC9`, `0x007C5EE4`, `0x007C5F00`.
- **Point helpers:** `0x006D2140` projection; `0x004114B0` circular word scanline; `0x007BCFB0` Z clear; `0x007BAEB0` DSurface point writer.
- **INI authority:** `ini/rulesmd.ini:756,26075-26105,26345-26353`; base fallback `ini/rules.ini:615,18874-18904,19055-19063`.
- **Current Rust patterns:** `src/sim/particles/mod.rs:33-160`; `src/sim/particles/spawn.rs:26-126`; `src/sim/particles/system_ai.rs:81-112`; `src/sim/rng.rs:213-247`; `src/sim/world/world_hash.rs:130-155`; `src/app_instances/particles.rs:41-121`; `src/app_render/draw_passes.rs:232-303`; `src/render/shroud_buffer.rs`.
- **Current git premise check:** HEAD `016ae152`; commit `d0f5ba87` concerns the distinct damage-particle timer and demonstrates shared snapshot/hash ownership, not ParticleClass behavior-3 implementation.

## Plan Review Verdict

- **Spec coverage:** complete for the design's implementation-ready arithmetic, state, injected simulation, command, and pure resolver surfaces; every public-activation prerequisite is explicitly excluded with a closing condition.
- **Architecture:** `sim/` imports only rules/util primitives and owns no render types; render consumes immutable raw state and injected frame data.
- **Determinism:** no hardware float enters simulation; raw state is hashed in stable order; production RNG/tick order is unchanged.
- **Risk control:** arithmetic and point stages have focused raw/boundary tests; no approximate buffer or guessed runtime constant is introduced.
- **Review corrections integrated:** dual integer/raw candidate handling, post-inverse scalar helper order, non-associative matrix coverage, native projection overflow staging, and retained cross-channel interpolation state are explicit in tasks and tests.
- **Certification:** static mechanism may be re-verified, but whole-domain arithmetic and final-pixel parity remain honestly unverified until native evidence closes them.
- **Execution prerequisite:** run `/review-plan docs/plans/2026-07-18-spark-native-float-and-point-compositor-plan.md` before implementation because this plan introduces a new arithmetic primitive and touches a shared `Particle` interface.
