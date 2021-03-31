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
    crate::error::{
        Error,
        Result,
    },
};

#[derive(From, Trace, Finalize)]
pub(crate) enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    Closure(Gc<Closure>),
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

    pub(crate) fn as_number(&self) -> Option<f64> { if let Value::Number(n) = *self { Some(n) } else { None } }
    pub(crate) fn as_string(&self) -> Option<Gc<String>> { if let Value::String(s) = self { Some(s.clone()) } else { None } }

    fn read(stream: &mut impl Read) -> Result<Value> {
        Ok(match stream.read_u8()? {
            0 => Value::Nil,
            1 => Value::Bool(false),
            2 => Value::Bool(true),
            3 => Value::Number(stream.read_f64::<LittleEndian>()?),
            4 => Value::Closure(Gc::new(Closure::read(stream)?)),
            5 => {
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
            Value::String(s) => {
                sink.write_u8(5)?;
                sink.write_u64::<LittleEndian>(s.len().try_into().expect("string is longer than u64::MAX bytes"))?;
                sink.write_all(s.as_bytes())?;
            }
        }
        Ok(())
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
    pub(crate) constants: Vec<Gc<Value>>,
}

impl FunctionInner {
    pub(crate) fn wrap(self) -> Function {
        Gc::new(GcCell::new(self))
    }

    pub(crate) fn add_constant(&mut self, value: Gc<Value>) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub(crate) fn read(stream: &mut impl Read, is_script: bool) -> Result<FunctionInner> {
        Ok(FunctionInner {
            arity: if is_script { 0 } else { unimplemented!(/*TODO*/) },
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
        })
    }

    pub(crate) fn write(&self, sink: &mut impl Write) -> io::Result<()> {
        let FunctionInner { arity, chunk, constants } = self;
        sink.write_u8(0xc0)?; // magic byte to distinguish rlox bytecode from Lox source code
        assert_eq!(*arity, 0); //TODO write arity if function name exists
        sink.write_u8(constants.len().try_into().expect("more than u8::MAX constants"))?;
        for constant in constants {
            constant.write(sink)?;
        }
        sink.write_u64::<LittleEndian>(chunk.len().try_into().expect("bytecode is longer than u64::MAX bytes"))?;
        sink.write_all(&chunk)?;
        Ok(())
    }
}

impl fmt::Display for FunctionInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<script>") //TODO print function name if any
    }
}

pub(crate) type Function = Gc<GcCell<FunctionInner>>;
