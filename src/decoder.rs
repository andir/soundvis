// this file is heavily copied by
// https://github.com/nwoeanhinnogaehr/pvoc-rs/blob/master/src/lib.rs
// since the original implementation throws away the bins after computation
// I decided to adjust the code to my needs

use apodize;
use rustfft;
use num::{Float, Complex, ToPrimitive, FromPrimitive};
use std::f64::consts::PI;
use std::sync::Arc;
use std::collections::VecDeque;
use num;

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
    sample_rate: f64, // sample rate e.g. 44100
    octaves: usize,
    ring_buffer: Vec<f64>,
    num_bins: usize,
    ffts: Vec<Arc<rustfft::FFT<f64>>>,
    windows: Vec<(usize, Vec<f64>)>,
    freqs: Vec<usize>,
}

impl Decoder {
    pub fn new(sample_rate: f64, octaves: usize, min_window: usize, max_window: usize) -> Decoder {
        let mut planner = rustfft::FFTplanner::new(false);
        let EASY_LEDS: f64 = 2200.0;
        let NICE_STRETCH = f64::from(EASY_LEDS * (octaves as f64) / 12.0).ceil();
        let num_bins: usize = 12 * octaves * NICE_STRETCH as usize;

        let freqs: Vec<usize> = (0..num_bins)
            .map(|v| {
                let v = v as f64;
                let freq = 440.0 * 2.0_f64.powf((v / 12.0 / NICE_STRETCH) - 3.0) / sample_rate;
                freq as usize
            })
            .collect();
        Decoder {
            sample_rate: sample_rate,
            octaves: octaves,
            ring_buffer: vec![0 as f64; 2usize.pow(max_window as u32)],
            num_bins: num_bins,
            ffts: (min_window..max_window)
                .map(|n| planner.plan_fft(num::pow::pow(2, n)))
                .collect(),
            windows: (min_window..max_window)
                .map(|n| {
                    (n, apodize::hanning_iter(2usize.pow(n as u32)).collect())
                })
                .collect(),
            freqs: freqs,
        }
    }


    fn _update_ring_buffer<S>(&mut self, input: &Vec<S>)
    where
        S: Float + ToPrimitive + FromPrimitive,
    {
        // shift around the ringbuffer & insert new data at end
        self.ring_buffer.rotate_left(input.len());

        let offset = self.ring_buffer.len() - input.len();
        for (i, v) in input.iter().map(|v| v.to_f64().unwrap()).enumerate() {
            self.ring_buffer[offset + i] = v;
        }
        //self.to_bins(input)
    }

    pub fn to_bins<S>(&mut self, input: Vec<S>) -> Vec<f64>
    where
        S: Float + ToPrimitive + FromPrimitive,
    {
        self._update_ring_buffer(&input);

        // initialize one fft_out with the maximum size of samples we expect
        // we will use `element_count` to restrict the range below
        // this should hopefully be a (premature) optimization to reduce the amount of
        // memory allocations for each sample
        let mut fft_out = vec![c64::new(0.0, 0.0); self.ring_buffer.len()];

        // initialize a new buffer for fft input same logic as with fft_out
        let mut fft_in = vec![c64::new(0.0, 0.0); self.ring_buffer.len()];

        // our "output",  we are awaiting self.num_bins bins in the output
        let mut bins: Vec<f64> = vec![0.0; self.windows.len() * self.freqs.len()];
        let mut bins_pos : usize = 0;

        for (i, &(k, ref window)) in self.windows.iter().enumerate() {
            // element_count is the amount of items we should take a look at with the current window
            // element_count is also a divisor for each value after the fft as applied, since the windows are build around 2**n(?!?)
            let element_count = num::pow::pow(2, k);
            let divisor: f64 = element_count as f64;


            // Apply windowing to sample and convert to complex
            // we retrieve the last `element_count` elements form the ring buffer
            {
                let offset = self.ring_buffer.len() - element_count;
                let buffer = &self.ring_buffer[offset..];
                for (i, element) in buffer.iter().enumerate() {
                    fft_in[i] = c64::new(element * window[i], 0.0);
                    fft_out[i] = c64::new(0.0, 0.0); // reset the out value
                }
            }

            // apply fft on multiple workers, the amount of jobs is equal to the amount of elements in fft_in
            self.ffts[i].process(&mut fft_in[..element_count], &mut fft_out[..element_count]);

            // post process fft values
            // we skip the first element (why??) and consume up to element_count elements from fft_out
            // we map the first two values to zero since those are low frequencies we are unable to hear anyway(?)
            // all other values are being divided by element_count (still not sure why)
            let tmp: Vec<f64> = fft_out[1..element_count]
                .iter()
                .enumerate()
                .map(|(i, val)| {
                    // everything below 20 should be zero
                    if i <= 20 {
                        return 0.0;
                    }
                    // get absolute value of real path and divide by 2**k
                    let ret: f64 = val.re.abs() / divisor;
                    ret
                })
                .collect();

            // extract requests frequencies from tmp and add it to the bins
            bins.splice(bins_pos..bins_pos+self.freqs.len(), self.freqs.iter().map(|v| tmp[*v]));
            bins_pos += self.freqs.len();
        }

        bins[..bins_pos].to_vec()
    }
}
