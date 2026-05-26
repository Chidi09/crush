use tempfile::tempdir;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use crush_image::layer::{safe_unpack, LayerExtractor};
use crush_image::compress::CompressionFormat;

#[test]
fn test_extract_layer_gzip() {
    let dir = tempdir().unwrap();
    let dest = dir.path().join("rootfs");
    std::fs::create_dir_all(&dest).unwrap();

    let mut tar_builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(12);
    header.set_cksum();
    tar_builder.append_data(&mut header, "test.txt", b"hello world\n".as_slice()).unwrap();
    let tar_data = tar_builder.into_inner().unwrap();

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_data).unwrap();
    let gz_data = encoder.finish().unwrap();

    let extractor = LayerExtractor::new(&dest);
    extractor.extract_layer_streamed(std::io::Cursor::new(gz_data), CompressionFormat::Gzip).unwrap();

    let content = std::fs::read_to_string(dest.join("test.txt")).unwrap();
    assert_eq!(content, "hello world\n");
}

#[test]
fn test_safe_unpack_symlink() {
    let dir = tempdir().unwrap();
    let dest = dir.path().join("rootfs");
    std::fs::create_dir_all(&dest).unwrap();

    let mut tar_builder = tar::Builder::new(Vec::new());

    let mut header1 = tar::Header::new_gnu();
    header1.set_entry_type(tar::EntryType::Symlink);
    header1.set_size(0);
    tar_builder.append_link(&mut header1, "valid_abs", "/usr/lib").unwrap();

    std::fs::create_dir_all(dest.join("subdir")).unwrap();
    let mut header2 = tar::Header::new_gnu();
    header2.set_entry_type(tar::EntryType::Symlink);
    header2.set_size(0);
    tar_builder.append_link(&mut header2, "subdir/valid_rel", "../test").unwrap();

    let mut header3 = tar::Header::new_gnu();
    header3.set_entry_type(tar::EntryType::Symlink);
    header3.set_size(0);
    tar_builder.append_link(&mut header3, "subdir/malicious", "../../escaped").unwrap();

    let tar_data = tar_builder.into_inner().unwrap();
    let mut archive = tar::Archive::new(&tar_data[..]);

    let result = safe_unpack(&mut archive, &dest);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Symlink traversal blocked"));
}

#[test]
fn test_safe_unpack_hardlink() {
    let dir = tempdir().unwrap();
    let dest = dir.path().join("rootfs");
    std::fs::create_dir_all(&dest).unwrap();

    let mut tar_builder = tar::Builder::new(Vec::new());

    let mut header1 = tar::Header::new_gnu();
    header1.set_size(5);
    header1.set_cksum();
    tar_builder.append_data(&mut header1, "file.txt", b"hello".as_slice()).unwrap();

    let mut header2 = tar::Header::new_gnu();
    header2.set_entry_type(tar::EntryType::Link);
    header2.set_size(0);
    tar_builder.append_link(&mut header2, "malicious", "../outside.txt").unwrap();

    let tar_data = tar_builder.into_inner().unwrap();
    let mut archive = tar::Archive::new(&tar_data[..]);

    let result = safe_unpack(&mut archive, &dest);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Hardlink traversal blocked"));
}
