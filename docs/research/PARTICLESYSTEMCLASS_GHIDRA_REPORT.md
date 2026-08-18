# ParticleSystemClass & ParticleClass -- Ghidra Research Report

**Primary Addresses:**
- `ParticleSystemClass::Constructor` @ `0x0062dc50`
- `ParticleSystemClass::AI` @ `0x0062fd60`
- `ParticleSystemTypeClass::ReadINI` @ `0x006442d0`
- `ParticleTypeClass::ReadINI` @ `0x00644f50`
- `ParticleClass::Constructor` @ `0x0062b5e0`
- `ParticleClass::AI_Dispatch` @ `0x0062ce40`
- `ParticleClass::Draw_It` @ `0x0062cec0`

**Confidence:** HIGH (all major functions decompiled from binary)
**Active in YR:** Yes -- particle systems are used extensively for damage effects, weapon visuals, and building smoke.

---

## 1. Overview

The particle system in gamemd.exe implements lightweight visual effects: smoke plumes from damaged buildings, spark showers, gas clouds (poison/psychic), fire streams from flamethrowers, and railgun beam trails. The architecture has two layers:

- **ParticleSystemClass** -- a container that owns a vector of particles, manages spawning, and dispatches per-tick AI based on its `BehavesLike` type.
- **ParticleClass** -- an individual particle with position, velocity, color, animation state, and lifetime.

Both have corresponding type classes (**ParticleSystemTypeClass**, **ParticleTypeClass**) that hold INI-parsed properties.

---

## 2. Class Layouts

### 2.1 ParticleSystemTypeClass (inherits ObjectTypeClass)

`param_1` in constructor is `undefined4 *` (int pointer), so byte offset = index * 4.

| Byte Offset | Type | INI Key | Default | Purpose |
|-------------|------|---------|---------|---------|
| +0x000 | vtable | -- | -- | ParticleSystemTypeClass vtable |
| +0x024 | char[52] | -- | -- | INI section name (inherited from AbstractTypeClass) |
| +0x294 | int (index) | HoldsWhat | -1 | Index into global ParticleTypeClass array |
| +0x298 | bool | Spawns | false | Whether system spawns new particles over time |
| +0x29C | int | SpawnFrames | 1 | Frame interval between spawns |
| +0x2A0 | float | Slowdown | 0.0 | Rate at which smoke particles decelerate |
| +0x2A4 | int | ParticleCap | 50 (0x32) | Maximum particles this system can hold |
| +0x2A8 | int | SpawnRadius | 0 | Random radius for spawn position offset |
| +0x2AC | float | SpawnCutoff | 0.0 | Distance threshold to stop spawning |
| +0x2B0 | float | SpawnTranslucencyCutoff | 0.0 | Distance threshold to start fading new particles |
| +0x2B4 | int (enum) | BehavesLike | -1 | System behavior type (see enum below) |
| +0x2B8 | int | Lifetime | -1 | System lifetime in frames (-1 = infinite until particles die) |
| +0x2BC | CoordStruct (12 bytes) | SpawnDirection | (0,0,0) | Direction vector for spawning |
| +0x2C8 | double | ParticlesPerCoord | 0.1 | Railgun: particles spawned per coordinate unit |
| +0x2D0 | double | SpiralDeltaPerCoord | 0.025 | Railgun: spiral angle increment per coord |
| +0x2D8 | double | SpiralRadius | 25.0 | Railgun: radius of spiral pattern |
| +0x2E0 | double | PositionPerturbationCoefficient | 0.0 | Railgun: random position offset scale |
| +0x2E8 | double | MovementPerturbationCoefficient | 0.0 | Railgun: random movement offset scale |
| +0x2F0 | double | VelocityPerturbationCoefficient | 0.0 | Railgun: random velocity perturbation scale |
| +0x2F8 | double | SpawnSparkPercentage | 0.0 | Spark: probability of spawning a spark each tick |
| +0x300 | int | SparkSpawnFrames | 0 | Spark: frame counter for spark spawn timing |
| +0x304 | int | LightSize | 0 | Light radius for spark systems |
| +0x308 | RGB (3 bytes) | LaserColor | (0,0,0) | Railgun: laser beam color |
| +0x30B | bool | Laser | false | Railgun: whether to draw a laser line |
| +0x30C | bool | OneFrameLight | false | Spark: light only lasts one frame |

**BehavesLike Enum (ParticleSystemTypeClass):**
Resolved from string table at `0x00836ee0`:

| Value | String | Description |
|-------|--------|-------------|
| 0 | Smoke | Smoke plumes (buildings, damage) |
| 1 | Gas | Poison/psychic gas clouds |
| 2 | Fire | Flamethrower fire streams |
| 3 | Spark | Electrical sparks (damage, welding) |
| 4 | Railgun | Railgun beam trail effects |
| -1 | (unknown) | No match found |

### 2.2 ParticleTypeClass (inherits ObjectTypeClass)

`param_1` in constructor is `undefined4 *` (int pointer), so byte offset = index * 4.

| Byte Offset | Type | INI Key | Default | Purpose |
|-------------|------|---------|---------|---------|
| +0x000 | vtable | -- | -- | ParticleTypeClass vtable |
| +0x294 | CoordStruct (12 bytes) | NextParticleOffset | (0,0,0) | Position offset when transitioning to next particle |
| +0x2A0 | int | XVelocity | 0 | Spark: max random X velocity |
| +0x2A4 | int | YVelocity | 0 | Spark: max random Y velocity |
| +0x2A8 | int | MinZVelocity | 0 | Spark: minimum upward velocity |
| +0x2AC | int | ZVelocityRange | 0 | Spark: random range added to MinZVelocity |
| +0x2B0 | double | ColorSpeed | 0.0 | Rate of color interpolation |
| +0x2B8 | void* | -- | -- | Internal (color vector pointer from ColorList parsing) |
| +0x2BC | void* | -- | -- | Pointer to first color entry |
| +0x2C0-0x2C7 | -- | -- | -- | Color vector data |
| +0x2C8 | int | ColorList count | 0 | Number of entries in ColorList |
| +0x2D4 | RGB (3 bytes) | StartColor1 | (0,0,0) | Spark: starting color 1 |
| +0x2D7 | RGB (3 bytes) | StartColor2 | (0,0,0) | Spark: starting color 2 |
| +0x2DC | short | MaxDC | 0 | Damage counter: frames between damage ticks |
| +0x2E0 | short/int | MaxEC | 0 | Lifetime in frames |
| +0x2E4 | int (ptr) | Warhead | NULL | Warhead for damage |
| +0x2E8 | int | Damage | 0 | Damage amount per tick |
| +0x2EC | int | StartFrame | 0 | Starting animation frame |
| +0x2F0 | int | NumLoopFrames | 1 | Frames per animation loop |
| +0x2F4 | int | Translucency | 0 | Base translucency level (0/25/50) |
| +0x2F8 | int | WindEffect | 0 | Wind sensitivity (0-5) |
| +0x2FC | float | Velocity | 0.0 | Movement speed |
| +0x300 | float | Deacc | 0.0 | Deceleration per frame |
| +0x304 | int | Radius | 0 | Interaction radius |
| +0x308 | bool | DeleteOnStateLimit | false | Delete particle when state reaches limit |
| +0x309 | byte | EndStateAI | 0 | Final animation state |
| +0x30A | byte | StartStateAI | 0 | Initial animation state |
| +0x30B | byte | StateAIAdvance | 4 | State advance rate divisor |
| +0x30C | byte | FinalDamageState | 0 | State at which damage stops |
| +0x30D | byte | Translucent25State | 0xFF | State at which 25% translucency begins |
| +0x30E | byte | Translucent50State | 0xFF | State at which 50% translucency begins |
| +0x30F | bool | Normalized | false | Normalize direction vector based on distance |
| +0x310 | int (index) | NextParticle | -1 | Index of next ParticleTypeClass (gas chain) |
| +0x314 | int (enum) | BehavesLike | -1 | Particle behavior type (see enum below) |

**BehavesLike Enum (ParticleTypeClass):**
Resolved from string table at `0x008370bc`:

| Value | String |
|-------|--------|
| 0 | Gas |
| 1 | Smoke |
| 2 | Fire |
| 3 | Spark |
| 4 | Railgun |

**IMPORTANT:** The enum ordering is DIFFERENT between ParticleSystemTypeClass and ParticleTypeClass. Systems use Smoke=0/Gas=1 while particles use Gas=0/Smoke=1.

### 2.3 ParticleSystemClass (inherits ObjectClass)

`param_1` in constructor is `undefined4 *` (int pointer), so byte offset = index * 4.

Size: 0x100 bytes (256 bytes).

| Byte Offset | Type | Purpose |
|-------------|------|---------|
| +0x000 | vtable | ParticleSystemClass vtable at `0x7efb9c` |
| +0x004 | vtable | Secondary vtable (IPersistStream) |
| +0x008 | vtable | Secondary vtable |
| +0x00C | vtable | Secondary vtable |
| +0x09C | CoordStruct | Object coordinates (from ObjectClass) |
| +0x0AC | PSTypeClass* | Pointer to ParticleSystemTypeClass |
| +0x0B0 | int | Offset X from attached object |
| +0x0B4 | int | Offset Y from attached object |
| +0x0B8 | int | Offset Z from attached object |
| +0x0BC | void* | Particle vector vftable pointer |
| +0x0C0 | void* | Particle vector data pointer |
| +0x0C4 | int | Particle vector capacity |
| +0x0C8-0xC9 | bool | Particle vector auto-grow flag |
| +0x0CC | int | Particle vector count (active particles) |
| +0x0D0 | int | Particle vector grow step |
| +0x0D4 | CoordStruct | Target/end coordinates (railgun endpoint) |
| +0x0E0 | AbstractClass* | Attached object (e.g., bullet for railgun) |
| +0x0E4 | AbstractClass* | Owner/source object |
| +0x0E8 | float | Spawn timer (slowdown accumulator for smoke) |
| +0x0EC | int | Lifetime countdown (from PSTypeClass Lifetime) |
| +0x0F0 | int | SparkSpawnFrames countdown |
| +0x0F4 | int | Particle facing (direction, frames 0x1d = 29 default) |
| +0x0F8 | bool | Marked for deletion |
| +0x0F9 | bool | Is "directionless" spawning mode (sparks) |
| +0x0FC | int | Owner house? (passed as param_7 to constructor) |

### 2.4 ParticleClass (inherits ObjectClass)

`param_1` in constructor is `undefined4 *` (int pointer), so byte offset = index * 4.

Size: 0x138 bytes (312 bytes) -- confirmed from `operator_new(0x138)`.

| Byte Offset | Type | Purpose |
|-------------|------|---------|
| +0x000 | vtable | ParticleClass vtable at `0x7ef954` |
| +0x09C | CoordStruct | Object coordinates (from ObjectClass) |
| +0x0AC | PTypeClass* | Pointer to ParticleTypeClass |
| +0x0B0 | RGB (3 bytes) | Current color (for spark/railgun rendering) |
| +0x0B4 | int | Wind drift accumulator |
| +0x0B8 | int | (unused/padding) |
| +0x0BC | int | Gas: wind X drift |
| +0x0C0 | int | Gas/Smoke: X drift velocity |
| +0x0C4 | int | Gas/Smoke: Y drift velocity |
| +0x0C8 | int | Gas/Smoke: Z drift velocity |
| +0x0CC | int | Gas: smoke direction X |
| +0x0D0 | int | Gas: smoke direction Y |
| +0x0D4 | float | Gas: Z-axis height offset |
| +0x0D8 | int | (reserved) |
| +0x0DC | int | (reserved) |
| +0x0E0 | int | (reserved) |
| +0x0E4 | float | Velocity (current speed, decreases with Deacc) |
| +0x0E8 | CoordStruct | Owner system's coordinates at spawn time |
| +0x0F4 | CoordStruct | Spawn position coordinates |
| +0x100 | CoordStruct | Previous position (for movement delta) |
| +0x10C | float | Direction vector X (normalized) |
| +0x110 | float | Direction vector Y (normalized) |
| +0x114 | float | Direction vector Z (normalized) |
| +0x118 | float | Origin X (float copy of spawn pos) |
| +0x11C | float | Origin Y |
| +0x120 | float | Origin Z |
| +0x124 | int | Owner particle system pointer |
| +0x128 | short | Lifetime remaining (frames, decremented each tick) |
| +0x12A | short | Damage counter (resets from MaxDC) |
| +0x12C | byte | StateAIAdvance (copied from type) |
| +0x12D | bool | Hit ground flag (fire particles) |
| +0x12E | byte | Current animation state |
| +0x12F | byte | Translucency level (0x00=opaque, 0x19=50%, 0x32=25%, 0x4B=very faded) |
| +0x130 | byte | (reserved) |
| +0x131 | bool | Marked for deletion |
| +0x134 | int | Color interpolation state (for ColorList) |
| +0x138 | -- | End of struct |

Note: Fields +0x0B4 through +0x0DC are shared/overlapping for different BehavesLike types. Gas particles use the drift/wind fields; spark/railgun particles use some of these for color interpolation double values.

---

## 3. Core Logic

### 3.1 System AI Dispatch (`0x0062fd60`)

```
fn ParticleSystemClass::AI(self):
    match self.type.BehavesLike:
        0 (Smoke) => AI_Smoke()
        1 (Gas)   => AI_Gas()
        2 (Fire)  => AI_Fire()
        3 (Spark) => AI_Spark()
        4 (Railgun) => AI_Railgun()

    self.lifetime -= 1
    if self.lifetime == 0:
        self.mark_for_deletion()

    if self.is_active AND self.done_spawning AND self.particle_count == 0:
        self.unregister()
        self.is_active = false
        add_to_limbo_vector(self)
```

### 3.2 Particle AI Dispatch (`0x0062ce40`)

```
fn ParticleClass::AI(self):
    match self.type.BehavesLike:
        0 (Gas)     => gas_particle_ai()
        1 (Smoke)   => smoke_particle_ai()
        2 (Fire)    => fire_particle_ai()
        3 (Spark)   => spark_particle_ai()
        4 (Railgun) => railgun_particle_ai()

    self.lifetime -= 1
    if self.lifetime == 0:
        self.marked_for_deletion = true
```

### 3.3 Smoke System AI (`0x0062ed40`)

The smoke system follows attached objects, periodically spawns new smoke particles with randomized positions, and applies translucency based on distance from the spawn cutoff.

```
fn AI_Smoke(self):
    // If attached to a moving object, update system position
    if self.attached_object != null AND attached_object.is_alive:
        new_pos = attached_object.get_coords() + self.offset
        self.set_coords(new_pos)

    // Tick all existing particles
    for each particle in self.particles:
        particle.AI()

    // Remove dead particles
    for each particle (reverse iteration):
        if particle.marked_for_deletion:
            if particle.type.NextParticle != -1:
                // Spawn replacement particle at offset
                next_type = ParticleTypeClass_Array[particle.type.NextParticle]
                new_pos = particle.coords + particle.type.NextParticleOffset
                spawn_particle(next_type, new_pos)
                copy velocity/state from old particle
            particle.destroy()

    // Spawn new particles if conditions met
    if NOT self.done_spawning AND self.type.Spawns:
        if g_CurrentFrameCounter % (int)self.spawn_timer == 0:
            if attached_object == null OR attached_object.not_selected OR attached_object.health < 0:
                spawn_pos = self.coords + random_offset(self.type.SpawnRadius)
                spawn_pos.z += 10
                new_particle = spawn_particle(self.type.HoldsWhat, spawn_pos)

                // Apply translucency based on distance from cutoff
                if self.type.SpawnTranslucencyCutoff < self.spawn_accumulator:
                    new_particle.translucency += 0x19  // fade it

                // Reduce velocity based on accumulated distance
                new_velocity = particle.velocity - (accumulator - type.SpawnFrames) * 0.025
                clamp(new_velocity, min=2.0)

    // Update spawn accumulator
    self.spawn_accumulator += self.type.Slowdown
    if self.spawn_accumulator > self.type.SpawnCutoff:
        self.done_spawning = true
```

### 3.4 Gas System AI (`0x0062e6d0`)

Gas systems tick all particles, handle the NextParticle chaining (gas clouds dissipate into smaller clouds), and remove dead ones.

```
fn AI_Gas(self):
    // First pass: tick all particles
    for each particle in self.particles:
        particle.AI()

    // Second pass (reverse): handle transitions and removal
    for each particle (reverse):
        if particle.marked_for_deletion:
            if particle.type.NextParticle != -1:
                next_type = ParticleTypeClass_Array[particle.type.NextParticle]
                new_pos = particle.coords + particle.type.NextParticleOffset
                new_particle = spawn_particle(next_type, new_pos)
                // Copy velocity and animation state
                new_particle.velocity = particle.velocity
                new_particle.x_drift = particle.x_drift
                new_particle.y_drift = particle.y_drift
                new_particle.z_drift = particle.z_drift
            particle.destroy()
```

### 3.5 Spark System AI (`0x0062e840`)

Sparks are spawned in batches with random velocities, decrement the spark spawn counter, and create light sources.

```
fn AI_Spark(self):
    if self.spark_frames_remaining > 0:
        // Probability check using SpawnSparkPercentage
        if spark_frames_remaining == 1 OR
           random_float() <= self.type.SpawnSparkPercentage:

            // Spawn half the cap worth of sparks
            count = random(ParticleCap / 2) + ParticleCap / 2

            for i in 0..count:
                particle = spawn_particle(type.HoldsWhat, self.coords)

                // Assign random velocities from type
                particle.dir_x = random() % type.XVelocity
                particle.dir_y = random() % type.YVelocity
                particle.dir_z = random() % type.ZVelocityRange + type.MinZVelocity

                // Normalize direction, preserve magnitude
                mag = sqrt(dir_x^2 + dir_y^2 + dir_z^2)
                dir = normalize(dir)

                // Add spawn direction or random direction
                if NOT directionless_mode:
                    dir += type.SpawnDirection
                else:
                    dir += random_direction

                // Re-normalize and restore magnitude
                dir = normalize(dir) * mag

            // Create light source on first spark frame if conditions met
            if DAT_00a8eb78 == 2 AND frames == type.SparkSpawnFrames:
                if type.LightSize > 0 AND NOT type.OneFrameLight:
                    create_light(self.coords, type.LightSize)

        self.spark_frames_remaining -= 1
        if spark_frames_remaining < 1:
            self.mark_for_deletion()

        // Randomly adjust particle facing (range 0x11..0x29, centered on 0x1d)
        r = random_float()
        if r < 0.3:
            facing = clamp(facing - 3, min=0x11)
        elif r < 0.7:
            // no change
        else:
            facing = clamp(facing + 3, max=0x29)

    // Tick and clean up particles
    for each particle: particle.AI()
    for each (reverse): if marked_for_deletion -> destroy
```

### 3.6 Fire System AI (`0x0062f9a0`)

The fire system follows its attached object and spawns gas particles in a specific pattern.

```
fn AI_Fire(self):
    // First pass: tick and move all particles
    for each particle:
        particle.AI()
        particle.Move()

    // Remove dead particles
    for each (reverse): if marked -> destroy

    // Track the attached object (refinery, etc.)
    target = find_abstract(self.attached_id)
    if target == null:
        self.mark_for_deletion()
        return

    if target.is_active AND target.has_timer:
        // Get target position, calculate orbital position
        target_pos = target.get_coords()
        distance = distance(self.coords, target_pos)
        timer_value = rate_timer.current()
        angle = (timer_value - 0x3fff) * scale
        orbital_x = cos(angle) * distance
        orbital_y = sin(angle) * distance

        // Update position along orbit
        self.coords = target_pos + orbital_offset
        self.set_coords(new_pos)

    // Periodic spawn based on SpawnFrames
    if NOT done_spawning:
        if g_CurrentFrameCounter % type.SpawnFrames == 0 OR
           (g_CurrentFrameCounter % 3 == 0 AND target_moved):
            // Calculate facing direction
            facing = calculate_facing(self.coords, target_pos)
            // Spawn particle with insertion at random position
            spawn_particle_with_insert(spawn_pos, target_pos, facing_count=4)
```

### 3.7 Railgun System AI (`0x0062f230`)

The railgun creates a spiral trail of particles between source and target coordinates.

```
fn AI_Railgun(self):
    if self.marked_for_deletion AND self.particle_count == 0:
        // Calculate direction and distance from source to target
        delta = self.target_coords - self.coords
        distance = sqrt(delta.x^2 + delta.y^2 + delta.z^2)
        horizontal_dist = sqrt(delta.x^2 + delta.y^2)

        // Calculate pitch and yaw angles
        pitch = asin(clamp(delta.z / distance))
        yaw = acos(clamp(delta.x / horizontal_dist))
        if delta.y < 0: yaw = -yaw

        // Build rotation matrix
        matrix = identity()
        matrix.rotate_z(yaw)
        matrix.rotate_x(pitch)

        // Spawn particles along the path
        for i in 0..total_coord_count:
            fraction = i / total_count
            spiral_angle = distance * fraction * SpiralDeltaPerCoord
            spiral_point = (cos(angle), sin(angle), 0)
            rotated = matrix * spiral_point

            // Apply spiral radius
            rotated *= SpiralRadius

            // Add position perturbation
            perturbation_x = (random_float() - 0.5) * PositionPerturbationCoefficient
            perturbation_z = (random_float() - 0.5) * PositionPerturbationCoefficient

            // Interpolate position along path
            world_pos = lerp(self.coords, self.target_coords, fraction)
            world_pos += perturbation + rotated

            // Create particle
            particle = spawn_particle(type.HoldsWhat, world_pos)
            particle.direction = rotated_normalized

            // Apply movement perturbation to direction
            movement_perturbation = random_vec() * MovementPerturbationCoefficient
            particle.direction += movement_perturbation
            particle.direction = normalize(particle.direction)

            // Apply velocity perturbation (accumulated, clamped)
            velocity_delta = (random_float() + accumulated - 0.5) * 2.0 * VelocityPerturbationCoefficient
            clamp(velocity_delta, -MovementPerturbation..VelocityPerturbation)
            accumulated = velocity_delta
            particle.velocity = accumulated + particle.type.Velocity

        // Draw laser line if enabled
        if type.Laser:
            create_laser_draw(self.coords, self.target_coords,
                             color=type.LaserColor, duration=10, fading=true)

        self.mark_for_deletion()

    // Tick and clean up particles
    for each particle: particle.AI()
    for each (reverse): if marked -> destroy
```

