#![allow(dead_code)]

use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Num(u8),
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
    CharToken(char),
}

pub(crate) fn shunting_yard(input: &str) -> Result<Vec<Token>, String> {
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

    for c in input.chars() {
        current_position += 1; // Update position for each character
        match c {
            '0'..='9' => {
                let digit = c.to_digit(10).unwrap() as i64;
                number_buffer = match number_buffer {
                    Some(number) => {
                        let new_number = number as i64 * 10i64 + digit as i64;
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
            '+' | '-' | '*' | '/' | '%' | '#' | '&' | '|' | ':' | '^' | '<' | '>' | '?' | '@' => {
                push_number_buffer(&mut number_buffer, &mut output_queue, current_position)?;
                handle_operator(
                    &mut operator_stack,
                    &mut output_queue,
                    match c {
                        '+' => Token::Add,
                        '-' => Token::Sub,
                        '*' => Token::Mul,
                        '/' => Token::Div,
                        '%' => Token::Mod,
                        '#' => Token::Pow,
                        '&' => Token::BitAnd,
                        '|' => Token::BitOr,
                        ':' => Token::BitAndNot,
                        '^' => Token::BitXor,
                        '<' => Token::BitLShift,
                        '>' => Token::BitRShift,
                        '?' => Token::Greater,
                        '@' => Token::Weight,
                        _ => unreachable!(),
                    },
                );
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
                output_queue.push_back(Token::CharToken(c));
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

fn is_higher_precedence(new_op: &Token, top_op: &Token) -> bool {
    let (new_prec, _) = operator_precedence(new_op);
    let (_, top_prec) = operator_precedence(top_op);
    new_prec > top_prec
}

fn operator_precedence(op: &Token) -> (i32, i32) {
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

fn valid_tok(tok: char) -> bool {
    matches!(
        tok,
        'c' | 's'
            | 'Y'
            | 'r'
            | 'x'
            | 'y'
            | 'N'
            | 'R'
            | 'G'
            | 'B'
            | 'e'
            | 'b'
            | 'H'
            | 'L'
            | 'h'
            | 'v'
            | 'd'
    )
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
        let expected = Ok(vec![
            Token::CharToken('c'),
            Token::CharToken('Y'),
            Token::Add,
        ]);
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
