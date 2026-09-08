"""Execute bounded Foot Z fixtures in the original, hash-pinned gamemd image.

Run ``python -m tools.render_depth_oracle`` with RA2_DIR or
VERA20K_GAMEMD_EXE set. ``--check`` compares fresh native results to the saved
JSON. Unicorn is an already-installed dependency; this tool downloads nothing.

This is raw-function emulation, not a game/session or rendered-pixel oracle.
Fixtures supply a Unit, its actual UnitType, ordinary mapped cells, and relocated
TMP headers. Native functions, virtual methods, cell lookup, TMP dimension
lookup, x87 AdjustForZ, and Foot max/add composition execute unmodified. Code
hooks observe addresses only; no native return values or branches are replaced.
The null locomotor contributes zero. Constructors, INI/map loading, active
tunnels, overlays, infantry-specific alternatives, and drawing are not covered.
No executable or retail art bytes are copied into the vector file.
"""

from __future__ import annotations

import argparse
from collections import Counter, deque
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, __version__ as unicorn_version
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_EBX,
    UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_ESI, UC_X86_REG_EDI,
    UC_X86_REG_EIP, UC_X86_REG_EFLAGS, UC_X86_REG_FPCW,
)

from tools.rmg_oracle.harness import GAMEMD, NATIVE_FPCW, _load_image


NATIVE_SHA256 = "1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c"
STANDARD_Z_MULTIPLIER_BITS = 0x3FC25E5374344960  # startup store 0x006D1BDD
MEM = 0x21000000
CELL_TABLE = MEM
CELLS = MEM + 0x100000
UNIT = MEM + 0x180000
UNIT_TYPE = MEM + 0x190000
TILE_REGISTRY = MEM + 0x1A0000
TILE_TYPES = MEM + 0x1A1000
TMP_HEADERS = MEM + 0x1C0000
TMP_CELLS = MEM + 0x1C8000
STACK = MEM + 0x1FFFF0
RETURN = 0x30000000
CURRENT = (2, 2)
CELL_STRIDE = 0x148
DEFAULT_CELL = {"level": 0, "ramp": 0, "flags": 0, "tile_index": 65535, "tmp_height": 30}
TARGETS = {
    "cliff_multiplier": 0x704240,
    "column_multiplier": 0x703E70,
    "tunnel_multiplier": 0x704000,
    "near_bridge": 0x703B10,
    "base_z_adjust": 0x704350,
    "foot_z_adjust": 0x4DAFC0,
}
REGIONS = {
    "direction_initializer": (0x49F2F0, 0x49F39C),
    "cliff_multiplier": (0x704240, 0x704343),
    "base_z_adjust": (0x704350, 0x7049BD),
    "foot_z_adjust": (0x4DAFC0, 0x4DB09F),
    "tmp_dimensions": (0x547150, 0x5471A3),
}


def signed(value: int) -> int:
    return ((value + 0x80000000) & 0xFFFFFFFF) - 0x80000000


