use {
    std::{
        convert::TryInto as _,
        fmt,
        io::{
            self,
            prelude::*,
        },
    },
    byteorder::{
        LittleEndian,
        ReadBytesExt as _,
        WriteBytesExt as _,
    },
    derive_more::From,
    gc::{
        Finalize,
        Gc,
        GcCell,
        Trace,
    },
    crate::{
        error::{
            Error,
            Result,
        },
        vm::OpCode,
    },
};

#[derive(From, Trace, Finalize)]
pub(crate) enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    Closure(Gc<Closure>),
    Function(Function),
    NativeFn(NativeFn),
    String(Gc<String>),
}

impl Value {
    pub(crate) fn new(value: impl Into<Value>) -> Gc<Value> { Gc::new(value.into()) }
    pub(crate) fn nil() -> Gc<Value> { Gc::new(Value::Nil) }

    pub(crate) fn as_bool(&self) -> bool {
        match *self {
            Value::Nil => false,
            Value::Bool(b) => b,
            _ => true,
        }
    }

    pub(crate) fn as_function(&self) -> Option<Function> { if let Value::Function(f) = self { Some(f.clone()) } else { None } }
    pub(crate) fn as_number(&self) -> Option<f64> { if let Value::Number(n) = *self { Some(n) } else { None } }
    pub(crate) fn as_string(&self) -> Option<Gc<String>> { if let Value::String(s) = self { Some(s.clone()) } else { None } }

    fn read(stream: &mut impl Read) -> Result<Value> {
        Ok(match stream.read_u8()? {
            0 => Value::Nil,
            1 => Value::Bool(false),
            2 => Value::Bool(true),
            3 => Value::Number(stream.read_f64::<LittleEndian>()?),
            4 => Value::Closure(Gc::new(Closure::read(stream)?)),
            5 => Value::Function(FunctionInner::read(stream, false)?.wrap()),
            6 => Value::NativeFn(NativeFn { inner: crate::native::deserialize(stream.read_u8()?).ok_or_else(|| Error::Decode("NativeFn"))? }),
            7 => {
                let len = stream.read_u64::<LittleEndian>()?.try_into().map_err(|_| Error::Decode("String"))?;
                let mut buf = Vec::with_capacity(len);
                stream.read_exact(&mut buf)?;
                Value::String(Gc::new(String::from_utf8(buf).map_err(|_| Error::Decode("String"))?))
            }
            _ => return Err(Error::Decode("Value")),
        })
    }

    fn write(&self, sink: &mut impl Write) -> io::Result<()> {
        match self {
            Value::Nil => sink.write_u8(0)?,
            Value::Bool(false) => sink.write_u8(1)?,
            Value::Bool(true) => sink.write_u8(2)?,
            Value::Number(n) => {
                sink.write_u8(3)?;
                sink.write_f64::<LittleEndian>(*n)?;
            }
            Value::Closure(closure) => {
                sink.write_u8(4)?;
                closure.write(sink)?;
            }
            Value::Function(function) => {
                sink.write_u8(5)?;
                function.borrow().write(sink)?;
            }
            Value::NativeFn(NativeFn { inner }) => {
                sink.write_u8(6)?;
                sink.write_u8(crate::native::serialize(*inner))?;
            }
            Value::String(s) => {
                sink.write_u8(7)?;
                sink.write_u64::<LittleEndian>(s.len().try_into().expect("string is longer than u64::MAX bytes"))?;
                sink.write_all(s.as_bytes())?;
            }
        }
        Ok(())
    }
}

impl From<crate::native::NativeFn> for Value {
    fn from(inner: crate::native::NativeFn) -> Value {
        Value::NativeFn(NativeFn { inner })
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::String(Gc::new(s))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(true) => write!(f, "true"),
            Value::Bool(false) => write!(f, "false"),
            Value::Number(n) => n.fmt(f),
            Value::Closure(closure) => closure.fmt(f),
            Value::Function(function) => function.borrow().fmt(f),
            Value::NativeFn(_) => write!(f, "<native fn>"),
            Value::String(s) => s.fmt(f),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, rhs: &Value) -> bool {
        match (self, rhs) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(lhs), Value::Bool(rhs)) => lhs == rhs,
            (Value::Number(lhs), Value::Number(rhs)) => lhs == rhs,
            (Value::Closure(lhs), Value::Closure(rhs)) => Gc::ptr_eq(lhs, rhs),
            (Value::String(lhs), Value::String(rhs)) => lhs == rhs, //TODO adjust for interning
            //TODO other kinds of objects
            (_, _) => false, // values of different types are never equal
        }
    }
}

#[derive(Trace, Finalize)]
pub(crate) struct Closure {
    pub(crate) function: Function,
}

