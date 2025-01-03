#![deny(clippy::perf, clippy::correctness)]
#![warn(
    rust_2018_idioms,
    clippy::complexity,
    clippy::nursery,
)]

use crate::eval::EvalContext;
use clap::Parser;
use console::{style, Emoji};
use dirs::home_dir;
use gif::{Encoder, Repeat};
use image::codecs::gif::GifDecoder;
use image::codecs::webp::WebPDecoder;
use image::{guess_format, AnimationDecoder, DynamicImage, Frame, GenericImage, GenericImageView, ImageDecoder, ImageFormat, Pixel, RgbaImage};
use indicatif::{ProgressBar, ProgressStyle};
use rand::prelude::StdRng;
use rand::{RngCore, SeedableRng};
use rayon::prelude::*;
use std::fs;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::iter::Filter;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use webp_animation::EncoderOptions;
use crate::token::Token;

mod bounds;
mod eval;
mod parser;
mod token;
mod rgb;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None, author)]
struct Args {
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

    /// Seed for the random number generator (Default: Current time)
    #[arg(short, long)]
    seed: Option<u64>,

    /// Number of threads to use (Default: Number of cores)
    #[arg(long)]
    threads: Option<u64>,

    /// The expressions to evaluate
    #[arg(short, long, required_unless_present = "expression_file", long_help = "The expressions to evaluate")]
    expressions: Vec<String>,

    /// A file containing expressions to evaluate
    #[arg(short = 'f', long, required_unless_present = "expressions", long_help = "A file containing expressions to evaluate (Appended to the expressions provided)")]
    expression_file: Option<PathBuf>,
}

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static DOWNLOAD: Emoji<'_, '_> = Emoji("🌍  ", "");
static IMAGE: Emoji<'_, '_> = Emoji("🗃️  ", "");
static ERROR: Emoji<'_, '_> = Emoji("❌  ", "");
static OK: Emoji<'_, '_> = Emoji("✅  ", "");
static EYE: Emoji<'_, '_> = Emoji("👁️  ", "");
static SEED: Emoji<'_, '_> = Emoji("🌱  ", "");

