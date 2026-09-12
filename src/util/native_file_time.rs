//! Retail saved-list timestamp formatting. Native 0x005596A0 uses the active
//! Windows locale through GetDateFormatA / GetTimeFormatA, then converts ACP.

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeFileTime {
    low: u32,
    high: u32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeSystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[cfg(windows)]
pub(crate) fn format_timestamp_parts(unix_secs: u64) -> Option<(String, String)> {
    const WINDOWS_EPOCH_SECONDS: u64 = 11_644_473_600;
    const TICKS_PER_SECOND: u64 = 10_000_000;

    let ticks = unix_secs
        .checked_add(WINDOWS_EPOCH_SECONDS)?
        .checked_mul(TICKS_PER_SECOND)?;
    format_file_time_parts(ticks)
}

/// Format native FILETIME without discarding its epoch or fractional fields.
#[cfg(windows)]
pub(crate) fn format_file_time_parts(ticks: u64) -> Option<(String, String)> {
    // Saved-list 0x005596A0 omits both columns for either sentinel DWORD.
    if ticks as u32 == u32::MAX || (ticks >> 32) as u32 == u32::MAX {
        return None;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn FileTimeToLocalFileTime(
            source: *const NativeFileTime,
            local: *mut NativeFileTime,
        ) -> i32;
        fn FileTimeToSystemTime(
            file_time: *const NativeFileTime,
            system_time: *mut NativeSystemTime,
        ) -> i32;
    }

    let source = NativeFileTime {
        low: ticks as u32,
        high: (ticks >> 32) as u32,
    };
    let mut local = NativeFileTime { low: 0, high: 0 };
    let mut system = NativeSystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    // SAFETY: all pointers refer to live, correctly laid-out Win32 structs.
    unsafe {
        if FileTimeToLocalFileTime(&source, &mut local) == 0
            || FileTimeToSystemTime(&local, &mut system) == 0
        {
            return None;
        }
    }
    format_local_system_time(&system)
}

#[cfg(windows)]
fn format_local_system_time(system: &NativeSystemTime) -> Option<(String, String)> {
    const LOCALE_USER_DEFAULT: u32 = 0x0400;
    const DATE_SHORTDATE: u32 = 1;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDateFormatA(
            locale: u32,
            flags: u32,
            date: *const NativeSystemTime,
            format: *const u8,
            output: *mut u8,
            output_len: i32,
        ) -> i32;
        fn GetTimeFormatA(
            locale: u32,
            flags: u32,
            time: *const NativeSystemTime,
            format: *const u8,
            output: *mut u8,
            output_len: i32,
        ) -> i32;
    }

    let mut date = [0u8; 128];
    let mut time = [0u8; 128];
    // SAFETY: Win32 receives a valid SYSTEMTIME and fixed 128-byte outputs,
    // exactly matching the native load/save dialog.
    let (date_len, time_len) = unsafe {
        (
            GetDateFormatA(
                LOCALE_USER_DEFAULT,
                DATE_SHORTDATE,
                system,
                std::ptr::null(),
                date.as_mut_ptr(),
                date.len() as i32,
            ),
            GetTimeFormatA(
                LOCALE_USER_DEFAULT,
                0,
                system,
                std::ptr::null(),
                time.as_mut_ptr(),
                time.len() as i32,
            ),
        )
    };
    if date_len <= 0 || time_len <= 0 {
        return None;
    }
    let date_payload = &date[..date_len.saturating_sub(1) as usize];
    let time_payload = &time[..time_len.saturating_sub(1) as usize];
    Some((
        crate::util::native_string::acp_decode(date_payload),
        crate::util::native_string::acp_decode(time_payload),
    ))
}

#[cfg(not(windows))]
pub(crate) fn format_timestamp_parts(_unix_secs: u64) -> Option<(String, String)> {
    None
}

#[cfg(not(windows))]
pub(crate) fn format_file_time_parts(_ticks: u64) -> Option<(String, String)> {
    None
}

#[cfg(all(test, windows))]
mod tests {
    use super::{NativeSystemTime, format_local_system_time};

    #[test]
    fn fixed_system_time_uses_platform_short_date_and_time() {
        let fixed = NativeSystemTime {
            year: 2026,
            month: 7,
            day_of_week: 4,
            day: 30,
            hour: 13,
            minute: 45,
            second: 12,
            milliseconds: 0,
        };
        let (date, time) = format_local_system_time(&fixed).expect("Win32 locale formatting");
        assert!(!date.is_empty());
        assert!(!time.is_empty());
        assert!(!date.contains("ago"));
        assert!(!time.contains("ago"));
    }
}
