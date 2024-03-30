#![deny(clippy::perf, clippy::correctness)]
#![warn(
    rust_2018_idioms,
    clippy::nursery,
    clippy::complexity,
    clippy::cognitive_complexity
)]

use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::Mutex;

use ansiterm::Color;
use clap::Parser;
use gif::{Encoder, Repeat};
use image::codecs::gif::GifDecoder;
use image::{
    guess_format, AnimationDecoder, DynamicImage, GenericImage, GenericImageView, ImageDecoder,
    ImageFormat, Pixel,
};
use rayon::prelude::*;

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

    /// Optional output file
    #[arg(short, long)]
    output: Option<String>,

    /// Open the output file after processing
    #[arg(long, default_value = "false")]
    open: bool,

    /// Disable the state during processing
    #[arg(long, default_value = "false")]
    no_state: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    // If we want to pass the arguments to a function, we need to clone them
    let mut writer = std::io::stdout();
    let is_tty = std::io::IsTerminal::is_terminal(&writer);
    if args.input.starts_with("http") {
        write_painted(
            &mut writer,
            " Downloading Image: ",
            Color::BrightGreen,
            (true, false),
            is_tty,
        )?;
    } else {
        write_painted(
            &mut writer,
            " Input File: ",
            Color::BrightPurple,
            (true, true),
            is_tty,
        )?;
    }
    write_painted(
        &mut writer,
        &args.input,
        Color::RGB(255, 165, 0),
        (false, false),
        is_tty,
    )?;
    write_painted(
        &mut writer,
        "\n\nParsing expressions...\n",
        Color::BrightBlue,
        (true, false),
        is_tty,
    )?;
    let mut parsed: Vec<(String, Vec<Token>)> = vec![];
    for e in &args.expressions {
        let tokens = match parser::shunting_yard(e) {
            Ok(tokens) => tokens,
            Err(err) => {
                write_painted(
                    &mut writer,
                    format!("\nExpression:{}\n", e).as_str(),
                    Color::Red,
                    (true, true),
                    is_tty,
                )?;
                write_painted(
                    &mut writer,
                    format!("{}\n", err).as_str(),
                    Color::Red,
                    (false, false),
                    is_tty,
                )?;
                return Ok(());
            }
        };
        write_painted(
            &mut writer,
            "\nExpression  \n",
            Color::BrightCyan,
            (true, true),
            is_tty,
        )?;
        write_painted(
            &mut writer,
            format!("[  \"{}\"  ]\n\n", e).as_str(),
            Color::RGB(255, 165, 0),
            (false, false),
            is_tty,
        )?;
        tokens.clone().iter().for_each(|t| match is_tty {
            true => {
                writer
                    .write_fmt(format_args!("\t{}\n", t))
                    .unwrap_or_default();
            }
            false => {
                writer
                    .write_fmt(format_args!("\t{:?}\n", t))
                    .unwrap_or_default();
            }
        });
        parsed.push((e.to_string(), tokens));
    }
    write_painted(
        &mut writer,
        "\n\nConsuming Expressions...\n\n",
        Color::BrightPurple,
        (true, false),
        is_tty,
    )?;
    handle_image(&args, &mut writer, &parsed, is_tty)?;
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
    writer: &mut impl Write,
    parsed: &[(String, Vec<Token>)],
    is_tty: bool,
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
    let name = match &args.input {
        file if file.starts_with("http") => file.split('/').last().expect("last").to_string(),
        file => Path::new(&file)
            .file_name()
            .expect("file name")
            .to_str()
            .expect("Unable to get filename")
            .to_string(),
    };
    write_painted(
        writer,
        "\t Processing Image: ",
        Color::BrightGreen,
        (true, false),
        is_tty,
    )?;
    write_painted(
        writer,
        format!("{}\n", &name).as_str(),
        Color::RGB(255, 165, 0),
        (false, false),
        is_tty,
    )?;
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
    match format {
        ImageFormat::Png => {
            let img = image::load_from_memory(&img)?;

            write_painted(
                writer,
                "\tProcessing mode: 󰸭 PNG\n",
                Color::BrightGreen,
                (true, false),
                is_tty,
            )?;
            let out = process(img, parsed, args.no_state)?;
            out.save_with_format(output.clone(), format)?;
        }
        ImageFormat::Jpeg => {
            let img = image::load_from_memory(&img)?;

            write_painted(
                writer,
                "\tProcessing mode: 󰈥 JPEG\n",
                Color::BrightGreen,
                (true, false),
                is_tty,
            )?;
            let out = process(img, parsed, args.no_state)?;
            out.save_with_format(output.clone(), format)?;
        }
        ImageFormat::Gif => {
            write_painted(
                writer,
                "\tProcessing mode: 󰵸 GIF\n\n",
                Color::BrightGreen,
                (true, false),
                is_tty,
            )?;
            let mut reader = std::io::Cursor::new(img);
            let decoder = GifDecoder::new(&mut reader)?;
            let [w, h] = [decoder.dimensions().0, decoder.dimensions().1];
            let frames = decoder.into_frames().collect_frames()?;
            write_painted(
                writer,
                format!("Processing {} frames...\n\n", frames.len()).as_str(),
                Color::BrightCyan,
                (true, true),
                is_tty,
            )?;
            let output = std::fs::File::create(output.clone())?;
            let mut img_writer = BufWriter::new(output);
            let mut encoder = Encoder::new(&mut img_writer, w as u16, h as u16, &[])?;
            encoder.set_repeat(Repeat::Infinite)?;

            let new_frames = Mutex::new(Vec::with_capacity(frames.len()));

            (0..frames.len()).into_par_iter().for_each(|i| {
                let frame = frames.get(i).expect("Failed to get frame");
                let frame = frame.clone();
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
    write_painted(
        writer,
        "Saved output to: ",
        Color::BrightYellow,
        (true, true),
        is_tty,
    )?;
    write_painted(
        writer,
        output_file.to_str().unwrap(),
        Color::RGB(255, 165, 0),
        (false, false),
        is_tty,
    )?;
    if args.open {
        open::that(output_file)?;
    }
    Ok(())
}

fn write_painted(
    w: &mut impl Write,
    s: &str,
    color: Color,
    bold_ul: (bool, bool),
    is_tty: bool,
) -> Result<(), std::io::Error> {
    w.write_all(
        match is_tty {
            true => match bold_ul {
                (true, true) => color.bold().underline().paint(s).to_string(),
                (false, true) => color.underline().paint(s).to_string(),
                (true, false) => color.bold().paint(s).to_string(),
                (false, false) => color.paint(s).to_string(),
            },
            false => s.to_string(),
        }
        .as_bytes(),
    )
}

fn process(
    mut img: DynamicImage,
    expressions: &[(String, Vec<Token>)],
    no_state: bool,
) -> anyhow::Result<DynamicImage> {
    let mut output_image = DynamicImage::new(img.width(), img.height(), img.color());

    for val in expressions.iter() {
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
                        ignore_state: no_state,
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
