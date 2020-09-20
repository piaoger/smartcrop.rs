
use std::clone::Clone;
use std::fs::File;
use std::path::Path;

use image::DynamicImage::ImageRgb8;
use image::{GenericImage, GenericImageView, ImageBuffer, Rgb, Rgba};

#[derive(Debug)]
pub struct CropResult {
    crops: Vec<CropInfo>,
    pub top_crop: CropInfo,
}

#[derive(Clone, Debug, Default)]
struct CropScore {
    detail: f64,
    saturation: f64,
    skin: f64,
    total: f64,
}

#[derive(Clone, Debug, Default)]
pub struct CropSize {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct CropInfo {
    pub size: CropSize,
    score: CropScore,
}

fn thirds(x: f64) -> f64 {
    let y = ((x - (1. / 3.) + 1.0) % 2.0 * 0.5 - 0.5) * 16.;
    f64::max(1.0 - y * y, 0.0)
}

fn cie(r: f64, g: f64, b: f64) -> f64 {
    0.5126 * b + 0.7152 * g + 0.0722 * r
}

fn sample(pixel: Rgba<u8>) -> f64 {
    let r = pixel[0] as f64;
    let g = pixel[1] as f64;
    let b = pixel[2] as f64;
    cie(r, g, b)
}

fn saturation(pixel: Rgba<u8>) -> f64 {
    let r = pixel[0] as f64;
    let g = pixel[1] as f64;
    let b = pixel[2] as f64;
    let id = vec![r / 255., g / 255., b / 255.];
    let maximum = id.iter().fold(0.0 / 0.0, |m, v| v.max(m));
    let minumum = id.iter().fold(0.0 / 0.0, |m, v| v.min(m));
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

#[derive(Clone, Debug)]
pub struct SmartCrop {
    pub width: u32,
    pub height: u32,
    aspect: i32,
    crop_width: i32,
    crop_height: i32,
    detail_weight: f64,
    skin_color: (f64, f64, f64),
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
    score_down_sample: u32,
    step: u32,
    scale_step: f64,
    min_scale: f64,
    max_scale: f64,
    edge_radius: f64,
    edge_weight: f64,
    outside_importance: f64,
    rule_of_thirds: bool,
    prescale: bool,
    debug: bool,
    // save_quality: i32,    // not support
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
            skin_color: (0.78, 0.57, 0.44),
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
            // save_quality: 90,
            file_type: "JPEG".to_string(),
        }
    }
}

impl SmartCrop {
    pub fn new() -> SmartCrop {
        SmartCrop::default()
    }

    pub fn crop(&mut self, path: &Path, opts: &SmartCrop) -> CropResult {
        let mut options = (*opts).clone();
        let mut img = image::open(path).unwrap();
        let (img_width, img_height) = img.dimensions();

        let mut scale = 1.;
        let mut prescale = 1.;
        if options.width != 0 && options.height != 0 {
            scale = f64::min(
                img_width as f64 / options.width as f64,
                img_height as f64 / options.height as f64,
            );
            options.crop_width = f64::floor(options.width as f64 * scale) as i32;
            options.crop_height = f64::floor(options.height as f64 * scale) as i32;
            // img = 100x100, width = 95x95, scale = 100/95, 1/scale > min
            // don't set minscale smaller than 1/scale
            // -> don't pick crops that need upscaling
            options.min_scale =
                f64::min(options.max_scale, f64::max(1. / scale, (options.min_scale)));
        }

        if options.width != 0 && options.height != 0 && options.prescale != false {
            prescale = 1. / scale / options.min_scale;
            if prescale < 1. {
                img = img.resize(
                    (img_width as f64 * prescale) as u32,
                    (img_height as f64 * prescale) as u32,
                    image::imageops::FilterType::Lanczos3,
                );
                if self.debug {
                    //let ref mut fout = File::create(&Path::new("debug.thumb.jpg")).unwrap();
                    let _ = img.save_with_format("debug.thumb.jpg", image::ImageFormat::Jpeg);
                }
                self.crop_width = f64::floor(options.crop_width as f64 * prescale) as i32;
                self.crop_height = f64::floor(options.crop_height as f64 * prescale) as i32;
            } else {
                prescale = 1.;
            }
        }

        let mut result = self.analyse(img);
        for crop in result.crops.iter_mut() {
            (*crop).size = CropSize {
                x: ((*crop).size.x as f64 / prescale).floor() as u32,
                y: ((*crop).size.y as f64 / prescale).floor() as u32,
                width: ((*crop).size.width as f64 / prescale).floor() as u32,
                height: ((*crop).size.height as f64 / prescale).floor() as u32,
            };
        }

        result.top_crop.size = CropSize {
            x: (result.top_crop.size.x as f64 / prescale).floor() as u32,
            y: (result.top_crop.size.y as f64 / prescale).floor() as u32,
            width: (result.top_crop.size.width as f64 / prescale).floor() as u32,
            height: (result.top_crop.size.height as f64 / prescale).floor() as u32,
        };

        result
    }

