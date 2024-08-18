#![allow(dead_code)]
use ansiterm::{Color, Style};
use std::collections::VecDeque;
use std::fmt::Formatter;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum Token {
    Num(u8),
    Random(u8),
    RGBColor((char, u8)),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    BitAnd,
    BitOr,
    BitAndNot,
    BitXor,
    BitLShift,
    BitRShift,
    Greater,
    Weight,
    LeftParen,
    RightParen,
    Char(char),
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
        }
    }
}
impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut content: Option<&str> = None;
        match self {
            Self::Char(ch) => match ch {
                'c' => {
                    content = Some("Current Pixel Value");
                }
                'b' => {
                    content = Some("Blurred");
                }
                'h' => {
                    content = Some("Horizontal");
                }
                'v' => {
                    content = Some("Vertical");
                }
                'd' => {
                    content = Some("Diagonal");
                }
                'Y' => {
                    content = Some("Luminosity");
                }
                'N' => {
                    content = Some("Noise");
                }
                'R' => {
                    content = Some("Red");
                }
                'G' => {
                    content = Some("Green");
                }
                'B' => {
                    content = Some("Blue");
                }
                'S' => {
                    content = Some("Previous Saved Pixel Value");
                }
                't' => {
                    content = Some("Random Color in 6x6 Grid");
                }
                'g' => {
                    content = Some("Random Color in the Entire Image");
                }
                'x' => {
                    content = Some("X Coordinate");
                }
                'y' => {
                    content = Some("Y Coordinate");
                }
                'H' => {
                    content = Some("Highest Value");
                }
                'L' => {
                    content = Some("Lowest Value");
                }
                _ => {}
            },
            Self::BitAnd => {
                content = Some("Bitwise AND");
            }
            Self::BitAndNot => {
                content = Some("Bitwise AND NOT");
            }
            Self::BitOr => {
                content = Some("Bitwise OR");
            }
            Self::BitXor => {
                content = Some("Bitwise XOR");
            }
            Self::BitLShift => {
                content = Some("Bitwise Left Shift");
            }
            Self::BitRShift => {
                content = Some("Bitwise Right Shift");
            }
            Self::Add => {
                content = Some("Addition");
            }
            Self::Sub => {
                content = Some("Subtraction");
            }
            Self::Mul => {
                content = Some("Multiplication");
            }
            Self::Div => {
                content = Some("Division");
            }
            Self::Mod => {
                content = Some("Modulus");
            }
            Self::Pow => {
                content = Some("Power");
            }
            Self::Greater => {
                content = Some("Greater");
            }
            Self::Weight => {
                content = Some("Weight");
            }
            Self::Random(range) => {
                content = Some(format!("Random color grid -{range}x{range}").leak());
            }
            Self::RGBColor((part, val)) => {
                content = Some(format!("RGB Color - {part}: {val}").leak());
            }
            _ => {}
        }
        match content {
            Some(content) => {
                let style = self.get_style();
                let painted = style.paint(content);
                f.write_str(painted.to_string().as_str())
            }
            None => {
                let style = self.get_style();
                let painted = style.paint(format!("{:?}", self));
                f.write_str(painted.to_string().as_str())
            }
        }
    }
}

pub fn shunting_yard(input: &str) -> Result<Vec<Token>, String> {
    let mut output_queue: VecDeque<Token> = VecDeque::new();
    let mut operator_stack: Vec<Token> = Vec::new();
    let mut number_buffer: Option<u8> = None;
    let mut current_position: usize = 0;

    let push_number_buffer = |number_buffer: &mut Option<u8>,
                              output_queue: &mut VecDeque<Token>,
                              _position: usize|
     -> Result<(), String> {
        if let Some(number) = *number_buffer {
            output_queue.push_back(Token::Num(number));
            *number_buffer = None;
        }
        Ok(())
    };

    let mut chars_iter = input.chars().peekable();
    while let Some(c) = chars_iter.next() {
        current_position += 1; // Update position for each character
        match c {
            '0'..='9' => {
                let digit = c.to_digit(10).unwrap() as i64;
                number_buffer = match number_buffer {
                    Some(number) => {
                        let new_number = number as i64 * 10i64 + digit;
                        if new_number > 255 {
                            return Err(format!(
                                "Number exceeds 255 at position {}",
                                current_position
                            ));
                        } else {
                            Some(new_number as u8)
                        }
                    }
                    None => Some(digit as u8),
                };
            }
            'r' => {
                push_number_buffer(&mut number_buffer, &mut output_queue, current_position)?;
                let mut range_str = String::new();
                while let Some(&next_char) = chars_iter.peek() {
                    if next_char.is_ascii_digit() {
                        range_str.push(chars_iter.next().unwrap());
                        current_position += 1;
                    } else {
                        break;
                    }
                }
                let range = if range_str.is_empty() {
                    1
                } else {
                    range_str.parse::<u8>().map_err(|_| {
                        format!("Invalid range specified at position {}", current_position)
                    })?
                };

                if range == 0 {
                    return Err("Range cannot be 0 just use 'c'".to_string());
                }

                output_queue.push_back(Token::Random(range));
            }
            'R' | 'G' | 'B' => {
                push_number_buffer(&mut number_buffer, &mut output_queue, current_position)?;
                let part = c;
                let mut value_str = String::new();
                while let Some(&next_char) = chars_iter.peek() {
                    if next_char.is_ascii_digit() {
                        value_str.push(chars_iter.next().unwrap());
                        current_position += 1;
                    } else {
                        break;
                    }
                }
                let value = if value_str.is_empty() {
                    255
                } else {
                    value_str.parse::<u8>().map_err(|_| {
                        format!("Invalid value specified at position {}", current_position)
                    })?
                };
                output_queue.push_back(Token::RGBColor((part, value)));
            }
            c if char_to_token(c).is_some() => {
                push_number_buffer(&mut number_buffer, &mut output_queue, current_position)?;
                if let Some(token) = char_to_token(c) {
                    handle_operator(&mut operator_stack, &mut output_queue, token);
                }
            }
            '(' => {
                push_number_buffer(&mut number_buffer, &mut output_queue, current_position)?;
                operator_stack.push(Token::LeftParen);
            }
            ')' => {
                push_number_buffer(&mut number_buffer, &mut output_queue, current_position)?;
                while let Some(op) = operator_stack.pop() {
                    if matches!(op, Token::LeftParen) {
                        break;
                    }
                    output_queue.push_back(op);
                }
            }
            _ if c.is_whitespace() => {
                push_number_buffer(&mut number_buffer, &mut output_queue, current_position)?;
            }
            _ if valid_tok(c) => {
                output_queue.push_back(Token::Char(c));
            }
            _ => {
                return Err(format!(
                    "Invalid character '{}' at position {}",
                    c, current_position
                ))
            }
        }
    }

    push_number_buffer(&mut number_buffer, &mut output_queue, current_position)?;

    while let Some(op) = operator_stack.pop() {
        if matches!(op, Token::LeftParen) {
            return Err("Mismatched parenthesis detected".to_string());
        }
        output_queue.push_back(op);
    }

    Ok(output_queue.into())
}

