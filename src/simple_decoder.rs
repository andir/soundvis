use rustfft;
use num::Complex;
use std::sync::Arc;
use apodize;

#[allow(non_camel_case_types)]
type c64 = Complex<f64>;


pub struct SimpleDecoder {
    pub sample_rate: usize,
    pub sample_count: usize,
    pub freqs: Vec<usize>,
    window: Vec<f64>,
    fft: Arc<rustfft::FFT<f64>>,
    fft_in: Vec<c64>,
    fft_out: Vec<c64>,
}

const PER_OCTAVE: usize = 12;
const KAMMER_TON: f64 = 440.0;
const LOW_CUT:usize = 20;

impl SimpleDecoder {
    pub fn new_simple() -> SimpleDecoder {
        SimpleDecoder::new(2usize.pow(14), 44100)
    }
    pub fn new(sample_count: usize, sample_rate: usize) -> SimpleDecoder {
        let mut planner = rustfft::FFTplanner::new(false);

        let num_outputs = 7 * PER_OCTAVE; // FIXME
        let complex_freqs = (0..num_outputs)
            .map(|v| {
                (KAMMER_TON * 2.0_f64.powf(v as f64 / PER_OCTAVE as f64 - 3.0) /
                     sample_rate as f64 * sample_count as f64) as usize
            })
            .filter(|&v| v < sample_count / 2) // Only the lower half of the result buffer contains the meaningful frequencies in the range(0, 22kHz).
            .collect();

        let window = apodize::hanning_iter(sample_count).collect();
        let fft = planner.plan_fft(sample_count);

        SimpleDecoder {
            sample_rate: sample_rate,
            sample_count: sample_count,
            freqs: complex_freqs,
            fft: fft,
            window: window,
            fft_in: vec![c64::new(0.0, 0.0); sample_count],
            fft_out: vec![c64::new(0.0, 0.0); sample_count],
        }
    }


    pub fn decode(&mut self, input: &[f32]) -> Vec<f32> {
        // apply windowing
        assert_eq!(input.len(), self.sample_count);

        // apply window and convert to complex
        for (i, element) in input.iter().enumerate() {
            self.fft_in[i] = c64::new((*element as f64) * self.window[i], 0.0);
        }
        // apply fft
        self.fft.process(&mut self.fft_in, &mut self.fft_out);

        // collect peak magnitude at each frequency
        let mut spectrum = vec![0.0 as f32; self.freqs.len()];
        for (i, &index) in self.freqs.iter().enumerate() {
            // cut off low frequencies
            let magnitude = /*if index < LOW_CUT  { // FIXME: should it always be 20?
                0.0
            } else */{
                let val = self.fft_out[1 + index];
                val.norm_sqr().sqrt()
            };
            spectrum[i] = magnitude as f32;
        }

        return spectrum;
    }
}
