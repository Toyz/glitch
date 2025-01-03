use std::fmt::Formatter;
use ansiterm::{Color, Style};
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

impl Token {
    fn write_dynamic(&self,f: &mut Formatter<'_>, dynamic: &str) -> Result<(), std::fmt::Error> {
        let style = self.get_style();
        let painted = style.paint(dynamic);
        f.write_str(&painted.to_string())
    }

    fn write_styled(&self, f: &mut Formatter<'_>, content: &str) -> Result<(), std::fmt::Error> {
        let style = self.get_style();
        let painted = style.paint(content);
        f.write_str(&painted.to_string())
    }

    fn write_unknown(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let style = self.get_style();
        let painted = style.paint(format!("{:?}", self));
        f.write_str(&painted.to_string())
    }
}

pub trait DisplayStyle {
    fn get_style(&self) -> Style;
}

impl DisplayStyle for Token {
    fn get_style(&self) -> Style {
        match self {
            Self::Num(_) => Style::new().fg(Color::BrightYellow),
            Self::Greater => Style::new().fg(Color::BrightBlue),
            Self::Char(_) => Style::new().fg(Color::BrightBlue),
            Self::BitLShift => Style::new().fg(Color::BrightYellow),
            Self::BitRShift => Style::new().fg(Color::BrightYellow),
            Self::LeftParen => Style::new().fg(Color::BrightCyan),
            Self::RightParen => Style::new().fg(Color::BrightCyan),
            Self::BitAnd => Style::new().fg(Color::BrightYellow),
            Self::BitXor => Style::new().fg(Color::BrightYellow),
            Self::Sub => Style::new().fg(Color::BrightBlue),
            Self::Add => Style::new().fg(Color::BrightBlue),
            Self::Div => Style::new().fg(Color::BrightBlue),
            Self::Mul => Style::new().fg(Color::BrightBlue),
            Self::Mod => Style::new().fg(Color::BrightBlue),
            Self::BitAndNot => Style::new().fg(Color::BrightYellow),
            Self::BitOr => Style::new().fg(Color::BrightYellow),
            Self::Pow => Style::new().fg(Color::BrightYellow),
            Self::Weight => Style::new().fg(Color::BrightYellow),
            Self::Random(_) => Style::new().fg(Color::BrightBlue),
            Self::RGBColor(_) => Style::new().fg(Color::BrightBlue),
            Self::Brightness(_) => Style::new().fg(Color::BrightBlue),
            Self::Invert => Style::new().fg(Color::BrightBlue),
        }
    }
}
impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let content = match self {
            Self::Char(ch) => match ch {
                'c' => "Current Pixel Value",
                'b' => "Blurred",
                'h' => "Horizontal",
                'v' => "Vertical",
                'd' => "Diagonal",
                'Y' => "Luminosity",
                'N' => "Noise",
                'R' => "Red",
                'G' => "Green",
                'B' => "Blue",
                's' => "Previous Saved Pixel Value",
                't' => "Random Color in 6x6 Grid",
                'g' => "Random Color in the Entire Image",
                'x' => "X Coordinate",
                'y' => "Y Coordinate",
                'H' => "Highest Value",
                'L' => "Lowest Value",
                _ => return self.write_unknown(f),
            },
            Self::BitAnd => "Bitwise AND",
            Self::BitAndNot => "Bitwise AND NOT",
            Self::BitOr => "Bitwise OR",
            Self::BitXor => "Bitwise XOR",
            Self::BitLShift => "Bitwise Left Shift",
            Self::BitRShift => "Bitwise Right Shift",
            Self::Add => "Addition",
            Self::Sub => "Subtraction",
            Self::Mul => "Multiplication",
            Self::Div => "Division",
            Self::Mod => "Modulus",
            Self::Pow => "Power",
            Self::Greater => "Greater",
            Self::Weight => "Weight",
            Self::Invert => "Invert",
            Self::Random(range) => {
                return self.write_dynamic(f, &format!("Random color grid - {range}x{range}"));
            }
            Self::RGBColor((part, val)) => {
                return self.write_dynamic(f, &format!("RGB Color - {part}: {val}"));
            }
            Self::Brightness(val) => {
                return self.write_dynamic(f, &format!("Brightness - {val}"));
            }
            _ => return self.write_unknown(f),
        };

        self.write_styled(f, content)
    }
}