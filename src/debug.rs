
use std::fmt;
use std::path::Path;
use std::fs::File;

use std::io::Write;

pub fn write_gnuplot_data<F, T, O>(filename: &str, data: &[T], mut fun: F)
where
    F: FnMut(&T) -> (O, O),
    O: fmt::Display,
{
    let p = Path::new(filename);

    let mut file = match File::create(&p) {
        Err(e) => panic!("Failed to create file {}: {}", p.display(), e),
        Ok(file) => file,
    };

    for d in data {
        let (x, y) = fun(&d);
        match file.write_fmt(format_args!("{} {}\n", x, y)) {
            Err(e) => panic!("Failed to format to file: {}", e),
            Ok(_) => (),
        }
    }
    file.flush();
}
