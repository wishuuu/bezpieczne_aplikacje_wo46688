use std::io::Read;

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, OsRng},
    Aes256Gcm, Key, KeyInit, Nonce,
};
use axum::Json;
use base64::{prelude::BASE64_STANDARD, Engine};
use rsa::{
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct EncryptedDataDto {
    pub data: String,
    pub key: String,
}

#[derive(Clone)]
pub struct RsaCerts {
    pub public: &'static RsaPublicKey,
    pub private: &'static RsaPrivateKey,
}

impl EncryptedDataDto {
    pub fn new(data: String, key: String) -> Self {
        Self { data, key }
    }
}

fn encrypt_key(
    public_key: &RsaPublicKey,
    data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut rng = OsRng;

    let encrypted_data = public_key.encrypt(&mut rng, Pkcs1v15Encrypt, data)?;
    Ok(encrypted_data)
}

fn decrypt_key(
    private_key: &RsaPrivateKey,
    encrypted_data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let decrypted_data = private_key.decrypt(Pkcs1v15Encrypt, encrypted_data)?;
    Ok(decrypted_data)
}

pub async fn get_body_encryption(body: String) -> Json<EncryptedDataDto> {
    let mut file = std::fs::File::open("public.pem").unwrap();
    let mut public = String::new();
    file.read_to_string(&mut public).unwrap();
    let public_key = RsaPublicKey::from_pkcs1_pem(&public).unwrap();

    // generate random 32 bytes key
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    println!("Encryption key: {:?}", key);
    let key_rsa = encrypt_key(&public_key, &key).unwrap();

    let key = Key::<Aes256Gcm>::from_slice(&key[..32]);

    let nonce = [0u8; 12];
    let nonce = Nonce::from_slice(&nonce);

    let cipher = Aes256Gcm::new(key);

    let ciphered_data = cipher
        .encrypt(&nonce, body.as_bytes())
        .expect("failed to encrypt");

    let mut encrypted_data: Vec<u8> = nonce.to_vec();
    encrypted_data.extend_from_slice(&ciphered_data);

    let key_str = BASE64_STANDARD.encode(key_rsa);
    let encrypted_data = BASE64_STANDARD.encode(encrypted_data);

    Json(EncryptedDataDto::new(encrypted_data, key_str))
}

pub async fn get_body_decryption(body: Json<EncryptedDataDto>) -> String {
    let mut file = std::fs::File::open("private.pem").unwrap();
    let mut private = String::new();
    file.read_to_string(&mut private).unwrap();
    let private_key = RsaPrivateKey::from_pkcs1_pem(&private).unwrap();

    let k = BASE64_STANDARD.decode(body.key.as_bytes()).unwrap();
    let k = decrypt_key(&private_key, &k).unwrap();
    println!("Decryption key: {:?}", k);
    let d = BASE64_STANDARD.decode(body.data.as_bytes()).unwrap();
    println!("Decryption data: {:?}", d);

    let nonce = Nonce::from_slice(&d[0..12]);

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&k));

    let decrypted_data = cipher.decrypt(nonce, &d[12..]).unwrap();

    String::from_utf8(decrypted_data).unwrap()
}
