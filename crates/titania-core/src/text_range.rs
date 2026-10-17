use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::TextRangeError;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextRange {
    start_byte: u32,
    end_byte: u32,
}

impl TextRange {
    pub fn new(start_byte: u32, end_byte: u32) -> Result<Self, TextRangeError> {
        (end_byte >= start_byte)
            .then_some(Self { start_byte, end_byte })
            .ok_or(TextRangeError::EndBeforeStart { start: start_byte, end: end_byte })
    }

    #[must_use]
    pub const fn start(self) -> u32 {
        self.start_byte
    }

    #[must_use]
    pub const fn end(self) -> u32 {
        self.end_byte
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.end_byte.saturating_sub(self.start_byte)
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start_byte == self.end_byte
    }

    #[must_use]
    pub const fn contains_byte(self, byte: u32) -> bool {
        byte >= self.start_byte && byte < self.end_byte
    }

    #[must_use]
    pub const fn overlaps(self, other: Self) -> bool {
        self.start_byte < other.end_byte && other.start_byte < self.end_byte
    }
}

impl fmt::Display for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}..{})", self.start_byte, self.end_byte)
    }
}

impl fmt::Debug for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TextRange(start={}, end={})", self.start_byte, self.end_byte)
    }
}

impl Serialize for TextRange {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = ser.serialize_struct("TextRange", 2)?;
        s.serialize_field("start_byte", &self.start_byte)?;
        s.serialize_field("end_byte", &self.end_byte)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for TextRange {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Repr {
            start_byte: u32,
            end_byte: u32,
        }
        let r = Repr::deserialize(de)?;
        Self::new(r.start_byte, r.end_byte).map_err(serde::de::Error::custom)
    }
}
