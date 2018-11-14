use crate::value::{Value, Visitable};
use std::fmt;
pub type VisitResult = Result<(), Error>;

pub struct Error {
    // TODO
}

/// An object-safe streaming visitor.
pub trait Visit {
    /// Visit an unsigned integer value.
    ///
    /// This defaults to calling `self.visit_any()`; implementations wishing to
    /// provide behaviour specific to unsigned integers may override the default
    /// implementation.
    fn visit_uint(&mut self, value: u64) -> VisitResult {
        self.visit_any(&value)
    }

    /// Visit a signed integer value.
    ///
    /// This defaults to calling `self.visit_any()`; implementations wishing to
    /// provide behaviour specific to signed integers may override the default
    /// implementation.
    fn visit_int(&mut self, value: i64) -> VisitResult {
        self.visit_any(&value)
    }

    /// Visit a floating-point value.
    ///
    /// This defaults to calling `self.visit_any()`; implementations wishing to
    /// provide behaviour specific to floating-point values may override the
    /// default implementation.
    fn visit_float(&mut self, value: f64) -> VisitResult {
        self.visit_any(&value)
    }

    /// Visit a string value.
    ///
    /// This defaults to calling `self.visit_any()`; implementations wishing to
    /// provide behaviour specific to strings may override the default
    /// implementation.
    fn visit_str(&mut self, value: &str) -> VisitResult {
        self.visit_any(&value)
    }

    /// Visit an unsigned 8-bit value.
    ///
    /// This defaults to calling `self.visit_any()`; implementations wishing to
    /// provide behaviour specific to byte values may override the default
    /// implementation.
    fn visit_byte(&mut self, value: u8) -> VisitResult {
        self.visit_any(&value)
    }

    /// Visit a boolean value.
    ///
    /// This defaults to calling `self.visit_any()`; implementations wishing to
    /// provide behaviour specific to booleans may override the default
    /// implementation.
    fn visit_bool(&mut self, value: bool) -> VisitResult {
        self.visit_any(&value)
    }

    /// Visit an arbitrarily-typed value.
    fn visit_any(&mut self, value: &dyn Visitable) -> VisitResult;

    /// Visit a key-value association.
    ///
    /// The key and the value are both known to implement `Value`.
    fn visit_kv(&mut self, k: Value, v: Value) -> VisitResult;

    /// Visit an arbitrary set of pre-compiled format arguments.
    fn visit_fmt(&mut self, args: fmt::Arguments) -> VisitResult;

    /// Indicates that the next visited value is a named type with the name
    /// `name`.
    ///
    /// This is called prior to visiting structs, tuple structs, and enum
    /// variants.
    fn named_type(&mut self, name: &str) -> VisitResult;

    /// Begin visiting a key-value map.
    ///
    /// After this function has returned `Ok(())`, the `Visit` may expect
    /// that all subsequent calls will be to `visit_kv` (representing the
    /// key-value pairs in the map) until `close_map` is called.
    ///
    /// The visitor should perform any internal state transitions necessary to
    /// visit a map.
    fn open_map(&mut self) -> VisitResult;

    /// Finish visiting a map.
    ///
    /// When this is called, the visitor should expect calls to arbitrary
    /// `visit` methods.
    fn close_map(&mut self) -> VisitResult;

    /// Begin visiting an ordered list of values.
    ///
    /// When this function has returned `Ok(())` any subsequent calls to
    /// `visit` represent the elements of the list, until `close_list` is
    /// called.
    ///
    /// The visitor should perform any internal state transitions necessary to
    /// visit a list (for example, begin serializing comma-delimited values).
    fn open_list(&mut self) -> VisitResult;

    /// Finish visiting a list.
    fn close_list(&mut self) -> VisitResult;

    /// Begin visiting a `struct`.
    ///
    /// After this function has returned `Ok(())`, the `Visit` may expect
    /// that all subsequent calls will be to `visit_kv` (representing the
    /// field names and values of the struct) until `close_struct` is called.
    ///
    /// The visitor should perform any internal state transitions necessary to
    /// visit a struct.
    fn open_struct(&mut self) -> VisitResult;

