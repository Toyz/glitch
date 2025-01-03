#[derive(Debug, Clone, Copy, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, b: u8, g: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn new_red(r: u8) -> Self {
        Self { r, g: 0, b: 0 }
    }

    pub const fn new_green(g: u8) -> Self {
        Self { r: 0, g, b: 0 }
    }

    pub const fn new_blue(b: u8) -> Self {
        Self { r: 0, g: 0, b }
    }
}

impl From<[u8; 3]> for Rgb {
    fn from(rgb: [u8; 3]) -> Self {
        Self {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        }
    }
}

impl From<Rgb> for image::Rgb<u8> {
    fn from(rgb: Rgb) -> Self {
        Self([rgb.r, rgb.g, rgb.b])
    }
}