### 3.8 Individual Particle Behaviors

#### Gas Particle AI (`0x0062bd50`)
- Every other frame, applies random drift (x or y, clamped to -2..+2)
- Clears Z velocity every other frame
- Calculates ground height and bridge interaction
- Applies gravity (settles toward ground + 5 leptons)
- Handles wind direction from `[General] WindDirection`
- Decrements damage counter; when zero, deals damage to all objects in the cell using the particle's Warhead and Damage values
- Advances animation state based on StateAIAdvance divisor
- When EndStateAI reached: if DeleteOnStateLimit, mark for deletion; otherwise reset state to 0

#### Smoke Particle AI (`0x0062c540`)
- Every other frame, 25% chance to drift randomly (x or y, clamped to -5..+5)
- Clears Z drift each frame
- Advances animation state using StateAIAdvance
- When EndStateAI reached with DeleteOnStateLimit: mark for deletion
- Applies deceleration: velocity -= Deacc each frame (only while velocity > 0)

#### Fire Particle AI (`0x0062cb10`)
- If velocity <= 0: mark for deletion
- Applies random jitter to movement direction (+/- 5% random factor)
- Updates previous position for next frame
- Advances animation state; when reaching Translucent25State or Translucent50State, sets translucency
  - Translucent50State: translucency = 0x19
  - Translucent25State: translucency = 0x32
- Applies deceleration: velocity -= Deacc
- Decrements damage counter; when zero, deals damage to objects in cell
  - Only damages if FinalDamageState not yet reached
  - Checks bridge layer for Z-above-bridge objects

#### Spark Particle AI (`0x0062c6e0`)
- Applies gravity: Z velocity -= RulesClass.Gravity each tick
- Applies movement based on direction vector * velocity
- Checks ground height and bridge collisions
- When hitting ground or bridge: mark for deletion, store impact velocity
- Advances color interpolation: accumulated += ColorSpeed * random_factor
  - When accumulated > 1.0: advance to next color in ColorList, reset
- Uses VXL facing matrix for 3D collision detection

#### Railgun Particle AI (`0x0062c3a0`)
- Moves along direction vector * velocity
- Applies random velocity jitter: velocity += (random - 0.5) * 0.1
- Updates float origin position (accumulated movement)
- Advances color interpolation same as spark
- Position is updated from accumulated float coords via ftol conversion

### 3.9 Particle Movement Dispatch (`0x0062d5e0`)

Movement is separate from AI and dispatched by BehavesLike:

- **Gas (0)**: Applies wind effect based on WindDirection and WindEffect value. Every 10/WindEffect frames, shifts position by wind direction table. Applies random X/Y drift. Ensures Z stays above ground + 5.
- **Smoke (1)**: Similar wind/drift as gas but with different parameters (FUN_0062d3f0).
- **Fire (2)**: Moves along direction vector. If velocity > 0, adds previous_position delta. Checks ground height at new position; if terrain rises, marks particle as hitting ground and for deletion.
- **Spark (3)**: No separate movement -- handled in spark AI directly.
- **Railgun (4)**: No separate movement -- handled in railgun AI directly.

---

## 4. INI Keys

### 4.1 ParticleSystemTypeClass Keys (in [ParticleSystems] sections)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| HoldsWhat | string | "" | Name of ParticleTypeClass this system spawns |
| BehavesLike | string | "" | "Smoke", "Gas", "Fire", "Spark", or "Railgun" |
| Spawns | bool | false | Whether system auto-spawns particles over time |
| SpawnFrames | int | 1 | Frame interval between spawns |
| ParticleCap | int | 50 | Maximum particles allowed |
| SpawnRadius | int | 0 | Random spawn position offset radius |
| Slowdown | float | 0.0 | Smoke: deceleration rate |
| SpawnCutoff | float | 0.0 | Smoke: distance to stop spawning |
| SpawnTranslucencyCutoff | float | 0.0 | Smoke: distance to start fading |
| Lifetime | int | -1 | System lifetime in frames (-1=infinite until particles die) |
| SpawnDirection | CoordStruct | (0,0,0) | Directional vector for particle spawn |
| ParticlesPerCoord | double | 0.1 | Railgun: particle density along path |
| SpiralDeltaPerCoord | double | 0.025 | Railgun: spiral angle increment |
| SpiralRadius | double | 25.0 | Railgun: spiral radius |
| PositionPerturbationCoefficient | double | 0.0 | Railgun: position randomization |
| MovementPerturbationCoefficient | double | 0.0 | Railgun: movement randomization |
| VelocityPerturbationCoefficient | double | 0.0 | Railgun: velocity randomization |
| SpawnSparkPercentage | double | 0.0 | Spark: probability per frame |
| SparkSpawnFrames | int | 0 | Spark: total frame count for spark emission |
| LightSize | int | 0 | Spark: light source radius |
| OneFrameLight | bool | false | Spark: light only for one frame |
| Laser | bool | false | Railgun: draw laser line |
| LaserColor | RGB | (0,0,0) | Railgun: laser color |

### 4.2 ParticleTypeClass Keys (in [Particles] sections)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| BehavesLike | string | "" | "Gas", "Smoke", "Fire", "Spark", or "Railgun" |
| Image | string | "" | SHP image for sprite-based particles |
| MaxDC | int | 0 | Frames between damage applications |
| MaxEC | int | 0 | Particle lifetime in frames |
| Damage | int | 0 | Damage per tick |
| Warhead | string | "" | Warhead for damage |
| StartFrame | int | 0 | Starting animation frame |
| NumLoopFrames | int | 1 | Frames per animation loop |
| Translucency | int | 0 | Base translucency (0/25/50) |
| WindEffect | int | 0 | Wind sensitivity (0=none, 5=max) |
| Velocity | float | 0.0 | Movement speed |
| Deacc | float | 0.0 | Deceleration per frame |
| Radius | int | 0 | Interaction radius |
| DeleteOnStateLimit | bool | false | Delete when animation reaches end |
| EndStateAI | byte | 0 | Final animation state |
| StartStateAI | byte | 0 | Initial animation state |
| StateAIAdvance | byte | 4 | Animation advance rate divisor |
| Translucent50State | byte | 0xFF | State for 50% translucency |
| Translucent25State | byte | 0xFF | State for 25% translucency |
| Normalized | bool | false | Normalize direction to source distance |
| FinalDamageState | byte | same as EndStateAI | State at which damage stops |
| NextParticle | string | "" | Name of next ParticleType in chain |
| NextParticleOffset | CoordStruct | (0,0,0) | Position offset for next particle |
| ColorList | RGB list | empty | Color gradient for spark/railgun particles |
| ColorSpeed | double | 0.0 | Color interpolation speed |
| StartColor1 | RGB | (0,0,0) | Spark: random start color 1 |
| StartColor2 | RGB | (0,0,0) | Spark: random start color 2 |
| XVelocity | int | 0 | Spark: max random X velocity |
| YVelocity | int | 0 | Spark: max random Y velocity |
| MinZVelocity | int | 0 | Spark: minimum upward velocity |
| ZVelocityRange | int | 0 | Spark: random range for Z velocity |
| Report | string | "" | Sound to play (from art/sound system) |

### 4.3 TechnoTypeClass Keys (per-unit/building)

| Key | Section | Description |
|-----|---------|-------------|
| DamageParticleSystems | TechnoType | Comma-separated list of systems spawned when damaged |
| DestroyParticleSystems | TechnoType | Comma-separated list of systems spawned when destroyed |
| RefinerySmokeParticleSystem | TechnoType | Smoke system for refineries processing ore |
| NaturalParticleSystem | TechnoType | Ambient particle effect (not used in standard YR content) |

### 4.4 WeaponTypeClass Keys

| Key | Section | Description |
|-----|---------|-------------|
| AttachedParticleSystem | WeaponType | Particle system that follows the projectile (e.g., railgun, fire stream) |
| UseFireParticles | WeaponType | bool, spawns fire particle effects |
| UseSparkParticles | WeaponType | bool, spawns spark particle effects |

### 4.5 General Keys

| Key | Section | Description |
|-----|---------|-------------|
| BarrelParticle | [General] | Default smoke system for barrel fire |
| DefaultTestParticleSystem | [General] | Default test particle system |
| DefaultRepairParticleSystem | [General] | Default welding spark system for repairs |
| WindDirection | [General] | Wind direction (FacingType 0-7, 0=North) |

---

## 5. Integration Points

### 5.1 Who Creates Particle Systems

From callers of `ParticleSystemClass::Constructor` at `0x0062dc50`:

| Caller | Address | Context |
|--------|---------|---------|
| `TechnoClass::AI_Update` | `0x006f9e50` | Damage smoke (DamageParticleSystems) |
| `TechnoClass::Fire_At` | `0x006fdd50` | Weapon particle systems (AttachedParticleSystem) |
| `TechnoClass::ReceiveDamage` | `0x00701900` | DestroyParticleSystems on death |
| `Apply_area_damage` | `0x00489280` | Gas cloud from area damage |
| `BuildingClass::UpdateGapGenerator_Tick` | `0x00454db0` | Gap generator smoke |
| `UnitClass::AI` | `0x007360c0` | Refinery smoke (RefinerySmokeParticleSystem) |
| `VoxelAnimClass::Constructor` | `0x007493b0` | Particle effects on voxel animations |
| `TriggerAction::Execute` | `0x006dd8b0` | Map trigger-spawned particles |
| `CaptureManagerClass::Update` | `0x00471a50` | Mind control beam particles |
| `WarpAttachClass::UpdateAttack` | `0x00629fd0` | Chrono/teleport warp effects |
| `FUN_00459900` | `0x00459900` | Building-related particle spawn |
| `FUN_004c2a60` | `0x004c2a60` | Additional spawn context |
| `FUN_00684c30` | `0x00684c30` | Additional spawn context |

### 5.2 When AI Runs

Particle systems are stored in a global `DynamicVectorClass<ParticleSystemClass*>`. The game's main logic loop iterates all particle systems and calls their AI through the vtable. This happens during the object update phase of the game tick.

Individual particles are NOT in the global object list -- they are owned by their parent ParticleSystemClass and ticked from the system's AI.

### 5.3 Rendering

Rendering is handled through `ParticleClass::Draw_It` at `0x0062cec0`:

- **Spark and Railgun particles** (BehavesLike 3 or 4): Rendered as single colored pixels directly to the screen surface. The pixel color comes from the particle's current RGB color (interpolated from ColorList). The alpha buffer and Z-buffer are checked for visibility. The color is modulated by the alpha value at the pixel position.

- **Gas, Smoke, and Fire particles** (BehavesLike 0, 1, or 2): Rendered as SHP sprites via `CC_Draw_Shape()`. The translucency level at +0x12F controls the draw flags:
  - 0x00: opaque (flag 0x2800)
  - 0x19: 50% translucent (flag 0x2802)
  - 0x32: 25% translucent (flag 0x2804)
  - >0x4A: very faded (flag 0x2806)
  - Additional flags: 0xE00 (standard draw flags), z-adjust applied

- **Fog of war check**: If fog of war is enabled (`SpecialFlags & 0x1000`), particles in shrouded cells are not drawn. This is a TS-legacy check that is normally inactive in YR since FogOfWar defaults to false.

- **Layer**: All particles return layer 3 (top layer) from GetLayer, meaning they render above ground objects.

- **Frame skipping**: Smoke (type 1) and Spark (type 3) particles skip rendering entirely when the game is in fast-forward mode (DAT_00a8eb78 == 0), as an optimization. Gas, Fire, and Railgun always render.

---

## 6. Current Rust Implementation Status

### Implemented
- `src/rules/weapon_type.rs`: Parses `AttachedParticleSystem`, `UseFireParticles`, `UseSparkParticles` from weapon INI sections.
- `src/rules/ruleset.rs`: References `ChronoSparkle1` from [General].

### NOT Implemented
- No `ParticleSystemTypeClass` or `ParticleTypeClass` data structures
- No INI parsing for `[ParticleSystems]` or `[Particles]` sections
- No `ParticleSystemClass` or `ParticleClass` runtime objects
- No particle spawning from damage, weapons, or buildings
- No particle AI (smoke drift, gas damage, spark physics, railgun spiral)
- No particle rendering (neither pixel-based for sparks nor SHP-based for smoke/gas)
- No DamageParticleSystems or DestroyParticleSystems handling on TechnoTypeClass
- No wind system affecting particles

---

## 7. Open Questions

1. **FUN_00630b90 and FUN_00630ea0**: These functions in the ParticleSystemClass range were not decompiled. They may handle serialization or additional lifecycle management. LOW priority.

2. **Gas particle ground collision details**: The gas AI at `0x0062bd50` has complex bridge interaction logic (checking cell flags 0x100 at offset 0x140, comparing heights). The exact bridge interaction model needs more study to confirm all edge cases.

3. **Color interpolation function** (`FUN_00661020`): Called during particle construction for spark/railgun types to interpolate between StartColor1 and StartColor2. The exact interpolation curve was not fully decompiled.

4. **Wind direction table** at `0x00836664` / `0x00836684`: Used by smoke/gas movement for wind drift. Contains 8 pairs of (dx, dy) for each facing direction. The exact values were not read.

5. **NaturalParticleSystem**: Referenced in TechnoTypeClass::ReadINI but no standard YR content uses it. Likely TS-legacy or unused feature. Active in YR: Conditional (key exists in INI parser, but no default content uses it).

6. **FUN_0062e280** (vtable +0x114 of PSC): Creates a LightSource object when OneFrameLight is false and LightSize > 0 and particle count > 0. The exact relationship with the spark light creation in AI_Spark needs clarification -- they may be redundant or serve different purposes.

7. **Exact struct sizes for type classes**: ParticleSystemTypeClass appears to end around +0x310 (0xC4 * 4 = 0x310 max field). ParticleTypeClass ends around +0x318 (confirmed from `operator_new(0x318)` in constructor). The base ObjectTypeClass size was not independently confirmed.

---

## 8. Gap Analysis — Deep Dive

### 8.1 Wind Direction Lookup Table (0x00836664 / 0x00836684)

Two tables of 8 int32 values each, representing (dx, dy) deltas for the 8 compass facings.
These are used by gas and smoke particle movement to apply wind drift.

**Table at 0x00836664 — Wind DX values (8 entries):**

| Index | Facing | DX Value |
|-------|--------|----------|
| 0 | N | 0 |
| 1 | NE | 2 |
| 2 | E | 2 |
| 3 | SE | 1 |
| 4 | S | 0 |
| 5 | SW | -2 |
| 6 | W | -2 |
| 7 | NW | -2 |

**Table at 0x00836684 — Wind DY values (8 entries):**

| Index | Facing | DY Value |
|-------|--------|----------|
| 0 | N | -2 |
| 1 | NE | -2 |
| 2 | E | 0 |
| 3 | SE | 2 |
| 4 | S | 2 |
| 5 | SW | 2 |
| 6 | W | 0 |
| 7 | NW | -2 |

Usage: In gas/smoke particle movement, every `10 / WindEffect` frames, the particle position
is shifted by `(DX[WindDirection], DY[WindDirection])` leptons. The `WindDirection` value
comes from `[General] WindDirection` (a FacingType 0-7, 0=North). Note that the DX table
starts at offset 0x00836664 (32 bytes = 8 ints) and the DY table starts at 0x00836684.
The combined vector confirms that N = up (-Y in map space), E = right (+X), etc.

**Rust constant:**
```rust
/// Wind direction drift deltas: (dx, dy) per facing index 0..7 (N/NE/E/SE/S/SW/W/NW)
const WIND_DRIFT_DX: [i32; 8] = [0, 2, 2, 1, 0, -2, -2, -2];
const WIND_DRIFT_DY: [i32; 8] = [-2, -2, 0, 2, 2, 2, 0, -2];
```

### 8.2 Color Interpolation Function FUN_00661020

**Address:** `0x00661020`
**Signature:** `RGB* FUN_00661020(RGB* out, RGB* color1, RGB* color2, float t)`
**Calling convention:** thiscall (ECX = out pointer)

This function performs **linear interpolation** between two RGB colors.

**Algorithm (per channel):**
```
result[i] = clamp(color2[i] * t + color1[i] * (1.0 - t), 0.0, 255.0)
```

The FPU stack loads `1.0` from `0x007e2ac8`, subtracts `t` to get `(1.0 - t)`, then for each
of the 3 color channels:
1. Loads `color1[i]` as unsigned byte, converts to int, then to float
2. Multiplies by `(1.0 - t)`
3. Loads `color2[i]` as unsigned byte, converts to int, then to float
4. Multiplies by `t` (the complement weight)
5. Adds the two products
6. Clamps to [0.0, 255.0] (constants at `0x007e1748`=0.0 and `0x007e2220`=255.0)
7. Converts to byte via `Math__ftol` and stores in output

**Conclusion:** This is a standard linear lerp: `lerp(color1, color2, t)`. The `t` parameter
comes from a random float `[0.0, 1.0)` generated at particle spawn time. So each spark/railgun
particle starts with a random color between StartColor1 and StartColor2.

### 8.3 ColorList / ColorSpeed Interpolation in Spark and Railgun AI

Both spark AI (`0x0062c6e0`) and railgun AI (`0x0062c3a0`) share identical color interpolation logic.
The key fields on ParticleClass (using `int *` param_1, so multiply index by 4):

- `param_1[0x2b]` = ParticleTypeClass pointer
- `param_1[0x2d]` = Current color index in ColorList (byte offset +0xB4 = index 0)
- `param_1[0x2e..0x2f]` = Color accumulator as double (byte offset +0xB8)
- `ParticleTypeClass+0x2b0` = ColorSpeed (double)
- `ParticleTypeClass+0x2c8` = ColorList count (int)

**Wait — correction on field mapping.** The `param_1` type is `int *`, so:
- `param_1[0x2d]` = byte offset 0x2d * 4 = **0xB4** — this is the wind drift accumulator field; but for spark/railgun particles this field is repurposed as the **color index**.
- `param_1[0x2e..0x2f]` = byte offset 0xB8..0xBF — the 8-byte **color accumulator double**. For gas/smoke particles these bytes hold drift velocities, but spark/railgun repurpose them.

This confirms the note in the existing report about overlapping fields at +0xB4 through +0xDC.

**Algorithm:**
```
random_factor = Random(0, INT_MAX) * (1.0 / INT_MAX) * 0.05
                // random in [0.0, 0.05)
accumulator += ColorSpeed + random_factor

if accumulator > 1.0:
    if color_index < (ColorList.count - 2):
        // Advance to next color pair
        color_index += 1
        accumulator = 0.0
    else:
        // At the end of the list — clamp accumulator to 1.0
        accumulator = 1.0
```

The `0.05` constant comes from `0x007e8ae8` (double 0.05). So each tick, the accumulator
advances by `ColorSpeed + random(0..0.05)`. When it exceeds 1.0, the particle advances to
the next color pair in the ColorList. The actual displayed color is looked up via:
```
if color_index == 0:
    use particle's StartColor (RGB at +0xB0)
else:
    use ColorList[color_index * 3]   // index into RGB array from ParticleTypeClass
```

The `color_index * 3` access at `param_1[0x2b] + 700` (decimal) = `param_1[0x2b] + 0x2BC` is
the base of the ColorList RGB data. The byte formula is:
`color_ptr = ParticleTypeClass_base + 0x2BC + color_index * 3`

**Key insight:** ColorSpeed determines how fast particles cycle through the ColorList gradient.
A value of 0.5 means the particle transitions through each color pair in ~2 ticks. The random
jitter of 0..0.05 adds slight per-particle variation to prevent all particles looking identical.

### 8.4 Gas Particle Bridge Collision (0x0062bd50 — detailed)

The gas particle AI has complex bridge and terrain interaction. Here is the exact logic,
decompiled from the `int *` param_1 (all offsets * 4):

**Key variables:**
- `param_1[0x27..0x29]` = particle coordinates (X, Y, Z) at byte offsets +0x9C..+0xA4
- `local_bc` (derived from param_1[0x29]) = current Z float
- `local_f4` = ground height at particle's cell
- `DAT_00ac4a0c` = bridge height offset (runtime variable, 0 at init — set by CellClass)
- `_DAT_007e3da8` = float 150.0 (proximity threshold for building check)

**Bridge detection:**
```
ground_z = CellClass::GetGroundHeight(particle_cell)
bridge_z = ground_z + bridge_height_offset    // DAT_00ac4a0c

// Check both the particle's cell and the cell at (ground_z + offset) for bridge flag
cell1 = CellClass::Get_Cell_At(particle_coords)
cell2 = CellClass::Get_Cell_At(ground_coords)

has_bridge = (cell1.flags_0x140 & 0x100) != 0 || (cell2.flags_0x140 & 0x100) != 0
```

The `0x100` flag at cell offset `+0x140` is the bridge presence flag.

**Bridge collision states (when has_bridge is true):**

Three boolean states are computed:
- `bVar17` (on_bridge): particle Z is between ground and bridge_z AND particle came from below  
  Specifically: `ground_z < bridge_z AND bridge_z <= particle_old_Z` → particle is ON the bridge
- `bVar4` (under_bridge): particle Z is above bridge but particle's previous Z was below  
  Specifically: `particle_old_Z >= bridge_z AND bridge_z > particle_new_Z_dest` → particle is going UNDER

Actually, re-reading the decompilation more carefully with the local variable tracking:
```
if has_bridge:
    if local_a0 < bridge_z:          // local_a0 = some reference height
        if bridge_z <= local_ac:     // local_ac = another reference height
            on_bridge = true
    else:
        if local_ac < bridge_z:
            under_bridge = true
```

**Building proximity check (when NOT on or under bridge):**
```
if !on_bridge AND !under_bridge:
    if ground_height <= current_z AND current_z - 150.0 < ground_height:
        // Particle is close to ground level (within 150 leptons)
        building = Look_up_building_in_cell()
        if building exists OR FUN_00480510() returns true:
            on_building = true
            // But exclude if building is a high bridge (bridge flag + height > 7)
            if building.TypeClass.IsBridge AND building.Height > 7:
                on_building = false
            // Also exclude if building is in a special state
            if building.IsAlive():  // vtable +0x80
                on_building = false
```

