"""Execute original bridge OverlayClass constructor and its reached callees.

Supplied sparse cells, registered types and preallocated registry capacity are
explicit fixture inputs. Original instructions/calls are never patched.
"""
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EIP

from tools.native_oracle import (
    load_image, run_checked, STACK_BASE, STACK_SIZE, RET_MAGIC,
    finish_vectors, provenance,
)
from tools.spatial_oracle.map_queries import dwords, packed
from tools.spatial_oracle.terrain_recalc import LAT_GLOBALS

D = 0x44000000
REGISTRIES = (0xA8E360, 0xB0F720, 0xB0F670, 0xB0F618, 0xA8EC50)
MAP, DUMMY = 0x87F7E8, 0xABDC50


class OriginalBridgeConstructor:
    def __init__(self, kind, overlay_id):
        self.kind, self.overlay_id = kind, overlay_id
        self.uc = u = Uc(UC_ARCH_X86, UC_MODE_32)
        load_image(u)
        u.mem_map(D, 0x200000)
        u.mem_map(STACK_BASE, STACK_SIZE)
        u.mem_map(RET_MAGIC, 0x1000)
        self.obj, typ = D + 0x1000, D + 0x2000
        terrain, terrain_vtable = D + 0x3000, D + 0x4000
        self.scenario, self.coord = D + 0x5000, D + 0x6000
        table, cells = D + 0x100000, D + 0x10000
        self.ptrs = {(x, y): cells + i * 0x200 for i, (x, y) in enumerate(
            (x, y) for y in range(12, 21) for x in range(12, 21))}
        for (x, y), cell in self.ptrs.items():
            u.mem_write(table + (y * 512 + x) * 4, dwords(cell))
            u.mem_write(cell + 0x24, packed(x, y))
            u.mem_write(cell + 0x44, dwords(-1))
            u.mem_write(cell + 0x11B, b'\x06')
        self.cell, self.typ = self.ptrs[16, 16], typ
        u.mem_write(MAP + 0x13C, dwords(table, 0x40000))
        u.mem_write(MAP + 0xF4, dwords(16, 16, 0, 0, 16, 16))
        u.mem_write(DUMMY, bytes(0x200))
        u.mem_write(DUMMY + 0x38, dwords(0xffff))
        u.mem_write(DUMMY + 0x44, dwords(-1))
        for n, registry in enumerate(REGISTRIES):
            # Actual startup vtable stores:4E7AFD,7252ED,72536D,7253ED,4E68FD.
            u.mem_write(registry, dwords((0x7E4F64, 0x7E91EC, 0x7E91EC, 0x7E91EC, 0x7E9D24)[n]))
            u.mem_write(registry + 4, dwords(D + 0x30000 + n * 0x100, 16))
            u.mem_write(registry + 16, dwords(0))
        u.mem_write(0xB0F69C, dwords(D + 0x31000, 16))
        u.mem_write(0xB0F698, dwords(0x7E91EC))  # Startup72586D.
        u.mem_write(0xB0F6A8, dwords(0))
        u.mem_write(0xA8B230, dwords(self.scenario))
        u.mem_write(self.scenario + 0x214, dwords(1000))
        u.mem_write(0xA8E9A0, b'\x01')
        u.mem_write(0xA8E7AC, dwords(0))
        u.mem_write(self.coord, packed(16, 16))
        u.mem_write(typ, dwords(0x7EF600))
        u.mem_write(typ + 0x294, dwords(overlay_id))
        self.required = [0x5F3900, 0x410230, 0x68BCB0, 0x47C550]
        if kind == 'terrain':
            u.mem_write(self.cell + 0xE4, dwords(terrain))
            u.mem_write(terrain, dwords(terrain_vtable))
            u.mem_write(terrain_vtable + 0x2C, dwords(0x71D300))
            self.required += [0x71D300]
        elif kind == 'slope':
            u.mem_write(self.cell + 0x11C, b'\x05')
            self.required += [0x5F4EC0, 0x5FC570, 0x5FC5E3, 0x5F521C]
        else:
            assert kind in ('success', 'overrides')
            self.prepare_success(kind)

    def call(self, entry, receiver=0, args=(), required=()):
        u = self.uc
        sp = STACK_BASE + STACK_SIZE - 0x1000
        u.mem_write(sp, dwords(RET_MAGIC, *args))
        u.reg_write(UC_X86_REG_ESP, sp)
        u.reg_write(UC_X86_REG_ECX, receiver)
        run_checked(u, entry, RET_MAGIC, count=100000, required_addresses=required)
        return u.reg_read(UC_X86_REG_EAX)

    def prepare_success(self, kind):
        u = self.uc
        # Original direction initialization; no supplied direction table.
        self.call(0x49F2F0)
        rules, overlays, old = D + 0x40000, D + 0x44000, D + 0x45000
        head, tmp, tiles = D + 0x46000, D + 0x47000, D + 0x48000
        zones, levels = D + 0x50000, D + 0x60000
        # Startup40B563 initializes this empty listener owner's vector vtable.
        u.mem_write(0x87F5D8, dwords(0x7E17CC))
        u.mem_write(0x8871E0, dwords(rules))
        u.mem_write(0xA83D84, dwords(overlays))
        u.mem_write(overlays + self.overlay_id * 4, dwords(self.typ))
        u.mem_write(overlays + 7 * 4, dwords(old))
        u.mem_write(old + 0x294, dwords(7))
        u.mem_write(old + 0x2B2, b'\x01')
        if kind == 'overrides':
            u.mem_write(self.cell + 0x44, dwords(7))
        u.mem_write(MAP + 0x68, dwords(zones, 33 * 33, levels))
        u.mem_write(head, dwords(0x7ECC48))
        u.mem_write(head + 0xA4, dwords(tmp))
        u.mem_write(head + 0x2C8, dwords(-1))
        u.mem_write(head + 0x2D4, dwords(-1))
        u.mem_write(tmp, dwords(1, 1, 8, 4, tmp + 20))
        u.mem_write(tmp + 60, bytes((3, 11, 0)))
        u.mem_write(0xA8ED2C, dwords(tiles))
        u.mem_write(0xA8ED38, dwords(1))
        u.mem_write(tiles, dwords(head))
        for address in LAT_GLOBALS:
            u.mem_write(address, dwords(-1))
        for address in (0x89EA48, 0x89EA48 + 36):
            u.mem_write(address, struct.pack('<f', 1.0))
        self.required += [0x5F4EC0, 0x5FC570, 0x47D2B0, 0x5FD216]

    def read_u32(self, address):
        return struct.unpack('<I', self.uc.mem_read(address, 4))[0]

    def run(self):
        returned = self.call(0x5FC380, self.obj, (self.typ, self.coord, -1), self.required)
        assert returned == self.obj
        u = self.uc
        b = u.mem_read(self.obj, 0xB0)
        result = dict(kind=self.kind, overlay_id=self.overlay_id,
                    native_id=struct.unpack_from('<I', b, 0x10)[0],
                    cursor=self.read_u32(self.scenario + 0x214),
                    alive=b[0x90], limbo=b[0x81], on_map=b[0x74], redraw=b[0x80],
                    registry_counts=[self.read_u32(r + 16) for r in REGISTRIES],
                    queue_count=self.read_u32(0xB0F6A8),
                    object_world=struct.unpack_from('<3i', b, 0x9C),
                    overlay=struct.unpack('<i', u.mem_read(self.cell + 0x44, 4))[0])
        def cells():
            rows = []
            for coord, pointer in self.ptrs.items():
                flags = self.read_u32(pointer + 0x140)
                anchor = self.read_u32(pointer + 0x2C)
                overlay = struct.unpack('<i', u.mem_read(pointer + 0x44, 4))[0]
                state = bytes(u.mem_read(pointer + 0x11E, 1))[0]
                if flags or anchor or overlay != -1 or state:
                    rows.append(dict(coord=coord, flags=flags, overlay=overlay, state=state,
                                     anchor=next((c for c, p in self.ptrs.items() if p == anchor), None)))
            return rows
        result['cells'] = before_cells = cells()
        frees = []
        # Original typeid uses Windows SEH and IsBadReadPtr during finalization.
        # Supply those OS facilities after the complete constructor has returned.
        u.mem_map(0, 0x1000)
        u.mem_write(0, dwords(-1))
        pointer_probe = RET_MAGIC + 0x100
        u.mem_write(0x7E115C, dwords(pointer_probe))
        def external_services(_u, address, _size, _data):
            if address == pointer_probe:
                sp = u.reg_read(UC_X86_REG_ESP)
                ret, pointer, length = struct.unpack('<3I', u.mem_read(sp, 12))
                u.mem_read(pointer, length)  # Validate every supplied success.
                u.reg_write(UC_X86_REG_EAX, 0)
                u.reg_write(UC_X86_REG_ESP, sp + 12)
                u.reg_write(UC_X86_REG_EIP, ret)
                return
            if address != 0x7C8B3D:
                return
            sp = u.reg_read(UC_X86_REG_ESP)
            ret, pointer = struct.unpack('<2I', u.mem_read(sp, 8))
            assert pointer == self.obj
            frees.append('overlay')
            u.reg_write(UC_X86_REG_ESP, sp + 4)
            u.reg_write(UC_X86_REG_EIP, ret)
        u.hook_add(UC_HOOK_CODE, external_services)
        self.call(0x725C70, required=(0x5FDF70, 0x5F3B80) if result['queue_count'] else ())
        result['after_drain'] = dict(registry_counts=[self.read_u32(r + 16) for r in REGISTRIES],
                                    queue_count=self.read_u32(0xB0F6A8), frees=frees,
                                    cursor=self.read_u32(self.scenario + 0x214), cells=cells())
        assert result['after_drain']['cells'] == before_cells
        return result


