use std::{
    ffi::c_void,
    fs::File,
    io::Read,
    mem::zeroed,
    os::windows::{ffi::OsStrExt, io::FromRawHandle},
    ptr::null_mut,
    str::FromStr,
};

use windows::{
    core::{Result, PCWSTR, PWSTR},
    Win32::{
        Foundation::{GetLastError, GENERIC_WRITE, HANDLE, TRUE},
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_NONE, OPEN_EXISTING, PIPE_ACCESS_INBOUND,
        },
        System::{
            Console::{CreatePseudoConsole, COORD, HPCON},
            Memory::{GetProcessHeap, HeapAlloc, HeapFree},
            Pipes::{CreateNamedPipeW, PIPE_TYPE_BYTE},
            Threading::{
                CreateProcessW, InitializeProcThreadAttributeList, UpdateProcThreadAttribute,
                EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, STARTUPINFOEXW,
            },
        },
    },
};

pub struct ConsoleSession {
    stdin_write_file: File,
    stdout_read_file: File,
    pesudo_console_handle: HPCON,
    process_info: PROCESS_INFORMATION,
}
impl ConsoleSession {
    pub fn new(app_name: &str, app_args: Option<&[&str]>) -> Result<Self> {
        unsafe {
            let (mut stdout_read_handle, mut stdout_write_handle) = create_pipe("stdout")?;
            let (mut stdin_read_handle, mut stdin_write_handle) = create_pipe("stdin")?;

            let pesudo_console_handle = CreatePseudoConsole(
                COORD { X: 600, Y: 700 },
                stdin_read_handle,
                stdout_write_handle,
                0,
            )?;

            let s_attr = SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: null_mut(),
                bInheritHandle: TRUE,
            };

            let mut process_info = PROCESS_INFORMATION::default();
            let startup_info_ex = create_startup_info(pesudo_console_handle)?;

            let mut app_name_wide = wstr(app_name);

            let lpapplicationname = PCWSTR(app_name_wide.as_mut_ptr());

            CreateProcessW(
                lpapplicationname,
                None,
                None,
                None,
                true,
                EXTENDED_STARTUPINFO_PRESENT,
                None,
                None,
                &startup_info_ex.StartupInfo,
                &mut process_info,
            )?;

            let stdin_write_file = File::from_raw_handle(stdin_write_handle.0);
            let stdout_read_file = File::from_raw_handle(stdout_read_handle.0);

            Ok(Self {
                stdin_write_file,
                stdout_read_file,
                pesudo_console_handle,
                process_info,
            })
        }
    }
}

unsafe fn create_pipe(pipe_name: &str) -> Result<(HANDLE, HANDLE)> {
    let pipe_name_prefixed = if pipe_name.starts_with(r"\\.\pipe\") {
        pipe_name.to_string()
    } else {
        format!(r"\\.\pipe\{}", pipe_name)
    };

    let mut pipe_name_with_null = wstr(&pipe_name_prefixed);

    let pipe_name_pwstr = PWSTR {
        0: pipe_name_with_null.as_mut_ptr(),
    };

    let s_attr = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: TRUE,
    };

    let h_read = CreateNamedPipeW(
        pipe_name_pwstr,
        PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED,
        PIPE_TYPE_BYTE,
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

fn wstr(pipe_name: &str) -> Vec<u16> {
    std::ffi::OsString::from_str(pipe_name)
        .unwrap()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

unsafe fn create_startup_info(hpcon: HPCON) -> Result<STARTUPINFOEXW> {
    let mut startup_info_ex = STARTUPINFOEXW::default();
    startup_info_ex.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    let mut bytes_required: usize = 0;

    _ = InitializeProcThreadAttributeList(None, 1, None, &mut bytes_required);

    if bytes_required == 0 {
        return Err(GetLastError().into());
    }

    let heap_handle = GetProcessHeap()?;

    startup_info_ex.lpAttributeList.0 = HeapAlloc(heap_handle, Default::default(), bytes_required);

    if startup_info_ex.lpAttributeList.0.is_null() {
        return Err(GetLastError().into());
    }

    if let Err(e) = InitializeProcThreadAttributeList(
        Some(startup_info_ex.lpAttributeList),
        1,
        None,
        &mut bytes_required,
    ) {
        _ = HeapFree(
            heap_handle,
            Default::default(),
            Some(startup_info_ex.lpAttributeList.0),
        );
        return Err(e);
    };

    if let Err(e) = UpdateProcThreadAttribute(
        startup_info_ex.lpAttributeList,
        0,
        PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
        Some(hpcon.0 as *const c_void),
        size_of::<HPCON>(),
        None,
        None,
    ) {
        _ = HeapFree(
            heap_handle,
            Default::default(),
            Some(startup_info_ex.lpAttributeList.0),
        )?;
        return Err(e);
    };

    Ok(startup_info_ex)
}

#[test]
fn pipe_test() -> Result<()> {
    let (read_handle, write_handle) = unsafe { create_pipe("name") }?;

    let mut write_file = unsafe { File::from_raw_handle(write_handle.0) };
    let mut read_file = unsafe { File::from_raw_handle(read_handle.0) };

    let test_bytes = b"0123456789ABCDEF";
    std::io::Write::write_all(&mut write_file, test_bytes)?;

    let mut buffer = [0u8; 24];
    let len = std::io::Read::read(&mut read_file, &mut buffer)?;
    assert_eq!(test_bytes[..], buffer[..len]);

    Ok(())
}

#[test]
fn console_session_new_test() -> Result<()> {
    let console_session = ConsoleSession::new("C:\\Windows\\System32\\cmd.exe", None)?;
    Ok(())
}
