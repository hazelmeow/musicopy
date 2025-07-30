use anyhow::Context;
use dashmap::DashMap;
use rubato::{FftFixedIn, Resampler};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use symphonia::core::{
    formats::{TrackType, probe::Hint},
    io::MediaSourceStream,
};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey(String, Vec<u8>);

/// An in-memory cache of the transcoding status of files.
///
/// This is populated on startup by reading the transcode cache directory and
/// updated as files are transcoded. It's initialized in Core and passed down
/// to TranscodePool because it needs to be shared with Node as well.
///
/// We need to key by hash because rescanning causes file IDs to change and
/// can happen at any time. This also accounts for multiple copies of the same
/// file existing in the library.
#[derive(Debug, Clone)]
pub struct TranscodeStatusCache(Arc<DashMap<CacheKey, TranscodeStatus>>);

impl TranscodeStatusCache {
    /// Create a new TranscodeStatusCache.
    pub fn new() -> Self {
        TranscodeStatusCache(Arc::new(DashMap::new()))
    }
}

impl Default for TranscodeStatusCache {
    fn default() -> Self {
        Self::new()
    }
}

/// An item in the transcoding queue.
pub struct TranscodeItem {
    pub hash_kind: String,
    pub hash: Vec<u8>,
    pub local_path: PathBuf,
}

/// A command sent to the transcoding pool.
pub enum TranscodeCommand {
    /// Sent when files are added to the library. Files are enqueued if they
    /// aren't already transcoded or in the queue.
    ///
    /// It's inefficient to send files that are already transcoded, but it is
    /// safe to do so, and they will not be transcoded again.
    Add(Vec<TranscodeItem>),

    /// Increase the priority of some files. Sent when files are requested.
    /// This is useful for partial downloads when the library isn't fully
    /// transcoded yet.
    Prioritize(Vec<u64>),

    /// Sent when files are removed from the library. Files are dequeued if
    /// they are currently queued for transcoding.
    Remove(Vec<TranscodeItem>),

    /// Delete transcodes of files that aren't in the library anymore.
    CollectGarbage(Vec<u64>),
}

/// The transcode status of a file.
#[derive(Debug)]
pub enum TranscodeStatus {
    Queued,
    Done(PathBuf),
}

/// A pool of worker threads for transcoding files.
pub struct TranscodePool {
    transcodes_dir: PathBuf,
    status_cache: TranscodeStatusCache,
}

impl TranscodePool {
    /// Create a TranscodePool.
    ///
    /// The transcode status cache is guaranteed to be populated after this
    /// returns.
    pub fn new(transcodes_dir: PathBuf, status_cache: TranscodeStatusCache) -> Self {
        let pool = TranscodePool {
            transcodes_dir,
            status_cache,
        };

        pool.read_transcodes_dir();

        pool
    }

    // initialize the transcode status cache by reading the transcode cache directory
    fn read_transcodes_dir(&self) {
        // create transcode cache directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&self.transcodes_dir) {
            log::error!(
                "failed to create transcode cache directory at {}: {}",
                self.transcodes_dir.display(),
                e
            );
        }

        // read the transcode cache directory
        let items = match std::fs::read_dir(&self.transcodes_dir) {
            Ok(entries) => entries,
            Err(e) => {
                log::error!(
                    "failed to read transcode cache directory at {}: {}",
                    self.transcodes_dir.display(),
                    e
                );
                return;
            }
        };

        // parse transcode cache directory entries
        let items = items
            .filter_map(|entry| match entry {
                Ok(entry) => Some(entry),
                Err(e) => {
                    log::error!("failed to read entry in transcode cache directory: {}", e);
                    None
                }
            })
            .filter_map(|entry| match self.parse_transcodes_dir_entry(&entry) {
                Ok(res) => Some(res),
                Err(e) => {
                    log::error!(
                        "failed to parse transcode cache directory entry at {}: {}",
                        entry.path().display(),
                        e
                    );
                    None
                }
            })
            .collect::<Vec<_>>();

