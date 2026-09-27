// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

use crate::ConfigError;
use std::path::PathBuf;

fn non_empty_env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub(crate) fn home_dir() -> Result<PathBuf, ConfigError> {
    #[cfg(windows)]
    {
        if let Some(path) = non_empty_env_path("USERPROFILE") {
            return Ok(path);
        }

        if let (Some(home_drive), Some(home_path)) =
            (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH"))
        {
            let mut merged = std::ffi::OsString::new();
            merged.push(home_drive);
            merged.push(home_path);
            if !merged.is_empty() {
                return Ok(PathBuf::from(merged));
            }
        }

        if let Some(path) = non_empty_env_path("HOME") {
            return Ok(path);
        }

        Err(ConfigError::PlatformPathUnavailable("home"))
    }

    #[cfg(not(windows))]
    {
        non_empty_env_path("HOME").ok_or(ConfigError::PlatformPathUnavailable("home"))
    }
}

pub(crate) fn config_dir() -> Result<PathBuf, ConfigError> {
    #[cfg(target_os = "windows")]
    {
        return document_dir().map(|path| path.join("My Games").join("openmw"));
    }

    #[cfg(target_os = "macos")]
    {
        return home_dir().map(|path| path.join("Library").join("Preferences").join("openmw"));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(path) = non_empty_env_path("XDG_CONFIG_HOME") {
            return Ok(path.join("openmw"));
        }

        home_dir().map(|path| path.join(".config").join("openmw"))
    }
}

pub(crate) fn data_dir() -> Result<PathBuf, ConfigError> {
    #[cfg(target_os = "windows")]
    {
        return config_dir();
    }

    #[cfg(target_os = "macos")]
    {
        return home_dir().map(|path| {
            path.join("Library")
                .join("Application Support")
                .join("openmw")
        });
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(path) = non_empty_env_path("XDG_DATA_HOME") {
            return Ok(path.join("openmw"));
        }

        home_dir().map(|path| path.join(".local").join("share").join("openmw"))
    }
}

#[cfg(windows)]
fn document_dir() -> Result<PathBuf, ConfigError> {
    use std::os::windows::ffi::OsStringExt;
    use std::ptr::null_mut;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath};

    let mut raw_path: windows_sys::core::PWSTR = null_mut();

    // SAFETY: SHGetKnownFolderPath initializes `raw_path` on success for FOLDERID_Documents.
    let status = unsafe { SHGetKnownFolderPath(&FOLDERID_Documents, 0, null_mut(), &mut raw_path) };
    if status != 0 || raw_path.is_null() {
        return Err(ConfigError::PlatformPathUnavailable("documents"));
    }

    let mut len = 0_usize;
    // SAFETY: `raw_path` is a valid null-terminated UTF-16 string from SHGetKnownFolderPath.
    unsafe {
        while *raw_path.add(len) != 0 {
            len += 1;
        }
    }

    // SAFETY: `raw_path` points to at least `len` UTF-16 code units plus terminator.
    let slice = unsafe { std::slice::from_raw_parts(raw_path, len) };
    let os_string = std::ffi::OsString::from_wide(slice);

    // SAFETY: `raw_path` is allocated by SHGetKnownFolderPath and must be released via CoTaskMemFree.
    unsafe {
        CoTaskMemFree(raw_path.cast());
    }

    Ok(PathBuf::from(os_string))
}
