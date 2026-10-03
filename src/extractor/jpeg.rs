use std::io::{self, Read, Seek, SeekFrom, Write};
use crate::models::PhotoMetadata;

/// Scans an input stream (using buffered chunked scanning) to locate embedded JPEG markers.
/// Returns (start_offset, length, metadata) if a valid JPEG is found.
pub fn scan_for_jpeg<R: Read + Seek>(reader: &mut R, total_file_size: u64) -> io::Result<Option<PhotoMetadata>> {
    reader.seek(SeekFrom::Start(0))?;

    // Chunk size for searching SOI
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut buffer = [0u8; CHUNK_SIZE];
    let mut current_offset: u64 = 0;
    let mut soi_offset: Option<u64> = None;

    // Scan for SOI marker: 0xFF, 0xD8, 0xFF
    let mut overlap = [0u8; 3];
    let mut overlap_len = 0;

    while current_offset < total_file_size {
        let to_read = ((total_file_size - current_offset) as usize).min(CHUNK_SIZE);
        let read_bytes = reader.read(&mut buffer[..to_read])?;
        if read_bytes == 0 {
            break;
        }

        let slice = &buffer[..read_bytes];

        // Check across chunk boundary
        if overlap_len > 0 {
            let mut combined = [0u8; 6];
            combined[..overlap_len].copy_from_slice(&overlap[..overlap_len]);
            let fill = 3.min(read_bytes);
            combined[overlap_len..overlap_len + fill].copy_from_slice(&slice[..fill]);

            for i in 0..overlap_len {
                if combined[i] == 0xFF && combined[i + 1] == 0xD8 && combined[i + 2] == 0xFF {
                    soi_offset = Some(current_offset - overlap_len as u64 + i as u64);
                    break;
                }
            }
            if soi_offset.is_some() {
                break;
            }
        }

        // Search within current buffer
        if slice.len() >= 3 {
            for i in 0..slice.len() - 2 {
                if slice[i] == 0xFF && slice[i + 1] == 0xD8 && slice[i + 2] == 0xFF {
                    soi_offset = Some(current_offset + i as u64);
                    break;
                }
            }
            if soi_offset.is_some() {
                break;
            }
        }

        // Save last 2 bytes for next chunk overlap
        if read_bytes >= 2 {
            overlap[0] = slice[read_bytes - 2];
            overlap[1] = slice[read_bytes - 1];
            overlap_len = 2;
        } else {
            overlap_len = 0;
        }

        current_offset += read_bytes as u64;
    }

    let start = match soi_offset {
        Some(s) => s,
        None => return Ok(None),
    };

    // Now parse segments from start offset to find dimensions and exact EOI
    match parse_jpeg_stream(reader, start, total_file_size) {
        Ok(Some((width, height, end_offset))) => {
            let jpeg_len = end_offset - start;
            Ok(Some(PhotoMetadata {
                width,
                height,
                jpeg_offset: start,
                jpeg_len,
                source_size_bytes: total_file_size,
                output_size_bytes: jpeg_len,
            }))
        }
        Ok(None) => Ok(None),
        Err(_) => {
            // Fallback: search for EOI (0xFF 0xD9) in stream
            if let Some(end_offset) = fallback_find_eoi(reader, start, total_file_size)? {
                let (width, height) = read_dimensions_fallback(reader, start, end_offset)?;
                let jpeg_len = end_offset - start;
                Ok(Some(PhotoMetadata {
                    width: width.unwrap_or(0),
                    height: height.unwrap_or(0),
                    jpeg_offset: start,
                    jpeg_len,
                    source_size_bytes: total_file_size,
                    output_size_bytes: jpeg_len,
                }))
            } else {
                Ok(None)
            }
        }
    }
}

/// Parses JPEG segments starting at `start_offset`.
/// Returns (width, height, end_offset) where end_offset is after EOI (0xFF 0xD9).
fn parse_jpeg_stream<R: Read + Seek>(
    reader: &mut R,
    start_offset: u64,
    total_file_size: u64,
) -> io::Result<Option<(u32, u32, u64)>> {
    reader.seek(SeekFrom::Start(start_offset))?;

    // Verify SOI
    let mut soi = [0u8; 2];
    reader.read_exact(&mut soi)?;
    if soi != [0xFF, 0xD8] {
        return Ok(None);
    }

    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut pos = start_offset + 2;

    while pos < total_file_size {
        reader.seek(SeekFrom::Start(pos))?;
        let mut marker_prefix = [0u8; 1];
        if reader.read(&mut marker_prefix)? == 0 {
            break;
        }

        if marker_prefix[0] != 0xFF {
            // Out of sync, break to fallback
            return Ok(None);
        }

        // Handle padding 0xFF bytes
        let mut marker_byte = [0u8; 1];
        loop {
            if reader.read(&mut marker_byte)? == 0 {
                return Ok(None);
            }
            if marker_byte[0] != 0xFF {
                break;
            }
        }

        let marker = marker_byte[0];

        // EOI
        if marker == 0xD9 {
            let end_offset = reader.stream_position()?;
            if let (Some(w), Some(h)) = (width, height) {
                return Ok(Some((w, h, end_offset)));
            } else {
                return Ok(Some((0, 0, end_offset)));
            }
        }

        // Standalone markers without length: TEM (0x01), RST0..RST7 (0xD0..0xD7)
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            pos = reader.stream_position()?;
            continue;
        }

        // SOS (Start of Scan) - Entropy coded data starts after SOS header
        if marker == 0xDA {
            let mut len_buf = [0u8; 2];
            reader.read_exact(&mut len_buf)?;
            let sos_len = u16::from_be_bytes(len_buf) as u64;
            let entropy_start = reader.stream_position()? + sos_len - 2;

            // Search for EOI in entropy stream
            if let Some(end_offset) = scan_entropy_for_eoi(reader, entropy_start, total_file_size)? {
                let w = width.unwrap_or(0);
                let h = height.unwrap_or(0);
                return Ok(Some((w, h, end_offset)));
            } else {
                return Ok(None);
            }
        }

        // Read segment length
        let mut len_buf = [0u8; 2];
        if reader.read_exact(&mut len_buf).is_err() {
            break;
        }
        let seg_len = u16::from_be_bytes(len_buf) as u64;
        if seg_len < 2 {
            break;
        }

        // Check if this is a SOFn marker (SOF0 = 0xC0, SOF1 = 0xC1, SOF2 = 0xC2, etc.)
        let is_sof = matches!(
            marker,
            0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD | 0xCE | 0xCF
        );

        if is_sof && width.is_none() {
            let mut sof_buf = [0u8; 5]; // [precision, h_hi, h_lo, w_hi, w_lo]
            if reader.read_exact(&mut sof_buf).is_ok() {
                let h = ((sof_buf[1] as u32) << 8) | (sof_buf[2] as u32);
                let w = ((sof_buf[3] as u32) << 8) | (sof_buf[4] as u32);
                height = Some(h);
                width = Some(w);
            }
        }

        // Advance to next marker
        pos += 2 + seg_len; // 2 for marker bytes + payload length
    }

    Ok(None)
}