class NativeFixture:
    def __init__(self) -> None:
        actual_hash = hashlib.sha256(GAMEMD.read_bytes()).hexdigest()
        if actual_hash != NATIVE_SHA256:
            raise RuntimeError(f"Unsupported gamemd SHA-256: {actual_hash}")
        self.uc = Uc(UC_ARCH_X86, UC_MODE_32)
        _load_image(self.uc)
        self.uc.mem_map(MEM, 0x200000)
        self.uc.mem_map(RETURN, 0x1000)
        self.region_hashes = {
            name: hashlib.sha256(bytes(self.uc.mem_read(start, end - start))).hexdigest()
            for name, (start, end) in REGIONS.items()
        }
        self.last_addresses: deque[int] = deque(maxlen=16)
        self.visits: Counter[int] = Counter()
        self.instruction_count = 0
        self.uc.hook_add(UC_HOOK_CODE, self.observe)
        # Execute the retail initializer instead of copying inferred directions.
        self.call(0x49F2F0)
        self.directions = list(struct.iter_unpack("<hh", bytes(self.uc.mem_read(0x89F688, 32))))
        self.put32(0x87F924, CELL_TABLE)  # MapClass cell-pointer vector
        self.put32(0x87F928, 512 * 512)
        self.put32(0xA8ED2C, TILE_REGISTRY)  # IsoTileType** backing array
        self.put32(0xAA10B0, 0)  # clear-tile index for 0xFF/0xFFFF fallback
        self.uc.mem_write(0xB0CD48, struct.pack("<Q", STANDARD_Z_MULTIPLIER_BITS))
        self.uc.mem_write(0x822D80, struct.pack("<H", NATIVE_FPCW))
        for tile_index in range(65):
            tile_type = TILE_TYPES + tile_index * 0x400
            header = TMP_HEADERS + tile_index * 0x100
            tile = TMP_CELLS + tile_index * 0x100
            self.put32(TILE_REGISTRY + tile_index * 4, tile_type)
            self.put32(tile_type, 0x7ECC48)  # actual IsoTileType virtual methods
            self.put32(tile_type + 0xA4, header)
            # Already-relocated one-cell TMP: +0x10 is an absolute cell pointer.
            for offset, value in [(0, 1), (4, 1), (8, 60), (12, 30), (16, tile)]:
                self.put32(header + offset, value)
        for y in range(8):
            for x in range(8):
                self.put32(CELL_TABLE + (y * 512 + x) * 4, self.cell_address(x, y))

    def put32(self, address: int, value: int) -> None:
        self.uc.mem_write(address, struct.pack("<I", value & 0xFFFFFFFF))

    @staticmethod
    def cell_address(x: int, y: int) -> int:
        if not (0 <= x < 8 and 0 <= y < 8):
            raise ValueError("This fixture covers only its mapped 8x8 ordinary cell grid")
        return CELLS + (y * 8 + x) * CELL_STRIDE

    def observe(self, _uc: Uc, address: int, _size: int, _data: object) -> None:
        self.last_addresses.append(address)
        self.visits[address] += 1
        self.instruction_count += 1

    def call(self, address: int, receiver: int = UNIT) -> int:
        self.last_addresses.clear()
        self.visits.clear()
        self.instruction_count = 0
        self.uc.mem_write(STACK - 0x1000, bytes(0x1004))
        self.put32(STACK, RETURN)
        for register in [UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_EBX,
                         UC_X86_REG_EBP, UC_X86_REG_ESI, UC_X86_REG_EDI]:
            self.uc.reg_write(register, 0)
        self.uc.reg_write(UC_X86_REG_ESP, STACK)
        self.uc.reg_write(UC_X86_REG_ECX, receiver)
        self.uc.reg_write(UC_X86_REG_EFLAGS, 2)
        self.uc.reg_write(UC_X86_REG_FPCW, NATIVE_FPCW)
        try:
            self.uc.emu_start(address, RETURN, count=100000)
        except Exception as error:
            trail = ", ".join(f"0x{value:08X}" for value in self.last_addresses)
            raise RuntimeError(f"Native 0x{address:08X} failed; final instructions: {trail}") from error
        if self.uc.reg_read(UC_X86_REG_EIP) != RETURN:
            raise RuntimeError(f"Native 0x{address:08X} did not return before instruction limit")
        return signed(self.uc.reg_read(UC_X86_REG_EAX))

    def configure(self, case: dict) -> None:
        self.uc.mem_write(UNIT, bytes(0x1000))
        self.uc.mem_write(UNIT_TYPE, bytes(0x1800))
        self.put32(UNIT, 0x7F5C70)  # actual Unit vtable (type/current cell/Z getters)
        self.put32(UNIT + 0x520, UNIT_TYPE)
        self.put32(UNIT + 0x6C4, UNIT_TYPE)
        context = case["context"]
        self.uc.mem_write(UNIT + 0x9C, struct.pack("<iii", 640, 640, context["world_z_leptons"]))
        self.uc.mem_write(UNIT + 0x8C, bytes([int(context["on_bridge"])]))
        self.uc.mem_write(UNIT + 0x388, struct.pack("<H", context["facing_u16"]))
        for name, offset in [("cliff", 0xDC0), ("column", 0xDC4), ("tunnel", 0xDC8), ("bridge", 0xDCC)]:
            self.put32(UNIT_TYPE + offset, case["coefficients"][name])
        self.put32(0xAA0E28, 0)  # supplied bridge tile range base for column cases
        overrides = {tuple(cell["coords"]): cell for cell in case["cells"]}
        for index in range(65):
            self.uc.mem_write(TMP_CELLS + index * 0x100, bytes(0x34))
        for y in range(8):
            for x in range(8):
                cell = DEFAULT_CELL | overrides.get((x, y), {})
                address = self.cell_address(x, y)
                self.uc.mem_write(address, bytes(CELL_STRIDE))
                self.put32(address, 0x7E4EEC)  # actual CellClass vtable
                self.uc.mem_write(address + 0x24, struct.pack("<hh", x, y))
                self.put32(address + 0x38, cell["tile_index"])
                self.put32(address + 0x44, -1)  # no overlay
                self.uc.mem_write(address + 0x116, struct.pack("<h", -1))  # no tube
                self.uc.mem_write(address + 0x11B, bytes([cell["level"] & 0xFF, cell["ramp"]]))
                self.put32(address + 0x140, cell["flags"])
                if cell["tmp_height"] != 30:
                    index = cell["tile_index"]
                    if not 0 <= index < 65:
                        raise ValueError("Non-default TMP needs a mapped tile index")
                    tile = TMP_CELLS + index * 0x100
                    self.put32(tile + 4, 100)  # stored Y
                    self.put32(tile + 0x18, 130 - cell["tmp_height"])  # raw extra Y
                    self.put32(tile + 0x24, 1)  # HasExtraData

    def execute(self, case: dict) -> dict:
        self.configure(case)
        values = {}
        for name, address in TARGETS.items():
            result = self.call(address)
            values[name] = bool(result & 0xFF) if name == "near_bridge" else result
        # Inspect the final full-Foot execution, not just the separate probes.
        for name in ["cliff_multiplier", "column_multiplier", "tunnel_multiplier", "base_z_adjust"]:
            if self.visits[TARGETS[name]] == 0:
                raise RuntimeError(f"Full Foot execution bypassed required native callee {name}")
        return values


