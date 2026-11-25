//! Bounded host-side loading of plain ADFs and regular ADF members of archives.

use debugger::launch::{disk_image_is_supported, ADF_BYTES};
use std::borrow::Cow;
use std::io::{Cursor, Read};

pub(crate) const DISK_EXTENSIONS: &[&str] = &["adf", "zip", "7z"];
const MAX_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_7Z_DECODED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_7Z_MEMORY_KIB: usize = 32 * 1024;
pub(crate) const MAX_DISK_CHOICES: usize = 64;

#[derive(Debug, Clone)]
pub(crate) struct DiskImage {
    pub filename: String,
    source: String,
    member: Option<String>,
    pub bytes: Vec<u8>,
}

impl DiskImage {
    pub(crate) fn description(&self) -> Cow<'_, str> {
        self.member.as_ref().map_or_else(
            || Cow::Borrowed(self.source.as_str()),
            |member| Cow::Owned(format!("{} — {member}", self.source)),
        )
    }
}

#[derive(Debug, Default)]
pub(crate) struct DiskSelection {
    pub images: Vec<DiskImage>,
    pub issues: Vec<String>,
}

impl DiskSelection {
    pub(crate) fn issue(source: &str, message: &str) -> Self {
        Self {
            issues: vec![format!("{source}: {message}")],
            ..Self::default()
        }
    }

    fn member_result(&mut self, source: &str, member: &str, result: Result<DiskImage, String>) {
        match result {
            Ok(image) => self.images.push(image),
            Err(issue) => self.issues.push(format!("{source} — {member}: {issue}")),
        }
    }
}

fn extension(filename: &str) -> Option<&str> {
    let (stem, extension) = filename.rsplit_once('.')?;
    (!stem.is_empty()).then_some(extension)
}

pub(crate) fn is_adf(filename: &str) -> bool {
    extension(filename).is_some_and(|suffix| suffix.eq_ignore_ascii_case("adf"))
}

fn invalid_adf_length(size: u64) -> String {
    format!("Unsupported ADF length ({size} bytes). Select a standard 880 KiB ADF.")
}

