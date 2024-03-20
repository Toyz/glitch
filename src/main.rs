use std::path::PathBuf;

use clap::Parser;
use image::{ColorType, DynamicImage, GenericImage, GenericImageView, Pixel};
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
    let path = std::path::Path::new(&args.input);
    // check if file exists
    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist"));
    }

    let mut img = ImageReader::open(path)?.decode()?;
    let mut output_image = DynamicImage::new(img.width(), img.height(), ColorType::Rgb8);

    for e in &args.expressions {
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

    println!("Saving image");

    let output_file = match args.output {
        Some(file) => PathBuf::from(file),
        None => {
            let file_extension = path.extension().expect("file extension").to_str().expect("to string");
            PathBuf::from(format!("output.{}", file_extension))
        }
    };

    let format = match output_file.extension().expect("file extension").to_str().expect("to string") {
        "png" => image::ImageFormat::Png,
        "jpg" | "jpeg" => image::ImageFormat::Jpeg,
        "gif" => image::ImageFormat::Gif,
        "bmp" => image::ImageFormat::Bmp,
        "ico" => image::ImageFormat::Ico,
        "tiff" => image::ImageFormat::Tiff,
        "webp" => image::ImageFormat::WebP,
        "hdr" => image::ImageFormat::Hdr,
        _ => return Err(anyhow::anyhow!("Unsupported file format")),
    };

    output_image.save_with_format(output_file, format)?;

    Ok(())
}
