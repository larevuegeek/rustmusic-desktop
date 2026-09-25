//! DSF (Sony) container reader: opens a `.dsf` file, parses its container
//! chunks, and streams 1-bit DSD bytes per channel block by block.
//!
//! The chunk parsing is delegated to `audio_metadata::file_format::dsf`
//! (single source of truth for the DSF spec). This module focuses on the
//! streaming side: locating the data chunk, reading per-channel blocks,
//! advancing the playback position, and seeking.
//!
//! Data layout inside a DSF data chunk (per Sony spec):
//! the audio is organised in *interleaved channel blocks*, each block
//! `block_size_per_channel` bytes long. The order is:
//!
//! ```text
//! ch0_block0, ch1_block0, ..., chN_block0,
//! ch0_block1, ch1_block1, ..., chN_block1,
//! ...
//! ```
//!
//! `read_next_blocks()` returns one such super-block at a time
//! (i.e. one block per channel for the current super-block index).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::core::audio_decoder::dsd::dsd_container::DsdContainerReader;
use crate::core::audio_decoder::error::DecodeError;
use crate::core::audio_metadata::file_format::dsf::{parse_dsd_chunk, parse_fmt_chunk};

/// Streaming reader for a DSF file. Holds the file handle plus enough
/// metadata to navigate the data chunk.
pub struct DsfDecoder {
    file: File,
    sample_rate: u32,
    channel_count: u8,
    bits_per_sample: u8,
    sample_count_per_channel: u64,
    block_size_per_channel: u32,
    /// Absolute file offset where the DSD samples start (after the data chunk header).
    data_offset: u64,
    /// Total number of `block_size_per_channel`-byte blocks per channel.
    total_blocks: u64,
    /// Index of the next block to read (0-based).
    current_block_index: u64,
}

impl DsfDecoder {
    /// Open a DSF file and parse its DSD/fmt/data chunks.
    /// Returns an error if the file is not a valid DSF or uses an unsupported variant.
    pub fn open(path: &Path) -> Result<Self, DecodeError> {
        let mut file = File::open(path)?;

        // 1. DSD chunk (28 bytes at offset 0) — gives total file size + metadata pointer
        let _dsd = parse_dsd_chunk(&mut file)?;

        // 2. fmt chunk (52 bytes at offset 28) — gives audio format details
        let fmt = parse_fmt_chunk(&mut file)?;

        // 3. data chunk header (12 bytes at offset 80) — verify magic, payload starts after
        let mut data_header = [0u8; 12];
        file.read_exact(&mut data_header)?;

        if &data_header[0..4] != b"data" {
            return Err(DecodeError::InvalidFormat(format!(
                "Expected 'data' chunk magic, got {:?}",
                &data_header[0..4]
            )));
        }

        // Audio bytes start right after the 12-byte header → offset 92.
        // We don't trust the chunk size field in the header (some encoders write it wrong);
        // we derive total_blocks from sample_count_per_channel which is canonical.
        let data_offset = 92u64;

        // Compute blocks per channel: sample_count → bytes (round up) → blocks (round up)
        let bytes_per_channel = (fmt.sample_count + 7) / 8;
        let block_size = fmt.block_size_per_channel as u64;
        let total_blocks = if block_size == 0 {
            return Err(DecodeError::InvalidFormat(
                "block_size_per_channel is zero".into(),
            ));
        } else {
            (bytes_per_channel + block_size - 1) / block_size
        };

        Ok(Self {
            file,
            sample_rate: fmt.sample_rate,
            channel_count: fmt.channel_count as u8,
            bits_per_sample: fmt.bits_per_sample as u8,
            sample_count_per_channel: fmt.sample_count,
            block_size_per_channel: fmt.block_size_per_channel,
            data_offset,
            total_blocks,
            current_block_index: 0,
        })
    }

    // ─── DSF-specific accessors (not in the trait) ───

    pub fn bits_per_sample(&self) -> u8 {
        self.bits_per_sample
    }
    pub fn sample_count_per_channel(&self) -> u64 {
        self.sample_count_per_channel
    }
    pub fn block_size_per_channel(&self) -> u32 {
        self.block_size_per_channel
    }
}

impl DsdContainerReader for DsfDecoder {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channel_count(&self) -> u8 {
        self.channel_count
    }

    fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.sample_count_per_channel as f64 / self.sample_rate as f64
        }
    }

    /// Read the next per-channel block. Returns `Ok(None)` at end of file.
    /// The outer Vec has `channel_count` entries, each of `block_size_per_channel` bytes.
    /// The last block may include trailing padding zeros (per spec).
    fn read_next_blocks(&mut self) -> Result<Option<Vec<Vec<u8>>>, DecodeError> {
        if self.current_block_index >= self.total_blocks {
            return Ok(None);
        }

        let block_size = self.block_size_per_channel as usize;
        let mut blocks: Vec<Vec<u8>> = Vec::with_capacity(self.channel_count as usize);

        for _ch in 0..self.channel_count {
            let mut block = vec![0u8; block_size];
            self.file.read_exact(&mut block)?;
            blocks.push(block);
        }

        self.current_block_index += 1;
        Ok(Some(blocks))
    }

    /// Reposition the reader to the closest block boundary at the requested time.
    /// Returns the actual time after seek (may differ slightly due to block alignment).
    fn seek_to_seconds(&mut self, seconds: f64) -> Result<f64, DecodeError> {
        let seconds = seconds.max(0.0);
        let target_sample = (seconds * self.sample_rate as f64) as u64;

        // 1 bit per sample → 8 samples per byte
        let target_byte_per_channel = target_sample / 8;
        let target_block = (target_byte_per_channel / self.block_size_per_channel as u64)
            .min(self.total_blocks);

        // File offset of the start of super-block N:
        //   data_offset + N × (channel_count × block_size_per_channel)
        let super_block_size = self.channel_count as u64 * self.block_size_per_channel as u64;
        let file_offset = self.data_offset + target_block * super_block_size;

        self.file.seek(SeekFrom::Start(file_offset))?;
        self.current_block_index = target_block;

        // Actual time after seek (block-aligned)
        let actual_sample = target_block * self.block_size_per_channel as u64 * 8;
        Ok(actual_sample as f64 / self.sample_rate as f64)
    }
}
