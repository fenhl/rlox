use {
    std::fmt,
    derive_more::From,
    gc::{
        Finalize,
        Gc,
        GcCell,
        Trace,
    },
};

#[derive(From, Trace, Finalize)]
pub(crate) enum Value {
    Closure(Gc<Closure>),
    Nil,
}

impl Value {
    pub(crate) fn new(value: impl Into<Value>) -> Gc<Value> { Gc::new(value.into()) }
    pub(crate) fn nil() -> Gc<Value> { Gc::new(Value::Nil) }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Closure(closure) => closure.fmt(f),
            Value::Nil => write!(f, "nil"),
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
}

impl FunctionInner {
    pub(crate) fn wrap(self) -> Function {
        Gc::new(GcCell::new(self))
    }
}

impl fmt::Display for FunctionInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<script>") //TODO print function name if any
    }
}

pub(crate) type Function = Gc<GcCell<FunctionInner>>;
