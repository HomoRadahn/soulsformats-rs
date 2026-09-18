use std::{
    ffi::{OsStr, c_void},
    io,
};

use libloading::Library;
use oodle::{OodleCompressionLevel, OodleCompressor};

type CompressFn = unsafe extern "C" fn(
    compressor: OodleCompressor,
    raw_buf: *const c_void,
    raw_len: isize,
    comp_buf: *mut c_void,
    level: OodleCompressionLevel,
    options: *const c_void,
    dictionary_base: *const c_void,
    lrm: *const c_void,
    scratch_mem: *mut c_void,
    scratch_size: isize,
) -> isize;

type DecompressFn = unsafe extern "C" fn(
    comp_buf: *const c_void,
    comp_buf_size: isize,
    raw_buf: *mut c_void,
    raw_len: isize,
    fuzz_safe: i32,
    check_crc: i32,
    verbosity: i32,
    dec_buf_base: *mut c_void,
    dec_buf_size: isize,
    callback: *mut c_void,
    callback_user_data: *mut c_void,
    decoder_memory: *mut c_void,
    decoder_memory_size: isize,
    thread_phase: i32,
) -> isize;

type GetCompressedBufferSizeNeededFn =
    unsafe extern "C" fn(compressor: OodleCompressor, raw_size: isize) -> isize;

struct OodleCompat {
    compress: CompressFn,
    decompress: DecompressFn,
    get_compressed_buffer_size_needed: GetCompressedBufferSizeNeededFn,
    _library: Library,
}

impl OodleCompat {
    fn load(path: impl AsRef<OsStr>) -> io::Result<Self> {
        let library = unsafe { Library::new(path) }.map_err(io::Error::other)?;

        let compress: CompressFn = unsafe {
            *library
                .get::<CompressFn>(b"OodleLZ_Compress")
                .map_err(io::Error::other)?
        };
        let decompress: DecompressFn = unsafe {
            *library
                .get::<DecompressFn>(b"OodleLZ_Decompress")
                .map_err(io::Error::other)?
        };
        let get_compressed_buffer_size_needed: GetCompressedBufferSizeNeededFn = unsafe {
            *library
                .get::<GetCompressedBufferSizeNeededFn>(b"OodleLZ_GetCompressedBufferSizeNeeded")
                .map_err(io::Error::other)?
        };

        Ok(Self {
            compress,
            decompress,
            get_compressed_buffer_size_needed,
            _library: library,
        })
    }

    fn compressed_buffer_size(&self, compressor: OodleCompressor, raw_size: usize) -> usize {
        unsafe { (self.get_compressed_buffer_size_needed)(compressor, raw_size as isize) as usize }
    }

    fn compress(
        &self,
        compressor: OodleCompressor,
        level: OodleCompressionLevel,
        input: &[u8],
        output: &mut [u8],
    ) -> io::Result<usize> {
        let result = unsafe {
            (self.compress)(
                compressor,
                input.as_ptr().cast(),
                input.len() as isize,
                output.as_mut_ptr().cast(),
                level,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
            )
        };
        if result <= 0 {
            Err(io::Error::other("Oodle compression failed"))
        } else {
            Ok(result as usize)
        }
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> io::Result<usize> {
        let result = unsafe {
            (self.decompress)(
                input.as_ptr().cast(),
                input.len() as isize,
                output.as_mut_ptr().cast(),
                output.len() as isize,
                1,
                0,
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                3,
            )
        };
        if result <= 0 {
            Err(io::Error::other("Oodle decompression failed"))
        } else {
            Ok(result as usize)
        }
    }
}

fn load() -> io::Result<OodleCompat> {
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(std::env::current_dir()?)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy().to_ascii_lowercase();
        if file_name.starts_with("oo2core") && file_name.ends_with(".dll") {
            candidates.push(entry.path());
        }
        else if file_name.starts_with("liboo2core") {
            candidates.push(entry.path());
        }
    }

    candidates.sort();
    let path = candidates.into_iter().next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no oo2core*.dll was found in the current directory",
        )
    })?;

    OodleCompat::load(path)
}

fn compression_level(level: u8) -> io::Result<OodleCompressionLevel> {
    match level {
        0 => Ok(OodleCompressionLevel::None),
        1 => Ok(OodleCompressionLevel::SuperFast),
        2 => Ok(OodleCompressionLevel::VeryFast),
        3 => Ok(OodleCompressionLevel::Fast),
        4 => Ok(OodleCompressionLevel::Normal),
        5 => Ok(OodleCompressionLevel::Optimal1),
        6 => Ok(OodleCompressionLevel::Optimal2),
        7 => Ok(OodleCompressionLevel::Optimal3),
        8 => Ok(OodleCompressionLevel::Optimal4),
        9 => Ok(OodleCompressionLevel::Optimal5),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported Oodle compression level: {level}"),
        )),
    }
}

pub(crate) fn compress(
    input: &[u8],
    compressor: OodleCompressor,
    level: u8,
) -> io::Result<Vec<u8>> {
    let oodle = load()?;
    let level = compression_level(level)?;
    let output_size = oodle.compressed_buffer_size(compressor, input.len());
    let mut output = vec![0u8; output_size];
    let compressed_size = oodle.compress(compressor, level, input, &mut output)?;
    output.truncate(compressed_size);
    Ok(output)
}

pub(crate) fn decompress(input: &[u8], output_size: usize) -> io::Result<Vec<u8>> {
    let oodle = load()?;
    let mut output = vec![0u8; output_size];
    let written = oodle.decompress(input, &mut output)?;
    output.truncate(written);
    Ok(output)
}