        // update status cache
        for (path, hash_kind, hash) in items {
            self.status_cache
                .0
                .insert(CacheKey(hash_kind, hash), TranscodeStatus::Done(path));
        }
    }

    fn parse_transcodes_dir_entry(
        &self,
        entry: &std::fs::DirEntry,
    ) -> anyhow::Result<(PathBuf, String, Vec<u8>)> {
        // get entry file type
        let file_type = entry.file_type().context("failed to get file type")?;

        // skip non-files
        if !file_type.is_file() {
            anyhow::bail!("entry is not a file");
        }

        // parse file name as <hash kind>-<hash hex>.ext
        let path = entry.path();
        let file_stem = path
            .file_stem()
            .context("file missing name")?
            .to_string_lossy();
        let (hash_kind, hash) = file_stem
            .split_once("-")
            .context("failed to parse file name")?;
        let hash_kind = hash_kind.to_string();
        let hash = hex::decode(hash).context("failed to decode hash bytes")?;

        Ok((path, hash_kind, hash))
    }

    pub async fn run(
        self,
        mut rx: mpsc::UnboundedReceiver<TranscodeCommand>,
    ) -> anyhow::Result<()> {
        let (job_tx, job_rx) = mpsc::unbounded_channel();
        let job_rx = Arc::new(Mutex::new(job_rx));

        // TODO
        for _ in 0..8 {
            TranscodeWorker::new(
                job_rx.clone(),
                self.status_cache.clone(),
                self.transcodes_dir.clone(),
            );
        }

        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        TranscodeCommand::Add(transcode_add_items) => {
                            for item in transcode_add_items {
                                // TODO: remove these clones
                                let key = CacheKey(item.hash_kind.clone(), item.hash.clone());
                                let status = self.status_cache.0.get(&key);
                                if status.is_none() {
                                    log::trace!("TranscodePool: queueing file {}", item.local_path.display());

                                    // set status to queued
                                    self.status_cache.0.insert(
                                        key,
                                        TranscodeStatus::Queued,
                                    );

                                    // send job to a worker
                                    job_tx.send(item).context("failed to send transcode job")?;
                                } else {
                                    log::trace!("TranscodePool: skipping file {}", item.local_path.display());
                                }
                            }
                        },
                        TranscodeCommand::Prioritize(items) => {
                            // TODO: we need to switch to a priority queue + 0-length bounded channel + dispatcher task
                            todo!()
                        },
                        TranscodeCommand::Remove(items) => {
                            // TODO: this also needs the pqueue because we can't remove the mpsc internal buffer
                            todo!()
                        },
                        TranscodeCommand::CollectGarbage(items) => todo!(),
                    }
                }
            }
        }
    }
}

struct TranscodeWorker {}

impl TranscodeWorker {
    /// Start a new transcode worker thread and return a handle to it.
    pub fn new(
        job_rx: Arc<Mutex<mpsc::UnboundedReceiver<TranscodeItem>>>,
        status_cache: TranscodeStatusCache,
        transcodes_dir: PathBuf,
    ) -> Self {
        std::thread::spawn(move || {
            if let Err(e) = Self::run(job_rx, status_cache, transcodes_dir) {
                log::error!("transcode worker failed: {e:#}");
            }
        });

        Self {}
    }

    /// Implementation of the transcode worker thread.
    fn run(
        job_rx: Arc<Mutex<mpsc::UnboundedReceiver<TranscodeItem>>>,
        status_cache: TranscodeStatusCache,
        transcodes_dir: PathBuf,
    ) -> anyhow::Result<()> {
        loop {
            let job = {
                let mut job_rx = job_rx.lock().expect("failed to lock job receiver");
                let Some(job) = job_rx.blocking_recv() else {
                    log::warn!("transcode worker receiver closed, shutting down");
                    break;
                };
                job
            };

            // TODO: write to temp filename and move after, clean up on startup
            let output_path =
                transcodes_dir.join(format!("{}-{}.ogg", job.hash_kind, hex::encode(&job.hash)));

            log::info!("transcoding file: {}", job.local_path.display());
            if let Err(e) = transcode(&job.local_path, &output_path) {
                log::error!(
                    "failed to transcode file: {} -> {}: {e:#}",
                    job.local_path.display(),
                    output_path.display()
                );

                // try to remove the output file
                let _ = std::fs::remove_file(&output_path);

                // TODO: set status to error

                return Err(e).with_context(|| {
                    format!("failed to transcode file: {}", job.local_path.display())
                });
            } else {
                log::info!(
                    "finished transcoding file: {} -> {}",
                    job.local_path.display(),
                    output_path.display()
                );

                // set status to done
                let key = CacheKey(job.hash_kind, job.hash);
                status_cache
                    .0
                    .insert(key, TranscodeStatus::Done(output_path));
            }
        }

        // worker shut down
        Ok(())
    }
}

