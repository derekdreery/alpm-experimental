//! Module to hold logic for parsing and comparing versions.
use itertools::Itertools;
use std::{
    borrow::Cow,
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    hash::{Hash, Hasher},
};

const DEFAULT_EPOCH: &'static str = "0";

/// An alpm version.
///
/// It has a custom Ord impl to match version ordering. See the tests to get a feel for how it
/// works.
#[derive(Debug, Clone)]
pub struct Version<'a> {
    /// The epoch (optional, defaults to "0")
    pub epoch: Cow<'a, str>,
    /// The version
    pub version: Cow<'a, str>,
    /// The release (optional)
    pub release: Option<Cow<'a, str>>,
}

impl<'a> Version<'a> {
    /// Helper function to create version
    fn new(epoch: &'a str, version: &'a str, release: Option<&'a str>) -> Self {
        Self {
            epoch: Cow::Borrowed(epoch),
            version: Cow::Borrowed(version),
            release: release.map(Cow::Borrowed),
        }
    }

    /// Parse a string into a version.
    // Match wierd algorithms in alpm.
    pub fn parse(input: &str) -> Version {
        let mut input_minus_epoch = input;
        // there is an epoch if the version begins with `digit* ':'`
        let mut input_iter = input.char_indices();
        let mut epoch_end: Option<usize> = None;
        // Search for epoch
        loop {
            match input_iter.next() {
                // There was an epoch
                Some((idx, ch)) if ch == ':' => {
                    epoch_end = Some(idx);
                    // remove the epoch
                    input_minus_epoch = &input[idx + 1..];
                    break;
                }
                // There might be an epoch (we're still in the first set of digits)
                Some((idx, ch)) if ch.is_ascii_digit() => continue,
                // There is no epoch
                Some((_, _)) | None => break,
            }
        }
        let release_separator_idx = input_minus_epoch.rfind('-');

        Version::new(
            match epoch_end {
                Some(idx) => &input[..idx],
                None => DEFAULT_EPOCH,
            },
            match release_separator_idx {
                Some(idx) => &input_minus_epoch[..idx],
                // Whole thing if there is no release
                None => input_minus_epoch,
            },
            release_separator_idx.map(|idx| &input_minus_epoch[idx + 1..]),
        )
    }

    pub fn into_owned(self) -> Version<'static> {
        Version {
            epoch: Cow::Owned(self.epoch.into_owned()),
            version: Cow::Owned(self.version.into_owned()),
            release: self.release.map(|release| Cow::Owned(release.into_owned())),
        }
    }

    /// Checks for byte equality, you can use this to see if the version is the same, but written
    /// differently.
    pub fn byte_eq(&self, other: &Self) -> bool {
        self.epoch == other.epoch && self.version == other.version && self.release == other.release
    }
}

impl PartialOrd for Version<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        use self::Ordering::Equal;
        match version_cmp(&self.epoch, &other.epoch) {
            Equal => (),
            gtlt => return gtlt,
        };
        match version_cmp(&self.version, &other.version) {
            Ordering::Equal => (),
            gtlt => return gtlt,
        };
        // If either is missing just evaluate to equal - this matches alpm.
        match (&self.release, &other.release) {
            (Some(left), Some(right)) => version_cmp(left, right),
            _ => Ordering::Equal,
        }
    }
}

impl PartialEq for Version<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version<'_> {}

impl Hash for Version<'_> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        version_hash(&self.epoch, hasher);
        version_hash(&self.version, hasher);
        if let Some(ref release) = self.release {
            version_hash(release, hasher);
        }
    }
}

/// Part of the version string
enum Block<'a> {
    /// A number of non-alphanumeric characters (length)
    Separator(usize),
    /// A block of alpha characters
    Alpha(&'a [u8]),
    /// A block of numeric characters (leading zeros have been removed).
    Numeric(&'a [u8]),
}

/// An iterator over input that yields `Block`s.
struct BlocksIter<'a> {
    rest: &'a [u8],
}

impl<'a> BlocksIter<'a> {
    fn new(input: &'a [u8]) -> Self {
        BlocksIter { rest: input }
    }
}

