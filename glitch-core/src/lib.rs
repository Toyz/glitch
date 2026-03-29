#![deny(clippy::perf, clippy::correctness)]
#![warn(rust_2018_idioms, clippy::complexity, clippy::nursery)]

pub mod bounds;
pub mod eval;
pub mod parser;
pub mod rgb;
pub mod token;

pub use eval::EvalContext;
pub use token::Token;
pub use rgb::Rgb;

/// Result of a successful expression verification.
#[derive(Debug, Clone)]
pub struct VerifyResult {
    /// The parsed RPN token list.
    pub tokens: Vec<Token>,
    /// Human-readable description of each token (via Display).
    pub token_descriptions: Vec<String>,
    /// Number of tokens in the compiled expression.
    pub token_count: usize,
}

/// Parse and verify an expression without needing any image data.
///
/// This performs two levels of validation:
/// 1. **Syntax** — Shunting Yard parsing (balanced parens, valid tokens, number ranges)
/// 2. **Semantics** — Stack simulation (no underflows, exactly one result)
///
/// # Errors
/// Returns a human-readable error string if:
/// - The expression has invalid syntax (from parser)
/// - An operator would underflow the stack (not enough operands)
/// - The expression produces zero or multiple results
///
/// # Example
/// ```
/// let result = glitch_core::verify("128 & (c - 150)").unwrap();
/// assert_eq!(result.token_count, 5); // [128, c, 150, -, &]
/// ```
pub fn verify(expr: &str) -> Result<VerifyResult, String> {
    // Phase 1: Syntax — parse into RPN tokens
    let tokens = parser::shunting_yard(expr)?;

    // Phase 2: Semantics — simulate the evaluation stack
    let mut depth: i32 = 0;

    for (i, tok) in tokens.iter().enumerate() {
        let (pops, pushes) = stack_effect(tok);

        if depth < pops {
            return Err(format!(
                "Stack underflow at token {} ({}): needs {} operand{} but stack has {}",
                i + 1,
                tok,
                pops,
                if pops == 1 { "" } else { "s" },
                depth
            ));
        }

        depth -= pops;
        depth += pushes;
    }

    if depth == 0 {
        return Err("Expression produces no result (empty or all operators)".to_string());
    }

    if depth > 1 {
        return Err(format!(
            "Expression produces {} values instead of 1 — missing operator(s) between values",
            depth
        ));
    }

    let token_descriptions = tokens.iter().map(|t| format!("{}", t)).collect();
    let token_count = tokens.len();

    Ok(VerifyResult {
        tokens,
        token_descriptions,
        token_count,
    })
}

/// Returns (pops, pushes) for each token's stack effect.
fn stack_effect(tok: &Token) -> (i32, i32) {
    match tok {
        // Values — push 1
        Token::Num(_)
        | Token::Char(_)
        | Token::Random(_)
        | Token::RGBColor(_)
        | Token::Brightness(_)
        | Token::Invert => (0, 1),

        // Binary operators — pop 2, push 1
        Token::Add
        | Token::Sub
        | Token::Mul
        | Token::Div
        | Token::Mod
        | Token::Pow
        | Token::BitAnd
        | Token::BitOr
        | Token::BitXor
        | Token::BitAndNot
        | Token::BitLShift
        | Token::BitRShift
        | Token::Greater
        | Token::Weight => (2, 1),

        // Parens should never appear in RPN output, but be safe
        Token::LeftParen | Token::RightParen => (0, 0),
    }
}
