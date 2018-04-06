use rustfft;
use num::{Float, Complex, ToPrimitive, FromPrimitive};
use num;
use std::sync::Arc;
use apodize;

#[allow(non_camel_case_types)]
type c64 = Complex<f64>;


pub struct SimpleDecoder {
    sample_rate: usize,
    pub sample_count: usize,
    pub freqs: Vec<usize>,
    window: Vec<f64>,
    fft: Arc<rustfft::FFT<f64>>,
}

impl SimpleDecoder {
    pub fn new_simple() -> SimpleDecoder {
        SimpleDecoder::new(2usize.pow(15), 44100)
    }
    pub fn new(sample_count: usize, sample_rate: usize) -> SimpleDecoder {
        let mut planner = rustfft::FFTplanner::new(false);
        let kammer_ton = 440.0;
        let num_outputs = 6 * 12;
//        let simple_freqs = (0..sample_count).map(|v| { v * sample_rate / sample_count }).collect();
        let complex_freqs = (0..num_outputs).map(|v| kammer_ton * 2.0_f64.powf(v as f64 / 12.0 - 3.0) / sample_rate as f64 * sample_count as f64).map(|v| v as usize).collect();
        println!("complex_freqs: {:?}", complex_freqs);
        SimpleDecoder {
            sample_rate: sample_rate,
            sample_count: sample_count,
            freqs: complex_freqs,
            fft: planner.plan_fft(sample_count),
            window: apodize::hanning_iter(sample_count).collect(),
        }
    }


    pub fn decode(&self, input: &[f32]) -> Vec<f32> {
            // apply windowing
            assert_eq!(input.len(), self.sample_count);


            // apply window and convert to complex
            let mut fft_in = vec![c64::new(0.0, 0.0); self.sample_count];
            for (i, element) in input.iter().enumerate() {
                fft_in[i] = c64::new((*element as f64) * self.window[i], 0.0);
            }
            // apply fft
            let mut fft_out = vec![c64::new(0.0, 0.0); self.sample_count];
            self.fft.process(&mut fft_in, &mut fft_out);

            // collect peak magnitude at each frequency
            let mut spectrum = vec![0.0 as f32; self.freqs.len()];
            for (i, val) in fft_out.iter().enumerate() {
                let magnitude = (val.re.powi(2) + val.im.powi(2)).sqrt();
                let freq = i * self.sample_rate / self.sample_count;
                for n in 0..self.freqs.len() - 1 {
                   if freq >= self.freqs[n] && freq <= self.freqs[n+1] {
                       spectrum[n] = magnitude as f32;
                   }
                }
            }

            return spectrum;
    }
}