/// Just to avoid repeating myself
macro_rules! get_block {
    ($input:expr, $test:expr) => {{
        let mut end_idx = 1;
        loop {
            match $input.get(end_idx) {
                Some(ch) if $test(ch) => end_idx += 1,
                _ => break,
            }
        }
        let result = &$input[..end_idx];
        $input = &$input[end_idx..];
        result
    }};
}

impl<'a> Iterator for BlocksIter<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.rest.get(0) {
            // Next block is numeric
            Some(ch) if ch.is_ascii_digit() => {
                let block = get_block!(self.rest, u8::is_ascii_digit);
                Some(Block::Numeric(discard_zeros(block)))
            }
            // Next block is alpha
            Some(ch) if ch.is_ascii_alphabetic() => {
                let block = get_block!(self.rest, u8::is_ascii_digit);
                Some(Block::Alpha(block))
            }
            // Next block is non-alphanumeric
            Some(ch) => {
                let block = get_block!(self.rest, |ch: &u8| !ch.is_ascii_alphanumeric());
                Some(Block::Separator(block.len()))
            }
            // We've reached the end of the input
            None => None,
        }
    }
}

/// Find which version section is newer, or if they are equal
///
///  - First, split the input up into blocks of *alpha*, *digit* or *non-alphanum*.
///  - For each pair of blocks
///     - If the types are different, then *non-alphanum* is newer than *numeric*,
///       which is newer than *alpha*
///     - If the types are the same, then the rules are
///       - For *non-alphanum*, compare lengths, longer is newer, equal lengths are equal segments
///         (so *--* and *::* are the same)
///       - For *alpha* just do a lexicographic comparison (so *b* is newer than *a* etc.)
///       - For *numeric*, do a numeric comparison. (this can be done by skipping leading zeros,
///         then comparing length, then comparing lexicographically, to avoid overflow on integer
///         conversion)
///   - If one input is longer than the other, and all sections so far have been equal, then if
///     the next section of the longer is *alpha*, it is older, and if it is *numeric* it is newer.
///     (so "1a" and "1-a" are older than "1", "a1" and "a-1" are newer than "a").
///   - If the inputs have the same number of sections that are all equal, or one input has some
///     extra separator at the end, then they are equal.
fn version_cmp(left: &str, right: &str) -> Ordering {
    use self::{
        Block::{Alpha, Numeric, Separator},
        Ordering::{Equal, Greater, Less},
    };
    use itertools::EitherOrBoth::{Both, Left, Right};
    let mut blocks_iter = BlocksIter::new(left.as_bytes())
        .zip_longest(BlocksIter::new(right.as_bytes()))
        .peekable();
    while let Some(pair) = blocks_iter.next() {
        match pair {
            // left is newer
            Both(Separator(_), Numeric(_))
            | Both(Separator(_), Alpha(_))
            | Both(Numeric(_), Alpha(_))
            | Left(Numeric(_))
            | Right(Alpha(_)) => return Ordering::Greater,
            // right is newer
            Both(Numeric(_), Separator(_))
            | Both(Alpha(_), Separator(_))
            | Both(Alpha(_), Numeric(_))
            | Left(Alpha(_))
            | Right(Numeric(_)) => return Ordering::Less,
            // Find the next non-segment or the end
            Left(Separator(_)) | Right(Separator(_)) => continue,
            // We have a match on segment type
            Both(Numeric(left), Numeric(right)) => {
                // equal means continue, otherwise return
                match left.len().cmp(&right.len()) {
                    Equal => match left.cmp(right) {
                        Equal => continue,
                        gtlt => return gtlt,
                    },
                    gtlt => return gtlt,
                }
            }
            Both(Alpha(left), Alpha(right)) => match left.cmp(right) {
                Equal => continue,
                gtlt => return gtlt,
            },
            Both(Separator(left_len), Separator(right_len)) => {
                // If we're at the end it's different to when we're in the middle.
                if blocks_iter.peek().is_none() {
                    return Equal;
                } else {
                    match left_len.cmp(&right_len) {
                        Equal => continue,
                        gtlt => return gtlt,
                    }
                }
            }
        }
    }
    // If we've fallen through then all blocks of the version matched.
    Ordering::Equal
}

