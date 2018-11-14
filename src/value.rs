use crate::visitor::{Visit, VisitResult};
use std::{
    borrow::Borrow,
    collections,
    fmt,
    hash::Hash,
};

/// A structured field value of an erased type.
///
/// Implementors of `Value` may call the appropriate typed recording methods on
/// the `Record` passed to `Record` in order to indicate how their data
/// should be recorded.
pub trait Visitable: Send {
    /// Visits the value with the given `Visit`.
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult;
}


pub struct Value<'a> {
    inner: ValueKind<'a>,
}

enum ValueKind<'a> {
    Borrowed(&'a dyn Visitable),
    Owned(Box<dyn Visitable + 'a>),
    Display(&'a (dyn fmt::Display + Sync)),
    Debug(&'a (dyn fmt::Debug + Sync)),
}

impl<'a> Value<'a> {
    pub fn display(value: &'a (impl fmt::Display + Sync)) -> Self {
        Value {
            inner: ValueKind::Display(value),
        }
    }

    pub fn debug<T>(value: &'a T) -> Self
    where
        T: fmt::Debug + Sync + 'a,
    {
        Value {
            inner: ValueKind::Debug(value),
        }
    }

    pub fn borrowed(value: &'a impl Visitable) -> Self {
        Value {
            inner: ValueKind::Borrowed(value)
        }
    }

    pub fn owned<V, B>(value: &B) -> Self
    where
        B: ToOwned<Owned = V>,
        V: Visitable + Borrow<B> + 'a,
    {
        Value {
            inner: ValueKind::Owned(Box::new(value.to_owned()))
        }
    }

    pub fn with_visit<T, F>(value: T, visit: F) -> Self
    where
        T: Send + 'a,
        F: Fn(&T, &mut dyn Visit) -> VisitResult + Send + 'a,
    {
        struct WithVisit<T, F> {
            visit: F,
            value: T,
        }

        impl<T, F> Visitable for WithVisit<T, F>
        where
            T: Send,
            F: Fn(&T, &mut dyn Visit) -> VisitResult + Send,
        {
            fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
                (self.visit)(&self.value, visitor)
            }
        }

        let with_visit = WithVisit {
            value,
            visit,
        };
        Value {
            inner: ValueKind::Owned(Box::new(with_visit))
        }
    }

    pub fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        match self.inner {
            ValueKind::Borrowed(ref v) => v.visit(visitor),
            ValueKind::Owned(ref v) => v.as_ref().visit(visitor),
            ValueKind::Display(ref v) => visitor.visit_fmt(format_args!("{}", v)),
            ValueKind::Debug(ref v) => visitor.visit_fmt(format_args!("{:?}", v)),
        }
    }
}

macro_rules! impl_values {
    ( $( $visit:ident( $( $whatever:tt)+ ) ),+ ) => {
        $(
            impl_value!{ $visit( $( $whatever )+ ) }
        )+
    }
}
macro_rules! impl_value {
    ( $visit:ident( $( $value_ty:ty ),+ ) ) => {
        $(
            impl Visitable for $value_ty {
                fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
                    visitor.$visit(*self)
                }
            }
        )+
    };
    ( $visit:ident( $( $value_ty:ty ),+ as $as_ty:ty) ) => {
        $(
            impl Visitable for $value_ty {
                fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
                    visitor.$visit(*self as $as_ty)
                }
            }
        )+
    };
}

impl_values! {
    visit_byte(u8),
    visit_uint(u64),
    visit_uint(usize, u32, u16 as u64),
    visit_int(i64),
    visit_int(isize, i32, i16, i8 as i64),
    visit_float(f64, f32 as f64),
    visit_bool(bool)
}

impl<'a> Visitable for &'a str {
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        visitor.visit_str(self)
    }
}

impl<T> Visitable for [T]
where
    T: Visitable,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        visitor.visit_list(self.iter().map(Value::borrowed))
    }
}

impl<T> Visitable for Vec<T>
where
    T: Visitable,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        self.as_slice().visit(visitor)
    }
}

impl<K, V> Visitable for collections::HashMap<K, V>
where
    K: Visitable + Hash + Eq,
    V: Visitable,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        visitor.visit_map(self.iter().map(|(k, v)| (Value::borrowed(k), Value::borrowed(v))))
    }
}

impl<T> Visitable for collections::HashSet<T>
where
    T: Visitable + Hash + Eq,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        visitor.visit_tuple(self.iter().map(Value::borrowed))
    }
}

impl<K, V> Visitable for collections::BTreeMap<K, V>
where
    K: Visitable + Eq,
    V: Visitable,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        visitor.visit_map(self.iter().map(|(k, v)| (Value::borrowed(k), Value::borrowed(v))))
    }
}

impl<T> Visitable for collections::BTreeSet<T>
where
    T: Visitable + Eq,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        visitor.visit_tuple(self.iter().map(Value::borrowed))
    }
}

impl<T> Visitable for collections::LinkedList<T>
where
    T: Visitable,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        visitor.visit_list(self.iter().map(Value::borrowed))
    }
}

impl<T> Visitable for collections::VecDeque<T>
where
    T: Visitable,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        visitor.visit_list(self.iter().map(Value::borrowed))
    }
}

impl<T> Visitable for collections::BinaryHeap<T>
where
    T: Visitable + Ord,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        // NOTE: the values will *not* be visited in order --- is that something
        // we want to guarantee?
        visitor.visit_list(self.iter().map(Value::borrowed))
    }
}

impl<'a, T> Visitable for &'a T
where
    T: Visitable + Sync + 'a,
{
    fn visit(&self, visitor: &mut dyn Visit) -> VisitResult {
        (*self).visit(visitor)
    }
}
