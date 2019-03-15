//! Module to hold logic for parsing and comparing versions.
use itertools::Itertools;
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

const DEFAULT_EPOCH: &'static str = "0";

/// An alpm version.
///
/// It has a custom Ord impl to match version ordering. See the tests to get a feel for how it
/// works.
#[derive(Debug, Copy, Clone)]
pub struct Version<'a> {
    /// The epoch (optional, defaults to "0")
    pub epoch: &'a str,
    /// The version
    pub version: &'a str,
    /// The release (optional)
    pub release: Option<&'a str>,
}

impl<'a> Version<'a> {
    // Match wierd algorithms in alpm.
    pub fn parse(input: &str) -> Version {
        let has_epoch = HasEpoch::parse(input);
        let release_start_idx = input.rfind('-');
        Version {
            epoch: has_epoch.epoch(),
            version: &input
                [has_epoch.version_start_idx()..release_start_idx.unwrap_or(input.len())],
            release: release_start_idx.map(|idx| &input[idx + 1..]),
        }
    }

    pub fn to_owned(&self) -> VersionOwned {
        VersionOwned {
            epoch: self.epoch.to_owned(),
            version: self.version.to_owned(),
            release: self.release.map(|r| r.to_owned()),
        }
    }
}

/// An alpm version
pub struct VersionOwned {
    /// The epoch (optional, defaults to "0")
    pub epoch: String,
    /// The version
    pub version: String,
    /// The release (optional)
    pub release: Option<String>,
}

impl VersionOwned {
    pub fn borrow<'a>(&'a self) -> Version<'a> {
        Version {
            epoch: &self.epoch,
            version: &self.version,
            release: self.release.as_ref().map(AsRef::as_ref),
        }
    }
}

impl PartialOrd for Version<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match rpm_ver_cmp(&self.epoch.as_bytes(), &other.epoch.as_bytes()) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            Ordering::Equal => (), // continue
        };
        match rpm_ver_cmp(self.version.as_bytes(), other.version.as_bytes()) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            Ordering::Equal => (), // continue
        };
        // If either is missing just evaluate to equal - this matches alpm.
        match (&self.release, &other.release) {
            (Some(left), Some(right)) => rpm_ver_cmp(left.as_bytes(), right.as_bytes()),
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

/// Helper enum to store state from the version parse algorithm.
///
/// TODO probably should refactor this out at some point.
enum HasEpoch<'a> {
    Yes {
        epoch: &'a str,
        version_start_idx: usize,
    },
    No,
}

impl<'a> HasEpoch<'a> {
    #[inline]
    fn parse(input: &'a str) -> Self {
        if let Some((idx, _)) = input
            .char_indices()
            .skip_while(|(_, ch)| ch.is_ascii_digit())
            .next()
        {
            HasEpoch::Yes {
                epoch: &input[0..idx],
                version_start_idx: idx + 1,
            }
        } else {
            HasEpoch::No
        }
    }

    #[inline]
    fn version_start_idx(&self) -> usize {
        match self {
            HasEpoch::Yes {
                version_start_idx, ..
            } => *version_start_idx,
            HasEpoch::No => 0,
        }
    }

    #[inline]
    fn epoch(&self) -> &'a str {
        match self {
            HasEpoch::Yes { epoch, .. } => epoch,
            HasEpoch::No => DEFAULT_EPOCH,
        }
    }
}

macro_rules! debug_assert_boundary {
    ($str:expr, $idx:expr) => {
        debug_assert!(
            $str.is_char_boundary($idx) || $str.len() == $idx,
            "reading character out of utf8 alignment"
        )
    };
}

