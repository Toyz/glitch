use std::path::PathBuf;

use anyhow::{anyhow, Context};
use clap::Parser;
use image::io::Reader as ImageReader;
use image::{ColorType, DynamicImage, GenericImage, GenericImageView, Pixel};

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
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    eprintln!("Input File: {}", args.input);

    // create path buffer
    let path = std::path::Path::new(&args.input);
    // check if file exists
    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist"));
    }

    let mut img = ImageReader::open(path)?.decode()?;
    let mut output_image = DynamicImage::new(img.width(), img.height(), ColorType::Rgb8);

    for e in &args.expressions {
        let tokens = parser::shunting_yard(e)?;
        eprintln!("Expression: {:?}", e);
        eprintln!("Tokens: {:?}", tokens);

        let width = img.width();
        let height = img.height();

        let mut sr = 0u8;
        let mut sg = 0u8;
        let mut sb = 0u8;

        let bounds = bounds::find_non_zero_bounds(&img)
            .ok_or_else(|| anyhow!("Failed to find non-zero bounds"))?;
        let min_x = bounds.min_x();
        let max_x = bounds.max_x();

        let min_y = bounds.min_y();
        let max_y = bounds.max_y();
        eprintln!("Bounds: {:?}", bounds);
        let mut rng = rand::thread_rng();

        for x in min_x..max_x {
            for y in min_y..max_y {
                let colors = img.get_pixel(x, y).to_rgba();
                let [r, g, b, a] = colors.0;

                // The eval function is assumed to be synchronous and CPU-bound
                let result = eval::eval(
                    x,
                    y,
                    width,
                    height,
                    r,
                    g,
                    b,
                    if a == 0 { 255 } else { a },
                    sr,
                    sg,
                    sb,
                    &img,
                    &mut rng,
                    tokens.clone(),
                )
                .with_context(|| format!("Failed to evaluate expression {e:?}"))?;

                sr = result[0];
                sg = result[1];
                sb = result[2];

                output_image.put_pixel(x, y, result);
            }
        }

        img = output_image.clone();
    }

    eprintln!("Saving image...");

    let output_format = image::ImageFormat::from_path(path).unwrap_or_else(|_| {
        eprintln!("Unrecognized image format, defaulting to PNG");
        image::ImageFormat::Png
    });

    let output_file = args.output.map_or_else(
        || {
            let extension = output_format.extensions_str()[0];
            PathBuf::from(format!("output.{extension}"))
        },
        PathBuf::from,
    );

    output_image.save_with_format(&output_file, output_format)?;
    eprintln!("Image saved to {:?}", output_file.to_string_lossy());

    Ok(())
}
