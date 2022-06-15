pub fn swap_bytes8(v: u64) -> u64 {
    let mut r = ((v & 0x00ff00ff00ff00ff) << 8) | ((v & 0xff00ff00ff00ff00) >> 8);
    r = ((r & 0x0000ffff0000ffff) << 16) | ((r & 0xffff0000ffff0000) >> 16);
    return (r << 32) | (r >> 32);
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
}