    fn detect_edge(
        &mut self,
        img: &image::DynamicImage,
        output: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    ) {
        let (w, h) = img.dimensions();
        for (x, y, output_pixel) in output.enumerate_pixels_mut() {
            let pixel = img.get_pixel(x, y);
            let mut lightness = if x == 0 || x >= w - 1 || y == 0 || y >= h - 1 {
                sample(pixel)
            } else {
                sample(pixel) * 4.
                    - sample(img.get_pixel(x - 1, y))
                    - sample(img.get_pixel(x, y - 1))
                    - sample(img.get_pixel(x, y + 1))
                    - sample(img.get_pixel(x + 1, y))
            };
            lightness = if lightness < 0. {
                0.
            } else if lightness > 255. {
                255.
            } else {
                lightness
            };
            *output_pixel = Rgb([pixel[0], lightness as u8, pixel[2]]);
        }
        if self.debug {
            let _ = output.save("edge.jpg");
        }
    }

    fn detect_skin(
        &mut self,
        img: &image::DynamicImage,
        output: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    ) {
        for (x, y, output_pixel) in output.enumerate_pixels_mut() {
            let pixel = img.get_pixel(x, y);
            let lightness = sample(pixel) / 255.;
            let skin = self.get_skin_color(pixel);
            let r: u8 = if skin > self.skin_threshold
                && lightness >= self.skin_brightness_min
                && lightness <= self.skin_brightness_max
            {
                let mut tr = (skin - self.skin_threshold) * (255. / (1. - self.skin_threshold));
                tr = if tr < 0. {
                    0.
                } else if tr > 255. {
                    255.
                } else {
                    tr
                };
                tr as u8
            } else {
                0
            };
            *output_pixel = Rgb([r, output_pixel[1], output_pixel[2]]);
        }
        if self.debug {
            let _ = output.save("skin.jpg");
        }
    }

    fn detect_saturation(
        &mut self,
        img: &image::DynamicImage,
        output: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    ) {
        for (x, y, output_pixel) in output.enumerate_pixels_mut() {
            let pixel = img.get_pixel(x, y);
            let lightness = sample(pixel) / 255.;
            let sat = saturation(pixel);
            let b: u8 = if sat > self.saturation_threshold
                && lightness >= self.saturation_brightness_min
                && lightness <= self.saturation_brightness_max
            {
                let mut tr =
                    (sat - self.saturation_threshold) * (255. / (1. - self.saturation_threshold));
                tr = if tr < 0. {
                    0.
                } else if tr > 255. {
                    255.
                } else {
                    tr
                };
                tr as u8
            } else {
                0
            };
            *output_pixel = Rgb([output_pixel[0], output_pixel[1], b]);
        }
        if self.debug {
            let _ = output.save("sat.jpg");
        }
    }

    fn get_skin_color(&mut self, pixel: Rgba<u8>) -> f64 {
        let r = pixel[0] as f64;
        let g = pixel[1] as f64;
        let b = pixel[2] as f64;
        let (rt, gt, bt) = self.skin_color;
        let mag = f64::sqrt(r * r + g * g + b * b);
        let (rd, gd, bd) = if mag == 0. {
            (-rt, -gt, -bt)
        } else {
            (r / mag - rt, g / mag - gt, b / mag - bt)
        };

        1. - f64::sqrt(rd * rd + gd * gd + bd * bd)
    }

    fn importance(&mut self, crop: &CropSize, x: u32, y: u32) -> f64 {
        if crop.x > x || x >= crop.x + crop.width || crop.y > y || y >= crop.y + crop.height {
            return self.outside_importance;
        }
        let tx = (x - crop.x) as f64 / crop.width as f64;
        let ty = (y - crop.y) as f64 / crop.height as f64;
        let px = (0.5 - tx).abs() * 2.;
        let py = (0.5 - ty).abs() * 2.;
        // distance from edge
        let dx = f64::max(px - 1.0 + self.edge_radius, 0.);
        let dy = f64::max(py - 1.0 + self.edge_radius, 0.);
        let d = (dx * dx + dy * dy) * self.edge_weight;
        let mut s = 1.41 - (px * px + py * py).sqrt();
        if self.rule_of_thirds {
            s += (f64::max(0., s + d + 0.5) * 1.2) * (thirds(px) + thirds(py));
        }
        s + d
    }

