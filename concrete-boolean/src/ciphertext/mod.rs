//! An encryption of a boolean message.
//!
//! This module implements the ciphertext structure containing an encryption of a Boolean message.

use concrete_core::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A structure containing a ciphertext, meant to encrypt a Boolean message.
///
/// It is used to evaluate a Boolean circuits homomorphically.
#[derive(Clone, Debug)]
pub enum Ciphertext {
    Encrypted(LweCiphertext32),
    Trivial(bool),
}

#[derive(Serialize, Deserialize)]
enum SerializableCiphertext {
    Encrypted(Vec<u8>),
    Trivial(bool),
}

impl Serialize for Ciphertext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser_eng = DefaultSerializationEngine::new(()).map_err(serde::ser::Error::custom)?;

        match self {
            Ciphertext::Encrypted(lwe) => {
                let ciphertext = ser_eng.serialize(lwe).map_err(serde::ser::Error::custom)?;
                SerializableCiphertext::Encrypted(ciphertext)
            }
            Ciphertext::Trivial(b) => SerializableCiphertext::Trivial(*b),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Ciphertext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let thing = SerializableCiphertext::deserialize(deserializer)?;

        let mut de_eng = DefaultSerializationEngine::new(()).map_err(serde::de::Error::custom)?;

        Ok(match thing {
            SerializableCiphertext::Encrypted(data) => {
                let lwe = de_eng
                    .deserialize(data.as_slice())
                    .map_err(serde::de::Error::custom)?;
                Self::Encrypted(lwe)
            }
            SerializableCiphertext::Trivial(b) => Self::Trivial(b),
        })
    }
}
