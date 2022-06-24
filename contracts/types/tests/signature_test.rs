#[cfg(test)]
mod signature {
    use types::signature::{KeyType, PublicKey, SecretKey, Signature};

    trait FromRandom {
        fn from_random(key_type: KeyType) -> Self;
    }

    trait FromSeed {
        fn from_seed(key_type: KeyType, seed: &str) -> Self;
    }

    impl FromSeed for PublicKey {
        fn from_seed(key_type: KeyType, seed: &str) -> Self {
            use crate::signature::test::ed25519_key_pair_from_seed;
            use types::signature::ED25519PublicKey;
            match key_type {
                KeyType::ED25519 => {
                    let keypair = ed25519_key_pair_from_seed(seed);
                    PublicKey::ED25519(ED25519PublicKey(keypair.public.to_bytes()))
                }
                _ => unimplemented!(),
            }
        }
    }

    impl FromRandom for SecretKey {
        fn from_random(key_type: KeyType) -> SecretKey {
            use types::signature::ED25519SecretKey;

            match key_type {
                KeyType::ED25519 => {
                    let keypair = ed25519_dalek::Keypair::generate(&mut rand_core_05::OsRng);
                    SecretKey::ED25519(ED25519SecretKey(keypair.to_bytes()))
                }
                KeyType::SECP256K1 => {
                    SecretKey::SECP256K1(libsecp256k1::SecretKey::random(&mut rand_core_06::OsRng))
                }
            }
        }
    }

    impl FromSeed for SecretKey {
        fn from_seed(key_type: KeyType, seed: &str) -> Self {
            use crate::signature::test::ed25519_key_pair_from_seed;
            use crate::signature::test::secp256k1_secret_key_from_seed;
            use types::signature::ED25519SecretKey;
            match key_type {
                KeyType::ED25519 => {
                    let keypair = ed25519_key_pair_from_seed(seed);
                    SecretKey::ED25519(ED25519SecretKey(keypair.to_bytes()))
                }
                _ => SecretKey::SECP256K1(secp256k1_secret_key_from_seed(seed)),
            }
        }
    }
    mod test {
        use super::*;
        use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
        use near_sdk::serde_json;

        use types::signature::{KeyType, PublicKey, SecretKey};

        pub fn ed25519_key_pair_from_seed(seed: &str) -> ed25519_dalek::Keypair {
            let seed_bytes = seed.as_bytes();
            let len = std::cmp::min(ed25519_dalek::SECRET_KEY_LENGTH, seed_bytes.len());
            let mut seed: [u8; ed25519_dalek::SECRET_KEY_LENGTH] =
                [b' '; ed25519_dalek::SECRET_KEY_LENGTH];
            seed[..len].copy_from_slice(&seed_bytes[..len]);
            let secret = ed25519_dalek::SecretKey::from_bytes(&seed).unwrap();
            let public = ed25519_dalek::PublicKey::from(&secret);
            ed25519_dalek::Keypair { secret, public }
        }

        pub fn secp256k1_secret_key_from_seed(seed: &str) -> libsecp256k1::SecretKey {
            let seed_bytes = seed.as_bytes();
            let len = std::cmp::min(32, seed_bytes.len());
            let mut seed: [u8; 32] = [b' '; 32];
            seed[..len].copy_from_slice(&seed_bytes[..len]);
            let mut rng: rand_08::rngs::StdRng = rand_08::SeedableRng::from_seed(seed);
            libsecp256k1::SecretKey::random(&mut rng)
        }

        #[test]
        fn test_sign_verify() {
            for key_type in vec![KeyType::ED25519, KeyType::SECP256K1] {
                let secret_key = SecretKey::from_random(key_type);
                let public_key = secret_key.public_key();
                use near_sdk::env;
                let data = env::sha256(b"123").to_vec();
                let signature = secret_key.sign(&data);
                assert!(signature.verify(&data, &public_key));
            }
        }

        #[test]
        fn test_json_serialize_ed25519() {
            let sk = SecretKey::from_seed(KeyType::ED25519, "test");
            let pk = sk.public_key();
            let expected = "\"ed25519:DcA2MzgpJbrUATQLLceocVckhhAqrkingax4oJ9kZ847\"";
            assert_eq!(serde_json::to_string(&pk).unwrap(), expected);
            assert_eq!(pk, serde_json::from_str(expected).unwrap());
            assert_eq!(
                pk,
                serde_json::from_str("\"DcA2MzgpJbrUATQLLceocVckhhAqrkingax4oJ9kZ847\"").unwrap()
            );
            let pk2: PublicKey = pk.to_string().parse().unwrap();
            assert_eq!(pk, pk2);

            let expected = "\"ed25519:3KyUuch8pYP47krBq4DosFEVBMR5wDTMQ8AThzM8kAEcBQEpsPdYTZ2FPX5ZnSoLrerjwg66hwwJaW1wHzprd5k3\"";
            assert_eq!(serde_json::to_string(&sk).unwrap(), expected);
            assert_eq!(sk, serde_json::from_str(expected).unwrap());

            let signature = sk.sign(b"123");
            let expected = "\"ed25519:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj\"";
            assert_eq!(serde_json::to_string(&signature).unwrap(), expected);
            assert_eq!(signature, serde_json::from_str(expected).unwrap());
            let signature_str: String = signature.to_string();
            let signature2: Signature = signature_str.parse().unwrap();
            assert_eq!(signature, signature2);
        }

