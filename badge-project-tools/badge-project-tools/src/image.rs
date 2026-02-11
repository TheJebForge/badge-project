use std::{fs, path::PathBuf};

use image::{imageops::{replace, FilterType}, ImageResult, RgbImage, RgbaImage};

#[derive(clap::Parser, Debug)]
#[command(
    about="Converts any image data into binary file that can be directly loaded by microcontroller to be sent as display data",
    long_about=None
)]
pub struct ImageCli {
    #[arg(help = "Image file to process")]
    input_file: PathBuf,
    #[arg(help = "Path to write to")]
    output_file: PathBuf,
    #[arg(long = "width", help = "Width of the result raw image", default_value_t = 320)]
    width: u32,
    #[arg(long = "height", help = "Height of the result raw image", default_value_t = 480)]
    height: u32,
    #[arg(short = 'l', help = "To use little endian instead of big endian endian", default_value_t = false)]
    little_endian: bool
}
pub fn process_image(cli: ImageCli) -> anyhow::Result<()> {
    let bytes = fs::read(cli.input_file)?;

    let result: Vec<u8> = encode_image_data(&bytes, cli.width, cli.height, cli.little_endian)?;

    fs::write(cli.output_file, result)?;

    Ok(())
}


pub fn encode_image_data(input: &[u8], width: u32, height: u32, little_endian: bool) -> ImageResult<Vec<u8>> {
    let scaled_image = image::load_from_memory(input)?
        .resize(width, height, FilterType::Lanczos3)
        .to_rgb8();

    let mut image = RgbImage::new(width, height);

    replace(
        &mut image,
        &scaled_image,
        width as i64 / 2 - scaled_image.width() as i64 / 2,
        height as i64 / 2 - scaled_image.height() as i64 / 2
    );

    Ok(image.enumerate_pixels()
        .flat_map(|(_, _, p)| {
            let [r, g, b] = p.0;
            if little_endian {
                rgb_to_565(r, g, b).to_le_bytes()
            } else {
                rgb_to_565(r, g, b).to_be_bytes()
            }
        })
        .collect())
}

pub fn rgb_to_565(r: u8, g: u8, b: u8) -> u16 {
    let r5 = r / 8;
    let g6 = g / 4;
    let b5 = b / 8;

    (r5 as u16) << 11 | (g6 as u16) << 5 | b5 as u16
}
