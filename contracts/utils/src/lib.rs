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
    let mut r = ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff) << 8) | ((v & 0xff00ff00ff00ff00ff00ff00ff00ff00) >> 8);
    r = ((r & 0x0000ffff0000ffff0000ffff0000ffff) << 16) | ((r & 0xffff0000ffff0000ffff0000ffff0000) >> 16);
    r = ((r & 0x00000000ffffffff00000000ffffffff) << 32) | ((r & 0xffffffff00000000ffffffff00000000) >> 32);
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
