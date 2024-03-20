use image::{DynamicImage, GenericImageView};

/// Represents the bounds of an image with non-zero pixels.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl Bounds {
    /// Creates a new `Bounds` instance. Initially, min values are set very high and max values very low
    /// so they can be adjusted down or up respectively to find the actual bounds.
    fn new() -> Self {
        Bounds {
            min_x: u32::MAX,
            min_y: u32::MAX,
            max_x: u32::MIN,
            max_y: u32::MIN,
        }
    }

    /// Updates the bounds based on the x, y coordinates provided.
    fn update(&mut self, x: u32, y: u32) {
        if x < self.min_x {
            self.min_x = x;
        }
        if x > self.max_x {
            self.max_x = x;
        }
        if y < self.min_y {
            self.min_y = y;
        }
        if y > self.max_y {
            self.max_y = y;
        }
    }

    pub fn min_x(&self) -> u32 {
        self.min_x
    }

    pub fn max_x(&self) -> u32 {
        self.max_x
    }

    pub fn min_y(&self) -> u32 {
        self.min_y
    }

    pub fn max_y(&self) -> u32 {
        self.max_y
    }
}

/// Finds the bounds of non-zero pixels in an image.
pub fn find_non_zero_bounds(img: &DynamicImage) -> Option<Bounds> {
    let width = img.width();
    let height = img.height();

    let mut bounds = Bounds::new();

    for y in 0..height {
        for x in 0..width {
            let [r, g, b, a] = img.get_pixel(x, y).0;
            if r != 0 || g != 0 || b != 0 || a != 0 {
                bounds.update(x, y);
            }
        }
    }

    // Check if bounds were updated, implying the image had non-zero pixels
    if bounds.min_x <= bounds.max_x && bounds.min_y <= bounds.max_y {
        Some(bounds)
    } else {
        None // No non-zero pixels found
    }
}