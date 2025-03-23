use std::io::{self, Read, Seek, SeekFrom};

/// XOR a buffer with a key, starting at a given offset.
/// https://github.com/bitcoin/bitcoin/blob/770d39a37652d40885533fecce37e9f71cc0d051/src/streams.h#L28-L45
fn xor_buffer(data: &mut [u8], key: &[u8], key_offset: usize) {
    if key.is_empty() {
        return;
    }

    let key_offset = key_offset % key.len();
    let mut j = key_offset;

    (0..data.len()).for_each(|i| {
        data[i] ^= key[j];
        j += 1;
        if j == key.len() {
            j = 0;
        }
    });
}

/// XorReader wraps a reader and XORs it if a key is set.
/// Similar to how CAutoFile operates.
pub struct XorReader<R: Read + Seek> {
    reader: R,
    xor_key: Vec<u8>,
    position: Option<u64>,
}

impl<R: Read + Seek> XorReader<R> {
    pub fn new(mut reader: R, xor_key: Vec<u8>) -> io::Result<Self> {
        let position = reader.stream_position().ok();
        Ok(Self {
            reader,
            xor_key,
            position,
        })
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.reader.read_exact(buf)?;

        // Apply XOR if we have a key and know our position
        if !self.xor_key.is_empty() {
            if let Some(pos) = self.position {
                xor_buffer(buf, &self.xor_key, pos as usize);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "XorReader: position unknown for XOR application",
                ));
            }
        }

        // Update position if we're tracking it
        if let Some(pos) = self.position.as_mut() {
            *pos += buf.len() as u64;
        }

        Ok(())
    }

    pub fn read_u64_le(&mut self) -> io::Result<u64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    pub fn read_i64_le(&mut self) -> io::Result<i64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(i64::from_le_bytes(buf))
    }
}

impl<R: Read + Seek> Read for XorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.reader.read(buf)?;

        if bytes_read > 0 && !self.xor_key.is_empty() {
            if let Some(pos) = self.position {
                xor_buffer(&mut buf[..bytes_read], &self.xor_key, pos as usize);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "XorReader: position unknown for XOR application",
                ));
            }
        }

        // Update position if we're tracking it
        if let Some(pos) = self.position.as_mut() {
            *pos += bytes_read as u64;
        }

        Ok(bytes_read)
    }
}

impl<R: Read + Seek> Seek for XorReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = self.reader.seek(pos)?;
        self.position = Some(new_pos);
        Ok(new_pos)
    }
}
