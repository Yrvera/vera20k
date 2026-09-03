//! `VoxelAnimClass` — the flying VXL debris a vehicle or building throws when
//! it dies, and the meteors a scenario drops.
//!
//! gamemd-derived: constructor `0x007493B0`, AI `0x00749F30` (vtable `+0x5C`),
//! destructor `0x007499F0`. Each instance carries one `BounceClass` physics body
//! embedded at `+0xB0`; that half lives in [`crate::sim::bounce`].
//!
//! This is NOT `AnimClass` — that draws SHP sprites and is a separate hierarchy
//! sharing only `ObjectClass`. It is also not `sim::components::VoxelAnimation`,
//! which is a per-entity HVA frame cursor.
//!
//! The store IS wired: `world::substrate.voxel_anims` owns it, the LogicVector
//! owns AI order through `ObjectKind::VoxelAnim`, and it folds into the state
//! hash and the snapshot (`SNAPSHOT_VERSION` 120).
//!
//! RESIDUAL (GSI-05.14) — the pieces are simulated but not DRAWN. The unit
//! voxel renderer is keyed by entity type, house remap and facing, and a
//! `VoxelAnimClass` has none of those; its draw orientation is the quaternion
//! tumble, which is itself blocked on the unread `Math__SinFromTable` /
//! `Math__CosFromTable` contents (see [`crate::sim::bounce::BounceState`]), so
//! a renderer added now would have to invent a facing.
//! - Trigger: a death whose type authors `DebrisTypes=`.
//! - Player effect: the tyres a dying harvester throws are invisible; the SHP
//!   half of the same block does draw, so the death is not silent.
//! - Frequency: 36 stock sections author `DebrisTypes=` — the Allied and Soviet
//!   miners, the Battle Fortress, the Flak Track, the demo truck. The other 420
//!   sections that author `MaxDebris=` take the SHP arms and are visible.
//! - Downstream risk: none to the stream — the pieces already consume their
//!   draws, hash, and expire on schedule, so adding the draw later moves no
//!   state.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, map/, util/ and the rest of sim/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use glam::IVec3;
use serde::{Deserialize, Serialize};

use crate::rules::voxel_anim_type::{VoxelAnimType, VoxelAnimTypeId};
use crate::sim::bounce::{BounceOutcome, BounceState, BounceTerrain};
use crate::sim::intern::InternedId;
use crate::sim::rng::SimRng;
use crate::util::native_x87::{NativeF64Bits, NativeX87Error, X87Chop53};

/// One live `VoxelAnimClass` instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelAnimObject {
    pub stable_id: u64,
    pub type_id: VoxelAnimTypeId,
    /// `+0x140`, seeded from the type's `Duration=` (`+0x29C`) by the
    /// constructor. Ticks remaining. `VoxelAnimClass::AI @ 0x00749F30`
    /// decrements it while non-zero and runs the expiry arm once it is not
    /// positive.
    pub duration: i32,
    /// `+0x110`. Queued for removal; the AI deletes on its next visit.
    pub marked_for_deletion: bool,
    /// Owner house, or `None` — the constructor's fourth parameter, which the
    /// debris path passes as the dying object's house.
    pub owner_house: Option<InternedId>,
    /// The embedded `BounceClass` at `+0xB0`.
    pub bounce: BounceState,
    /// LogicClass membership, reconstructed from the serialized vector.
    #[serde(skip)]
    pub in_logic_vector: bool,
}

impl VoxelAnimObject {
    /// The world coordinate the draw and the damage arms read, in leptons.
    ///
    /// `VoxelAnimClass::AI` refreshes `ObjectClass`'s own coordinate from the
    /// physics body every tick via `CoordStruct::FromDoubles`, so the body is
    /// the authority and this is the conversion.
    pub fn world_coord(&self) -> IVec3 {
        self.bounce.position_leptons()
    }
}

/// Deterministic store, keyed by the shared object id.
///
/// `BTreeMap` for the same reason `EntityStore` uses one: the tick walk and the
/// state hash must see a fixed order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VoxelAnimStore(BTreeMap<u64, VoxelAnimObject>);

impl VoxelAnimStore {
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &VoxelAnimObject)> + '_ {
        self.0.iter()
    }

    pub fn get(&self, id: u64) -> Option<&VoxelAnimObject> {
        self.0.get(&id)
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut VoxelAnimObject> + '_ {
        self.0.values_mut()
    }

    pub(crate) fn get_mut(&mut self, id: u64) -> Option<&mut VoxelAnimObject> {
        self.0.get_mut(&id)
    }

    pub fn contains_key(&self, id: u64) -> bool {
        self.0.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Insert an object whose identity the shared allocator already assigned.
    pub(crate) fn insert(&mut self, object: VoxelAnimObject) -> u64 {
        let id = object.stable_id;
        debug_assert_ne!(id, 0, "voxel anim requires an assigned stable id");
        self.0.insert(id, object);
        id
    }

    pub(crate) fn remove(&mut self, id: u64) -> Option<VoxelAnimObject> {
        self.0.remove(&id)
    }

    /// Ids in store order, for a walk that mutates the store as it goes.
    pub(crate) fn ids(&self) -> Vec<u64> {
        self.0.keys().copied().collect()
    }
}

