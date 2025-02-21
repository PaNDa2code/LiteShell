use std::{
    fs::File,
    io::Write,
    sync::{Arc, Mutex},
};

use circular_buffer::CircularBuffer;
use console_session::ConsoleSession;

fn main() -> std::io::Result<()> {
    let circular_buffer = Arc::new(Mutex::new(CircularBuffer::new(64 * 1024)?));
    let console = ConsoleSession::new_shell(console_session::ShellApp::CMD)?;

    let mut stdin = console.stdin_write_file;
    let stdout = console.stdout_read_file;

    stdin.write_all(b"echo HelloWorld\r\n")?;
    stdin.flush()?;

    let circular_buffer_clone = Arc::clone(&circular_buffer);

    let _ = std::thread::spawn(move || {
        read_thread(circular_buffer_clone, stdout);
    });

    let mut len = 0;
    let circular_buffer_clone2 = Arc::clone(&circular_buffer);

    loop {
        let circular_buffer = circular_buffer_clone2.lock().unwrap();
        if circular_buffer.len() != len {
            println!("{:02x?}", circular_buffer.to_slice());
            len = circular_buffer.len();
        }
    }
}

// Change parameter type to Arc<Mutex<CircularBuffer>>
fn read_thread(circular_buffer: Arc<Mutex<CircularBuffer>>, mut stdout: File) {
    loop {
        let mut circular_buffer = circular_buffer.lock().unwrap();
        circular_buffer.read_from_file(&mut stdout).unwrap();
    }
}