pub(crate) fn check_disk_source_size(filename: &str, size: u64) -> Result<(), String> {
    let Some(extension) = extension(filename) else {
        return Err("Select an ADF, ZIP or 7z file".into());
    };
    if extension.eq_ignore_ascii_case("adf") {
        return if size == ADF_BYTES as u64 {
            Ok(())
        } else {
            Err(invalid_adf_length(size))
        };
    }
    if extension.eq_ignore_ascii_case("zip") || extension.eq_ignore_ascii_case("7z") {
        return if size <= MAX_ARCHIVE_BYTES {
            Ok(())
        } else {
            Err("Archive exceeds the 64 MiB input limit".into())
        };
    }
    Err("Select an ADF, ZIP or 7z file".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_disk_path(path: &std::path::Path) -> DiskSelection {
    let source = path.display().to_string();
    let read = || -> Result<DiskSelection, String> {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("The source has no Unicode filename")?;
        let metadata = std::fs::metadata(path).map_err(|error| error.to_string())?;
        if !metadata.is_file() {
            return Err("Select a regular file".into());
        }
        check_disk_source_size(filename, metadata.len())?;
        let limit = if is_adf(filename) {
            ADF_BYTES as u64
        } else {
            MAX_ARCHIVE_BYTES
        };
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .and_then(|file| file.take(limit + 1).read_to_end(&mut bytes))
            .map_err(|error| format!("Could not read file: {error}"))?;
        Ok(decode_disk_source(&source, filename, bytes))
    };
    match read() {
        Ok(selection) => selection,
        Err(error) => DiskSelection::issue(&source, &error),
    }
}

pub(crate) fn decode_disk_source(source: &str, filename: &str, bytes: Vec<u8>) -> DiskSelection {
    if let Err(error) = check_disk_source_size(filename, bytes.len() as u64) {
        return DiskSelection::issue(source, &error);
    }
    if is_adf(filename) {
        return DiskSelection {
            images: vec![DiskImage {
                filename: filename.into(),
                source: source.into(),
                member: None,
                bytes,
            }],
            issues: Vec::new(),
        };
    }
    let mut selection = if extension(filename).is_some_and(|ext| ext.eq_ignore_ascii_case("zip")) {
        read_zip(source, &bytes)
    } else {
        read_7z(source, &bytes)
    };
    if selection.images.is_empty() && selection.issues.is_empty() {
        selection = DiskSelection::issue(source, "Archive contains no ADF files");
    }
    selection
}

fn member_filename(member: &str) -> Result<&str, String> {
    let mut components = member.split(['/', '\\']);
    if components
        .clone()
        .any(|part| part.is_empty() || part == ".." || part.contains(':') || part.contains('\0'))
    {
        return Err("Invalid archive member path".into());
    }
    let filename = components
        .next_back()
        .ok_or("Missing archive member filename")?;
    crate::game_library::validate_disk_filename(filename)?;
    Ok(filename)
}

fn read_member(
    source: &str,
    member: &str,
    filename: &str,
    reader: &mut dyn Read,
) -> Result<DiskImage, String> {
    let mut bytes = Vec::with_capacity(ADF_BYTES + 1);
    reader
        .take((ADF_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Corrupt archive member: {error}"))?;
    if !disk_image_is_supported(&bytes) {
        return Err(invalid_adf_length(bytes.len() as u64));
    }
    Ok(DiskImage {
        filename: filename.into(),
        source: source.into(),
        member: Some(member.into()),
        bytes,
    })
}

fn read_zip(source: &str, bytes: &[u8]) -> DiskSelection {
    use zip::result::ZipError;

    let mut archive = match zip::ZipArchive::new(Cursor::new(bytes)) {
        Ok(archive) => archive,
        Err(error) => {
            let reason = match error {
                ZipError::UnsupportedArchive(_) => "Unsupported ZIP archive",
                _ if bytes.starts_with(b"PK") => "Corrupt ZIP archive",
                _ => "Invalid ZIP archive",
            };
            return DiskSelection::issue(source, &format!("{reason}: {error}"));
        }
    };
    let mut selection = DiskSelection::default();
    for index in 0..archive.len() {
        let metadata = match archive.by_index_data(index) {
            Ok(entry) => entry,
            Err(error) => {
                selection
                    .issues
                    .push(format!("{source}: Corrupt ZIP archive: {error}"));
                continue;
            }
        };
        if !metadata.is_file()
            || metadata
                .unix_mode()
                .is_some_and(|mode| !matches!(mode & 0o170000, 0 | 0o100000))
        {
            continue;
        }
        let member = match metadata.name() {
            Ok(member) if is_adf(&member) => member.into_owned(),
            Ok(_) => continue,
            Err(error) => {
                selection
                    .issues
                    .push(format!("{source}: Invalid ZIP member name: {error}"));
                continue;
            }
        };
        if selection.images.len() >= MAX_DISK_CHOICES {
            selection.issues.push(format!(
                "{source}: Archive exceeds the {MAX_DISK_CHOICES}-image limit"
            ));
            break;
        }
        let encrypted = metadata.encrypted();
        let size = metadata.size();
        let result = (|| {
            let filename = member_filename(&member)?;
            if encrypted {
                return Err("Encrypted ZIP member; choose an unencrypted archive".into());
            }
            if size != ADF_BYTES as u64 {
                return Err(invalid_adf_length(size));
            }
            let mut entry = archive.by_index(index).map_err(|error| match error {
                ZipError::UnsupportedArchive(_) | ZipError::CompressionMethodNotSupported(_) => {
                    format!("Unsupported ZIP member: {error}")
                }
                _ => format!("Corrupt ZIP member: {error}"),
            })?;
            read_member(source, &member, filename, &mut entry)
        })();
        selection.member_result(source, &member, result);
    }
    selection
}

fn describe_7z_error(error: sevenz_rust2::Error) -> String {
    use sevenz_rust2::{EncoderMethod, Error};

    match error {
        Error::PasswordRequired => "Encrypted 7z archive; choose an unencrypted archive".into(),
        Error::UnsupportedCompressionMethod(ref method)
            if method == EncoderMethod::AES256_SHA256.name() =>
        {
            "Encrypted 7z archive; choose an unencrypted archive".into()
        }
        Error::MaxMemLimited { .. } => "7z archive exceeds the decoder memory limit".into(),
        Error::BadSignature(_) => format!("Invalid 7z archive: {error}"),
        Error::UnsupportedCompressionMethod(_)
        | Error::UnsupportedVersion { .. }
        | Error::Unsupported(_)
        | Error::ExternalUnsupported => format!("Unsupported 7z archive: {error}"),
        _ => format!("Corrupt 7z archive: {error}"),
    }
}

fn read_7z(source: &str, bytes: &[u8]) -> DiskSelection {
    use sevenz_rust2::{Archive, BlockDecoder, Password};

    if !bytes.starts_with(b"7z\xbc\xaf\x27\x1c") {
        return DiskSelection::issue(source, "Invalid 7z archive signature");
    }
    let mut reader = Cursor::new(bytes);
    let password = Password::empty();
    let archive = match Archive::read_with_memory_limit(&mut reader, &password, MAX_7Z_MEMORY_KIB) {
        Ok(archive) => archive,
        Err(error) => return DiskSelection::issue(source, &describe_7z_error(error)),
    };
    let mut selection = DiskSelection::default();
    let mut members: Vec<_> = (0..archive.blocks.len()).map(|_| Vec::new()).collect();
    for (index, entry) in archive.files.iter().enumerate() {
        let unix_type = entry.windows_attributes >> 16 & 0o170000;
        if entry.is_directory
            || entry.is_anti_item
            || entry.windows_attributes & 0x400 != 0
            || !matches!(unix_type, 0 | 0o100000)
            || !is_adf(&entry.name)
        {
            continue;
        }
        let result = (|| {
            let filename = member_filename(&entry.name)?;
            if archive.is_file_encrypted(index) {
                return Err("Encrypted 7z member; choose an unencrypted archive".into());
            }
            if entry.size != ADF_BYTES as u64 {
                return Err(invalid_adf_length(entry.size));
            }
            let block = archive.stream_map.file_block_index[index]
                .ok_or("Corrupt 7z member: missing its data stream")?;
            members[block].push((index, filename));
            Ok(())
        })();
        if let Err(issue) = result {
            selection
                .issues
                .push(format!("{source} — {}: {issue}", entry.name));
        }
    }

    let mut remaining_output = MAX_7Z_DECODED_BYTES;
    for (block_index, block_members) in members.into_iter().enumerate() {
        if block_members.is_empty() {
            continue;
        }
        let Some(remaining) =
            remaining_output.checked_sub(archive.blocks[block_index].get_unpack_size())
        else {
            selection.issues.push(format!(
                "{source} — {}: 7z archive exceeds the 128 MiB decoded limit",
                archive.files[block_members[0].0].name
            ));
            continue;
        };
        remaining_output = remaining;
        if selection.images.len() >= MAX_DISK_CHOICES {
            selection.issues.push(format!(
                "{source}: Archive exceeds the {MAX_DISK_CHOICES}-image limit"
            ));
            break;
        }
        // Decode solid blocks once. Unrelated entries in the same block are streamed and discarded.
        let block = BlockDecoder::new_with_memory_limit(
            1,
            MAX_7Z_MEMORY_KIB,
            block_index,
            &archive,
            &password,
            &mut reader,
        );
        let mut file_index = archive.stream_map.block_first_file_index[block_index];
        let mut block_members = block_members.into_iter().peekable();
        let mut decoded = DiskSelection::default();
        let result = block.for_each_entries(&mut |entry, input| {
            if block_members
                .peek()
                .is_some_and(|(index, _)| *index == file_index)
            {
                if selection.images.len() + decoded.images.len() >= MAX_DISK_CHOICES {
                    decoded.issues.push(format!(
                        "{source}: Archive exceeds the {MAX_DISK_CHOICES}-image limit"
                    ));
                    return Ok(false);
                }
                if let Some((_, filename)) = block_members.next() {
                    let result = read_member(source, &entry.name, filename, input);
                    decoded.member_result(source, &entry.name, result);
                }
            }
            file_index += 1;
            Ok(true)
        });
        match result {
            Ok(_) => {
                selection.images.extend(decoded.images);
                selection.issues.extend(decoded.issues);
            }
            Err(error) => selection
                .issues
                .push(format!("{source}: {}", describe_7z_error(error))),
        }
    }
    selection
}
