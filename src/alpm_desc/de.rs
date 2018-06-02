//! Serde deserializer for alpm desc format
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

pub use super::de_error::{Error, ErrorKind, Result};

use itertools::Itertools;
use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, Visitor};

use std::fmt;
use std::str::FromStr;

pub const NEWLINE_CRLF: &str = "\r\n";
pub const NEWLINE_LF: &str = "\n";

pub struct Deserializer<'de> {
    input: &'de str,
    line_ending: &'static str,
    double_line_ending: &'static str,
}

impl<'de> Deserializer<'de> {
    /// Create a deserializer from a str.
    #[cfg(windows)]
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            input,
            line_ending: NEWLINE_CRLF,
            double_line_ending: "\r\n\r\n", // concat! doesn't work
        }
    }

    /// Create a deserializer from a str.
    #[cfg(not(windows))]
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            input,
            line_ending: NEWLINE_LF,
            double_line_ending: "\n\n",
        }
    }

    /// Like from_str, but with a custom line ending.
    pub fn from_str_line_ending(
        input: &'de str,
        line_ending: &'static str,
        double_line_ending: &'static str,
    ) -> Self {
        Deserializer {
            input,
            line_ending,
            double_line_ending,
        }
    }

    /// Returns the next key, and consumes it.
    fn parse_key(&mut self) -> Result<&'de str> {
        match nom_parsers::parse_key(self.input, self.line_ending) {
            Ok((rest, name)) => {
                self.input = rest;
                Ok(name)
            }
            Err(_) => Err(ErrorKind::ExpectedKey.into()),
        }
    }

    /// Returns the next value, consuming it and the delimiter.
    fn parse_value(&mut self) -> Result<&'de str> {
        match self.split_next_double_newline() {
            (ref line, Some(ref rest)) => {
                self.input = rest;
                Ok(line)
            }
            (ref all, None) => {
                self.input = &self.input[self.input.len()..];
                Ok(all)
            }
        }
    }

    /// Returns all the input up to the next newline
    ///
    /// Returns `(<current line without newline>, Some(<everything after the newline>))` if a
    /// newline str was found, `(<everything>, None)` otherwise.
    fn split_next_double_newline(&self) -> (&'de str, Option<&'de str>) {
        match self.input.find(self.double_line_ending) {
            Some(newline_pos) => (
                &self.input[..newline_pos],
                Some(&self.input[newline_pos + self.double_line_ending.len()..]),
            ),
            None => (&self.input, None),
        }
    }
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(AlpmMap::new(&mut self, &[]))
    }

    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // We don't support structs where two fields differ only by case.
        for (idx, item) in fields.iter().enumerate() {
            for item2 in fields.iter().skip(idx + 1) {
                if item.eq_ignore_ascii_case(item2) {
                    return Err(ErrorKind::Unsupported.into());
                }
            }
        }
        visitor.visit_map(AlpmMap::new(&mut self, fields))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // can only deserialize structures at root
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct AlpmMap<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    fields: &'static [&'static str],
}

impl<'a, 'de> AlpmMap<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, fields: &'static [&'static str]) -> Self {
        AlpmMap { de, fields }
    }
}

impl<'a, 'de> MapAccess<'de> for AlpmMap<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // if we're at the end of input we're done
        if self.de.input.trim().len() == 0 {
            return Ok(None);
        }
        // if there is a struct field that matches case-insensitively, use that instead.
        let real_key = self.de.parse_key()?;
        let mut key = &real_key;
        for field in self.fields {
            if field.eq_ignore_ascii_case(key) {
                key = &field;
                break;
            }
        }
        seed.deserialize(DeserializerInner {
            input: &key,
            allow_list: false,
            line_ending: self.de.line_ending,
        }).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self.de.parse_value()?;
        seed.deserialize(DeserializerInner {
            input: value,
            allow_list: true,
            line_ending: self.de.line_ending,
        })
    }
}

pub struct DeserializerInner<'de> {
    input: &'de str,
    allow_list: bool,
    line_ending: &'static str,
}