/// Scans the entropy stream after SOS marker for EOI (0xFF 0xD9), ignoring escaped 0xFF 0x00
fn scan_entropy_for_eoi<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    total_size: u64,
) -> io::Result<Option<u64>> {
    reader.seek(SeekFrom::Start(start))?;
    const CHUNK: usize = 64 * 1024;
    let mut buf = [0u8; CHUNK];
    let mut pos = start;

    let mut prev_ff = false;

    while pos < total_size {
        let to_read = ((total_size - pos) as usize).min(CHUNK);
        let n = reader.read(&mut buf[..to_read])?;
        if n == 0 {
            break;
        }

        for i in 0..n {
            let b = buf[i];
            if prev_ff {
                if b == 0xD9 {
                    // Found EOI!
                    return Ok(Some(pos + i as u64 + 1));
                }
                prev_ff = false;
            } else if b == 0xFF {
                prev_ff = true;
            }
        }

        pos += n as u64;
    }

    Ok(None)
}

/// Fallback scanning for 0xFF 0xD9
fn fallback_find_eoi<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    total_size: u64,
) -> io::Result<Option<u64>> {
    reader.seek(SeekFrom::Start(start + 2))?;
    const CHUNK: usize = 64 * 1024;
    let mut buf = [0u8; CHUNK];
    let mut pos = start + 2;
    let mut prev_ff = false;

    // Scan backwards from end or forwards
    while pos < total_size {
        let to_read = ((total_size - pos) as usize).min(CHUNK);
        let n = reader.read(&mut buf[..to_read])?;
        if n == 0 {
            break;
        }

        for i in 0..n {
            let b = buf[i];
            if prev_ff {
                if b == 0xD9 {
                    return Ok(Some(pos + i as u64 + 1));
                }
                prev_ff = false;
            } else if b == 0xFF {
                prev_ff = true;
            }
        }
        pos += n as u64;
    }
    Ok(None)
}

/// Fallback dimension reader
fn read_dimensions_fallback<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    end: u64,
) -> io::Result<(Option<u32>, Option<u32>)> {
    reader.seek(SeekFrom::Start(start))?;
    let len = ((end - start) as usize).min(1024 * 1024); // read up to first 1MB of header
    let mut header = vec![0u8; len];
    reader.read_exact(&mut header)?;

    for i in 0..header.len().saturating_sub(9) {
        if header[i] == 0xFF {
            let marker = header[i + 1];
            if matches!(
                marker,
                0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD | 0xCE | 0xCF
            ) {
                let h = ((header[i + 5] as u32) << 8) | (header[i + 6] as u32);
                let w = ((header[i + 7] as u32) << 8) | (header[i + 8] as u32);
                if w > 0 && h > 0 {
                    return Ok((Some(w), Some(h)));
                }
            }
        }
    }
    Ok((None, None))
}

/// Stream-extracts raw JPEG bytes from `reader` at `[start_offset, start_offset + length]`
/// directly into `writer` using 64 KB buffered chunks without allocating large in-memory buffers.
pub fn stream_extract_jpeg<R: Read + Seek, W: Write>(
    reader: &mut R,
    writer: &mut W,
    start_offset: u64,
    length: u64,
) -> io::Result<u64> {
    reader.seek(SeekFrom::Start(start_offset))?;
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut buffer = [0u8; CHUNK_SIZE];
    let mut remaining = length;
    let mut written = 0u64;

    while remaining > 0 {
        let to_read = (remaining as usize).min(CHUNK_SIZE);
        let n = reader.read(&mut buffer[..to_read])?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Unexpected EOF while stream-extracting JPEG",
            ));
        }
        writer.write_all(&buffer[..n])?;
        written += n as u64;
        remaining -= n as u64;
    }

    writer.flush()?;
    Ok(written)
}

/// Reads the raw JPEG bytes into a single vector (used ONLY for single-image PNG decode or thumbnail,
/// then immediately dropped to prevent RAM accumulation).
pub fn read_jpeg_bytes<R: Read + Seek>(
    reader: &mut R,
    start_offset: u64,
    length: u64,
) -> io::Result<Vec<u8>> {
    reader.seek(SeekFrom::Start(start_offset))?;
    let mut buffer = vec![0u8; length as usize];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}