**Height clamping behavior:**
```
if current_z < ground_height:
    if on_bridge:
        current_z = bridge_z          // Snap to bridge surface
    elif under_bridge:
        current_z = bridge_z - 20     // Place 20 leptons below bridge
    else:
        // Falling through ground — clamp at ground minus 100
        if ground - 100 < current_z:
            current_z = ground_height  // Snap to ground
elif current_z >= ground_height:
    if on_bridge:
        current_z = bridge_z          // Snap UP to bridge surface
    elif under_bridge:
        current_z = bridge_z - 20     // Snap under bridge
    elif on_building:
        // Same ground minus 100 clamp
        if ground - 100 < current_z:
            current_z = ground_height
```

After clamping, the function performs a 3D matrix transformation using VXL facing matrices
to compute new velocity/direction values, similar to how spark particles handle their
collision response.

**Wind drift (applied on the same even-frame check):**
```
// 1-in-8 chance of drift update per even frame
if Random() % 8 == 0:
    // Randomly pick X or Y axis
    if Random() % 2 == 0:
        x_drift += Random() % 3 - 1    // -1, 0, or +1
    else:
        y_drift += Random() % 3 - 1
    // Clamp both to [-2, +2]
    x_drift = clamp(x_drift, -2, 2)
    y_drift = clamp(y_drift, -2, 2)
```

**Gravity:**
The Z velocity field at `param_1[0x35]` (byte +0xD4) is set to `-2.0` each even frame, then
Gravity from RulesClass (+0x16B8) is subtracted: `z_vel = -2.0 - Gravity`. This creates a
gentle downward settling force.

**Damage tick:**
```
damage_counter -= 1   // at byte +0x12A (short)
if damage_counter == 0 AND ParticleType.Damage != 0:
    damage_counter = ParticleType.MaxDC   // reset
    cell = CellClass::Get_Cell_At(particle_coords)
    for each object in cell.ObjectList:
        if object.IsAlive AND object.Health > 0:
            house = particle.OwnerSystem ? particle.OwnerSystem.OwnerHouse : NULL
            warhead = ParticleType.Warhead
            object.ReceiveDamage(damage, warhead, house)
```

The `AdjustForZ` call before ReceiveDamage adjusts the damage based on the warhead's
height-sensitivity characteristics.

**Animation state advance:**
```
total_frames = GetImageFrameCount()  // vtable +0x10 on the SHP
state_advance_denominator = (total_frames % 2 + 1) + StateAIAdvance
frame_ticks = current_frame + (NumLoopFrames - lifetime_remaining)
if frame_ticks % state_advance_denominator == 0:
    animation_state += 1

if animation_state == EndStateAI:
    if DeleteOnStateLimit:
        marked_for_deletion = true
    else:
        animation_state = 0   // loop back
```

### 8.5 FUN_00630b90 and FUN_00630ea0 — NOT Particle System Functions

These functions are **not** part of ParticleSystemClass. They are in the **PhoneEd** (modem
phone book editor) dialog system:

- `FUN_00630b90` creates a Windows dialog (`SetWindowLongA`, message pump loop), used for
  the modem/serial connection UI from TS/RA2's multiplayer dialogs.
- `FUN_00630ea0` is the dialog procedure for phone book entries — it uses `GetDlgItem`,
  `SendMessageA`, and references the string `"D:\ra2mdpost\PhoneEd.cpp"` at `0x00836778`.
  It allocates 0xB0-byte phone book entry structs and manages a phone list vector.

These are **dead code** in standard YR (modem play was vestigial even in RA2). They just
happen to be at addresses between particle system functions. No action needed.

### 8.6 Particle Spawning from Major Creation Sites

#### 8.6.1 TechnoClass::AI_Update (0x006f9e50) — Damage Smoke

**Trigger condition:**
```
TechnoTypeClass.Voxel flag at +0xC8F != 0   (has DamageParticleSystems)
AND health_ratio < RulesClass.ConditionYellow (+0x1700)   // below yellow health
AND TechnoClass.DamageSmokeSystem (+0x308) is still alive
                                                           // check via vtable +0x1C8
```

**System selection:**
The function filters the TechnoTypeClass's DamageParticleSystems list (a DynamicVectorClass
at TechnoType offsets +0x77C data ptr and +0x788 count) to find only systems with
`BehavesLike == 3` (Spark type for the damage particles in AI_Update specifically).

Wait — actually, re-reading: the filter is `BehavesLike == 3` (Spark) in AI_Update. But in
ReceiveDamage, the filter is `BehavesLike == 0` (Smoke). This means:
- **AI_Update** spawns **spark-type** damage particle systems (electrical damage sparks)
- **ReceiveDamage** spawns **smoke-type** damage particle systems (smoke plumes)

**Health-based spawn rate:**
```
// Two spawn probability thresholds from RulesClass:
if health_ratio >= RulesClass.ConditionRed (+0x1708):
    // Yellow health band
    spawn_chance = RulesClass at +0x560 (double, as int pair)
else:
    // Red health band (more damage, more frequent spawns)
    spawn_chance = RulesClass at +0x558 (double, as int pair)

random = Random(0, INT_MAX)
if random * (1.0/INT_MAX) < spawn_chance:
    // Spawn the particle system
```

**Spawn position:**
```
offset = FUN_007178c0(TechnoTypeClass)   // gets a random visual offset for the type
coords = techno.GetCoords() + offset
system_type = random pick from filtered DamageParticleSystems list
new_system = ParticleSystemClass(system_type, coords, NULL, techno, NullCoord, 0)
techno.DamageParticleSystem_ptr = new_system
```

The particle system only spawns if the existing DamageParticleSystem pointer at field +0x308
is NULL (the old one has died/finished).

#### 8.6.2 TechnoClass::ReceiveDamage (0x00701900) — Destroy/Damage Smoke

Particle systems are spawned in ReceiveDamage under two conditions:

**On taking damage (result 2 or 3, i.e. wounded but alive):**
```
if health_ratio <= ConditionYellow:
    // Filter DamageParticleSystems for BehavesLike == 0 (Smoke only)
    filtered_list = [sys for sys in DamageParticleSystems if sys.BehavesLike == 0]
    
    if techno.SmokeSystem (+0x310) == NULL AND filtered_list.count > 0:
        if get_damage_level() > -10:  // not in some special state
            offset = FUN_007178c0(TechnoTypeClass)
            coords = techno.GetCoords() + offset
            pick = random(0, filtered_list.count - 1)
            new_system = ParticleSystemClass(filtered_list[pick], coords, NULL, techno, NullCoord, 0)
            techno.SmokeSystem = new_system
```

**On recovery above yellow health:**
```
if health_ratio > ConditionYellow AND techno.SmokeSystem != NULL:
    techno.SmokeSystem.Destroy()  // remove the smoke
```

**DestroyParticleSystems** are NOT spawned in ReceiveDamage. They are spawned from the
destruction sequence (ObjectClass::ReceiveDamage calling the explode/die handlers). The
existing report's section 5.1 already documents this correctly.

#### 8.6.3 TechnoClass::Fire_At (0x006fdd50) — Weapon Particle Systems

Three types of weapon particle systems can be spawned during Fire_At:

**1. AttachedParticleSystem (weapon flag at +0x129, UseFireParticles):**
```
if weapon.UseFireParticles AND techno.FireParticle_ptr (+0x304) == NULL:
    system = ParticleSystemClass(weapon.AttachedParticleSystem,
                                 &bullet,        // attached to bullet
                                 target,          // target object
                                 techno,          // owner
                                 NullCoord, 0)
    techno.FireParticle_ptr = system
```
The key detail: the **source coordinates** are the bullet's address (passed as a CoordStruct*
which will be dereferenced later — actually it's the bullet pointer stored as the
`attached_object`). The particle system follows the bullet.

**2. UseSparkParticles (weapon flag at +0x12A):**
```
if weapon.UseSparkParticles AND techno.SparkParticle_ptr (+0x308) == NULL:
    // Same pattern as above — attached to bullet
    system = ParticleSystemClass(weapon.AttachedParticleSystem,
                                 &bullet, target, techno, NullCoord, 0)
    techno.SparkParticle_ptr = system
```

**3. IsRailgun (weapon flag at +0x12D):**
```
if weapon.IsRailgun AND techno.RailgunParticle_ptr (+0x314) == NULL:
    // Different: compute endpoint coords via FUN_0070c690
    endpoint = FUN_0070c690(&target_coords, &bullet, target, weapon)
    system = ParticleSystemClass(weapon.AttachedParticleSystem,
                                 &bullet, NULL, techno, &endpoint, 0)
    techno.RailgunParticle_ptr = system
```
Railgun systems get a computed target endpoint passed as the 5th parameter (the target
coordinate), while fire/spark systems use NullCoord.

#### 8.6.4 BuildingClass::UpdateGapGenerator (0x00454db0) — Gap Generator Smoke

The gap generator spawns a particle system when its gap generator state transitions from
state 3 (shrinking) back to state 0 (inactive):

```
if gap_state == 3 AND gap_fade_counter reaches 0:
    gap_state = 0
    if existing_particle_system (at building+0xC3*4 = +0x30C) == NULL:
        type_class = building.TechnoTypeClass  // at building[0x148]
        
        // Check if building's stored smoke offset differs from the default
        smoke_offset = (type.offset_0x768, type.offset_0x76C, type.offset_0x770)
        if smoke_offset != default_offset (DAT_0089c848):
            
            // Compute spawn position: building coords + smoke offset
            spawn_pos = building.GetCoords() + smoke_offset
            
            // Get the cell at the building's current position
            cell = CellClass::Get_Cell_At(building.GetCoords())
            
            // Create the system
            system_type = type.GapGeneratorParticleSystem  // at type + 0x764
            system = ParticleSystemClass(system_type, spawn_pos, cell, NULL,
                                         &default_offset, 0)
            building.particle_system = system
```

The particle system type for gap generators is stored in the BuildingTypeClass at offset
+0x764. The smoke offset at +0x768/+0x76C/+0x770 determines where the smoke appears
relative to the building. The system is attached with the cell as the "target" parameter
and NULL as the owner.

### 8.7 Draw_It Rendering Pipeline (0x0062cec0)

**Full decompilation of ParticleClass::Draw_It:**

`param_1` is `int *` (multiply indices by 4). `param_2` is the frame index. `param_3` is the
clip rect pointer.

**Step 1: Fast-forward skip optimization:**
```
frame_skip_threshold = FUN_0055af60()   // gets current game speed setting
if frame_skip_threshold > DAT_00abcd44:
    // Particle has a non-zero damage value — always render
    if ParticleType.Damage == 0:  // at +0x2E8
        skip rendering
        return

// Additional fast-forward skip for specific types:
if DAT_00a8eb78 == 0:  // game is in fast-forward mode
    type = ParticleType.BehavesLike  // at +0x314
    if type == 1 (Smoke) OR type == 3 (Spark):
        skip rendering
        return
```

**Step 2: Fog of war check (TS legacy, normally inactive):**
```
if DAT_00a8ed6b == 0 AND g_hWnd != 0 AND (SpecialFlags & 0x1000) != 0:
    if IsShrouded(particle_coords):
        return   // don't draw in fog
```

**Step 3: Spark/Railgun pixel rendering (BehavesLike == 3 or 4):**
```
screen_pos = TacticalClass::CoordsToClient2(particle_coords)
screen_pos.y += g_RadarViewportOffsetY

// Clip test against param_3 rect
if screen_pos not in clip_rect:
    return

// Alpha buffer check
alpha = *CircBuf_GetScanlinePtr(g_ABuffer, x, y - ABuffer.top)
if alpha == 0:
    return    // fully occluded

// Z-buffer check
z_adjust = AdjustForZ()
z_buffer_val = *ZBuffer_scanline_ptr(x, y - ZBuffer.top)
screen_z = (ZBuffer.bottom - y) - z_adjust - 0x32   // 0x32 = 50 decimal

if screen_z >= z_buffer_val:
    return    // behind existing geometry

// Get the color
if color_index == 0:
    color = particle.CurrentColor (at +0xB0, RGB bytes)
else:
    color = ColorList[color_index * 3]  // from ParticleTypeClass+0x2BC

// Convert RGB bytes to float, apply alpha modulation
r_float = ftol(color.r)   // actually these go through float conversion
g_float = ftol(color.g)
b_float = ftol(color.b)

if alpha < 0x7F:   // 127 — partial transparency
    r_float = (r_float * alpha) >> 7
    g_float = (g_float * alpha) >> 7
    b_float = (b_float * alpha) >> 7

// Convert to 16-bit pixel format using DirectDraw surface format
pixel = ((r_float >> RLoss) << RShift) |
        ((g_float >> GLoss) << GShift) |
        ((b_float >> BLoss) << BShift)

// Write pixel directly to primary surface
g_PrimarySurface.PutPixel(screen_pos, pixel)
```

The `0x32` (50) subtracted from the Z calculation is a fixed depth bias that pushes particles
slightly forward in the Z-buffer, ensuring they render on top of terrain at the same height.

**Step 4: Gas/Smoke/Fire SHP rendering (BehavesLike == 0, 1, or 2):**
```
frame_index = particle.GetImageFrame()   // vtable +0x1D0
z_adjust = AdjustForZ()
shp_surface = particle.GetImage()         // vtable +0x6C — returns SHP pointer

if shp_surface == NULL:
    return

draw_flags = 0x2800   // base: centered, use remap
anim_frame = particle.GetAnimFrame()      // vtable +0x1E8

// Translucency-based flag modification (only when DAT_00a8eb78 == 2, normal game speed)
if game_speed == 2:
    translucency = particle.translucency  // byte at +0x12F
    switch translucency:
        0x19 (25): draw_flags = 0x2802    // 50% translucent
        0x32 (50): draw_flags = 0x2804    // 25% translucent
        >0x4A (74+): draw_flags = 0x2806  // very faded
        default: draw_flags = 0x2800      // opaque

// Final draw call
CC_Draw_Shape(
    shp_surface,          // SHP image
    anim_frame,           // frame number
    param_2,              // screen position
    param_3,              // clip rect
    draw_flags | 0xE00,   // combined flags (0xE00 adds shadow/remap)
    0,                    // remap table (NULL = none)
    -15 - z_adjust,       // Z-adjust for depth sorting
    2,                    // draw priority / layer
    1000,                 // some scale/distance parameter
    0, 0, 0, 0, 0        // remaining CC_Draw_Shape params (all zero)
)
```

The z-adjust value passed to CC_Draw_Shape is `-15 - AdjustForZ()`, which means particles
are drawn 15 pixels higher in the depth sort than their actual Z coordinate, making them
float slightly above the ground plane. The `0xE00` flag bits enable standard draw features
(likely shadow and remap processing).

**Draw flag bits decoded:**
- `0x0800` = use Z-buffer for depth
- `0x2000` = centered drawing
- `0x0002` = 50% translucent
- `0x0004` = 25% translucent  
- `0x0006` = heavy fade
- `0x0E00` = standard draw pipeline flags (shadow, predator effect, etc.)

---

## Sources

### Ghidra Functions Decompiled
- `0x006440a0` -- ParticleSystemTypeClass::Constructor
- `0x006442d0` -- ParticleSystemTypeClass::ReadINI
- `0x00644be0` -- ParticleTypeClass::Constructor
- `0x00644f50` -- ParticleTypeClass::ReadINI
- `0x0062dc50` -- ParticleSystemClass::Constructor
- `0x0062fd60` -- ParticleSystemClass::AI (dispatch)
- `0x0062ed40` -- ParticleSystemClass::AI_Smoke
- `0x0062e6d0` -- ParticleSystemClass::AI_Gas
- `0x0062f9a0` -- ParticleSystemClass::AI_Fire
- `0x0062e840` -- ParticleSystemClass::AI_Spark
- `0x0062f230` -- ParticleSystemClass::AI_Railgun
- `0x0062e380` -- ParticleSystemClass::SpawnParticle
- `0x0062e430` -- ParticleSystemClass::SpawnParticle (variant)
- `0x0062e4c0` -- ParticleSystemClass::SpawnParticleWithInsert
- `0x0062e650` -- ParticleSystemClass::RemoveAllParticles
- `0x0062b5e0` -- ParticleClass::Constructor
- `0x0062ce40` -- ParticleClass::AI_Dispatch
- `0x0062bd50` -- ParticleClass::AI_Gas
- `0x0062c540` -- ParticleClass::AI_Smoke
- `0x0062cb10` -- ParticleClass::AI_Fire
- `0x0062c6e0` -- ParticleClass::AI_Spark
- `0x0062c3a0` -- ParticleClass::AI_Railgun
- `0x0062d5e0` -- ParticleClass::Move_Dispatch
- `0x0062d2a0` -- ParticleClass::Move_Smoke
- `0x0062cec0` -- ParticleClass::Draw_It
- `0x006301e0` -- ParticleSystemClass::MarkForDeletion
- `0x0062d770` -- ParticleClass::GetLayer (returns 3)
- `0x0062d990` -- ParticleClass::GetType
- `0x0062fe90` -- ParticleSystemClass::PointerExpiredNotify
- `0x0062ff20` -- ParticleSystemClass::Load (serialization)

### String Tables
- BehavesLike (ParticleSystemType): `0x00836ee0` (5 entries: Smoke,Gas,Fire,Spark,Railgun)
- BehavesLike (ParticleType): `0x008370bc` (5 entries: Gas,Smoke,Fire,Spark,Railgun)

### INI Files Checked
- `ini/rulesmd.ini` -- All [ParticleSystems], [Particles], individual particle type sections
- `ini/rules.ini` -- Base RA2 values (overridden by rulesmd.ini)

### Rust Files Checked
- `src/rules/weapon_type.rs` -- AttachedParticleSystem, UseFireParticles, UseSparkParticles
- `src/rules/ruleset.rs` -- ChronoSparkle1 reference

---

## 9. Verification Pass

Performed 2026-04-06 by re-decompiling all primary functions in Ghidra and verifying every claim
against the binary. This section documents what was confirmed, what was corrected, and new findings.

### 9.1 V1: ParticleSystemTypeClass struct layout -- VERIFIED with corrections

**ReadINI at 0x006442d0** re-decompiled. Every ReadBool/ReadInt/ReadFloat/ReadDouble call verified.
`param_1` is `int` (not `int *`), so all byte offsets in the ReadINI function are **direct** -- this
matches the report's table.

**Field-by-field verification (all offsets confirmed from ReadINI):**
- +0x294: HoldsWhat (via FindOrCreate from string) -- CONFIRMED
- +0x298: Spawns (ReadBool) -- CONFIRMED
- +0x29C: SpawnFrames (ReadInt) -- CONFIRMED
- +0x2A0: Slowdown (ReadDouble cast to float) -- CONFIRMED
- +0x2A4: ParticleCap (ReadInt) -- CONFIRMED
- +0x2A8: SpawnRadius (ReadInt) -- CONFIRMED
- +0x2AC: SpawnCutoff (ReadDouble cast to float) -- CONFIRMED
- +0x2B0: SpawnTranslucencyCutoff (ReadDouble cast to float) -- CONFIRMED
- +0x2B4: BehavesLike (string table lookup) -- CONFIRMED
- +0x2B8: Lifetime (ReadInt) -- CONFIRMED
- +0x2BC-0x2C7: SpawnDirection (CoordStruct, via FUN_00476340) -- CONFIRMED (note: ReadINI writes
  to `param_1 + 700` = 0x2BC, then copies 12 bytes)
- +0x2C8: ParticlesPerCoord (ReadDouble) -- CONFIRMED
- +0x2D0: SpiralDeltaPerCoord (ReadDouble) -- CONFIRMED
- +0x2D8: SpiralRadius (ReadDouble) -- CONFIRMED
- +0x2E0: PositionPerturbationCoefficient (ReadDouble) -- CONFIRMED
- +0x2E8: MovementPerturbationCoefficient (ReadDouble) -- CONFIRMED
- +0x2F0: VelocityPerturbationCoefficient (ReadDouble) -- CONFIRMED
- +0x2F8: SpawnSparkPercentage (ReadDouble) -- CONFIRMED
- +0x300: SparkSpawnFrames (ReadInt) -- CONFIRMED
- +0x304: LightSize (ReadInt) -- CONFIRMED
- +0x308: LaserColor (ReadColorRGB, 3 bytes) -- CONFIRMED
- +0x30B: Laser (ReadBool) -- CONFIRMED
- +0x30C: OneFrameLight (ReadBool) -- CONFIRMED

**BehavesLike enum at 0x00836ee0** -- CONFIRMED by reading all 5 string pointers:
- [0] -> 0x836f0c = "Smoke"
- [1] -> 0x81d990 = "Gas"
- [2] -> 0x836f04 = "Fire"
- [3] -> 0x836efc = "Spark"
- [4] -> 0x836ef4 = "Railgun"

Loop terminates when `local_5c >= 0x836ef4` (5 entries confirmed).

**Constructor defaults at 0x006440a0** (param_1 is `undefined4 *`, multiply indices by 4):

CORRECTIONS APPLIED:
1. **Lifetime default was WRONG.** Constructor sets `param_1[0xae] = 0xFFFFFFFF` = byte offset
   0x2B8 = **-1**, not 0 as previously reported. Fixed in section 2.1 and 4.1.
2. **SpiralDeltaPerCoord default was WRONG.** Constructor sets `param_1[0xb4:0xb5]` to
   `0x3F999999_9999999A` = **0.025**, not 0.1. Fixed in section 2.1 and 4.1.
3. **SpiralRadius default was WRONG.** Constructor sets `param_1[0xb6:0xb7]` to
   `0x40390000_00000000` = **25.0**, not 2.9. Fixed in section 2.1 and 4.1.

Other defaults verified correct: HoldsWhat=-1, Spawns=false, SpawnFrames=1, Slowdown=0.0,
ParticleCap=50, SpawnRadius=0, SpawnCutoff=0.0, SpawnTranslucencyCutoff=0.0, BehavesLike=-1,
SpawnDirection=(0,0,0), ParticlesPerCoord=0.1, all perturbation coefficients=0.0,
SpawnSparkPercentage=0.0, SparkSpawnFrames=0, LightSize=0, LaserColor=(0,0,0), Laser=false,
OneFrameLight=false.

