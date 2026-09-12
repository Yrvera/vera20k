"""Original saved-game resources plus full 60B7A0 ordinary child placement.

558DD0 selects B7/2B4/2B5 for its saved-game receiver modes 1/2/3. Active
60C0C0 enrolls their ordinary list/prompt/edit controls in 60B7A0. The sibling
oracle supplies bounded Win32 hooks and executes that complete original body.
"""

from pathlib import Path
import hashlib
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from tools.native_oracle import finish_vectors, load_image, provenance
from tools.storage_oracle.in_game_shell_geometry import native_child_case


RESOURCES = ((0xB7, 0xBEFCF8, 340, 6),
             (0x2B4, 0xC012F4, 364, 7),
             (0x2B5, 0xC01460, 348, 6))


def resource(dialog, address, size, expected_count):
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(uc)
    data = bytes(uc.mem_read(address, size))
    style, exstyle, count, *bounds = struct.unpack_from("<IIH4h", data)
    if (style, exstyle, count, bounds) != (
            0x40000040, 0, expected_count, [0, 0, 533, 369]):
        raise RuntimeError("Unexpected original saved-game resource header")
    offset = 18

    def variable():
        nonlocal offset
        start = offset
        word = struct.unpack_from("<H", data, offset)[0]
        if word == 0xFFFF:
            offset += 4
            return struct.unpack_from("<H", data, start + 2)[0]
        while struct.unpack_from("<H", data, offset)[0]:
            offset += 2
        offset += 2
        return data[start:offset - 2].decode("utf-16-le")

    for _ in range(3):
        variable()
    point_size = struct.unpack_from("<H", data, offset)[0]
    offset += 2
    font = variable()
    if (point_size, font) != (8, "MS Sans Serif"):
        raise RuntimeError("Unexpected original saved-game resource font")
    controls = []
    for _ in range(count):
        offset = (offset + 3) & ~3
        child_style, child_exstyle, x, y, w, h, control = struct.unpack_from(
            "<II4hH", data, offset)
        offset += 18
        class_id, caption = variable(), variable()
        extra = struct.unpack_from("<H", data, offset)[0]
        offset += 2 + extra
        controls.append({"control_id": control, "style": child_style,
                         "exstyle": child_exstyle, "class": class_id,
                         "caption": caption, "dlu_rect": [x, y, w, h]})
    if offset > size or any(data[offset:]):
        raise RuntimeError("Unparsed saved-game resource bytes")
    return {"dialog_id": dialog, "address": address, "size": size,
            "sha256": hashlib.sha256(data).hexdigest(), "hex": data.hex(),
            "controls": controls}


def generate():
    resources = [resource(*args) for args in RESOURCES]
    cases = []
    for item in resources:
        for child in item["controls"]:
            if child["control_id"] not in (0x40C, 0x525, 0x526, 0x527, 0x528):
                continue
            x, y, w, h = child["dlu_rect"]
            pixels = [(x * 6 + 2) // 4, (y * 13 + 4) // 8,
                      (w * 6 + 2) // 4, (h * 13 + 4) // 8]
            for width, height in ((640, 480), (800, 600), (1024, 768)):
                case = native_child_case(child["control_id"], pixels, width, height)
                case["dialog_id"] = item["dialog_id"]
                case["dlu_rect"] = child["dlu_rect"]
                cases.append(case)
    if len(cases) != 21:
        raise RuntimeError("Missing ordinary saved-game geometry cases")
    return {"source": "unicorn/gamemd.exe", "resources": resources, "cases": cases}


if __name__ == "__main__":
    finish_vectors(generate, Path(__file__).with_suffix(".json"),
                   provenance=lambda: provenance(
        scope="Original B7/2B4/2B5 resources and 21 complete 60B7A0 ordinary child placements at three resolutions",
        assumptions=[
            "Pinned original resource addresses independently checked through PE RT_DIALOG tree; raw bytes and SHA256 retained",
            "Supplied retail 6x13 dialog base units map DLU through MulDiv rounding; Windows font metrics not emulated",
            "60C0C0 active dispatcher and 608500/608CD0 exclusions established by instruction inspection; only complete 60B7A0 is executed",
            "Parent origin0 with extent640x480,800x600,1024x768; original base800/600 globals remain untouched",
            "Title, right-button and footer placement, painting, list input and persistence are outside this fixture",
        ],
        substitutions=[
            "Reuse native_child_case hooks for supplied GetParent/GetWindowRect/MoveWindow and true69BBE0; original placement instructions unchanged",
        ],
        entry_points={"browser": 0x558DD0, "dispatcher": 0x60C0C0,
                      "ordinary_child": 0x60B7A0},
    ))
