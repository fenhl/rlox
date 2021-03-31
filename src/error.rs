use {
    std::{
        fmt,
        io,
        num::ParseFloatError,
    },
    derive_more::From,
    lalrpop_util::{
        ParseError,
        lexer::Token,
    },
};

pub struct OwnedToken(usize, String);

impl fmt::Display for OwnedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.1.fmt(f)
    }
}

impl<'input> From<Token<'input>> for OwnedToken {
    fn from(Token(location, lexeme): Token<'_>) -> OwnedToken {
        OwnedToken(location, lexeme.to_owned())
    }
}

#[derive(From)]
pub enum Error {
    Compile(String),
    #[from]
    Io(io::Error),
    Parse(ParseError<usize, OwnedToken, Box<Error>>),
    #[from]
    ParseFloat(ParseFloatError),
    Runtime(String),
}

impl<'input> From<ParseError<usize, Token<'input>, Error>> for Error {
    fn from(e: ParseError<usize, Token<'input>, Error>) -> Error {
        Error::Parse(e.map_token(OwnedToken::from).map_error(Box::new))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Compile(msg) => write!(f, "compile error: {}", msg),
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Parse(e) => write!(f, "parse error: {}", e),
            Error::ParseFloat(e) => e.fmt(f),
            Error::Runtime(msg) => write!(f, "runtime error: {}", msg),
        }
    }
}

pub(crate) type Result<T = (), E = Error> = std::result::Result<T, E>;
