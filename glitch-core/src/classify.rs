use crate::parser::shunting_yard;
use crate::token::Token;
use serde::{Deserialize, Serialize};

/// Per-category classification scores for a glitch expression.
///
/// Each field is a `f64` in `[0.0, 1.0]` produced by soft-saturation
/// normalization (`1 - e^(-raw)`), giving an "N+1" property where
/// additional relevant tokens raise the score with diminishing returns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Classification {
    pub edge: f64,
    pub spatial: f64,
    pub bitwise: f64,
    pub channel: f64,
    pub noise: f64,
    pub symmetry: f64,
    pub blur: f64,
    pub contrast: f64,
    pub arithmetic: f64,
    pub morphological: f64,
    pub feedback: f64,
    pub displacement: f64,
    pub posterization: f64,
    pub pattern: f64,
    pub blending: f64,
    pub brightness: f64,
}

/// Raw (pre-normalization) accumulators, one per category.
#[derive(Default)]
struct RawScores {
    edge: f64,
    spatial: f64,
    bitwise: f64,
    channel: f64,
    noise: f64,
    symmetry: f64,
    blur: f64,
    contrast: f64,
    arithmetic: f64,
    morphological: f64,
    feedback: f64,
    displacement: f64,
    posterization: f64,
    pattern: f64,
    blending: f64,
    brightness: f64,
}

/// Classify a glitch expression string into 16 effect-category scores.
///
/// Performs static analysis only — inspects the parsed token stream without
/// requiring any image data or evaluation context.
///
/// # Errors
/// Returns the parser error if the expression has invalid syntax.
///
/// # Example
/// ```
/// let c = glitch_core::classify::classify("128 ^ (e | (R&c) - x)").unwrap();
/// assert!(c.bitwise > 0.8);
/// assert!(c.edge > 0.5);
/// assert_eq!(c.noise, 0.0);
/// ```
pub fn classify(expr: &str) -> Result<Classification, String> {
    let tokens = shunting_yard(expr)?;
    if tokens.is_empty() {
        return Err("Expression is empty".to_string());
    }

    // --- Phase 1: Collect interaction metadata ---
    let has_coords = tokens.iter().any(|t| matches!(t, Token::Char('x' | 'y')));

    // Count distinct pixel sources for blending detection.
    let source_count = count_distinct_sources(&tokens);

    // --- Phase 2: Accumulate raw scores ---
    let mut raw = RawScores::default();

    for tok in &tokens {
        accumulate_token(tok, &mut raw, has_coords, source_count);
    }

    // --- Phase 3: Normalize via soft saturation ---
    Ok(Classification {
        edge: saturate(raw.edge),
        spatial: saturate(raw.spatial),
        bitwise: saturate(raw.bitwise),
        channel: saturate(raw.channel),
        noise: saturate(raw.noise),
        symmetry: saturate(raw.symmetry),
        blur: saturate(raw.blur),
        contrast: saturate(raw.contrast),
        arithmetic: saturate(raw.arithmetic),
        morphological: saturate(raw.morphological),
        feedback: saturate(raw.feedback),
        displacement: saturate(raw.displacement),
        posterization: saturate(raw.posterization),
        pattern: saturate(raw.pattern),
        blending: saturate(raw.blending),
        brightness: saturate(raw.brightness),
    })
}

/// Soft-saturation: `1 - e^(-x)`, rounded to 1 decimal place.
/// Returns 0.0 for non-positive inputs.
fn saturate(raw: f64) -> f64 {
    if raw <= 0.0 {
        return 0.0;
    }
    let v = 1.0 - (-raw).exp();
    (v * 10.0).round() / 10.0
}