impl Closure {
    pub(crate) fn new(function: Function) -> Gc<Closure> {
        Gc::new(Closure { function })
    }

    fn read(stream: &mut impl Read) -> Result<Closure> {
        Ok(Closure {
            function: FunctionInner::read(stream, false)?.wrap(),
        })
    }

    fn write(&self, sink: &mut impl Write) -> io::Result<()> {
        let Closure { function } = self;
        function.borrow().write(sink)?;
        Ok(())
    }
}

impl fmt::Display for Closure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.function.borrow().fmt(f)
    }
}

#[derive(Default, Trace, Finalize)]
pub(crate) struct FunctionInner {
    pub(crate) arity: u8,
    pub(crate) chunk: Vec<u8>,
    pub(crate) lines: Vec<u32>,
    pub(crate) constants: Vec<Gc<Value>>,
    pub(crate) name: Option<Gc<String>>,
}

impl FunctionInner {
    pub(crate) fn wrap(self) -> Function {
        Gc::new(GcCell::new(self))
    }

    pub(crate) fn add_code(&mut self, line: u32, code: u8) {
        self.lines.push(line);
        self.chunk.push(code);
    }

    pub(crate) fn add_constant(&mut self, value: Gc<Value>) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub(crate) fn read(stream: &mut impl Read, is_script: bool) -> Result<FunctionInner> {
        Ok(FunctionInner {
            name: if is_script { None } else {
                let len = stream.read_u64::<LittleEndian>()?.try_into().map_err(|_| Error::Decode("String"))?;
                let mut buf = Vec::with_capacity(len);
                stream.read_exact(&mut buf)?;
                Some(Gc::new(String::from_utf8(buf).map_err(|_| Error::Decode("String"))?))
            },
            arity: if is_script { 0 } else { stream.read_u8()? },
            constants: {
                let len = stream.read_u8()?.into();
                let mut constants = Vec::with_capacity(len);
                for _ in 0..len {
                    constants.push(Gc::new(Value::read(stream)?));
                }
                constants
            },
            chunk: {
                let len = stream.read_u64::<LittleEndian>()?.try_into().map_err(|_| Error::Decode("Chunk"))?;
                let mut chunk = vec![0; len];
                stream.read_exact(&mut chunk)?;
                //TODO validate chunk for safety (and maybe offer an unsafe more where none of the other parts of a .rlox file are validated either)
                chunk
            },
            lines: {
                let mut lines = Vec::default();
                loop {
                    let run_len = stream.read_u8()?;
                    if run_len == 0 { break }
                    let line = stream.read_u32::<LittleEndian>()?;
                    lines.resize(lines.len() + usize::from(run_len), line);
                }
                lines
            },
        })
    }

    pub(crate) fn write(&self, sink: &mut impl Write) -> io::Result<()> {
        let FunctionInner { name, arity, chunk, lines, constants } = self;
        if let Some(name) = name {
            sink.write_u64::<LittleEndian>(name.len().try_into().expect("function name is longer than u64::MAX bytes"))?;
            sink.write_all(name.as_bytes())?;
            sink.write_u8(*arity)?;
        } else {
            sink.write_u8(0xc0)?; // magic byte to distinguish rlox bytecode from Lox source code
            assert_eq!(*arity, 0);
        }
        sink.write_u8(constants.len().try_into().expect("more than u8::MAX constants"))?;
        for constant in constants {
            constant.write(sink)?;
        }
        sink.write_u64::<LittleEndian>(chunk.len().try_into().expect("bytecode is longer than u64::MAX bytes"))?;
        sink.write_all(&chunk)?;
        let mut lines = lines.iter().peekable();
        while let Some(&line) = lines.next() {
            let mut run_len = 1;
            while run_len < u8::MAX && lines.peek().map_or(false, |&&next_line| next_line == line) {
                run_len += 1;
                let _ = lines.next();
            }
            sink.write_u8(run_len)?;
            sink.write_u32::<LittleEndian>(line)?;
        }
        sink.write_u8(0)?; // end of lines
        Ok(())
    }

    pub(crate) fn disassemble(&self) {
        println!("== {} ==", self);
        let mut rest = &*self.chunk;
        while !rest.is_empty() {
            OpCode::disassemble(&mut rest, &self.constants);
        }
    }
}

impl fmt::Display for FunctionInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref name) = self.name {
            name.fmt(f)
        } else {
            write!(f, "<script>")
        }
    }
}

pub(crate) type Function = Gc<GcCell<FunctionInner>>;

#[derive(Trace, Finalize)]
pub(crate) struct NativeFn {
    #[unsafe_ignore_trace]
    pub(crate) inner: crate::native::NativeFn,
}
