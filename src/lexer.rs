use {
    std::{
        fmt,
        io::{
            self,
            prelude::*,
        },
    },
    byteorder::ReadBytesExt as _,
    crate::error::{
        Error,
        Result,
    },
};
pub(crate) use self::Token::*;

pub(crate) struct Lexer<'a> {
    peek: Option<u8>,
    stream: Option<Box<dyn Read + 'a>>,
    line: u32,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(stream: Box<dyn Read + 'a>) -> Lexer {
        Lexer {
            peek: None,
            stream: Some(stream),
            line: 1,
        }
    }

    fn next_byte(&mut self) -> Result<Option<u8>> {
        if let Some(c) = self.peek.take() {
            Ok(Some(c))
        } else if let Some(ref mut stream) = self.stream {
            match stream.read_u8() {
                Ok(c) => Ok(Some(c)),
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    self.stream = None;
                    Ok(None)
                }
                Err(e) => Err(e.into()),
            }
        } else {
            Ok(None)
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<(u32, Token, u32)>;

    fn next(&mut self) -> Option<Result<(u32, Token, u32)>> {
        let byte = loop {
            let byte = match self.next_byte() {
                Ok(Some(byte)) => byte,
                Ok(None) => return None,
                Err(e) => return Some(Err(e)),
            };
            match byte {
                b' ' | b'\r' | b'\t' => {}
                b'\n' => self.line += 1,
                b'/' => match self.next_byte() {
                    Ok(Some(b'/')) => loop {
                        match self.next_byte() {
                            Ok(Some(b'\n')) => {
                                self.line += 1;
                                break
                            }
                            Ok(Some(_)) => {}
                            Ok(None) => return None,
                            Err(e) => return Some(Err(e)),
                        }
                    },
                    Ok(Some(next_byte)) => {
                        self.peek = Some(next_byte);
                        break byte
                    }
                    Ok(None) => break byte,
                    Err(e) => return Some(Err(e)),
                },
                byte => break byte,
            }
        };
        let starting_line = self.line;
        let token = match byte {
            b'A'..=b'Z' | b'_' | b'a'..=b'z' => {
                let mut ident = vec![byte];
                loop {
                    match self.next_byte() {
                        Ok(Some(next_byte @ b'0'..=b'9')) | Ok(Some(next_byte @ b'A'..=b'Z')) | Ok(Some(next_byte @ b'_')) | Ok(Some(next_byte @ b'a'..=b'z')) => ident.push(next_byte),
                        Ok(Some(next_byte)) => {
                            self.peek = Some(next_byte);
                            break
                        }
                        Ok(None) => break,
                        Err(e) => return Some(Err(e)),
                    }
                }
                let ident = unsafe { String::from_utf8_unchecked(ident) }; //SAFETY: bytes are ASCII
                match &*ident {
                    "and" => AND(starting_line),
                    "class" => CLASS(starting_line),
                    "else" => ELSE(starting_line),
                    "false" => FALSE(starting_line),
                    "for" => FOR(starting_line),
                    "fun" => FUN(starting_line),
                    "if" => IF(starting_line),
                    "nil" => NIL(starting_line),
                    "or" => OR(starting_line),
                    "print" => PRINT(starting_line),
                    "return" => RETURN(starting_line),
                    "super" => SUPER(starting_line),
                    "this" => THIS(starting_line),
                    "true" => TRUE(starting_line),
                    "var" => VAR(starting_line),
                    "while" => WHILE(starting_line),
                    _ => IDENTIFIER((starting_line, ident)),
                }
            }
            b'0'..=b'9' => {
                let mut period_found = false;
                let mut number = vec![byte];
                loop {
                    match self.next_byte() {
                        Ok(Some(next_byte @ b'0'..=b'9')) => number.push(next_byte),
                        Ok(Some(b'.')) if !period_found => {
                            //TODO only consider part of the literal if followed by a digit; requires 2-byte lookahead
                            period_found = true;
                            number.push(b'.');
                        }
                        Ok(Some(next_byte)) => {
                            self.peek = Some(next_byte);
                            break
                        }
                        Ok(None) => break,
                        Err(e) => return Some(Err(e)),
                    }
                }
                let number = unsafe { String::from_utf8_unchecked(number) }; //SAFETY: bytes are ASCII
                match number.parse() {
                    Ok(n) => NUMBER((starting_line, n)),
                    Err(e) => return Some(Err(e.into())),
                }
            }
            b'(' => LEFT_PAREN(starting_line),
            b')' => RIGHT_PAREN(starting_line),
            b'{' => LEFT_BRACE(starting_line),
            b'}' => RIGHT_BRACE(starting_line),
            b';' => SEMICOLON(starting_line),
            b',' => COMMA(starting_line),
            b'.' => DOT(starting_line),
            b'-' => MINUS(starting_line),
            b'+' => PLUS(starting_line),
            b'/' => SLASH(starting_line),
            b'*' => STAR(starting_line),
            b'!' => match self.next_byte() {
                Ok(Some(b'=')) => BANG_EQUAL(starting_line),
                Ok(Some(next_byte)) => {
                    self.peek = Some(next_byte);
                    BANG(starting_line)
                }
                Ok(None) => BANG(starting_line),
                Err(e) => return Some(Err(e)),
            },
            b'=' => match self.next_byte() {
                Ok(Some(b'=')) => EQUAL_EQUAL(starting_line),
                Ok(Some(next_byte)) => {
                    self.peek = Some(next_byte);
                    EQUAL(starting_line)
                }
                Ok(None) => EQUAL(starting_line),
                Err(e) => return Some(Err(e)),
            },
            b'<' => match self.next_byte() {
                Ok(Some(b'=')) => LESS_EQUAL(starting_line),
                Ok(Some(next_byte)) => {
                    self.peek = Some(next_byte);
                    LESS(starting_line)
                }
                Ok(None) => LESS(starting_line),
                Err(e) => return Some(Err(e)),
            },
            b'>' => match self.next_byte() {
                Ok(Some(b'=')) => GREATER_EQUAL(starting_line),
                Ok(Some(next_byte)) => {
                    self.peek = Some(next_byte);
                    GREATER(starting_line)
                }
                Ok(None) => GREATER(starting_line),
                Err(e) => return Some(Err(e)),
            },
            b'"' => {
                let mut buf = Vec::default();
                loop {
                    match self.next_byte() {
                        Ok(Some(b'\n')) => {
                            self.line += 1;
                            buf.push(b'\n');
                        }
                        Ok(Some(b'"')) => break,
                        Ok(Some(byte)) => buf.push(byte),
                        Ok(None) => return Some(Err(Error::Compile(format!("Unterminated string.")))),
                        Err(e) => return Some(Err(e)),
                    }
                }
                match String::from_utf8(buf) {
                    Ok(s) => STRING((starting_line, self.line, s)),
                    Err(e) => return Some(Err(e.into())),
                }
            }
            _ => return Some(Err(Error::Compile(format!("Unexpected character.")))),
        };
        Some(Ok((starting_line, token, self.line)))
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub enum Token {
    AND(u32),
    CLASS(u32),
    ELSE(u32),
    FALSE(u32),
    FOR(u32),
    FUN(u32),
    IF(u32),
    NIL(u32),
    OR(u32),
    PRINT(u32),
    RETURN(u32),
    SUPER(u32),
    THIS(u32),
    TRUE(u32),
    VAR(u32),
    WHILE(u32),
    IDENTIFIER((u32, String)),
    NUMBER((u32, f64)),
    LEFT_PAREN(u32),
    RIGHT_PAREN(u32),
    LEFT_BRACE(u32),
    RIGHT_BRACE(u32),
    SEMICOLON(u32),
    COMMA(u32),
    DOT(u32),
    MINUS(u32),
    PLUS(u32),
    SLASH(u32),
    STAR(u32),
    BANG_EQUAL(u32),
    BANG(u32),
    EQUAL_EQUAL(u32),
    EQUAL(u32),
    LESS_EQUAL(u32),
    LESS(u32),
    GREATER_EQUAL(u32),
    GREATER(u32),
    STRING((u32, u32, String)),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AND(_) => write!(f, "and"),
            CLASS(_) => write!(f, "class"),
            ELSE(_) => write!(f, "else"),
            FALSE(_) => write!(f, "false"),
            FOR(_) => write!(f, "for"),
            FUN(_) => write!(f, "fun"),
            IF(_) => write!(f, "if"),
            NIL(_) => write!(f, "nil"),
            OR(_) => write!(f, "or"),
            PRINT(_) => write!(f, "print"),
            RETURN(_) => write!(f, "return"),
            SUPER(_) => write!(f, "super"),
            THIS(_) => write!(f, "this"),
            TRUE(_) => write!(f, "true"),
            VAR(_) => write!(f, "var"),
            WHILE(_) => write!(f, "while"),
            IDENTIFIER((_, name)) => name.fmt(f),
            NUMBER((_, n)) => n.fmt(f),
            LEFT_PAREN(_) => write!(f, "("),
            RIGHT_PAREN(_) => write!(f, ")"),
            LEFT_BRACE(_) => write!(f, "{{"),
            RIGHT_BRACE(_) => write!(f, "}}"),
            SEMICOLON(_) => write!(f, ";"),
            COMMA(_) => write!(f, ","),
            DOT(_) => write!(f, "."),
            MINUS(_) => write!(f, "-"),
            PLUS(_) => write!(f, "+"),
            SLASH(_) => write!(f, "/"),
            STAR(_) => write!(f, "*"),
            BANG_EQUAL(_) => write!(f, "!="),
            BANG(_) => write!(f, "!"),
            EQUAL_EQUAL(_) => write!(f, "=="),
            EQUAL(_) => write!(f, "="),
            LESS_EQUAL(_) => write!(f, "<="),
            LESS(_) => write!(f, "<"),
            GREATER_EQUAL(_) => write!(f, ">="),
            GREATER(_) => write!(f, ">"),
            STRING((_, _, s)) => write!(f, "\"{}\"", s),
        }
    }
}