fn handle_operator(operator_stack: &mut Vec<Token>, output_queue: &mut VecDeque<Token>, op: Token) {
    while let Some(top_op) = operator_stack.last() {
        if is_higher_precedence(&op, top_op) {
            break;
        }
        output_queue.push_back(operator_stack.pop().unwrap());
    }
    operator_stack.push(op);
}

const fn is_higher_precedence(new_op: &Token, top_op: &Token) -> bool {
    let (new_prec, _) = operator_precedence(new_op);
    let (_, top_prec) = operator_precedence(top_op);
    new_prec > top_prec
}

const fn operator_precedence(op: &Token) -> (i32, i32) {
    match op {
        Token::Add | Token::Sub | Token::BitOr | Token::BitXor => (4, 4),
        Token::Mul
        | Token::Div
        | Token::Mod
        | Token::BitAnd
        | Token::BitAndNot
        | Token::BitLShift
        | Token::BitRShift
        | Token::Pow => (5, 5),
        Token::Greater | Token::Weight => (6, 6),
        _ => (-1, -1),
    }
}

const fn valid_tok(tok: char) -> bool {
    matches!(
        tok,
        'c' | 's'
            | 'Y'
            | 'x'
            | 'y'
            | 'N'
            /*| 'R'
            | 'G'
            | 'B'*/
            | 'e'
            | 'b'
            | 'H'
            | 'L'
            | 'h'
            | 'v'
            | 'd'
            | 'g'
            | 't'
    )
}

const fn char_to_token(c: char) -> Option<Token> {
    match c {
        '+' => Some(Token::Add),
        '-' => Some(Token::Sub),
        '*' => Some(Token::Mul),
        '/' => Some(Token::Div),
        '%' => Some(Token::Mod),
        '#' => Some(Token::Pow),
        '&' => Some(Token::BitAnd),
        '|' => Some(Token::BitOr),
        ':' => Some(Token::BitAndNot),
        '^' => Some(Token::BitXor),
        '<' => Some(Token::BitLShift),
        '>' => Some(Token::BitRShift),
        '?' => Some(Token::Greater),
        '@' => Some(Token::Weight),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_expression() {
        let input = "3+5";
        let expected = Ok(vec![Token::Num(3), Token::Num(5), Token::Add]);
        assert_eq!(shunting_yard(input), expected);
    }

    #[test]
    fn test_invalid_character() {
        let input = "3$5";
        assert!(shunting_yard(input).is_err());
    }

    #[test]
    fn test_number_exceeds_255() {
        let input = "256";
        assert!(shunting_yard(input).is_err());
    }

    #[test]
    fn test_mixed_operators() {
        let input = "3+5*2";
        let expected = Ok(vec![
            Token::Num(3),
            Token::Num(5),
            Token::Num(2),
            Token::Mul,
            Token::Add,
        ]);
        assert_eq!(shunting_yard(input), expected);
    }

    #[test]
    fn test_parentheses() {
        let input = "(3+5)*2";
        let expected = Ok(vec![
            Token::Num(3),
            Token::Num(5),
            Token::Add,
            Token::Num(2),
            Token::Mul,
        ]);
        assert_eq!(shunting_yard(input), expected);
    }

    #[test]
    fn test_mismatched_parentheses() {
        let input = "(3+5*2";
        assert!(shunting_yard(input).is_err());
    }

    #[test]
    fn test_valid_characters() {
        let input = "c+Y";
        let expected = Ok(vec![Token::Char('c'), Token::Char('Y'), Token::Add]);
        assert_eq!(shunting_yard(input), expected);
    }

    #[test]
    fn test_complete_expression() {
        let input = "3 + 5 / (2 - 1) * 4";
        let expected = Ok(vec![
            Token::Num(3),
            Token::Num(5),
            Token::Num(2),
            Token::Num(1),
            Token::Sub,
            Token::Div,
            Token::Num(4),
            Token::Mul,
            Token::Add,
        ]);
        assert_eq!(shunting_yard(input), expected);
    }
}