/// The Z the constructor launches from: `GetCoords().Z + 10` (`0x0074950C`,
/// `ADD EAX, 0xa`).
const LAUNCH_Z_OFFSET_LEPTONS: i32 = 10;

/// Placeholder identity for a piece built inside the combat transaction.
///
/// Native's `AbstractClass::AssignUniqueID` runs inside
/// `VoxelAnimClass::Constructor`, interleaved with the death's other object
/// allocations. Combat here does not hold the shared allocator — it borrows the
/// entity store out of the world — so the id is stamped when the world admits
/// the piece, preserving spawn ORDER but not the interleaving with any other
/// allocation the same tick makes between the two points. The stores are keyed
/// by that id, so order is what the hash and the AI walk read.
const UNASSIGNED_STABLE_ID: u64 = 0;

/// The `1.0` at `0x007E1718` that both inclusive-range divisors add before the
/// `ftol` (`0x007495A8`, `0x0074960B`). It makes `MaxZVel`/`MaxAngularVelocity`
/// reachable, and it is what collapses `[TIRE]`'s 12deg..24deg spin band to a
/// divisor of 1 — the band is stored in RADIANS, so its width is 0.21.
const INCLUSIVE_RANGE_BIAS: NativeF64Bits = NativeF64Bits::from_bits(0x3ff0_0000_0000_0000);

/// The gravity `VoxelAnimClass::Constructor` hands `BounceClass::Init` as two
/// literal pushes: `0x60000000` / `0x3ff66666` (`0x00749651`, `0x0074964C`) —
/// the double `1.4`. It is NOT read from the type.
const DEBRIS_GRAVITY: NativeF64Bits = NativeF64Bits::from_bits(0x3ff6_6666_6000_0000);

/// One raw `Random__Next()` draw taken through native's `CDQ/XOR/SUB` absolute
/// value and then `CDQ/IDIV`.
///
/// Mirrors [`SimRng::next_raw_abs_modulo`] exactly; it is spelled out here
/// because the constructor takes all four draws BEFORE any divisor exists, so
/// the draw and its remainder cannot be one call.
///
/// VERA-internal, gamemd equivalent UNCHECKED: a zero divisor returns 0 where
/// native's `IDIV` raises a divide error. It needs `MaxXYVel` under 0.5, or a
/// `MaxZVel` below `MinZVel - 1`; every stock `[VoxelAnims]` section authors
/// `MaxXYVel` at 10 or above, so this is unreachable in stock.
fn raw_abs_modulo(draw: u32, divisor: i32) -> i32 {
    if divisor == 0 {
        return 0;
    }
    ((draw as i32) % divisor).abs()
}