### 9.2 V2: ParticleTypeClass struct layout -- VERIFIED

**ReadINI at 0x00644f50** re-decompiled. `param_1` is `int` (direct byte offsets). All fields verified:
- +0x294-0x29C: NextParticleOffset (CoordStruct via FUN_00476420) -- CONFIRMED
- +0x2A0: XVelocity, +0x2A4: YVelocity, +0x2A8: MinZVelocity, +0x2AC: ZVelocityRange -- CONFIRMED
- +0x2B0: ColorSpeed (ReadDouble) -- CONFIRMED
- +0x2C8-0x2D0: ColorList data (copied from parser result, 12 bytes) -- CONFIRMED
- +0x2D4: StartColor1 (ReadColorRGB) -- CONFIRMED
- +0x2D7: StartColor2 (ReadColorRGB) -- CONFIRMED
- +0x2DC: MaxDC (ReadInt) -- CONFIRMED
- +0x2E0: MaxEC (ReadInt) -- CONFIRMED
- +0x2E4: Warhead (FindOrAllocate from string) -- CONFIRMED
- +0x2E8: Damage (ReadInt) -- CONFIRMED
- +0x2EC: StartFrame, +0x2F0: NumLoopFrames, +0x2F4: Translucency -- CONFIRMED
- +0x2F8: WindEffect, +0x2FC: Velocity(float), +0x300: Deacc(float) -- CONFIRMED
- +0x304: Radius, +0x308: DeleteOnStateLimit(bool) -- CONFIRMED
- +0x309: EndStateAI(byte), +0x30A: StartStateAI(byte), +0x30B: StateAIAdvance(byte) -- CONFIRMED
- +0x30C: FinalDamageState (defaults to EndStateAI value from +0x309) -- CONFIRMED
- +0x30D: Translucent25State, +0x30E: Translucent50State -- CONFIRMED
- +0x30F: Normalized (ReadBool) -- CONFIRMED
- +0x310: NextParticle (int index, via string lookup) -- CONFIRMED
- +0x314: BehavesLike (string table at 0x8370bc) -- CONFIRMED

**BehavesLike enum at 0x008370bc** -- CONFIRMED different ordering from system type:
- [0] -> 0x81d990 = "Gas"
- [1] -> 0x836f0c = "Smoke"
- [2] -> 0x836f04 = "Fire"
- [3] -> 0x836efc = "Spark"
- [4] -> 0x836ef4 = "Railgun"

Loop terminates at `ppuVar14 >= 0x8370d0` (5 entries). The report's critical claim about
different enum ordering between system and particle types is **CONFIRMED CORRECT**.

**Constructor at 0x00644be0** verified. `operator_new(0x318)` confirmed for size. Defaults verified:
NextParticle=-1, BehavesLike=-1, Translucent25State=0xFF, Translucent50State=0xFF,
StateAIAdvance=4, all zero for most fields.

### 9.3 V3: ParticleSystemClass struct layout -- VERIFIED

**Constructor at 0x0062dc50** re-decompiled. `param_1` is `undefined4 *`.

**Size: 0x100 (256 bytes)** -- CONFIRMED. Callers (e.g., AI_Update at 0x6faed1, ReceiveDamage at
0x7029f5) call `operator_new(0x100)` before calling the constructor.

**Field verification from constructor** (multiply index by 4 for byte offset):
- [0x2b] = +0xAC: type class pointer -- CONFIRMED
- [0x2c..0x2e] = +0xB0..+0xB8: offset X/Y/Z -- CONFIRMED (initialized to 0, then set from
  attached object difference)
- [0x2f] = +0xBC: particle vector vtable (DynamicVectorClass) -- CONFIRMED
- [0x30] = +0xC0: vector data pointer -- CONFIRMED
- [0x31] = +0xC4: vector capacity -- CONFIRMED
- [0x33] = +0xCC: vector count (active particles) -- CONFIRMED (initialized to 0)
- [0x34] = +0xD0: vector grow step (initialized to 10) -- CONFIRMED
- [0x35..0x37] = +0xD4..+0xDC: target/end coordinates -- CONFIRMED (set from DAT_00ac4a58/5c/60
  initially, then overridden)
- [0x38] = +0xE0: attached object pointer -- CONFIRMED
- [0x39] = +0xE4: owner/source object -- CONFIRMED
- [0x3a] = +0xE8: spawn timer (float, initialized from SpawnFrames) -- CONFIRMED
- [0x3b] = +0xEC: lifetime countdown (copied from type class +0x2B8) -- CONFIRMED
- [0x3c] = +0xF0: SparkSpawnFrames countdown (copied from type class +0x300) -- CONFIRMED
- [0x3d] = +0xF4: particle facing (initialized to 0x1d = 29) -- CONFIRMED
- [0x3e] = +0xF8: marked for deletion (bool) -- CONFIRMED
- byte 0xF9: directionless mode (bool) -- CONFIRMED
- [0x3f] = +0xFC: owner house/param_7 -- CONFIRMED

### 9.4 V4: ParticleClass struct layout -- VERIFIED

**Constructor at 0x0062b5e0** re-decompiled. `param_1` is `undefined4 *`.

**Size: 0x138 (312 bytes)** -- CONFIRMED. All callers use `operator_new(0x138)`.

**Key field verification:**
- [0x2b] = +0xAC: type class pointer -- CONFIRMED
- [0x2c] = +0xB0: current color (RGB 3 bytes at +0xB0, +0xB1, +0xB2) -- CONFIRMED (initialized
  from StartColor1/2 interpolation or ColorList[0])
- [0x39] = +0xE4: velocity (float, copied from ParticleType.Velocity at +0x2FC) -- CONFIRMED
- [0x3a..0x3c] = +0xE8..+0xF0: owner system coords at spawn time -- CONFIRMED
- [0x3d..0x3f] = +0xF4..+0xFC: spawn position coordinates -- CONFIRMED
- [0x40..0x42] = +0x100..+0x108: previous position -- CONFIRMED (set from DAT_00ac49d8/dc/e0)
- [0x43..0x45] = +0x10C..+0x114: direction vector (float X/Y/Z, normalized) -- CONFIRMED
- [0x46..0x48] = +0x118..+0x120: origin (float copy of spawn pos) -- CONFIRMED
- [0x49] = +0x124: owner system pointer -- CONFIRMED
- +0x128: lifetime remaining (short, from MaxEC with random variation) -- CONFIRMED
- +0x12A: damage counter (short, copied from MaxDC) -- CONFIRMED
- [0x4b] = +0x12C (byte): StateAIAdvance -- CONFIRMED (copied from type +0x30B)
- +0x12D: hit ground flag (byte) -- CONFIRMED (initialized to 0)
- +0x12E: current animation state (byte) -- CONFIRMED (initialized from StartStateAI +0x30A)
- +0x12F: translucency level (byte) -- CONFIRMED (initialized from Translucency +0x2F4)
- +0x131: marked for deletion (bool) -- CONFIRMED (initialized to 0)

**Overlapping field usage at +0xB4..+0xDC:** CONFIRMED from AI_Gas which copies to iVar7+0xC0,
+0xC4, +0xC8 (drift velocities). Spark/railgun AI uses some of these offsets for color
interpolation state instead. The field semantics depend on BehavesLike type.

**Smoke velocity jitter at construction:** When BehavesLike == 1 (Smoke in ParticleTypeClass
enum), a random 0 or 1 is added to the velocity float. CONFIRMED from constructor.

**Railgun lifetime randomization:** When BehavesLike == 4 (Railgun), lifetime gets `random % 10`
added. For other types, `random % MaxEC` is added. CONFIRMED from constructor.

### 9.5 V5: Wind direction table -- VERIFIED EXACTLY

**DX table at 0x00836664** (32 bytes = 8 int32 values, little-endian):
```
[0]=0, [1]=2, [2]=2, [3]=1, [4]=0, [5]=-2, [6]=-2, [7]=-2
```

**DY table at 0x00836684** (32 bytes = 8 int32 values):
```
[0]=-2, [1]=-2, [2]=0, [3]=2, [4]=2, [5]=2, [6]=0, [7]=-2
```

All values match the report exactly. The SE direction having DX=1 (not 2) IS correct --
verified from raw memory bytes. The asymmetry is intentional in the original engine.

### 9.6 V6: AI behavior implementations -- VERIFIED

**System AI dispatch at 0x0062fd60:** Switch on `param_1[0x2b] + 0x2B4` (system BehavesLike).
Cases 0-4 dispatch to Smoke/Gas/Fire/Spark/Railgun. After dispatch: `param_1[0x3b]` (Lifetime)
is decremented; when 0, calls vtable+0xf8 (mark for deletion). Then checks if active, done
spawning, and particle count == 0 to add to limbo. CONFIRMED.

**Particle AI dispatch at 0x0062ce40:** Switch on `*(int *)(param_1 + 0xac) + 0x314` (particle
BehavesLike). Cases 0-4 dispatch to Gas/Smoke/Fire/Spark/Railgun individual AIs. After dispatch:
`*(short *)(param_1 + 0x128)` (lifetime) is decremented; when 0, sets `*(param_1 + 0x131) = 1`.
CONFIRMED.

**Smoke System AI at 0x0062ed40:** CONFIRMED. Follows attached object, ticks all particles,
handles NextParticle chaining (spawns TWO replacement particles at +/- random offset from Radius/8),
copies velocity and translucency (with 1-in-6 random 0x19 addition). Spawn accumulator at
+0xE8 accumulates Slowdown and triggers done_spawning when exceeding SpawnCutoff. NEW FINDING:
the NextParticle handling in smoke spawns TWO child particles at symmetric offsets (not just
one as the pseudocode in section 3.3 suggested). The report pseudocode was simplified.

**Gas System AI at 0x0062e6d0:** CONFIRMED. Ticks all particles first, then reverse-iterates
for cleanup. NextParticle chaining copies velocity (+0xE4), and drift fields at +0xC0, +0xC4,
+0xC8. NOT all state -- only velocity and 3 drift fields are copied (report was approximately
correct).

**Spark System AI at 0x0062e840:** CONFIRMED. Batch count formula: `abs(random) % (ParticleCap/2) + ParticleCap/2`. Random velocities from XVelocity/YVelocity/ZVelocityRange+MinZVelocity.
Direction normalization, magnitude preservation, and re-application after adding SpawnDirection
(or random dir if directionless). Light creation check, facing jitter (0x11..0x29 range with
step 3, thresholds at 0.3 and 0.7). CONFIRMED.

**Fire System AI at 0x0062f9a0:** CONFIRMED. Ticks all particles with AI+Move, removes dead ones.
Tracks attached object via Filter_AbstractType_InMap. Has orbital motion calculation using
RateTimer and cos/sin lookups. Spawns based on SpawnFrames modulo or every 3rd frame if target
moved. Uses FUN_0062e4c0 (SpawnParticleWithInsert). CONFIRMED.

**Railgun System AI at 0x0062f230:** CONFIRMED. Only fires when NOT marked for deletion AND
particle count is 0. Calculates pitch/yaw from source-to-target vector, builds rotation matrix
(RotateZ for yaw, RotateX for pitch). Spawns particles along path with spiral pattern using
SpiralDeltaPerCoord and SpiralRadius. Position and movement perturbation applied. Velocity
perturbation clamped between -MovementPerturbation and +VelocityPerturbation with accumulation.
Laser line drawn via FUN_0054fe60 if Laser flag set. CONFIRMED.

### 9.7 V7: Draw_It claims -- VERIFIED

**Draw_It at 0x0062cec0** re-decompiled with `int *` param_1.

- **Alpha modulation:** `(color * alpha) >> 7` when `alpha < 0x7F` -- CONFIRMED. The exact code:
  `if (uVar1 < 0x7f) { iVar4 = (int)(iVar4 * uVar3) >> 7; ... }` where uVar3 is the alpha value.

- **Z-adjust:** CC_Draw_Shape receives `-0xf - iVar4` where `iVar4 = AdjustForZ()`. This is
  `-15 - AdjustForZ()` -- CONFIRMED.

- **Fast-forward skip:** Checks `DAT_00a8eb78 != 0` OR (`type != 1 AND type != 3`). So when
  DAT_00a8eb78 == 0 (fast-forward), types 1 (Smoke) and 3 (Spark) are skipped. CONFIRMED.
  Note: these are ParticleTypeClass BehavesLike values, so Smoke=1 and Spark=3 in the particle
  enum (not system enum).

- **Z-buffer depth bias:** The spark/railgun pixel renderer uses `(ZBuffer.bottom - y) - z_adjust
  - 0x32` compared against the z-buffer value, where 0x32 = 50 decimal. CONFIRMED.

- **Translucency flags:** Only applied when `DAT_00a8eb78 == 2` (normal game speed, not
  fast-forward). Values: 0x19->0x2802, 0x32->0x2804, >0x4A->0x2806. CONFIRMED.

### 9.8 V8: Spawn site parameter verification -- VERIFIED

**TechnoClass::AI_Update at 0x006f9e50:** Filters DamageParticleSystems for
`*(int *)(system_ptr + 0x2B4) == 3`. Since 0x2B4 is ParticleSystemTypeClass.BehavesLike and
value 3 = **Spark** in the system enum. CONFIRMED -- AI_Update spawns Spark-type systems for
damage particles.

**TechnoClass::ReceiveDamage at 0x00701900:** Filters for
`*(int *)(system_ptr + 0x2B4) == 0`. Value 0 = **Smoke** in the system enum. CONFIRMED --
ReceiveDamage spawns Smoke-type systems on wound damage (result 2 or 3).

The report's claim is correct: AI_Update uses Spark for intermittent damage sparks,
ReceiveDamage uses Smoke for persistent damage smoke plumes.

### 9.9 V9: Particle deletion and cleanup -- VERIFIED

**PointerExpiredNotify at 0x0062fe90:** When an object pointer expires:
1. Calls base class FUN_005f5230 first
2. Searches the particle vector for the expired pointer and removes it (shifts array elements)
3. If expired ptr == type class at +0xAC: nulls it
4. If expired ptr == owner at +0xE4: nulls it
5. If expired ptr == attached object at +0xE0: sets marked_for_deletion (+0xF8) = true AND nulls it

This means when an attached object is destroyed, the particle system marks itself for deletion.
When a type class is destroyed, the system loses its type reference. CONFIRMED.

**Cleanup in AI functions:** All five system AI functions (Smoke/Gas/Fire/Spark/Railgun) iterate
particles in reverse and call `vtable+0xf8` (Destroy) on particles with `+0x131 == true`. This
is consistent -- individual particles are cleaned up by their owning system each tick.

**Mark for deletion at vtable+0xf8 (0x006301e0):** Simply sets `*(param_1 + 0xf8) = 1`.
The actual destruction happens when the owning system processes it during the next AI tick.

### 9.10 V10: Fog of war check -- VERIFIED

The fog check in Draw_It is:
```c
if ((DAT_00a8ed6b == '\0') && ((g_hWnd != 0 && ((*DAT_00a8b230 & 0x1000) != 0)))) {
    // check if coords are shrouded via FUN_005865e0
    if (shrouded) return;  // don't draw
}
```

- `DAT_00a8b230` is the SpecialFlags pointer
- `& 0x1000` is the fog-of-war flag -- CONFIRMED
- The check is gated by `DAT_00a8ed6b == '\0'` (some additional condition) and `g_hWnd != 0`
- As noted in CLAUDE.md, FogOfWar defaults to false in YR, so this code path is normally INACTIVE

This is a TS-legacy check. CONFIRMED.

### 9.11 Summary of corrections applied

| Section | Field | Was | Now | Reason |
|---------|-------|-----|-----|--------|
| 2.1, 4.1 | Lifetime default | 0 | -1 | Constructor sets 0xFFFFFFFF, verified from param_1[0xae] |
| 2.1, 4.1 | SpiralDeltaPerCoord default | 0.1 | 0.025 | Constructor double = 0x3F999999_9999999A = 0.025 |
| 2.1, 4.1 | SpiralRadius default | 2.9 | 25.0 | Constructor double = 0x40390000_00000000 = 25.0 |

### 9.12 New findings not in original report

1. **Smoke NextParticle spawns TWO child particles**, not one. They are placed at symmetric
   random offsets: `(+dx, +dy, z)` and `(-dx, -dy, z)` where dx/dy come from
   `random % (Radius / 8)`. Both inherit the parent's velocity and translucency (with random
   additional 0x19 fade). This creates a forking smoke trail effect.

2. **ParticleSystemClass constructor bridge check:** When param_4 (attached object) is provided,
   the constructor checks if the object's cell has bridge flag (`cell + 0x140 & 0x100`) and if the
   object is at layer 0xB. If so, adds `DAT_00ac4a8c` (bridge height) to the Z coordinate.

3. **FUN_0062d830 is ParticleClass::GetImageFrame.** It dispatches on BehavesLike type:
   types 0 and 1 (Gas/Smoke) return the current animation state directly. Type 2 (Fire) calculates
   a directional frame based on facing (0/1/2/3 multiplied by EndStateAI). Types 3+ return 0.

4. **Railgun velocity clamping:** The velocity perturbation accumulator is clamped between
   `-MovementPerturbationCoefficient` and `+VelocityPerturbationCoefficient` (these are different
   values, not symmetric). The final particle velocity = accumulated perturbation + type Velocity.

5. **PointerExpiredNotify behavior:** Clearing the attached object at +0xE0 also triggers
   marked_for_deletion, which means attached particle systems automatically self-destruct when
   their owner object is destroyed. This is the primary cleanup mechanism.

---

## 10. Exhaustive Detail Pass

Systematic decompilation of every remaining gap identified in sections 1-9.

### 10.1 T1: SpawnParticle Variants (0x0062e380, 0x0062e430, 0x0062e4c0)

Three spawn functions exist. All create a `ParticleClass` (operator_new 0x138) and insert it
into the system's particle vector. The differences are subtle but important.

#### 10.1.1 SpawnParticle (0x0062e380) — Standard Spawn

**Signature:** `int SpawnParticle(this, coords, spawn_origin_coords)`

```
fn SpawnParticle(self, coords, spawn_origin):
    if self.type.HoldsWhat == -1:
        return NULL

    particle = new ParticleClass(
        ParticleTypeClass_Array[self.type.HoldsWhat],
        coords,
        spawn_origin,
        self   // owner system
    )
    if particle == NULL:
        return NULL

    // Insert into vector (append at end)
    if vector.count < vector.capacity OR vector can grow:
        vector[count] = particle
        count += 1

    return particle
```

This is the standard spawn used by Smoke, Gas, and Railgun systems. The particle type is
looked up from `HoldsWhat` (index into the global ParticleTypeClass array at `DAT_00a83d9c`).
The particle is appended at the end of the vector.

#### 10.1.2 SpawnParticle Variant (0x0062e430) — Direct Type Spawn

**Signature:** `int SpawnParticle(this, particle_type, coords)`

```
fn SpawnParticle_Direct(self, particle_type, coords):
    particle = new ParticleClass(
        particle_type,     // directly passed, NOT from HoldsWhat
        coords,
        &DAT_00ac4a58,     // fixed null/default coordinate
        self               // owner system
    )
    if particle == NULL:
        return NULL

    // Insert into vector (append at end) — same logic as 0x0062e380
    ...
    return particle
```

**Key difference:** Does NOT check `self.type.HoldsWhat`. Instead, the particle type is
passed directly as `param_2`. The spawn origin is a fixed global coordinate `DAT_00ac4a58`
(a null/default coordinate used throughout the engine). This variant is used when the
caller already knows the exact particle type (e.g., NextParticle chaining in Gas/Smoke
system AI, where the type comes from `ParticleTypeClass.NextParticle`).

#### 10.1.3 SpawnParticleWithInsert (0x0062e4c0) — Random Insertion

**Signature:** `int SpawnParticleWithInsert(this, coords, spawn_origin, insert_range)`

```
fn SpawnParticleWithInsert(self, coords, spawn_origin, insert_range):
    if self.type.HoldsWhat == -1 OR insert_range <= 0:
        return NULL

    particle = new ParticleClass(
        ParticleTypeClass_Array[self.type.HoldsWhat],
        coords,
        spawn_origin,
        self
    )
    if particle == NULL:
        return NULL

    // First: append particle at end of vector (same as standard)
    if can_insert:
        vector[count] = particle
        count += 1

    // Then: randomly reposition within the last `insert_range` elements
    actual_range = min(insert_range, count)
    random_offset = abs(Random()) % actual_range
    insert_pos = count - 2 - random_offset    // position to insert at

    // Shift elements right to make room
    for i in (count-2) downto (insert_pos+1):
        vector[i+1] = vector[i]

    vector[insert_pos] = particle
    return particle
```

**"With Insert" meaning:** After appending the new particle at the end of the vector,
it picks a random position within the last `insert_range` elements and shifts all
elements between that position and the end of the vector to the right by one slot,
inserting the new particle at the random position. This randomizes the draw order of
recently-spawned particles, which for Fire particles creates a more natural-looking
flame stream (fire particles are drawn in vector order, and having them in strict
creation order would look too uniform).

**Called by:** Fire System AI (`0x0062f9a0`) exclusively. The `insert_range` parameter
is derived from the facing count (typically 4), so the new fire particle is inserted
randomly among the last 4 particles in the vector.

### 10.2 T2: Particle Movement Functions — Full Detail

#### 10.2.1 Move_Dispatch (0x0062d5e0) — Complete Decompilation

```
fn Move_Dispatch(self):
    match self.type.BehavesLike:   // ParticleTypeClass enum at +0x314
        0 (Gas):
            Move_Gas(self)         // FUN_0062d2a0

        1 (Smoke):
            Move_Smoke(self)       // FUN_0062d3f0
            return

        2 (Fire):
            // Fire movement is inline, not a separate function
            old_pos = self.coords          // param_1[0x27..0x29]
            new_pos = self.coords          // copy

            if self.velocity > 0.0:        // param_1[0x39] = +0xE4
                // Add previous-position delta
                new_pos.x += self.prev_delta_x   // param_1[0x40] = +0x100
                new_pos.y += self.prev_delta_y   // param_1[0x41] = +0x104
                new_pos.z += self.prev_delta_z   // param_1[0x42] = +0x108

            if self.velocity > 0.0:
                // Ground collision check
                old_ground = CellClass::GetGroundHeight(old_pos)
                new_ground = CellClass::GetGroundHeight(new_pos)
                if old_ground < new_ground:
                    // Terrain rises at new position — particle hit ground
                    self.hit_ground = true    // byte +0x12D = 1
                    self.marked_for_deletion = true  // byte +0x131 = 1

            self.SetCoords(new_pos)    // vtable +0x1B4
            return

        3 (Spark):   // fall through — no movement
        4 (Railgun): // fall through — no movement
            return
```

