use core::fmt;
use core::ops::Deref;
use core::slice;
use core::str;

pub trait ZeroCopy {
    type Output;
    fn as_slice(&self) -> &[u8];
    fn as_str(&self) -> &str;
}

pub struct ByteSlice {
    data: [u8],
}

impl ByteSlice {
    #[inline]
    pub fn new(data: &[u8]) -> &ByteSlice {
        unsafe { &*(data as *const [u8] as *const ByteSlice) }
    }

    #[inline]
    pub unsafe fn from_raw(ptr: *const u8, len: usize) -> &ByteSlice {
        let slice = slice::from_raw_parts(ptr, len);
        &*(slice as *const [u8] as *const ByteSlice)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&u8> {
        self.data.get(index)
    }

    #[inline]
    pub fn split_at(&self, mid: usize) -> (&ByteSlice, &ByteSlice) {
        let (left, right) = self.data.split_at(mid);
        (ByteSlice::new(left), ByteSlice::new(right))
    }

    #[inline]
    pub fn starts_with(&self, prefix: &[u8]) -> bool {
        self.data.starts_with(prefix)
    }

    #[inline]
    pub fn ends_with(&self, suffix: &[u8]) -> bool {
        self.data.ends_with(suffix)
    }

    #[inline]
    pub fn contains(&self, byte: u8) -> bool {
        self.data.contains(&byte)
    }
}

impl Deref for ByteSlice {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl fmt::Debug for ByteSlice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.data, f)
    }
}

impl fmt::Display for ByteSlice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.data))
    }
}

pub struct StrView {
    data: [u8],
}

impl StrView {
    #[inline]
    pub fn new(s: &str) -> &StrView {
        unsafe { &*(s.as_bytes() as *const [u8] as *const StrView) }
    }

    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Option<&StrView> {
        str::from_utf8(bytes).ok().map(|s| StrView::new(s))
    }

    #[inline]
    pub unsafe fn from_utf8_unchecked(bytes: &[u8]) -> &StrView {
        &*(bytes as *const [u8] as *const StrView)
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.data) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<char> {
        self.as_str().chars().nth(index)
    }

    #[inline]
    pub fn chars(&self) -> core::str::Chars<'_> {
        self.as_str().chars()
    }

    #[inline]
    pub fn bytes(&self) -> core::slice::Iter<'_, u8> {
        self.data.iter()
    }

    #[inline]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.as_str().starts_with(prefix)
    }

    #[inline]
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.as_str().ends_with(suffix)
    }

    #[inline]
    pub fn contains(&self, substring: &str) -> bool {
        self.as_str().contains(substring)
    }

    #[inline]
    pub fn split(&self, sep: &str) -> core::str::Split<'_> {
        self.as_str().split(sep)
    }

    #[inline]
    pub fn trim(&self) -> &StrView {
        StrView::new(self.as_str().trim())
    }

    #[inline]
    pub fn to_lowercase(&self) -> String {
        self.as_str().to_lowercase()
    }

    #[inline]
    pub fn to_uppercase(&self) -> String {
        self.as_str().to_uppercase()
    }
}

impl Deref for StrView {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Debug for StrView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for StrView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl PartialEq for StrView {
    fn eq(&self, other: &StrView) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for StrView {}

impl PartialEq<str> for StrView {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for StrView {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

pub struct CowStr<'a> {
    data: CowData<'a>,
}

enum CowData<'a> {
    Borrowed(&'a str),
    Owned(String),
}

impl<'a> CowStr<'a> {
    #[inline]
    pub fn borrow(s: &'a str) -> CowStr<'a> {
        CowStr {
            data: CowData::Borrowed(s),
        }
    }

    #[inline]
    pub fn owned(s: String) -> CowStr<'a> {
        CowStr {
            data: CowData::Owned(s),
        }
    }

    #[inline]
    pub fn is_borrowed(&self) -> bool {
        matches!(self.data, CowData::Borrowed(_))
    }

    #[inline]
    pub fn is_owned(&self) -> bool {
        matches!(self.data, CowData::Owned(_))
    }

    #[inline]
    pub fn into_owned(self) -> String {
        match self.data {
            CowData::Borrowed(s) => s.to_string(),
            CowData::Owned(s) => s,
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.data {
            CowData::Borrowed(s) => s,
            CowData::Owned(s) => s.as_str(),
        }
    }
}

impl Deref for CowStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Debug for CowStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for CowStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

pub struct CowBytes<'a> {
    data: CowBytesData<'a>,
}

enum CowBytesData<'a> {
    Borrowed(&'a [u8]),
    Owned(Vec<u8>),
}

impl<'a> CowBytes<'a> {
    #[inline]
    pub fn borrow(s: &'a [u8]) -> CowBytes<'a> {
        CowBytes {
            data: CowBytesData::Borrowed(s),
        }
    }

    #[inline]
    pub fn owned(s: Vec<u8>) -> CowBytes<'a> {
        CowBytes {
            data: CowBytesData::Owned(s),
        }
    }

    #[inline]
    pub fn is_borrowed(&self) -> bool {
        matches!(self.data, CowBytesData::Borrowed(_))
    }

    #[inline]
    pub fn into_owned(self) -> Vec<u8> {
        match self.data {
            CowBytesData::Borrowed(s) => s.to_vec(),
            CowBytesData::Owned(s) => s,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        match &self.data {
            CowBytesData::Borrowed(s) => s,
            CowBytesData::Owned(s) => s.as_slice(),
        }
    }
}

impl Deref for CowBytes<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl fmt::Debug for CowBytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.as_slice(), f)
    }
}

pub struct Substring<'a> {
    parent: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Substring<'a> {
    #[inline]
    pub fn new(parent: &'a str, start: usize, end: usize) -> Option<Substring<'a>> {
        if end <= parent.len() && start < end {
            Some(Substring { parent, start, end })
        } else {
            None
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.parent[self.start..self.end]
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl Deref for Substring<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Debug for Substring<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for Substring<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}
