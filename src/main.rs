#![deny(clippy::perf, clippy::correctness)]
#![warn(
    rust_2018_idioms,
    clippy::nursery,
    clippy::complexity,
    clippy::cognitive_complexity
)]

use std::fs;
use clap::Parser;
use console::{style, Emoji};
use gif::{Encoder, Repeat};
use image::codecs::gif::GifDecoder;
use image::{
    guess_format, AnimationDecoder, DynamicImage, GenericImage, GenericImageView, ImageDecoder,
    ImageFormat, Pixel,
};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::io::{BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use crate::eval::EvalContext;
use crate::parser::Token;

mod bounds;
mod eval;
mod parser;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    /// The expression to evaluate
    #[arg(short, long)]
    expressions: Vec<String>,

    /// The input file
    input: String,

    /// Optional output file (default: output.{png,jpg,gif})
    #[arg(short, long)]
    output: Option<String>,

    /// Open the output file after processing
    #[arg(long, default_value = "false")]
    open: bool,

    /// Disable the state during processing
    #[arg(long, default_value = "false")]
    no_state: bool,

    /// Enable verbose output
    #[arg(short, long, default_value = "false")]
    verbose: bool,
}

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static DOWNLOAD: Emoji<'_, '_> = Emoji("🌍  ", "");
static IMAGE: Emoji<'_, '_> = Emoji("🗃️  ", "");
static ERROR: Emoji<'_, '_> = Emoji("❌  ", "");
static OK: Emoji<'_, '_> = Emoji("✅  ", "");
static EYE: Emoji<'_, '_> = Emoji("👁️  ", "");

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    // If we want to pass the arguments to a function, we need to clone them
    if args.input.starts_with("http") {
        // use the writer
        println!(
            "{} Downloading Image: {}",
            DOWNLOAD,
            style(&args.input).bold().cyan()
        );
    } else {
        println!(
            "{} Local File: {}",
            IMAGE,
            style(&args.input).bold().cyan()
        );
    }

    println!(
        "{} Parsing {} Expression{}...",
        LOOKING_GLASS,
        style(&args.expressions.len()).bold().cyan(),
        if args.expressions.len() > 1 { "s" } else { "" }
    );
    let mut parsed: Vec<(String, Vec<Token>)> = vec![];
    let mut idx = 1;
    let expression_count = args.expressions.len();
    for e in &args.expressions {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")?
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        );
        spinner.set_message(format!("Parsing [{}/{}] {}", idx, expression_count, style(e).bold().cyan()));
        spinner.enable_steady_tick(Duration::from_millis(100));

        let tokens = match parser::shunting_yard(e) {
            Ok(tokens) => tokens,
            Err(err) => {
                spinner.finish_and_clear();

                println!("{} Expression {} failed to parse...", ERROR, style(e).bold().cyan());
                println!("{} {} -> {}", ERROR, style("ERROR").red().bold(), err);
                return Ok(());
            }
        };
        spinner.finish_and_clear();

        println!("{} [{}/{}] Parsed {} tokens from -> {}", OK, idx, expression_count, style(tokens.len()).cyan().bold(), style(e).bold().cyan());

        if args.verbose {
            tokens.clone().iter().for_each(|t| {
                println!("\t{}", t);
            });
        }

        idx += 1;

        parsed.push((e.to_string(), tokens));
    }

    handle_image(&args, &parsed)?;
    Ok(())
}

fn download_image(url: &str) -> anyhow::Result<Vec<u8>> {
    let response = reqwest::blocking::get(url)?;
    let bytes = response.bytes()?;
    let img = bytes.into_iter().collect::<Vec<u8>>();
    Ok(img)
}

