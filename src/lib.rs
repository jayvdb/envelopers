mod key_provider;
pub use crate::key_provider::KeyProvider;
pub use crate::key_provider::SimpleKeyProvider;
use aes_gcm::aead::{Aead, NewAead, Payload};
use aes_gcm::aes::cipher::consts::U12;
use aes_gcm::{Aes128Gcm, Key, Nonce}; // Or `Aes256Gcm`
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use std::cell::RefCell;

#[derive(Debug)]
pub struct EncryptedRecord<'aad> {
    pub ciphertext: Vec<u8>,
    pub encrypted_key: Vec<u8>,
    pub nonce: Nonce<U12>,
    pub aad: &'aad [u8], // FIXME: Just call this the key_id to keep it simple
}

#[derive(Debug)]
pub struct EncryptionError;

#[derive(Debug)]
pub struct DecryptionError;

pub struct EnvelopeCipher<K, R = ChaChaRng>
where
    K: KeyProvider,
    R: SeedableRng + RngCore,
{
    pub key_provider: K,
    pub rng: RefCell<R>,
}

impl<K, R> EnvelopeCipher<K, R>
where
    K: KeyProvider,
    R: SeedableRng + RngCore,
{
    pub fn init(key_provider: K) -> Self {
        Self {
            key_provider,
            rng: RefCell::new(R::from_entropy()),
        }
    }

    pub fn decrypt(&self, encrypted_record: EncryptedRecord) -> Result<Vec<u8>, DecryptionError> {
        let key = self
            .key_provider
            .decrypt_data_key(encrypted_record.encrypted_key)
            .map_err(|_| DecryptionError)?;
        let aad = encrypted_record.aad;
        let msg = encrypted_record.ciphertext.as_ref();
        let payload = Payload { msg, aad };

        let cipher = Aes128Gcm::new(&key);
        let message = cipher
            .decrypt(&encrypted_record.nonce, payload)
            .map_err(|_| DecryptionError)?;

        return Ok(message);
    }

    // TODO: Maybe this should take a vec as well??
    pub fn encrypt(&self, msg: &[u8]) -> Result<EncryptedRecord, EncryptionError> {
        let mut nonce: Nonce<U12> = Default::default();
        let data_key = self
            .key_provider
            .generate_data_key()
            .map_err(|_| EncryptionError)?;

        let aad = data_key.key_id.as_bytes();

        self.rng
            .borrow_mut()
            .try_fill_bytes(&mut nonce)
            .map_err(|_| EncryptionError)?;

        let payload = Payload { msg, aad };

        // TODO: Use Zeroize for the drop
        let key = Key::from_slice(&data_key.key);
        let cipher = Aes128Gcm::new(key);
        let ciphertext = cipher
            .encrypt(&nonce, payload)
            .map_err(|_| EncryptionError)?;

        return Ok(EncryptedRecord {
            ciphertext,
            nonce,
            encrypted_key: data_key.encrypted_key,
            aad,
        });
    }
}