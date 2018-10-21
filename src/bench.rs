use std::time::{Duration, Instant};

pub struct Benchmark {
    last_call: Instant,
    measurements: Vec<Duration>,
}

impl Benchmark {
    pub fn new() -> Benchmark {
        Benchmark {
            last_call: Instant::now(),
            measurements: Vec::with_capacity(100),
        }
    }

    pub fn measure(&mut self) {
        let now = Instant::now();
        let elapsed = self.last_call.elapsed();
        self.last_call = now;
        if self.measurements.len() > 0 {
            self.measurements.rotate_right(1);
            self.measurements[0] = elapsed;
        } else {
            self.measurements.push(elapsed);
        }
    }

    pub fn avg(&self) -> u32 {
        if self.measurements.len() == 0 {
            0
        } else {
            let duration  =self.measurements.iter().fold(
                Duration::new(0, 0),
                |a, b| a + *b,
            ) / self.measurements.len() as u32;

            // return miliseconds
            duration.subsec_nanos() / 1000000
        }
    }
}
