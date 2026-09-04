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
