use std::{mem::zeroed, ptr::null_mut};
use windows::Win32::{
    Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE},
    System::{
        Memory::{
            CreateFileMappingA, MapViewOfFile3, UnmapViewOfFile, VirtualAlloc2, VirtualFree,
            MEMORY_MAPPED_VIEW_ADDRESS, MEM_PRESERVE_PLACEHOLDER, MEM_RELEASE,
            MEM_REPLACE_PLACEHOLDER, MEM_RESERVE, MEM_RESERVE_PLACEHOLDER, PAGE_READWRITE,
            VIRTUAL_FREE_TYPE,
        },
        SystemInformation::GetSystemInfo,
    },
};

#[derive(Debug)]
pub struct CircularBuffer {
    base: *mut u8,
    size: usize,
    head: usize,
    tail: usize,
    filled: bool,
}

impl CircularBuffer {
    pub fn new(min_size: usize) -> windows::core::Result<Self> {
        unsafe {
            let mut size = min_size;
            let mut sys_info = zeroed();

            GetSystemInfo(&mut sys_info);

            if (size % sys_info.dwAllocationGranularity as usize) != 0 {
                size += size % sys_info.dwAllocationGranularity as usize;
            }

            let placeholder1 = VirtualAlloc2(
                None,
                None,
                2 * size,
                MEM_RESERVE | MEM_RESERVE_PLACEHOLDER,
                0x01,
                None,
            );

            if placeholder1.is_null() {
                println!("VirtualAlloc2 error\n");
                return Err(GetLastError().into());
            }

            VirtualFree(
                placeholder1,
                size,
                VIRTUAL_FREE_TYPE(MEM_RELEASE.0 | MEM_PRESERVE_PLACEHOLDER.0),
            )?;

            let placeholder2 = placeholder1.add(size);

            let section = CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                size as u32,
                None,
            )?;

            if section.0.is_null() {
                println!("CreateFileMappingA error\n");
                return Err(GetLastError().into());
            }

            let view1 = MapViewOfFile3(
                section,
                None,
                Some(placeholder1),
                0,
                size,
                MEM_REPLACE_PLACEHOLDER,
                PAGE_READWRITE.0,
                None,
            );

            if view1.Value.is_null() {
                println!("MapViewOfFile3 error\n");
                _ = CloseHandle(section);
                return Err(GetLastError().into());
            }

            let view2 = MapViewOfFile3(
                section,
                None,
                Some(placeholder2),
                0,
                size,
                MEM_REPLACE_PLACEHOLDER,
                PAGE_READWRITE.0,
                None,
            );

            _ = CloseHandle(section);

            if view2.Value.is_null() {
                _ = UnmapViewOfFile(view1);
                return Err(GetLastError().into());
            }

            let slice = std::ptr::slice_from_raw_parts_mut(view1.Value as *mut u8, size);
            (*slice).fill(0);

            Ok(Self {
                base: view1.Value.cast(),
                size,
                head: 0,
                tail: 0,
                filled: false,
            })
        }
    }

    #[inline]
    pub fn to_slice(self: &Self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.base.add(self.head), self.len()) }
    }

    #[inline]
    pub fn to_slice_mut(self: &mut Self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.base.add(self.head), self.len()) }
    }

    #[inline]
    pub fn to_raw_slice(self: &Self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.base, self.size) }
    }

    #[inline]
    fn is_full(self: &Self) -> bool {
        self.filled
    }

    #[inline]
    pub fn len(self: &Self) -> usize {
        if self.filled {
            self.size
        } else {
            self.tail
        }
    }
}

impl Drop for CircularBuffer {
    fn drop(&mut self) {
        unsafe {
            if self.base != null_mut() {
                _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.base.cast(),
                });
                _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.base.add(self.size).cast(),
                });
            }
        }
    }
}

impl std::io::Write for CircularBuffer {
    fn write(self: &mut Self, buffer: &[u8]) -> std::io::Result<usize> {
        let tail = self.tail;
        let bytes_to_write = buffer.len().min(self.size);

        unsafe {
            std::ptr::copy_nonoverlapping(buffer.as_ptr(), self.base.add(tail), bytes_to_write)
        };

        self.tail += bytes_to_write;

        if self.filled || self.tail >= self.size {
            self.tail -= self.size;
            self.filled = true;
        }
        if self.filled {
            self.head = self.tail;
        }
        Ok(bytes_to_write)
    }

    fn flush(self: &mut Self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Read for CircularBuffer {
    fn read(self: &mut Self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes_to_read = if self.is_full() {
            self.size % buf.len()
        } else {
            self.tail % buf.len()
        };
        unsafe {
            std::ptr::copy_nonoverlapping(self.base.add(self.head), buf.as_mut_ptr(), bytes_to_read)
        }
        Ok(bytes_to_read)
    }
}

#[test]
fn test_ring_buffer_write() {
    let test_str = b"0123456789ABCDEF";
    let ring_buffer_size = 64 * 1024;
    let mut ring_buffer = CircularBuffer::new(ring_buffer_size).unwrap();

    for _ in 0..16 * 1024 {
        std::io::Write::write(&mut ring_buffer, test_str)
            .expect("Error while writing to ring buffer");
    }

    let ring_buffer_slice = ring_buffer.to_slice();

    dbg!(&ring_buffer);
    assert_eq!(test_str, &ring_buffer_slice[..16]);
    assert_eq!(test_str, &ring_buffer_slice[ring_buffer_size - 16..]);
    assert_eq!(ring_buffer.size, ring_buffer.len());
}

#[test]
fn test_ring_buffer_write2() {
    let test_str = b"12345";
    let ring_buffer_size = 64 * 1024;
    let mut ring_buffer = CircularBuffer::new(ring_buffer_size).unwrap();

    for _ in 0..16 * 1024 {
        std::io::Write::write(&mut ring_buffer, test_str)
            .expect("Error while writing to ring buffer");
    }

    dbg!(&ring_buffer);

    let ring_buffer_slice = ring_buffer.to_slice();

    assert_eq!(ring_buffer.size, ring_buffer.len());
    assert_eq!(test_str, &ring_buffer_slice[ring_buffer.len() - test_str.len()..]);
}

