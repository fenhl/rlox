use {
    std::{
        fmt,
        io,
    },
    derive_more::From,
    lalrpop_util::{
        ParseError,
        lexer::Token,
    },
};

pub(crate) struct OwnedToken(usize, String);

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
pub(crate) enum Error {
    #[from]
    Io(io::Error),
    Parse(ParseError<usize, OwnedToken, String>),
    Runtime(String),
}

impl<'input> From<ParseError<usize, Token<'input>, &'input str>> for Error {
    fn from(e: ParseError<usize, Token<'input>, &str>) -> Error {
        Error::Parse(e.map_token(OwnedToken::from).map_error(ToOwned::to_owned))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Parse(e) => write!(f, "parse error: {}", e),
            Error::Runtime(msg) => write!(f, "runtime error: {}", msg),
        }
    }
}

pub(crate) type Result<T = (), E = Error> = std::result::Result<T, E>;