    fn get_score(&mut self, img: &image::DynamicImage, crop: &CropSize) -> CropScore {
        let mut detail = 0.;
        let mut skin = 0.;
        let mut saturation = 0.;
        let (w, h) = img.dimensions();
        let downsample = self.score_down_sample;
        let inv_downsample = 1. / downsample as f64;
        let output_height_downsample = h * downsample;
        let output_width_downsample = w * downsample;

        for y in (0..output_height_downsample).filter(|y| y % downsample == 0) {
            for x in (0..output_width_downsample).filter(|x| x % downsample == 0) {
                let downsample_x = (x as f64 * inv_downsample).floor() as u32;
                let downsample_y = (y as f64 * inv_downsample).floor() as u32;
                let importance = self.importance(crop, x, y);
                let pixel = img.get_pixel(downsample_x, downsample_y);
                let d = pixel[1] as f64 / 255.;
                skin = skin + (pixel[0] as f64) / 255. * (d + self.skin_bias) * importance;
                detail = detail + d * importance;
                saturation =
                    saturation + (pixel[2] as f64) / 255. * (d + self.saturation_bias) * importance;
            }
        }

        let total = (detail * self.detail_weight
            + skin * self.skin_weight
            + saturation * self.saturation_weight)
            / crop.width as f64
            / crop.height as f64;
        CropScore {
            total: total,
            detail: detail,
            skin: skin,
            saturation: saturation,
        }
    }

    fn analyse(&mut self, img: image::DynamicImage) -> CropResult {
        let (size_x, size_y) = img.dimensions();
        let mut output = ImageBuffer::new(size_x, size_y);

        self.detect_edge(&img, &mut output);
        self.detect_skin(&img, &mut output);
        self.detect_saturation(&img, &mut output);

        let score_output = ImageRgb8(output).resize(
            ((size_x as f64 / self.score_down_sample as f64) as f64).ceil() as u32,
            ((size_y as f64 / self.score_down_sample as f64) as f64).ceil() as u32,
            image::imageops::FilterType::Lanczos3,
        );

        let mut top_score = i32::min_value() as f64;
        let mut top_crop: Option<CropInfo> = None;
        let mut crops = self.crops(img);

        for crop in crops.iter_mut() {
            crop.score = self.get_score(&score_output, &crop.size);
            if crop.score.total > top_score {
                top_crop = Some(crop.clone());
                top_score = crop.score.total;
            }
        }

        CropResult {
            crops: crops,
            top_crop: top_crop.unwrap(),
        }
    }

    fn crops(&mut self, img: image::DynamicImage) -> Vec<CropInfo> {
        let mut crops = Vec::new();
        let (w, h) = img.dimensions();
        let min_dimension = if w > h { h } else { w };
        let crop_width = if self.crop_width != 0 {
            self.crop_width
        } else {
            min_dimension as i32
        };
        let crop_height = if self.crop_height != 0 {
            self.crop_height
        } else {
            min_dimension as i32
        };
        let range_min = (self.min_scale * 100.) as u32;
        let range_max = ((self.max_scale + self.scale_step) * 100.) as u32;
        let range_step = (self.scale_step * 100.) as u32;
        let mut scales: Vec<f64> = (range_min..range_max)
            .filter(|v| v % range_step == 0)
            .map(|v| v as f64 / 100.)
            .collect();
        scales.reverse();

        for scale in scales.iter() {
            for y in (0..h).map(|y| y).filter(|y| y % self.step == 0) {
                if !((y as f64 + crop_height as f64 * scale) as u32 <= h) {
                    break;
                }
                for x in (0..w).map(|x| x).filter(|x| x % self.step == 0) {
                    if !((x as f64 + crop_width as f64 * scale) as u32 <= w) {
                        break;
                    }
                    crops.push(CropInfo {
                        size: CropSize {
                            x: x as u32,
                            y: y as u32,
                            width: (crop_width as f64 * scale) as u32,
                            height: (crop_height as f64 * scale) as u32,
                        },
                        score: CropScore {
                            ..CropScore::default()
                        },
                    });
                }
            }
        }
        crops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image;
    use std::fs::File;
    use std::path::Path;

    #[test]
    fn it_works() {
        let mut sc = SmartCrop::new();
        let path = Path::new("test.jpg");
        let mut opts = SmartCrop::default();
        opts.width = 100;
        opts.height = 100 / (100 / 100);
        let result = sc.crop(path, &opts);
        let mut img = image::open(path).unwrap();
        let size = result.top_crop.size;

        let output_img = img.crop(size.x, size.y, size.width, size.height);
        let ref mut fout = File::create(&Path::new("out.jpg")).unwrap();
        let _ = output_img.save(fout, image::ImageFormat::Jpeg);
    }
}