    /// Finish visiting a `struct`.
    fn close_struct(&mut self) -> VisitResult;

    /// Begin visiting a tuple.
    ///
    ///  When this function has returned `Ok(())` any subsequent calls to
    /// `visit` represent the elements of the tuple, until `close_tuple` is
    /// called.
    ///
    /// The visitor should perform any internal state transitions necessary to
    /// visit a tuple.
    fn open_tuple(&mut self) -> VisitResult;

    /// Finish visiting a `struct`.
    fn close_tuple(&mut self) -> VisitResult;
}

impl<'v> dyn Visit + 'v {
    /// Visit a map of key-value data.
    ///
    /// This function manages calling `open_map`, visiting the key-value
    /// data in the given iterator, and closing the map.
    ///
    /// This is the suggested way for `Value` implementations to visit maps,
    /// rather than calling those functions directly, unless different behaviour
    /// is needed.
    pub fn visit_map<'a, I>(&mut self, i: I) -> VisitResult
    where
        I: IntoIterator<Item = (Value<'a>, Value<'a>)>,
    {
        self.open_map()?;
        for (k, v) in i {
            self.visit_kv(k, v)?;
        }
        self.close_map()
    }

    /// Visit an ordered list of `Value`s.
    ///
    /// This function manages calling `open_list`, visiting the list elements
    /// in the given iterator, and closing the list.
    ///
    /// This is the suggested way for `Value` implementations to visit lists,
    /// rather than calling those functions directly, unless different behaviour
    /// is needed.
    pub fn visit_list<'a, I>(&mut self, i: I) -> VisitResult
    where
        I: IntoIterator<Item = Value<'a>>,
    {
        self.open_list()?;
        for v in i {
            v.visit(self)?;
        }
        self.close_list()
    }

    /// Visit a `struct` of `Value`s, given the struct's `name` and an
    /// iterator over its `fields`.
    ///
    /// This function manages calling `open_struct`, visiting the struct's
    /// fields, and closing the struct.
    ///
    /// This is the suggested way for `Value` implementations to visit structs,
    /// rather than calling those functions directly, unless different behaviour
    /// is needed.
    pub fn visit_struct<'a, I>(&mut self, name: &str, fields: I) -> VisitResult
    where
        I: IntoIterator<Item = (&'a str, Value<'a>)>,
    {
        self.named_type(name);
        self.open_struct()?;
        for (name, v) in fields {
            self.visit_kv(Value::borrowed(&name), v)?;
        }
        self.close_struct()
    }

    /// Visit a tuple.
    ///
    /// This function manages calling `open_tuple`, visiting the tuple's
    /// fields, and closing the tuple.
    ///
    /// This is the suggested way for `Value` implementations to visit tuples,
    /// rather than calling those functions directly, unless different behaviour
    /// is needed.
    pub fn visit_tuple<'a, I>(&mut self, i: I) -> VisitResult
    where
        I: IntoIterator<Item = Value<'a>>,
    {
        self.open_tuple()?;
        for v in i {
            v.visit(self)?;
        }
        self.close_tuple()
    }


    /// Visit a tuple `struct` of `Value`s, given the struct's `name` and an
    /// iterator over its `fields`.
    ///
    /// This function manages calling `open_struct`, visiting the struct's
    /// fields, and closing the struct.
    ///
    /// This is the suggested way for `Value` implementations to visit structs,
    /// rather than calling those functions directly, unless different behaviour
    /// is needed.
    pub fn visit_tuple_struct<'a, I>(&mut self, name: &str, fields: I) -> VisitResult
    where
        I: IntoIterator<Item = Value<'a>>,
    {
        self.named_type(name);
        self.open_tuple()?;
        for v in fields {
            v.visit(self)?;
        }
        self.close_tuple()
    }
}