def fixtures() -> list[dict]:
    cases = []

    def add(name: str, *, current: int = 0, first: int = 0, second: int = 0,
            cliff: int = 10, column: int = 5, tunnel: int = 10, bridge: int = 0,
            world_z: int = 0, facing: int = 0, on_bridge: bool = False,
            cells: list[dict] | None = None) -> None:
        overrides = {CURRENT: {"level": current}, (3, 3): {"level": first}, (4, 4): {"level": second}}
        for cell in cells or []:
            coords = tuple(cell["coords"])
            overrides[coords] = overrides.get(coords, {}) | cell
        cases.append({
            "name": name,
            "context": {"world_z_leptons": world_z, "facing_u16": facing, "on_bridge": on_bridge},
            "coefficients": {"cliff": cliff, "column": column, "tunnel": tunnel, "bridge": bridge},
            "cells": [DEFAULT_CELL | values | {"coords": list(coords)}
                      for coords, values in sorted(overrides.items())],
        })

    for first, second, name in [(0, 0, "neither"), (4, 0, "first"), (0, 4, "second"), (4, 4, "both"),
                                 (3, 0, "first_below_threshold"), (0, 3, "second_below_threshold"),
                                 (4, 3, "first_kept"), (3, 4, "second_only")]:
        add(f"stock_{name}", first=first, second=second)
    for current, first, second in [(-4, 0, -4), (-4, -4, 0), (-128, -124, -128),
                                    (127, -128, 127), (-128, 127, -128), (10, 13, 14)]:
        add(f"signed_levels_{current}_{first}_{second}", current=current, first=first, second=second)
    for coefficient in [0, 7, -7, 1073741824, 2147483647, -2147483648]:
        for first, second, probe in [(4, 0, "first"), (0, 4, "second")]:
            add(f"coefficient_{coefficient}_{probe}", first=first, second=second, cliff=coefficient)
    for world_z in [-832, -104, -1, 1, 104, 727, 728, 832]:
        add(f"world_z_{world_z}", first=4, world_z=world_z)
    for first, second, probe in [(4, 0, "first"), (0, 4, "second"), (4, 4, "both")]:
        add(f"on_bridge_{probe}", first=first, second=second, on_bridge=True)
    for direction in range(8):
        for height in [36, 37]:
            # (3,3) belongs to both cardinal triplets, but diagonal facings
            # skip this TMP-height branch entirely.
            add(f"facing_{direction}_tmp_{height}", first=4, facing=direction << 13,
                cells=[{"coords": [3, 3], "tile_index": 28, "tmp_height": height}])
    for direction in range(8):
        for position in [(1, 3), (3, 1)]:
            # SW and NE distinguish the two cardinal probe triplets.
            add(f"facing_{direction}_tall_at_{position[0]}_{position[1]}", first=4,
                facing=direction << 13, cells=[{"coords": list(position),
                    "tile_index": position[1] * 8 + position[0] + 1, "tmp_height": 37}])
    for facing in [4095, 4096, 12287, 12288, 61439, 61440, 65535]:
        add(f"facing_round_boundary_{facing}", first=4, facing=facing,
            cells=[{"coords": [3, 3], "tile_index": 28, "tmp_height": 37}])
    for coefficient in [7, 20, 37, -7]:
        add(f"bridge_max_{coefficient}", first=4, bridge=coefficient,
            cells=[{"coords": [2, 2], "flags": 0x100}])
    for coefficient in [7, 20, 37, -7]:
        add(f"column_max_{coefficient}", first=4, column=coefficient,
            cells=[{"coords": [2, 2], "flags": 0x100}, {"coords": [3, 3], "tile_index": 6}])
    add("bridge_on_bridge_gate", first=4, bridge=37, on_bridge=True,
        cells=[{"coords": [2, 2], "flags": 0x100}])
    return cases


