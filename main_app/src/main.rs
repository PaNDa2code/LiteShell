use circular_buffer::CircularBuffer;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let mut circular_buffer =
        CircularBuffer::new(64 * 1024).expect("Error while createing a circular buffer");
    
    circular_buffer.write_all(b"0123456789ABCDEF")?;
    Ok(())
}
