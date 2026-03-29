use std::fmt::Formatter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, Serialize, Deserialize)]
pub enum Token {
    // -- Literals and values
    Num(u8),
    Random(u8),
    Brightness(u8),
    RGBColor((char, u8)),
    Char(char),

    // -- Arithmetic operators
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

    // -- Bitwise operators
    BitAnd,
    BitOr,
    BitXor,
    BitAndNot,
    BitLShift,
    BitRShift,
    Invert,

    // -- Comparison
    Greater,

    // -- Other symbols / markers
    Weight,
    LeftParen,
    RightParen,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Char(ch) => match ch {
                'c' => f.write_str("Current Pixel Value"),
                'b' => f.write_str("Blurred"),
                'h' => f.write_str("Horizontal"),
                'v' => f.write_str("Vertical"),
                'd' => f.write_str("Diagonal"),
                'Y' => f.write_str("Luminosity"),
                'N' => f.write_str("Noise"),
                'R' => f.write_str("Red"),
                'G' => f.write_str("Green"),
                'B' => f.write_str("Blue"),
                's' => f.write_str("Previous Saved Pixel Value"),
                't' => f.write_str("Random Color in 6x6 Grid"),
                'g' => f.write_str("Random Color in the Entire Image"),
                'x' => f.write_str("X Coordinate"),
                'y' => f.write_str("Y Coordinate"),
                'H' => f.write_str("Highest Value"),
                'L' => f.write_str("Lowest Value"),
                _ => write!(f, "{:?}", self),
            },
            Self::Num(n) => write!(f, "Num({})", n),
            Self::BitAnd => f.write_str("Bitwise AND"),
            Self::BitAndNot => f.write_str("Bitwise AND NOT"),
            Self::BitOr => f.write_str("Bitwise OR"),
            Self::BitXor => f.write_str("Bitwise XOR"),
            Self::BitLShift => f.write_str("Bitwise Left Shift"),
            Self::BitRShift => f.write_str("Bitwise Right Shift"),
            Self::Add => f.write_str("Addition"),
            Self::Sub => f.write_str("Subtraction"),
            Self::Mul => f.write_str("Multiplication"),
            Self::Div => f.write_str("Division"),
            Self::Mod => f.write_str("Modulus"),
            Self::Pow => f.write_str("Power"),
            Self::Greater => f.write_str("Greater"),
            Self::Weight => f.write_str("Weight"),
            Self::Invert => f.write_str("Invert"),
            Self::Random(range) => write!(f, "Random color grid - {range}x{range}"),
            Self::RGBColor((part, val)) => write!(f, "RGB Color - {part}: {val}"),
            Self::Brightness(val) => write!(f, "Brightness - {val}"),
            _ => write!(f, "{:?}", self),
        }
    }
}