/// `VoxelAnimClass::Constructor @ 0x007493B0`, the ordinary (non-`IsMeteor`)
/// arm at `0x007494F0`..`0x00749663` — the one a death's `DebrisTypes=` takes.
///
/// Seven draws, in this exact order, all on the shared scenario stream
/// (`[0x00A8B230] + 0x218`):
/// - four `Random__Next()` at `0x0074951D`, `0x00749531`, `0x00749544`,
///   `0x00749559`, then
/// - three `RandomRanged(-0xFFFF, 0xFFFF)` inside [`BounceState::init`].
///
/// The four raw draws are taken up front and only then divided, so the divisors
/// are computed between the draws and the remainders. Reading the decompile's
/// variable order instead would pair the wrong draw with the wrong axis; the
/// pairing below is from the `FILD`/`FSUB`/`FSTP` ladder at
/// `0x0074958D`..`0x007495FA`:
/// - draw 1 -> the spin angle: `|d| % ftol(MaxAngularVelocity - MinAngularVelocity + 1.0) + MinAngularVelocity`
/// - draw 2 -> `Velocity.Z`: `|d| % ftol(MaxZVel - MinZVel + 1.0) + MinZVel`
/// - draw 3 -> `Velocity.Y`: `|d| % ftol(MaxXYVel + MaxXYVel) - MaxXYVel`
/// - draw 4 -> `Velocity.X`, same form as Y and the SAME divisor, which the
///   function saves in `[ESP+0x14]` rather than recomputing.
///
/// The velocity triple is written to the stack as `[X, Y, Z]` at `[ESP+0x1C]`,
/// `[ESP+0x20]`, `[ESP+0x24]` and that address is what `Init` receives, so the
/// Y component is the one from the THIRD draw even though its store comes last.
pub fn spawn_debris_piece(
    stable_id: u64,
    type_id: VoxelAnimTypeId,
    voxel_anim_type: &VoxelAnimType,
    owner_house: Option<InternedId>,
    origin: IVec3,
    rng: &mut SimRng,
) -> Result<VoxelAnimObject, NativeX87Error> {
    let start = IVec3::new(
        origin.x,
        origin.y,
        origin.z.saturating_add(LAUNCH_Z_OFFSET_LEPTONS),
    );

    let angular_draw = rng.next_u32();
    let z_draw = rng.next_u32();
    let y_draw = rng.next_u32();
    let x_draw = rng.next_u32();

    let max_xy = X87Chop53::load_f64(voxel_anim_type.max_xy_vel)?;
    let xy_divisor = X87Chop53::ftol_i64(X87Chop53::add(max_xy, max_xy))? as i32;
    let bias = X87Chop53::load_f64(INCLUSIVE_RANGE_BIAS)?;

    let velocity_y = X87Chop53::store_f32(X87Chop53::sub(
        X87Chop53::load_i32(raw_abs_modulo(y_draw, xy_divisor)),
        X87Chop53::load_f64(voxel_anim_type.max_xy_vel)?,
    ))?;
    let z_divisor = X87Chop53::ftol_i64(X87Chop53::add(
        X87Chop53::sub(
            X87Chop53::load_f64(voxel_anim_type.max_z_vel)?,
            X87Chop53::load_f64(voxel_anim_type.min_z_vel)?,
        ),
        bias,
    ))? as i32;
    let velocity_z = X87Chop53::store_f32(X87Chop53::add(
        X87Chop53::load_i32(raw_abs_modulo(z_draw, z_divisor)),
        X87Chop53::load_f64(voxel_anim_type.min_z_vel)?,
    ))?;
    let velocity_x = X87Chop53::store_f32(X87Chop53::sub(
        X87Chop53::load_i32(raw_abs_modulo(x_draw, xy_divisor)),
        X87Chop53::load_f64(voxel_anim_type.max_xy_vel)?,
    ))?;
    let angular_divisor = X87Chop53::ftol_i64(X87Chop53::add(
        X87Chop53::sub(
            X87Chop53::load_f64(voxel_anim_type.max_angular_velocity)?,
            X87Chop53::load_f64(voxel_anim_type.min_angular_velocity)?,
        ),
        bias,
    ))? as i32;
    let rotation_angle_per_tick = X87Chop53::store_f64(X87Chop53::add(
        X87Chop53::load_i32(raw_abs_modulo(angular_draw, angular_divisor)),
        X87Chop53::load_f64(voxel_anim_type.min_angular_velocity)?,
    ))?;

    let bounce = BounceState::init(
        start,
        voxel_anim_type.elasticity,
        DEBRIS_GRAVITY,
        NativeF64Bits::POSITIVE_ZERO,
        [velocity_x, velocity_y, velocity_z],
        rotation_angle_per_tick,
        rng,
    )?;

    Ok(VoxelAnimObject {
        stable_id,
        type_id,
        duration: voxel_anim_type.duration,
        marked_for_deletion: false,
        owner_house,
        bounce,
        in_logic_vector: false,
    })
}

/// What one `VoxelAnimClass::AI` visit asks the world to do after it returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelAnimAiOutcome {
    /// Still flying. The object stays in the store and in the LogicVector.
    Alive,
    /// The AI reached `vtable+0xF8` (`Delete`). The world retires the object.
    Delete,
}

/// One `VoxelAnimClass::AI @ 0x00749F30` visit for the debris arm.
///
/// The verified order:
/// 1. `StopSound` (`+0x2DC`) looping-sound maintenance.
/// 2. `marked_for_deletion` (`+0x110`) -> `Delete`, return.
/// 3. `if (duration != 0) duration -= 1` — the decrement is unconditional on
///    the sign, so a negative duration walks further negative.
/// 4. `duration > 0`: the `TrailerAnim=` arm on even frames, then
///    `BounceClass::Update`, then the `IsMeteor` extra gravity, then the
///    contact arms; a `Bounced` on WATER sets `duration = 0` instead of playing
///    the `BounceAnim=`, and a `Stopped` sets `duration = 0` outright.
/// 5. Otherwise the expiry arm, ending in `Delete`.
///
/// RESIDUAL (GSI-05.14) — three arms of the native AI are not built here:
/// - The `BounceAnim=`/`ExpireAnim=`/`TrailerAnim=` `AnimClass` spawns and the
///   expiry `Apply_area_damage` (`+0x2F0`/`+0x2F4`/`+0x2F8`). Every stock
///   `DebrisTypes=` line names `[TIRE]`, which authors none of the four keys,
///   so the death-debris path never reaches any of them in stock; `[PIECE]` and
///   `[GASTANK]` carry `Damage=`/`ExpireAnim=` but no stock section throws
///   them. Trigger: a mod pointing `DebrisTypes=` at a damaging type. Player
///   effect: that debris would leave no puff and do no splash damage.
///   Frequency: zero in stock skirmish. Downstream risk: the damage arm would
///   need the combat AoE transaction, so wiring it belongs with that owner.
/// - The `IsMeteor` arms — the extra per-tick gravity add at `+0xDC`, the
///   `Spawns=`/`SpawnCount=` child burst (two `RandomRanged(0, SpawnCount)`
///   draws, summed), and the `IsTiberium` ore-laying ring. `IsMeteor` types
///   reach the field through scenario meteor storms, never through a death.
///   Frequency: zero on this path.
/// - The water splash pair at expiry (`Rules+0x94`, `Rules+0xBC4`). Trigger:
///   debris expiring over water while below the deck. Player effect: no splash
///   ring. Frequency: only when a piece scatters off a shoreline.
pub fn voxel_anim_ai(
    object: &mut VoxelAnimObject,
    terrain: &dyn BounceTerrain,
) -> Result<VoxelAnimAiOutcome, NativeX87Error> {
    if object.marked_for_deletion {
        return Ok(VoxelAnimAiOutcome::Delete);
    }
    if object.duration != 0 {
        object.duration -= 1;
    }
    if object.duration <= 0 {
        return Ok(VoxelAnimAiOutcome::Delete);
    }

    match object.bounce.update(terrain)? {
        BounceOutcome::Falling => {}
        BounceOutcome::Bounced => {
            // Native distinguishes a water landing here and kills the piece
            // instead of playing its `BounceAnim=`; the anim itself is the
            // residual above, the `duration = 0` is not.
            if terrain.is_water(object.world_coord()) {
                object.duration = 0;
            }
        }
        BounceOutcome::Stopped => object.duration = 0,
    }
    Ok(VoxelAnimAiOutcome::Alive)
}

