use libva_sys::*;

use std::ffi::CStr;
use std::collections::HashMap;
use std::path::Path;
use std::fs::OpenOptions;
use std::os::unix::io::IntoRawFd;
use std::os::unix::io::AsRawFd;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub entrypoints: Vec<String>,
}

pub struct VaInstance {
    va_display: VADisplay,
    version: (i32, i32),
}

impl VaInstance {
    pub fn new() -> Result<Self, ()> {
        let va_display = unsafe { va_open_display() };

        if va_display.is_null() {
            return Err(());
        }

        let mut major = 0;
        let mut minor = 0;
        let va_status = unsafe { vaInitialize(va_display, &mut major, &mut minor) };

        if va_status != VA_STATUS_SUCCESS as i32 {
            return Err(());
        }

        Ok(Self {
            va_display,
            version: (major, minor),
        })
    }

    pub fn with_drm(drm: impl AsRef<Path>) -> Result<Self, ()> {
        let drm_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(drm.as_ref())
            .map_err(|_| ())?;

        let va_display = unsafe {
            va_display_drm::vaGetDisplayDRM(drm_file.as_raw_fd())
        };

        if va_display.is_null() {
            return Err(())
        }

        let mut major = 0;
        let mut minor = 0;
        let va_status = unsafe { vaInitialize(va_display, &mut major, &mut minor) };

        if va_status != VA_STATUS_SUCCESS as i32 {
            return Err(());
        }

        // NOTE: Its important to consume `drm_file` here and leak it so that libva doesnt crash.
        let _ = drm_file.into_raw_fd();

        Ok(Self {
            va_display,
            version: (major, minor),
        })
    }

    pub fn version(&self) -> (i32, i32) {
        self.version
    }

    pub fn vendor_string(&self) -> String {
        unsafe {
            let raw_vendor_str = vaQueryVendorString(self.va_display);

            if raw_vendor_str.is_null() {
                return "<unknown>".into();
            }

            CStr::from_ptr(raw_vendor_str).to_string_lossy().to_string()
        }
    }

    pub fn profiles(&self) -> Result<Vec<Profile>, ()> {
        let mut max_num_entrypoints = unsafe { vaMaxNumEntrypoints(self.va_display) };
        let mut entrypoints = Vec::with_capacity(max_num_entrypoints as usize);

        let mut max_num_profiles = unsafe { vaMaxNumProfiles(self.va_display) };
        let mut profiles = Vec::with_capacity(max_num_profiles as usize);

        let va_status = unsafe {
            vaQueryConfigProfiles(
                self.va_display,
                profiles.as_mut_ptr(),
                &mut max_num_profiles,
            )
        };

        if va_status != VA_STATUS_SUCCESS as i32 {
            return Err(());
        }

        unsafe {
            profiles.set_len(max_num_profiles as usize);
        }

        let mut real_profiles = HashMap::new();

        for profile in profiles {
            unsafe {
                let va_status = vaQueryConfigEntrypoints(
                    self.va_display,
                    profile,
                    entrypoints.as_mut_ptr(),
                    &mut max_num_entrypoints,
                );

                entrypoints.set_len(max_num_entrypoints as usize);

                if va_status == VA_STATUS_ERROR_UNSUPPORTED_PROFILE as i32
                    || va_status != VA_STATUS_SUCCESS as i32
                {
                    continue;
                }

                for entrypoint in entrypoints.iter() {
                    let profile_str = vaProfileStr(profile);
                    let profile_str = CStr::from_ptr(profile_str).to_string_lossy().to_string();

                    let entrypoint_str = vaEntrypointStr(*entrypoint);
                    let entrypoint_str =
                        CStr::from_ptr(entrypoint_str).to_string_lossy().to_string();
                    
                    real_profiles.entry(profile_str.clone())
                        .or_insert(Profile {
                            name: profile_str,
                            entrypoints: Vec::new()
                        })
                        .entrypoints.push(entrypoint_str);

                }
            }
        }

        Ok(real_profiles.into_values().collect())
    }
}

impl Drop for VaInstance {
    fn drop(&mut self) {
        unsafe {
            vaTerminate(self.va_display);
            va_close_display(self.va_display);
        }
    }
}
