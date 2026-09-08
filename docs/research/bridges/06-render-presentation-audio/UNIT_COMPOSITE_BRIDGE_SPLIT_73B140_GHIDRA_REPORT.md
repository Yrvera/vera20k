# Unit composite bridge split — 0x0073B140

Read-only live Ghidra investigation, 2026-09-08. Program: active retail YR
`gamemd.exe`, SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
The 792 bytes beginning at `0x0073B140` match the local retail executable;
their SHA256 is
`3c3c839c2ea2171457cfa004e483d3d0a06d5d99e3a20d5699d6cf81b4176f49`.
Evidence below comes from instruction bodies, callers, vtable writers and
retail INI data, not function labels. No native image bytes are included.

## Corrected identity and active callers

Unit's primary vtable starts at **0x007F5C70**, written by the constructor at
`0x0073543A` and load at `0x00744521`. Its `+0x55C` slot, at `0x007F61CC`,
points to `0x0073B140`. The former `0x007F6000/+0x1CC` identification used an
interior address as a vtable base and is withdrawn.

`Unit DrawExtras 0x0073CEC0` tests the active type's `Voxel` byte at
`0x0073D317`. The voxel branch calls `+0x554 = 0x0073B470` at `0x0073D359`;
that function calls `+0x55C = 0x0073B140` at `0x0073C1A5`, after rendering
the hull, turret and barrel into a temporary indexed surface. Retail
`MTNK Image=GTNK`, `GTNK Voxel=yes`, and `MTNK TooBigToFitUnderBridge=yes`
establish an ordinary tank caller. This is not a SHP-only or shadow helper.

The temporary surface is 256 by 256, one byte per pixel
(`0x007473FA..0x00747434`). The final composite rectangle is accumulated in
`0x00B1CFC0..0x00B1CFCC`; `0x00706933..0x007069F0` updates its union.
Native source rectangle bounds must be distinguished from padded Rust atlas
allocations when implementing the split.

SHP units with turrets also composite body/turret through this function
(`0x0073CC36`, `0x0073CD08`, final call `0x0073CDDB`). The no-turret SHP
branch at `0x0073CE0D` instead draws directly through `+0x50C`, with its own
TooBig `a7=-16, gradient=0` branch. Infantry's `0x00518F90` draw is separate.
Retail SQD (`Voxel=no`, `Turret=no`, `TooBigToFitUnderBridge=yes`) reaches
this direct SHP branch. It applies to the whole frame, with no composite
height threshold and no WeaponsFactory alternative. The implementation's
SHP caller now supplies this whole-body adjustment through the shared Foot
adapter; it does not set the voxel composite flag.

## Eligibility

Assembly `0x0073B1A8..0x0073B207` requires the active Unit type pointer
`[unit+0x6C4]` to have byte `+0xE16` set, then either condition below:

1. `0x00703B10(unit)` is true and `0x00703E70(unit)` returns zero.
2. `[unit+0x5A4]` (NavCom) is non-null, first radio link
   `0x0065AD40(unit,0)` is non-null, its virtual `+0x2C` returns 6, and its
   type `[link+0x520]` has byte `+0x16BD` set.

The latter type byte is **WeaponsFactory**. The INI reader at
`0x00460A72..0x00460A8C` uses the literal key at `0x0081AA4C`; describing
this gate as repair-hut state is incorrect. The alternative is evaluated
only after the bridge condition fails. DrawExtras may temporarily substitute
Harvester `UnloadingClass` (`0x0073D2C4`, restored `0x0073D3A2`), so the type
active at drawing is authoritative.

`0x00703B10` returns false for `OnBridge` byte `[unit+0x8C] != 0`. Otherwise
it accepts the current cell's structural bridge flag `0x100`, or suitable
orthogonal neighbors: north/south require orientation `0x800`, east/west
require that orientation bit clear. This is a below/beside-bridge test,
despite the existing `IsOnBridge_ForFiring` label.

