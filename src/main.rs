use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::eval::EvalContext;
use crate::parser::Token;
use clap::Parser;
use gif::{Encoder, Repeat};
use image::codecs::gif::GifDecoder;
use image::io::Reader as ImageReader;
use image::{
    AnimationDecoder, ColorType, DynamicImage, GenericImage, GenericImageView, ImageDecoder, Pixel,
};

mod bounds;
mod eval;
mod parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The expression to evaluate
    #[arg(short, long)]
    expressions: Vec<String>,

    /// The input file
    input: String,

    /// optional output file
    #[arg(short, long)]
    output: Option<String>,

    /// open the output file after processing
    #[arg(long, default_value = "false")]
    open: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("Input File: {}", args.input);

    let path = Path::new(&args.input);
    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist"));
    }

    println!("Parsing expressions");
    let mut parsed: Vec<(String, Vec<Token>)> = vec![];
    for e in &args.expressions {
        let tokens = match parser::shunting_yard(e) {
            Ok(tokens) => tokens,
            Err(err) => {
                println!("Expression: {}", e);
                println!("{}", err);
                return Ok(());
            }
        };

        println!("\tExpression: {:?}", e);
        println!("\tTokens: {:?}", tokens.clone());

        parsed.push((e.to_string(), tokens));
    }

    println!("Consuming expressions");
    let format = get_format(path);
    let output_extension = get_output_extension(path);

    let output_file = match args.output {
        Some(file) => PathBuf::from(file),
        None => PathBuf::from(format!("output.{}", output_extension)),
    };
    let output_file = output_file.as_path();

    let img = ImageReader::open(path)?.decode()?;
    println!("\tProcessing: {}", path.display());
    println!("\tSize: {}x{}", img.width(), img.height());
    match format {
        image::ImageFormat::Png => {
            println!("\tProcessing mode: PNG");

            let out = process(img, parsed)?;
            out.save_with_format(output_file, format)?;
        }
        image::ImageFormat::Jpeg => {
            println!("\tProcessing mode: JPEG");

            let out = process(img, parsed)?;
            out.save_with_format(output_file, format)?;
        }
        image::ImageFormat::Gif => {
            println!("\tProcessing mode: GIF");

            let f = std::fs::File::open(path)?;
            let decoder = GifDecoder::new(BufReader::new(f))?;
            let [w, h] = [decoder.dimensions().0, decoder.dimensions().1];
            let frames = decoder.into_frames().collect_frames()?;
            println!("\tProcessing Frames: {}", frames.len());

            let output = std::fs::File::create(output_file)?;
            let mut writer = BufWriter::new(output);

            let mut encoder = Encoder::new(&mut writer, w as u16, h as u16, &[])?;
            encoder.set_repeat(Repeat::Infinite)?;

            for frame in &frames {
                let frame = frame.clone();
                let delay = frame.delay().numer_denom_ms().0 as u16;
                let img = frame.into_buffer();
                let out = process(img.into(), parsed.clone())?;
                let mut bytes = out.as_bytes().to_vec();

                let mut new_frame = gif::Frame::from_rgba_speed(w as u16, h as u16, &mut bytes, 10);

                new_frame.delay = delay / 10;
                encoder.write_frame(&new_frame)?;
            }
        }
        _ => return Err(anyhow::anyhow!("Unsupported file format")),
    };

    println!("Saved output to: {}", output_file.display());

    if args.open {
        open::that(output_file)?;
    }

    Ok(())
}

fn process(
    mut img: DynamicImage,
    expressions: Vec<(String, Vec<Token>)>,
) -> anyhow::Result<DynamicImage> {
    let mut output_image = DynamicImage::new(img.width(), img.height(), ColorType::Rgba8);

    for val in &expressions {
        let (_, tokens) = val;

        let width = img.width();
        let height = img.height();

        let mut sr = 0u8;
        let mut sg = 0u8;
        let mut sb = 0u8;

        let bounds = bounds::find_non_zero_bounds(&img).expect("Failed to find non-zero bounds");
        let min_x = bounds.min_x();
        let max_x = bounds.max_x();

        let min_y = bounds.min_y();
        let max_y = bounds.max_y();
        let rng = rand::thread_rng();

        for x in min_x..max_x {
            for y in min_y..max_y {
                let colors = img.get_pixel(x, y).to_rgba();

                let result = eval::eval(
                    EvalContext {
                        tokens: tokens.clone(),
                        size: (width, height),
                        rgba: colors.0,
                        saved_rgb: [sr, sg, sb],
                        position: (x, y),
                    },
                    &img,
                    rng.clone(),
                )
                .expect("Failed to evaluate");

                sr = result[0];
                sg = result[1];
                sb = result[2];

                output_image.put_pixel(x, y, result);
            }
        }

        img = output_image.clone();
    }
    Ok(output_image)
}

fn get_format(file: &Path) -> image::ImageFormat {
    match file
        .extension()
        .expect("file extension")
        .to_str()
        .expect("to string")
    {
        "png" => image::ImageFormat::Png,
        "jpg" | "jpeg" => image::ImageFormat::Jpeg,
        "gif" => image::ImageFormat::Gif,
        "bmp" => image::ImageFormat::Bmp,
        "ico" => image::ImageFormat::Ico,
        "tiff" => image::ImageFormat::Tiff,
        "webp" => image::ImageFormat::WebP,
        "hdr" => image::ImageFormat::Hdr,
        _ => panic!("Unsupported file format"),
    }
}

fn get_output_extension(file: &Path) -> &str {
    file.extension()
        .expect("file extension")
        .to_str()
        .expect("to string")
}