/// Which list the SHP half of the death block draws its anim from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShpDebrisSource {
    /// `TechnoType+0x5C4` — the section's own `DebrisAnims=`.
    TypeDebrisAnims,
    /// `RulesClass+0x140` / `+0x14C` — `[General] MetallicDebris=`.
    RulesMetallicDebris,
}

/// One `VoxelAnimClass` the death block asks the world to construct.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelDebrisSpawn {
    pub type_id: VoxelAnimTypeId,
    /// Everything the constructor and `BounceClass::Init` already decided; the
    /// world only assigns the shared object id.
    pub object: VoxelAnimObject,
}

/// One SHP debris anim the death block asks the world to construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShpDebrisSpawn {
    /// Index into the source list the row names.
    pub index: usize,
    pub source: ShpDebrisSource,
}

/// Everything one death throws, in native emission order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeathDebris {
    pub voxels: Vec<VoxelDebrisSpawn>,
    pub anims: Vec<ShpDebrisSpawn>,
}

impl DeathDebris {
    pub fn is_empty(&self) -> bool {
        self.voxels.is_empty() && self.anims.is_empty()
    }
}

/// The inputs the debris block reads off the dying object's `TechnoTypeClass`.
pub struct DebrisTypeData<'a> {
    /// `+0x5BC`.
    pub max_debris: i32,
    /// `+0x5C0`.
    pub min_debris: i32,
    /// `+0x314` vector, count at `+0x324`. Resolved entries in list order; an
    /// entry naming a `[VoxelAnims]` section that does not exist is `None`.
    /// Native resolves the names once at rules-load time and stores pointers,
    /// so an unresolvable name would be a null slot there too.
    pub debris_types: &'a [Option<(VoxelAnimTypeId, &'a VoxelAnimType)>],
    /// `+0x330` vector, positionally paired with `debris_types`.
    pub debris_maximums: &'a [i32],
    /// `+0x5C4` vector, count at `+0x5D4`.
    pub debris_anim_count: usize,
    /// `RulesClass+0x14C`.
    pub metallic_debris_count: usize,
}

/// VERA-internal, gamemd equivalent UNCHECKED: the ceiling on the drain loop.
///
/// Native's loop is `while (0 < budget)` and only the per-entry count drains
/// it, so a `DebrisMaximums=` list of all zeros gives a divisor of 1, a
/// remainder that is always 0, and an infinite loop — gamemd hangs. Every stock
/// `DebrisMaximums=` line is 4 or 6, so the divisor is never 1 and the loop is
/// geometric; this bound only stops a mod from freezing the tick. It sits far
/// above any reachable iteration count for a stock budget (`MaxDebris` tops out
/// at 15 in stock rules).
const DEBRIS_LOOP_ITERATION_CEILING: u32 = 4096;

