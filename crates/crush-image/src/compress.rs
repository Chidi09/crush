use std::io::{Read, Write};
use crush_types::{Result, CrushError};

pub enum CompressionFormat {
    Gzip,
    Zstd,
    Uncompressed,
}

pub fn detect_format(data: &[u8]) -> CompressionFormat {
    if data.len() < 2 {
        return CompressionFormat::Uncompressed;
    }
    match (data[0], data[1]) {
        (0x1f, 0x8b) => CompressionFormat::Gzip,
        (0x28, 0xb5) => CompressionFormat::Zstd,
        _ => CompressionFormat::Uncompressed,
    }
}

pub fn decompress_stream<R: Read>(reader: R, format: CompressionFormat) -> Result<Vec<u8>> {
    match format {
        CompressionFormat::Gzip => {
            let mut decoder = flate2::read::GzDecoder::new(reader);
            let mut buf = Vec::with_capacity(64 * 1024);
            decoder.read_to_end(&mut buf)
                .map_err(|e| CrushError::StorageError(format!("Gzip decompress error: {}", e)))?;
            Ok(buf)
        }
        CompressionFormat::Zstd => {
            let mut decoder = zstd::Decoder::new(reader)
                .map_err(|e| CrushError::StorageError(format!("Zstd decoder error: {}", e)))?;
            let mut buf = Vec::with_capacity(64 * 1024);
            decoder.read_to_end(&mut buf)
                .map_err(|e| CrushError::StorageError(format!("Zstd decompress error: {}", e)))?;
            Ok(buf)
        }
        CompressionFormat::Uncompressed => {
            let mut buf = Vec::new();
            let mut reader = reader;
            reader.read_to_end(&mut buf)
                .map_err(|e| CrushError::StorageError(format!("Read error: {}", e)))?;
            Ok(buf)
        }
    }
}

pub fn decompress_stream_into<R: Read, W: Write>(reader: R, writer: &mut W, format: CompressionFormat) -> Result<()> {
    match format {
        CompressionFormat::Gzip => {
            let mut decoder = flate2::read::GzDecoder::new(reader);
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = decoder.read(&mut buf)
                    .map_err(|e| CrushError::StorageError(format!("Gzip read error: {}", e)))?;
                if n == 0 { break; }
                writer.write_all(&buf[..n])
                    .map_err(|e| CrushError::StorageError(format!("Write error: {}", e)))?;
            }
        }
        CompressionFormat::Zstd => {
            let mut decoder = zstd::Decoder::new(reader)
                .map_err(|e| CrushError::StorageError(format!("Zstd decoder error: {}", e)))?;
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = decoder.read(&mut buf)
                    .map_err(|e| CrushError::StorageError(format!("Zstd read error: {}", e)))?;
                if n == 0 { break; }
                writer.write_all(&buf[..n])
                    .map_err(|e| CrushError::StorageError(format!("Write error: {}", e)))?;
            }
        }
        CompressionFormat::Uncompressed => {
            let mut reader = reader;
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = reader.read(&mut buf)
                    .map_err(|e| CrushError::StorageError(format!("Read error: {}", e)))?;
                if n == 0 { break; }
                writer.write_all(&buf[..n])
                    .map_err(|e| CrushError::StorageError(format!("Write error: {}", e)))?;
            }
        }
    }
    Ok(())
}

pub fn compress_zstd(data: &[u8], level: i32) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut encoder = zstd::Encoder::new(&mut compressed, level.max(1).min(22))
        .map_err(|e| CrushError::StorageError(format!("Zstd encoder error: {}", e)))?;
    encoder.write_all(data)
        .map_err(|e| CrushError::StorageError(format!("Zstd write error: {}", e)))?;
    encoder.finish()
        .map_err(|e| CrushError::StorageError(format!("Zstd finish error: {}", e)))?;
    Ok(compressed)
}

pub fn compress_zstd_parallel(data: &[u8], level: i32) -> Result<Vec<u8>> {
    use rayon::prelude::*;

    let num_chunks = rayon::current_num_threads().max(4);
    let chunk_size = (data.len() / num_chunks).max(64 * 1024);

    let chunks: Vec<&[u8]> = data.par_chunks(chunk_size).collect();

    let compressed_chunks: Result<Vec<Vec<u8>>> = chunks.par_iter()
        .map(|chunk| compress_zstd(chunk, level))
        .collect();

    let compressed = compressed_chunks?;
    let mut result = Vec::with_capacity(data.len() / 3);
    for c in &compressed {
        let len_bytes = (c.len() as u32).to_le_bytes();
        result.extend_from_slice(&len_bytes);
        result.extend_from_slice(c);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_gzip() {
        let gzip_data = [0x1f, 0x8b, 0x08, 0x00];
        assert!(matches!(detect_format(&gzip_data), CompressionFormat::Gzip));
    }

    #[test]
    fn test_detect_zstd() {
        let zstd_data = [0x28, 0xb5, 0x2f, 0xfd];
        assert!(matches!(detect_format(&zstd_data), CompressionFormat::Zstd));
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let data = b"hello crush runtime test data";
        let compressed = compress_zstd(data, 3).unwrap();
        let decompressed = decompress_stream(&compressed[..], CompressionFormat::Zstd).unwrap();
        assert_eq!(&decompressed, data);
    }
}
