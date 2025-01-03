use image::{DynamicImage, GenericImageView, Pixel, Rgba};
use rand::{Rng, RngCore};
use std::collections::HashMap;
use std::ops::{BitAnd, BitOr, BitXor};
use crate::rgb::Rgb;
use crate::token::Token;

#[derive(Debug, Default)]
struct SumSave {
    v_y: Option<Rgb>,
    v_b: Option<Rgb>,
    v_e: Option<Rgb>,
    v_r: Option<HashMap<u8, Rgb>>,
    v_t: Option<Rgb>,
    v_g: Option<Rgb>,
    v_h: Option<Rgb>,
    v_v: Option<Rgb>,
    v_d: Option<Rgb>,
    v_high: Option<Rgb>,
    v_low: Option<Rgb>,
}

#[derive(Debug, Clone)]
pub struct EvalContext {
    pub tokens: Vec<Token>,
    pub size: (u32, u32),
    pub rgba: Rgba<u8>,
    pub saved_rgb: [u8; 3],
    pub position: (u32, u32),

    pub ignore_state: bool,
}

fn binary_stack_op(stack: &mut Vec<Rgb>, op: fn(u8, u8) -> u8) -> Result<(), String> {
    let b = stack.pop().ok_or("Stack underflow")?;
    let a = stack.pop().ok_or("Stack underflow")?;
    stack.push(Rgb::new(op(a.r, b.r), op(a.g, b.g), op(a.b, b.b)));
    Ok(())
}

enum ChannelOp {
    Pow,
    BitLShift,
    BitRShift,
}

fn channel_op(stack: &mut Vec<Rgb>, op: ChannelOp) -> Result<(), String> {
    let b = stack.pop().ok_or("Stack underflow")?;
    let a = stack.pop().ok_or("Stack underflow")?;

    let (rr, gg, bb) = match op {
        ChannelOp::Pow => (
            a.r.wrapping_pow(b.r.into()),
            a.g.wrapping_pow(b.g.into()),
            a.b.wrapping_pow(b.b.into()),
        ),
        ChannelOp::BitLShift => (
            a.r.wrapping_shl(b.r.into()),
            a.g.wrapping_shl(b.g.into()),
            a.b.wrapping_shl(b.b.into()),
        ),
        ChannelOp::BitRShift => (
            a.r.wrapping_shr(b.r.into()),
            a.g.wrapping_shr(b.g.into()),
            a.b.wrapping_shr(b.b.into()),
        ),
    };

    stack.push(Rgb::new(rr, gg, bb));
    Ok(())
}

