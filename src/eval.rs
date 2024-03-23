use std::collections::HashMap;
use image::{DynamicImage, GenericImageView, Rgba};
use rand::prelude::ThreadRng;
use rand::Rng;

use crate::parser::Token;

#[derive(Debug, Clone, Copy, Default)]
struct RgbSum {
    r: u8,
    g: u8,
    b: u8,
}

impl RgbSum {
    fn new(r: u8, g: u8, b: u8) -> Self {
        RgbSum { r, g, b }
    }
}

impl From<[u8; 3]> for RgbSum {
    fn from(rgb: [u8; 3]) -> Self {
        RgbSum {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        }
    }
}

#[derive(Debug, Default)]
struct SumSave {
    v_y: Option<RgbSum>,
    v_b: Option<RgbSum>,
    v_e: Option<RgbSum>,
    v_r: Option<HashMap<u8, RgbSum>>,
    v_t: Option<RgbSum>,
    v_g: Option<RgbSum>,
    v_h: Option<RgbSum>,
    v_v: Option<RgbSum>,
    v_d: Option<RgbSum>,
    v_high: Option<RgbSum>,
    v_low: Option<RgbSum>,
}

#[derive(Debug, Clone)]
pub struct EvalContext {
    pub tokens: Vec<Token>,
    pub size: (u32, u32),
    pub rgba: [u8; 4],
    pub saved_rgb: [u8; 3],
    pub position: (u32, u32),

    pub ignore_state: bool,
}

