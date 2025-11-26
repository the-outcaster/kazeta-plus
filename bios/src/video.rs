use ffmpeg_next as ffmpeg;
use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context as Scaler, flag::Flags};
use ffmpeg::util::frame::video::Video;
use macroquad::prelude::*;
use std::path::Path;

pub struct VideoPlayer {
    decoder: ffmpeg::decoder::Video,
    input_context: ffmpeg::format::context::Input,
    stream_index: usize,
    scaler: Scaler,
    frame_rgb: Video,
    video_frame: Video,
    pub texture: Texture2D,
    pub width: u32,
    pub height: u32,
    pub duration_secs: f64,

    // [!] NEW FIELDS FOR SYNC
    time_base: f64,      // To convert timestamps to seconds
    frame_ready: bool,   // Do we have a decoded frame waiting?
}

impl VideoPlayer {
    pub fn new(path: &Path) -> Result<Self, String> {
        ffmpeg::init().map_err(|e| e.to_string())?;

        let input_context = input(path).map_err(|e| e.to_string())?;

        let stream = input_context
        .streams()
        .best(Type::Video)
        .ok_or("No video stream found")?;

        let stream_index = stream.index();
        let time_base = f64::from(stream.time_base()); // Save this for later

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
        .map_err(|e| e.to_string())?;

        let decoder = context_decoder.decoder().video().map_err(|e| e.to_string())?;

        let width = decoder.width();
        let height = decoder.height();
        let duration_secs = stream.duration() as f64 * time_base;

        let scaler = Scaler::get(
            decoder.format(),
            width,
            height,
            Pixel::RGBA,
            width,
            height,
            Flags::BILINEAR,
        ).map_err(|e| e.to_string())?;

        let texture = Texture2D::from_image(&Image {
            width: width as u16,
            height: height as u16,
            bytes: vec![0; (width * height * 4) as usize],
        });

        Ok(Self {
            decoder,
            input_context,
            stream_index,
            scaler,
            frame_rgb: Video::empty(),
            video_frame: Video::empty(),
            texture,
            width,
            height,
            duration_secs,
            time_base,
            frame_ready: false, // Start empty
        })
    }

    /// Updates the texture to match the elapsed time.
    /// Returns None if the video has finished.
    pub fn update(&mut self, elapsed_time: f64) -> Option<()> {
        loop {
            // 1. Decode a frame if we don't have one ready
            if !self.frame_ready {
                let mut decoded = false;
                // We loop packets until the decoder gives us a full frame
                for (stream, packet) in self.input_context.packets() {
                    if stream.index() == self.stream_index {
                        let _ = self.decoder.send_packet(&packet);
                        if self.decoder.receive_frame(&mut self.video_frame).is_ok() {
                            decoded = true;
                            self.frame_ready = true;
                            break; // We have a frame, stop reading packets
                        }
                    }
                }
                // If we finished the packet loop without a frame, we are likely at EOF
                if !decoded {
                    return None;
                }
            }

            // 2. Check Synchronization
            let timestamp = self.video_frame.timestamp().unwrap_or(0) as f64 * self.time_base;

            if timestamp > elapsed_time {
                // [!] WAIT: The video is ahead of the audio/clock.
                // We have a frame ready, but it's too early to show it.
                // Return now, and we will check this exact same frame again next update.
                return Some(());
            }

            // 3. Render (Timestamp is <= elapsed, so show it)
            if let Ok(_) = self.scaler.run(&self.video_frame, &mut self.frame_rgb) {
                let data = self.frame_rgb.data(0);
                let stride = self.frame_rgb.stride(0);

                let mut bytes = Vec::with_capacity((self.width * self.height * 4) as usize);
                for i in 0..self.height as usize {
                    let start = i * stride;
                    let end = start + (self.width as usize * 4);
                    bytes.extend_from_slice(&data[start..end]);
                }

                let img = Image {
                    width: self.width as u16,
                    height: self.height as u16,
                    bytes,
                };

                self.texture.update(&img);
            }

            // 4. Consume the frame
            self.frame_ready = false;

            // [!] LOOP AGAIN:
            // If we were behind (timestamp < elapsed), we immediately loop back to step 1
            // to decode the *next* frame. This allows us to "skip" frames to catch up
            // if the game lags, ensuring A/V sync.
        }
    }

    pub fn reset(&mut self) { // allow video to loop (for themes that use a video background)
        // Seek to the beginning of the file
        let _ = self.input_context.seek(0, ..);
        // Clear the frame ready flag so we decode immediately
        self.frame_ready = false;
    }
}
