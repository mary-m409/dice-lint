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