        #[test]
        fn test_json_serialize_secp256k1() {
            use near_sdk::env;
            let data = env::sha256(b"123").to_vec();

            let sk = SecretKey::from_seed(KeyType::SECP256K1, "test");
            let pk = sk.public_key();
            let expected = "\"secp256k1:5ftgm7wYK5gtVqq1kxMGy7gSudkrfYCbpsjL6sH1nwx2oj5NR2JktohjzB6fbEhhRERQpiwJcpwnQjxtoX3GS3cQ\"";
            assert_eq!(serde_json::to_string(&pk).unwrap(), expected);
            assert_eq!(pk, serde_json::from_str(expected).unwrap());
            let pk2: PublicKey = pk.to_string().parse().unwrap();
            assert_eq!(pk, pk2);

            let expected = "\"secp256k1:X4ETFKtQkSGVoZEnkn7bZ3LyajJaK2b3eweXaKmynGx\"";
            assert_eq!(serde_json::to_string(&sk).unwrap(), expected);
            assert_eq!(sk, serde_json::from_str(expected).unwrap());

            let signature = sk.sign(&data);
            let expected = "\"secp256k1:5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6\"";
            assert_eq!(serde_json::to_string(&signature).unwrap(), expected);
            assert_eq!(signature, serde_json::from_str(expected).unwrap());
            let signature_str: String = signature.to_string();
            let signature2: Signature = signature_str.parse().unwrap();
            assert_eq!(signature, signature2);
        }

        #[test]
        fn test_nearcore_json_serialize_secp256k1() {
            use near_sdk::env;
            use std::str::FromStr;
            let data = env::sha256(b"123").to_vec();

            let sk = SecretKey::from_str("secp256k1:9ZNzLxNff6ohoFFGkbfMBAFpZgD7EPoWeiuTpPAeeMRV")
                .unwrap();
            let pk = sk.public_key();
            let expected = "\"secp256k1:BtJtBjukUQbcipnS78adSwUKE38sdHnk7pTNZH7miGXfodzUunaAcvY43y37nm7AKbcTQycvdgUzFNWsd7dgPZZ\"";
            assert_eq!(serde_json::to_string(&pk).unwrap(), expected);
            assert_eq!(pk, serde_json::from_str(expected).unwrap());
            let pk2: PublicKey = pk.to_string().parse().unwrap();
            assert_eq!(pk, pk2);

            let expected = "\"secp256k1:9ZNzLxNff6ohoFFGkbfMBAFpZgD7EPoWeiuTpPAeeMRV\"";
            assert_eq!(serde_json::to_string(&sk).unwrap(), expected);
            assert_eq!(sk, serde_json::from_str(expected).unwrap());

            let signature = sk.sign(&data);
            let expected = "\"secp256k1:7iA75xRmHw17MbUkSpHxBHFVTuJW6jngzbuJPJutwb3EAwVw21wrjpMHU7fFTAqH7D3YEma8utCdvdtsqcAWqnC7r\"";
            assert_eq!(serde_json::to_string(&signature).unwrap(), expected);
            assert_eq!(signature, serde_json::from_str(expected).unwrap());
            let signature_str: String = signature.to_string();
            let signature2: Signature = signature_str.parse().unwrap();
            assert_eq!(signature, signature2);
        }

        #[test]
        fn test_borsh_serialization() {
            use near_sdk::env;
            let data = env::sha256(b"123").to_vec();
            for key_type in vec![KeyType::ED25519, KeyType::SECP256K1] {
                let sk = SecretKey::from_seed(key_type, "test");
                let pk = sk.public_key();
                let bytes = pk.try_to_vec().unwrap();
                assert_eq!(PublicKey::try_from_slice(&bytes).unwrap(), pk);

                let signature = sk.sign(&data);
                let bytes = signature.try_to_vec().unwrap();
                assert_eq!(Signature::try_from_slice(&bytes).unwrap(), signature);

                assert!(PublicKey::try_from_slice(&[0]).is_err());
                assert!(Signature::try_from_slice(&[0]).is_err());
            }
        }

        #[test]
        fn test_invalid_data() {
            let invalid = "\"secp256k1:2xVqteU8PWhadHTv99TGh3bSf\"";
            assert!(serde_json::from_str::<PublicKey>(invalid).is_err());
            assert!(serde_json::from_str::<SecretKey>(invalid).is_err());
            assert!(serde_json::from_str::<Signature>(invalid).is_err());
        }
    }
}
