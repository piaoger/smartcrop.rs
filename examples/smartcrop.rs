extern crate chrono;
extern crate image;
extern crate smartcrop;

use chrono::UTC;
use smartcrop::SmartCrop;
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let file = if env::args().count() == 2 {
        env::args().nth(1).unwrap()
    } else {
        println!("[usage] smartcrop FILE");
        return;
    };

    let path = Path::new(file.as_str());
    let mut sc = SmartCrop::new();
    let mut opts = SmartCrop::default();
    opts.width = 100;
    opts.height = 100;
    let start = UTC::now();
    let result = sc.crop(path, &opts);
    let end = UTC::now();
    let diff = end - start;
    println!("[result]\n{:?}", result);
    println!("time elapsed: {:?}", diff.num_milliseconds());
    let size = result.top_crop.size;

    let mut img = image::open(path).unwrap();
    let output_img = img.crop(size.x, size.y, size.width, size.height);
    let ref mut fout = File::create(&Path::new("out.jpg")).unwrap();
    let save_img = output_img.resize(opts.width, opts.height, image::FilterType::Lanczos3);
    let _ = save_img.save(fout, image::JPEG);
}
