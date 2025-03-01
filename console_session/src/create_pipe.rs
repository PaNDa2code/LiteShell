use std::{char, ffi::OsStr, os::windows::ffi::OsStrExt, ptr::null_mut};

use rand::{distr::Alphanumeric, Rng};
use windows::{
    core::{Result, PCWSTR, PWSTR},
    Win32::{
        Foundation::{GetLastError, GENERIC_WRITE, HANDLE, TRUE},
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_NONE, OPEN_EXISTING, PIPE_ACCESS_INBOUND,
        },
        System::Pipes::{CreateNamedPipeW, PIPE_NOWAIT, PIPE_TYPE_BYTE},
    },
};


pub fn create_pipe() -> Result<(HANDLE, HANDLE)> {
    let random_pipe_name: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(|x| (x as char).to_ascii_lowercase())
        .collect();

    let pipe_name_prefixed: String = format!(r"\\.\pipe\{}", random_pipe_name);

    let pipe_name_prefixed_wide: Vec<u16> = OsStr::new(&pipe_name_prefixed)
        .encode_wide()
        .chain(Some(0))
        .collect();

    let pipe_name_pwstr = PCWSTR(pipe_name_prefixed_wide.as_ptr());

    let s_attr = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: TRUE,
    };

    let h_read = unsafe {
        CreateNamedPipeW(
            pipe_name_pwstr,
            PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED,
            PIPE_TYPE_BYTE | PIPE_NOWAIT,
            1,
            0,
            6144,
            0,
            Some(&s_attr),
        )
    };

    if h_read.is_invalid() {
        return Err(unsafe { GetLastError().into() });
    }

    let h_write = unsafe {
        CreateFileW(
            pipe_name_pwstr,
            GENERIC_WRITE.0,
            FILE_SHARE_NONE,
            Some(&s_attr),
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            None,
        )
    }?;

    Ok((h_read, h_write))
}
