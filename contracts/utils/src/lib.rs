pub type Hash = [u8; 32];

pub trait Hashable {
    fn hash(&self) -> Hash;
}

pub fn swap_bytes4(v: u32) -> u32 {
    let r = ((v & 0x00ff00ff) << 8) | ((v & 0xff00ff00) >> 8);
    return (r << 16) | (r >> 16);
}

pub fn swap_bytes8(v: u64) -> u64 {
    let mut r = ((v & 0x00ff00ff00ff00ff) << 8) | ((v & 0xff00ff00ff00ff00) >> 8);
    r = ((r & 0x0000ffff0000ffff) << 16) | ((r & 0xffff0000ffff0000) >> 16);
    return (r << 32) | (r >> 32);
}

pub fn swap_bytes16(v: u128) -> u128 {
    let mut r = ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff) << 8)
        | ((v & 0xff00ff00ff00ff00ff00ff00ff00ff00) >> 8);
    r = ((r & 0x0000ffff0000ffff0000ffff0000ffff) << 16)
        | ((r & 0xffff0000ffff0000ffff0000ffff0000) >> 16);
    r = ((r & 0x00000000ffffffff00000000ffffffff) << 32)
        | ((r & 0xffffffff00000000ffffffff00000000) >> 32);
    return (r << 64) | (r >> 64);
}

pub mod u128_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", num))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u128::from_str_radix(&s, 10).map_err(de::Error::custom)
    }
}

pub mod u128_dec_format_compatible {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer};

    pub use super::u128_dec_format::serialize;

    #[derive(Deserialize)]
    #[serde(untagged)]
    #[serde(crate = "near_sdk::serde")]
    enum U128 {
        Number(u128),
        String(String),
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        match U128::deserialize(deserializer)? {
            U128::Number(value) => Ok(u128::from(value)),
            U128::String(value) => u128::from_str_radix(&value, 10).map_err(de::Error::custom),
        }
    }
}

pub mod u64_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", num))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u64::from_str_radix(&s, 10).map_err(de::Error::custom)
    }
}

pub mod u64_dec_format_compatible {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer};

    pub use super::u64_dec_format::serialize;

    #[derive(Deserialize)]
    #[serde(untagged)]
    #[serde(crate = "near_sdk::serde")]
    enum U64 {
        Number(u64),
        String(String),
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        match U64::deserialize(deserializer)? {
            U64::Number(value) => Ok(u64::from(value)),
            U64::String(value) => u64::from_str_radix(&value, 10).map_err(de::Error::custom),
        }
    }
}

pub mod merkle_u8_format {
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(data: &u8, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let direction = if data == &0u8 {
            "Left"
        } else {
            "Right"
        };
        serializer.serialize_str(&direction)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u8, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if s == "Left" {
            Ok(0)
        } else {
            Ok(1)
        }
    }
}

pub mod logging {
    use crate::to_base;
    use std::fmt::Debug;

    const VECTOR_MAX_LENGTH: usize = 5;
    const STRING_PRINT_LEN: usize = 128;

    pub fn pretty_utf8(buf: &[u8]) -> String {
        match std::str::from_utf8(buf) {
            Ok(s) => pretty_hash(s),
            Err(_) => {
                if buf.len() <= STRING_PRINT_LEN {
                    pretty_hash(&to_base(buf))
                } else {
                    pretty_vec(buf)
                }
            }
        }
    }

    pub fn pretty_vec<T: Debug>(buf: &[T]) -> String {
        if buf.len() <= VECTOR_MAX_LENGTH {
            format!("{:#?}", buf)
        } else {
            format!(
                "({})[{:#?}, {:#?}, … {:#?}, {:#?}]",
                buf.len(),
                buf[0],
                buf[1],
                buf[buf.len() - 2],
                buf[buf.len() - 1]
            )
        }
    }

    pub fn pretty_str(s: &str, print_len: usize) -> String {
        if s.len() <= print_len {
            format!("`{}`", s)
        } else {
            format!(
                "({})`{}…`",
                s.len(),
                &s.chars().take(print_len).collect::<String>()
            )
        }
    }

    pub fn pretty_hash(s: &str) -> String {
        pretty_str(s, STRING_PRINT_LEN)
    }
}

pub fn to_base<T: AsRef<[u8]>>(input: T) -> String {
    near_sdk::bs58::encode(input).into_string()
}