impl<'de> de::Deserializer<'de> for DeserializerInner<'de> {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // we cannot guess the type
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_signed()?)
    }

    fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_signed()?)
    }

    fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_signed()?)
    }

    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_signed()?)
    }

    fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_unsigned()?)
    }

    fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_unsigned()?)
    }

    fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_unsigned()?)
    }

    fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parse_float()?)
    }

    fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.parse_float()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let ch = self.input.chars().next().ok_or(Error::from(ErrorKind::ExpectedChar))?;
        visitor.visit_char(ch)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.input)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.input.as_bytes())
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.is_empty() {
            visitor.visit_unit()
        } else {
            Err(ErrorKind::ExpectedEmpty.into())
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.allow_list {
            unimplemented!()
        } else {
            Err(ErrorKind::Unsupported.into())
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.allow_list {
            unimplemented!()
        } else {
            Err(ErrorKind::Unsupported.into())
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.allow_list {
            unimplemented!()
        } else {
            Err(ErrorKind::Unsupported.into())
        }
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // only unkeyed lists are supported
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // only unkeyed lists are supported
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Enums are not supported
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}
/*
impl<'de> DeserializerValueOrList<'de> {

/// Check and consume the next newline, and the one after if we're not in a list
///
/// If we get to eof before reading all the newlines, then just return an empty str.
fn check_next_newline(&self, input: &str) -> Result<&str> {
    let line_str = if self.win_line_endings {
        if self.allow_list {
            NEWLINE_CRLF
        } else {
            concat!(NEWLINE_CRLF, NEWLINE_CRLF)
        }
    } else {
        if self.allow_list {
            NEWLINE_LF
        } else {
            concat!(NEWLINE_LF, NEWLINE_LF)
        }
    };
    // we check each character is what we expect, but we allow it to be missing (if we got to
    // the end of the input)
    for (expected, actual) in line_str.chars().zip(self.input.chars()) {
        if expected != actual {
            return Err(ErrorKind::UnexpectedInput.into());
        }
    }
    let len = ::std::cmp::min(self.input.len(), line_str.len());
    Ok(&input[len..])
}

#[inline]
fn newline(&self) -> &'static str {
    if self.win_line_endings {
        NEWLINE_CRLF
    } else {
        NEWLINE_LF
    }
}

/// Returns all the input up to the next newline
///
/// Returns `(<current line without newline>, Some(<everything after the newline>))` if a
/// newline str was found, `(<everything>, None)` otherwise.
fn split_next_newline(&self) -> (&str, Option<&str>) {
    let newline = self.newline();
    match self.input.find(newline) {
        Some(newline_pos) =>
            (&self.input[.. newline_pos], Some(&self.input[newline_pos + newline.len() ..]))
        None => (&self.input, None)
    }
}
}
*/
/*
impl<'de> de::Deserializer<'de> for DeserializerValueOrList<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        // we need to know the types to deserialize
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let (val, input) = if input.starts_with("true") {
            (true, &self.input["true".len()..])
        } else if input.starts_with("false") {
            (false, &self.input["false".len()..])
        } else {
            return Err(ErrorKind::UnexpectedInput.into())
        };
        let input = self.check_next_newline(input)?;
        self.input = input;
        visitor.visit_bool(val)
    }

    fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u8(self.parse_unsigned()?)
    }

    fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u32(self.parse_unsigned()?)
    }

    fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u64(self.parse_unsigned()?)
    }

    fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i8(self.parse_signed()?)
    }

    fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i16(self.parse_signed()?)
    }

    fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i32(self.parse_signed()?)
    }

    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i64(self.parse_signed()?)
    }

    fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_f32(self.parse_float()?)
    }

    fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_f64(self.parse_float()?)
    }

    fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let ch = match self.input.get(0).ok_or(ErrorKind::UnexpectedInput.into())? {
            '\n' | '\r' => return Err(ErrorKind::UnexpectedInput.into()),
            ch => ch
        };
        let input = &self.input[ch.len_utf8()..];
        let input = self.check_next_newline(input)?;
        self.input = input;
        visitor.visit_char(ch)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match self.input.split_next_newline() {
            (ref line, Some(ref rest)) => {
                let input = self.input[string.len()..];
                let input = self.check_next_newline(input)?;
                self.input = input;
                visitor.visit_borrowed_str(line)
            },
            (ref line, None) => {
                self.input = &self.input[self.input.len()..];
                visitor.visit_borrowed_str(line)
            }
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>
    {
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        Err(ErrorKind::Unsupported.into())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.input.len() == 0 {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.input.len() == 0 {
            visitor.visit_unit()
        } else {
            Err(ErrorKind::UnexpectedInput.into())
        }
    }

    fn deserialize_unit_struct<V>(self,
                                  _name: &'static str,
                                  visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self,
                                  _name: &'static str,
                                  visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_newtype_struct(self)
    }
}
*/
/// Trait for shared parsing functionality for simple types
impl<'de> DeserializerInner<'de> {
    /// Parse a boolean
    fn parse_bool(&mut self) -> Result<bool> {
        let input = self.input;
        if input.starts_with("true") {
            self.input = &input["true".len()..];
            Ok(true)
        } else if input.starts_with("false") {
            self.input = &input["false".len()..];
            Ok(false)
        } else {
            Err(ErrorKind::ExpectedBool.into())
        }
    }

    /// Parse an unsigned int
    fn parse_unsigned<T>(&mut self) -> Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: fmt::Debug,
    {
        match nom_parsers::parse_unsigned(self.input) {
            Ok((next_input, recognised)) => {
                self.input = next_input;
                Ok(recognised
                    .parse()
                    .expect("internal error: recognised but failed parse"))
            }
            Err(_) => Err(ErrorKind::ExpectedUnsigned.into()),
        }
    }

    /// Parse a signed int
    fn parse_signed<T>(&mut self) -> Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: fmt::Debug,
    {
        match nom_parsers::parse_signed(self.input) {
            Ok((next_input, recognised)) => {
                self.input = next_input;
                Ok(recognised
                    .parse()
                    .expect("internal error: recognised but failed parse"))
            }
            Err(_) => Err(ErrorKind::ExpectedSigned.into()),
        }
    }

    /// Parse a float
    ///
    /// exponential notation is not currently supported
    fn parse_float<T>(&mut self) -> Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: ::std::error::Error
    {
        match nom_parsers::parse_float(self.input) {
            Ok((next_input, unparsed)) => {
                // unlike previously we can see that some recognised strings will not parse
                let parsed = unparsed.parse::<T>()
                    .map_err(|_| Error::from(ErrorKind::ExpectedFloat))?;
                self.input = next_input;
                Ok(parsed)
            }
            Err(_) => Err(ErrorKind::ExpectedFloat.into()),
        }
    }
}

/// These have to be in a separate module to avoid a name clash for `ErrorKind`
mod nom_parsers {
    use nom::{ErrorKind, IResult};

    /// We need our own is_digit, because nom's version works on u8, not char
    fn is_digit(chr: char) -> bool {
        match chr {
            '0'..='9' => true,
            _ => false,
        }
    }

    named!(pub parse_float(&str) -> &str, recognize!(
        do_parse!(
            opt!(alt!(tag!("+") | tag!("-"))) >>
            take_till!(call!(is_digit)) >>
            opt!(tag!(".")) >>
            take_till!(call!(is_digit)) >>
            (())
        )
    ));

    named!(pub parse_unsigned(&str) -> &str,
           recognize!(take_till1!(call!(is_digit))));

    named!(pub parse_signed(&str) -> &str, recognize!(do_parse!(
        opt!(alt!(tag!("+") | tag!("-"))) >>
        take_till1!(call!(is_digit)) >>
        (())
    )));

    pub fn parse_key<'a>(input: &'a str, line_ending: &'static str) -> IResult<&'a str, &'a str> {
        do_parse!(input,
            tag!("%") >>
            name: take_till1!(|ch| ch == '%') >>
            tag!("%") >>
            tag!(line_ending) >>
            (name)
        )
    }
}
