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
    pub fn combine_hash2(x: Hash, y: Hash) -> Hash {
        let combined: Vec<u8> = x.iter().copied().chain(y.iter().copied()).collect();

        return env::sha256(&combined).try_into().unwrap();
    }
    pub fn combine_hash3(x: Hash, y: Hash, z: Hash) -> Hash {
        let part1: Vec<u8> = x.iter().copied().chain(y.iter().copied()).collect();
        let part2: Vec<u8> = env::sha256(&part1)
            .iter()
            .copied()
            .chain(z.iter().copied())
            .collect();

        return env::sha256(&part2).try_into().unwrap();
    }
    pub fn encode_hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            write!(&mut s, "{:02x}", b).unwrap();
        }
        s
    }
}