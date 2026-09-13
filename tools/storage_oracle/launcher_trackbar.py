"""Bounded original shell trackbar projection and pointer arithmetic.

Active owner 55FC80 supplies D5/proc 55FDB0 at 55FCB3..55FCC1. That proc
sends ranges 1/2/6/10 (message 406) and positions (405) at 560139..560463.
Original RT_DIALOG D5/language 1033 declares that trackbar class for controls
52B/50F/52A (120x13 DLU) and 52F/532/536 (85x13 DLU).
Common control initialization selects 61D950 for the original ASCII class
msctls_trackbar32 (835848) at 60FC76..60FCB9; the selected handler is retained
at 60FF70 behind subclass dispatcher 610CA0. The first three launcher controls
disable the 50px plaque with 4AC; the three audio controls retain it.
Active in-game BBB owner 4E1FE0 sends 4AC=0 at 4E207F..4E2089 and
4E2128..4E2132 for GameSpeed/ScrollRate, then sets range0..6. Their resource
128x13 DLU gives the additional ordinary 192x21 plain-rail geometry.

These fixtures execute original interior blocks, not a Windows dialog. Supplied
client RECTs start at (0, 0), range minimum is zero, step is one and maximum is
positive. We bypass HWND lookup, OS pointer retrieval, capture admission, paint,
notifications and audio. Initial/state-set/drag projection is sampled separately;
only the original instructions produce output coordinates. No code is patched.
"""

from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EBP, UC_X86_REG_EBX,
    UC_X86_REG_ESI, UC_X86_REG_ESP,
)

from tools.native_oracle import (
    STACK_BASE, STACK_SIZE, finish_vectors, load_image, provenance, run_checked,
)


class TrackbarFixture:
    def __init__(self):
        self.uc = Uc(UC_ARCH_X86, UC_MODE_32)
        load_image(self.uc)
        self.uc.mem_map(STACK_BASE, STACK_SIZE)
        self.stack = STACK_BASE + STACK_SIZE - 0x1000
        self.registers = self.uc.context_save()

    def put(self, offset, value):
        self.uc.mem_write(self.stack + offset, struct.pack("<i", value))

    def get(self, offset):
        return struct.unpack("<i", self.uc.mem_read(self.stack + offset, 4))[0]

    def prepare(self, width, reserve, maximum):
        # Restore registers and every stack slot read by the sampled blocks.
        self.uc.context_restore(self.registers)
        self.uc.mem_write(self.stack, bytes(0x200))
        self.uc.reg_write(UC_X86_REG_ESP, self.stack)
        self.uc.reg_write(UC_X86_REG_EBP, maximum)
        self.uc.reg_write(UC_X86_REG_ESI, 0)  # Native range minimum.
        self.put(0x1C, 1)  # Native step.
        self.put(0x20, reserve)
        self.put(0xA8, width)  # Client right; client left +0xA0 is zero.
        # Execute the original usable-span calculation and minimum clamp.
        run_checked(self.uc, 0x61DA52, 0x61DA7D, count=30,
                    required_addresses=[0x61DA68, 0x61DA6B])

    def paint_bounds(self):
        # Project stored +0x18 pixel offset to the native half-open thumb RECT.
        run_checked(self.uc, 0x61DBB9, 0x61DBD6, count=20,
                    required_addresses=[0x61DBC4, 0x61DBCF])
        return self.get(0x84), self.get(0x28)

    def position(self, width, reserve, maximum, position, path):
        self.prepare(width, reserve, maximum)
        self.uc.reg_write(UC_X86_REG_EBX, position)
        if path == "set_position":
            self.put(0x160, position)  # 405 lParam; EAX retains usable span.
            run_checked(self.uc, 0x61E486, 0x61E4A8, count=30,
                        required_addresses=[0x61E497, 0x61E499, 0x61E49D])
        elif path == "set_range":
            self.put(0x160, maximum << 16)  # 406 MAKELONG(0, maximum).
            run_checked(self.uc, 0x61E59A, 0x61E5C9, count=30,
                        required_addresses=[0x61E5AC, 0x61E5BA, 0x61E5BE])
        elif path == "initial":
            # EBX models the position returned by original TBM_GETPOS;
            # ESI/EBP model the preceding native minimum/range queries.
            run_checked(self.uc, 0x61DB40, 0x61DB52, count=20,
                        required_addresses=[0x61DB46, 0x61DB4A, 0x61DB4E])
        else:
            raise ValueError(path)
        return self.paint_bounds()

    def pointer(self, width, reserve, maximum, x, path):
        self.prepare(width, reserve, maximum)
        if path == "drag":
            self.put(0x64, x)  # Client x after GetCursorPos/ScreenToClient.
            run_checked(self.uc, 0x61DC00, 0x61DC6D, count=70,
                        required_addresses=[0x61DC2B, 0x61DC2F,
                                            0x61DC54, 0x61DC58])
        elif path == "rail_click":
            # EAX is client x after LPARAM unpacking and y/thumb admission.
            # Only in-client nonnegative x values take this comparison path.
            self.uc.reg_write(UC_X86_REG_EAX, x)
            run_checked(self.uc, 0x61E545, 0x61E598, count=70,
                        required_addresses=[0x61E56C, 0x61E570,
                                            0x61E58E, 0x61E592])
        else:
            raise ValueError(path)
        value = self.uc.reg_read(UC_X86_REG_EBX)
        return (value, *self.paint_bounds())


