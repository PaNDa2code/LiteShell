use std::io::Write;

use circular_buffer::CircularBuffer;
use console_session::ConsoleSession;

fn main() -> std::io::Result<()> {
    let console = ConsoleSession::new(r"C:\Windows\System32\cmd.exe", None)?;
    Ok(())
}
