use windows::Win32::{
    Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE},
    System::{
        Memory::{
            CreateFileMappingA, MapViewOfFile3, UnmapViewOfFile, VirtualAlloc2, VirtualFree,
            MEMORY_MAPPED_VIEW_ADDRESS, MEM_PRESERVE_PLACEHOLDER, MEM_RELEASE,
            MEM_REPLACE_PLACEHOLDER, MEM_RESERVE, MEM_RESERVE_PLACEHOLDER, PAGE_READWRITE,
            VIRTUAL_FREE_TYPE,
        },
        SystemInformation::{GetSystemInfo, SYSTEM_INFO},
    },
};

#[derive(Debug)]
pub struct CircularBuffer<'a> {
    base: &'a mut [u8],
    doubled: &'a mut [u8],
    size: usize,
    head: usize,
    tail: usize,
    len: usize,
}

impl<'a> CircularBuffer<'a> {
    pub fn new(min_size: usize) -> windows::core::Result<Self> {
        unsafe {
            let mut size = min_size;
            let mut sys_info = SYSTEM_INFO::default();

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

            let base = std::slice::from_raw_parts_mut(view1.Value as *mut u8, size);
            base.fill(0);

            let doubled = std::slice::from_raw_parts_mut(view1.Value as *mut u8, size);
            doubled.fill(0);

            Ok(Self {
                base,
                doubled,
                size,
                head: 0,
                tail: 0,
                len: 0,
            })
        }
    }

    #[inline]
    pub fn to_slice(self: &Self) -> &[u8] {
        &self.base[self.head..self.len()]
    }

    #[inline]
    pub fn write_slice(self: &mut Self) -> &mut [u8] {
        &mut self.doubled[self.tail..]
    }

    #[inline]
    fn is_full(self: &Self) -> bool {
        self.len == self.size
    }

    #[inline]
    pub fn len(self: &Self) -> usize {
        self.len
    }

    pub fn write(self: &mut Self, buffer: &[u8]) -> usize {
        let size = self.size;
        let bytes_to_write = buffer.len().min(size);
        self.write_slice().copy_from_slice(&buffer[..bytes_to_write]);
        self.commit_write(bytes_to_write);
        bytes_to_write
    }

    #[inline]
    fn commit_write(self: &mut Self, data_len: usize) {
        let size = self.size;

        self.tail += data_len;
        self.tail &= size - 1;

        if self.is_full() {
            self.head = self.tail;
            return;
        }

        self.len += data_len;
        if self.len > size {
            self.head = self.tail;
            self.len = size;
        }
    }


    pub fn read_from_file(self: &mut Self, file: &mut std::fs::File) -> std::io::Result<usize> {
        let write_slice = self.write_slice();
        match std::io::Read::read(file, write_slice) {
            Ok(bytes) => {
                self.commit_write(bytes);
                Ok(bytes)
            },
            Err(e) => Err(e)
        }
    }
}

impl<'a> Drop for CircularBuffer<'a> {
    fn drop(&mut self) {
        unsafe {
            _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.base.as_mut_ptr().cast(),
            });
            _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.base.as_mut_ptr().add(self.size).cast(),
            });
        }
    }
}

impl<'a> std::io::Write for CircularBuffer<'a> {
    fn write(self: &mut Self, buffer: &[u8]) -> std::io::Result<usize> {
        Ok(self.write(buffer))
    }

    fn flush(self: &mut Self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> std::io::Read for CircularBuffer<'a> {
    fn read(self: &mut Self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes_to_read = self.len().min(buf.len());
        buf.copy_from_slice(&self.to_slice()[..bytes_to_read]);
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
    assert_eq!(
        test_str,
        &ring_buffer_slice[ring_buffer.len() - test_str.len()..]
    );
}
