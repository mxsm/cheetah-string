use crate::cheetah_string::InnerString;
use crate::CheetahString;
use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for CheetahString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.inner {
            InnerString::ArcString(s) => serializer.serialize_str(s.as_str()),
            InnerString::StaticStr(s) => serializer.serialize_str(s),
            #[cfg(feature = "bytes")]
            InnerString::Bytes(bytes) => serializer.serialize_bytes(bytes.as_ref()),
            InnerString::Empty => serializer.serialize_bytes(&[]),
        }
    }
}

pub fn cheetah_string<'de: 'a, 'a, D>(deserializer: D) -> Result<CheetahString, D::Error>
where
    D: Deserializer<'de>,
{
    struct CheetahStringVisitor;

    impl<'a> Visitor<'a> for CheetahStringVisitor {
        type Value = CheetahString;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
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
            Ok(CheetahString::from(v))
        }

        fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(CheetahString::from(v))
        }

        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where
            E: Error,
        {
            match String::from_utf8(v) {
                Ok(s) => Ok(CheetahString::from_string(s)),
                Err(e) => Err(Error::invalid_value(
                    Unexpected::Bytes(&e.into_bytes()),
                    &self,
                )),
            }
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