/// Count how many distinct pixel-source tokens appear in the expression.
/// Sources are tokens that read pixel data from some location.
fn count_distinct_sources(tokens: &[Token]) -> usize {
    let mut seen = std::collections::HashSet::new();

    for tok in tokens {
        let key: Option<&str> = match tok {
            Token::Char('c') => Some("c"),
            Token::Char('s') => Some("s"),
            Token::Char('e') => Some("e"),
            Token::Char('h') => Some("h"),
            Token::Char('v') => Some("v"),
            Token::Char('d') => Some("d"),
            Token::Char('g') => Some("g"),
            Token::Char('t') => Some("t"),
            Token::Char('N') => Some("N"),
            Token::Char('H') => Some("H"),
            Token::Char('L') => Some("L"),
            Token::Char('Y') => Some("Y"),
            Token::Char('x') => Some("x"),
            Token::Char('y') => Some("y"),
            Token::Random(_) => Some("r"),
            Token::RGBColor(_) => Some("RGB"),
            Token::Brightness(_) => Some("b"),
            Token::Invert => Some("i"),
            _ => None,
        };
        if let Some(k) = key {
            seen.insert(k);
        }
    }

    seen.len()
}

/// Add a single token's signal weights to the raw score accumulators.
fn accumulate_token(
    tok: &Token,
    raw: &mut RawScores,
    has_coords: bool,
    source_count: usize,
) {
    // Blending multiplier: only count blending ops when multiple sources exist.
    let blend_mult = if source_count >= 2 { 1.0 } else { 0.0 };
    // Pattern multiplier: coordinate-dependent operators only score pattern
    // when coordinate tokens (x, y) are present.
    let pattern_mult = if has_coords { 1.0 } else { 0.0 };

    match tok {
        // ── Values ──────────────────────────────────────────────────────

        Token::Num(_) => {
            // Constants contribute mildly to brightness when combined with
            // arithmetic, but that interaction is hard to detect statically.
            // We give a small baseline.
            raw.brightness += 0.1;
        }

        Token::Char('c') => {
            // Identity / current pixel — no signal by itself.
        }

        Token::Char('s') => {
            raw.feedback += 1.0;
        }

        Token::Char('Y') => {
            raw.channel += 0.7;
        }

        Token::Char('x') => {
            raw.spatial += 1.0;
            raw.pattern += 0.8;
        }

        Token::Char('y') => {
            raw.spatial += 1.0;
            raw.pattern += 0.8;
        }

        Token::Char('e') => {
            raw.edge += 1.0;
            raw.morphological += 0.4;
            raw.displacement += 0.5;
        }

        Token::Char('H') => {
            raw.morphological += 1.0;
            raw.edge += 0.3;
            raw.contrast += 0.4;
            raw.blur += 0.2;
            raw.displacement += 0.5;
        }

        Token::Char('L') => {
            raw.morphological += 1.0;
            raw.edge += 0.3;
            raw.contrast += 0.4;
            raw.blur += 0.2;
            raw.displacement += 0.5;
        }

        Token::Char('N') => {
            raw.noise += 1.0;
        }

        Token::Char('h') => {
            raw.symmetry += 1.0;
            raw.spatial += 0.4;
            raw.displacement += 0.8;
        }

        Token::Char('v') => {
            raw.symmetry += 1.0;
            raw.spatial += 0.4;
            raw.displacement += 0.8;
        }

        Token::Char('d') => {
            raw.symmetry += 1.0;
            raw.spatial += 0.4;
            raw.displacement += 0.8;
        }

        Token::Char('g') => {
            raw.noise += 0.8;
            raw.displacement += 1.0;
            raw.spatial += 0.2;
        }

        Token::Char('t') => {
            raw.noise += 0.6;
            raw.displacement += 0.7;
            raw.blur += 0.3;
            raw.spatial += 0.2;
        }

        Token::Random(_) => {
            raw.noise += 0.7;
            raw.displacement += 0.7;
            raw.blur += 0.4;
            raw.spatial += 0.2;
        }

        Token::RGBColor(_) => {
            raw.channel += 1.0;
        }

        Token::Brightness(_) => {
            raw.brightness += 1.0;
            raw.contrast += 0.5;
        }

        Token::Invert => {
            raw.contrast += 0.7;
            raw.channel += 0.3;
        }

        // ── Binary operators ────────────────────────────────────────────

        Token::Add => {
            raw.arithmetic += 0.5;
            raw.blending += 0.3 * blend_mult;
        }

        Token::Sub => {
            raw.arithmetic += 0.5;
            raw.edge += 0.15;
            raw.blending += 0.3 * blend_mult;
        }

        Token::Mul => {
            raw.arithmetic += 0.7;
            raw.contrast += 0.2;
            raw.brightness += 0.2;
            raw.pattern += 0.3 * pattern_mult;
        }

        Token::Div => {
            raw.arithmetic += 0.7;
            raw.posterization += 0.5;
        }

        Token::Mod => {
            raw.arithmetic += 0.9;
            raw.posterization += 0.4;
            raw.pattern += 0.6 * pattern_mult;
        }

        Token::Pow => {
            raw.arithmetic += 0.8;
            raw.contrast += 0.8;
        }

        Token::BitAnd => {
            raw.bitwise += 1.0;
            raw.posterization += 0.7;
            raw.pattern += 0.4 * pattern_mult;
        }

        Token::BitOr => {
            raw.bitwise += 1.0;
        }

        Token::BitXor => {
            raw.bitwise += 1.0;
            raw.pattern += 0.5 * pattern_mult;
        }

        Token::BitAndNot => {
            raw.bitwise += 1.0;
        }

        Token::BitLShift => {
            raw.bitwise += 0.8;
            raw.contrast += 0.3;
        }

        Token::BitRShift => {
            raw.bitwise += 0.8;
            raw.posterization += 0.8;
        }

        Token::Greater => {
            raw.contrast += 1.0;
            raw.edge += 0.3;
            raw.posterization += 0.6;
        }

        Token::Weight => {
            raw.blending += 1.0 * blend_mult;
            raw.blur += 0.6;
        }

        // Parens should never be in RPN output, but be safe.
        Token::LeftParen | Token::RightParen => {}

        // Catch-all for any future tokens.
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: assert a score is approximately equal to expected (±0.1).
    fn approx(actual: f64, expected: f64, label: &str) {
        assert!(
            (actual - expected).abs() <= 0.1 + f64::EPSILON,
            "{}: expected ~{:.1}, got {:.1}",
            label,
            expected,
            actual
        );
    }

    #[test]
    fn test_worked_example() {
        // 128 ^ (e | (R&c) - x)
        let c = classify("128 ^ (e | (R&c) - x)").unwrap();

        // Bitwise: ^ + | + & = 3.0 raw → 0.95 → 0.9 or 1.0
        assert!(c.bitwise >= 0.9, "bitwise: {}", c.bitwise);

        // Edge: e=1.0, -=0.15 → 1.15 raw → ~0.7
        approx(c.edge, 0.7, "edge");

        // Spatial: x=1.0 → 0.63 → 0.6
        approx(c.spatial, 0.6, "spatial");

        // Channel: R=1.0 → 0.63 → 0.6
        approx(c.channel, 0.6, "channel");

        // Noise: nothing → 0.0
        assert_eq!(c.noise, 0.0, "noise should be 0");

        // Symmetry: nothing → 0.0
        assert_eq!(c.symmetry, 0.0, "symmetry should be 0");

        // Blur: nothing → 0.0
        assert_eq!(c.blur, 0.0, "blur should be 0");
    }

    #[test]
    fn test_pure_noise() {
        let c = classify("N").unwrap();
        approx(c.noise, 0.6, "noise");
        assert_eq!(c.edge, 0.0);
        assert_eq!(c.bitwise, 0.0);
        assert_eq!(c.symmetry, 0.0);
    }

    #[test]
    fn test_symmetry_heavy() {
        // h + v + d → symmetry from h,v,d; spatial from h,v,d; displacement from h,v,d
        let c = classify("h + v + d").unwrap();

        // Symmetry: h=1.0 + v=1.0 + d=1.0 = 3.0 raw → 0.95 → 1.0
        assert!(c.symmetry >= 0.9, "symmetry: {}", c.symmetry);

        // Displacement: 0.8 * 3 = 2.4 raw → high
        assert!(c.displacement >= 0.8, "displacement: {}", c.displacement);

        // Spatial: 0.4 * 3 = 1.2 raw → ~0.7
        assert!(c.spatial >= 0.6, "spatial: {}", c.spatial);

        // Blending: + appears twice, sources = h,v,d (3 sources > 2) → activated
        assert!(c.blending > 0.0, "blending: {}", c.blending);
    }

    #[test]
    fn test_feedback() {
        let c = classify("c + s").unwrap();
        approx(c.feedback, 0.6, "feedback");
    }

    #[test]
    fn test_coordinate_pattern() {
        // x ^ y → pattern detection: has_coords=true, ^ contributes pattern
        let c = classify("x ^ y").unwrap();
        assert!(c.pattern >= 0.7, "pattern: {}", c.pattern);
        assert!(c.spatial >= 0.8, "spatial: {}", c.spatial);
        assert!(c.bitwise >= 0.6, "bitwise: {}", c.bitwise);
    }

    #[test]
    fn test_channel_isolation() {
        let c = classify("R + G128").unwrap();
        assert!(c.channel >= 0.8, "channel: {}", c.channel);
    }

    #[test]
    fn test_morphological() {
        let c = classify("H - L").unwrap();
        // H=1.0 + L=1.0 = 2.0 raw → ~0.86 → 0.9
        assert!(c.morphological >= 0.8, "morphological: {}", c.morphological);
        // Edge from H=0.3 + L=0.3 + Sub=0.15 = 0.75 → ~0.5
        assert!(c.edge >= 0.4, "edge: {}", c.edge);
    }

    #[test]
    fn test_posterization() {
        // c & 224 → BitAnd posterization + bitwise
        let c = classify("c & 224").unwrap();
        assert!(c.posterization >= 0.4, "posterization: {}", c.posterization);
        assert!(c.bitwise >= 0.6, "bitwise: {}", c.bitwise);
    }

    #[test]
    fn test_brightness() {
        let c = classify("b128").unwrap();
        approx(c.brightness, 0.7, "brightness");
        assert!(c.contrast > 0.0, "contrast should be non-zero");
    }

    #[test]
    fn test_pattern_without_coords() {
        // % without x or y should NOT produce pattern signal
        let c = classify("c % 7").unwrap();
        assert_eq!(c.pattern, 0.0, "pattern without coords: {}", c.pattern);
    }

    #[test]
    fn test_blending_needs_multiple_sources() {
        // c + 128 → only 1 source (c), blending should be zero
        let c = classify("c + 128").unwrap();
        assert_eq!(c.blending, 0.0, "blending single source: {}", c.blending);
    }

    #[test]
    fn test_invalid_expression() {
        assert!(classify("$$$").is_err());
    }

    #[test]
    fn test_empty_expression() {
        assert!(classify("").is_err());
    }

    #[test]
    fn test_complex_expression() {
        // A complex real-world expression touching many categories
        let c = classify("128 & (e | (R&c) - x) + N * h").unwrap();
        assert!(c.bitwise > 0.0, "should have bitwise");
        assert!(c.edge > 0.0, "should have edge");
        assert!(c.noise > 0.0, "should have noise");
        assert!(c.symmetry > 0.0, "should have symmetry");
        assert!(c.spatial > 0.0, "should have spatial");
        assert!(c.channel > 0.0, "should have channel");
        assert!(c.displacement > 0.0, "should have displacement");
    }
}
