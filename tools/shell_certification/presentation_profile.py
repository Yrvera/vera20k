"""Derive an immutable RGB565 presentation profile from sealed shell evidence.

The profile is deliberately scoped to the enrolled guard environment. This
module never edits the guard or its Oracle source runs and never overwrites an
existing output.
"""

from __future__ import annotations

import stat
from pathlib import Path
from typing import Any, Mapping

from .core import (
    GUARD_ID,
    GUARD_SCHEMA_VERSION,
    GUARD_STATE,
    SEALED_MAIN_MENU_GUARD_SHA256,
    OutputExistsError,
    ValidationError,
    _parse_json_bytes,
    _read_regular_bytes,
    _require_array,
    _require_keys,
    _require_object,
    _require_sha256,
    _require_string,
    absolute_path,
    sha256_bytes,
    utc_now,
    validate_guard,
    write_json_exclusive,
)


PROFILE_SCHEMA_VERSION = "vera20k.shell-presentation-profile.v1"
PROFILE_CHECKPOINT = "main-menu-0xe2-rgb565-presentation"
SOURCE_COUNT = 3
BYTES_PER_PIXEL = 4
EXPECTED_BLUE_CARDINALITY = 32
EXPECTED_GREEN_CARDINALITY = 64
EXPECTED_RED_CARDINALITY = 32
EXPECTED_ALPHA_VALUES = (255,)
EXPECTED_SURFACE_FORMAT = "B8G8R8A8_UNORM"
MAX_GUARD_BYTES = 4 * 1024 * 1024


def derive_channel_codebooks(
    frame: bytes, width: int, height: int
) -> tuple[tuple[int, ...], tuple[int, ...], tuple[int, ...], tuple[int, ...]]:
    """Return sorted B, G, R, A sets after exact BGRA length validation."""

    if type(width) is not int or type(height) is not int:
        raise ValidationError("frame width and height must be integers")
    if width <= 0 or height <= 0:
        raise ValidationError("frame width and height must be positive")
    expected_length = width * height * BYTES_PER_PIXEL
    if len(frame) != expected_length:
        raise ValidationError(
            f"source frame byte length is {len(frame)}, expected {expected_length}"
        )
    return (
        tuple(sorted(set(frame[0::BYTES_PER_PIXEL]))),
        tuple(sorted(set(frame[1::BYTES_PER_PIXEL]))),
        tuple(sorted(set(frame[2::BYTES_PER_PIXEL]))),
        tuple(sorted(set(frame[3::BYTES_PER_PIXEL]))),
    )


def _is_link(path: Path) -> bool:
    """Recognize both symbolic links and Windows directory junctions."""

    try:
        if path.is_symlink():
            return True
        is_junction = getattr(path, "is_junction", None)
        return bool(is_junction and is_junction())
    except OSError as exc:
        raise ValidationError(f"cannot inspect path for links {path}: {exc}") from exc


def _require_oracle_runs_root(path: Path) -> Path:
    root = absolute_path(path)
    if _is_link(root):
        raise ValidationError(f"Oracle runs root must not be a link: {root}")
    try:
        metadata = root.stat()
    except FileNotFoundError as exc:
        raise ValidationError(f"Oracle runs root does not exist: {root}") from exc
    except OSError as exc:
        raise ValidationError(f"cannot stat Oracle runs root {root}: {exc}") from exc
    if not stat.S_ISDIR(metadata.st_mode):
        raise ValidationError(f"Oracle runs root is not a directory: {root}")
    return root


def _relative_guard_path(value: Any, field: str, *, one_component: bool) -> Path:
    text = _require_string(value, field)
    if not text:
        raise ValidationError(f"{field} must not be empty")
    path = Path(text)
    parts = path.parts
    if (
        path.is_absolute()
        or bool(path.drive)
        or not parts
        or any(part in ("", ".", "..") for part in parts)
    ):
        raise ValidationError(f"{field} must be a traversal-free relative path")
    if one_component and len(parts) != 1:
        raise ValidationError(f"{field} must be exactly one path component")
    return path


def _validate_source_path(root: Path, relative: Path, field: str) -> Path:
    """Reject links and prove an existing source remains beneath the runs root."""

    candidate = root / relative
    cursor = root
    for component in relative.parts:
        cursor = cursor / component
        if _is_link(cursor):
            raise ValidationError(f"{field} must not traverse a link: {cursor}")

    try:
        resolved_root = root.resolve(strict=True)
        resolved_candidate = candidate.resolve(strict=True)
    except FileNotFoundError as exc:
        raise ValidationError(f"{field} does not exist: {candidate}") from exc
    except OSError as exc:
        raise ValidationError(f"cannot resolve {field} {candidate}: {exc}") from exc
    try:
        resolved_candidate.relative_to(resolved_root)
    except ValueError as exc:
        raise ValidationError(
            f"{field} escapes the Oracle runs root: {candidate}"
        ) from exc
    return candidate