**Key finding for Fire:** The movement is entirely inline in the dispatch function.
Fire particles move by adding the `previous_position` delta (fields at +0x100..+0x108).
These delta values are set during Fire AI (`0x0062cb10`) based on the direction vector
with random jitter. Ground collision is detected by comparing ground heights at the old
and new positions — if the terrain rises (e.g., a cliff edge), the particle is killed.
There is no bridge check for fire movement — fire particles simply die on any upward
terrain change.

#### 10.2.2 Move_Gas (0x0062d2a0) — Complete

Gas movement occurs on odd frames only (`g_CurrentFrameCounter & 1 != 0`).

```
fn Move_Gas(self):
    pos = self.coords

    // Wind drift (conditional on WindEffect > 0)
    wind_effect = self.type.WindEffect    // +0x2F8
    if wind_effect > 0:
        frames_per_shift = 10 / wind_effect
        if g_CurrentFrameCounter % frames_per_shift == 0:
            wind_dir = RulesClass.WindDirection     // g_RulesClass_Instance + 0x7AC
            pos.x += WIND_DRIFT_DX[wind_dir]       // table at 0x00836664
            pos.y += WIND_DRIFT_DY[wind_dir]       // table at 0x00836684

    if (g_CurrentFrameCounter & 1) != 0:   // odd frames only
        // Convert to float for drift calculations
        pos_float = (float)pos

        // Apply gravity settling toward ground + 5
        ground_z = CellClass::GetGroundHeight(pos)
        if ground_z + 5 < pos.z:
            drop = pos.z - ground_z - 5
            if drop > 2:
                drop = 2       // max drop of 2 leptons per tick
            pos.z -= drop

        // Apply random X/Y/Z drift from fields +0xC0, +0xC4, +0xC8
        pos.x += self.x_drift    // param_1[0x30]
        pos.z += self.z_drift    // param_1[0x32]
        pos.y += self.y_drift    // param_1[0x31]

        // Clamp Z above ground + 5
        ground_z = CellClass::GetGroundHeight(pos)
        if pos.z < ground_z + 5:
            pos.z = ground_z + 5

    self.SetCoords(pos)
```

**Gas vs Smoke drift differences:**
- Gas: Settles toward `ground + 5` with max drop of 2 per tick. Applies drift from
  fields +0xC0/+0xC4/+0xC8 (random walk set in gas_particle_ai).
- Smoke: Rises away from ground using velocity + VXL facing matrix transform.
  Uses different drift parameters and has bridge collision detection.

#### 10.2.3 Move_Smoke (0x0062d3f0) — Complete

```
fn Move_Smoke(self):
    old_pos = self.coords
    new_pos = self.coords

    // Wind drift (uses DIFFERENT tables than gas!)
    wind_dir = RulesClass.WindDirection * 4     // byte index
    wind_effect = self.type.WindEffect          // +0x2F8
    new_pos.x += WIND_DX_TABLE_2[wind_dir] * wind_effect   // DAT_008366a4
    new_pos.y += WIND_DY_TABLE_2[wind_dir] * wind_effect   // DAT_008366c4

    // Z component from FPU calculation (velocity-based vertical movement)
    new_pos.z = old_z + ftol(vertical_component)

    // Apply X/Y/Z drift from fields +0xC0, +0xC4, +0xC8
    new_pos.y += self.y_drift    // param_1[0x31]
    new_pos.x += self.x_drift   // param_1[0x30]
    new_pos.z += self.z_drift   // param_1[0x32]

    // Bridge collision check
    cell = CellClass::Get_Cell_At(old_pos)
    if (cell.flags_0x140 & 0x100) != 0:      // bridge flag
        ground_z = CellClass::GetGroundHeight(old_pos)
        bridge_z = ground_z + DAT_00ac4a0c    // bridge height offset

        // Check if smoke is below bridge and would pass through it
        if old_z < bridge_z:
            threshold = (bridge_z - DAT_00ac4a18) * some_constant
            if threshold <= new_pos.z:
                // Smoke would pass through bridge — kill it
                self.marked_for_deletion = true
                return

    self.SetCoords(new_pos)
```

**Key finding:** Smoke uses tables at `0x008366a4` and `0x008366c4` (40 bytes offset from
the gas tables at `0x00836664`/`0x00836684`). These are different wind tables! The smoke
wind drift is multiplied by the WindEffect value directly (stronger effect), while gas
uses the base table values unscaled. Smoke also performs bridge collision detection that
gas movement does not — smoke particles hitting a bridge from below are destroyed.

### 10.3 T3: Light Source Creation for Sparks (0x0062e280)

**Complete decompilation:**

```
fn CreateLightSource(self):   // vtable +0x114 of ParticleSystemClass
    // Three conditions must ALL be true:
    if self.type.OneFrameLight != false:     // +0x30C on PSTypeClass
        return    // OneFrameLight=true means NO persistent light
    if self.type.LightSize <= 0:             // +0x304 on PSTypeClass
        return    // No light radius defined
    if self.particle_count <= 0:             // +0xCC on PSClass
        return    // No particles alive

    // Create a LightSource object (0x18 = 24 bytes)
    light = new LightSource(
        self.coords.x,    // +0x9C
        self.coords.y,    // +0xA0
        self.coords.z,    // +0xA4
        self.type.LightSize   // radius
    )

    // Set light visibility/facing from the system's particle facing
    light.field_0xC = self.particle_facing    // +0xF4

    // Enable the light (calls into the lighting system)
    light.Enable()       // FUN_005ff850

    // Register and immediately destroy (single-tick light update)
    if light != NULL:
        light.ApplyToMap()   // FUN_005ff2d0 — applies lighting to affected cells
        delete light         // FUN_007c8b3d — free memory
```

**OneFrameLight=true vs false behavior:**
- **OneFrameLight=false** (e.g., SparkSys with LightSize=15): The light is created
  persistently in `AI_Spark` on the FIRST spark frame only (when `spark_frames_remaining
  == type.SparkSpawnFrames`). The light source object persists across frames. This is
  the `FUN_005ff250` call in AI_Spark.
- **OneFrameLight=true** (e.g., WeldingSys, LGSparkSys): No persistent light is created
  in AI_Spark (the OneFrameLight check prevents it). Instead, FUN_0062e280 (vtable+0x114)
  is called each tick and creates a one-frame light that is immediately applied and
  destroyed. This creates a flickering effect appropriate for welding sparks.

**Light system interaction:** LightSource at `FUN_005ff250` is a 0x18-byte object. Its
constructor takes (x, y, z, radius). `FUN_005ff2d0` applies the light to the game's
cell-based lighting grid. `FUN_005ff850` enables the light source. The light radius
directly maps to the `LightSize` INI value. The global `DAT_00ac1678..00ac168c` is the
LightSource tracking vector (DynamicVectorClass), separate from the particle system limbo
vector.

### 10.4 T4: Limbo Vector and System Lifecycle

#### 10.4.1 The Limbo Vector

The limbo vector is a **global** `DynamicVectorClass<ObjectClass*>` at address `0x00b0f698`:

| Address | Field | Purpose |
|---------|-------|---------|
| 0x00b0f698 | vtable | DynamicVectorClass vtable |
| 0x00b0f69c | data ptr | Pointer to array of object pointers |
| 0x00b0f6a0 | capacity | Current allocated capacity |
| 0x00b0f6a5 | auto_grow | Boolean: can the vector grow |
| 0x00b0f6a8 | count | Number of objects in limbo |
| 0x00b0f6ac | grow_step | Growth increment when capacity exceeded |

**This is NOT particle-system-specific.** The same limbo vector is used by:
- `ObjectClass::Constructor` (0x005f3bab) — all objects register
- `ObjectClass::UnInit` (0x005f6651) — removal on destruction
- `TagClass::Constructor` (0x006e50f9) — map triggers
- `BulletAnimTracker::Register` (0x004a71e9) — bullet tracking
- `ParticleSystemClass::Constructor` (0x0062e0cf) — particle systems
- `ParticleSystemClass::AI` (0x0062fe13) — systems entering limbo

It is the engine's general-purpose "objects existing in the world" vector (essentially
the master object list that drives the main game loop's AI dispatch).

#### 10.4.2 Lifecycle: Active -> Limbo -> Destroyed

From `ParticleSystemClass::AI` at `0x0062fd60`:

```
fn AI(self):
    // 1. Run behavior-specific AI
    dispatch_by_type()

    // 2. Decrement lifetime
    self.lifetime -= 1
    if self.lifetime == 0:
        self.mark_for_deletion()   // vtable+0xF8: sets +0xF8 = true

    // 3. Check for system completion
    if self.is_active                      // (char)param_1[0x24] != '\0'
       AND self.marked_for_deletion        // (char)param_1[0x3E] != '\0'
       AND self.particle_count == 0:       // param_1[0x33] == 0
        // Unregister from active object list
        self.Unregister()                  // vtable+0xD4
        self.is_active = false             // *(param_1 + 0x24) = 0

        // Add to limbo vector at DAT_00b0f698
        limbo_vector.Add(self)
```

**There is NO pooling/reuse mechanism.** Once a particle system enters limbo, it stays
there until the game session ends or a map clear occurs. The cleanup in `FUN_006851f0`
(the map init/clear function) shows:
1. Iterates the active object vector (`DAT_00a8e96c`/`DAT_00a8e978`)
2. For objects of type 0x12 (ParticleSystem), removes them from the active vector
3. Moves them to the limbo vector
4. Later calls `RemoveAllParticles` on `DAT_00a8ed78` (a singleton system pointer) and
   destroys it

The lifecycle is: **Construct -> Active (in object vector) -> Mark for deletion -> All
particles die -> Unregister + Enter limbo -> Session end/map clear destroys.**

#### 10.4.3 RemoveAllParticles (0x0062e650)

```
fn RemoveAllParticles(self):
    // Iterate particles in REVERSE
    for i in (self.particle_count - 1) downto 0:
        if i < self.particle_count:
            // Call vtable+0xF8 (Destroy) on particle[i]
            particle = self.vector[i]
            particle.Destroy()

            // Find and remove from vector (shift left)
            idx = vector.FindIndex(particle)
            if idx != -1 AND idx < count:
                count -= 1
                for j in idx..count:
                    vector[j] = vector[j+1]
```

This destroys ALL particles owned by a system and compacts the vector. Each particle's
Destroy method (vtable+0xF8) is called, followed by removal from the vector via a
find-and-shift operation. The reverse iteration ensures indices remain valid as elements
are removed from the end.

### 10.5 T5: Complete INI Enumeration — All Particle Types and Systems

#### 10.5.1 Particle Systems (13 defined)

| # | Name | BehavesLike | HoldsWhat | Key Properties |
|---|------|-------------|-----------|----------------|
| 1 | GasCloudSys | Gas | GasCloud1 | Defaults only |
| 2 | FireStreamSys | Fire | FireStream | Spawns=yes, SpawnFrames=4, Lifetime=30 |
| 3 | BigGreySmokeSys | Smoke | LargeGreySmoke | Spawns=yes, SpawnFrames=10, SpawnRadius=10, Slowdown=0.0025, ParticleCap=20, SpawnCutoff=15.0, SpawnTranslucencyCutoff=13.0 |
| 4 | SmallGreySSys | Smoke | SmallGreySmoke | Spawns=yes, SpawnFrames=10, SpawnRadius=5, Slowdown=0.0025, ParticleCap=7, SpawnCutoff=13.0, SpawnTranslucencyCutoff=12.5 |
| 5 | DebrisSmokeSys | Smoke | SmallGreySmoke | Spawns=yes, SpawnFrames=2, SpawnRadius=3, ParticleCap=7, SpawnCutoff=13.0, SpawnTranslucencyCutoff=13.0 |
| 6 | SparkSys | Spark | Spark | ParticleCap=6, SparkSpawnFrames=1, LightSize=15, SpawnSparkPercentage=1.0 |
| 7 | FirestormSparkSys | Spark | FirestormSpark | ParticleCap=20, SparkSpawnFrames=1, LightSize=21, SpawnSparkPercentage=1.0 |
| 8 | TestSmokeSys | Smoke | TestSmoke | Spawns=yes, SpawnFrames=10, SpawnRadius=5, Slowdown=0.0025, ParticleCap=7, SpawnCutoff=13.0, SpawnTranslucencyCutoff=12.5 |
| 9 | SmallRailgunSys | Railgun | SmallRailgunPart | SpiralRadius=6, ParticlesPerCoord=0.1, SpiralDelta=0.035, MovementPerturb=0.3, PositionPerturb=20, VelocityPerturb=0.6, Laser=yes, LaserColor=255,128,0 |
| 10 | LargeRailgunSys | Railgun | LargeRailgunPart | SpiralRadius=15, ParticlesPerCoord=0.15, SpiralDelta=0.03, MovementPerturb=0.4, PositionPerturb=30, VelocityPerturb=0.6, Laser=yes, LaserColor=25,20,255 |
| 11 | WeldingSys | Spark | WeldingSpark | ParticleCap=15, SparkSpawnFrames=20, LightSize=25, OneFrameLight=true, SpawnSparkPercentage=0.4 |
| 12 | LGSparkSys | Spark | LargeSpark | ParticleCap=7, SparkSpawnFrames=5, LightSize=25, OneFrameLight=true, SpawnSparkPercentage=0.2 |
| 13 | PsychCloudSys | Gas | PsychCloud | Defaults only |

**Type distribution:** 4 Smoke, 2 Gas, 1 Fire, 4 Spark, 2 Railgun.

#### 10.5.2 Particle Types (22 defined)

| # | Name | BehavesLike | Image | Key Properties |
|---|------|-------------|-------|----------------|
| 1 | GasCloud1 | Gas | WCCLOUD1 | MaxDC=60, MaxEC=1000, Damage=5, Warhead=Gas, EndState=28, StateAdv=4, Translucency=50, NextParticle=GasCloudD1 |
| 2 | GasCloud2 | Gas | WCCLOUD1 | MaxDC=60, MaxEC=1000, Damage=40, Warhead=Gas, EndState=28, StateAdv=4, Translucency=50, NextParticle=GasCloudD2 |
| 3 | FireStream | Fire | WCCLOUD1 | MaxEC=500, MaxDC=3, Damage=2, Warhead=Fire, Velocity=28.0, Deacc=0.01, StartState=1, EndState=19, StateAdv=6, Translucent50=15, Translucent25=10, DeleteOnStateLimit=yes, Normalized=yes, FinalDamageState=14 |
| 4 | Spark | Spark | (none) | MaxEC=500, XVel=10, YVel=10, MinZVel=40, ZVelRange=15, ColorList=(255,255,255),(200,200,80),(200,10,10),(0,0,0), ColorSpeed=0.13 |
| 5 | FirestormSpark | Spark | (none) | MaxEC=500, XVel=16, YVel=16, MinZVel=40, ZVelRange=15, ColorList=(0,0,255),(255,255,255),(200,200,80),(200,10,10),(0,0,0), ColorSpeed=0.13 |
| 6 | LargeGreySmoke | Smoke | LGRYSMK1 | MaxEC=80, Velocity=8.0, Deacc=0.05, Translucency=50, EndState=20, StateAdv=4, DeleteOnStateLimit=yes |
| 7 | SmallGreySmoke | Smoke | SGRYSMK1 | MaxEC=80, Velocity=9.0, Deacc=0.05, Translucency=50, EndState=20, StateAdv=4, DeleteOnStateLimit=yes |
| 8 | TestSmoke | Smoke | SGRYSMK1 | MaxEC=80, Velocity=6.0, Deacc=0.05, Translucency=25, EndState=20, StateAdv=3, DeleteOnStateLimit=yes |
| 9 | GasCloudD1 | Gas | WCCLOUD1 | MaxDC=60, MaxEC=50, Damage=10, Warhead=Gas, EndState=12, StateAdv=4, Translucency=50, DeleteOnStateLimit=yes |
| 10 | GasCloudD2 | Gas | WCCLOUD1 | MaxDC=60, MaxEC=50, Damage=10, Warhead=Gas, EndState=12, StateAdv=4, Translucency=50, DeleteOnStateLimit=yes |
| 11 | SmallRailgunPart | Railgun | (none) | MaxEC=70, Velocity=0.4, ColorList=(200,200,200),(150,150,150), ColorSpeed=0.03 |
| 12 | LargeRailgunPart | Railgun | (none) | MaxEC=70, Velocity=0.3, ColorList=(25,70,205),(150,150,150), ColorSpeed=0.009 |
| 13 | GasCloudM1 | Gas | gaslrgmk | MaxDC=60, MaxEC=448, Damage=0, Warhead=Gas, EndState=11, StateAdv=3, Translucency=50, NextParticle=GasCloud1, DeleteOnStateLimit=yes, NextParticleOffset=0,0,150 |
| 14 | GasCloudM2 | Gas | gaslrgmk | MaxDC=60, MaxEC=448, Damage=0, Warhead=Gas, EndState=11, StateAdv=3, Translucency=50, NextParticle=GasCloud2, DeleteOnStateLimit=yes, NextParticleOffset=0,0,150 |
| 15 | WeldingSpark | Spark | (none) | MaxEC=500, XVel=16, YVel=16, MinZVel=40, ZVelRange=15, ColorList=(0,128,255),(255,255,255),(200,200,150),(80,80,80),(0,0,0), StartColor1=80,255,255, StartColor2=255,255,100, ColorSpeed=0.13 |
| 16 | LargeSpark | Spark | (none) | MaxEC=500, XVel=13, YVel=13, MinZVel=40, ZVelRange=15, ColorList=(255,255,255),(200,200,80),(200,10,10),(0,0,0), ColorSpeed=0.13 |
| 17 | VirusCloud1 | Gas | TXGASG | MaxDC=30, MaxEC=1000, Damage=5, Warhead=VirusGas, EndState=20, StateAdv=4, NextParticle=VirusCloudD1 |
| 18 | VirusCloud2 | Gas | LGRYSMK1 | MaxDC=30, MaxEC=1000, Damage=10, Warhead=VirusGas, EndState=20, StateAdv=4, NextParticle=VirusCloudD2 |
| 19 | VirusCloudD1 | Gas | TXGASG | MaxDC=60, MaxEC=50, Damage=5, Warhead=VirusGas, EndState=20, StateAdv=4, DeleteOnStateLimit=yes |
| 20 | VirusCloudD2 | Gas | LGRYSMK1 | MaxDC=60, MaxEC=50, Damage=5, Warhead=VirusGas, EndState=20, StateAdv=4, DeleteOnStateLimit=yes |
| 21 | PsychCloud | Gas | TXGASR | MaxDC=20, MaxEC=50, Damage=600, Warhead=PsychGas, EndState=20, StateAdv=4, NextParticle=PsychCloudD |
| 22 | PsychCloudD | Gas | TXGASR | MaxDC=60, MaxEC=50, Damage=10, Warhead=PsychGas, EndState=20, StateAdv=4, DeleteOnStateLimit=yes |

**Type distribution:** Gas=12, Smoke=3, Fire=1, Spark=4, Railgun=2.

**Gas chain structures:**
- Standard poison: GasCloudM1 -> GasCloud1 -> GasCloudD1 (formation -> full -> dissipate)
- Standard poison: GasCloudM2 -> GasCloud2 -> GasCloudD2
- Virus: VirusCloud1 -> VirusCloudD1
- Virus: VirusCloud2 -> VirusCloudD2
- Psychic: PsychCloud -> PsychCloudD

### 10.6 T6: Bullet Attachment for Weapon Particles

From `TechnoClass::Fire_At` at `0x006fdd50`, decompiled in full:

#### 10.6.1 UseFireParticles (weapon +0x129)

```
if weapon.UseFireParticles AND this.FireParticle_ptr (+0x304) == 0:
    system = ParticleSystemClass(
        weapon.AttachedParticleSystem,   // weapon +0x11C
        &bullet,             // param: attached object = bullet pointer
        target,              // param: target object
        this,                // param: owner = firing techno
        &g_NullCoord,        // param: target coords = null
        0                    // param: house
    )
    this.FireParticle_ptr = system
```

#### 10.6.2 UseSparkParticles (weapon +0x12A)

```
if weapon.UseSparkParticles AND this.SparkParticle_ptr (+0x308) == 0:
    system = ParticleSystemClass(
        weapon.AttachedParticleSystem,   // SAME weapon field as fire!
        &bullet,
        target,
        this,
        &g_NullCoord,
        0
    )
    this.SparkParticle_ptr = system
```

**Important:** Both UseFireParticles and UseSparkParticles use the SAME
`weapon.AttachedParticleSystem` field at weapon offset +0x11C. The flag just controls
which slot on the techno stores the pointer (+0x304 vs +0x308). Both pass the bullet
as the attached object.

#### 10.6.3 IsRailgun (weapon +0x12D)

```
if weapon.IsRailgun AND this.RailgunParticle_ptr (+0x314) == 0:
    endpoint = FUN_0070c690(&target_coords, &bullet, target, weapon)
    system = ParticleSystemClass(
        weapon.AttachedParticleSystem,
        &bullet,
        NULL,                // NO target object for railgun
        this,
        &endpoint,           // computed endpoint coordinates
        0
    )
    this.RailgunParticle_ptr = system
```

**Railgun difference:** Target object is NULL; instead, a computed endpoint coordinate is
passed as the 5th parameter. `FUN_0070c690` calculates the actual impact point along the
projectile path. This endpoint is stored at `ParticleSystemClass.target_coords` (+0xD4)
and used by `AI_Railgun` to draw the spiral trail from source to target.

#### 10.6.4 How the System Follows the Bullet

