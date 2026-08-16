//! Source-level dependency guards for the engine domain-boundaries ledger
//! (`docs/plans/2026-08-15-engine-domain-boundaries-design.md`, local).
//!
//! Installed ahead of F14 per the validation protocol: every boundary the
//! design forbids is scanned now, boundaries that are already clean are held
//! at zero, and the finite F04/F05/F06 remainder is pinned by an explicit
//! exception inventory that may only shrink. Production code only:
//! `tests.rs` / `*_tests.rs` files and `#[cfg(test)]` items are excluded, so
//! test-only reverse edges (tracked by the ledger for F14 relocation) do not
//! appear here. `cfg(any(test, debug_assertions))` items are deliberately
//! treated as production — they compile into debug builds.
//!
//! The `app` pseudo-root currently matches `crate::app::*`, root
//! `crate::app_*` modules, and `crate::skirmish_scenarios`. F12 moves the
//! remaining root app inventory under `src/app/`, after which `crate::app::`
//! covers the whole layer and F14 adds the no-root-`app_*` guard.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Directory under `src/` -> reference roots its production code must not
/// name. Direction contract: ENGINE.md "Architecture boundaries" and the
/// design's "Proven dependency inversions".
const LAYER_RULES: &[(&str, &[&str])] = &[
    ("assets", &["sim", "rules", "map", "render", "sidebar", "ui", "app"]),
    ("util", &["sim", "rules", "map", "render", "sidebar", "ui", "app"]),
    ("rules", &["sim", "map"]),
    ("map", &["sim", "render"]),
    ("render", &["app"]),
    ("sidebar", &["app"]),
    ("ui", &["app", "skirmish_scenarios"]),
    ("sim", &["render", "sidebar", "ui", "audio", "net"]),
];

/// The frozen ledger's remaining production exceptions. Entries may only be
/// REMOVED — by the ledger item that closes them — never added. An edge that
/// disappears from the source must also be deleted here, so the ratchet
/// tightens monotonically.
const FROZEN_EXCEPTIONS: &[(&str, &str)] = &[
    // F06: combat-light draw DTO moves to render.
    ("render/combat_light.rs", "app"),
    // F06: cursor ID / software-cursor DTOs move to render.
    ("render/cursor_atlas.rs", "app"),
    // F06: sidebar gets a concrete ArmedSidebarEntry instead of TargetingMode.
    ("sidebar/sidebar_view.rs", "app"),
    // F06: MapMenuEntry consolidates with the scenario catalog DTO.
    ("ui/skirmish_shell/state/choose_map.rs", "skirmish_scenarios"),
    ("ui/skirmish_shell/state/combos.rs", "app"),
    ("ui/skirmish_shell/state/hit_test.rs", "app"),
    ("ui/skirmish_shell/state/launch.rs", "app"),
    ("ui/skirmish_shell/state/player_name.rs", "app"),
    ("ui/skirmish_shell/state/trackbars.rs", "app"),
];

#[test]
fn production_dependency_edges_match_frozen_ledger_inventory() {
    let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let found = scan_forbidden_edges(&src_root);
    let expected: BTreeSet<(String, String)> = FROZEN_EXCEPTIONS
        .iter()
        .map(|(file, root)| (file.to_string(), root.to_string()))
        .collect();

    let unexpected: Vec<_> = found.difference(&expected).collect();
    let stale: Vec<_> = expected.difference(&found).collect();
    assert!(
        unexpected.is_empty(),
        "new forbidden production dependency edge(s): {unexpected:?}\n\
         The layer contract (ENGINE.md, architecture boundaries) forbids these \
         directions. Move the type/function to its owning layer instead of \
         importing upward; FROZEN_EXCEPTIONS only ever shrinks."
    );
    assert!(
        stale.is_empty(),
        "stale FROZEN_EXCEPTIONS entr(ies): {stale:?}\n\
         The edge no longer exists in production source. Delete the entry so \
         the ratchet tightens."
    );

    // The #1 invariant (ENGINE.md): sim never depends on presentation, audio,
    // or net in production. Zero exceptions, frozen or otherwise.
    assert!(
        !FROZEN_EXCEPTIONS.iter().any(|(f, _)| f.starts_with("sim/")),
        "sim/ may never appear in FROZEN_EXCEPTIONS"
    );
}

fn scan_forbidden_edges(src_root: &Path) -> BTreeSet<(String, String)> {
    let mut found = BTreeSet::new();
    for (layer, forbidden) in LAYER_RULES {
        let dir = src_root.join(layer);
        assert!(
            dir.is_dir(),
            "layer directory src/{layer} missing — update LAYER_RULES"
        );
        visit_rust_files(&dir, &mut |path| {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name == "tests.rs" || name.ends_with("_tests.rs") {
                return;
            }
            let source = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let production = strip_test_items(&blank_comments_and_literals(&source));
            for root in *forbidden {
                if contains_crate_ref(&production, root) {
                    let rel = path
                        .strip_prefix(src_root)
                        .expect("scanned file under src")
                        .to_string_lossy()
                        .replace('\\', "/");
                    found.insert((rel, root.to_string()));
                }
            }
        });
    }
    found
}

fn visit_rust_files(dir: &Path, visit: &mut dyn FnMut(&Path)) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    let mut paths: Vec<_> = entries
        .map(|e| e.expect("dir entry").path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            visit_rust_files(&path, visit);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            visit(&path);
        }
    }
}