/// rpm's version cmp operation
///
/// This is a load of rubbish, but we have to match it to be compatible.
fn rpm_ver_cmp(left: &[u8], right: &[u8]) -> Ordering {
    let mut left_slice = left;
    let mut right_slice = right;

    if std::ptr::eq(left, right) {
        return Ordering::Equal;
    }

    while left_slice.len() > 0 && right_slice.len() > 0 {
        let left = split_off_nonalphanum(left_slice);
        let left_nonalpha = left.0;
        left_slice = left.1;
        let right = split_off_nonalphanum(right_slice);
        let right_nonalpha = right.0;
        right_slice = right.1;

        if left_slice.is_empty() || right_slice.is_empty() {
            break;
        }

        match left_nonalpha.len().cmp(&right_nonalpha.len()) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            Ordering::Equal => (), // continue
        };

        // cannot panic, we know the slice isn't empty.
        let numeric = left_slice[0].is_ascii_digit();
        // collect segments
        let left_current;
        let right_current;
        if numeric {
            let left = split_off_numeric(left_slice);
            left_current = left.0;
            left_slice = left.1;
            let right = split_off_numeric(right_slice);
            right_current = right.0;
            right_slice = right.1;
        } else {
            let left = split_off_alpha(left_slice);
            left_current = left.0;
            left_slice = left.1;
            let right = split_off_alpha(right_slice);
            right_current = right.0;
            right_slice = right.1;
        }

        if right_current.is_empty() {
            return if numeric {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }

        if numeric {
            // shadow
            let left_current = discard_zeros(left_current);
            let right_current = discard_zeros(right_current);

            match left_current.len().cmp(&right_current.len()) {
                Ordering::Greater => return Ordering::Greater,
                Ordering::Less => return Ordering::Less,
                Ordering::Equal => (), // continue
            }
            match left_current.cmp(right_current) {
                Ordering::Greater => return Ordering::Greater,
                Ordering::Less => return Ordering::Less,
                Ordering::Equal => (), // continue
            }
        } else {
            match left_current.cmp(right_current) {
                Ordering::Greater => return Ordering::Greater,
                Ordering::Less => return Ordering::Less,
                Ordering::Equal => (), // continue
            }
        }
    }

    if left_slice.is_empty() && right_slice.is_empty() {
        Ordering::Equal
    } else if (left_slice.is_empty() && !right_slice[0].is_ascii_alphabetic())
        || left_slice[0].is_ascii_alphabetic()
    {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

fn split_off_nonalphanum(input: &[u8]) -> (&[u8], &[u8]) {
    split_off(input, |ch| !ch.is_ascii_alphanumeric())
}

fn split_off_alpha(input: &[u8]) -> (&[u8], &[u8]) {
    split_off(input, |ch| ch.is_ascii_alphabetic())
}
fn split_off_numeric(input: &[u8]) -> (&[u8], &[u8]) {
    split_off(input, |ch| ch.is_ascii_digit())
}

fn split_off<F>(input: &[u8], include: F) -> (&[u8], &[u8])
where
    F: Fn(u8) -> bool,
{
    let idx = input
        .iter()
        .enumerate()
        .filter(|(_, ch)| !include(**ch))
        .map(|(idx, _)| idx)
        .next()
        .unwrap_or(input.len());
    (&input[..idx], &input[idx..])
}

fn discard_zeros(input: &[u8]) -> &[u8] {
    let mut pos = 0;
    while input.get(pos) == Some(&b'0') {
        pos += 1;
    }
    &input[pos..]
}

/*

/// Sections of the version string, either all alpha or all numeric.
enum VersionSection<'a> {
    /// non-ascii-alphanumeric chars with length
    Separator(usize),
    /// alpha chars
    Alpha(&'a str),
    /// digit chars
    Digit(&'a str),
}

/// Returns slices on the input stream
struct BlockIter {
    rest: &str,
    /// Whether we have just had a separator. Every gap must have a separator (can be 0 length).
    had_separator: bool,
}

impl BockIter {
    fn new(input: &str) -> Self {
        BlockIter {
            rest: input,
            had_separator: false,
        }
    }
}

impl Iterator for BlockIter<'a> {
    type Item = VersionSection<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.had_separator {
            // Check we got all the non-alphanum stuff.
            match self.rest.chars().next() {
                Some(ch) if ch.is_ascii_alpha() => {
                    match self
                        .rest
                        .char_indices()
                        .filter(|(_, ch)| ch.is_ascii_alpha())
                        .next()
                        .map(|(idx, _)| idx)
                    {
                        Some(idx) => {
                            let section = VersionSection::Alpha(self.rest[..idx]);
                            self.rest = self.rest[idx..];
                            section
                        }
                        None => {
                            let section = VersionSection::Alpha(self.rest);
                            self.rest = "";
                            section
                        }
                    }
                }
                Some(ch) if ch.is_ascii_digit() => {
                    match self
                        .rest
                        .char_indices()
                        .filter(|(_, ch)| ch.is_ascii_digit())
                        .next()
                        .map(|(idx, _)| idx)
                    {
                        Some(idx) => {
                            let section = VersionSection::Digit(self.rest[..idx]);
                            self.rest = self.rest[idx..];
                            section
                        }
                        None => {
                            let section = VersionSection::Digit(self.rest);
                            self.rest = "";
                            section
                        }
                    }
                }
                Some(_) => panic!("had a separator, so expecting alphanum"),
                None => {
                    self.rest = "";
                    None
                }
            }
        } else {
            // skip non-alpha-numeric
            match self
                .rest
                .char_indices()
                .filter(|(_, ch)| ch.is_ascii_alphanumeric())
                .next()
            {
                Some(p) => {
                    self.rest = self.rest[p..];
                    return Some(VersionSection::Separator(p));
                }
                None => {
                    // will make it faster if we call again.
                    self.rest = "";
                    return None;
                }
            };
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use std::cmp::Ordering::*;

    #[test]
    fn rpm_ver_cmp() {
        let test_set = vec![
            (&b""[..], &b""[..], Equal),
            (&b"1"[..], &b""[..], Greater),
            (&b""[..], &b"1"[..], Less),
            (&b"1"[..], &b"1"[..], Equal),
            (&b"a"[..], &b"1"[..], Less),
            (&b"1"[..], &b"2"[..], Less),
            (&b"2"[..], &b"1"[..], Greater),
            (&b"001"[..], &b"2"[..], Less),
            (&b"001"[..], &b"1"[..], Equal),
            (&b"aa||123"[..], &b"aa^^123"[..], Equal),
            (&b"aa||123"[..], &b"aa^^123"[..], Equal),
            (&b"1.2.4alpha"[..], &b"1.2.4"[..], Less),
            (&b"1.2.4-alpha"[..], &b"1.2.4"[..], Greater),
            (&b"123abc%%^%123abc"[..], &b"123**$%abc123abc"[..], Less),
        ];
        for (left, right, cmp) in test_set.into_iter() {
            assert_eq!(
                super::rpm_ver_cmp(left, right),
                cmp,
                r#"cmp("{}", "{}")"#,
                String::from_utf8_lossy(left),
                String::from_utf8_lossy(right)
            );
        }
    }

    fn version() {
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
                super::Version::parse(left).cmp(&super::Version::parse(right)),
                cmp,
                r#"cmp("{}", "{}")"#,
                left,
                right
            );
        }
    }
}
