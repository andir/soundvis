use super::simple_decoder;
use std::time::{Duration, Instant};


const SAMPLING_DURATION: u64 = 16; // in milliseconds

pub struct Processor {
    decoder: simple_decoder::SimpleDecoder,
    samples: Vec<f32>,
    fresh_samples: usize,
    needed_samples: usize,
    draw_time: Option<Instant>,
}

impl Processor {
    pub fn new(k: usize, sample_rate: usize) -> Self {
        let dec = simple_decoder::SimpleDecoder::new(2usize.pow(k as u32), sample_rate);
        let needed_samples = SAMPLING_DURATION as usize * dec.sample_rate / 1000;
        let samples = vec![0.0; dec.sample_count];
        Processor {
            decoder: dec,
            samples: samples,
            fresh_samples: 0,
            needed_samples: needed_samples,
            draw_time: None,
        }
    }

    fn get_elapsed_time(&mut self) -> Duration {
        let elapsed = if let Some(dt) = self.draw_time {
            dt.elapsed()
        } else {
            let n = Instant::now();
            let e = n.elapsed();
            self.draw_time = Some(n);
            e
        };
        elapsed
    }

    pub fn process(&mut self, samples: Vec<f32>) -> Option<Vec<f32>> {
        let elapsed = self.get_elapsed_time();
        let new = usize::min(samples.len(), self.decoder.sample_count);
        self.fresh_samples += samples.len();
        self.samples.rotate_right(new);
        self.samples.splice(..new, samples.into_iter().take(new));

        // if we are being called too often start skipping frames
        if elapsed < Duration::from_millis(SAMPLING_DURATION) {
            return None
        }

        // if there are enough new samples do all the expensive stuff
        if self.fresh_samples >= self.needed_samples {
            let s = &self.samples[..self.decoder.sample_count];
            let out = self.decoder.decode(s);
            self.fresh_samples = 0;
            self.draw_time = Some(Instant::now());
            Some(out)
        } else {
            None
        }
    }
}