/// Hash a version section, following the law `h1 == h2 => hash(h1) == hash(h2)`, where equality
/// comes from `Ord`.
fn version_hash<H>(input: &str, hasher: &mut H)
where
    H: Hasher,
{
    use self::{
        Block::{Alpha, Numeric, Separator},
        Ordering::{Equal, Greater, Less},
    };
    let mut blocks_iter = BlocksIter::new(input.as_bytes()).peekable();
    while let Some(block) = blocks_iter.next() {
        match block {
            Alpha(bytes) => hasher.write(bytes),
            Numeric(bytes) => hasher.write(bytes),
            Separator(length) => {
                if blocks_iter.peek().is_some() {
                    hasher.write_usize(length)
                }
            }
        }
    }
}

/// Remove leading `b'0'`s from a byte string.
fn discard_zeros(input: &[u8]) -> &[u8] {
    let mut pos = 0;
    while input.get(pos) == Some(&b'0') {
        pos += 1;
    }
    &input[pos..]
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering::*;

    #[test]
    fn version_cmp() {
        let test_set = vec![
            (&""[..], &""[..], Equal),
            (&"1"[..], &""[..], Greater),
            (&""[..], &"1"[..], Less),
            (&"1"[..], &"1"[..], Equal),
            (&"a"[..], &"1"[..], Less),
            (&"1"[..], &"2"[..], Less),
            (&"2"[..], &"1"[..], Greater),
            (&"001"[..], &"2"[..], Less),
            (&"001"[..], &"1"[..], Equal),
            (&"aa||123"[..], &"aa^^123"[..], Equal),
            (&"aa||123"[..], &"aa^^123"[..], Equal),
            (&"1.2.4alpha"[..], &"1.2.4"[..], Less),
            (&"1.2.4-alpha"[..], &"1.2.4"[..], Less),
            (&"1.2.4-1"[..], &"1.2.4"[..], Greater),
            (&"1.2.4--"[..], &"1.2.4---"[..], Equal),
            (&"123abc%%^%123abc"[..], &"123**$%abc123abc"[..], Less),
        ];
        for (left, right, cmp) in test_set.into_iter() {
            assert_eq!(
                super::version_cmp(left, right),
                cmp,
                r#"version_cmp("{}", "{}")"#,
                left,
                right
            );
        }
    }

    #[test]
    fn parse_version() {
        use super::Version;
        let test_set = vec![
            (&""[..], Version::new("0", "", None)),
            (&"1"[..], Version::new("0", "1", None)),
            (&"0"[..], Version::new("0", "0", None)),
            (&"1-1"[..], Version::new("0", "1", Some("1"))),
            (&"1:1-1"[..], Version::new("1", "1", Some("1"))),
            (&"alpha:1-1"[..], Version::new("0", "alpha:1", Some("1"))),
        ];
        for (test, expected) in test_set.into_iter() {
            let version = super::Version::parse(test);
            assert_eq!(version.epoch, expected.epoch, "epoch");
            assert_eq!(version.version, expected.version, "version");
            assert_eq!(version.release, expected.release, "release");
        }
    }

    #[test]
    fn version() {
        use super::Version;
        let test_set = vec![
            (&""[..], &""[..], Equal),
            (&"1"[..], &""[..], Greater),
            (&"0"[..], &"1"[..], Less),
            (&"0:1"[..], &"1:0"[..], Less),
            (&"1-1"[..], &"1"[..], Equal),
            (&"v1.0.0-alpha"[..], &"v1.0.0"[..], Equal),
            (&"1:1.0.0-100"[..], &"0:v1000.0.0"[..], Greater),
        ];
        for (left, right, cmp) in test_set.into_iter() {
            assert_eq!(
                Version::parse(left).cmp(&Version::parse(right)),
                cmp,
                r#"cmp("{}", "{}")"#,
                left,
                right
            );
        }
    }

    #[test]
    fn hash() {
        use super::Version;
        use std::collections::{BTreeSet, HashSet};
        let mut set1 = HashSet::new();
        let mut set2 = BTreeSet::new();
        for val in vec![
            Version::parse("1-"),
            Version::parse("1 "),
            Version::parse("1"),
            Version::parse("01"),
            Version::parse("a"),
        ] {
            set1.insert(val.clone());
            set2.insert(val);
        }
        assert_eq!(set1.len(), 2, "set1.len()");
        assert_eq!(set2.len(), 2, "set2.len()");
    }
}