/// Replace comment and string/char-literal contents with spaces, preserving
/// newlines and all code structure, so later brace counting and `crate::`
/// searches cannot be confused by literals or prose (doc links included).
fn blank_comments_and_literals(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    out.push(b' ');
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let mut depth = 1u32;
                out.extend_from_slice(b"  ");
                i += 2;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        out.extend_from_slice(b"  ");
                        i += 2;
                    } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        out.extend_from_slice(b"  ");
                        i += 2;
                    } else {
                        out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' });
                        i += 1;
                    }
                }
            }
            b'r' | b'b' if is_raw_string_start(bytes, i) => {
                // r"...", r#"..."#, br"..." etc.: no escapes, terminated by
                // a quote followed by the opening number of hashes.
                let mut j = i + 1;
                if bytes.get(j) == Some(&b'r') {
                    j += 1;
                }
                let mut hashes = 0usize;
                while bytes.get(j) == Some(&b'#') {
                    hashes += 1;
                    j += 1;
                }
                // j is at the opening quote.
                out.resize(out.len() + (j - i) + 1, b' ');
                i = j + 1;
                while i < bytes.len() {
                    if bytes[i] == b'"' && bytes[i + 1..].iter().take(hashes).filter(|&&c| c == b'#').count() == hashes {
                        out.resize(out.len() + 1 + hashes, b' ');
                        i += 1 + hashes;
                        break;
                    }
                    out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' });
                    i += 1;
                }
            }
            b'"' => {
                out.push(b' ');
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        out.extend_from_slice(b"  ");
                        i += 2;
                    } else if bytes[i] == b'"' {
                        out.push(b' ');
                        i += 1;
                        break;
                    } else {
                        out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' });
                        i += 1;
                    }
                }
            }
            b'\'' => {
                // Distinguish char literals from lifetimes: a char literal
                // closes with a quote after one (possibly escaped) character.
                if bytes.get(i + 1) == Some(&b'\\') {
                    out.push(b' ');
                    i += 1;
                    while i < bytes.len() && bytes[i] != b'\'' {
                        out.push(b' ');
                        i += 1;
                    }
                    out.push(b' ');
                    i += 1;
                } else if bytes.get(i + 2) == Some(&b'\'') {
                    out.extend_from_slice(b"   ");
                    i += 3;
                } else {
                    // Lifetime — keep as-is.
                    out.push(b);
                    i += 1;
                }
            }
            _ => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).expect("blanking preserves UTF-8 structure")
}

fn is_raw_string_start(bytes: &[u8], i: usize) -> bool {
    // Accept `r` / `br` prefixes; plain `b"…"` falls through to the ordinary
    // string arm on its quote.
    let mut j = i;
    if bytes[j] == b'b' {
        j += 1;
    }
    if bytes.get(j) != Some(&b'r') {
        return false;
    }
    j += 1;
    while bytes.get(j) == Some(&b'#') {
        j += 1;
    }
    bytes.get(j) == Some(&b'"')
}

/// Remove `#[cfg(test)]` items (attribute plus the following item, braced or
/// single-statement) from already-blanked source. `cfg(all(test, …))` counts
/// as test-only; `cfg(any(test, debug_assertions))` does not.
fn strip_test_items(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth: i64 = 0;
    let mut skip_above: Option<i64> = None;
    let mut awaiting_item = false;
    for line in text.lines() {
        let opens = line.bytes().filter(|&b| b == b'{').count() as i64;
        let closes = line.bytes().filter(|&b| b == b'}').count() as i64;
        let is_test_attr = line.contains("cfg(test") || line.contains("cfg(all(test");
        let mut emit = true;

        if skip_above.is_some() {
            emit = false;
        } else if is_test_attr {
            emit = false;
            if opens > 0 {
                // Attribute and item share the line.
                skip_above = Some(depth);
            } else {
                awaiting_item = true;
            }
        } else if awaiting_item {
            emit = false;
            let trimmed = line.trim_start();
            if trimmed.starts_with("#[") && opens == 0 {
                // Further attribute on the same item; keep waiting.
            } else if opens > 0 {
                skip_above = Some(depth);
                awaiting_item = false;
            } else if trimmed.trim_end().ends_with(';') {
                // Single-statement item (e.g. `use`), fully consumed.
                awaiting_item = false;
            }
        }

        depth += opens - closes;
        if let Some(above) = skip_above {
            if depth <= above {
                skip_above = None;
            }
        }
        if emit {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Token-boundary search for `crate::<root>`. The `app` pseudo-root also
/// matches root `crate::app_*` modules.
fn contains_crate_ref(text: &str, root: &str) -> bool {
    let needle = format!("crate::{root}");
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(pos) = text[start..].find(&needle) {
        let abs = start + pos;
        let end = abs + needle.len();
        let prev_ok = abs == 0 || {
            let p = bytes[abs - 1];
            !(p.is_ascii_alphanumeric() || p == b'_' || p == b':')
        };
        let next = bytes.get(end).copied();
        let boundary_ok = match root {
            "app" => matches!(next, Some(b':' | b'_')),
            _ => !matches!(next, Some(c) if c.is_ascii_alphanumeric() || c == b'_'),
        };
        if prev_ok && boundary_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}