def generate():
    fixture = TrackbarFixture()
    geometries = []
    # Preserve the original sixteen launcher arithmetic fixtures verbatim and
    # append ordinary BBB plain192/range6 and B8 numeric263/range10 controls.
    inputs = [(width, reserve, maximum)
              for width in (128, 180) for reserve in (0, 50)
              for maximum in (1, 2, 6, 10)]
    inputs.extend(((192, 0, 6), (263, 50, 10)))
    for width, reserve, maximum in inputs:
        positions = []
        for position in range(maximum + 1):
            bounds = fixture.position(width, reserve, maximum, position,
                                      "set_position")
            for path in ("set_range", "initial"):
                other = fixture.position(width, reserve, maximum,
                                         position, path)
                if other != bounds:
                    raise RuntimeError(f"Native {path} disagrees: "
                                       f"{width, reserve, maximum, position}")
            positions.append({"position": position,
                              "thumb_left": bounds[0],
                              "thumb_right": bounds[1]})
        pointers = []
        for x in range(-8, width + 9):
            value, left, right = fixture.pointer(width, reserve, maximum,
                                                x, "drag")
            if 0 <= x < width:
                click = fixture.pointer(width, reserve, maximum,
                                        x, "rail_click")
                if click != (value, left, right):
                    raise RuntimeError(f"Native click/drag disagree: "
                                       f"{width, reserve, maximum, x}")
            pointers.append({"x": x, "position": value,
                             "thumb_left": left, "thumb_right": right})
        geometries.append({"width": width, "reserve": reserve,
                           "maximum": maximum, "positions": positions,
                           "pointers": pointers})
    return {"source": "unicorn/gamemd.exe", "geometries": geometries}


if __name__ == "__main__":
    finish_vectors(generate, Path(__file__).with_suffix(".json"),
                   provenance=lambda: provenance(
        scope=("18 supplied geometries; 110 valid positions compared across "
               "initialization/405/406; 3225 drag pointer samples, including "
               "2919 in-client samples compared with the admitted rail-click block"),
        assumptions=[
            "Client RECT left/top zero; widths 128 and 180, plaque reserve 0 or 50; these cross combinations are arithmetic fixtures, not observed layouts",
            "Additional BBB fixture is width192/reserve0/maximum6, from 128x13 DLU and the active 4E1FE0 configuration; height21 does not enter these arithmetic blocks",
            "B8 adds width263/reserve50/maximum10 from175x13 DLU;6B6382..647F disables cue but retains the default numeric plaque",
            "Range minimum zero, maximum 1/2/6/10, step one, every valid position; no invalid setter or zero-range claims",
            "Client x is every integer from -8 through width+8; negative/overshoot samples model capture drag, not admitted rail clicks",
            "The admitted rail-click block is compared for x in [0,width); y/old-thumb admission, HWND state lookup, Windows pointer retrieval and notifications are outside the fixture",
            "Registers and stack are reset before each block sequence; native span arithmetic supplies EAX and stack+0x10; original signed integer division executes",
            "Thumb bounds are read after native stored-offset projection; pixel appearance, repaint timing and whole-dialog equivalence are not established",
        ],
        substitutions=[],
        entry_points={
            "span": 0x61DA52, "span_end": 0x61DA7D,
            "initial": 0x61DB40, "initial_end": 0x61DB52,
            "set_position": 0x61E486, "set_position_end": 0x61E4A8,
            "set_range": 0x61E59A, "set_range_end": 0x61E5C9,
            "drag": 0x61DC00, "drag_end": 0x61DC6D,
            "rail_click": 0x61E545, "rail_click_end": 0x61E598,
            "thumb_bounds": 0x61DBB9, "thumb_bounds_end": 0x61DBD6,
        },
    ))
