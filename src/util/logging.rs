//! Logging initialization helpers.
//!
//! Binaries use this to route `log`/`env_logger` output into a file under
//! `logs/` in the current working directory.

use std::backtrace::Backtrace;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use env_logger::{Builder, Env, Target};

static EXCEPTION_DUMP_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Initialize env_logger to append to `logs/<name>.log`.
///
/// Honors `RUST_LOG` and defaults to `info` when the variable is unset.
/// Returns the resolved log file path on success.
pub fn init_file_logger(name: &str) -> io::Result<PathBuf> {
    let mut log_dir = std::env::current_dir()?;
    log_dir.push("logs");
    fs::create_dir_all(&log_dir)?;

    let mut log_path = log_dir;
    log_path.push(format!("{name}.log"));

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    // Suppress wgpu_hal's per-frame "Suboptimal present" warnings which
    // flood the log (~100K+ lines per session on AMD Vulkan).
    let env = Env::default().default_filter_or("info,wgpu_hal=error");
    let mut builder = Builder::from_env(env);
    builder.format_timestamp_secs();
    builder.target(Target::Pipe(Box::new(file)));
    builder.init();

    Ok(log_path)
}

/// Register a panic hook that writes the panic info and backtrace to the log
/// file when one is available. Each outermost panic creates or replaces
/// retail's `except.txt` report in the current working directory; recursive
/// hook entry is suppressed. The default stderr hook is preserved so terminal
/// users still see output.
pub fn install_panic_hook(log_path: Option<&Path>) {
    let prev_hook = std::panic::take_hook();
    let log_path = log_path.map(Path::to_owned);

    std::panic::set_hook(Box::new(move |info| {
        let Some(_guard) = claim_exception_dump(&EXCEPTION_DUMP_ACTIVE) else {
            prev_hook(info);
            return;
        };

        // Capture while the panic stack is still live.
        let backtrace = Backtrace::force_capture();

        if let Some(log_path) = &log_path {
            if let Ok(mut file) = OpenOptions::new().append(true).open(log_path) {
                let _ = writeln!(file, "\n========== PANIC ==========");
                let _ = writeln!(file, "{info}");
                let _ = writeln!(file, "\n{backtrace}");
                let _ = writeln!(file, "===========================");
                let _ = file.flush();
            }
        }

        let _ = write_exception_dump(info, &backtrace);

        // Preserve default stderr output for terminal users.
        prev_hook(info);
    }));
}

struct ExceptionDumpGuard<'a> {
    active: &'a AtomicBool,
}

impl Drop for ExceptionDumpGuard<'_> {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
    }
}

#[inline]
fn claim_exception_dump(active: &AtomicBool) -> Option<ExceptionDumpGuard<'_>> {
    active
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .ok()
        .map(|_| ExceptionDumpGuard { active })
}

fn write_exception_dump(
    info: &std::panic::PanicHookInfo<'_>,
    backtrace: &Backtrace,
) -> io::Result<()> {
    let report = normalize_crlf(&format!(
        "{info}\nInternal Version {}\n\n{backtrace}\n",
        crate::util::version::retail_internal_version()
    ));

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0).attributes(0x80);
    }

    let mut file = options.open("except.txt")?;
    file.write_all(report.as_bytes())?;
    file.flush()
}

fn normalize_crlf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use super::{claim_exception_dump, normalize_crlf};

    #[test]
    fn exception_dump_guard_blocks_recursion_then_rearms() {
        let active = AtomicBool::new(false);
        let outer = claim_exception_dump(&active).expect("first handler entry");
        assert!(
            claim_exception_dump(&active).is_none(),
            "recursive handler entry must not overwrite the active report"
        );

        drop(outer);
        assert!(
            claim_exception_dump(&active).is_some(),
            "a later independent handler entry must replace except.txt"
        );
    }

    #[test]
    fn exception_dump_uses_windows_line_endings() {
        assert_eq!(normalize_crlf("one\ntwo\r\n"), "one\r\ntwo\r\n");
    }
}
