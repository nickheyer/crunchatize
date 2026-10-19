use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const TARGET_BUFFER_MS: u64 = 120;

// Pushes mixed stereo samples to the default output device through a shared queue
pub struct Audio {
    queue: Arc<Mutex<VecDeque<i16>>>,
    rate: u32,
    _stream: cpal::Stream,
}

impl Audio {
    pub fn open() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no audio output device")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        let rate = config.sample_rate();
        let channels = config.channels() as usize;
        let queue = Arc::new(Mutex::new(VecDeque::<i16>::new()));
        let consumer = Arc::clone(&queue);
        let stream = device
            .build_output_stream(
                config.config(),
                move |out: &mut [f32], _| {
                    let mut q = consumer.lock().unwrap();
                    for frame in out.chunks_mut(channels) {
                        let l = q.pop_front().unwrap_or(0) as f32 / 32768.0;
                        let r = q.pop_front().unwrap_or(0) as f32 / 32768.0;
                        for (i, s) in frame.iter_mut().enumerate() {
                            *s = if i % 2 == 0 { l } else { r };
                        }
                    }
                },
                |e| eprintln!("audio stream error: {e}"),
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self { queue, rate, _stream: stream })
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    // Number of stereo frames to render so the queue stays near the target depth
    pub fn frames_wanted(&self) -> usize {
        let target = (self.rate as u64 * TARGET_BUFFER_MS / 1000) as usize;
        let queued = self.queue.lock().unwrap().len() / 2;
        target.saturating_sub(queued)
    }

    pub fn push(&self, samples: &[i16]) {
        self.queue.lock().unwrap().extend(samples.iter().copied());
    }
}