`0x00703E70` is gated by that predicate or the corresponding low-bridge
`0x400` predicate `0x00703CC0`. It resolves south, east, southeast cells.
A tile is a hit when it is neither `0xFF` nor `0xFFFF` and signed
`tile - [0x00AA0E28] + 1` lies in 7..=16. The result is
`(south_hit || east_hit ? 1 : 0) + (southeast_hit ? 1 : 0)`, hence **0..=2**,
not a count of three independent hits. `0x00AA0E28` is the concrete bridge
tile-set base. The coordinate offsets are established by initializer
`0x0049F2F0`, not their zero-initialized image bytes.

## Rectangles, Z and local draw order

Let the accumulated source rectangle be `(x,y,w,h)`, and the supplied
screen anchor be `(screen_x,screen_y)`. Destination origin is
`(dx,dy)=(x+screen_x-128,y+screen_y-128)`.
The height test `h > 16` happens **before clipping** at
`0x0073B2FE..0x0073B307`. If eligible and tall enough:

| Order | Source rectangle | Destination rectangle | Gradient | Z adjustment |
|---|---|---|---:|---:|
| 1 | `(x,y,w,h-16)` | `(dx,dy,w,h-16)` | 0 | `FootZ-5` |
| 2 | `(x,y+h-16,w,16)` | `(dx,dy+h-16,w,16)` | 2 | `FootZ` |

The lower strip is **full width by 16**, not 16 by 16. Instructions
`0x0073B37C..0x0073B395` advance source/destination Y by the first height
and replace only their heights with 16. X and width do not change.
The two calls to `Standard_SHP_blitter 0x004373B0` occur immediately at
`0x0073B377` and `0x0073B3E5`. There is no later bridge-pass enqueue here.

Otherwise the whole rectangle is drawn once at `0x0073B446`, using
locomotor gradient `+0x2F0` (default 2) and `FootZ`.

**Foot GetZAdjust `+0x2EC = 0x004DAFC0` has no mode argument.** Its return
at `0x004DB09E` pops no argument. Pushes 0 and 2 before those virtual calls
are pending arguments for the subsequent blitter, not modes supplied to
Foot. The split calls Foot separately for each piece. There is no additional
DrawSHP-wrapper `-2` on this direct composite path.

Each blitter call clips its own rectangles before seeding
(`0x00437461`, then `0x004374BB..0x004375A7`). Gradient 0 uses the top seed
and decreases Z each row; gradient 2 uses a bottom-based seed and a
three-row accumulator. The live six-dword table entries at `0x00817710`
are `{1,1,1,1,-1,1}` and `{1,3,1,3,1,0}` respectively. A common full-image
seed with only a midpoint gradient change does not reproduce this behavior.

Ordinary draw flags are `0x2800`. Selector `0x00490B90` chooses slot `+0x98`;
initialization `0x0048F721/0x0048F735` installs vtable `0x007E56F0`, whose
scanline method is `0x00494B60`. That leaf skips index zero and accepts a
pixel only when incoming Z is **strictly less** than stored Z
(`CMP/JGE 0x00494BC5/0x00494BC7`). It writes palette/intensity color and
does **not** write Z. Equality rejects the unit pixel.

## Shadows and implementation boundary

Voxel shadow dispatch occurs after body composition, at
`0x0073C5C4 -> 0x004DB0D0 -> 0x00706BD0`. Cached and fresh leaves receive
flags `0x2001`. They do not traverse the two-piece `0x0073B140` body split.
SHP-with-turret shadows instead precede body composition, at `0x0073C7A7`,
using flags `0x2E01` and `FootZ-2`. Do not inherit the body's split into
either shadow path.

VERA currently emits voxel shadows before their bodies. That is a residual
relative to the native voxel caller's order above; the no-Z-write body leaf
does not support the older explanation that the shadow tests depth written
by its own hull. The split change keeps shadow handling separate and does
not claim to settle its blending, overlap or ordering parity.

For opaque indexed parts, one shared rectangle/region Z plus ordered
nonzero-index overwrites and no Z writes is equivalent to first assembling
**VERA's current ordered per-part indexed image**: all parts at one pixel
have the same depth result. This establishes the split transformation, not
native cached inter-part depth/composition equivalence, which remains
unresolved. Parts retain their parent painter position and overlap order.
Separate per-part alpha blending is not equivalent to compositing indices
first and blending once; existing translucent/cloak overlap remains a limit.

