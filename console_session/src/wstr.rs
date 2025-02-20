use std::{os::windows::ffi::OsStrExt, str::FromStr};


#[inline]
pub fn wstr(pipe_name: &str) -> Vec<u16> {
    std::ffi::OsString::from_str(pipe_name)
        .unwrap()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