pub fn from_base(s: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    near_sdk::bs58::decode(s)
        .into_vec()
        .map_err(|err| err.into())
}

pub fn to_base64<T: AsRef<[u8]>>(input: T) -> String {
    near_sdk::base64::encode(&input)
}

pub fn from_base64(s: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    near_sdk::base64::decode(s).map_err(|err| err.into())
}

pub mod base64_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    use super::{from_base64, to_base64};

    pub fn serialize<S, T>(data: T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: AsRef<[u8]>,
    {
        serializer.serialize_str(&to_base64(data))
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: From<Vec<u8>>,
    {
        let s = String::deserialize(deserializer)?;
        from_base64(&s)
            .map_err(|err| de::Error::custom(err.to_string()))
            .map(Into::into)
    }
}

pub mod option_base64_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    use super::{from_base64, to_base64};

    pub fn serialize<S>(data: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref bytes) = data {
            serializer.serialize_str(&to_base64(bytes))
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        if let Some(s) = s {
            Ok(Some(
                from_base64(&s).map_err(|err| de::Error::custom(err.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }
}

pub mod base_hash_format {
    use crate::Hash;
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    use super::to_base;

    pub fn serialize<S>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&to_base(data))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Hash, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut array = [0; 32];
        let _length = near_sdk::bs58::decode(s)
            .into(&mut array[..])
            .map_err(|err| de::Error::custom(err.to_string()))?;
        Ok(array)
    }
}

pub mod base_hash_format_many {
    use crate::Hash;
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};
    use near_sdk::serde::ser::SerializeSeq;

    use super::to_base;

    pub fn serialize<S>(data: &Vec<Hash>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(data.len()))?;
        for e in data {
            seq.serialize_element(&to_base(e))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Hash>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<String>::deserialize(deserializer)?;
        
        let mut rec: Vec<Hash> = vec![];
        for s in vec {
            let mut array = [0; 32];
            let _length = near_sdk::bs58::decode(s)
                .into(&mut array[..])
                .map_err(|err| de::Error::custom(err.to_string()))?;
            rec.push(array);
        }
        Ok(rec)
    }
}

pub mod base_bytes_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    use super::{from_base, to_base};

    pub fn serialize<S>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&to_base(data))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        from_base(&s).map_err(|err| de::Error::custom(err.to_string()))
    }
}

pub mod string_bytes_format_many {
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};
    use near_sdk::serde::ser::SerializeSeq;

    pub fn serialize<S>(data: &Vec<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(data.len()))?;
        for e in data {
            seq.serialize_element(std::str::from_utf8(e).unwrap())?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<String>::deserialize(deserializer)?;
        
        let mut rec: Vec<Vec<u8>> = vec![];
        for s in vec {
            rec.push(s.as_bytes().to_vec());
        }
        Ok(rec)
    }
}

pub mod hashes {
    use crate::Hash;
    use near_sdk::{bs58, env};
    use std::fmt::Write;

    pub fn deserialize_hash(s: &String) -> Option<Hash> {
        // base58-encoded string is at most 1.4 longer than the binary sequence, but factor of 2 is
        // good enough to prevent DoS.
        if s.len() > std::mem::size_of::<Hash>() * 2 {
            return None;
        }
        match bs58::decode(&s).into_vec() {
            Ok(x) => Some(x.try_into().unwrap()),
            _ => None,
        }
    }
    // TODO make generic combine hash
    pub fn combine_hash2(x: Hash, y: Hash) -> Hash {
        let combined: Vec<u8> = x.iter().copied().chain(y.iter().copied()).collect();

        return env::sha256(&combined).try_into().unwrap();
    }
    pub fn combine_hash3(x: Hash, y: Hash, z: Hash) -> Hash {
        return combine_hash2(combine_hash2(x, y), z);
    }
    pub fn combine_hash4(x: Hash, y: Hash, z: Hash, q: Hash) -> Hash {
        return combine_hash2(combine_hash3(x, y, z), q);
    }
    pub fn encode_hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            write!(&mut s, "{:02x}", b).unwrap();
        }
        s
    }

    pub fn decode_hex(chars: &str) -> Vec<u8> {
        (0..chars.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&chars[i..i + 2], 16).unwrap())
            .collect()
    }
}