## Follow-up rectangle and ordering evidence

The bounds producer is now traced through actual Unit `Techno_Render
0x00706ED0`: call `0x00706FDA` submits all eight full VXL tailer corners
through `0x007540F0`, once per limb; a single `0x00754510` follows at
`0x00706FEF`. Submitted corners update one global f32 AABB. Relative to the
existing grid-scaled transform, its endpoints are **0..size**, not voxel
centers 0..size-1. Padding follows the union of limbs in that VXL file.

At `0x00754513..0x0075470F`, center is `(min+max)*0.5`, stored once to f32
under the process's x87 `0x0E7F` control word; span goes directly from the
f32-extrema difference to truncating `ftol`. With `W=trunc(spanX)+8` and
`H=trunc(spanY)+8`, the destination offsets are
`trunc(centerX)-W/2, trunc(centerY)-H/2`. Source dirty-rectangle offsets are
`124-trunc(spanX)/2, 124-trunc(spanY)/2`. Integer divisions truncate.

Unmodified native `0x00754510` was executed with ten prescribed f32 AABBs,
empty submission lists at `B2D820/B2FB70`, and extrema at `B2D5E0/B2D948`.
The six returned integers provide the native rectangle regression fixtures.
The original cached union block `0x0070755D..0x00707678` was also executed
with ten initial/next rectangles. It replaces an empty accumulator, ignores
an empty next rectangle when the accumulator is valid, expands minimum
edges normally, and adds **one** only when extending a maximum edge.
These executed results are retained in
[`vxl_raster.rs` tests](../../../../src/render/vxl_raster.rs).
They establish those leaves over the supplied cases; constructors, complete
cache rasterization and whole-game pixels were not part of that execution.

Atlas entries now keep native draw metadata separate from texture bounds,
including the slope-transition cache. The metadata projection inherits
VERA's existing f32/glam matrices and cannot establish full native matrix
equivalence. A nonzero TurretOffset is another explicit boundary: native
`0x0073BA4C` performs `FILD Type+0x720 * B1D008`, stores f32, then applies
`0x005AE980` before bounding. The existing external Rust `/8` displacement
is approximate. The prepared MTNK comparison has zero TurretOffset; this
change does not migrate the full pivot/raster path.

The separate native global order has now been traced: railing draws
`0x006D7C00 -> 0x004802A0 -> 0x00547230` occur within terrain phase
`0x006D3040`, before object traversal `0x006D3D10`. The presentation changes
place those railings before the ground-object traversal and retain objects
at their native parent painter position, rather than postponing an entire
below-bridge unit to a later bridge stream.

This report establishes native function/caller behavior and bounded native
rectangle-leaf comparisons. It does not claim whole bridge-rendering parity,
exact Rust/native transformed atlas bounds, every cloak leaf, native
inter-part cache composition, or matching full-screen pixels. GPU split
regressions are implementation checks, not native image goldens. Infantry
navigation is a separate investigation.

## Implemented and tested boundary

The candidate was integrated with main `fd387663` before final checks. The
shader split stays in the normal Ground parent draw and reads the actual
scissor height. Requested-part native bounds travel separately from atlas
storage padding; only split-selected bodies adopt them. Ordinary unsplit
bounds remain unchanged. The direct no-turret SHP branch, including stock
SQD, uses its separate whole-body -16/gradient-zero adjustment.

Validation: 8,544 library tests passed, zero failed, 89 ignored; Clippy
completed with warnings. Eleven actual GPU depth regressions passed,
including the three new split/direct-SHP cases. Ten production state-adapter
checks passed. The native extent and ordered-union leaves have ten executed
cases each. Existing 85 native Foot-depth and 158 height fixtures reproduce.
The ordinary E1 underpass and three deck-crossing regressions also pass.

These results do not replace paired game-image acceptance or certify the
explicit transformed-bounds, pivot, inter-part, translucent or shadow limits
above. No game asset, retail INI/MIX, snapshot version or replay golden was
changed for this correction.

The release build also completed successfully. Its as-built SHA256 is
`15692f826c94a243d39d5c673e822f902f1e02c2e6cefcbba25d3eb58629458c`.
The independent source/validation review found no scoped implementation blocker.
