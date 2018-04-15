
pub trait BeatDetector {

    // feed samples into the detector and returns if a beat was detected within thos samples
    fn analyze(&mut self, samples: &[f32]) -> bool;
}

pub struct SimpleBeatDetector {
    sample_rate: usize,
    needed_samples: usize,
    fresh_samples: usize,
    threshold: f32,
    samples: Vec<f32>,
    power_history: Vec<f32>,
}

impl SimpleBeatDetector {
    pub fn new(sample_rate: usize) -> Self {
        let needed_samples = sample_rate / 50;
        assert!(needed_samples != 0);
        SimpleBeatDetector{
            sample_rate: sample_rate,
            needed_samples: needed_samples,
            fresh_samples: 0,
            threshold: 1.4,
            samples: vec![0.0; needed_samples],
            power_history: vec![0.0; 50],
        }
    }

    fn analyze_samples(&mut self) -> bool {
        // compute power of newest needed_samples samples
        let power = self.samples[..self.needed_samples].iter().map(|v| v.powi(2)).sum();

        // compute the reference power;
        let sum : f32 = self.power_history.iter().sum();
        let reference_level : f32 = sum * self.needed_samples as f32 / self.sample_rate as f32;

        // add new power level to history
        self.power_history.rotate_right(1);
        self.power_history[0] = power;
        // if the power is self.threshold times greater then the average we have a simple beat
        power >= reference_level * self.threshold
    }
}

impl BeatDetector for SimpleBeatDetector {
    fn analyze(&mut self, samples: &[f32]) -> bool {

        // retrieve up to needed_samples samples from the buffer, discard the rest
        let samples: &[f32] = if samples.len() > self.needed_samples {
            &samples[..self.needed_samples]
        } else {
            &samples
        };

        self.fresh_samples += samples.len();

        // rotate samples right and add new samples to the beginning
        self.samples.rotate_right(samples.len());
        self.samples.splice(..samples.len(), samples.iter().map(|v| *v));

        // if there are enough new samples analyze and reset fresh counter
        if self.fresh_samples >= self.needed_samples {
            self.fresh_samples = 0;
            return self.analyze_samples();
        } else { // otherwise just return false
            return false
        }
    }
}

