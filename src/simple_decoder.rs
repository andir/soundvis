use rustfft;
use num::{Float, Complex, ToPrimitive, FromPrimitive};
use num;
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

const PER_OCTAVE: usize = 48;

impl SimpleDecoder {
    pub fn new_simple() -> SimpleDecoder {
        SimpleDecoder::new(2usize.pow(14), 44100)
    }
    pub fn new(sample_count: usize, sample_rate: usize) -> SimpleDecoder {
        let mut planner = rustfft::FFTplanner::new(false);
        let kammer_ton = 440.0;

        let num_outputs = 7 * PER_OCTAVE;
        //        let simple_freqs = (0..sample_count).map(|v| { v * sample_rate / sample_count }).collect();
        // let complex_freqs : Vec<f64> = (0..num_outputs).map(|v| kammer_ton * 2.0_f64.powf(v as f64 / PER_OCTAVE as f64 - 3.0) / sample_rate as f64 * sample_count as f64).collect();
        //        let weights = complex_freqs.iter().map(|v| { let index = v.floor() as f64; (index as usize, 1.0-(v-index), v-index)}).filter(|&(v,_,_)| (v as usize)+1 < sample_count).collect();
        let complex_freqs = (0..num_outputs)
            .map(|v| {
                (kammer_ton * 2.0_f64.powf(v as f64 / PER_OCTAVE as f64 - 3.0) /
                     sample_rate as f64 * sample_count as f64) as usize
            })
            .filter(|&v| v < sample_count)
            .collect();

        //println!("weights: {:?}", weights);
        SimpleDecoder {
            sample_rate: sample_rate,
            sample_count: sample_count,
            freqs: complex_freqs,
            fft: planner.plan_fft(sample_count),
            window: apodize::hanning_iter(sample_count).collect(),
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
            let val = self.fft_out[index];
            let magnitude = val.norm_sqr().sqrt();
            spectrum[i] = magnitude as f32;
        }
        return spectrum;
    }
}
