use crate::{CheetahStr, CheetahString};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str;
use serde::de::{Error as DeError, Visitor};
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
            E: DeError,
        {
            Ok(CheetahString::from_slice(v))
        }

        fn visit_borrowed_str<E>(self, v: &'a str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(CheetahString::from_slice(v))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(CheetahString::from_string(v))
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            str::from_utf8(v)
                .map(CheetahString::from_slice)
                .map_err(DeError::custom)
        }

        fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            str::from_utf8(v)
                .map(CheetahString::from_slice)
                .map_err(DeError::custom)
        }

        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            CheetahString::try_from_vec(v).map_err(DeError::custom)
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

impl Serialize for CheetahStr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

pub fn cheetah_str<'de: 'a, 'a, D>(deserializer: D) -> Result<CheetahStr, D::Error>
where
    D: Deserializer<'de>,
{
    struct CheetahStrVisitor;

    impl<'a> Visitor<'a> for CheetahStrVisitor {
        type Value = CheetahStr;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(CheetahStr::from_slice(v))
        }

        fn visit_borrowed_str<E>(self, v: &'a str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(CheetahStr::from_slice(v))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(CheetahStr::from_string(v))
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            str::from_utf8(v)
                .map(CheetahStr::from_slice)
                .map_err(DeError::custom)
        }

        fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            str::from_utf8(v)
                .map(CheetahStr::from_slice)
                .map_err(DeError::custom)
        }

        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            String::from_utf8(v)
                .map(CheetahStr::from_string)
                .map_err(DeError::custom)
        }
    }

    deserializer.deserialize_str(CheetahStrVisitor)
}

impl<'de> Deserialize<'de> for CheetahStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        cheetah_str(deserializer)
    }
}
