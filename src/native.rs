use {
    std::{
        collections::HashMap,
        convert::TryInto as _,
        time::Instant,
    },
    gc::Gc,
    once_cell::sync::Lazy,
    crate::value::Value,
};

macro_rules! register {
    ($($f:ident,)*) => {
        static FUNCTIONS: Lazy<Vec<NativeFn>> = Lazy::new(|| {
            let functions = vec![$($f as NativeFn)*];
            assert!(functions.len() < 256);
            functions
        });

        pub(crate) fn all() -> HashMap<Gc<String>, Gc<Value>> {
            let mut map = HashMap::default();
            $(
                map.insert(Gc::new(stringify!($f).to_owned()), Value::new($f as NativeFn));
            )*
            map
        }

        pub(crate) fn deserialize(fn_id: u8) -> Option<NativeFn> {
            FUNCTIONS.get(usize::from(fn_id)).copied()
        }

        pub(crate) fn serialize(f: NativeFn) -> u8 {
            FUNCTIONS.iter()
                .position(|&iter_fn| iter_fn as usize == f as usize)
                .expect("tried to serialize unregistered native function")
                .try_into()
                .expect("more than u8::MAX native functions")
        }
    };
}

static EPOCH: Lazy<Instant> = Lazy::new(|| Instant::now());

pub(crate) type NativeFn = fn(&[Gc<Value>]) -> Gc<Value>;

fn clock(_: &[Gc<Value>]) -> Gc<Value> {
    Value::new(EPOCH.elapsed().as_secs_f64())
}

register! {
    clock,
}
