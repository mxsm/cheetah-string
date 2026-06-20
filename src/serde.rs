use crate::CheetahString;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for CheetahString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

pub fn cheetah_string<'de: 'a, 'a, D>(deserializer: D) -> Result<CheetahString, D::Error>
where
    D: Deserializer<'de>,
{
    struct CheetahStringVisitor;

    impl<'a> Visitor<'a> for CheetahStringVisitor {
        type Value = CheetahString;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(CheetahString::from_slice(v))
        }

        fn visit_borrowed_str<E>(self, v: &'a str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(CheetahString::from_slice(v))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(CheetahString::from_string(v))
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: Error,
        {
            str::from_utf8(v)
                .map(CheetahString::from_slice)
                .map_err(Error::custom)
        }

        fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> Result<Self::Value, E>
        where
            E: Error,
        {
            str::from_utf8(v)
                .map(CheetahString::from_slice)
                .map_err(Error::custom)
        }

        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where
            E: Error,
        {
            CheetahString::try_from_vec(v).map_err(Error::custom)
        }
    }
    deserializer.deserialize_str(CheetahStringVisitor)
}

impl<'de> Deserialize<'de> for CheetahString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        cheetah_string(deserializer)
    }
}
