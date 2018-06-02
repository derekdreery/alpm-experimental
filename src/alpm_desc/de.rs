//! Serde deserializer for alpm desc format
//!
//! format is
//! ```text
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
use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor};

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

impl<'de> DeserializerInner<'de> {
    /// Returns the next element in a sequence
    fn parse_seq_element(&mut self) -> &'de str {
        match self.input.find(self.line_ending) {
            Some(newline_pos) => {
                let value = &self.input[..newline_pos];
                self.input = &self.input[newline_pos + self.line_ending.len()..];
                value
            },
            None => {
                let value = &self.input[..];
                self.input = &self.input[self.input.len()..];
                value
            }
        }
    }
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

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.allow_list {
            visitor.visit_seq(AlpmSeq::new(&mut self))
        } else {
            Err(ErrorKind::Unsupported.into())
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
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
        self.deserialize_seq(visitor)
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
    {
        self.input.parse().map_err(|_| ErrorKind::ExpectedUnsigned.into())
    }

    /// Parse a signed int
    fn parse_signed<T>(&mut self) -> Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: fmt::Debug,
    {
        self.input.parse().map_err(|_| ErrorKind::ExpectedSigned.into())
    }

    /// Parse a float
    ///
    /// exponential notation is not currently supported
    fn parse_float<T>(&mut self) -> Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: ::std::error::Error
    {
        self.input.parse().map_err(|_| ErrorKind::ExpectedFloat.into())
    }
}

struct AlpmSeq<'a, 'de: 'a> {
    de: &'a mut DeserializerInner<'de>,
}

impl<'a, 'de> AlpmSeq<'a, 'de> {
    fn new(de: &'a mut DeserializerInner<'de>) -> Self {
        AlpmSeq { de }
    }
}

impl<'a, 'de> SeqAccess<'de> for AlpmSeq<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where
            T: DeserializeSeed<'de>,
    {
        let element = self.de.parse_seq_element();
        Ok(if element.len() == 0 {
            None
        } else {
            Some(seed.deserialize(DeserializerInner {
                input: element,
                allow_list: false,
                line_ending: self.de.line_ending,
            })?)
        })
    }
}

/// These have to be in a separate module to avoid a name clash for `ErrorKind`
mod nom_parsers {
    use nom::{ErrorKind, IResult};

    /*
    /// We need our own is_digit, because nom's version works on u8, not char
    fn is_digit(chr: char) -> bool {
        match chr {
            '0'..='9' => true,
            _ => false,
        }
    }

    named!(pub parse_unsigned(&str) -> &str, recognize!(
        take_while1!(call!(is_digit))
    ));

    named!(pub parse_signed(&str) -> &str, recognize!(do_parse!(
        opt!(alt!(tag!("+") | tag!("-"))) >>
        take_while1!(call!(is_digit)) >>
        (())
    )));

    named!(pub parse_float(&str) -> &str, recognize!(
        do_parse!(
            opt!(alt!(tag!("+") | tag!("-"))) >>
            take_while!(call!(is_digit)) >>
            opt!(tag!(".")) >>
            take_while!(call!(is_digit)) >>
            (())
        )
    ));
    */

    pub fn parse_key<'a>(input: &'a str, line_ending: &'static str) -> IResult<&'a str, &'a str> {
        do_parse!(input,
            tag!("%") >>
            name: take_till1!(|ch| ch == '%') >>
            tag!("%") >>
            tag!(line_ending) >>
            (name)
        )
    }

    /*
    #[test]
    fn test_is_digit() {
        for positive in ['0', '1', '2', '3', '8', '9'].iter() {
            assert!(is_digit(*positive));
        }
        for negative in ['a', '.', '$', '😄'].iter() {
            assert!(!is_digit(*negative));
        }
    }

    #[test]
    fn test_parse_unsigned() {
        assert_eq!(parse_unsigned("60 sef"), Ok((" sef", "60")));
    }

    #[test]
    fn test_parse_signed() {
        assert_eq!(parse_signed("60 sef"), Ok((" sef", "60")));
    }

    #[test]
    fn test_parse_float() {
        assert_eq!(parse_float("60. sef"), Ok((" sef", "60.")));
    }
    */

    #[test]
    fn test_parse_key() {
        assert_eq!(parse_key("%NAME%\n sef", "\n"), Ok((" sef", "NAME")));
        assert_eq!(parse_key("%NAME%\r\n sef", "\r\n"), Ok((" sef", "NAME")));
    }
}
