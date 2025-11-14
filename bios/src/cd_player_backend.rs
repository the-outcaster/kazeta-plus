use cd_da_reader::{CdReader, Toc};
use rodio::{buffer::SamplesBuffer, OutputStream, Sink, Source};
use rodio::OutputStreamBuilder;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Represents the status of the CD player.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerStatus {
    NoDisc,
    DataDisc,
    Scanning,
    Stopped,
    Loading,
    Playing,
    Paused,
}

/// Holds the state and logic for the CD player.
pub struct CdPlayerBackend {
    pub status: PlayerStatus,
    pub toc: Option<Toc>,
    pub current_track: usize,
    pub track_duration: Duration,

    // Rodio audio components
    _stream: Option<OutputStream>, // Must be kept alive
    pub sink: Option<Sink>,

    // Thread management
    audio_thread_handle: Option<thread::JoinHandle<()>>,

    // timeline
    pub playback_start_time: Option<Instant>,
    pub paused_elapsed_time: Option<Duration>,

    // Holds the audio data for the loaded track to enable seeking
    current_track_data: Option<SamplesBuffer>,
}

impl CdPlayerBackend {
    pub fn new() -> Self {
        Self {
            status: PlayerStatus::Scanning,
            toc: None,
            current_track: 0,
            track_duration: Duration::ZERO,
            _stream: None,
            sink: None,
            audio_thread_handle: None,
            playback_start_time: None,
            paused_elapsed_time: None,
            current_track_data: None,
        }
    }

    /// Scans the CD drive for a TOC.
    pub fn scan_disc(&mut self) {
        self.status = PlayerStatus::Scanning;
        self.toc = None;
        thread::sleep(Duration::from_millis(50));

        match CdReader::open("/dev/sr0") {
            Ok(reader) => {
                match reader.read_toc() {
                    Ok(toc) => {
                        let is_audio = !toc.tracks.is_empty()
                        && toc.tracks.iter().all(|t| t.is_audio);

                        if is_audio {
                            self.status = PlayerStatus::Stopped;
                            self.toc = Some(toc);
                        } else {
                            self.status = PlayerStatus::DataDisc;
                            self.toc = None;
                        }
                    }
                    Err(e) => {
                        println!("[CD Player] Failed to read TOC: {:?}", e);
                        self.status = PlayerStatus::NoDisc;
                    }
                }
            }
            Err(_) => {
                self.status = PlayerStatus::NoDisc;
            }
        }
    }

    /// Play a specific track in a new thread.
    // This is your original, correct code.
    pub fn play(backend_arc: Arc<Mutex<Self>>, track_index: usize) {
        // Clone Arc for the new thread
        let backend_clone = backend_arc.clone();

        // Spawn a new thread to load and play the track
        let handle = thread::spawn(move || {
            let track_number: u8;

            // --- 1. Set Status to Loading ---
            {
                let mut backend = backend_clone.lock().unwrap();
                // Stop any previous track
                backend.stop_internal();

                backend.status = PlayerStatus::Loading;
                backend.current_track = track_index;

                let toc = match backend.toc.as_ref() {
                    Some(t) => t,
                    None => {
                        println!("[CD Thread] Error: No TOC found.");
                        backend.status = PlayerStatus::Stopped;
                        return;
                    }
                };

                let track = match toc.tracks.get(track_index) {
                    Some(t) => t,
                    None => {
                        println!("[CD Thread] Error: Track not found.");
                        backend.status = PlayerStatus::Stopped;
                        return;
                    }
                };

                // We still need these for the read_track() call
                track_number = track.number;

                // --- Calculate TRUE Duration ---
                let start_lba = track.start_lba;
                let end_lba: u32;

                if track_index == toc.tracks.len() - 1 {
                    // This is the LAST track. Use the disc's leadout.
                    end_lba = toc.leadout_lba;
                } else {
                    // Any other track. Use the next track's start.
                    let next_track = &toc.tracks[track_index + 1];
                    end_lba = next_track.start_lba;
                }

                let total_lbas_for_track = end_lba - start_lba;
                // 75 frames (LBAs) per second for an audio CD
                let total_seconds = total_lbas_for_track as u64 / 75;

                // Set the *correct* duration for the UI
                backend.track_duration = Duration::from_secs(total_seconds);
            } // Mutex guard is dropped here

            // --- 2. Load Track Data (This is the slow part) ---
            println!("[CD Thread] Opening drive to read track {}...", track_number);
            let reader = match CdReader::open("/dev/sr0") {
                Ok(r) => r,
                Err(e) => {
                    println!("[CD Thread] Failed to open drive: {:?}", e);
                    let mut backend = backend_clone.lock().unwrap();
                    backend.status = PlayerStatus::Stopped;
                    return;
                }
            };

            // We must read the Toc again inside this thread
            let toc = match reader.read_toc() {
                Ok(t) => t,
                Err(e) => {
                    println!("[CD Thread] Failed to read TOC: {:?}", e);
                    let mut backend = backend_clone.lock().unwrap();
                    backend.status = PlayerStatus::Stopped;
                    return;
                }
            };

            // Use read_track, which reads the whole track
            let track_data_bytes = match reader.read_track(&toc, track_number) {
                Ok(data) => data,
                Err(e) => {
                    println!("[CD Thread] Failed to read track: {:?}", e);
                    let mut backend = backend_clone.lock().unwrap();
                    backend.status = PlayerStatus::Stopped;
                    return;
                }
            };
            println!("[CD Thread] Read {} bytes. Converting to f32...", track_data_bytes.len());

            // Convert Vec<u8> (raw bytes) to Vec<i16> (PCM)
            let pcm_data: Vec<i16> = track_data_bytes
            .chunks_exact(2)
            .map(|a| i16::from_le_bytes([a[0], a[1]]))
            .collect();

            // Convert i16 samples to f32 samples
            let f32_data: Vec<f32> = pcm_data.into_iter().map(|s| s as f32 / 32768.0).collect();
            println!("[CD Thread] Converted to {} samples.", f32_data.len());

            // [!] CREATE THE BUFFER HERE, *BEFORE* THE FINAL LOCK
            let source_buffer = SamplesBuffer::new(2, 44100, f32_data);

            // --- 3. Play the Buffer ---
            {
                let mut backend = backend_clone.lock().unwrap();
                if backend.status != PlayerStatus::Loading {
                    // User might have pressed Back while we were loading
                    println!("[CD Thread] Playback cancelled.");
                    return;
                }

                let stream = OutputStreamBuilder::open_default_stream()
                .expect("open default audio stream");
                let sink = Sink::connect_new(&stream.mixer());

                // [!] Use the buffer we created earlier
                sink.append(source_buffer.clone()); // Append a clone

                backend._stream = Some(stream);
                backend.sink = Some(sink);
                backend.status = PlayerStatus::Playing;
                backend.playback_start_time = Some(Instant::now());
                backend.paused_elapsed_time = None;

                // [!] STORE THE BUFFER
                backend.current_track_data = Some(source_buffer);

                println!("[CD Thread] Playback started.");
            }
        });

        // Store the thread handle so we can check on it
        let mut backend = backend_arc.lock().unwrap();
        backend.audio_thread_handle = Some(handle);
    }