def _load_guard_document(
    guard_path: Path, validated_sha256: str
) -> Mapping[str, Any]:
    raw = _read_regular_bytes(
        guard_path, "native shell guard", maximum_length=MAX_GUARD_BYTES
    )
    actual_sha256 = sha256_bytes(raw)
    if actual_sha256 != validated_sha256:
        raise ValidationError(
            "native shell guard changed after validation: "
            f"got {actual_sha256}, expected {validated_sha256}"
        )
    return _parse_json_bytes(raw, "native shell guard")


def _source_records(document: Mapping[str, Any]) -> tuple[Mapping[str, Any], ...]:
    values = _require_array(document.get("sources"), "guard.sources")
    if len(values) != SOURCE_COUNT:
        raise ValidationError(
            f"guard.sources has {len(values)} entries, expected {SOURCE_COUNT}"
        )
    records: list[Mapping[str, Any]] = []
    for index, value in enumerate(values):
        field = f"guard.sources[{index}]"
        source = _require_object(value, field)
        _require_keys(
            source,
            (
                "run_id",
                "frame_blob",
                "frame_pixel_sha256",
                "surface_blob",
                "surface_pixel_sha256",
                "environment_identity_sha256",
            ),
            field,
        )
        records.append(source)
    return tuple(records)


def _surface_format(document: Mapping[str, Any]) -> str:
    environment = _require_object(
        document.get("environment_identity"), "guard.environment_identity"
    )
    presentation = _require_object(
        environment.get("presentation_configuration"),
        "guard.environment_identity.presentation_configuration",
    )
    surface_format = _require_string(
        presentation.get("surface_format"),
        "guard.environment_identity.presentation_configuration.surface_format",
    )
    if surface_format != EXPECTED_SURFACE_FORMAT:
        raise ValidationError(
            "guard presentation surface format is "
            f"{surface_format!r}, expected {EXPECTED_SURFACE_FORMAT!r}"
        )
    return surface_format


def _require_cardinalities(
    blue: tuple[int, ...],
    green: tuple[int, ...],
    red: tuple[int, ...],
    alpha: tuple[int, ...],
    field: str,
) -> None:
    cardinalities = (len(blue), len(green), len(red))
    expected = (
        EXPECTED_BLUE_CARDINALITY,
        EXPECTED_GREEN_CARDINALITY,
        EXPECTED_RED_CARDINALITY,
    )
    if cardinalities != expected:
        raise ValidationError(
            f"{field} B/G/R cardinalities are {cardinalities}, expected {expected}"
        )
    if alpha != EXPECTED_ALPHA_VALUES:
        raise ValidationError(
            f"{field} alpha values are {alpha}, expected {EXPECTED_ALPHA_VALUES}"
        )


def _merge_source_codebooks(
    expected: tuple[tuple[int, ...], tuple[int, ...], tuple[int, ...]] | None,
    observed: tuple[tuple[int, ...], tuple[int, ...], tuple[int, ...]],
) -> tuple[tuple[int, ...], tuple[int, ...], tuple[int, ...]]:
    if expected is not None and observed != expected:
        raise ValidationError("guard source B/G/R codebooks disagree")
    return observed if expected is None else expected