fn main() -> anyhow::Result<()> {
    let mut args = Args::parse();
    if args.threads.is_some() {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads.unwrap() as usize)
            .build_global()
            .expect("Failed to set thread count");
    }

    if args.input.starts_with("http") {
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

    // Determine which RNG to use based on the provided seed
    let seed = get_random_seed(&args);
    args.seed = Some(seed);
    let mut rng: Box<dyn RngCore> = Box::new(StdRng::seed_from_u64(seed));

    println!(
        "{} Using Seed: {}",
        SEED,
        style(seed).bold().cyan()
    );

    if args.expressions.is_empty() && args.expression_file.is_none() {
        println!("{} No expressions provided...", ERROR);
        return Ok(());
    }

    if let Some(path) = &args.expression_file {
        let reader = fs::File::open(path)?;
        let reader = BufReader::new(reader);
        let expressions = reader.lines().collect::<Result<Vec<String>, std::io::Error>>()?;
        let expressions: Vec<_> = Filter::collect(expressions.into_iter().filter(|e| !e.is_empty() && !e.starts_with('#')));

        println!(
            "{} Reading {} Expression{} from file: {}",
            LOOKING_GLASS,
            style(&expressions.len()).bold().cyan(),
            if expressions.len() > 1 { "s" } else { "" },
            style(path.display()).bold().cyan()
        );

        args.expressions.extend(expressions);
    }

    let expression_list_hash = hash_strings(args.expressions.clone());
    let load_parsed_from_cache = get_precompiled_cache(format!("{}", expression_list_hash).as_str());
    let mut parsed: Vec<(String, Vec<Token>)> = vec![];

    let mut from_cache = false;
    if let Some(cache) = load_parsed_from_cache {
        let serialized = fs::read(&cache)?;
        parsed = match bincode::deserialize::<Vec<(String, Vec<Token>)>>(&serialized) {
            Ok(p) => {
                println!(
                    "{} Loaded {} Expression{} from cache...",
                    OK,
                    style(p.len()).bold().cyan(),
                    if p.len() > 1 { "s" } else { "" }
                );

                from_cache = true;
                p
            }
            Err(err) => {
                println!("{} Failed to deserialize cache...", ERROR);
                println!("{} {} -> {}", ERROR, style("ERROR").red().bold(), err);

                // delte the cache file
                fs::remove_file(cache)?;
                vec![]
            }
        }
    }

    if !from_cache {
        println!(
            "{} Parsing {} Expression{}...",
            LOOKING_GLASS,
            style(&args.expressions.len()).bold().cyan(),
            if args.expressions.len() > 1 { "s" } else { "" }
        );
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

        let serialized = bincode::serialize(&parsed)?;
        save_to_cache(format!("{}", expression_list_hash).as_str(), &serialized)?;
    }

    handle_image(&args, &parsed, &mut rng)?;
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
    rand: &mut Box<dyn RngCore>,
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
                ImageFormat::WebP => "webp",
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

    let multi_progress = indicatif::MultiProgress::new();

    if args.verbose {
        // print the filetype
        println!("{} Filetype: {}", IMAGE, style(format!("{:?}", format).to_lowercase()).bold().cyan());
    }

    match format {
        ImageFormat::Png => {
            let img = image::load_from_memory(&img)?;

            println!("{} Processing mode: 󰸭 {}", IMAGE, style("PNG").bold().cyan());

            let out = process(img, parsed, args, rand, Some(ProgressBar::new(0)))?;
            out.save_with_format(output.clone(), format)?;
        }
        ImageFormat::Jpeg => {
            let img = image::load_from_memory(&img)?;

            println!("{} Processing mode: 󰸭 {}", IMAGE, style("JPEG").bold().cyan());

            let out = process(img, parsed, args, rand, Some(ProgressBar::new(0)))?;
            out.save_with_format(output.clone(), format)?;
        }
        ImageFormat::WebP => {
            let reader = std::io::Cursor::new(img);
            let img = WebPDecoder::new(reader)?;
            let w = img.dimensions().0;
            let h = img.dimensions().1;

            let frames = img.into_frames().collect_frames()?;
            let frame_count = frames.len();

            println!("{} Processing mode: 󰸭 {} with {} frames", IMAGE, style("WEBP").bold().cyan(), style(frames.len()).bold().cyan());

            let frames_spin = multi_progress.add(ProgressBar::new(frame_count as u64));
            frames_spin.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")?
            );


            let seed = args.seed.unwrap();

            let new_frames = Mutex::new(Vec::with_capacity(frames.len()));
            (0..frames.len()).into_par_iter().for_each(|i| {
                let pb = multi_progress.add(ProgressBar::new(0));
                pb.enable_steady_tick(Duration::from_millis(100));

                let mut rng: Box<dyn RngCore> = Box::new(StdRng::seed_from_u64(seed));

                let frame = frames.get(i).expect("Failed to get frame").to_owned();
                let delay = frame.delay().numer_denom_ms().0;

                let img = frame.into_buffer();
                let out = process(img.into(), parsed, args, &mut rng, Some(pb)).expect("Failed to process frame");

                let frame = Frame::new(RgbaImage::from(out));
                new_frames.lock().expect("failed to unlock").push((i, (frame, delay)));

                frames_spin.inc(1);
            });

            let mut frames = new_frames.into_inner().expect("Failed to get frames");
            frames.sort_by(|a, b| a.0.cmp(&b.0));

            frames_spin.reset();
            frames_spin.set_length(frames.len() as u64);
            frames_spin.set_message("Encoding frames...");
            let options = EncoderOptions {
                encoding_config: Some(webp_animation::EncodingConfig::new_lossy(100.0)),
                ..Default::default()
            };
            let mut encoder = webp_animation::prelude::Encoder::new_with_options((w, h), options).expect("Failed to create encoder");
            let mut last_ms = 0i32;
            for (i, frame) in frames {
                let buffer = frame.0.into_buffer();

                encoder.add_frame(&buffer, last_ms).unwrap_or_else(|e| panic!("Failed to add frame: {} ms: {} dur: {} -> {}", i, last_ms, frame.1, e));

                last_ms += frame.1 as i32;

                frames_spin.inc(1);
            }

            frames_spin.finish_and_clear();

            let webp_data = encoder.finalize(last_ms).unwrap();
            fs::write(output.clone(), webp_data).expect("Failed to write webp data");
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

            let frame_count = frames.len();
            println!("{} Processing mode: 󰸭 {} with {} frames", IMAGE, style("GIF").bold().cyan(), style(frames.len()).bold().cyan());

            let frames_spin = multi_progress.add(ProgressBar::new(frame_count as u64));
            frames_spin.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")?
            );

            let seed = args.seed.unwrap();
            (0..frame_count).into_par_iter().for_each(|i| {
                let pb = multi_progress.add(ProgressBar::new(0));
                // update a bit slower
                pb.enable_steady_tick(Duration::from_millis(100));

                let mut rng: Box<dyn RngCore> = Box::new(StdRng::seed_from_u64(seed));

                let frame = frames.get(i).expect("Failed to get frame").to_owned();
                let delay = frame.delay().numer_denom_ms().0 as u16;
                let img = frame.into_buffer();
                let out =
                    process(img.into(), parsed, args, &mut rng, Some(pb)).expect("Failed to process frame");
                let mut bytes = out.as_bytes().to_vec();

                let mut new_frame = gif::Frame::from_rgba_speed(w as u16, h as u16, &mut bytes, 10);

                new_frame.delay = delay / 10;
                new_frames
                    .lock()
                    .expect("failed to unlock")
                    .push((i, new_frame));

                frames_spin.inc(1);
            });

            frames_spin.reset();
            frames_spin.set_length(frame_count as u64);
            frames_spin.set_message("Encoding frames...");

            let mut frames = new_frames.into_inner().expect("Failed to get frames");
            frames.sort_by(|a, b| a.0.cmp(&b.0));
            for (_, frame) in frames {
                encoder.write_frame(&frame)?;

                frames_spin.inc(1);
            }

            frames_spin.finish_and_clear();

            println!("{} Processed {} frames...", OK, style(frame_count).bold().cyan());
        }
        _ => return Err(anyhow::anyhow!("Unsupported file format\n")),
    };

    let output_file = Path::new(&output);

   println!(
        "{} Processed {} Expression{}...",
        OK,
        style(expression_count).bold().cyan(),
        if expression_count > 1 { "s" } else { "" }
    );

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
    args: &Args,
    rand: &mut Box<dyn RngCore>,
    progress_bar: Option<ProgressBar>
) -> anyhow::Result<DynamicImage> {
    let mut output_image = DynamicImage::new(img.width(), img.height(), img.color());

    let pb = if let Some(pb) = progress_bar {
       // get total pixels to process of the image * number of expressions as u64
        let total_pixels = ((img.width() * img.height()) * expressions.len() as u32) as u64;
        pb.set_length(total_pixels);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")?
        );

        Some(pb)
    } else {
        None
    };

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
                        rgba: colors,
                        saved_rgb: [sr, sg, sb],
                        position: (x, y),
                        ignore_state: args.no_state,
                    },
                    &img,
                    rand,
                )
                    .expect("Failed to evaluate");

                sr = result[0];
                sg = result[1];
                sb = result[2];

                output_image.put_pixel(x, y, result);

                if let Some(pb) = &pb {
                    pb.inc(1);
                }
            }
        }

        img = output_image.clone();
    }

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    Ok(output_image)
}

