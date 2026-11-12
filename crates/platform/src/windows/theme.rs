//! The theme of Windows: dark or light for applications, from the registry, and high contrast.

use core::ffi::c_void;

use super::{advapi32, check, kernel32, user32, wide};
use crate::error::PlatformError;
use crate::theme::{Appearance, SystemTheme};

/// The key of the registry that keeps the person's choice of colors.
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

/// The value under it that is 1 when applications should be light, 0 when dark.
const APPS_USE_LIGHT_THEME: &str = "AppsUseLightTheme";

/// The theme the person set for the system: dark or light, and whether high contrast is on.
///
/// # Errors
///
/// `PlatformError::System` when the registry or the high contrast setting could not be read.
pub fn system_theme() -> Result<SystemTheme, PlatformError> {
    Ok(SystemTheme {
        appearance: appearance()?,
        is_high_contrast: is_high_contrast()?,
    })
}

/// Whether applications should be dark or light. Without the value, as before Windows 10 version 1607, light.
fn appearance() -> Result<Appearance, PlatformError> {
    let key = wide(PERSONALIZE);
    let name = wide(APPS_USE_LIGHT_THEME);
    let mut value: u32 = 1;
    let mut bytes = u32::try_from(size_of::<u32>()).unwrap_or_default();
    let flags = advapi32::RRF_RT_REG_DWORD;
    let data = (&raw mut value).cast::<c_void>();
    // SAFETY: the key and the name are strings ended by 0; `data` points to a u32 and `bytes` says its size, as a
    // REG_DWORD needs; all are alive for the call. The type is not asked for.
    let status = unsafe {
        advapi32::RegGetValueW(
            advapi32::HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            flags,
            std::ptr::null_mut(),
            data,
            &raw mut bytes,
        )
    };
    match status.cast_unsigned() {
        0 => Ok(if value == 0 {
            Appearance::Dark
        } else {
            Appearance::Light
        }),
        kernel32::ERROR_FILE_NOT_FOUND => Ok(Appearance::Light),
        code => Err(PlatformError::System {
            call: "RegGetValueW",
            code,
        }),
    }
}

/// Whether high contrast is on.
fn is_high_contrast() -> Result<bool, PlatformError> {
    let size = u32::try_from(size_of::<user32::HIGHCONTRASTW>()).unwrap_or_default();
    let mut setting = user32::HIGHCONTRASTW {
        cbSize: size,
        dwFlags: 0,
        lpszDefaultScheme: std::ptr::null_mut(),
    };
    let pointer = (&raw mut setting).cast::<c_void>();
    // SAFETY: `setting` is a HIGHCONTRASTW with its size filled in, as SPI_GETHIGHCONTRAST asks, alive for the call.
    // The name of the scheme it points to afterwards belongs to the system and is not read.
    check("SystemParametersInfoW", unsafe {
        user32::SystemParametersInfoW(user32::SPI_GETHIGHCONTRAST, size, pointer, 0)
    })?;
    Ok(setting.dwFlags & user32::HCF_HIGHCONTRASTON != 0)
}
