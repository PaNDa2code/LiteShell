use std::io::{Read, Write};

use console_session::ConsoleSession;

fn main() -> std::io::Result<()> {
    let console = ConsoleSession::new_shell(console_session::ShellApp::Bash)?;

    let mut stdin = console.stdin_write_file;
    let mut stdout = console.stdout_read_file;

    stdin.write_all(b"echo HelloWorld\r\n")?;
    stdin.flush()?;

    let mut output = String::new();

    std::thread::sleep_ms(1000);

    for _ in 0..100 {
        stdout.read_to_string(&mut output)?;
    }

    println!("{output:?}");

    Ok(())
}