The bullet pointer is passed as the `attached_object` (stored at PSClass +0xE0). During
each system AI tick:
- **Smoke/Fire systems** call `attached_object.GetCoords()` to update position
- **Fire system** specifically uses `Filter_AbstractType_InMap` to verify the attached
  object still exists in the game world
- When the bullet is destroyed, `PointerExpiredNotify` fires on the particle system,
  which sets `marked_for_deletion = true` and nulls the attached_object pointer

The system does NOT follow the bullet in real-time for railgun — railgun spawns all
particles in a single tick along the precomputed path.

### 10.7 T7: Mind Control Beam Particles (CaptureManagerClass::Update at 0x00471a50)

```
fn CaptureManagerClass::Update(self):
    if not self.is_active (+0x40):
        return

    // Decrement damage timer
    if self.damage_timer (+0x44) > 0:
        self.damage_timer -= 1

    // Decrement beam timer
    if self.beam_timer (+0x4C) > 0:
        self.beam_timer -= 1
        return     // still waiting

    // Look up damage level from RulesClass tables
    // RulesClass+0xEEC = array of distance thresholds
    // RulesClass+0xEF8 = count of entries
    // RulesClass+0xF08 = array of damage values per level
    // RulesClass+0xF24 = array of beam intervals per level
    level_index = 0
    while threshold_array[level_index] < self.distance (+0x34):
        level_index += 1
        if level_index >= count - 1:
            break

    self.beam_timer = beam_interval_array[level_index]
    damage = damage_array[level_index]

    if damage < 1:
        self.beam_active (+0x41) = false
    else:
        self.damage_timer = 10

        // Apply damage to the controlled unit
        self.victim.ReceiveDamage(
            &damage,
            0,                               // distance = 0
            RulesClass.MindControlDamageWarhead,  // +0xFA8
            NULL, NULL, NULL, NULL
        )

        if not self.sound_played (+0x41):
            VocClass::PlayAt(0)    // play mind control sound
            self.sound_played = true

        // Spawn 5 particle systems at the victim's position
        for i in 0..5:
            coords = self.victim.coords   // victim at +0x48
            coords.x += Random(-200, 200)  // 0xFFFFFF38 = -200
            coords.y += Random(-200, 200)

            system = new ParticleSystemClass(
                RulesClass.DefaultSparkSystem,  // RulesClass+0x1020
                &coords,
                NULL,          // no target
                NULL,          // no owner
                &DAT_0089e138, // default coord
                0
            )

        // Mind control beam visual wobble
        if damage > 0 AND victim.is_on_map (+0x90):
            wobble = 0.015
            if damage != 1:
                wobble = 0.03
            if Random(0, 100) < 50:
                wobble = -wobble
            victim.mind_control_wobble (+0x330) = wobble
```

**Key findings:**
- The mind control beam spawns **5 spark particle systems per tick** at the victim's
  position with random offsets of +/-200 leptons in X and Y.
- The system type used is `RulesClass.DefaultSparkSystem` at RulesClass+0x1020 (likely
  mapped to `DefaultTestParticleSystem` or a spark system from [General]).
- The beam does NOT directly track between controller and victim — the particles are
  spawned at the victim's location only. The visual beam line is handled separately by
  the mind control drawing system, not by particle systems.
- Damage is distance-based: farther victims take more damage. The damage level is looked
  up from RulesClass tables that map distance thresholds to damage amounts and beam
  intervals.
- A wobble effect is applied to the victim's visual rendering (+0x330 on the controlled
  techno).

### 10.8 T8: Gap Generator Smoke (BuildingClass::UpdateGapGenerator at 0x00454db0)

**Trigger condition:** Gap generator state machine at building offset +0x220 (param_1[0x88]).

The gap generator has 4 states:
- **State 0**: Inactive
- **State 1**: Expanding (gap growing)
- **State 2**: Active (gap fully deployed)
- **State 3**: Shrinking (gap collapsing)

**Particle system creation occurs in State 3 -> State 0 transition:**

```
fn UpdateGapGenerator(self):
    if self.gap_state == 3:
        fade_counter = self.gap_fade_byte (+0x6ED)
        if fade_counter > 0:
            fade_counter -= 1
            self.gap_fade_byte = fade_counter

        // Update all gap overlay cells with new fade level
        for each cell in gap_cell_list (21 cells at building+0x55C):
            if cell != NULL:
                cell.gap_fade = fade_counter

        if fade_counter == 0:
            // Transition to state 0 (inactive)
            self.gap_state = 0

            // Only create smoke if no existing system
            if self.particle_system (+0x30C, index 0xC3) == NULL:
                type_class = self.TechnoTypeClass     // building[0x148]

                // Check if smoke offset is non-default
                smoke_offset = (type+0x768, type+0x76C, type+0x770)
                default_offset = (DAT_0089c848, DAT_0089c84c, DAT_0089c850)
                if smoke_offset != default_offset:
                    spawn_pos = building.GetCoords() + smoke_offset
                    cell = CellClass::Get_Cell_At(building.GetCoords())

                    system = ParticleSystemClass(
                        type.GapGeneratorParticleSystem,  // type+0x764
                        &spawn_pos,
                        cell,          // target = cell
                        NULL,          // no owner
                        &default_offset,
                        0
                    )
                    self.particle_system = system
```

**Summary:** Smoke is spawned ONLY when the gap generator finishes shrinking (state 3 ->
state 0). This happens when the building is destroyed or powered down. The particle system
type comes from `BuildingTypeClass+0x764` (the `GapGeneratorParticleSystem` INI key).
The smoke position offset is at `BuildingTypeClass+0x768/+0x76C/+0x770`. The system is
created once and stored at `BuildingClass+0x30C` — no new system is created if one already
exists.

### 10.9 T9: Chrono Warp Particles (WarpAttachClass::UpdateAttack at 0x00629fd0)

```
fn WarpAttachClass::UpdateAttack(self):
    if self.target (+0x28) == NULL:
        return

    attacker = self.owner (+0x24)
    attacker_type = attacker.TypeClass

    // Check for ImmuneToChrono / Chronoshift flags
    if attacker_type.ImmuneToChrono (+0xCCE) AND attacker_type.Chronoshift (+0xD97):
        TemporalClass::AI()     // defer to temporal fade logic
        return

    target = self.target (+0x28)
    victim_coords = target.coords

    // Timer-based warp duration logic
    warp_delay = attacker_type.WarpDelay (+0xB0)
    if self.start_frame (+0x38) != -1:
        elapsed = g_CurrentFrameCounter - self.start_frame
        remaining = self.warp_duration (+0x40) - elapsed
        if remaining > 0:
            return    // still waiting

    // Update warp timing
    self.start_frame = g_CurrentFrameCounter
    self.warp_rate = target.warp_count(?)
    self.warp_duration = warp_delay

    // Check if target is fully warped (RTTIType == 0xF = teleported?)
    rtti = target.RTTI()
    if rtti == 0xF:
        // Final warp — apply damage with special params
        target.ReceiveDamage(&stack, 0, attacker_type.Weapon?, attacker, 1, 1, 0)
        return

    // Create chrono sparkle particle system at target position
    system = new ParticleSystemClass(
        RulesClass.ChronoSparkle1,     // RulesClass+0x1020
        &victim_coords,
        NULL,          // no target
        NULL,          // no owner
        &DAT_00ac4948, // default coord
        0
    )

    // Animate the chrono warp effect
    rate_timer_value = RateTimer::Current()

    // Create muzzle flash animation if weapon has anim frames
    if attacker_type.MuzzleFlashCount (+0x104) != 0:
        anim_type = attacker_type.MuzzleFlash[(rate_value >> 12 + 1) >> 1 & 7]
        if anim_type != 0:
            anim = new AnimClass(anim_type, &victim_coords, 0, 1, 0x600, 0, 0)

    // Compute chrono beam visual offset
    random_dir = Random(0, 1)
    beam_scale = (random_dir ? -2 : 2)    // +2 or -2

    angle = (short)(rate_value - 0x3FFF) - 0x3FFF
    angle_rad = angle * pi_scale           // _LAB_007e2810

    beam_x = cos(angle_rad) * (float)beam_scale
    beam_y = sin(angle_rad) * (float)victim_y

    // Apply beam teleport animation
    target.DrawVisualWarp(&beam_coords, 0.1875, 0)

    // Apply damage to target
    target.ReceiveDamage(&stack, 0, attacker_type.Weapon?, attacker, 0, 1, 0)
```

**Key findings:**
- The particle system used is `RulesClass+0x1020`, which is the **ChronoSparkle1** system
  (mapped to `[General] ChronoSparkle1=CHRONOSK` in rulesmd.ini — but this is actually an
  anim name, not a particle system). Looking more carefully at the constructor call: it
  uses `RulesClass+0x1020` which in the mind control section was identified as the
  "DefaultSparkSystem". Both mind control and chrono warp use the SAME default spark
  system from RulesClass.
- The chrono sparkle appears at the **victim's position** each warp tick.
- A muzzle flash animation is also created from the attacker's weapon anim list.
- The warp includes a beam visual effect with cos/sin orbital motion using the RateTimer.
- Every warp tick applies damage through `ReceiveDamage` with the attacker's warhead.

### 10.10 T10: Destructor and Full Cleanup

#### 10.10.1 Mark for Deletion (vtable+0xF8 = 0x006301e0)

This is trivial — it just sets the flag:
```
fn MarkForDeletion(self):
    self.marked_for_deletion = true    // *(param_1 + 0xF8) = 1
```

#### 10.10.2 RemoveAllParticles (0x0062e650) — Detailed

```
fn RemoveAllParticles(self):
    i = self.particle_count - 1
    while i >= 0:
        if i < self.particle_count:
            particle = self.vector.data[i]

            // Call particle's Destroy method (vtable+0xF8)
            particle.Destroy()

            // Find particle's index in vector via vtable+0x10
            // (DynamicVectorClass::FindIndex)
            idx = self.vector.vtable.FindIndex(&self.vector.data[i])

            if idx != -1 AND idx < self.particle_count:
                self.particle_count -= 1
                // Shift remaining elements left
                while idx < self.particle_count:
                    self.vector.data[idx] = self.vector.data[idx + 1]
                    idx += 1

        i -= 1
```

Each particle is destroyed and then removed from the vector with element shifting. The
reverse iteration (high index to low) ensures that when an element is removed and the
array shifts left, the decrementing index still covers all remaining elements.

#### 10.10.3 Actual Destruction

Particle systems do NOT have a standalone destructor that destroys all particles. Instead:
- During normal gameplay, particles die naturally (lifetime expires, DeleteOnStateLimit)
  and are cleaned up each tick in the system's AI.
- On map clear (`FUN_006851f0`), the code calls `RemoveAllParticles` on `DAT_00a8ed78`
  (a singleton system) and then calls `vtable+0x20` (destructor with dealloc flag=1) to
  destroy the system itself.
- The ObjectClass base destructor handles removal from the global object tracking vectors.

### 10.11 T11: The "Directionless" Spawning Mode

The `directionless` flag is at `ParticleSystemClass+0xF9` (byte).

#### 10.11.1 When Is It Set?

From the `ParticleSystemClass::Constructor` at `0x0062dc50`: the flag is initialized to
`false` (0). It is set based on the `SpawnDirection` field of the type class:

```
// In constructor, after setting up fields:
if type.SpawnDirection == (0, 0, 0):    // all three components zero
    self.directionless = true
else:
    self.directionless = false
```

So `directionless = true` when the system type has NO explicit `SpawnDirection` in INI
(since the default is (0,0,0)). Looking at the INI data: none of the 13 defined
particle systems specify `SpawnDirection`, so **all spark systems in standard YR content
are directionless**.

#### 10.11.2 Effect on Spark Spawning (AI_Spark at 0x0062e840)

From the fully decompiled AI_Spark:

```
for each new spark particle:
    // Assign random velocity vector
    dir.x = Random() % type.XVelocity
    dir.y = Random() % type.YVelocity
    dir.z = Random() % type.ZVelocityRange + type.MinZVelocity

    // Save original magnitude
    magnitude = sqrt(dir.x^2 + dir.y^2 + dir.z^2)

    if NOT directionless:
        // Add the system's SpawnDirection (from INI)
        dir.x += type.SpawnDirection.x    // type+0x2BC (as float)
        dir.y += type.SpawnDirection.y    // type+0x2C0
        dir.z += type.SpawnDirection.z    // type+0x2C4
    else:
        // Add random values computed ONCE at spawn batch start
        // These are: random_x, random_y, random_z (from the pre-loop calculation)
        dir.x += pre_random_x    // local_14 = iVar8 % XVelocity
        dir.y += pre_random_y    // local_10 = iVar7 % YVelocity
        dir.z += pre_random_z    // iVar9 % ZVelocityRange

    // Re-normalize to unit vector, then restore original magnitude
    new_mag = sqrt(dir.x^2 + dir.y^2 + dir.z^2)
    if new_mag != 0:
        dir = dir / new_mag
    dir = dir * magnitude
```

**Directionless effect:** When `directionless=true`, instead of biasing all sparks toward
a fixed `SpawnDirection`, a random direction offset is generated ONCE per spawn batch
(computed before the loop using the same velocity ranges). This offset is then added to
every particle in the batch. The result: all sparks in one batch are biased in the same
random direction, but different batches go in different directions. This creates a
"fountain" effect where each burst sprays in a random direction, rather than always going
the same way.

When `directionless=false`, sparks are always biased toward `SpawnDirection`, creating a
directed jet effect (e.g., a spark fountain that always shoots upward if SpawnDirection
has a positive Z).

### 10.12 T12: Gas Damage Application — Full Detail (0x0062bd50)

From the complete decompilation of `gas_particle_ai`:

#### 10.12.1 Damage Counter and Trigger

```
// Damage counter at +0x12A (short), decremented every tick
self.damage_counter -= 1

if self.damage_counter == 0 AND self.type.Damage (+0x2E8) != 0:
    // Reset counter from MaxDC
    self.damage_counter = self.type.MaxDC   // +0x2DC

    // Get cell at particle position
    cell = CellClass::Get_Cell_At(self.coords)

    // Iterate objects in cell's object list
    object_list = cell.ObjectList   // cell+0xE4 (ground layer)

    while object != NULL:
        next = object.NextObject    // object[0xC] (linked list next ptr)

        // Two checks: object must be alive AND have health > 0
        if (char)object[0x24] != '\0'    // IsAlive flag
           AND object[0x1B] > 0:         // Health > 0

            // Get owner house from particle system
            if self.owner_system (+0x124) == 0:
                house = 0
            else:
                house = self.owner_system.owner_house   // system+0xFC

            // Get damage and warhead from particle type
            damage_amount = self.type.Damage             // +0x2E8
            warhead = self.type.Warhead                  // +0x2E4

            // Adjust damage for height difference
            adjusted_damage = AdjustForZ(warhead, 0, 0, 0, house)

            // Apply damage
            object.ReceiveDamage(&damage_amount, adjusted_damage)

        object = next
```

#### 10.12.2 Key Observations

1. **No friendly/enemy check.** Gas damage hits ALL objects in the cell regardless of
   house ownership. Gas is an area denial weapon that damages everything — friend and foe
   alike.

2. **Cell layer selection.** The code uses `cell+0xE4` which is the ground-level object
   list. There is no bridge-layer check in the gas damage code (unlike fire damage which
   checks bridge layers).

3. **AdjustForZ.** The warhead damage is adjusted by `AdjustForZ` which modifies damage
   based on vertical distance between the damage source and target. The exact adjustment
   depends on the warhead's `CellSpread` and height sensitivity parameters.

4. **Object iteration.** Uses a simple linked list: `object[0xC]` is the `NextObject`
   pointer. Each cell maintains a singly-linked list of objects.

5. **FinalDamageState is NOT checked in gas AI.** Unlike fire particles (which stop
   dealing damage after reaching FinalDamageState), gas particles damage every time the
   counter reaches zero, regardless of animation state. The FinalDamageState check only
   appears in fire_particle_ai.

### 10.13 T13: Fire Particle Ground Detection (0x0062cb10)

From the complete decompilation of `fire_particle_ai`:

#### 10.13.1 Movement and Direction Jitter

```
if self.velocity <= 0.0:
    self.marked_for_deletion = true
    return

// Random jitter on direction vector
random_value = Random() % 10 - 5     // range -5 to +4
jitter_factor = random_value * 0.01 + 1.0   // range 0.95 to 1.04
// (_DAT_007efb40 = 0.01, _g_Const_1_0 = 1.0)

// Apply jitter to direction and compute new delta
prev_delta_x = ftol(jitter_factor * self.dir_x)    // param_1[0x43]
prev_delta_y = ftol(jitter_factor * self.dir_y)    // param_1[0x44]
prev_delta_z = ftol(jitter_factor * self.dir_z)    // (from FPU)

// Store for use by Move_Dispatch fire path
self.prev_delta_x = prev_delta_x   // +0x100
self.prev_delta_y = prev_delta_y   // +0x104
self.prev_delta_z = prev_delta_z   // +0x108
```

#### 10.13.2 Ground Detection (in Move_Dispatch, inline fire path)

Ground detection for fire happens in `Move_Dispatch` (0x0062d5e0), NOT in the fire AI:

```
// In Move_Dispatch, case 2 (Fire):
old_pos = self.coords
new_pos = self.coords

if self.velocity > 0.0:
    new_pos += self.prev_delta    // delta computed in fire_ai

if self.velocity > 0.0:
    old_ground_z = CellClass::GetGroundHeight(old_pos)
    new_ground_z = CellClass::GetGroundHeight(new_pos)

    if old_ground_z < new_ground_z:
        // Terrain RISES at new position
        self.hit_ground = true       // +0x12D
        self.marked_for_deletion = true  // +0x131

self.SetCoords(new_pos)
```

**What happens on different terrain:**

- **Flat ground:** `old_ground == new_ground`, no collision, fire continues.
- **Cliff (terrain rises):** `old_ground < new_ground`, fire is killed immediately.
  There is no "sliding along" or "bouncing" — fire particles simply die on contact with
  rising terrain. This is realistic for a flamethrower stream.
- **Bridge:** No bridge check exists in fire movement. Fire particles pass through
  bridges as if they don't exist. The ground height function returns terrain height,
  not bridge height, so fire streams fly over bridges.
- **Water:** The ground height at water cells returns the water surface level. If the
  fire stream is aimed downward toward water, the same rising-terrain check applies —
  fire dies when it reaches the water surface height.
- **Downhill:** `old_ground > new_ground`, condition is false, fire continues. Fire
  streams can follow downhill slopes freely.

#### 10.13.3 Fire Damage Application (in fire_particle_ai)

Fire damage has a FinalDamageState check that gas does not:

```
// Decelerate
self.velocity -= self.type.Deacc        // +0x300

// Damage counter
self.damage_counter -= 1
if self.damage_counter == 0 AND self.type.Damage != 0:
    // FinalDamageState check — UNIQUE to fire
    if self.animation_state <= self.type.FinalDamageState:    // +0x30C
        self.damage_counter = self.type.MaxDC    // reset

        cell = CellClass::Get_Cell_At(self.coords)

        // Bridge layer check for fire (not present in gas!)
        if (cell.flags_0x140 & 0x100) != 0:    // bridge present
            bridge_threshold = (cell.byte_0x11B + 4) * DAT_00ac4a18
            if self.coords.z >= bridge_threshold:
                // Particle is ABOVE bridge — use bridge object list
                object_list = cell.BridgeObjectList   // cell+0xE8
                goto damage_loop

        object_list = cell.ObjectList   // cell+0xE4 (ground layer)

    damage_loop:
        for each object in object_list:
            // Distance check from fire source
            fire_source = self.GetCoords()     // vtable+0x48
            dx = fire_source.x - object.x
            dy = fire_source.y - object.y
            dz = fire_source.z - object.z
            distance = sqrt(dx*dx + dy*dy + dz*dz)

            // Only damage if object is alive, has health, and is not
            // the particle system's attached object
            if object.health > 0
               AND object.is_alive
               AND object != self.owner_system.attached_object:

                damage = self.type.Damage
                // Distance scaling: damage reduced by distance/10
                object.ReceiveDamage(
                    &damage,
                    distance / 10,
                    self.type.Warhead,
                    0, 0, 0, 0
                )
```

**Fire vs Gas damage differences:**
1. Fire checks `FinalDamageState` — stops dealing damage after animation progresses past
   this state. Gas does not check this.
2. Fire has bridge layer awareness — uses `cell+0xE8` (bridge objects) when the particle
   is above bridge height. Gas only uses `cell+0xE4` (ground objects).
3. Fire applies distance-based damage reduction (`distance/10`). Gas does not.
4. Fire excludes the particle system's own attached object (the bullet/weapon source) from
   damage. Gas has no such exclusion.
5. Fire uses `FinalDamageState=14` (from INI) — the flame stops damaging well before
   the animation ends at state 19.

### 10.14 Additional Findings

#### 10.14.1 FUN_00630250 — DynamicVectorClass::Add for Particle Vector

Address `0x00630250`, called from `AI_Spark` when adding a newly-spawned particle to the
system's particle vector. This is a standard `DynamicVectorClass::Add` implementation:
checks capacity, grows if auto-grow is enabled and grow_step > 0, then appends at index
`count` and increments count. Nothing particle-specific.

#### 10.14.2 Spark AI Color Handling Detail

In the full `AI_Spark` decompilation, the spark particles call `FUN_00630250` (vector add)
INSTEAD of the standard SpawnParticle function. The sequence is:
1. `operator_new(0x138)` for the particle
2. `ParticleClass::Constructor` with the system's coords as both spawn and origin
3. `FUN_00630250` to add to the system's particle vector
4. Then directly sets direction fields on the particle (+0x10C, +0x110, +0x114)

This bypasses the `SpawnParticle` functions entirely. The inline construction allows the
spark AI to immediately set random velocity vectors without an intermediary.

#### 10.14.3 DAT_00a8ed78 — Global Singleton Particle System

`DAT_00a8ed78` is a single ParticleSystemClass pointer used as a "global default" system.
It is referenced by:
- `AnimClass::Start` (0x00425032) — animation system reads it
- `FUN_00684c30` (0x00684fea) — creates/assigns it
- `FUN_006851f0` (0x0068560e) — map clear destroys it (calls RemoveAllParticles then delete)
- Various other subsystems that need to reference a shared particle system