fn handle_image(
    args: &Args,
    parsed: &[(String, Vec<Token>)],
) -> anyhow::Result<(), anyhow::Error> {
    let img = match &args.input {
        file if file.starts_with("http") => download_image(&args.input)?,
        file => {
            let path = Path::new(&file);
            let reader = std::fs::File::open(path)?;
            let reader = BufReader::new(reader);
            reader
                .bytes()
                .collect::<Result<Vec<u8>, std::io::Error>>()?
        }
    };
    // let name = match &args.input {
    //     file if file.starts_with("http") => file.split('/').last().expect("last").to_string(),
    //     file => Path::new(&file)
    //         .file_name()
    //         .expect("file name")
    //         .to_str()
    //         .expect("Unable to get filename")
    //         .to_string(),
    // };
    let format = guess_format(&img).unwrap_or(ImageFormat::Png);
    let output = match &args.output {
        Some(ref file) => file.to_owned(),
        None => {
            let ext = match format {
                ImageFormat::Png => "png",
                ImageFormat::Jpeg => "jpg",
                ImageFormat::Gif => "gif",
                _ => return Err(anyhow::anyhow!("Unsupported file format\n")),
            };
            format!("output.{}", ext)
        }
    };

    let expression_count = parsed.len();
    println!(
        "{} Processing {} Expression{}...",
        LOOKING_GLASS,
        style(expression_count).bold().cyan(),
        if expression_count > 1 { "s" } else { "" }
    );

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("🔍 {spinner} {wide_msg}")?
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
    );
    spinner.set_message("Processing...");
    spinner.enable_steady_tick(Duration::from_millis(10));

    match format {
        ImageFormat::Png => {
            let img = image::load_from_memory(&img)?;

            spinner.set_message(format!("{} Processing mode: 󰸭 {}", IMAGE, style("PNG").bold().cyan()));

            let out = process(img, parsed, args.no_state)?;
            out.save_with_format(output.clone(), format)?;
        }
        ImageFormat::Jpeg => {
            let img = image::load_from_memory(&img)?;

            spinner.set_message(format!("{} Processing mode: 󰸭 {}", IMAGE, style("JPEG").bold().cyan()));

            let out = process(img, parsed, args.no_state)?;
            out.save_with_format(output.clone(), format)?;
        }
        ImageFormat::Gif => {

            let mut reader = std::io::Cursor::new(img);
            let decoder = GifDecoder::new(&mut reader)?;
            let [w, h] = [decoder.dimensions().0, decoder.dimensions().1];
            let frames = decoder.into_frames().collect_frames()?;



            let output = std::fs::File::create(output.clone())?;
            let mut img_writer = BufWriter::new(output);
            let mut encoder = Encoder::new(&mut img_writer, w as u16, h as u16, &[])?;
            encoder.set_repeat(Repeat::Infinite)?;

            let new_frames = Mutex::new(Vec::with_capacity(frames.len()));

            spinner.set_message(format!("{} Processing mode: 󰸭 {} with {} frames", IMAGE, style("GIF").bold().cyan(), style(frames.len()).bold().cyan()));

            (0..frames.len()).into_par_iter().for_each(|i| {
                let frame = frames.get(i).expect("Failed to get frame").to_owned();
                let delay = frame.delay().numer_denom_ms().0 as u16;
                let img = frame.into_buffer();
                let out =
                    process(img.into(), parsed, args.no_state).expect("Failed to process frame");
                let mut bytes = out.as_bytes().to_vec();

                let mut new_frame = gif::Frame::from_rgba_speed(w as u16, h as u16, &mut bytes, 10);

                new_frame.delay = delay / 10;
                new_frames
                    .lock()
                    .expect("failed to unlock")
                    .push((i, new_frame));
            });

            let mut frames = new_frames.into_inner().expect("Failed to get frames");
            frames.sort_by(|a, b| a.0.cmp(&b.0));
            for (_, frame) in frames {
                encoder.write_frame(&frame)?;
            }
        }
        _ => return Err(anyhow::anyhow!("Unsupported file format\n")),
    }
    let output_file = Path::new(&output);

    spinner.finish_with_message(format!(
        "Processed {} Expression{}...",
        style(expression_count).bold().cyan(),
        if expression_count > 1 { "s" } else { "" }
    ));

    let absolute_path = fs::canonicalize(output_file).map_or_else(|_| output_file.to_path_buf(), |path| strip_windows_prefix(&path));
    println!(
        "{} Output File: {}",
        IMAGE,
        style(absolute_path.display()).bold().cyan()
    );

    if args.open {
        open::that(output_file)?;

        println!(
            "{} Opened output file with default application...",
            EYE,
        );
    }
    Ok(())
}

fn process(
    mut img: DynamicImage,
    expressions: &[(String, Vec<Token>)],
    no_state: bool,
) -> anyhow::Result<DynamicImage> {
    let mut output_image = DynamicImage::new(img.width(), img.height(), img.color());

    for  val in expressions.iter() {
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
                        ignore_state: no_state,
                    },
                    &img,
                    rand::thread_rng(),
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

fn strip_windows_prefix(path: &Path) -> PathBuf {
    path.to_str().and_then(|s| s.strip_prefix(r"\\?\")).map_or_else(|| path.to_path_buf(), PathBuf::from)
}