def derive_presentation_profile(
    guard_path: Path,
    oracle_runs: Path,
    *,
    expected_guard_sha256: str = SEALED_MAIN_MENU_GUARD_SHA256,
) -> dict[str, object]:
    """Validate the sealed guard and all guarded source frames, then report."""

    validated_guard = validate_guard(
        guard_path, expected_sha256=expected_guard_sha256
    )
    document = _load_guard_document(
        validated_guard.path, validated_guard.sha256
    )
    source_records = _source_records(document)
    surface_format = _surface_format(document)
    runs_root = _require_oracle_runs_root(oracle_runs)

    expected_codebooks: tuple[
        tuple[int, ...], tuple[int, ...], tuple[int, ...]
    ] | None = None
    environment_identity_sha256: str | None = None
    observed_run_ids: set[str] = set()
    observed_blob_paths: set[str] = set()
    output_sources: list[dict[str, str]] = []

    for index, source in enumerate(source_records):
        field = f"guard.sources[{index}]"
        run_path = _relative_guard_path(
            source["run_id"], f"{field}.run_id", one_component=True
        )
        surface_blob_text = _require_string(
            source["surface_blob"], f"{field}.surface_blob"
        )
        blob_path = _relative_guard_path(
            surface_blob_text, f"{field}.surface_blob", one_component=False
        )
        frame_blob_path = _relative_guard_path(
            source["frame_blob"], f"{field}.frame_blob", one_component=False
        )
        if frame_blob_path != blob_path:
            raise ValidationError(f"{field} frame_blob and surface_blob disagree")

        run_id = str(run_path)
        surface_blob = surface_blob_text
        source_identity = str(run_path / blob_path)
        if run_id in observed_run_ids:
            raise ValidationError(f"guard has duplicate source run_id: {run_id}")
        if source_identity in observed_blob_paths:
            raise ValidationError(
                f"guard has duplicate source path: {source_identity}"
            )
        observed_run_ids.add(run_id)
        observed_blob_paths.add(source_identity)

        expected_surface_sha256 = _require_sha256(
            source["surface_pixel_sha256"],
            f"{field}.surface_pixel_sha256",
        )
        expected_frame_sha256 = _require_sha256(
            source["frame_pixel_sha256"], f"{field}.frame_pixel_sha256"
        )
        if expected_frame_sha256 != expected_surface_sha256:
            raise ValidationError(
                f"{field} frame and presentation-surface pixel digests disagree"
            )

        source_environment_sha256 = _require_sha256(
            source["environment_identity_sha256"],
            f"{field}.environment_identity_sha256",
        )
        if environment_identity_sha256 is None:
            environment_identity_sha256 = source_environment_sha256
        elif source_environment_sha256 != environment_identity_sha256:
            raise ValidationError(
                "guard source environment identity digests disagree"
            )

        relative_source = run_path / blob_path
        source_path = _validate_source_path(
            runs_root, relative_source, f"{field} source frame"
        )
        expected_length = (
            validated_guard.presentation_width
            * validated_guard.presentation_height
            * BYTES_PER_PIXEL
        )
        frame = _read_regular_bytes(
            source_path,
            f"{field} source frame",
            exact_length=expected_length,
        )
        # Recheck the ancestry after the stable file read so a changed link
        # cannot silently redirect the next source or the recorded identity.
        _validate_source_path(
            runs_root, relative_source, f"{field} source frame"
        )
        actual_sha256 = sha256_bytes(frame)
        if actual_sha256 != expected_surface_sha256:
            raise ValidationError(
                f"{field} source frame SHA-256 mismatch: "
                f"got {actual_sha256}, expected {expected_surface_sha256}"
            )

        blue, green, red, alpha = derive_channel_codebooks(
            frame,
            validated_guard.presentation_width,
            validated_guard.presentation_height,
        )
        _require_cardinalities(blue, green, red, alpha, field)
        codebooks = (blue, green, red)
        expected_codebooks = _merge_source_codebooks(
            expected_codebooks, codebooks
        )

        output_sources.append(
            {
                "run_id": run_id,
                "frame_blob": _require_string(
                    source["frame_blob"], f"{field}.frame_blob"
                ),
                "frame_pixel_sha256": expected_frame_sha256,
                "surface_blob": surface_blob,
                "surface_pixel_sha256": actual_sha256,
            }
        )

    if expected_codebooks is None or environment_identity_sha256 is None:
        raise ValidationError("guard source validation produced no profile")
    blue, green, red = expected_codebooks
    if blue != red:
        raise ValidationError("guard source blue and red five-bit codebooks disagree")

    return {
        "schema_version": PROFILE_SCHEMA_VERSION,
        "checkpoint": PROFILE_CHECKPOINT,
        "evidence_status": "DERIVED_FROM_SEALED_NATIVE_SOURCES",
        "parity_certification": "NONE",
        "generated_at_utc": utc_now(),
        "guard": {
            "schema_version": GUARD_SCHEMA_VERSION,
            "guard_id": GUARD_ID,
            "state": GUARD_STATE,
            "path": str(validated_guard.path),
            "sha256": validated_guard.sha256,
        },
        "environment_identity_sha256": environment_identity_sha256,
        "surface": {
            "width": validated_guard.presentation_width,
            "height": validated_guard.presentation_height,
            "format": surface_format,
            "pixel_layout": "BGRA8",
            "bytes_per_pixel": BYTES_PER_PIXEL,
        },
        "sources": output_sources,
        "channel_cardinalities": {
            "blue": len(blue),
            "green": len(green),
            "red": len(red),
            "alpha": len(EXPECTED_ALPHA_VALUES),
        },
        "codebooks": {
            "five_bit": list(blue),
            "six_bit": list(green),
            "channel_mapping": {
                "blue": "five_bit",
                "green": "six_bit",
                "red": "five_bit",
            },
        },
        "alpha_values": list(EXPECTED_ALPHA_VALUES),
        "evidence_scope": (
            "enrolled active retail/AMD/DDrawCompat/DXGI presentation only"
        ),
        "not_certified": [
            "universal DirectDraw expansion",
            "title geometry and final glyph tint",
            "Bink phase and packed-domain blend order",
            "transitions, input, cursor states, audio, or another resolution",
        ],
    }


def write_presentation_profile(
    guard_path: Path,
    oracle_runs: Path,
    output: Path,
    *,
    expected_guard_sha256: str = SEALED_MAIN_MENU_GUARD_SHA256,
) -> dict[str, object]:
    """Write canonical JSON once; reject existing output or link targets."""

    output_path = absolute_path(output)
    if output_path.exists() or _is_link(output_path):
        raise OutputExistsError(f"refusing to overwrite output: {output_path}")
    profile = derive_presentation_profile(
        guard_path,
        oracle_runs,
        expected_guard_sha256=expected_guard_sha256,
    )
    write_json_exclusive(output_path, profile)
    return profile