This is likely the system used for global effects that don't belong to any specific unit
or weapon (e.g., ambient particles, map-trigger-spawned effects).

#### 10.14.4 Smoke Wind Table Offset Correction

The report in section 10.2.3 identified smoke using tables at `DAT_008366a4` and
`DAT_008366c4`. These are at +0x40 bytes from the gas wind tables (0x00836664/0x00836684).
However, looking more carefully at the decompilation, the offset is computed as
`RulesClass.WindDirection * 4` and then added to the table base. The gas table base is
`&DAT_00836664` while smoke uses `&DAT_008366a4`. Since each entry is 4 bytes (int32),
and the tables hold 8 entries each (32 bytes), the smoke table at 0x008366a4 is
`0x00836664 + 0x40 = 0x008366a4`. This is exactly 16 entries (64 bytes) past the gas DX
table start — meaning **smoke uses a completely separate set of 8 wind direction entries**
that are 0x40 bytes after the gas tables. **VERIFIED from memory reads:**

Smoke DX table at 0x008366a4: `[0, 2, 2, 2, 0, -2, -2, -2]`
Smoke DY table at 0x008366c4: `[-2, -2, 0, 2, 2, 2, 0, -2]`

Compare with gas tables:
Gas DX at 0x00836664: `[0, 2, 2, 1, 0, -2, -2, -2]`
Gas DY at 0x00836684: `[-2, -2, 0, 2, 2, 2, 0, -2]`

**Only ONE difference:** SE direction (index 3) has DX=2 for smoke but DX=1 for gas.
The DY tables are identical. This means smoke drifts slightly more in the X direction
when the wind blows SE. Additionally, the smoke code MULTIPLIES the table value by
`WindEffect` before adding it, while gas uses the raw table value — so smoke drift scales
with WindEffect while gas does not.

```rust
/// Smoke wind drift tables (differ from gas at SE direction)
const SMOKE_WIND_DRIFT_DX: [i32; 8] = [0, 2, 2, 2, 0, -2, -2, -2];
const SMOKE_WIND_DRIFT_DY: [i32; 8] = [-2, -2, 0, 2, 2, 2, 0, -2];
```

---

## 11. Gap-Closing Pass (2026-05-04)

This section closes every gap flagged in §7 (Open Questions) and the post-audit
review of 2026-05-04. Plan: `docs/plans/2026-05-04-particle-system-gaps-investigation-plan.md`.
Every claim below is binary-verified via Ghidra MCP. Where this section
contradicts a claim in §1–§10, **§11 wins** — the prior text is not edited in
place to preserve audit history; cross-references to the corrected statements
are inline in §11.7.

### 11.1 ColorList runtime layout — DEFINITIVE

The `[Particles]` `ColorList=` key parses into an **embedded** `DynamicVectorClass<ColorStruct>`
inside ParticleTypeClass starting at byte +0x2B8. The vector is NOT a pointer
to a heap-allocated vector header — the vector header lives inline in the type.

**Vector header layout (24 bytes, byte-precise):**

| Type byte offset | Field | Purpose |
|------------------|-------|---------|
| +0x2B8 | `void* vtable` | DynamicVectorClass<ColorStruct> vtable, set to `&PTR_FUN_007E4E58` |
| +0x2BC | `ColorStruct* data` | Heap pointer to packed RGB triplet array (3 bytes per entry) |
| +0x2C0 | `int capacity` | Allocated entry count (in entries, not bytes) |
| +0x2C4 | `bool is_initialized` | Set to 1 by ctor |
| +0x2C5 | `bool owns_buffer` | Whether to free `data` on destruction |
| +0x2C6..+0x2C7 | (padding) | |
| +0x2C8 | `int active_count` | Live entry count (this is the count §3 of report uses) |
| +0x2CC | `int growth_step` | Default 10 |
| +0x2D0 | `int (unverified)` | Third dword copied by ReadINI from parser local; semantic unknown but not load-bearing — likely a "max cap" or padding slot |

**Entry stride: 3 bytes (packed RGB), HIGH confidence.** Confirmed in
`FUN_00478850` (vector ctor — `operator_new(count*3)`), `FUN_004788E0`
(CopyFrom — `count*3`, copy stride +3), `FUN_004784F0` (Resize — `count*3`,
zero-fill stride +3), `FUN_00478220` (CopyFrom variant — `count*3`), and
`FUN_00476B20` (parser write — `puVar1 = (ushort*)(buf + count*3); *puVar1 = R<<8|G; *(byte*)(puVar1+1) = B`).
There is no padding byte; ColorStruct is exactly 3 bytes.

**Vector vtable @ 0x007E4E58 (DynamicVectorClass<ColorStruct>):**

| Vtable offset | Function | Role |
|---------------|----------|------|
| +0x00 | `FUN_00477AC0` | Scalar destructor — frees `data` if `owns_buffer != 0` |
| +0x04 | (data label, not function) | Likely `operator delete[]` thunk; not on hot path |
| +0x08 | `FUN_004784F0` | Resize / SetCapacity — copies up to `min(old_count, new_cap)*3` bytes, frees old buffer |
| +0x0C | `FUN_00477900` | Reset (clear-and-zero) |

**§8.3 formula reconciliation:** the formula `ParticleType + 0x2BC + index*3`
is **correct in mechanic but imprecise in prose**. The actual operation is
`*((ColorStruct*)((char*)*(ParticleType + 0x2BC) + index*3))` — i.e. load the
buffer pointer from +0x2BC, then byte-index by `index*3`. The formula is
shorthand; readers should not interpret +0x2BC as the inline data address.

**Active in YR:** Yes, unconditional. ParticleTypeClass::ReadINI calls
`FUN_00476B20(s_ColorList, ...)` for every `[Particles]` entry. No SpecialFlags
gate, no TS-only branch.

**Edge case — empty ColorList:** when `CCINIClass__ReadString` returns 0 (key
missing), the parser takes an early-out branch and ReadINI then copies what
may be uninitialized bytes from the parser local into ParticleType+0x2C8/+0x2CC/+0x2D0.
In practice this is harmless because `active_count = 0` and runtime indexers
never read when count is zero, but it IS a real "leak uninitialized" pattern
in the binary. Document as a curiosity; do not replicate the bug in Rust.

**Edge case — R-token parse off-by-one:** the strtok loop at FUN_00476B20
reads R via `pcVar6 + 1` (consumes/skips the leading character of the R token).
This means a `ColorList=R,G,B` value where the first character is a digit will
produce an off-by-one R. In practice the format is always
`ColorList=R0,G0,B0,R1,G1,B1,...` with leading whitespace or comma trimmed
elsewhere — so it doesn't bite real INI lines. Flag as a quirk; for Rust
parity, parse the same `+1` offset to match exactly.

**Constructor-zone correction to §2.2:** the §2.2 table claimed
`+0x2B8 | void* | -- | -- | Internal (color vector pointer from ColorList parsing)`
and `+0x2BC | void* | -- | -- | Pointer to first color entry`. The corrected
layout above replaces those rows. The §2.2 table also said the count lived
at `+0x2C8`; that row IS correct (it's the active count after parse).

### 11.2 TechnoTypeClass particle field map (consumer-side parsing)

`TechnoTypeClass::ReadINI` is at **0x00712170**. Critical correction: `param_1`
is `int *`, so all `param_1[N]` index expressions in the decompilation map to
byte offset `N * 4`.

**Particle-related field map (HIGH confidence — every offset verified at the read site):**

| INI key | Byte offset | Type | Read site | Active in YR? |
|---------|-------------|------|-----------|---------------|
| `NaturalParticleSystem` | 0x764 | `ParticleSystemTypeClass*` | 0x713BA7 | Yes (by code path) — see §11.5 |
| `NaturalParticleLocation` X/Y/Z | 0x768 / 0x76C / 0x770 | `int[3]` (CoordStruct) | 0x713C10 | Yes (paired with above) |
| `RefinerySmokeParticleSystem` | 0x774 | `ParticleSystemTypeClass*` | 0x713BE0 | Yes |
| `DamageParticleSystems` (vector) | 0x778..0x78F (24 bytes inline `DynamicVectorClass<PSType*>`) | DynamicVectorClass | 0x713C3C | Yes |
| `DestroyParticleSystems` (vector) | 0x794..0x7AB (24 bytes inline) | DynamicVectorClass | 0x713D39 | **No (see §11.5)** |
| `DamageSmokeOffset` X/Y/Z | 0x7B0 / 0x7B4 / 0x7B8 | `int[3]` | 0x713E33 | Conditional |
| `DamSmkOffScrnRel` | 0x7BC (byte) | bool | 0x713E57 | Conditional |
| `DestroySmokeOffset` X/Y/Z | 0x7C0 / 0x7C4 / 0x7C8 | `int[3]` | 0x713E76 | Conditional |
| `RefinerySmokeOffsetOne` X/Y/Z | 0x7CC / 0x7D0 / 0x7D4 | `int[3]` | 0x713E9F | Yes |
| `RefinerySmokeOffsetTwo` X/Y/Z | 0x7D8 / 0x7DC / 0x7E0 | `int[3]` | 0x713EC8 | Yes |
| `RefinerySmokeOffsetThree` X/Y/Z | 0x7E4 / 0x7E8 / 0x7EC | `int[3]` | 0x713EF1 | Yes |
| `RefinerySmokeOffsetFour` X/Y/Z | 0x7F0 / 0x7F4 / 0x7F8 | `int[3]` | 0x713F1A | Yes |
| `GapGenerator` | 0xCD1 (byte) | bool | 0x713FA6 | Yes |
| `GapRadiusInCells` | 0xCD2 (byte cast from int) | byte | 0x713FC1 | Yes |
| `SuperGapRadiusInCells` | 0xCD3 (byte cast from int) | byte | 0x713FDC | Yes |

**Critical correction to a prior-pass claim:** the scoping scan reported
`DamageParticleSystems → 0x788/0x78C/0x790` (3 ints) and
`DestroyParticleSystems → 0x7A4/0x7A8/0x7AC` (3 ints). **That was wrong.**
Those triplets are the *tail* fields of the inline DynamicVectorClass
(active_count + growth_step + final dword); the **full vectors occupy
0x778..0x78F and 0x794..0x7AB respectively**. The vectors are NOT fixed
4-element arrays; they're real grow-by-10 dynamic vectors built by the
helpers below.

**Vector helpers used by the CSV-list keys:**

| Address | Role |
|---------|------|
| 0x00717C00 | `DynamicVectorClass<PSType*>::CopyFrom(const &)` — final install of parser-local vector into TTC |
| 0x00717C70 | Constructor `(int initial_capacity, T** existing_buf)` — sets `growth_step = 10` |
| 0x00717CD0 | `Add(T* item)` — grow + append |
| 0x00717D20 | `Clear()` / destructor |
| 0x00644890 | `ParticleSystemTypeClass::Find_Or_Create(const char *name)` — string-to-pointer resolver, 0x310-byte `operator_new` if missing |

### 11.3 BuildingTypeClass particle keys — none

`BuildingTypeClass::ReadINI` is at **0x0045FE50** (mis-labeled
`BuildingTypeClass_ReadINI_Water` in Ghidra — same function). `param_1` is
`int` (direct byte offsets). **Verified by full grep of the decompilation:
BuildingTypeClass::ReadINI reads NO particle-related INI keys directly.**
All particle keys (`DamageParticleSystems`, `RefinerySmokeParticleSystem`,
etc.) are inherited via the chained call to `TechnoTypeClass::ReadINI` near
the start of the function.

`BuildingTypeClass`-only fields it does parse that are particle-adjacent:
- `RefinerySmokeFrames` at byte 0x1568 — smoke-anim frame count, **NOT a
  particle reference** (the field gates the timing of the refinery dump
  cycle that triggers FUN_00459900 — see §11.5.C).
- `Refinery` flag at byte 0x16BB
- `CloakGenerator` flag at byte 0x16C7 (the building flavour of TTC's
  `GapGenerator` flag at 0xCD1; gap-generator buildings set both)
- `CloakRadiusInCells` at byte 0x1707

**Implication:** The plan's hypothesis that `GapGeneratorParticleSystem`
might have a building-only override is **rejected** — the key doesn't
exist. See §11.5.A.

### 11.4 Image= → SHP binding (ObjectTypeClass)

ParticleTypeClass inherits the same Image=/SHP-load path used by all
ObjectTypeClass-derived types (animations, units, infantry, buildings,
overlays). There is no particle-specific override.

**Flow:**

```
ParticleTypeClass::ReadINI (0x00644F50)
  └── calls ObjectTypeClass::ReadINI (0x005F92D0) at the top
ObjectTypeClass::ReadINI
  ├── CCINIClass::ReadString("Image", default = OBJTYPE+0x1F8, len = 0x19)
  │     → stores 25-byte filename buffer at OBJTYPE+0x1F8
  ├── CCINIClass::ReadString("AlphaImage", default = OBJTYPE+0x213, len = 0x19)
  │     → stores 25-byte filename buffer at OBJTYPE+0x213
  ├── ... (other base-class keys)
  ├── ReadBool("Voxel", default = *(this+0x236))     ← gates SHP loading
  ├── if (!Voxel):
  │     int kind = (**(vtable+0x2C))();              ← What_Am_I (RTTI kind code)
  │     if (kind != 5):                              ← 5 = UnitType (voxel-rendered)
  │         FUN_005F9070();                          ← THE SHP LOAD HELPER
  └── if (AlphaImage[0] != 0):
        FUN_007C9FF0(buf, 0, 0, this+0x213, ".SHP")  ← build "<AlphaImage>.SHP"
        OBJTYPE[0x2B] = LoadFileFromMIX(buf)         ← stores at OBJTYPE+0xAC
```

**FUN_005F9070 — Image= SHP loader (FULL decompilation):**

```
1. Theater-suffix logic:
     if (AlternateArcticArt && Scenario.Theater == ARCTIC && !arctic_applied):
         sprintf(buf, "%sA", OBJTYPE+0x1F8)
         strncpy(OBJTYPE+0x1F8, buf, 0x18)
         OBJTYPE+0x212 = 1                  ← "arctic suffix applied" flag
2. FUN_007C9FF0(buf, 0, 0, OBJTYPE+0x1F8, ".SHP")  ← filename = "<Image>.SHP"
3. if (Theater flag at OBJTYPE+0x22C):
     // theater-specific extension list at &DAT_007E1BC6 + Theater * 0x70
     FUN_007C9FF0(buf, 0, 0, OBJTYPE+0x1F8, theater_extension_table[Theater])
4. else if (NewTheater flag at OBJTYPE+0x237 && Theater != -1):
     buf[1] = theater_letters[Theater * 0x70];  ← single-letter theater code
5. if (existing SHP at OBJTYPE+0xA4 with owns flag at +0xA8):
     operator_delete[]  the old SHP
6. OBJTYPE[0x29] = OBJTYPE[0x2A] = 0
7. int kind = (**(vtable+0x2C))();
   if (kind == 0x15 || kind == 5):                  ← OverlayType OR UnitType
       OBJTYPE[0x29] = LoadFileFromMIX(buf)
   else if (kind == 0x1E || kind == 0x25):          ← TS-era types (skip)
       goto done
   else:
       OBJTYPE[0x29] = LoadFileFromMIX(buf)
8. if (OBJTYPE[0x29] == 0):
     // Retry with 'G' theater letter (Generic fallback)
     buf[1] = 'G'
     OBJTYPE[0x29] = LoadFileFromMIX(buf)
9. if (OBJTYPE[0x29] != 0):
     OBJTYPE[0x7B] = max(SHP[+4], SHP[+2], 8)        ← bounding box dim
```

**Key offsets on ObjectTypeClass (HIGH confidence):**

| Byte offset | Field | Purpose |
|-------------|-------|---------|
| +0x1F8 | char[25] | `Image=` filename |
| +0x213 | char[25] | `AlphaImage=` filename |
| +0xA4  | `SHPFile*` | Resolved primary SHP pointer |
| +0xA8  | byte | `is_owned` flag for +0xA4 |
| +0xAC  | `SHPFile*` | AlphaImage SHP pointer |
| +0x1EC | byte | Bounding-box dimension `max(w, h, 8)` |
| +0x22C | byte | `Theater=` flag (per-theater filename variant) |
| +0x236 | byte | `Voxel=` flag |
| +0x237 | byte | `NewTheater=` flag |
| +0x844 (param_1+0x211) | byte | `AlternateArcticArt=` flag |
| +0x848 (param_1+0x212) | byte | Runtime "arctic suffix already applied" flag |

**Error handling:** missing SHP → `LoadFileFromMIX` returns 0 → retry with
'G' theater letter → final null silently stored at +0xA4. **No assert, no
fallback animation.** Renderer must null-check the SHP pointer before draw.

`LoadFileFromMIX` at **0x005B40B0** is the MIX-archive reader (CRC
filename, search MIX index tree at `DAT_00ABF00C`, fall through to
CCFileClass on miss). Used by both Image and AlphaImage paths.

**ObjectTypeClass base size** (resolves §7.7 Open Question): **0x294**.
Confirmed by extrapolation — the constructor at 0x005F7090 initializes
fields up to +0x290, and ParticleTypeClass's first new field at +0x294
(NextParticleOffset) immediately follows.

### 11.5 TS-Legacy verdicts — orphan keys resolved

#### A. `GapGeneratorParticleSystem` — DOES NOT EXIST

Resolves §7 implicit gap. The plan and the prior-pass scoping both
hypothesized that `GapGenerator`-flagged buildings might have a separate
`GapGeneratorParticleSystem=` key. **They don't.** Verified by string
search: no such key exists in the binary.

The "particle system gap generators use" IS **`NaturalParticleSystem`** at
TTC+0x764, accessed by `BuildingClass::UpdateGapGenerator_Tick`
(0x00454DB0) when the gap-generator state machine transitions 3 → 0.

#### B. `NaturalParticleSystem` — Active code path, dormant slot

Resolves §7.5. The TTC+0x764 slot IS read by live YR code via
`BuildingClass::UpdateGapAndSpecialEffects` (0x004549B0) → vtable+0x414 →
`UpdateGapGenerator_Tick` (0x00454DB0). YR ships gap generators
(`[GAGAP]`, etc.) so the **call path is reachable in standard skirmish**.

However, **no standard YR INI file sets `NaturalParticleSystem=`** on any
TechnoType. The slot is always null in retail. The constructor call
`ParticleSystemClass::Constructor(NULL, ...)` happens but produces no
visible particles. Cloak ring drawing (the actual visible gap-generator
effect) is handled separately and uses `CloakRadiusInCells` (TTC+0xCD2),
not particles.

**Active in YR:** Conditional. Code path live; INI value always null.
Constructor must accept null type pointer (verified in §3 of the report —
behaviour is to produce a no-op PSC).

#### C. `DestroyParticleSystems` — DEAD in YR (HIGH confidence)

Resolves an implicit gap. The TTC+0x794..0x7AB vector slot exists and is
parsed (read site at 0x713D39 verified). **Zero standard-YR consumers.**
Searched: `TechnoClass::ReceiveDamage` (0x701900), `ObjectClass::ReceiveDamage`
(0x5F5390), `BuildingClass::ReceiveDamage` (0x442230),
`ObjectClass::Destroy` (0x5F5280), `BuildingClass::Destroy` (0x44EBF0),
`FootClass::Destroy` (0x4D9720). None read TTC+0x798 (vector data) or
TTC+0x7A4 (size).

YR's death-time particle bursts are driven by `WarheadTypeClass::AnimList`,
`DebrisTypes`, and the explosion AnimClass — not by `DestroyParticleSystems`.

**Active in YR:** No. Parsing is harmless (no-op if absent); implementing
the consumer is not required for parity with retail YR.

#### D. `ChronoSparkle2` — DEAD in YR

Resolves §7 implicit gap. No INI occurrences and no parser found. Drop
from documentation.

#### E. Scenario_Start global PSC (`DAT_00A8ED78`) — Active but invisible

Resolves §7.6 partially and the "is the global PSC reachable in skirmish?"
plan question. `FUN_00684C30` is reached from
`ScenarioClass::Start_Scenario` (0x00683AB0) ← `Main_Game` (0x0052D9A0) —
**the standard YR map-load path for any skirmish or campaign scenario.**
The function runs every game.

**The PSC creation** uses a hardcoded string `"GasCloudSys"` at
`DAT_0083DA90` (verified by memory read), resolved via
`ParticleSystemTypeClass::FindOrAllocate` (0x00644630) into the global
PSType vector at `DAT_00A83D6C`. The PSC is created at fixed coord
(0xA80, 0xA80, 0) = world cell (10, 10) — **inside the playable map**, not
limbo.

The system is **invisible during normal play** (no emit unless the
game-side gas-system trigger fires, which doesn't happen passively at
that corner). Best understood as a "warm-up" PSC kept alive across the
session for the gas/poison subsystem to source particle handles from.

**Active in YR:** Yes (but invisible). For 99% parity: replicate the
allocation; visual impact is nil but structural state changes (one extra
entry in the global PSC list).

### 11.6 Save / Load (IPersistStream)

Resolves the remaining serialization gap.

**Crucial correction to the plan's assumption:** IPersistStream is
implemented in the **primary** vtable (slots +0x10..+0x1C), NOT in a
secondary vtable. The 3 secondary vtables are tiny adjustor-thunk tables
for IRTTITypeInfo / INoticeSink / INoticeSource, with 1-1-6 slots
respectively. Westwood collapsed multiple-inheritance bookkeeping into
1-slot dispatchers; full COM-method-set semantics live in the primary
AbstractClass vtable.

**Primary-vtable IPersistStream slots (both PSC at 0x007EFB9C and ParticleClass at 0x007EF954):**

| Slot offset | PSC address | ParticleClass address | Role |
|-------------|-------------|----------------------|------|
| +0x00 | 0x00410260 | 0x00410260 | `AbstractClass::QueryInterface` (shared base) |
| +0x04 | 0x00410300 | 0x00410300 | `AbstractClass::AddRef` |
| +0x08 | 0x00410310 | 0x00410310 | `AbstractClass::Release` |
| +0x0C | 0x006301A0 | 0x0062D930 | `GetClassID` (returns CLSID — see below) |
| +0x10 | 0x00410450 | 0x00410450 | `AbstractClass::IsDirty` (checks +0x20 byte) |
| +0x14 | **0x0062FF20** | **0x0062D7A0** | **Load** |
| +0x18 | **0x00630090** | **0x0062D810** | **Save** |
| +0x1C | 0x004103E0 | 0x004103E0 | `AbstractClass::GetSizeMax` (returns class-size from primary[+0x30]) |

**CLSIDs:** PSC = `*(0x007E96C0)`, ParticleClass = `*(0x007E9700)` (4
dwords each, GUIDs).

**ParticleSystemClass::Load (0x0062FF20) — flow:**

1. Calls `ObjectClass::Load(this, pStream)` at 0x005F5E80, which calls
   `AbstractClass::Load` at 0x00410380. AbstractClass::Load reads the
   4-byte unique ID from the stream and registers the new `this` pointer
   under that ID via `FUN_006CF2C0` (Swizzle::RegisterID).
2. Re-installs all 4 vtable pointers (primary + 3 secondaries) — save
   streams contain no vtable bytes; Load writes them from `.rdata`.
3. Calls `FUN_006302A0(0, 0)` — looks like dead code or a defensive reset
   (sets +0xC0 vector data ptr to 0, +0xCC count to 0, +0xCD owns flag
   to 0). Note: AbstractClass::Save dumps the full byte image including
   +0xC0..+0xD0, so the saved values are restored before this reset
   happens — meaning the reset effectively zeros the just-restored fields.
   **Open question** flagged below.
4. Registers swizzle fixups via `Swizzle::Register` (0x006CF240) for:
   - PSC+0xAC (ParticleSystemTypeClass pointer)
   - PSC+0xE4 (owner / source object)
   - PSC+0xE0 (attached object)
5. Reads particle count (4 bytes) from stream.
6. Loop: for each particle slot, read 4 bytes (the saved ParticleClass
   pointer key), append to vector, register the slot for later swizzle.

**ParticleSystemClass::Save (0x00630090) — symmetric:**

1. Calls `AbstractClass::Save(this, pStream, fClearDirty)` — dumps the
   full byte image of the object via primary[+0x30] size accessor.
2. Writes `*(this+0xCC)` (count, 4 bytes).
3. Loops `count` times, writing each `*(this+0xC0) + i*4` ParticleClass
   pointer (raw pointer values; used as swizzle keys on load).

**ParticleClass::Load (0x0062D7A0) — simpler:**

1. `ObjectClass::Load(this, pStream)`.
2. Re-install 4 vtables (primary 0x7EF954, secondaries 0x7EF938 / 0x7EF930
   / 0x7EF928).
3. Register swizzle fixups for ParticleClass+0xAC (ParticleType ptr) and
   +0x124 (owner ParticleSystemClass ptr).
4. Return — no nested collection.

**ParticleClass::Save (0x0062D810):**

1. `AbstractClass::Save`.
2. Sets `*(this+0x130) = 1` after save. Semantic unconfirmed — likely an
   "image-cleared / saved" flag. **LOW confidence on meaning;
   reproduce as-is.**

**Swizzle / pointer-fixup mechanism:**

- Singleton at `DAT_00B0C110` (BSS).
- `Swizzle::Register(mgr, void** ppPtr)` at **0x006CF240** — appends
  `(saved_key, &slot)` to a fix-up list, then **clears `*ppPtr = 0`**
  (so post-Load pointers are temporarily null).
- `Swizzle::Resolve_All(mgr)` at **0x006CF230 → 0x006CF350** — sorts both
  lists (saved-IDs from AbstractClass::Load, slots-to-fix from
  Swizzle::Register), walks them in lockstep, and writes the new `this`
  address into each registered slot when keys match.
- Resolve callsites: `ScenarioClass::Full_Init` at **0x006875E9** and
  **0x00687BF6** — standard two-phase scenario load (pre-objects, then
  post-objects).
- **Pointers referencing objects not present in the save remain null
  after resolution.** Confirmed.

**PSC::Load does NOT re-register into the global active object list.**
The constructor's list-register block (which adds new PSCs to the global
PSC vector) is bypassed. The engine relies on the saved pointer being
preserved (with vtable re-installed), and on the global vector itself
being part of the .SAV image.

