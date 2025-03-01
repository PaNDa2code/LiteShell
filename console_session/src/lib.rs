use std::{ffi::OsStr, fs::File, os::windows::{ffi::OsStrExt, io::FromRawHandle}};

use windows::{
    core::{Result, PCWSTR},
    Win32::System::{
        Console::{CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON},
        Threading::{CreateProcessW, EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION},
    },
};

mod create_pipe;
use create_pipe::create_pipe;

mod startupinfo;
use startupinfo::create_startup_info;

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
    pub pesudo_console_handle: HPCON,
    pub process_info: PROCESS_INFORMATION,
    pub size: ConsoleSize,
}

#[derive(Debug, Clone, Copy)]
pub struct ConsoleSize {
    x: u16,
    y: u16,
}

impl From<ConsoleSize> for COORD {
    fn from(size: ConsoleSize) -> Self {
        Self {
            X: size.x as i16,
            Y: size.y as i16,
        }
    }
}

impl ConsoleSession {
    pub fn new(app_name: &str, app_args: Option<&[&str]>) -> Result<Self> {
        _ = app_args;
        let (stdout_read_handle, stdout_write_handle) = create_pipe()?;
        let (stdin_read_handle, stdin_write_handle) = create_pipe()?;
        let size = ConsoleSize { x: 600, y: 800 };
        let cord: COORD = size.into();

        let pesudo_console_handle =
            unsafe { CreatePseudoConsole(cord, stdin_read_handle, stdout_write_handle, 0) }?;

        let mut process_info = PROCESS_INFORMATION::default();
        let startup_info_ex = create_startup_info(pesudo_console_handle)?;

        let app_name_wide: Vec<u16> = OsStr::new(app_name).encode_wide().chain(Some(0)).collect();

        let app_name_pcwstr = PCWSTR(app_name_wide.as_ptr());

        unsafe {
            CreateProcessW(
                app_name_pcwstr,
                None,
                None,
                None,
                true,
                EXTENDED_STARTUPINFO_PRESENT,
                None,
                None,
                &startup_info_ex.StartupInfo,
                &mut process_info,
            )
        }?;

        let stdin_write_file = unsafe { File::from_raw_handle(stdin_write_handle.0) };
        let stdout_read_file = unsafe { File::from_raw_handle(stdout_read_handle.0) };

        Ok(Self {
            stdin_write_file,
            stdout_read_file,
            pesudo_console_handle,
            process_info,
            size,
        })
    }

    pub fn new_shell(shell_app: ShellApp) -> Result<Self> {
        Self::new(shell_app.path(), None)
    }

    pub fn resize<T: Into<u16> + Copy>(self: &mut Self, width: T, hight: T) -> Result<()> {
        self.size.x = width.into();
        self.size.y = hight.into();
        unsafe { ResizePseudoConsole(self.pesudo_console_handle, self.size.into()) }?;
        Ok(())
    }
}

#[test]
fn pipe_test() -> Result<()> {
    let (read_handle, write_handle) = create_pipe()?;

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
    let mut console_session = ConsoleSession::new_shell(ShellApp::CMD)?;
    console_session.resize(100u16, 100u16)?;
    Ok(())
}
