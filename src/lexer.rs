//! Turns one line of dice notation into a flat token stream.
//!
//! Kept deliberately dumb: it does not know what a valid expression looks
//! like, it just chops characters into pieces the parser in `linter.rs` can
//! reason about. Column numbers are 1-based and count characters, not bytes,
//! since dice notation is expected to stay ASCII.

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Number(u64),
    Die,      // 'd' / 'D'
    KeepHigh, // "kh" or a bare 'k' (keep-highest is the common default)
    KeepLow,  // "kl"
    Bang,     // '!' (exploding dice)
    Plus,
    Minus,
    Star,     // '*' (multiplied terms)
    LParen,
    RParen,
    Unknown(char),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub col: usize,
}

pub fn tokenize(line: &str) -> Vec<Token> {
    let chars: Vec<char> = line.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            // A run of digits too long for u64 is a red flag on its own;
            // saturating to MAX lets the "unusually large" rule catch it
            // instead of the linter panicking on the input.
            let value = text.parse::<u64>().unwrap_or(u64::MAX);
            tokens.push(Token {
                kind: TokenKind::Number(value),
                col: start + 1,
            });
            continue;
        }

        let col = i + 1;
        match c {
            'd' | 'D' => {
                tokens.push(Token { kind: TokenKind::Die, col });
                i += 1;
            }
            '!' => {
                tokens.push(Token { kind: TokenKind::Bang, col });
                i += 1;
            }
            '+' => {
                tokens.push(Token { kind: TokenKind::Plus, col });
                i += 1;
            }
            '-' => {
                tokens.push(Token { kind: TokenKind::Minus, col });
                i += 1;
            }
            '*' => {
                tokens.push(Token { kind: TokenKind::Star, col });
                i += 1;
            }
            '(' => {
                tokens.push(Token { kind: TokenKind::LParen, col });
                i += 1;
            }
            ')' => {
                tokens.push(Token { kind: TokenKind::RParen, col });
                i += 1;
            }
            'k' | 'K' => match chars.get(i + 1).copied() {
                Some('h') | Some('H') => {
                    tokens.push(Token { kind: TokenKind::KeepHigh, col });
                    i += 2;
                }
                Some('l') | Some('L') => {
                    tokens.push(Token { kind: TokenKind::KeepLow, col });
                    i += 2;
                }
                _ => {
                    tokens.push(Token { kind: TokenKind::KeepHigh, col });
                    i += 1;
                }
            },
            other => {
                tokens.push(Token { kind: TokenKind::Unknown(other), col });
                i += 1;
            }
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<TokenKind> {
        tokenize(line).into_iter().map(|t| t.kind).collect()
    }

    fn cols(line: &str) -> Vec<usize> {
        tokenize(line).into_iter().map(|t| t.col).collect()
    }

    #[test]
    fn empty_line_has_no_tokens() {
        assert_eq!(kinds(""), vec![]);
    }

    #[test]
    fn whitespace_only_has_no_tokens() {
        assert_eq!(kinds("   \t "), vec![]);
    }

    #[test]
    fn plain_number() {
        assert_eq!(kinds("12"), vec![TokenKind::Number(12)]);
        // the whole run of digits is one token starting at the first digit
        assert_eq!(cols("12"), vec![1]);
    }

    #[test]
    fn die_letter_is_case_insensitive() {
        assert_eq!(kinds("d"), vec![TokenKind::Die]);
        assert_eq!(kinds("D"), vec![TokenKind::Die]);
    }

    #[test]
    fn bare_k_defaults_to_keep_high() {
        assert_eq!(kinds("k"), vec![TokenKind::KeepHigh]);
    }

    #[test]
    fn kh_and_kl_are_two_character_tokens() {
        assert_eq!(kinds("kh3"), vec![TokenKind::KeepHigh, TokenKind::Number(3)]);
        assert_eq!(cols("kh3"), vec![1, 3]);
        assert_eq!(kinds("KL2"), vec![TokenKind::KeepLow, TokenKind::Number(2)]);
    }

    #[test]
    fn bang_plus_minus() {
        assert_eq!(
            kinds("!+-"),
            vec![TokenKind::Bang, TokenKind::Plus, TokenKind::Minus]
        );
    }

    #[test]
    fn unknown_character_is_preserved() {
        assert_eq!(kinds("x"), vec![TokenKind::Unknown('x')]);
    }

    #[test]
    fn parens_and_star_are_their_own_tokens() {
        assert_eq!(
            kinds("(2d6)*3"),
            vec![
                TokenKind::LParen,
                TokenKind::Number(2),
                TokenKind::Die,
                TokenKind::Number(6),
                TokenKind::RParen,
                TokenKind::Star,
                TokenKind::Number(3),
            ]
        );
    }

    #[test]
    fn whitespace_between_tokens_is_skipped_but_columns_stay_absolute() {
        assert_eq!(kinds(" d 6"), vec![TokenKind::Die, TokenKind::Number(6)]);
        assert_eq!(cols(" d 6"), vec![2, 4]);
    }

    #[test]
    fn full_expression_columns() {
        // 3 d 6 k h 1 !
        // 1 2 3 4 5 6 7
        assert_eq!(
            kinds("3d6kh1!"),
            vec![
                TokenKind::Number(3),
                TokenKind::Die,
                TokenKind::Number(6),
                TokenKind::KeepHigh,
                TokenKind::Number(1),
                TokenKind::Bang,
            ]
        );
        assert_eq!(cols("3d6kh1!"), vec![1, 2, 3, 4, 6, 7]);
    }

    #[test]
    fn digit_run_too_long_for_u64_saturates_instead_of_panicking() {
        let huge = "9".repeat(30);
        assert_eq!(kinds(&huge), vec![TokenKind::Number(u64::MAX)]);
    }
}