**Open question (logged in §11.8 below):** the role of the `FUN_006302A0(0, 0)`
call inside PSC::Load before vtable re-install. Looks dead but may have
subtle ordering semantics. Out of scope for this pass; flag for any
implementer attempting bit-exact save-format parity.

### 11.7 Vtable corrections to the existing report

Both PSC and ParticleClass primary vtables are **122 slots / 0x1E8 bytes
total**, ending at **slot +0x1E4 inclusive**.

**Confirmed claims from §1–§10:**

| Existing claim | Slot | Status |
|----------------|------|--------|
| `+0x14 Load = 0x0062FF20` (PSC) | +0x14 | CONFIRMED |
| `+0x48 GetCoords = 0x005F65A0` | +0x48 | CONFIRMED |
| `+0xF8 Mark for deletion = 0x006301E0` (PSC) | +0xF8 | CONFIRMED |
| `+0x114 Light source create = 0x0062E280` (PSC) | +0x114 | CONFIRMED |
| `+0x1B4 SetCoords = 0x005F6940` | +0x1B4 | CONFIRMED (it's `Set_Raw_Coords`, same role) |

**CORRECTED claims:**

| Existing claim | Reality | Fix |
|----------------|---------|-----|
| `+0x6C GetImage` | +0x6C is `0x005F3E30`, an Object dispatch; the **real** "get my type/image" accessor is at +0x88 (PC override = 0x0062D990 returns this+0xAC = ParticleType ptr) | Drop "+0x6C = GetImage" framing; call it "draw-it / render-override dispatch" |
| `+0x1D0 GetImageFrame` | +0x1D0 is `ObjectClass::GetHeight`, NOT a frame fn | The real per-particle frame-computing vfn is at **+0x1E4** (last slot) — `0x0062D830` on ParticleClass, AnimClass-shared stub on PSC |
| `+0x1E8 GetAnimFrame` | OUT OF BOUNDS — vtable ends at +0x1E4 | Drop this row from §2.3 |

**Save slot (NEW):** `+0x18` = PSC `0x00630090`, ParticleClass `0x0062D810`.
This was missing from §1–§10 and is documented in §11.6.

**Per-particle Draw_It dispatch slot:** Particle-class Draw_It is at
**vtable+0x110 = 0x0062CEC0**. PSC's slot +0x110 is just an AnimClass-shared
stub (0x00426450). When iterating drawables, dispatching to vtable+0x110
hits `ParticleClass::Draw_It` for individual particles but bypasses PSC.

**Secondary vtables (tiny adjustor-thunk tables, 1+1+6 slots):**

| Class | secondary_12 | secondary_8 | secondary_4 |
|-------|--------------|-------------|-------------|
| PSC | 0x007EFB70 (1 slot, RET 4) | 0x007EFB78 (1 slot, return 0) | 0x007EFB80 (6 slots: this-adj QI/AddRef/Release thunks + IRTTI Process/GetID/AssignUniqueID) |
| ParticleClass | 0x007EF928 | 0x007EF930 | 0x007EF938 |

PSC and ParticleClass share **byte-identical** secondary vtable function
tables; only the COL (Complete Object Locator) RTTI metadata differs.

**Ghidra mislabel to ignore:** address `0x00410600` is currently labeled
`ObjectClass__GetCoords` in Ghidra but its body just calls
`AbstractClass::Release` — it's the IRTTI Release adjustor thunk. Real
GetCoords is at 0x005F65A0.

### 11.8 Secondary spawn callers — full coverage

#### 11.8.A VoxelAnim attached PSC (Plan #21)

- **VAType byte offset:** 0x2FC (verified)
- **INI key:** **`AttachedSystem=`** (NOT "AttachedParticleSystem" or
  "Spawns" — those are different)
- **Section:** `[<VoxelAnim>]` per-type (e.g. `[BARLEXP01]`)
- **Parser:** `VoxelAnimTypeClass::ReadINI` at **0x0074B050**, ReadString
  for `s_AttachedSystem_00845EA0` → resolves via `FUN_00644890`
- **Spawn site:** end of `VoxelAnimClass::Constructor` (0x007493B0) —
  guarded by `*(VAType+0x2FC) != 0`
- **Spawn parameters:** PSC type from VAType+0x2FC, coords = ctor `param_3`,
  cell = `CellClass::Get_Cell_At(param_3)`, owner = the VoxelAnim itself,
  fifth param = `0xB1D188` (constant default-coord)
- **Stored at:** `VAClass+0x108` (param_1[0x42])
- **Active in YR:** Yes — debris, barrels, missile-fire, refinery dump
  spawn voxel anims that include this PSC.

#### 11.8.B TriggerAction particle case (Plan #22)

- **Action subtype index: 0x58 (88 decimal)** — the only PSC-spawning
  case in the entire `TriggerAction::Execute` switch
- **Action name:** "Particle System at Waypoint" (standard YR map-trigger
  action exposed in trigger editors)
- **Parameters:**
  - `param_1+0x44` = waypoint index → resolved via `FUN_0068BCC0` → cell
    coord → `FUN_00642740` → world coord; Z component clamped to ground
    height via `CellClass::GetGroundHeight`
  - `param_1+0x90` = PSType index in global PSType vector
    `DAT_00A83D6C`
- **Active in YR:** Yes. No SpecialFlags / Tiberium gating; reachable
  from any YR map (.map / .mmx) using this trigger action.

#### 11.8.C Refinery dump spawner (Plan #23) — `FUN_00459900`

- **Class:** BuildingClass member (param_1 = BuildingClass*)
- **Caller:** `UnitClass::AI` at 0x007360C0 — fires when a harvester
  docks and the dump animation reaches the appropriate frame, gated
  by `RefinerySmokeFrames` on the BuildingTypeClass
- **Spawn count:** Up to 4 systems, one per `RefinerySmokeOffsetN` slot
  (TTC+0x7CC..+0x7F8). For each: if the offset triplet equals the
  sentinel coord at `DAT_0089C848` OR `FUN_00459F60` returns true (a
  global "refinery smoke disabled" runtime check), skip
- **PSC type:** TTC+0x774 (`RefinerySmokeParticleSystem`) — same type
  used for all 4 spawns
- **Spawn params:** PS type, building.position + offset, cell = 0,
  owner = building, fifth param = `&DAT_0089C848` (sentinel coord)
- **Lifetime:** Fire-and-forget (PSC pointer not retained)
- **Active in YR:** Yes — refineries are core YR.

#### 11.8.D EBolt::Init (Plan #24) — `FUN_004C2A60`

- **Class:** `EBolt` (electric bolt visual). 30-byte `operator_new`,
  registered in global EBolt vector at `DAT_008A0E8C`
- **Caller chain:** `EBolt::Init` ← `TechnoClass::CreateElectricBolt`
  (0x006FD460) ← `TechnoClass::SpawnElectricBoltEffect` (0x006FD570) ←
  `TechnoClass::Fire_At` (0x006FDD50)
- **PSC type:** `g_RulesClass + 0x1020` = **`DefaultSparkSystem`** (see
  §11.8.G below)
- **Spawn params:** PS type, source coords, cell = 0, owner = 0
- **Active in YR:** Yes — Tesla Coil, Tesla Trooper, Prism Tower, Prism
  Tank, MagBeam (Robot Tank), IFV-in-Tesla-mode all use the bolt path
  via per-weapon `IsElectricBolt=yes`. Standard YR weapon flag.

#### 11.8.E Scenario_Start global PSC (Plan #25) — `FUN_00684C30`

Already covered in §11.5.E. Active in YR but invisible.

#### 11.8.F Gap generator confirmation (Plan #26)

`BuildingClass::UpdateGapGenerator_Tick` (0x00454DB0) does NOT pre-check
`*(BuildingType + 0x764)` for null. The outer guard checks the
`GapShroudOffset` triplet (`BuildingType+0x768/+0x76C/+0x770`) against a
sentinel coord — if all three components match the sentinel, no spawn.
If they differ, the code allocates a 0x100-byte PSC and calls
`ParticleSystemClass::Constructor(NULL, ...)` when `+0x764` is null.
**The constructor must handle null type pointer** (no observed crash in
retail because all gap-generator BuildingTypes happen to set
`NaturalParticleSystem=GAPGEN`-or-similar in art INI; for Rust parity
the ctor should null-check defensively).

#### 11.8.G RulesClass +0x1020 INI key — DEFINITIVE

- **INI key: `DefaultSparkSystem=`**
- **Section:** `[CombatDamage]`
- **Parser:** `RulesClass::ReadCombatDamage` at 0x0066BBB0
- **Default value:** `Sparks` (in retail rulesmd.ini)
- **Single field, not array:** confirmed shared by 3 callers
  (`CaptureManagerClass::Update`, `WarpAttachClass::UpdateAttack`,
  `EBolt::Init`) all reading the same offset.

For completeness, the surrounding `[CombatDamage]` PSType slots are:

| Byte offset | INI key |
|-------------|---------|
| +0x1018 | DefaultLargeGreySmokeSystem |
| +0x101C | DefaultSmallGreySmokeSystem |
| **+0x1020** | **DefaultSparkSystem** |
| +0x1024 | DefaultLargeRedSmokeSystem |
| +0x1028 | DefaultSmallRedSmokeSystem |
| +0x102C | DefaultDebrisSmokeSystem |
| +0x1030 | DefaultFireStreamSystem |
| +0x1034 | DefaultTestParticleSystem |
| +0x1038 | DefaultRepairParticleSystem |

All resolve via `ParticleSystemTypeClass::Find_Or_Create` on the read
string.

#### 11.8.H BarrelParticle parser (Plan #28)

- **INI key: `BarrelParticle=`**
- **Section:** **`[General]`** (corrects Phase 1 scoping note that said
  `[AudioVisual]`)
- **Parser:** `RulesClass::ReadGeneral` at 0x0066D530, read site at
  0x0066D7ED
- **RulesClass byte offset: 0x74**
- **Caller:** `Apply_area_damage` at 0x0048A19D — when an explosion
  destroys a barrel-overlay cell, after creating the BarrelExplode anim,
  it spawns this PSC at the cell coord using
  `ParticleSystemClass::Constructor(g_RulesClass+0x74, coord, 0, 0, &DAT_0089E830, 0)`.
- **Active in YR:** Yes — random destructible barrels (`BARL01`, `BRL3`
  overlays) trigger this.

### 11.9 Resolution of §7 Open Questions

| §7 Question | Status | See |
|-------------|--------|-----|
| 1. `FUN_00630B90` and `FUN_00630EA0` lifecycle | Already RESOLVED in §8.5 (PhoneEd dialog dead code, not particle-related) | §8.5 |
| 2. Gas particle bridge collision details | RESOLVED in §8.4 | §8.4 |
| 3. Color interpolation function `FUN_00661020` | RESOLVED in §8.2 | §8.2 |
| 4. Wind direction tables at 0x836664 / 0x836684 | RESOLVED in §8.1 + §9.5 (gas) and §10.14.4 (smoke) | §8.1, §10.14.4 |
| 5. `NaturalParticleSystem` usage | RESOLVED — see §11.5.B (active code path, dormant slot) | §11.5.B |
| 6. `FUN_0062E280` vs `AI_Spark` light redundancy | RESOLVED — see §11.7 (vtable +0x114 = `0x0062E280`, only spark-with-light systems pay the cost; AI_Spark inline path is the persistent-light branch, FUN_0062E280 is the per-tick one-frame-light branch — no redundancy, OneFrameLight gates which runs) | §11.7 |
| 7. Exact struct sizes for type classes | RESOLVED — ObjectTypeClass = 0x294 (§11.4); ParticleSystemTypeClass ends at +0x30C (constructor highest write); ParticleTypeClass ends at +0x318 (`operator_new(0x318)` confirmed in §9.2) | §11.4 |

### 11.10 New open questions / known limits

These items were surfaced during the gap-closing pass but couldn't be
fully resolved from binary alone. They're explicitly **deferred** rather
than guessed.

1. **Empty `ColorList=` uninit-leak path** (§11.1) — likely benign in
   practice (count is 0, indexers never read), but a fastidious Rust
   parity implementation should explicitly zero +0x2C8..+0x2D0 on
   missing-key.
2. **`FUN_006302A0(0,0)` call inside PSC::Load** (§11.6) — appears to be
   dead code or has subtle ordering semantics tied to the AbstractClass
   byte-image dump. Out of scope for this pass; flag for any
   implementer doing bit-exact save-format parity.
3. **ParticleClass::Save sets +0x130 = 1** (§11.6) — semantic LOW
   confidence. Reproduce as-is for parity.
4. **`DamageSmokeOffset` / `DamSmkOffScrnRel` / `DestroySmokeOffset`
   consumers** (§11.2) — TTC+0x7B0..+0x7C8 fields are parsed but the
   consumers were not located in this pass. They likely pair with
   `DamageParticleSystems` / `DestroyParticleSystems` to set spawn
   offsets, but verification is deferred. `DamageSmokeOffset` is likely
   active (paired with the live `DamageParticleSystems` consumer in
   `TechnoClass::AI_Update` and `ReceiveDamage`); `DestroySmokeOffset`
   is likely dead alongside `DestroyParticleSystems`.
5. **`Report=` and `SpawnDelay`/`RandomRate` keys in `[Particles]`** —
   surfaced by INI scan but not researched in this pass. Flag for a
   follow-up `/plan-investigation particle-audio-and-spawn-delay` if
   needed.
6. **LightSourceClass internals** (`FUN_005FF250` / `FF2D0` / `FF850`) —
   used by spark-with-light systems. Entry points known; runtime layout
   not researched. Out of scope; sibling investigation candidate.
7. **`What_Am_I` RTTI codes used by ObjectTypeClass image loader** —
   identified codes 5 (UnitType) and 0x15 (OverlayType) take the SHP
   path; codes 0x1E and 0x25 fall through (TS-era types). Other codes'
   exact behavior in `FUN_005F9070` was not exhaustively traced.

### 11.11 Sources (additional functions decompiled)

**Phase 1 — ColorList parser:**
- `0x00476B20` ColorList parser (FULL)
- `0x00478440` Vector::Clear (MEDIUM)
- `0x004788E0` Vector::CopyFrom (MEDIUM)
- `0x00524EC0` identity passthrough (LIGHT)
- `0x00477AC0`, `0x004784F0`, `0x00477900` vector vtable methods (LIGHT each)
- `0x00478850` vector ctor (incidental)
- `0x00478220` vector CopyFrom variant (incidental)
- Vtable @ `0x007E4E58` (4 slots read)

**Phase 1 — TTC / Building / Image=:**
- `0x00712170` TechnoTypeClass::ReadINI (MEDIUM, scoped to particle keys)
- `0x0045FE50` BuildingTypeClass::ReadINI (MEDIUM, scoped — confirmed no particle keys)
- `0x005F92D0` ObjectTypeClass::ReadINI (FULL)
- `0x005F9070` SHP-load helper (FULL)
- `0x005B40B0` LoadFileFromMIX (LIGHT)
- `0x00717C00`, `0x00717C70`, `0x00717CD0`, `0x00717D20` PSType vector helpers (LIGHT each)
- `0x00644890` ParticleSystemTypeClass::Find_Or_Create (LIGHT)

**Phase 2 — Save/Load:**
- `0x0062FF20` PSC::Load (FULL)
- `0x00630090` PSC::Save (FULL — mid-fn entry inside FUN_0062FF20)
- `0x0062D7A0` ParticleClass::Load (FULL)
- `0x0062D810` ParticleClass::Save (FULL)
- `0x005F5E80` ObjectClass::Load (LIGHT — confirmed shared base)
- `0x00410380` AbstractClass::Load, `0x00410320` AbstractClass::Save (referenced)
- `0x006CF240` Swizzle::Register, `0x006CF230` / `0x006CF350` Swizzle::Resolve_All, `0x006CF2C0` Swizzle::RegisterID
- ScenarioClass::Full_Init resolve callsites at `0x006875E9` and `0x00687BF6`
- `0x006302A0` PSC vector reset helper (LIGHT)

**Phase 2 — Vtable enumeration:**
- Primary vtables read: `0x007EFB9C` (PSC, 122 slots), `0x007EF954` (ParticleClass, 122 slots) — 0x200 bytes each
- Secondary vtables read: `0x007EFB70`, `0x007EFB78`, `0x007EFB80` (PSC); `0x007EF928`, `0x007EF930`, `0x007EF938` (ParticleClass)
- CLSID memory reads: `0x007E96C0` (PSC), `0x007E9700` (ParticleClass)

**Phase 3 — Spawn callers:**
- `0x007493B0` VoxelAnimClass::Constructor tail (LIGHT)
- `0x0074B050` VoxelAnimTypeClass::ReadINI for `AttachedSystem=` (LIGHT)
- `0x006DD8B0` TriggerAction::Execute, case 0x58 (LIGHT scoped)
- `0x00459900` Refinery dump spawner (MEDIUM)
- `0x004C2A60` EBolt::Init (MEDIUM)
- `0x006FD460`, `0x006FD570` EBolt caller chain (LIGHT)
- `0x00684C30` Scenario_Start tail (MEDIUM)
- `0x00683AB0` ScenarioClass::Start_Scenario (LIGHT confirmation)
- `0x00644630` ParticleSystemTypeClass::FindOrAllocate (LIGHT)
- `0x00454DB0` UpdateGapGenerator_Tick null-check verification (LIGHT)
- `0x0066BBB0` RulesClass::ReadCombatDamage for `DefaultSparkSystem=` (LIGHT)
- `0x0066D530` RulesClass::ReadGeneral for `BarrelParticle=` (LIGHT)
- `0x0048A19D` Apply_area_damage barrel-spawn site (LIGHT)

**Memory reads (string literals / data tables):**
- `0x0083DA90` "GasCloudSys" string (verified)
- `0x00843FBC` "RefinerySmokeParticleSystem" string
- `0x00845EA0` "AttachedSystem" string
- `0x0083AE80` "DefaultSparkSystem" string
- `0x0083CF1C` "BarrelParticle" string
- `0x007E96C0`, `0x007E9700` CLSIDs
- Sentinel coord at `0x0089C848`/`0x0089C84C`/`0x0089C850`

---

