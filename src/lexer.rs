#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    Word(String),
    Pipe,
    RedirectWrite,
    RedirectAppend,
    RedirectRead,
    Ampersand,
}

enum State {
    Normal,
    InSingleQuote,
    InDoubleQuote,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut current_word = String::new();
    let mut state = State::Normal;
    let mut chars = input.chars().peekable();
    let mut has_token_content = false;

    while let Some(ch) = chars.next() {
        match state {
            State::Normal => match ch {
                '\'' => {
                    state = State::InSingleQuote;
                    has_token_content = true;
                }
                '"' => {
                    state = State::InDoubleQuote;
                    has_token_content = true;
                }
                '\\' => {
                    if let Some(next_ch) = chars.next() {
                        current_word.push(next_ch);
                        has_token_content = true;
                    } else {
                        return Err("Trailing blackslash at end of input".to_string());
                    }
                }
                '|' => {
                    flush_word(&mut current_word, &mut has_token_content, &mut tokens);
                    tokens.push(Token::Pipe);
                }
                '&' => {
                    flush_word(&mut current_word, &mut has_token_content, &mut tokens);
                    tokens.push(Token::Ampersand);
                }
                '>' => {
                    flush_word(&mut current_word, &mut has_token_content, &mut tokens);
                    if chars.peek() == Some(&'>') {
                        chars.next();
                        tokens.push(Token::RedirectAppend);
                    } else {
                        tokens.push(Token::RedirectWrite);
                    }
                }
                '<' => {
                    flush_word(&mut current_word, &mut has_token_content, &mut tokens);
                    tokens.push(Token::RedirectRead);
                }

                c if c.is_whitespace() => {
                    flush_word(&mut current_word, &mut has_token_content, &mut tokens);
                }

                _ => {
                    current_word.push(ch);
                    has_token_content = true;
                }
            },
            State::InSingleQuote => match ch {
                '\'' => state = State::Normal,
                _ => current_word.push(ch),
            },
            State::InDoubleQuote => match ch {
                '"' => state = State::Normal,
                '\\' => {
                    if let Some(&next_ch) = chars.peek() {
                        if matches!(next_ch, '$' | '`' | '"' | '\\' | '\n') {
                            chars.next();
                            current_word.push(next_ch);
                        } else {
                            current_word.push('\\');
                        }
                    } else {
                        current_word.push('\\');
                    }
                }
                _ => current_word.push(ch),
            },
        }
    }

    match state {
        State::InSingleQuote => return Err("Unterminated single quote (')".to_string()),
        State::InDoubleQuote => return Err("Unterminated double quote (\")".to_string()),
        State::Normal => {
            flush_word(&mut current_word, &mut has_token_content, &mut tokens);
        }
    }
    Ok(tokens)
}

fn flush_word(word: &mut String, has_content: &mut bool, tokens: &mut Vec<Token>) {
    if *has_content || !word.is_empty() {
        tokens.push(Token::Word(std::mem::take(word)));
        *has_content = false;
    }
}
