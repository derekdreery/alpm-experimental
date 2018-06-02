//! Serde serializer for alpm desc format
//!
//! format is
//! ```
//! %name%
//! value
//!
//! %name2%
//! value2
//!
//! ...
//! ```
use std::io::Write;

use serde::ser::{self, Serialize};

pub use super::ser_error::{Error, ErrorKind, Result};

/// The serializer for alpm database format.
#[derive(Debug)]
pub struct Serializer<W: Write> {
    /// The writer we will serialize to.
    writer: W,
}

/// Serialize the given value to a string in the alpm db format.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut output: Vec<u8> = Vec::new();
    {
        let mut serializer = Serializer {
            writer: &mut output,
        };
        value.serialize(&mut serializer)?;
    }
    // Our format is all valid utf8 - so we could probably use _unchecked, but safety first :)
    Ok(String::from_utf8(output).unwrap())
}

/// Serialize the given value to the given writer in the alpm db format.
pub fn to_writer<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    let mut serializer = Serializer { writer };
    value.serialize(&mut serializer)
}

/// A serializer for values or lists.
#[derive(Debug)]
struct SerializerValueOrList<'a, W: 'a>
where
    W: Write,
{
    /// The writer we will serialize to.
    inner: &'a mut Serializer<W>,
    /// Whether to allow lists
    in_list: bool,
}

/// A serializer for keys.
///
/// Just supports plain data types.
#[derive(Debug)]
struct SerializerKey<'a, W: 'a>
where
    W: Write,
{
    /// The writer we will serialize to.
    inner: &'a mut Serializer<W>,
}

impl<'a, W> ser::Serializer for &'a mut Serializer<W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    // only keyed maps are supported at root
    fn serialize_bool(self, _v: bool) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_i8(self, _v: i8) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_i16(self, _v: i16) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_i32(self, _v: i32) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_i64(self, _v: i64) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_u8(self, _v: u8) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_u16(self, _v: u16) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_u32(self, _v: u32) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_u64(self, _v: u64) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_char(self, _v: char) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_str(self, _v: &str) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_none(self) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        Err(ErrorKind::Unsupported.into())
    }

    fn serialize_unit(self) -> Result<()> {
        // todo is there any point in this impl
        write!(self.writer, "%%\n\n")?;
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        // todo is there any point in this impl
        write!(self.writer, "%{}%\n\n", name.to_uppercase())?;
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        write!(self.writer, "%{}%\n{}\n\n", name.to_uppercase(), variant)?;
        Ok(())
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // just pass through to inner type
        value.serialize(self)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(ErrorKind::Unsupported.into())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self> {
        // we ignore the struct's name and serialize the field names only
        Ok(self)
    }

    // only keyed maps are supported at root
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(ErrorKind::Unsupported.into())
    }
}

impl<'a, W> ser::SerializeMap for &'a mut Serializer<W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(SerializerKey { inner: self })?;
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(SerializerValueOrList {
            inner: self,
            in_list: false,
        })?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W> ser::SerializeStruct for &'a mut Serializer<W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        write!(self.writer, "%{}%\n", key.to_uppercase())?;
        value.serialize(SerializerValueOrList {
            inner: self,
            in_list: false,
        })?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::Serializer for SerializerValueOrList<'a, W> {
    // it's our job to put the blank line at the end
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    // none of the following are valid
    // they could be `!` (never type) once that is stable.
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = ser::Impossible<(), Error>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_i64(self, v: i64) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_u8(self, v: u8) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_u16(self, v: u16) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_u32(self, v: u32) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_u64(self, v: u64) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_f32(self, v: f32) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_f64(self, v: f64) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_char(self, v: char) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_str(self, v: &str) -> Result<()> {
        write!(self.inner.writer, "{}\n", v)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // We don't support binary data
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // serialize nothing
    fn serialize_none(self) -> Result<()> {
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    // pass through to inner serializer
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self)?;
        Ok(())
    }

    // serialize nothing
    fn serialize_unit(self) -> Result<()> {
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        write!(self.inner.writer, "{}\n", name)?;
        if !self.in_list {
            write!(self.inner.writer, "\n")?;
        }
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // just pass through to inner type
        value.serialize(self)?;
        Ok(())
    }

    // We cannot know we have the correct variant so we cannot support
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // defer to our seq impl
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        if self.in_list {
            Err(ErrorKind::Unsupported.into())
        } else {
            Ok(self)
        }
    }

    // defer to our tuple impl
    fn serialize_tuple(self, _len: usize) -> Result<Self> {
        if self.in_list {
            Err(ErrorKind::Unsupported.into())
        } else {
            Ok(self)
        }
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        if self.in_list {
            Err(ErrorKind::Unsupported.into())
        } else {
            Ok(self)
        }
    }

    // We cannot know we have the correct variant so we cannot support
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(ErrorKind::Unsupported.into())
    }

    // We cannot know which key so we cannot support
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(ErrorKind::Unsupported.into())
    }

    // for now don't try to serialize, relies on field order not changing
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(ErrorKind::Unsupported.into())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(ErrorKind::Unsupported.into())
    }
}

impl<'a, W> ser::SerializeSeq for SerializerValueOrList<'a, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(SerializerValueOrList {
            inner: &mut self.inner,
            in_list: true,
        })?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        write!(self.inner.writer, "\n")?;
        Ok(())
    }
}

impl<'a, W> ser::SerializeTuple for SerializerValueOrList<'a, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(SerializerValueOrList {
            inner: &mut self.inner,
            in_list: true,
        })?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        write!(self.inner.writer, "\n")?;
        Ok(())
    }
}

impl<'a, W> ser::SerializeTupleStruct for SerializerValueOrList<'a, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(SerializerValueOrList {
            inner: &mut self.inner,
            in_list: true,
        })?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        write!(self.inner.writer, "\n")?;
        Ok(())
    }
}

impl<'a, W> ser::Serializer for SerializerKey<'a, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    // none of these are valid keys, they are all unreachable
    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = ser::Impossible<(), Error>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    // only keyed maps are supported at root
    fn serialize_bool(self, v: bool) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_i8(self, v: i8) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_i16(self, v: i16) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_i32(self, v: i32) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_i64(self, v: i64) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_u8(self, v: u8) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_u16(self, v: u16) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_u32(self, v: u32) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_u64(self, v: u64) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_f32(self, v: f32) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_f64(self, v: f64) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_char(self, v: char) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_str(self, v: &str) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", v.to_uppercase())?;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        unimplemented!();
        write!(self.inner.writer, "%")?;
        for byte in v {
            write!(self.inner.writer, "{:x}", byte)?;
        }
        write!(self.inner.writer, "%\n")?;
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        write!(self.inner.writer, "%%\n")?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self)?;
        Ok(())
    }

    fn serialize_unit(self) -> Result<()> {
        write!(self.inner.writer, "%%\n")?;
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        write!(self.inner.writer, "%{}%\n", name.to_uppercase())?;
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // just pass through to inner type
        value.serialize(self)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(ErrorKind::Unsupported.into())
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(ErrorKind::Unsupported.into())
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        // we ignore the struct's name and serialize the field names only
        Err(ErrorKind::Unsupported.into())
    }

    // only keyed maps are supported at root
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(ErrorKind::Unsupported.into())
    }
}
