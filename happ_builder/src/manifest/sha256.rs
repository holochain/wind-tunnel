use serde::Deserialize;

use std::fmt::Formatter;
use std::str::FromStr;

/// A SHA-256 hash represented as a 32-byte array.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Sha256Hash([u8; 32]);

impl<'de> Deserialize<'de> for Sha256Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Sha256Hash::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl FromStr for Sha256Hash {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        Ok(Sha256Hash(
            bytes
                .try_into()
                .map_err(|_| hex::FromHexError::InvalidStringLength)?,
        ))
    }
}

impl std::fmt::Display for Sha256Hash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl From<[u8; 32]> for Sha256Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Sha256Hash(bytes)
    }
}

impl TryFrom<&[u8]> for Sha256Hash {
    type Error = hex::FromHexError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; 32] = slice
            .try_into()
            .map_err(|_| hex::FromHexError::InvalidStringLength)?;
        Ok(Sha256Hash(bytes))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn should_parse_sha256_hash() {
        let hash_str = "e3b0c44298fc1c149afbfc6c5d6a8e9b7f4f5c6d78e9f0a1b2c3d4e5f6a7b890";
        let hash = Sha256Hash::from_str(hash_str).expect("Failed to parse sha256 hash");
        assert_eq!(hash.to_string(), hash_str);
    }

    #[test]
    fn should_fail_on_invalid_sha256_hash() {
        let invalid_hash_str = "invalid_sha256_hash";
        let result = Sha256Hash::from_str(invalid_hash_str);
        assert!(result.is_err(), "Expected error for invalid sha256 hash");
    }

    #[test]
    fn should_convert_from_bytes() {
        let bytes: [u8; 32] = [0u8; 32];
        let hash = Sha256Hash::from(bytes);
        assert_eq!(hash.0, bytes);
    }

    #[test]
    fn should_try_convert_from_slice() {
        let bytes: [u8; 32] = [0u8; 32];
        let hash = Sha256Hash::try_from(&bytes[..]).expect("Failed to convert from slice");
        assert_eq!(
            hash.to_string(),
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn should_fail_on_invalid_slice_length() {
        let bytes: [u8; 31] = [0u8; 31];
        let result = Sha256Hash::try_from(&bytes[..]);
        assert!(result.is_err(), "Expected error for invalid slice length");
    }
}
