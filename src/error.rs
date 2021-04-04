use {
    std::{
        fmt,
        io,
        num::ParseFloatError,
        string::FromUtf8Error,
    },
    derive_more::From,
    lalrpop_util::ParseError,
    crate::{
        lexer::Token,
        vm::CallFrame,
    },
};

#[derive(From)]
pub enum Error {
    Compile {
        msg: String,
        line: u32,
    },
    CompileRepl,
    Decode(&'static str),
    #[from]
    Io(io::Error),
    Parse(ParseError<u32, Token, Box<Error>>),
    #[from]
    ParseFloat(ParseFloatError),
    Runtime {
        msg: String,
        call_stack: Vec<CallFrame>,
    },
    #[from]
    Utf8(FromUtf8Error),
}

impl From<ParseError<u32, Token, Error>> for Error {
    fn from(e: ParseError<u32, Token, Error>) -> Error {
        Error::Parse(e.map_error(Box::new))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Compile { msg, line } => write!(f, "[line {}] Error: {}", line, msg),
            Error::CompileRepl => write!(f, "invoking with --compile requires an input script"),
            Error::Decode(ty) => write!(f, "invalid {} in bytecode", ty),
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Parse(ParseError::User { error }) => error.fmt(f),
            Error::Parse(e) => write!(f, "parse error: {}", e),
            Error::ParseFloat(e) => e.fmt(f),
            Error::Runtime { msg, call_stack } => {
                writeln!(f, "{}", msg)?;
                for frame in call_stack.into_iter().rev() {
                    write!(f, "[line {}] in ", frame.closure.function.borrow().lines[frame.ip - 1])?;
                    if let Some(ref name) = frame.closure.function.borrow().name {
                        writeln!(f, "{}()", name)?;
                    } else {
                        writeln!(f, "script")?;
                    }
                }
                Ok(())
            }
            Error::Utf8(e) => write!(f, "error reading string: {}", e),
        }
    }
}

impl wheel::CustomExit for Error {
    fn exit(self, _: &'static str) -> ! {
        eprintln!("{}", self);
        std::process::exit(match self {
            Error::Compile { .. } | Error::Parse(_) => 65,
            Error::Runtime { .. } => 70,
            Error::Io(_) => 74,
            _ => 1,
        })
    }
}

pub(crate) type Result<T = (), E = Error> = std::result::Result<T, E>;