def generate() -> dict:
    fixture = NativeFixture()
    cases = fixtures()
    for case in cases:
        case["native"] = fixture.execute(case)
    return {
        "schema_version": 1,
        "native_sha256": NATIVE_SHA256,
        "generator_normalized_lf_sha256": hashlib.sha256(Path(__file__).read_text().encode()).hexdigest(),
        "unicorn_version": unicorn_version,
        "scope": "Raw-function emulation of original Foot Z composition and its original helpers over supplied Unit/type/cell/TMP fixtures; not full-game or pixel parity.",
        "restrictions": [
            "Only the mapped ordinary 8x8 cell grid is exercised; no alias, dummy, invalid coordinate, or map-load equivalence claim.",
            "Actual Unit vtable; null locomotor, no transporter, harvester alternative, overlays, low bridges or active tubes.",
            "Ramp and cell-flag 0x10000 gates remain zero. SHP/voxel caller admission and active-game object lifecycle are outside the oracle.",
            "Native direction initializer runs; fixture supplies runtime type coefficients and relocated TMP metadata without executing their constructors/loaders.",
            "Code hooks only record instruction addresses. No executable instruction or function result is replaced.",
            "No image, disassembly, retail art or decoded pixel bytes are stored in this JSON.",
        ],
        "defaults": {
            "current_coords": list(CURRENT), "world_xy_leptons": [640, 640],
            "mapped_grid_width": 8, "mapped_grid_height": 8, "native_cell_index_stride": 512,
            "cell": DEFAULT_CELL, "locomotor_z_adjust": 0, "bridge_tile_base": 0,
            "adjust_for_z_multiplier_bits": f"{STANDARD_Z_MULTIPLIER_BITS:016x}",
            "fpcw": f"{NATIVE_FPCW:04x}", "directions": fixture.directions,
        },
        "entries": {name: f"{address:08x}" for name, address in TARGETS.items()},
        "region_sha256": fixture.region_hashes,
        "cases": cases,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path(__file__).with_name("render_depth_vectors.json"))
    parser.add_argument("--check", action="store_true", help="re-run native code and compare without writing")
    args = parser.parse_args()
    result = generate()
    if args.check:
        expected = json.loads(args.output.read_text())
        # JSON tuples become lists when read; normalize before comparison.
        if json.loads(json.dumps(result)) != expected:
            raise SystemExit("FAIL: native Foot Z vectors differ from the saved fixture")
        print(f"PASS: {len(result['cases'])} native Foot Z fixtures reproduce")
    else:
        args.output.write_text(json.dumps(result, indent=2) + "\n")
        print(f"Wrote {len(result['cases'])} native Foot Z fixtures to {args.output}")


if __name__ == "__main__":
    main()
