//! Canonical Base64 representations for file-owned bulk fields.

use base64::{engine::general_purpose::STANDARD, Engine};
use machine_loop::denise::frame_builder::FRAME_BUFFER_CAPACITY;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, marker::PhantomData};

pub(super) fn encode<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(bytes))
}

pub(super) fn decode<E: de::Error>(text: &str, lengths: &[usize]) -> Result<Vec<u8>, E> {
    if !lengths.is_empty()
        && !lengths
            .iter()
            .any(|&len| base64::encoded_len(len, true) == Some(text.len()))
    {
        return Err(E::custom("Invalid Base64 field length"));
    }
    let bytes = STANDARD.decode(text).map_err(E::custom)?;
    if !lengths.is_empty() && !lengths.contains(&bytes.len()) {
        return Err(E::custom("Incorrect byte count in Base64 field"));
    }
    Ok(bytes)
}

pub(super) fn read_string<'de, D, T>(
    decoder: D,
    decode: impl FnOnce(&str) -> Result<T, D::Error>,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    struct Read<F>(F);
    impl<'de, F, T, E> de::Visitor<'de> for Read<F>
    where
        F: FnOnce(&str) -> Result<T, E>,
        E: de::Error,
    {
        type Value = Result<T, E>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a canonical Base64 string")
        }

        fn visit_str<Err: de::Error>(self, text: &str) -> Result<Self::Value, Err> {
            Ok(self.0(text))
        }
    }

    decoder.deserialize_str(Read(decode))?
}

pub(super) mod preview {
    use super::*;

    pub(crate) fn serialize<S: Serializer>(bytes: &[u8], encoder: S) -> Result<S::Ok, S::Error> {
        encode(bytes, encoder)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(decoder: D) -> Result<Vec<u8>, D::Error> {
        read_string(decoder, |text| decode(text, &[]))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Bytes<const LEN: usize>(pub Vec<u8>);

impl<const LEN: usize> Serialize for Bytes<LEN> {
    fn serialize<S: Serializer>(&self, encoder: S) -> Result<S::Ok, S::Error> {
        encode(&self.0, encoder)
    }
}

impl<'de, const LEN: usize> Deserialize<'de> for Bytes<LEN> {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        read_string(decoder, |text| decode(text, &[LEN]).map(Self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Pixels(pub Vec<u32>);

impl Serialize for Pixels {
    fn serialize<S: Serializer>(&self, encoder: S) -> Result<S::Ok, S::Error> {
        let bytes: Vec<u8> = self
            .0
            .iter()
            .flat_map(|pixel| pixel.to_le_bytes())
            .collect();
        encode(&bytes, encoder)
    }
}

impl<'de> Deserialize<'de> for Pixels {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        read_string(decoder, |text| {
            let bytes = decode(text, &[FRAME_BUFFER_CAPACITY * size_of::<u32>()])?;
            let pixels = bytes
                .chunks_exact(4)
                .map(|word| u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
                .collect();
            Ok(Self(pixels))
        })
    }
}

/// Allows null but requires an explicit field. Serde's plain Option also
/// accepts a missing map entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(super) struct Optional<T>(pub Option<T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Optional<T> {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Value<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Value<T> {
            type Value = Optional<T>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an explicit value or null")
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Optional(None))
            }

            fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
                T::deserialize(de::value::StrDeserializer::new(text))
                    .map(|value| Optional(Some(value)))
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                T::deserialize(de::value::MapAccessDeserializer::new(map))
                    .map(|value| Optional(Some(value)))
            }
        }

        decoder.deserialize_any(Value(PhantomData))
    }
}
