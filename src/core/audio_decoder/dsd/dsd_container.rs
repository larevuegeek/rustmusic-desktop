//! Trait that abstracts over DSD container formats (DSF, DFF, ...).
//!
//! Both `DsfDecoder` and `DffDecoder` implement this trait, so the playback
//! orchestrator `run_dsd_playback` can drive either of them generically.
//!
//! The trait exposes the minimum surface needed by the player loop:
//! source rate, channel count, total duration, sequential block reading,
//! and time-based seek. Format-specific accessors (e.g. block_size_per_channel
//! for DSF, FRM8 size for DFF) stay on the concrete types.

use crate::core::audio_decoder::error::DecodeError;

/// Streaming reader for a DSD container. Produces 1-bit DSD bytes organised
/// per channel, block by block.
pub trait DsdContainerReader {
    /// DSD sample rate in Hz (e.g. 2_822_400 for DSD64).
    fn sample_rate(&self) -> u32;

    /// Number of audio channels (typically 1, 2 or 6).
    fn channel_count(&self) -> u8;

    /// Total stream duration in seconds.
    fn duration_seconds(&self) -> f64;

    /// Read the next chunk of DSD bytes for all channels at once.
    /// Returns `Ok(None)` at end of stream.
    ///
    /// The outer Vec has `channel_count()` entries; each inner Vec contains
    /// the next slice of DSD bytes for that channel. The chunk size is left
    /// to the implementation (DSF uses fixed `block_size_per_channel` blocks,
    /// DFF reads whatever fits in a single buffer).
    fn read_next_blocks(&mut self) -> Result<Option<Vec<Vec<u8>>>, DecodeError>;

    /// Reposition the reader to the closest valid boundary at the requested time.
    /// Returns the actual time after seek (may differ slightly due to alignment).
    fn seek_to_seconds(&mut self, seconds: f64) -> Result<f64, DecodeError>;
}