pub fn eval(
    ctx: EvalContext,
    input: &DynamicImage,
    rng: &mut Box<dyn RngCore>,
) -> Result<Rgba<u8>, String> {
    let EvalContext {
        tokens,
        size,
        rgba,
        saved_rgb,
        position,
        ignore_state,
    } = ctx;
    let (width, height) = size;
    let (x, y) = position;

    let r = rgba[0];
    let g = rgba[1];
    let b = rgba[2];
    let a = rgba[3];

    let [sr, sg, sb] = saved_rgb;

    if a == 0 {
        return Ok(Rgba([0, 0, 0, 0]));
    }

    let mut stack: Vec<Rgb> = Vec::with_capacity(tokens.len());

    let div = |a: u8, b: u8| -> u8 {
        if b == 0 {
            return a;
        }
        a.wrapping_div(b)
    };

    let modu = |a: u8, b: u8| -> u8 {
        if b == 0 {
            return a;
        }
        a.wrapping_rem(b)
    };

    let bit_and_not = |a: u8, b: u8| -> u8 { a & !b };

    let weight = |a: u8, b: u8| -> u8 {
        let fuzz = f64::from(b) / 255.0;
        let r = f64::from(a) * fuzz;
        r as u8
    };

    let three_rule = |x: u32, max: u32| -> u8 { (((255 * x) / max) & 255) as u8 };

    let is_in_bounds = |x: u32, y: u32| -> bool { x < width && y < height };

    let get_pixel_in_bounds = |x: u32, y: u32| -> [u8; 4] {
        if is_in_bounds(x, y) {
            input.get_pixel(x, y).0
        } else {
            [0, 0, 0, 0]
        }
    };

    let rgb_from_colors = |colors: &[(i32, i32); 3]| -> Rgb {
        let mut rgb = [0; 3];
        for (i, (xx, yy)) in colors.iter().enumerate() {
            let x = (xx + x as i32) as u32;
            let y = (yy + y as i32) as u32;
            let pixel = get_pixel_in_bounds(x, y);
            rgb[i] = pixel[i];
        }
        Rgb::from(rgb)
    };

    let mut saved = SumSave::default();

    for tok in tokens {
        match tok {
            Token::Num(n) => stack.push(Rgb::new(n, n, n)),

            Token::Add => binary_stack_op(&mut stack, u8::wrapping_add)?,
            Token::Sub => binary_stack_op(&mut stack, u8::wrapping_sub)?,
            Token::Mul => binary_stack_op(&mut stack, u8::wrapping_mul)?,
            Token::Div => binary_stack_op(&mut stack, div)?,
            Token::Mod => binary_stack_op(&mut stack, modu)?,
            Token::BitAnd => binary_stack_op(&mut stack, u8::bitand)?,
            Token::BitOr => binary_stack_op(&mut stack, u8::bitor)?,
            Token::BitXor => binary_stack_op(&mut stack, u8::bitxor)?,
            Token::BitAndNot => binary_stack_op(&mut stack, bit_and_not)?,
            Token::Weight => binary_stack_op(&mut stack, weight)?,
            Token::Pow => channel_op(&mut stack, ChannelOp::Pow)?,
            Token::BitLShift => channel_op(&mut stack, ChannelOp::BitLShift)?,
            Token::BitRShift => channel_op(&mut stack, ChannelOp::BitRShift)?,

            Token::Greater => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;

                stack.push(Rgb::new(
                    if a.r > b.r { 255 } else { 0 },
                    if a.g > b.g { 255 } else { 0 },
                    if a.b > b.b { 255 } else { 0 },
                ));
            }

            Token::Random(num) => {
                let neg = std::ops::Neg::neg(num as i8);

                let v_r = saved.v_r.as_ref().and_then(|v| v.get(&num));

                let v_r = if let Some(v) = v_r {
                    *v
                } else {
                    let colors = gen_random_position(neg as i32, num as i32, rng);
                    let rgb = rgb_from_colors(&colors);
                    if !ignore_state {
                        saved.v_r.get_or_insert(HashMap::new()).insert(num, rgb);
                    }
                    rgb
                };

                stack.push(v_r);
            }

            Token::RGBColor((token, num)) => match token {
                'R' => stack.push(Rgb::new_red(num)),
                'G' => stack.push(Rgb::new_blue(num)),
                'B' => stack.push(Rgb::new_green(num)),
                _ => return Err(format!("Unexpected token: {:?}", token)),
            },

            Token::Brightness(brightness_value) => {
                let factor = (brightness_value as f64 / 255.0).clamp(0.0, 1.0);

                let pixel = input.get_pixel(x, y);
                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];

                let (nr, ng, nb) = adjust_brightness_hsv(r, g, b, factor);

                stack.push(Rgb::new(nr, ng, nb));
            }

            Token::Invert => {
                let pixel = input.get_pixel(x, y);
                let mut new_rgba = Rgba([pixel[0], pixel[1], pixel[2], pixel[3]]);
                new_rgba.invert();

                let r = new_rgba[0];
                let g = new_rgba[1];
                let b = new_rgba[2];

                stack.push(Rgb::new(r, g, b));
            }

            Token::Char(c) => match c {
                'c' => stack.push(Rgb::new(r, g, b)),
                'Y' => {
                    let v_y = match saved.v_y {
                        Some(v_y) => v_y,
                        None => {
                            let y =
                                f64::from(b).mul_add(0.0722, f64::from(r).mul_add(0.299, f64::from(g) * 0.587));
                            let v_y = Rgb::new(y as u8, y as u8, y as u8);
                            saved.v_y = Some(v_y);
                            v_y
                        }
                    };

                    stack.push(v_y);
                }
                's' => stack.push(Rgb::new(sr, sg, sb)),
                'x' => {
                    let xu = three_rule(x, width);
                    stack.push(Rgb::new(xu, xu, xu));
                }
                'y' => {
                    let yu = three_rule(y, height);
                    stack.push(Rgb::new(yu, yu, yu));
                }

                't' => {
                    let v_t = match saved.v_t {
                        Some(v_t) => v_t,
                        None => {
                            let colors = gen_random_position(-2, 2, rng);

                            let rgb = rgb_from_colors(&colors);
                            if !ignore_state {
                                saved.v_t = Some(rgb);
                            }
                            rgb
                        }
                    };

                    stack.push(v_t);
                }
                'g' => {
                    let v_g = match saved.v_g {
                        Some(v_g) => v_g,
                        None => {
                            let colors = gen_random_position(0i32, width as i32, rng);

                            let rgb = rgb_from_colors(&colors);
                            if !ignore_state {
                                saved.v_g = Some(rgb);
                            }
                            rgb
                        }
                    };

                    stack.push(v_g);
                }
                'e' => {
                    let v_e = match saved.v_e {
                        Some(v_e) => v_e,
                        None => {
                            let boxed = fetch_boxed(input, x as i32, y as i32, r, g, b);

                            let rr = boxed[8]
                                .r
                                .wrapping_sub(boxed[0].r)
                                .wrapping_add(boxed[5].r)
                                .wrapping_sub(boxed[3].r)
                                .wrapping_add(boxed[7].r)
                                .wrapping_sub(boxed[1].r)
                                .wrapping_add(boxed[6].r)
                                .wrapping_sub(boxed[2].r);

                            let gg = boxed[8]
                                .g
                                .wrapping_sub(boxed[0].g)
                                .wrapping_add(boxed[5].g)
                                .wrapping_sub(boxed[3].g)
                                .wrapping_add(boxed[7].g)
                                .wrapping_sub(boxed[1].g)
                                .wrapping_add(boxed[6].g)
                                .wrapping_sub(boxed[2].g);

                            let bb = boxed[8]
                                .b
                                .wrapping_sub(boxed[0].b)
                                .wrapping_add(boxed[5].b)
                                .wrapping_sub(boxed[3].b)
                                .wrapping_add(boxed[7].b)
                                .wrapping_sub(boxed[1].b)
                                .wrapping_add(boxed[6].b)
                                .wrapping_sub(boxed[2].b);

                            let v_e = Rgb::new(rr, gg, bb);
                            saved.v_e = Some(v_e);
                            v_e
                        }
                    };

                    stack.push(v_e);
                }
                'b' => {
                    let v_b = match saved.v_b {
                        Some(v_b) => v_b,
                        None => {
                            let boxed = fetch_boxed(input, x as i32, y as i32, r, g, b);

                            let rr = wrapping_vec_add_u32([
                                boxed[0].r, boxed[1].r, boxed[2].r, boxed[3].r, boxed[5].r,
                                boxed[6].r, boxed[7].r, boxed[8].r,
                            ]);
                            let gg = wrapping_vec_add_u32([
                                boxed[0].g, boxed[1].g, boxed[2].g, boxed[3].g, boxed[5].g,
                                boxed[6].g, boxed[7].g, boxed[8].g,
                            ]);
                            let bb = wrapping_vec_add_u32([
                                boxed[0].b, boxed[1].b, boxed[2].b, boxed[3].b, boxed[5].b,
                                boxed[6].b, boxed[7].b, boxed[8].b,
                            ]);

                            let v_b = Rgb::new((rr / 9) as u8, (gg / 9) as u8, (bb / 9) as u8);
                            if !ignore_state {
                                saved.v_b = Some(v_b);
                            }
                            v_b
                        }
                    };

                    stack.push(v_b);
                }
                'H' => {
                    let v_h = match saved.v_high {
                        Some(v_h) => v_h,
                        None => {
                            let boxed = fetch_boxed(input, x as i32, y as i32, r, g, b);

                            let r_m = max([
                                boxed[0].r, boxed[1].r, boxed[2].r, boxed[3].r, boxed[5].r,
                                boxed[6].r, boxed[7].r, boxed[8].r,
                            ]);
                            let g_m = max([
                                boxed[0].g, boxed[1].g, boxed[2].g, boxed[3].g, boxed[5].g,
                                boxed[6].g, boxed[7].g, boxed[8].g,
                            ]);
                            let b_m = max([
                                boxed[0].b, boxed[1].b, boxed[2].b, boxed[3].b, boxed[5].b,
                                boxed[6].b, boxed[7].b, boxed[8].b,
                            ]);

                            let v_h = Rgb::new(r_m, g_m, b_m);

                            if !ignore_state {
                                saved.v_high = Some(v_h);
                            }
                            v_h
                        }
                    };

                    stack.push(v_h);
                }
                'L' => {
                    let v_l = match saved.v_low {
                        Some(v_l) => v_l,
                        None => {
                            let boxed = fetch_boxed(input, x as i32, y as i32, r, g, b);

                            let r_m = min([
                                boxed[0].r, boxed[1].r, boxed[2].r, boxed[3].r, boxed[5].r,
                                boxed[6].r, boxed[7].r, boxed[8].r,
                            ]);
                            let g_m = min([
                                boxed[0].g, boxed[1].g, boxed[2].g, boxed[3].g, boxed[5].g,
                                boxed[6].g, boxed[7].g, boxed[8].g,
                            ]);
                            let b_m = min([
                                boxed[0].b, boxed[1].b, boxed[2].b, boxed[3].b, boxed[5].b,
                                boxed[6].b, boxed[7].b, boxed[8].b,
                            ]);

                            let v_l = Rgb::new(r_m, g_m, b_m);
                            if !ignore_state {
                                saved.v_low = Some(v_l);
                            }
                            v_l
                        }
                    };

                    stack.push(v_l);
                }
                'N' => stack.push(Rgb::new(
                    rng.gen_range(0..=255),
                    rng.gen_range(0..=255),
                    rng.gen_range(0..=255),
                )),
                'h' => {
                    let v_h = match saved.v_h {
                        Some(v_h) => v_h,
                        None => {
                            let h = width - x - 1;
                            // check that we are in bounds
                            if h >= width {
                                return Err("Out of bounds".to_string());
                            }

                            let pixel = input.get_pixel(h, y).0;

                            let v_h = Rgb::new(pixel[0], pixel[1], pixel[2]);
                            if !ignore_state {
                                saved.v_h = Some(v_h);
                            }
                            v_h
                        }
                    };

                    stack.push(v_h);
                }
                'v' => {
                    let v_v = match saved.v_v {
                        Some(v_v) => v_v,
                        None => {
                            let v = height - y - 1;
                            let pixel = input.get_pixel(x, v).0;

                            let v_v = Rgb::new(pixel[0], pixel[1], pixel[2]);
                            if !ignore_state {
                                saved.v_v = Some(v_v);
                            }
                            v_v
                        }
                    };

                    stack.push(v_v);
                }
                'd' => {
                    let v_d = match saved.v_d {
                        Some(v_d) => v_d,
                        None => {
                            let x = width - x - 1;
                            let y = height - y - 1;
                            let pixel = input.get_pixel(x, y).0;

                            let v_d = Rgb::new(pixel[0], pixel[1], pixel[2]);
                            if !ignore_state {
                                saved.v_d = Some(v_d);
                            }
                            v_d
                        }
                    };

                    stack.push(v_d);
                }
                _ => return Err(format!("Unexpected token: {:?}", c)),
            },

            _ => return Err(format!("Unexpected token: {:?}", tok)),
        }
    }

    let col = stack.last().ok_or("Stack underflow")?;
    Ok(Rgba([col.r, col.g, col.b, a]))
}

