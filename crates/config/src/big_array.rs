//! Custom Serde serializer and deserializer helper for arrays with length > 32.

use core::fmt;
use serde::de::{Error, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserializer, Serializer};

/// Serializes fixed-size arrays of any length `N`.
pub fn serialize<S, T, const N: usize>(data: &[T; N], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: serde::Serialize,
{
    let mut seq = serializer.serialize_seq(Some(N))?;
    for elem in data.iter() {
        seq.serialize_element(elem)?;
    }
    seq.end()
}

/// Deserializes fixed-size arrays of any length `N` where `T: Copy + Default`.
pub fn deserialize<'de, D, T, const N: usize>(deserializer: D) -> Result<[T; N], D::Error>
where
    D: Deserializer<'de>,
    T: serde::Deserialize<'de> + Copy + Default,
{
    struct ArrayVisitor<T, const N: usize>(core::marker::PhantomData<T>);

    impl<'de, T, const N: usize> Visitor<'de> for ArrayVisitor<T, N>
    where
        T: serde::Deserialize<'de> + Copy + Default,
    {
        type Value = [T; N];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "an array of length {}", N)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut arr = [T::default(); N];
            for (i, item) in arr.iter_mut().enumerate() {
                *item = seq
                    .next_element()?
                    .ok_or_else(|| A::Error::invalid_length(i, &self))?;
            }
            Ok(arr)
        }
    }

    deserializer.deserialize_seq(ArrayVisitor(core::marker::PhantomData))
}