def cases():
    return {'cases': [OriginalBridgeConstructor(kind, overlay).run()
                     for kind in ('terrain', 'slope', 'success', 'overrides')
                     for overlay in (24, 25, 237, 238)]}


if __name__ == '__main__':
    finish_vectors(cases, Path(__file__).with_suffix('.json'), provenance=lambda: provenance(
        scope='Original5FC380 bridge constructor over supplied sparse live map and type inputs',
        assumptions=[
            'Raw IDs24/25/237/238, frame-1; Terrain reject, slope5 reject, success and existing Overrides paths only',
            'One constructor per fresh registry/queue fixture; direct drain, not MainTick admission or mixed-queue ordering',
            'Supplied allocated object memory, empty registry/queue capacity16, Scenario cursor1000 and active game',
            'Sparse real9x9 cells centered16,16; source terrain type construction excluded',
            'Terrain list record uses original TerrainClass RTTI71D300',
            'Selected OverlayType uses original vtable with supplied raw ID and otherwise zero fields',
            'Success Recalc receives resident synthetic1x1 Road TMP; no native loader, artwork or whole repair claim',
            'Original startup vector vtables supplied with empty preallocated storage; other pointer-expiration recipients absent',
            'Empty Windows SEH chain supplied for original typeid during the post-constructor drain',
        ], substitutions=['7C8B3D external free and Windows IsBadReadPtr transport only during the post-constructor drain; no constructor call is substituted'],
        entry_points={'constructor': 0x5FC380, 'mark': 0x5FC570, 'drain': 0x725C70,
                      'overlay_destructor': 0x5FDF70, 'base_destructor': 0x5F3B80}))