pub fn eval(
    ctx: EvalContext,
    input: &DynamicImage,
    mut rng: ThreadRng,
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
    let [r, g, b, a] = rgba;
    let [sr, sg, sb] = saved_rgb;

    if a == 0 {
        return Ok(Rgba([0, 0, 0, 0]));
    }

    let mut stack: Vec<RgbSum> = Vec::with_capacity(tokens.len());

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

    let rgb_from_colors = |colors: &[(i32, i32); 3]| -> RgbSum {
        let mut rgb = [0; 3];
        for (i, (xx, yy)) in colors.iter().enumerate() {
            let x = (xx + x as i32) as u32;
            let y = (yy + y as i32) as u32;
            let pixel = get_pixel_in_bounds(x, y);
            rgb[i] = pixel[i];
        }
        RgbSum::from(rgb)
    };

    let mut saved = SumSave::default();

    for tok in tokens {
        match tok {
            Token::Num(n) => stack.push(RgbSum::new(n, n, n)),

            Token::Add => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(
                    a.r.wrapping_add(b.r),
                    a.g.wrapping_add(b.g),
                    a.b.wrapping_add(b.b),
                ));
            }

            Token::Sub => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(
                    a.r.wrapping_sub(b.r),
                    a.g.wrapping_sub(b.g),
                    a.b.wrapping_sub(b.b),
                ));
            }

            Token::Mul => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(
                    a.r.wrapping_mul(b.r),
                    a.g.wrapping_mul(b.g),
                    a.b.wrapping_mul(b.b),
                ));
            }

            Token::Div => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(div(a.r, b.r), div(a.g, b.g), div(a.b, b.b)));
            }

            Token::Mod => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(modu(a.r, b.r), modu(a.g, b.g), modu(a.b, b.b)));
            }

            Token::Pow => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(
                    a.r.wrapping_pow(b.r.into()),
                    a.g.wrapping_pow(b.g.into()),
                    a.b.wrapping_pow(b.b.into()),
                ));
            }

            Token::BitAnd => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(a.r & b.r, a.g & b.g, a.b & b.b));
            }

            Token::BitOr => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(a.r | b.r, a.g | b.g, a.b | b.b));
            }

            Token::BitAndNot => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(
                    bit_and_not(a.r, b.r),
                    bit_and_not(a.g, b.g),
                    bit_and_not(a.b, b.b),
                ));
            }

            Token::BitXor => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;
                stack.push(RgbSum::new(a.r ^ b.r, a.g ^ b.g, a.b ^ b.b));
            }

            Token::BitLShift => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;

                let rr = a.r.wrapping_shl(b.r.into());
                let gg = a.g.wrapping_shl(b.g.into());
                let bb = a.b.wrapping_shl(b.b.into());

                stack.push(RgbSum::new(rr, gg, bb));
            }

            Token::BitRShift => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;

                let rr = a.r.wrapping_shr(b.r.into());
                let gg = a.g.wrapping_shr(b.g.into());
                let bb = a.b.wrapping_shr(b.b.into());

                stack.push(RgbSum::new(rr, gg, bb));
            }

            Token::Weight => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;

                stack.push(RgbSum::new(
                    weight(a.r, b.r),
                    weight(a.g, b.g),
                    weight(a.b, b.b),
                ));
            }

            Token::Greater => {
                let b = stack.pop().ok_or("Stack underflow")?;
                let a = stack.pop().ok_or("Stack underflow")?;

                stack.push(RgbSum::new(
                    if a.r > b.r { 255 } else { 0 },
                    if a.g > b.g { 255 } else { 0 },
                    if a.b > b.b { 255 } else { 0 },
                ));
            }

            Token::Random(num) => {
                let neg = std::ops::Neg::neg(num as i8);
                // check if the number exists in v_r hashmap
                let v_r = if let Some(v) = saved.v_r.as_ref() {
                    v.get(&num)
                } else {
                    None
                };

                let v_r = if let Some(v) = v_r {
                    *v
                } else {
                    let colors = gen_random_position(neg as i32, num as i32, &mut rng);
                    let rgb = rgb_from_colors(&colors);
                    if !ignore_state {
                        saved.v_r.get_or_insert(HashMap::new()).insert(num, rgb);
                    }
                    rgb
                };

                stack.push(v_r);
            }

            Token::RGBColor((token, num)) => {
               match token {
                    'R' => stack.push(RgbSum::new(num, 0, 0)),
                    'G' => stack.push(RgbSum::new(0, num, 0)),
                    'B' => stack.push(RgbSum::new(0, 0, num)),
                    _ => return Err(format!("Unexpected token: {:?}", token)),
                }
            }

            Token::Char(c) => match c {
                'c' => stack.push(RgbSum::new(r, g, b)),
                'Y' => {
                    let v_y = match saved.v_y {
                        Some(v_y) => v_y,
                        None => {
                            let y =
                                f64::from(r) * 0.299 + f64::from(g) * 0.587 + f64::from(b) * 0.0722;
                            let v_y = RgbSum::new(y as u8, y as u8, y as u8);
                            saved.v_y = Some(v_y);
                            v_y
                        }
                    };

                    stack.push(v_y);
                }
                's' => stack.push(RgbSum::new(sr, sg, sb)),
                'x' => {
                    let xu = three_rule(x, width);
                    stack.push(RgbSum::new(xu, xu, xu));
                }
                'y' => {
                    let yu = three_rule(y, height);
                    stack.push(RgbSum::new(yu, yu, yu));
                }
                /*
                'r' => {
                    let v_r = match saved.v_r {
                        Some(v_r) => v_r,
                        None => {
                            let colors =
                                gen_random_position(-1, 1, &mut rng);

                            let rgb = rgb_from_colors(&colors);
                            if !ignore_state {
                                saved.v_r = Some(rgb);
                            }
                            rgb
                        }
                    };

                    stack.push(v_r);
                }*/
                't' => {
                    let v_t = match saved.v_t {
                        Some(v_t) => v_t,
                        None => {
                            let colors =
                                gen_random_position(-2, 2, &mut rng);

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
                            let colors =
                                gen_random_position(0i32, width as i32, &mut rng);

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

                            let v_e = RgbSum::new(rr, gg, bb);
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

                            let v_b = RgbSum::new((rr / 9) as u8, (gg / 9) as u8, (bb / 9) as u8);
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

                            let v_h = RgbSum::new(r_m, g_m, b_m);

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

                            let v_l = RgbSum::new(r_m, g_m, b_m);
                            if !ignore_state {
                                saved.v_low = Some(v_l);
                            }
                            v_l
                        }
                    };

                    stack.push(v_l);
                }
                'N' => stack.push(RgbSum::new(
                    rng.gen_range(0..=255),
                    rng.gen_range(0..=255),
                    rng.gen_range(0..=255),
                )),
                'h' => {
                    let v_h = match saved.v_h {
                        Some(v_h) => v_h,
                        None => {
                            let h = width - x - 1;
                            let pixel = input.get_pixel(h, y).0;

                            let v_h = RgbSum::new(pixel[0], pixel[1], pixel[2]);
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

                            let v_v = RgbSum::new(pixel[0], pixel[1], pixel[2]);
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

                            let v_d = RgbSum::new(pixel[0], pixel[1], pixel[2]);
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
fn fetch_boxed(input: &DynamicImage, x: i32, y: i32, r: u8, g: u8, b: u8) -> [RgbSum; 9] {
    let mut k = 0;

    let mut boxed: [RgbSum; 9] = [RgbSum::default(); 9];

    for i in x - 1..=x + 1 {
        for j in y - 1..=y + 1 {
            if i == x && j == y {
                boxed[k] = RgbSum { r, g, b };
                k += 1;
                continue;
            }

            if i < 0 || j < 0 {
                boxed[k] = RgbSum::default();
                k += 1;
                continue;
            }

            let pixel = input.get_pixel(i as u32, j as u32).0;
            boxed[k] = RgbSum {
                r: pixel[0],
                g: pixel[1],
                b: pixel[2],
            };
            k += 1;
        }
    }
    boxed
}

fn max(vals: [u8; 8]) -> u8 {
    vals.iter().cloned().max().unwrap_or_default()
}

fn min(vals: [u8; 8]) -> u8 {
    vals.iter().cloned().min().unwrap_or_default()
}

fn gen_random_position(min: i32, max: i32, rng: &mut ThreadRng) -> [(i32, i32); 3] {
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
