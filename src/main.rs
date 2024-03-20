use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use clap::Parser;
use gif::{Encoder, Repeat};
use image::{AnimationDecoder, ColorType, DynamicImage, GenericImage, GenericImageView, ImageDecoder, Pixel};
use image::codecs::gif::GifDecoder;
use image::io::Reader as ImageReader;

mod parser;
mod eval;
mod bounds;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The expression to evaluate
    #[arg(short, long)]
    expressions: Vec<String>,

    /// The input file
    input: String,

    /// optional output file
    #[arg(short,long)]
    output: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("Input File: {}", args.input);

    // create path buffer
    let path = std::path::PathBuf::from(&args.input);
    // check if file exists
    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist"));
    }

    let format = get_format(&path);
    let output_extension = get_output_extension(&path);
    println!("Saving image");

    let output_file = match args.output {
        Some(file) => PathBuf::from(file),
        None => PathBuf::from(format!("output.{}", output_extension)),
    };

    let img = ImageReader::open(&path)?.decode()?;
    match format {
        image::ImageFormat::Png => {
            let out = process(img, args.expressions)?;
            out.save_with_format(output_file, format)?;
        },
        image::ImageFormat::Jpeg => {
            let out = process(img, args.expressions)?;
            out.save_with_format(output_file, format)?;
        },
        image::ImageFormat::Gif => {
            let f = std::fs::File::open(&path)?;
            let decoder = GifDecoder::new(BufReader::new(f))?;
            let [w, h] = [decoder.dimensions().0, decoder.dimensions().1];
            let frames = decoder.into_frames().collect_frames()?;

            let output = std::fs::File::create(&output_file)?;
            let mut writer = BufWriter::new(output);


            let mut encoder = Encoder::new(&mut writer, w as u16, h as u16, &[])?;
            encoder.set_repeat(Repeat::Infinite)?;

            for frame in &frames {
                let frame = frame.clone();
                let delay = frame.delay().numer_denom_ms().0 as u16;
                let img = frame.into_buffer();
                let out = process(img.into(), args.expressions.clone())?;
                let mut bytes = out.as_bytes().to_vec();

                let mut new_frame = gif::Frame::from_rgba_speed(w as u16, h as u16, &mut bytes, 10);

                new_frame.delay = delay / 10;
                encoder.write_frame(&new_frame)?;
            }
        },
        _ => return Err(anyhow::anyhow!("Unsupported file format")),
    };

    Ok(())
}

fn process(mut img: DynamicImage, expressions: Vec<String>) -> anyhow::Result<DynamicImage> {
    let mut output_image = DynamicImage::new(img.width(), img.height(), ColorType::Rgba8);

    for e in &expressions {
        let tokens = parser::shunting_yard(e).map_err(|ee| anyhow::anyhow!(ee.clone()))?;
        println!("Expression: {:?}", e);
        println!("Tokens: {:?}", tokens);

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
        println!("Bounds: {:?}", bounds);
        let mut rng = rand::thread_rng();

        for x in min_x..max_x {
            for y in min_y..max_y {
                let colors = img.get_pixel(x, y).to_rgba();
                let [r, g, b, a] = colors.0;

                // The eval function is assumed to be synchronous and CPU-bound
                let result = eval::eval(x, y, width, height, r, g, b, if a <= 0 { 255 } else { a }, sr, sg, sb, &img, &mut rng, tokens.clone())
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

fn get_format(file: &PathBuf) -> image::ImageFormat {
    match file.extension().expect("file extension").to_str().expect("to string") {
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

fn get_output_extension(file: &PathBuf) -> &str {
    file.extension().expect("file extension").to_str().expect("to string")
}