#[inline]
fn fetch_boxed(input: &DynamicImage, x: i32, y: i32, r: u8, g: u8, b: u8) -> [Rgb; 9] {
    let mut k = 0;

    let mut boxed: [Rgb; 9] = [Rgb::default(); 9];

    for i in x - 1..=x + 1 {
        for j in y - 1..=y + 1 {
            if i == x && j == y {
                boxed[k] = Rgb { r, g, b };
                k += 1;
                continue;
            }

            if i < 0 || j < 0 {
                boxed[k] = Rgb::default();
                k += 1;
                continue;
            }

            let pixel = input.get_pixel(i as u32, j as u32).0;
            boxed[k] = Rgb {
                r: pixel[0],
                g: pixel[1],
                b: pixel[2],
            };
            k += 1;
        }
    }
    boxed
}

fn max(vals: [u8; 8]) -> u8  {
    vals.iter().cloned().max().unwrap_or_default()
}

fn min(vals: [u8; 8]) -> u8 {
    vals.iter().cloned().min().unwrap_or_default()
}

fn gen_random_position(min: i32, max: i32, rng: &mut Box<dyn RngCore>) -> [(i32, i32); 3] {
    let mut positions = [(0, 0); 3];
    for i in positions.iter_mut() {
        i.0 = rng.gen_range(min..=max);
        i.1 = rng.gen_range(min..=max);
    }
    positions
}