fn strip_windows_prefix(path: &Path) -> PathBuf {
    path.to_str().and_then(|s| s.strip_prefix(r"\\?\")).map_or_else(|| path.to_path_buf(), PathBuf::from)
}

fn get_random_seed(args: &Args) -> u64 {
    if args.seed.is_none() {
        return std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos() as u64;
    }

    args.seed.unwrap()
}

fn hash_strings(strings: Vec<String>) -> u64 {
    let mut hasher = DefaultHasher::new();
    for s in strings {
        s.hash(&mut hasher);
    }
    hasher.finish()
}

fn get_precompiled_cache(hash: &str) -> Option<PathBuf> {
    let home_dir = home_dir().expect("Failed to find home directory");

    let glitch_dir = Path::new(&home_dir).join(".glitch");
    if !glitch_dir.exists() {
        fs::create_dir(&glitch_dir).expect("Failed to create .glitch directory");
    }

    let cache_dir = glitch_dir.join("cache");
    if !cache_dir.exists() {
        fs::create_dir(&cache_dir).expect("Failed to create cache directory");
    }

    let cached_file_path = cache_dir.join(hash);

    if cached_file_path.exists() {
        Some(cached_file_path)
    } else {
        None
    }
}

fn save_to_cache(hash: &str, data: &[u8]) -> anyhow::Result<PathBuf> {
    let home_dir = home_dir().expect("Failed to find home directory");

    let glitch_dir = Path::new(&home_dir).join(".glitch");
    if !glitch_dir.exists() {
        fs::create_dir(&glitch_dir)?;
    }

    let cache_dir = glitch_dir.join("cache");
    if !cache_dir.exists() {
        fs::create_dir(&cache_dir)?;
    }

    let cached_file_path = cache_dir.join(hash);

    // Write data to the file
    let mut file = File::create(&cached_file_path)?;
    file.write_all(data)?;

    Ok(cached_file_path)
}