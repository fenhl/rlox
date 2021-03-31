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
    Nil,
    Bool(bool),
    Closure(Gc<Closure>),
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
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(true) => write!(f, "true"),
            Value::Bool(false) => write!(f, "false"),
            Value::Closure(closure) => closure.fmt(f),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, rhs: &Value) -> bool {
        match (self, rhs) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(lhs), Value::Bool(rhs)) => lhs == rhs,
            (Value::Closure(lhs), Value::Closure(rhs)) => Gc::ptr_eq(lhs, rhs),
            //TODO other cases (numbers, strings depending on interning, other kinds of objects)
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