#[inline]
fn wrapping_vec_add_u32(a: [u8; 8]) -> u32 {
    let mut sum: u32 = 0;
    for i in a {
        sum = sum.wrapping_add(i as u32);
    }
    sum
}

/// Convert an RGB (0–255) color into HSV, each component in [0.0, 1.0].
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;

    let cmax = rf.max(gf).max(bf);
    let cmin = rf.min(gf).min(bf);
    let delta = cmax - cmin;

    // Hue calculation
    let h = if delta < f64::EPSILON {
        0.0
    } else if (cmax - rf).abs() < f64::EPSILON {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if (cmax - gf).abs() < f64::EPSILON {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };

    // Saturation
    let s = if cmax < f64::EPSILON {
        0.0
    } else {
        delta / cmax
    };

    // Value
    let v = cmax;

    // Normalize hue to be in [0, 360)
    let mut hue = h;
    if hue < 0.0 {
        hue += 360.0;
    }

    (hue / 360.0, s, v)
}

/// Convert an HSV color (each in [0.0, 1.0]) back into RGB (0–255).
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h_deg = h * 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h_deg / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (rf, gf, bf) = match h_deg {
        d if d < 60.0  => (c, x, 0.0),
        d if d < 120.0 => (x, c, 0.0),
        d if d < 180.0 => (0.0, c, x),
        d if d < 240.0 => (0.0, x, c),
        d if d < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    let r = ((rf + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = ((gf + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = ((bf + m) * 255.0).round().clamp(0.0, 255.0) as u8;

    (r, g, b)
}

/// Scale the brightness (Value in HSV) by `factor` [0.0..=1.0].
fn adjust_brightness_hsv(r: u8, g: u8, b: u8, factor: f64) -> (u8, u8, u8) {
    let (h, s, v) = rgb_to_hsv(r, g, b);
    let new_v = (v * factor).clamp(0.0, 1.0);
    hsv_to_rgb(h, s, new_v)
}