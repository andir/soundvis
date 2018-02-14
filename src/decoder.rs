// this file is heavily copied by
// https://github.com/nwoeanhinnogaehr/pvoc-rs/blob/master/src/lib.rs
// since the original implementation throws away the bins after computation
// I decided to adjust the code to my needs

use apodize;
use rustfft;
use num::{Float, Complex, ToPrimitive, FromPrimitive};
use std::f64::consts::PI;
use std::sync::Arc;

#[allow(non_camel_case_types)]
type c64 = Complex<f64>;

#[derive(Copy, Clone)]
pub struct Bin {
    pub freq: f64,
    pub amp: f64,
}

impl Bin {
    pub fn new(freq: f64, amp: f64) -> Bin {
        Bin {
            freq: freq,
            amp: amp,
        }
    }
    pub fn empty() -> Bin {
        Bin {
            freq: 0.0,
            amp: 0.0,
        }
    }
}

pub struct Decoder {
    sample_rate: f64,
    frame_size: usize,
    time_res: usize,

    fft: Arc<rustfft::FFT<f64>>,
    window: Vec<f64>,

}

impl Decoder {
    pub fn new(sample_rate: f64, frame_size: usize, time_res: usize) -> Decoder {
        let mut planner = rustfft::FFTplanner::new(false);
        Decoder {
            sample_rate: sample_rate,
            frame_size: frame_size,
            time_res: time_res,
            fft: planner.plan_fft(frame_size),
            window: apodize::hanning_iter(frame_size).collect(),
        }
    }

    pub fn phase_to_frequency(&self, bin: usize, phase: f64) -> f64 {
        let frame_sizef = self.frame_size as f64;
        let freq_per_bin = self.sample_rate / frame_sizef;
        let time_resf = self.time_res as f64;
        let step_size = frame_sizef / time_resf;
        let expect = 2.0 * PI * step_size / frame_sizef;
        let mut tmp = phase;
        tmp -= (bin as f64) * expect;
        let mut qpd = (tmp / PI) as i32;
        if qpd >= 0 {
            qpd += qpd & 1;
        } else {
            qpd -= qpd & 1;
        }
        tmp -= PI * (qpd as f64);
        tmp = time_resf * tmp / (2.0 * PI);
        tmp = (bin as f64) * freq_per_bin + tmp * freq_per_bin;
        tmp
    }

    pub fn to_bins<S>(&self, input: Vec<S>) -> Vec<Vec<Bin>>
    where
        S: Float + ToPrimitive + FromPrimitive,
    {
        assert_eq!(input.len(), self.frame_size);

        let input_buf: Vec<f64> = input.iter().map(|v| v.to_f64().unwrap()).collect();
        let mut output_buf: Vec<Vec<Bin>> = vec![vec![Bin::empty();self.frame_size]; self.time_res];

        let mut fft_in = vec![c64::new(0.0, 0.0); self.frame_size];
        let mut fft_out = vec![c64::new(0.0, 0.0); self.frame_size];
        let frame_sizef = self.frame_size as f64;
        let time_resf = self.time_res as f64;
        let step_size = frame_sizef / time_resf;
        let mut last_phase = vec![0.0; self.frame_size];

        for t in 0..self.time_res {
            let mut analysis_out = vec![Bin::empty(); self.frame_size];
            for i in 0..self.frame_size {
                fft_in[i] = c64::new(input_buf[i] * self.window[i], 0.0);
            }
            self.fft.process(&mut fft_in, &mut fft_out);
            for i in 0..self.frame_size {
                let x = fft_out[i];
                let (amp, phase) = x.to_polar();
                let freq = self.phase_to_frequency(i, phase - last_phase[i]);
                last_phase[i] = phase;

                analysis_out[i] = Bin::new(freq, amp * 2.0);
            }
            output_buf[t] = analysis_out;
        }

        output_buf
    }
}
