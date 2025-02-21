use std::{
    fs::File,
    os::windows::{ffi::OsStrExt, io::FromRawHandle},
    str::FromStr,
};

use circular_buffer::CircularBuffer;
use windows::{
    core::{Result, PCWSTR},
    Win32::
        System::{
            Console::{CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON},
            Threading::{
                CreateProcessW, EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION,
            },
        }
    ,
};

mod create_pipe;
use create_pipe::create_pipe;

mod startupinfo;
use startupinfo::create_startup_info;

mod wstr;
use wstr::*;

pub enum ShellApp {
    CMD,
    PowerShell,
    Bash,
}

impl ShellApp {
    pub fn path(&self) -> &'static str {
        match self {
            ShellApp::CMD => r"C:\Windows\System32\cmd.exe",
            ShellApp::PowerShell => r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
            ShellApp::Bash => r"C:\msys64\usr\bin\bash.exe",
        }
    }
}

pub struct ConsoleSession {
    pub stdin_write_file: File,
    pub stdout_read_file: File,
    pesudo_console_handle: HPCON,
    process_info: PROCESS_INFORMATION,
}

impl ConsoleSession {
    pub fn new(app_name: &str, app_args: Option<&[&str]>) -> Result<Self> {
        _ = app_args;
        unsafe {
            let circular_buffer = CircularBuffer::new(64 * 1024)?;
            let (stdout_read_handle, stdout_write_handle) = create_pipe("stdout")?;
            let (stdin_read_handle, stdin_write_handle) = create_pipe("stdin")?;

            let pesudo_console_handle = CreatePseudoConsole(
                COORD { X: 600, Y: 700 },
                stdin_read_handle,
                stdout_write_handle,
                0,
            )?;

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

    pub fn new_shell(shell_app: ShellApp) -> Result<Self> {
        Self::new(shell_app.path(), None)
    }

    pub fn resize(self: &mut Self, width: u16, hight: u16) -> Result<()> {
        unsafe {
            ResizePseudoConsole(
                self.pesudo_console_handle,
                COORD {
                    X: width as i16,
                    Y: hight as i16,
                },
            )?
        };
        Ok(())
    }
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
    let mut console_session = ConsoleSession::new("C:\\Windows\\System32\\cmd.exe", None)?;
    console_session.resize(100, 100)?;
    Ok(())
}
