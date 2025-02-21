use std::ptr::null_mut;

use windows::{
    core::{Result, PWSTR},
    Win32::{
        Foundation::{GetLastError, GENERIC_WRITE, HANDLE, TRUE},
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_NONE, OPEN_EXISTING, PIPE_ACCESS_INBOUND,
        },
        System::Pipes::{CreateNamedPipeW, PIPE_NOWAIT, PIPE_TYPE_BYTE},
    },
};

use crate::wstr;

pub unsafe fn create_pipe(pipe_name: &str) -> Result<(HANDLE, HANDLE)> {
    let pipe_name_prefixed = if pipe_name.starts_with(r"\\.\pipe\") {
        pipe_name.to_string()
    } else {
        format!(r"\\.\pipe\{}", pipe_name)
    };

    let mut pipe_name_with_null = wstr(&pipe_name_prefixed);

    let pipe_name_pwstr = PWSTR(pipe_name_with_null.as_mut_ptr());

    let s_attr = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: TRUE,
    };

    let h_read = CreateNamedPipeW(
        pipe_name_pwstr,
        PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED,
        PIPE_TYPE_BYTE | PIPE_NOWAIT,
        1,
        4096,
        4096,
        0,
        Some(&s_attr),
    );

    if h_read.is_invalid() {
        return Err(GetLastError().into());
    }

    let h_write = CreateFileW(
        pipe_name_pwstr,
        GENERIC_WRITE.0,
        FILE_SHARE_NONE,
        Some(&s_attr),
        OPEN_EXISTING,
        FILE_FLAG_OVERLAPPED,
        None,
    )?;

    Ok((h_read, h_write))
}
