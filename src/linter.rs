//! Parses a line of dice notation and checks it against a small set of
//! rules. Supported shape: `[N]dM[kh#|kl#][!]` terms joined by `+` / `-`,
//! e.g. `3d6 + 2d4kh1! - 1`.

use crate::lexer::{self, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

pub struct Finding {
    pub line: usize,
    pub col: usize,
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
}

impl Finding {
    pub fn render(&self) -> String {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        format!("{}[{}]: {}", level, self.code, self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Keep {
    High,
    Low,
}

struct DiceTerm {
    count: u64,
    sides: u64,
    keep: Option<(Keep, u64)>,
    col: usize,
}

enum Term {
    Flat,
    Dice(DiceTerm),
}

struct ParseError {
    col: usize,
    message: String,
}

/// Every dice count above this is treated as a probable typo rather than
/// an intentionally huge roll (nobody rolls 50,000 d6 by hand).
const MAX_SANE_DICE_COUNT: u64 = 1000;

pub fn lint_line(line_no: usize, text: &str) -> Vec<Finding> {
    let tokens = lexer::tokenize(text);
    if tokens.is_empty() {
        return Vec::new();
    }
    match parse_terms(&tokens) {
        Ok(terms) => check_terms(line_no, &terms),
        Err(e) => vec![Finding {
            line: line_no,
            col: e.col,
            severity: Severity::Error,
            code: "E000",
            message: e.message,
        }],
    }
}

fn parse_terms(tokens: &[Token]) -> Result<Vec<Term>, ParseError> {
    let mut terms = Vec::new();
    let mut i = 0;
    let mut expect_sign = false;

    while i < tokens.len() {
        if expect_sign {
            match tokens[i].kind {
                TokenKind::Plus | TokenKind::Minus => i += 1,
                _ => {
                    return Err(ParseError {
                        col: tokens[i].col,
                        message: "expected '+' or '-' between terms".to_string(),
                    })
                }
            }
        }

        let leading_number = expect_number(tokens, &mut i)?;

        if i < tokens.len() && tokens[i].kind == TokenKind::Die {
            let die_col = tokens[i].col;
            i += 1;
            let sides = expect_number(tokens, &mut i)?;

            let mut keep = None;
            if i < tokens.len() {
                let kind = match tokens[i].kind {
                    TokenKind::KeepHigh => Some(Keep::High),
                    TokenKind::KeepLow => Some(Keep::Low),
                    _ => None,
                };
                if let Some(kind) = kind {
                    i += 1;
                    let amount = expect_number(tokens, &mut i)?;
                    keep = Some((kind, amount));
                }
            }

            if i < tokens.len() && tokens[i].kind == TokenKind::Bang {
                i += 1;
            }

            terms.push(Term::Dice(DiceTerm {
                count: leading_number,
                sides,
                keep,
                col: die_col,
            }));
        } else {
            terms.push(Term::Flat);
        }

        expect_sign = true;
    }

    Ok(terms)
}

fn expect_number(tokens: &[Token], i: &mut usize) -> Result<u64, ParseError> {
    if *i >= tokens.len() {
        return Err(ParseError {
            col: tokens.last().map(|t| t.col).unwrap_or(1),
            message: "expected a number, found end of expression".to_string(),
        });
    }
    match tokens[*i].kind {
        TokenKind::Number(n) => {
            *i += 1;
            Ok(n)
        }
        ref other => Err(ParseError {
            col: tokens[*i].col,
            message: format!("expected a number, found '{}'", describe(other)),
        }),
    }
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Number(n) => n.to_string(),
        TokenKind::Die => "d".to_string(),
        TokenKind::KeepHigh => "kh".to_string(),
        TokenKind::KeepLow => "kl".to_string(),
        TokenKind::Bang => "!".to_string(),
        TokenKind::Plus => "+".to_string(),
        TokenKind::Minus => "-".to_string(),
        TokenKind::Unknown(c) => c.to_string(),
    }
}

fn check_terms(line_no: usize, terms: &[Term]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for term in terms {
        let dice = match term {
            Term::Dice(d) => d,
            Term::Flat => continue,
        };

        if dice.count == 0 {
            findings.push(Finding {
                line: line_no,
                col: dice.col,
                severity: Severity::Error,
                code: "E001",
                message: "dice count is zero, this term always contributes nothing".to_string(),
            });
        } else if dice.count > MAX_SANE_DICE_COUNT {
            findings.push(Finding {
                line: line_no,
                col: dice.col,
                severity: Severity::Warning,
                code: "W002",
                message: format!("dice count {} is unusually large, check for a typo", dice.count),
            });
        }

        if dice.sides == 0 {
            findings.push(Finding {
                line: line_no,
                col: dice.col,
                severity: Severity::Error,
                code: "E002",
                message: "a die cannot have zero sides".to_string(),
            });
        } else if dice.sides == 1 {
            findings.push(Finding {
                line: line_no,
                col: dice.col,
                severity: Severity::Warning,
                code: "W001",
                message: "a single-sided die always rolls 1, this is probably a typo".to_string(),
            });
        }

        if let Some((_, amount)) = dice.keep {
            if amount == 0 {
                findings.push(Finding {
                    line: line_no,
                    col: dice.col,
                    severity: Severity::Error,
                    code: "E003",
                    message: "keep modifier keeps zero dice".to_string(),
                });
            } else if dice.count > 0 && amount >= dice.count {
                findings.push(Finding {
                    line: line_no,
                    col: dice.col,
                    severity: Severity::Warning,
                    code: "W003",
                    message: "keep modifier keeps all dice, it has no effect here".to_string(),
                });
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(text: &str) -> Vec<&'static str> {
        lint_line(1, text).iter().map(|f| f.code).collect()
    }

    #[test]
    fn valid_expression_has_no_findings() {
        assert_eq!(codes("2d6 + 3"), Vec::<&str>::new());
        assert_eq!(codes("4d6kh3 + 2 - 1d4"), Vec::<&str>::new());
    }

    #[test]
    fn blank_token_stream_has_no_findings() {
        // lint_line is only ever called on non-blank, non-comment lines by
        // main.rs, but it should still behave sanely if it isn't.
        assert_eq!(codes(""), Vec::<&str>::new());
    }

    #[test]
    fn zero_dice_count_is_an_error() {
        let findings = lint_line(5, "0d6");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "E001");
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].line, 5);
        assert_eq!(findings[0].col, 1);
    }

    #[test]
    fn zero_sided_die_is_an_error() {
        let findings = lint_line(1, "1d0");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "E002");
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn single_sided_die_is_a_warning() {
        let findings = lint_line(1, "1d1");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "W001");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn oversized_dice_count_is_a_warning_with_the_count_in_the_message() {
        let findings = lint_line(1, "5000d8");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "W002");
        assert!(findings[0].message.contains("5000"));
    }

    #[test]
    fn dice_count_at_the_sane_limit_does_not_warn() {
        assert_eq!(codes("1000d6"), Vec::<&str>::new());
    }

    #[test]
    fn keeping_zero_dice_is_an_error() {
        let findings = lint_line(1, "4d6kh0");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "E003");
    }

    #[test]
    fn keeping_every_die_rolled_is_a_warning() {
        let findings = lint_line(1, "3d6kh3");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "W003");
    }

    #[test]
    fn keeping_fewer_than_rolled_does_not_warn() {
        assert_eq!(codes("4d6kh1"), Vec::<&str>::new());
    }

    #[test]
    fn a_term_can_produce_more_than_one_finding() {
        // zero sides and an unusually large count at once
        let findings = lint_line(1, "5000d0");
        let mut codes: Vec<_> = findings.iter().map(|f| f.code).collect();
        codes.sort();
        assert_eq!(codes, vec!["E002", "W002"]);
    }

    #[test]
    fn missing_sides_after_die_is_a_parse_error() {
        let findings = lint_line(1, "3d");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "E000");
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn two_terms_without_an_operator_between_them_is_a_parse_error() {
        let findings = lint_line(1, "3d6 2d4");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "E000");
        assert_eq!(findings[0].col, 5);
        assert!(findings[0].message.contains("expected '+' or '-'"));
    }

    #[test]
    fn trailing_junk_character_is_a_parse_error() {
        let findings = lint_line(1, "3d6x");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "E000");
        assert_eq!(findings[0].col, 4);
    }

    #[test]
    fn render_format_matches_docs() {
        let findings = lint_line(1, "0d6");
        assert_eq!(
            findings[0].render(),
            "error[E001]: dice count is zero, this term always contributes nothing"
        );
    }
}