/// The whole debris block of `TechnoClass::ReceiveDamage @ 0x00701900`, read
/// from the disassembly at `0x00702281`..`0x0070256C`.
///
/// The gates and the draws, in native order:
/// 1. `MapClass::Get_CellClass_At_Coord` on the death cell, then
///    `CMP [cell+0xEC], 2 / JZ 0x00702672` at `0x00702274` — a unit dying on
///    WATER throws no debris at all and takes no draw. That gate is the
///    caller's: it is itself guarded by `vtable+0x1C8 < 0xB` and the object's
///    `+0x8F`, which this function does not model.
/// 2. `TechnoType+0x5BC` (`MaxDebris`) must be positive (`0x00702291`). Below
///    that the block is skipped whole and the shared stream is untouched.
/// 3. `budget = RandomRanged(MinDebris, MaxDebris - 1)` at `0x007022C8`. The
///    `DEC EDI` at `0x007022AD` is why the top is one BELOW `MaxDebris`, so
///    `MaxDebris=1` can only ever yield the `MinDebris` end. 254 stock sections
///    author `MaxDebris=` with no `MinDebris=`, so their budget is
///    `RandomRanged(0, MaxDebris - 1)` and can come out zero.
/// 4. The voxel loop, entered only when `DebrisTypes.Count > 0` (`+0x324`,
///    `0x007022EA`) AND `budget > 0` (`0x007022F8`). Per iteration:
///    - one `Random__Next()` at `0x0070232B`, then
///      `count = |next| % (DebrisMaximums[index] + 1)` (`0x00702339`);
///    - `if (count >= budget) count = budget` (`0x0070233B`);
///    - `count` constructions of `DebrisTypes[index]`, each taking the seven
///      draws in [`spawn_debris_piece`];
///    - **`budget -= count`** — `SUB EBX,EAX` at `0x007023B5`, with `EBX`
///      reloaded from `[ESP+0x14]` at `0x007023A7` because the spawn loop's
///      `MOV EBX,EAX` at `0x0070235B` clobbers it. The budget is a POOL THAT
///      DRAINS, not a per-entry cap;
///    - `index += 1`, wrapping to 0 once it reaches `DebrisTypes.Count`
///      (`0x007023D3`), and the loop repeats while `budget > 0`
///      (`0x007023DF`). So a type with two debris entries cycles the list until
///      the pool is spent, and the total thrown is exactly the budget.
/// 5. The SHP arms, on whatever budget survives. Since step 4 exits only at
///    `budget <= 0`, a type that lists `DebrisTypes=` never reaches them:
///    - `DebrisAnims.Count > 0` (`+0x5D4`, `0x007023FC`) and `budget > 0`:
///      `budget` anims, each taking one `RandomRanged(0, count - 1)` at
///      `0x00702473`;
///    - otherwise `DebrisTypes.Count == 0` (`0x007024D2`) and `budget > 0`:
///      `budget` anims from `[General] MetallicDebris=`, each taking one
///      `RandomRanged(0, RulesClass+0x14C - 1)` at `0x0070253A`.
///
/// That last arm is the one players see most: 254 stock sections carry
/// `MaxDebris=` alone, 166 carry it with `DebrisAnims=`, and only 36 name
/// `DebrisTypes=`.
pub fn throw_death_debris(
    data: &DebrisTypeData<'_>,
    owner_house: Option<InternedId>,
    origin: IVec3,
    rng: &mut SimRng,
) -> Result<DeathDebris, NativeX87Error> {
    let mut out = DeathDebris::default();
    if data.max_debris <= 0 {
        return Ok(out);
    }
    // `Random__RandomRanged` sorts reversed bounds and consumes no draw when
    // they coincide; `next_range_i32_inclusive` models that helper directly.
    // The signed form is the right one because a `MinDebris=` above
    // `MaxDebris - 1` is authored by stock buildings (`MinDebris=4`,
    // `MaxDebris=6` gives 4..5, but `MinDebris=7`/`MaxDebris=15` gives 7..14)
    // and an unsigned read of a negative top would invent a huge span.
    let mut budget = rng.next_range_i32_inclusive(data.min_debris, data.max_debris - 1);

    if !data.debris_types.is_empty() && budget > 0 {
        let mut index = 0usize;
        let mut iterations = 0u32;
        loop {
            // VERA-internal, gamemd equivalent UNCHECKED: a `DebrisMaximums`
            // entry this list does not have is treated as 0, where native reads
            // past the vector's end. Stock authors the two lists at equal
            // length — 36 sections, each `DebrisTypes=TIRE` with one maximum —
            // so this is unreachable in stock.
            let maximum = data.debris_maximums.get(index).copied().unwrap_or(0);
            let divisor = maximum.saturating_add(1).max(1) as u32;
            let mut count = rng.next_raw_abs_modulo(divisor) as i32;
            if count >= budget {
                count = budget;
            }
            if count > 0
                && let Some((type_id, voxel_type)) = data.debris_types[index]
            {
                for _ in 0..count {
                    out.voxels.push(VoxelDebrisSpawn {
                        type_id,
                        object: spawn_debris_piece(
                            UNASSIGNED_STABLE_ID,
                            type_id,
                            voxel_type,
                            owner_house,
                            origin,
                            rng,
                        )?,
                    });
                }
            }
            budget -= count;
            index += 1;
            if index >= data.debris_types.len() {
                index = 0;
            }
            iterations += 1;
            if budget <= 0 || iterations >= DEBRIS_LOOP_ITERATION_CEILING {
                break;
            }
        }
    }

    if data.debris_anim_count > 0 {
        for _ in 0..budget.max(0) {
            let index = rng.next_range_i32_inclusive(0, data.debris_anim_count as i32 - 1) as usize;
            out.anims.push(ShpDebrisSpawn {
                index,
                source: ShpDebrisSource::TypeDebrisAnims,
            });
        }
    } else if data.debris_types.is_empty() && data.metallic_debris_count > 0 {
        for _ in 0..budget.max(0) {
            let index =
                rng.next_range_i32_inclusive(0, data.metallic_debris_count as i32 - 1) as usize;
            out.anims.push(ShpDebrisSpawn {
                index,
                source: ShpDebrisSource::RulesMetallicDebris,
            });
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::voxel_anim_type::VoxelAnimType;
    use crate::sim::rng::SimRng;

    /// A `[VoxelAnims]` section with the stock `[TIRE]` numbers, which is the
    /// only type any stock `DebrisTypes=` line names.
    fn tire() -> VoxelAnimType {
        let ini = crate::rules::ini_parser::IniFile::from_str(
            "[TIRE]\nElasticity=0.8\nMinAngularVelocity=12.0\nMaxAngularVelocity=24.0\n\
             MinZVel=28.0\nMaxZVel=32.0\nMaxXYVel=10.0\nDuration=150\n",
        );
        VoxelAnimType::from_ini_section("TIRE", ini.section("TIRE").unwrap())
    }

    fn data<'a>(
        max_debris: i32,
        min_debris: i32,
        debris_types: &'a [Option<(VoxelAnimTypeId, &'a VoxelAnimType)>],
        debris_maximums: &'a [i32],
        debris_anim_count: usize,
        metallic_debris_count: usize,
    ) -> DebrisTypeData<'a> {
        DebrisTypeData {
            max_debris,
            min_debris,
            debris_types,
            debris_maximums,
            debris_anim_count,
            metallic_debris_count,
        }
    }

    fn throw(input: &DebrisTypeData<'_>, rng: &mut SimRng) -> DeathDebris {
        throw_death_debris(input, None, IVec3::new(1280, 2560, 0), rng)
            .expect("the debris block stays inside the verified x87 domain")
    }

    #[test]
    fn gsi_05_14_no_debris_without_a_positive_maxdebris_and_no_draw_taken() {
        // `TEST ECX,ECX / JLE` at `0x00702291` sits BEFORE the budget draw, so
        // a type with no `MaxDebris=` costs the shared stream nothing.
        let voxel_type = tire();
        let types = [Some((VoxelAnimTypeId(0), &voxel_type))];
        let input = data(0, 0, &types, &[3], 6, 20);
        let mut rng = SimRng::new(11);
        assert!(throw(&input, &mut rng).is_empty());
        assert_eq!(rng.logical_view(), SimRng::new(11).logical_view());
    }

    #[test]
    fn gsi_05_14_budget_range_stops_one_below_maxdebris() {
        // `DEC EDI` at `0x007022AD` — the range is [MinDebris, MaxDebris - 1].
        // With MinDebris == MaxDebris - 1 the two bounds coincide and the
        // helper (like native's RandomRanged) consumes no draw, so the budget
        // is exactly that value and every later draw belongs to an entry.
        let input = data(5, 4, &[], &[], 0, 20);
        let mut rng = SimRng::new(5);
        let thrown = throw(&input, &mut rng);
        // No DebrisTypes and no DebrisAnims: the whole budget goes to the
        // `[General] MetallicDebris=` arm, one RandomRanged draw each.
        assert_eq!(thrown.anims.len(), 4);
        assert!(thrown.voxels.is_empty());
        assert!(
            thrown
                .anims
                .iter()
                .all(|row| row.source == ShpDebrisSource::RulesMetallicDebris)
        );
    }

    #[test]
    fn gsi_05_14_the_voxel_budget_drains_and_the_type_index_wraps() {
        // The load-bearing correction to the earlier reading of this block.
        // `SUB EBX,EAX` at `0x007023B5` subtracts the spent count from the
        // budget, and `CMP EDI,ECX / JL` at `0x007023D3` wraps the type index,
        // so the loop cycles the list until the pool is empty and the TOTAL
        // thrown is exactly the budget. The rejected reading — a per-entry cap
        // that never drains — would let two entries each reach the cap and
        // throw up to twice the budget.
        let voxel_type = tire();
        let types = [
            Some((VoxelAnimTypeId(0), &voxel_type)),
            Some((VoxelAnimTypeId(0), &voxel_type)),
        ];
        for seed in 0..64u64 {
            let input = data(7, 6, &types, &[5, 5], 0, 20);
            let mut rng = SimRng::new(seed);
            let thrown = throw(&input, &mut rng);
            assert_eq!(
                thrown.voxels.len(),
                6,
                "seed {seed}: the pool drains to exactly the budget"
            );
            assert!(
                thrown.anims.is_empty(),
                "a type that lists DebrisTypes= spends the whole budget and never reaches the SHP arms"
            );
        }
    }

    #[test]
    fn gsi_05_14_each_voxel_piece_costs_exactly_seven_draws() {
        // Four `Random__Next()` in `VoxelAnimClass::Constructor` plus three
        // `RandomRanged(-0xFFFF, 0xFFFF)` in `BounceClass::Init`. The count is
        // the lockstep contract: a miscount desyncs every later consumer in the
        // same tick.
        let voxel_type = tire();
        let types = [Some((VoxelAnimTypeId(0), &voxel_type))];
        let input = data(4, 3, &types, &[9], 0, 20);
        let mut rng = SimRng::new(2024);
        let thrown = throw(&input, &mut rng);
        assert_eq!(thrown.voxels.len(), 3);

        // The budget bounds coincide, so it costs no draw. The loop then takes
        // one count draw per iteration; with DebrisMaximums=9 and a budget of
        // 3 the first iteration can already spend the pool.
        let mut expected = SimRng::new(2024);
        let mut spawned = 0;
        while spawned < 3 {
            let count = (expected.next_raw_abs_modulo(10) as i32).min(3 - spawned);
            for _ in 0..count {
                for _ in 0..4 {
                    expected.next_u32();
                }
                for _ in 0..3 {
                    expected.next_range_u32_inclusive(0, (0xFFFF - -0xFFFF) as u32);
                }
            }
            spawned += count;
        }
        assert_eq!(rng.logical_view(), expected.logical_view());
    }

    #[test]
    fn gsi_05_14_debris_anims_win_over_the_rules_metallic_list() {
        // `0x007023FC` tests `DebrisAnims.Count` first; only a type with an
        // EMPTY list AND no `DebrisTypes=` falls through to `RulesClass+0x140`
        // at `0x0070252D`. 166 stock building sections take the first arm and
        // 254 vehicle sections the second.
        let input = data(9, 7, &[], &[], 6, 20);
        let mut rng = SimRng::new(31);
        let thrown = throw(&input, &mut rng);
        assert!(!thrown.anims.is_empty());
        assert!(
            thrown
                .anims
                .iter()
                .all(|row| row.source == ShpDebrisSource::TypeDebrisAnims && row.index < 6)
        );
    }

    #[test]
    fn gsi_05_14_a_type_with_debris_anims_but_debris_types_throws_no_shp() {
        // The voxel loop exits only at `budget <= 0`, so both SHP arms see a
        // spent pool. `[HARV]` is the stock case: `MaxDebris=6`,
        // `DebrisTypes=TIRE`, no `DebrisAnims=`.
        let voxel_type = tire();
        let types = [Some((VoxelAnimTypeId(0), &voxel_type))];
        let input = data(6, 5, &types, &[4], 6, 20);
        let mut rng = SimRng::new(77);
        let thrown = throw(&input, &mut rng);
        assert_eq!(thrown.voxels.len(), 5);
        assert!(thrown.anims.is_empty());
    }

    #[test]
    fn gsi_05_14_launch_velocity_pairs_each_draw_with_the_axis_native_pairs() {
        // Draw 3 feeds Y and draw 4 feeds X, both through
        // `|d| % ftol(2 * MaxXYVel) - MaxXYVel`; draw 2 feeds Z through
        // `|d| % ftol(MaxZVel - MinZVel + 1) + MinZVel`. Swapping X and Y (the
        // natural mistake, because native stores Y last) would mirror every
        // piece's scatter across the diagonal.
        let voxel_type = tire();
        let mut rng = SimRng::new(9);
        let mut expected = SimRng::new(9);
        let (d1, d2, d3, d4) = (
            expected.next_u32(),
            expected.next_u32(),
            expected.next_u32(),
            expected.next_u32(),
        );
        let piece = spawn_debris_piece(
            7,
            VoxelAnimTypeId(0),
            &voxel_type,
            None,
            IVec3::new(0, 0, 0),
            &mut rng,
        )
        .expect("in domain");
        let velocity = piece.bounce.velocity_f32();
        // MaxXYVel = 10.0 -> divisor 20, offset -10. MaxZVel - MinZVel + 1 = 5.
        assert_eq!(velocity[0], ((d4 as i32) % 20).abs() as f32 - 10.0);
        assert_eq!(velocity[1], ((d3 as i32) % 20).abs() as f32 - 10.0);
        assert_eq!(velocity[2], ((d2 as i32) % 5).abs() as f32 + 28.0);
        // MaxAngularVelocity - MinAngularVelocity is 24deg - 12deg = 0.209 rad,
        // so the +1.0 bias leaves a divisor of 1 and the spin is always the
        // MinAngularVelocity end whatever the draw. Reading the keys as degrees
        // would give a divisor of 13 and a spin band that actually varies.
        let _ = d1;
        assert_eq!(
            f64::from_bits(piece.bounce.spin_angle.bits()),
            f64::from_bits(voxel_type.min_angular_velocity.bits())
        );
        assert_eq!(piece.duration, 150);
        assert_eq!(piece.stable_id, 7);
    }

    #[test]
    fn gsi_05_14_the_launch_point_is_ten_leptons_above_the_wreck() {
        // `ADD EAX, 0xa` at `0x0074950C` — the constructor lifts the coordinate
        // it got from `ObjectClass::GetCoords` before handing it to
        // `BounceClass::Init`.
        let mut rng = SimRng::new(4);
        let piece = spawn_debris_piece(
            1,
            VoxelAnimTypeId(0),
            &tire(),
            None,
            IVec3::new(1280, 2560, 96),
            &mut rng,
        )
        .expect("in domain");
        assert_eq!(piece.world_coord(), IVec3::new(1280, 2560, 106));
    }

    /// Flat ground at Z = 0 with no bridges, buildings or ramps — the terrain
    /// the overwhelming majority of debris lands on.
    struct FlatGround;

    impl BounceTerrain for FlatGround {
        fn ground_height_leptons(&self, _coord: IVec3) -> i32 {
            0
        }
        fn is_bridge_cell(&self, _coord: IVec3) -> bool {
            false
        }
        fn cell_height_level(&self, _coord: IVec3) -> i32 {
            0
        }
        fn ramp(&self, _coord: IVec3) -> u8 {
            0
        }
        fn has_bounce_surface(&self, _coord: IVec3) -> bool {
            false
        }
        fn is_water(&self, _coord: IVec3) -> bool {
            false
        }
    }

    #[test]
    fn gsi_05_14_a_tire_flies_bounces_and_comes_to_rest_inside_its_duration() {
        // The end-to-end physics check: `[TIRE]` launches at 28..32 leptons per
        // tick upward with 1.4 gravity and `Elasticity=0.8`, so it must leave
        // the ground, come back, bounce at least once, and stop through the
        // `FUN_00439A10` magnitude test well inside its 150-tick `Duration=`.
        let mut rng = SimRng::new(1234);
        let mut piece = spawn_debris_piece(
            1,
            VoxelAnimTypeId(0),
            &tire(),
            None,
            IVec3::new(1280, 2560, 0),
            &mut rng,
        )
        .expect("in domain");
        let ground = FlatGround;
        let mut peak = piece.world_coord().z;
        let mut bounces = 0;
        let mut ticks_alive = 0;
        for _ in 0..150 {
            match voxel_anim_ai(&mut piece, &ground).expect("in domain") {
                VoxelAnimAiOutcome::Alive => {}
                VoxelAnimAiOutcome::Delete => break,
            }
            ticks_alive += 1;
            peak = peak.max(piece.world_coord().z);
            if piece.duration == 0 {
                bounces += 1;
                break;
            }
        }
        assert!(peak > 100, "the tire should climb, peaked at {peak}");
        assert!(
            bounces > 0,
            "the tire should settle rather than fly forever"
        );
        assert!(
            ticks_alive < 150,
            "it settled after {ticks_alive} ticks, inside its Duration"
        );
        // The AI zeroes the duration on a stop; the next visit deletes.
        assert_eq!(
            voxel_anim_ai(&mut piece, &ground).expect("in domain"),
            VoxelAnimAiOutcome::Delete
        );
    }

    #[test]
    fn gsi_05_14_a_marked_piece_deletes_before_the_duration_decrement() {
        // `+0x110` is tested at the top of the AI, above the decrement, so a
        // marked object never spends another tick of its duration.
        let mut rng = SimRng::new(3);
        let mut piece =
            spawn_debris_piece(1, VoxelAnimTypeId(0), &tire(), None, IVec3::ZERO, &mut rng)
                .expect("in domain");
        piece.marked_for_deletion = true;
        assert_eq!(
            voxel_anim_ai(&mut piece, &FlatGround).expect("in domain"),
            VoxelAnimAiOutcome::Delete
        );
        assert_eq!(piece.duration, 150, "the decrement sits below the gate");
    }

    #[test]
    fn gsi_05_14_debris_landing_in_water_dies_on_contact() {
        // `CMP [cell+0xEC], 2` inside the AI's bounce arm sets `duration = 0`
        // instead of playing the BounceAnim, so a piece that scatters off a
        // shoreline sinks rather than skipping.
        struct Water;
        impl BounceTerrain for Water {
            fn ground_height_leptons(&self, _coord: IVec3) -> i32 {
                0
            }
            fn is_bridge_cell(&self, _coord: IVec3) -> bool {
                false
            }
            fn cell_height_level(&self, _coord: IVec3) -> i32 {
                0
            }
            fn ramp(&self, _coord: IVec3) -> u8 {
                0
            }
            fn has_bounce_surface(&self, _coord: IVec3) -> bool {
                false
            }
            fn is_water(&self, _coord: IVec3) -> bool {
                true
            }
        }
        let mut rng = SimRng::new(88);
        let mut piece = spawn_debris_piece(
            1,
            VoxelAnimTypeId(0),
            &tire(),
            None,
            IVec3::new(1280, 2560, 0),
            &mut rng,
        )
        .expect("in domain");
        let water = Water;
        let mut outcome = VoxelAnimAiOutcome::Alive;
        for _ in 0..150 {
            outcome = voxel_anim_ai(&mut piece, &water).expect("in domain");
            if outcome == VoxelAnimAiOutcome::Delete || piece.duration == 0 {
                break;
            }
        }
        assert!(
            piece.duration == 0 || outcome == VoxelAnimAiOutcome::Delete,
            "a water landing must end the piece"
        );
    }
}
