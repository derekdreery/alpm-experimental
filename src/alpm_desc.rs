//! Serde (de)serializers for alpm desc format
//!
//! format is
//! ```
//! %name%
//! value
//!
//! ```
use std::io::{self, Write};
use std::result::Result as StdResult;
use std::fmt::{self, Display};

use serde::ser::{self, Serialize};

/// Errors that can occur during (de)serialization.
#[derive(Debug)]
pub enum Error {
    /// Some i/o error occurred.
    Io(io::Error),
    /// This format does not support the given operation
    Unsupported,
    /// A Serialize method returned a custom error.
    Custom(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::Io(ref io_err) =>
                write!(f, "an i/o error of kind {:?} occured", io_err.kind()),
            &Error::Unsupported =>
                write!(f, "serialization of this field is unsupported in this context"),
            &Error::Custom(ref msg) =>
                write!(f, "custom serialize error occured: {}", msg),
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        "unimplemented - use display instead"
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match self {
            &Error::Io(ref err) => Some(err),
            _ => None
        }
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
        where T: Display
    {
        Error::Custom(format!("{}", msg))
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

pub type Result<T> = StdResult<T, Error>;

/// The root level serializer.
#[derive(Debug)]
pub struct Serializer<W: Write> {
    /// The writer we will serialize to.
    writer: W,
}

/// A serializer for values.
#[derive(Debug)]
struct SerializerValue<'a, W: 'a>
where W: Write
{
    writer: &'a W
}

impl<'a, W: Write> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    // only keyed maps are supported at root
    fn serialize_bool(self, v: bool) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_i8(self, v: i8) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_i16(self, v: i16) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_i32(self, v: i32) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_i64(self, v: i64) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_u8(self, v: u8) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_u16(self, v: u16) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_u32(self, v: u32) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_u64(self, v: u64) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_f32(self, v: f32) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_f64(self, v: f64) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_char(self, v: char) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_str(self, v: &str) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_none(self) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<()>
    where T: Serialize
    {
        Err(Error::Unsupported)
    }

    fn serialize_unit(self) -> Result<()> {
        write!(self.writer, "%%\n\n")?;
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        write!(self.writer, "%{}%\n\n", name.to_uppercase())?;
        Ok(())
    }

    fn serialize_unit_variant(self,
                              name: &'static str,
                              variant_index: u32,
                              variant: &'static str) -> Result<()> {
        write!(self.writer, "%{}%\n{}\n\n", name.to_uppercase(), variant)?;
        Ok(())
    }

    fn serialize_newtype_struct<T>(self,
                                           name: &'static str,
                                           value: &T) -> Result<()>
    where T: ?Sized + Serialize
    {
        write!(self.writer, "%{}%\n", name.to_uppercase())?;
        value.serialize(SerializerValue { writer: &mut self.writer })?;
        Ok(())
    }

    fn serialize_newtype_variant<T: ?Sized>(self,
                                 name: &'static str,
                                 variant_index: u32,
                                 variant: &'static str,
                                 value: &T) -> Result<()> {
        Err(Error::Unsupported)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_tuple_variant(self,
                               name: &'static str,
                               variant_index: u32,
                               variant: &'static str,
                               len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_struct(self,
                        name: &'static str,
                        len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_struct_variant(self,
                                name: &'static str,
                                variant_index: u32,
                                variant: &'static str,
                                len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeSeq for &'a mut Serializer<W>
where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeTuple for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeTupleStruct for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeTupleVariant for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeMap for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeStruct for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeStructVariant for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}


impl<'a, W: Write> ser::Serializer for SerializerValue<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = ();
    type SerializeTupleVariant = ();
    type SerializeMap = ();
    type SerializeStruct = ();
    type SerializeStructVariant = ();

    // only keyed maps are supported at root
    fn serialize_bool(self, v: bool) -> Result<()> {
        write!(self.writer, "{}", v)?;
        Ok(())
    }

    // only keyed maps are supported at root
    fn serialize_i8(self, v: i8) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_i16(self, v: i16) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_i32(self, v: i32) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_i64(self, v: i64) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_u8(self, v: u8) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_u16(self, v: u16) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_u32(self, v: u32) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_u64(self, v: u64) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_f32(self, v: f32) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_f64(self, v: f64) -> Result<()> {
        write!(self.writer, "{}", v)?;
    }

    // only keyed maps are supported at root
    fn serialize_char(self, v: char) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_str(self, v: &str) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_none(self) -> Result<()> {
        Err(Error::Unsupported)
    }

    // only keyed maps are supported at root
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<()>
        where T: Serialize
    {
        Err(Error::Unsupported)
    }

    fn serialize_unit(self) -> Result<()> {
        write!(self.writer, "%%\n\n")?;
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        write!(self.writer, "%{}%\n\n", name.to_uppercase())?;
        Ok(())
    }

    fn serialize_unit_variant(self,
                              name: &'static str,
                              variant_index: u32,
                              variant: &'static str) -> Result<()> {
        write!(self.writer, "%{}%\n{}\n\n", name.to_uppercase(), variant)?;
        Ok(())
    }

    fn serialize_newtype_struct<T>(self,
                                   name: &'static str,
                                   value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        write!(self.writer, "%{}%\n", name.to_uppercase())?;
        value.serialize(SerializerValue { writer: &mut self.writer })?;
        Ok(())
    }

    fn serialize_newtype_variant<T: ?Sized>(self,
                                            name: &'static str,
                                            variant_index: u32,
                                            variant: &'static str,
                                            value: &T) -> Result<()> {
        Err(Error::Unsupported)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_tuple_variant(self,
                               name: &'static str,
                               variant_index: u32,
                               variant: &'static str,
                               len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_struct(self,
                        name: &'static str,
                        len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }

    fn serialize_struct_variant(self,
                                name: &'static str,
                                variant_index: u32,
                                variant: &'static str,
                                len: usize) -> Result<Self> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeSeq for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeTuple for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeTupleStruct for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeTupleVariant for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeMap for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeStruct for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}

impl<'a, W> ser::SerializeStructVariant for &'a mut Serializer<W>
    where W: Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
        where T: ?Sized
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported)
    }
}
