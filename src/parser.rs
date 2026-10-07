use crate::lexer::Token;

#[derive(Debug, PartialEq, Eq)]
pub enum Redirection {
    Read(String),
    Write(String),
    Append(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Command {
    pub args: Vec<String>,
    pub redirects: Vec<Redirection>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pipeline {
    pub commands: Vec<Command>,
    pub background: bool,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    pub fn parse(&mut self) -> Result<Pipeline, String> {
        let mut commands = Vec::new();

        commands.push(self.parse_command()?);

        while let Some(Token::Pipe) = self.peek() {
            self.next();
            commands.push(self.parse_command()?);
        }

        let mut background = false;
        if let Some(Token::Ampersand) = self.peek() {
            self.next();
            background = true;
        }

        if self.pos < self.tokens.len() {
            return Err("Unexpected token after pipeline".to_string());
        }
        Ok(Pipeline {
            commands,
            background,
        })
    }

    fn parse_command(&mut self) -> Result<Command, String> {
        let mut args = Vec::new();
        let mut redirects = Vec::new();

        while let Some(token) = self.peek() {
            match token {
                Token::Word(_) => {
                    if let Some(Token::Word(w)) = self.next() {
                        args.push(w);
                    }
                }
                Token::RedirectWrite => {
                    self.next();
                    redirects.push(Redirection::Write(self.expect_target()?));
                }
                Token::RedirectAppend => {
                    self.next();
                    redirects.push(Redirection::Append(self.expect_target()?));
                }
                Token::RedirectRead => {
                    self.next();
                    redirects.push(Redirection::Append(self.expect_target()?));
                }
                Token::Pipe | Token::Ampersand => break,
            }
        }

        if args.is_empty() {
            return Err("Syntax error: expected command name".to_string());
        }
        Ok(Command { args, redirects })
    }

    fn expect_target(&mut self) -> Result<String, String> {
        match self.next() {
            Some(Token::Word(file)) => Ok(file),
            _ => Err("Syntax error: expected file path after redirection operator".to_string()),
        }
    }
}