fn transcode(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    let input_file = File::open(input_path).context("failed to open input file")?;

    let mss = MediaSourceStream::new(Box::new(input_file), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = input_path.extension() {
        hint.with_extension(extension.to_str().unwrap());
    }

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, Default::default(), Default::default())
        .context("failed to probe file format")?;

    // get the default audio track
    let audio_track = format
        .default_track(TrackType::Audio)
        .context("failed to get default audio track")?;
    let audio_track_id = audio_track.id;

    // get codec parameters for the audio track
    let codec_params = audio_track
        .codec_params
        .as_ref()
        .context("failed to get codec parameters")?;
    let audio_codec_params = codec_params
        .audio()
        .context("codec parameters are not audio")?;

    // get channel count and sample rate from codec parameters
    let channel_count = audio_codec_params
        .channels
        .as_ref()
        .context("failed to get channel count from codec params")?
        .count();
    let sample_rate = audio_codec_params
        .sample_rate
        .context("failed to get sample rate from codec params")? as usize;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio_codec_params, &Default::default())
        .context("failed to create decoder")?;

    // decode the audio track into planar samples
    let mut original_samples: Vec<Vec<f32>> = vec![Vec::new(); channel_count];
    loop {
        // decode next packet
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,

            // end of track
            Ok(None) => break,

            Err(e) => {
                return Err(e).context("failed to read packet");
            }
        };

        // skip packets from other tracks
        if packet.track_id() != audio_track_id {
            continue;
        }

        // decode packet
        let audio_buf = decoder.decode(&packet).context("failed to decode packet")?;

        // copy to output buffer
        // symphonia only lets us copy to vecs/slices, which replaces instead of extending
        // we need to manually resize each channel and then copy to mut slices of the new extended areas
        let mut output_slices = Vec::with_capacity(channel_count);
        for channel in &mut original_samples {
            let curr_len = channel.len();
            let new_len = curr_len + audio_buf.frames();
            channel.resize(new_len, 0.0);
            output_slices.push(&mut channel[curr_len..new_len]);
        }
        audio_buf.copy_to_slice_planar(&mut output_slices);
    }

    // construct the encoder before resampling to determine the lookahead
    let mut encoder = opus::Encoder::new(
        48000,
        match channel_count {
            1 => opus::Channels::Mono,
            2 => opus::Channels::Stereo,
            _ => anyhow::bail!("unsupported channel count: {}", channel_count),
        },
        opus::Application::Audio,
    )
    .context("failed to create opus encoder")?;
    encoder
        .set_bitrate(opus::Bitrate::Bits(128000))
        .context("failed to set opus bitrate")?;

    let lookahead_frames = encoder
        .get_lookahead()
        .context("failed to get opus encoder lookahead")? as usize;

    // resample to 48k if needed
    // also pad the start with zeros to account for encoder lookahead. doing
    // this now allows the encoding logic to be simpler and more efficient.
    let mut resampled_samples = if sample_rate != 48000 {
        let mut resampler = FftFixedIn::<f32>::new(
            sample_rate,
            48000,
            1024, // arbitrary
            4,    // arbitrary
            channel_count,
        )
        .context("failed to create resampler")?;

        let delay = resampler.output_delay();

        let original_frames = original_samples[0].len();

        // number of frames after resampling, including zero-padding for encoder lookahead
        let new_frames = (original_frames * 48000 / sample_rate) + lookahead_frames;

        // pre-allocate output buffer with enough capacity
        // TODO: we might need a little more than this, should check its final capacity to see if it gets resized usually
        let mut resampled_samples: Vec<Vec<f32>> =
            vec![Vec::with_capacity(new_frames + delay); channel_count];

        // pad start with zeros
        for channel in resampled_samples.iter_mut() {
            channel.resize(lookahead_frames, 0.0);
        }

        // allocate chunk input slices vec and chunk output buffer
        let mut input_slices: Vec<&[f32]> = vec![&[]; channel_count];
        let mut output_buf = resampler.output_buffer_allocate(true);

        // resample in chunks
        let mut pos = 0;
        loop {
            // get number of frames needed for next chunk
            let frames_needed = resampler.input_frames_next();

            // check if we have enough frames for a full chunk
            if pos + frames_needed > original_frames {
                break;
            }

            // copy reference to slice of original buffer to input slices vec
            for i in 0..channel_count {
                input_slices[i] = &original_samples[i][pos..(pos + frames_needed)];
            }

            // call resampler with chunk input slices vec and chunk output buffer
            let (input_frames, output_frames) = resampler
                .process_into_buffer(&input_slices, &mut output_buf, None)
                .expect("bad inputs to resampler");

            // copy chunk output buffer to resampled samples
            for i in 0..channel_count {
                resampled_samples[i].extend_from_slice(&output_buf[i][0..output_frames]);
            }

            // increment position by number of input frames consumed
            pos += input_frames;
        }

        // resample final chunk with remaining frames
        if pos < original_frames {
            // copy reference to remaining frames in original samples to input buffer
            for i in 0..channel_count {
                input_slices[i] = &original_samples[i][pos..original_frames];
            }

            let (_input_frames, output_frames) = resampler
                .process_partial_into_buffer(Some(&input_slices), &mut output_buf, None)
                .expect("bad inputs to resampler");

            // copy chunk output buffer to resampled samples
            for i in 0..channel_count {
                resampled_samples[i].extend_from_slice(&output_buf[i][0..output_frames]);
            }
        }

        // continue feeding zeros to the resampler until we have enough frames
        // this ensures we account for resample delay and push everything through its internal buffer
        while resampled_samples[0].len() < new_frames + delay {
            let (_input_frames, output_frames) = resampler
                .process_partial_into_buffer(None::<&[&[f32]]>, &mut output_buf, None)
                .expect("bad inputs to resampler");

            // copy chunk output buffer to resampled samples
            for i in 0..channel_count {
                resampled_samples[i].extend_from_slice(&output_buf[i][0..output_frames]);
            }
        }

        // remove resample delay frames from the start and truncate to new frame count
        // TODO: can we do this without a copy from .drain()?
        for channel in resampled_samples.iter_mut() {
            channel.drain(0..delay);
            channel.truncate(new_frames);
        }

        resampled_samples
    } else {
        // we don't need to resample, but we still need to pad the start with zeros

        let original_frames = original_samples[0].len();

        let mut resampled_samples = vec![Vec::new(); channel_count];
        for i in 0..channel_count {
            resampled_samples[i].resize(lookahead_frames + original_frames, 0.0);
            resampled_samples[i][lookahead_frames..].copy_from_slice(&original_samples[i][..]);
        }

        resampled_samples
    };

    // interleave samples since opus needs interleaved input
    // TODO: explore SIMD for this
    let interleaved_samples = if channel_count == 2 {
        let mut interleaved_samples = vec![0.0; resampled_samples[0].len() * channel_count];

        for i in 0..resampled_samples[0].len() {
            for j in 0..channel_count {
                interleaved_samples[i * channel_count + j] = resampled_samples[j][i];
            }
        }

        interleaved_samples
    } else if channel_count == 1 {
        resampled_samples.swap_remove(0)
    } else {
        unreachable!();
    };

    let mut output_file = File::create(output_path).context("failed to create output file")?;

    let mut packet_writer = ogg::PacketWriter::new(&mut output_file);

    // we write the number of lookahead frames as pre-skip in the opus header
    // we added this many zeros to the start of the resampled samples to account for encoder lookahead
    // players should skip these frames when decoding
    let preskip_bytes = lookahead_frames.to_le_bytes();

    // input sample rate is always 48000 since we resample to it
    let rate_bytes = 48000u32.to_le_bytes();

    #[rustfmt::skip]
	let opus_header: [u8; 19] = [
        b'O', b'p', b'u', b's', b'H', b'e', b'a', b'd', // magic signature
        1, // version, always 1
        channel_count as u8, // channel count
        preskip_bytes[0], preskip_bytes[1], // pre-skip
        rate_bytes[0], rate_bytes[1], rate_bytes[2], rate_bytes[3], // input sample rate
        0, 0, // output gain
        0, // channel mapping family
    ];

    #[rustfmt::skip]
    let comment_header: [u8; 24] = [
        b'O', b'p', b'u', b's', b'T', b'a', b'g', b's', // magic signature
        0x08, 0x00, 0x00, 0x00, // vendor string length (8u32 in little-endian)
        b'm', b'u', b's', b'i', b'c', b'o', b'p', b'y', // vendor string
        0x00, 0x00, 0x00, 0x00, // no user comments (0u32)
    ];

    // stream unique serial identifier
    let serial = 0;

    // write opus header and comment header packets
    packet_writer
        .write_packet(&opus_header, serial, ogg::PacketWriteEndInfo::EndPage, 0)
        .context("failed to write packet")?;
    packet_writer
        .write_packet(&comment_header, serial, ogg::PacketWriteEndInfo::EndPage, 0)
        .context("failed to write packet")?;

    // number of frames per chunk (48khz / 1000 * 20ms = 960 frames)
    // NB: we are calling opus frames 'chunks' to differentiate from sample frames (one sample per channel)
    let chunk_frames = 48000 / 1000 * 20;
    let chunk_samples = chunk_frames * channel_count;

    let interleaved_len = interleaved_samples.len();

    // encode in chunks
    let mut pos = 0;
    loop {
        // check if we have enough samples for a full chunk
        if pos + chunk_samples > interleaved_len {
            break;
        }

        // allocate chunk output buffer
        // encode_float uses the length (not capacity) as max_data_size
        // length comes from recommended max_data_size in opus documentation
        let mut output_buf = vec![0; 4000];

        // call encoder with input slice and chunk output buffer
        let output_len = encoder
            .encode_float(
                &interleaved_samples[pos..(pos + chunk_samples)],
                &mut output_buf,
            )
            .context("failed to encode chunk")?;
        output_buf.truncate(output_len);

        let end_info = if pos + chunk_samples == interleaved_len {
            // if this chunk ended exactly at the end of input
            ogg::PacketWriteEndInfo::EndStream
        } else {
            ogg::PacketWriteEndInfo::NormalPacket
        };

        // the number of frames up to and including the last frame in this packet
        // this is measured in frames, so mono and stereo increase at the same rate
        let granule_position = ((pos + chunk_samples) / channel_count) as u64;

        // write packet
        packet_writer
            .write_packet(output_buf, serial, end_info, granule_position)
            .context("failed to write packet")?;

        // increment position by number of samples consumed
        pos += chunk_samples;
    }

    // encode final chunk with remaining samples
    if pos < interleaved_len {
        // allocate chunk output buffer
        let mut output_buf = vec![0; 4000];

        // opus always requires a full chunk of input but we don't have enough remaining samples,
        // so allocate a zero-padded input buffer for the final chunk
        let mut input_buf = vec![0.0; chunk_samples];
        input_buf[0..(interleaved_len - pos)]
            .copy_from_slice(&interleaved_samples[pos..interleaved_len]);

        // call encoder with chunk input buffer and chunk output buffer
        let output_len = encoder
            .encode_float(&input_buf, &mut output_buf)
            .context("failed to encode final chunk")?;
        output_buf.truncate(output_len);

        // for end-trimming, the granule position of the final packet is the total number of input frames
        // this may be less than the position of the final frame in the final packet
        // this allows the player to trim the padding samples from the final chunk
        let granule_position = (interleaved_len / channel_count) as u64;

        // write packet
        packet_writer
            .write_packet(
                output_buf,
                serial,
                ogg::PacketWriteEndInfo::EndStream,
                granule_position,
            )
            .context("failed to write packet")?;
    }

    // we did it
    Ok(())
}
