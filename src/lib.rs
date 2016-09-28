extern crate image;

use std::path::Path;
use std::fs::File;
use image::{GenericImage, imageops};


fn thirds(x: f64) -> f64 {
    let y = ((x - (1. / 3.) + 1.0) % 2.0 * 0.5 - 0.5) * 16.;
    f64::max(1.0 - y * y, 0.0)
}

fn cie(r: f64, g: f64, b: f64) -> f64 {
    0.5126 * b + 0.7152 * g + 0.0722 * r
}

fn sample(r: f64, g: f64, b: f64) -> f64 {
    cie(r, g, b)
}

fn saturation(r: f64, g: f64, b: f64) -> f64 {
    let id = vec![r/255., g/255., b/255.];
    let maximum = id.iter().fold(0.0/0.0, |m, v| v.max(m));
    let minumum = id.iter().fold(0.0/0.0, |m, v| v.min(m));
    if maximum == minumum {
        return 0.;
    }
    let l = (maximum + minumum) / 2.;
    let d = maximum - minumum;
    if l > 0.5 {
        d / (2. - maximum - minumum)
    } else {
        d / (maximum + minumum)
    }
}

pub struct SmartCrop {
    width: i32,
    height: i32,
    aspect: i32,
    crop_width: i32,
    crop_height: i32,
    detail_weight: f64,
    skin_color: Vec<f64>,
    skin_bias: f64,
    skin_brightness_min: f64,
    skin_brightness_max: f64,
    skin_threshold: f64,
    skin_weight: f64,
    saturation_brightness_min: f64,
    saturation_brightness_max: f64,
    saturation_threshold: f64,
    saturation_bias: f64,
    saturation_weight: f64,
    // step * minscale rounded down to the next power of two should be good
    score_down_sample: i32,
    step: i32,
    scale_step: f64,
    min_scale: f64,
    max_scale: f64,
    edge_radius: f64,
    edge_weight: f64,
    outside_importance: f64,
    rule_of_thirds: bool,
    prescale: bool,
    debug: bool,
    save_quality: i32,
    file_type: String,
}

impl Default for SmartCrop {
    fn default() -> SmartCrop {
        SmartCrop {
            width: 0,
            height: 0,
            aspect: 0,
            crop_width: 0,
            crop_height: 0,
            detail_weight: 0.2,
            skin_color: vec![0.78, 0.57, 0.44],
            skin_bias: 0.01,
            skin_brightness_min: 0.2,
            skin_brightness_max: 1.0,
            skin_threshold: 0.8,
            skin_weight: 1.8,
            saturation_brightness_min: 0.05,
            saturation_brightness_max: 0.9,
            saturation_threshold: 0.4,
            saturation_bias: 0.2,
            saturation_weight: 0.3,
            // step * minscale rounded down to the next power of two should be good
            score_down_sample: 8,
            step: 8,
            scale_step: 0.1,
            min_scale: 0.9,
            max_scale: 1.0,
            edge_radius: 0.4,
            edge_weight: -20.0,
            outside_importance: -0.5,
            rule_of_thirds: true,
            prescale: true,
            debug: false,
            save_quality: 90,
            file_type: "JPEG".to_string(),
        }
    }
}

impl SmartCrop {
    pub fn new() -> SmartCrop {
        SmartCrop::default()
    }

    pub fn crop(&mut self, path: &Path) -> Result<String, String> {
        let mut options = Self::default();
        let mut img = image::open(path).unwrap();
        let (img_width, img_height) = img.dimensions();

        let mut scale = 1.;
        let mut prescale = 1.;
        if !(options.width == 0) && !(options.height == 0) {
            scale = f64::min(img_width as f64 / options.width as f64,
                             img_height as f64 / options.height as f64);
            options.crop_width = f64::floor(options.width as f64 * scale) as i32;
            options.crop_height = f64::floor(options.height as f64 * scale) as i32;
            // img = 100x100, width = 95x95, scale = 100/95, 1/scale > min
            // don't set minscale smaller than 1/scale
            // -> don't pick crops that need upscaling
            options.min_scale = f64::min(options.max_scale,
                                         f64::max(1. / scale, (options.min_scale)));
        }

        if options.width != 0 && options.height != 0 && options.prescale != false {
            prescale = 1. / scale / options.min_scale;
            if prescale < 1. {
                img = img.resize((img_width as f64 * prescale) as u32,
                                 (img_height as f64 * prescale) as u32,
                                 image::FilterType::Lanczos3);
                    self.crop_width = f64::floor(options.crop_width as f64 * prescale) as i32;
                    self.crop_height = f64::floor(options.crop_height as f64 * prescale) as i32;
                  let ref mut fout = File::create(&Path::new("d.thumb.jpg")).unwrap();
                      // Write the contents of this image to the Writer in PNG format.
                  img.save(fout, image::JPEG).unwrap();
            } else {
                prescale = 1.;
            }
        }
        //result = self.analyse(image)
        //for i in range(len(result['crops'])):
        //    crop = result['crops'][i]
        //    crop['x'] = int(math.floor(crop['x'] / prescale))
        //    crop['y'] = int(math.floor(crop['y'] / prescale))
        //    crop['width'] = int(math.floor(crop['width'] / prescale))
        //    crop['height'] = int(math.floor(crop['height'] / prescale))
        //    result['crops'][i] = crop
        //return result

        Ok("ok".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::*;

    #[test]
    fn it_works() {
        let mut sc = SmartCrop::new();
        let path = Path::new("test.jpg");
        sc.crop(path);
    }
}