    /// Seeks the current track forward or backward by a set duration.
    pub fn seek(&mut self, delta: Duration, forward: bool) {
        // 1. Check if we are in a state to seek (playing or paused)
        if self.status != PlayerStatus::Playing && self.status != PlayerStatus::Paused {
            return;
        }

        // 2. Check if we have the audio data loaded
        let source_data = match self.current_track_data.as_ref() {
            Some(data) => data,
            None => return, // No audio data to seek
        };

        // 3. Get current elapsed time
        let mut current_elapsed = Duration::ZERO;
        if self.status == PlayerStatus::Playing {
            if let Some(start) = self.playback_start_time {
                current_elapsed = start.elapsed();
            }
        } else if self.status == PlayerStatus::Paused {
            current_elapsed = self.paused_elapsed_time.unwrap_or(Duration::ZERO);
        }

        // 4. Calculate new time
        let new_time;
        if forward {
            new_time = (current_elapsed + delta).min(self.track_duration);
        } else {
            // `checked_sub` prevents underflow panic
            new_time = current_elapsed.checked_sub(delta).unwrap_or(Duration::ZERO);
        }

        // 5. Re-create the sink and source
        // Stop and drop the old stream/sink
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self._stream.take();

        // Create new stream and sink
        let new_stream = OutputStreamBuilder::open_default_stream()
        .expect("Failed to open stream for seek");
        let new_sink = Sink::connect_new(&new_stream.mixer());

        // 6. Create new source, skip to the new time, and append
        let new_source = source_data.clone().skip_duration(new_time);
        new_sink.append(new_source);

        // 7. Update state
        self._stream = Some(new_stream);
        self.sink = Some(new_sink);

        if self.status == PlayerStatus::Paused {
            // If we were paused, stay paused at the new location
            self.sink.as_ref().unwrap().pause();
            self.paused_elapsed_time = Some(new_time);
        } else {
            // We were playing, so update the start time to reflect the seek
            self.playback_start_time = Some(Instant::now() - new_time);
        }
    }

    // Internal stop, doesn't wait for thread
    pub fn stop_internal(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self._stream.take();
        self.audio_thread_handle.take(); // Drop the handle
        self.playback_start_time = None;
        self.paused_elapsed_time = None;
        self.current_track_data = None;

        if self.status != PlayerStatus::NoDisc && self.status != PlayerStatus::DataDisc {
            self.status = PlayerStatus::Stopped;
        }
    }

    // Public-facing stop, used for "Back" button
    pub fn stop(&mut self) {
        self.stop_internal();
        self.status = PlayerStatus::Stopped;
    }

    pub fn pause(&mut self) {
        if let Some(sink) = &self.sink {
            if self.status == PlayerStatus::Playing {
                sink.pause();
                self.status = PlayerStatus::Paused;

                // Store how much time has passed
                if let Some(start_time) = self.playback_start_time {
                    self.paused_elapsed_time = Some(start_time.elapsed());
                }
            }
        }
    }

    pub fn resume(&mut self) {
        if let Some(sink) = &self.sink {
            if self.status == PlayerStatus::Paused {
                sink.play();
                self.status = PlayerStatus::Playing;

                // Set new start time based on paused time
                let resume_time = Instant::now() - self.paused_elapsed_time.unwrap_or(Duration::ZERO);
                self.playback_start_time = Some(resume_time);
            }
        }
    }
}
