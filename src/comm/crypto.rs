use std::{fmt::Display, time::Duration};

use aes_gcm_siv::{
    aead::{generic_array::GenericArray, rand_core::RngCore, Aead, OsRng},
    Aes256GcmSiv, KeyInit,
};
use rkyv::{Archive, CheckBytes, Deserialize, Serialize};
use rsa::{
    pkcs1::{DecodeRsaPublicKey, EncodeRsaPublicKey},
    Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};
use tokio::{
    io,
    time::{self, error::Elapsed},
};

use super::encaps::{AsyncMsgRecv, AsyncMsgSend};

/// Initialization data for an AES256-GCM encrypted endpoint
/// Contains the encryption key and initial nonce value
#[derive(Archive, Serialize, Deserialize, CheckBytes, Debug)]
#[archive(check_bytes)]
pub struct AES256GCMInitializer {
    key: [u8; 32],
    nonce: [u8; 12],
}

impl AES256GCMInitializer {
    /// Constructs a new encryption key and initial nonce pair from the OS RNG
    pub fn new_rand() -> Self {
        let mut key = [0u8; 32];
        let mut nonce = [0u8; 12];

        // Randomly generate key and nonce
        OsRng.fill_bytes(&mut key);
        OsRng.fill_bytes(&mut nonce);

        Self { key, nonce }
    }
}

/// Initialization data for an AESE256-GCM encrypted channel
#[derive(Archive, Serialize, Deserialize, CheckBytes, Debug)]
#[archive(check_bytes)]
pub struct AES256GCMInitializerPair {
    pub cts: AES256GCMInitializer, // Client-to-server
    pub stc: AES256GCMInitializer, // Server-to-client
}

impl AES256GCMInitializerPair {
    pub fn new_rand() -> Self {
        Self {
            cts: AES256GCMInitializer::new_rand(),
            stc: AES256GCMInitializer::new_rand(),
        }
    }
}

/// Wrapper for an AsyncMsgSend object that provides AES256-GCM encryption
pub struct AES256GCMMsgSender<S>
where
    S: AsyncMsgSend,
{
    sender: S,
    cipher: Aes256GcmSiv,
    nonce: AESGCMNonceCounter,
}

impl<S> AES256GCMMsgSender<S>
where
    S: AsyncMsgSend,
{
    /// Constructs a new EncryptedWriter
    pub fn new(sender: S, init: &AES256GCMInitializer) -> Self {
        Self {
            sender,
            cipher: Aes256GcmSiv::new(&GenericArray::from(init.key)),
            nonce: AESGCMNonceCounter::new(init.nonce),
        }
    }
}

impl<W> AsyncMsgSend for AES256GCMMsgSender<W>
where
    W: AsyncMsgSend,
{
    async fn send(&mut self, msg: &[u8]) -> io::Result<()> {
        let nonce = self.nonce.next();

        // Encrypt message
        let ciphertext = self
            .cipher
            .encrypt(&GenericArray::from(nonce), msg)
            .expect("encryption error");

        // Send message
        self.sender.send(&ciphertext).await
    }
}
/// Wrapper for an AsyncMsgRecv object that provides AES256-GCM encryption
pub struct AES256GCMMsgReceiver<R>
where
    R: AsyncMsgRecv,
{
    receiver: R,
    cipher: Aes256GcmSiv,
    nonce: AESGCMNonceCounter,
}

impl<R> AES256GCMMsgReceiver<R>
where
    R: AsyncMsgRecv,
{
    /// Constructs a new EncryptedWriter
    pub fn new(receiver: R, init: &AES256GCMInitializer) -> Self {
        Self {
            receiver,
            cipher: Aes256GcmSiv::new(&GenericArray::from(init.key)),
            nonce: AESGCMNonceCounter::new(init.nonce),
        }
    }
}

impl<R> AsyncMsgRecv for AES256GCMMsgReceiver<R>
where
    R: AsyncMsgRecv,
{
    async fn recv(&mut self) -> io::Result<Vec<u8>> {
        // Receive message from channel
        let ciphertext = self.receiver.recv().await?;

        let nonce = self.nonce.next();

        // Decrypt message
        self.cipher
            .decrypt(&GenericArray::from(nonce), ciphertext.as_ref())
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "decryption error"))
    }
}

/// Iterator-like type over a stream of nonces which get incremented at
/// every use
pub struct AESGCMNonceCounter {
    nonce: [u8; 12],
}

impl AESGCMNonceCounter {
    /// Constructs a nonce counter with random starting nonce
    pub fn new(init: [u8; 12]) -> Self {
        Self { nonce: init }
    }

    /// Gets the next nonce in the counter
    fn next(&mut self) -> [u8; 12] {
        let val = self.nonce.clone();
        inc_multibyte(&mut self.nonce);
        val
    }
}

/// Interprets bytes stored in a slice as a numeric value
/// and increments it. Wraps to zero on overflow
fn inc_multibyte(num: &mut [u8]) {
    // Add from right-most byte
    for byte in num.iter_mut().rev() {
        if *byte == u8::MAX {
            *byte = 0;
        } else {
            *byte += 1;
            break;
        }
    }
}

/// Represents an RSA private and public key pair
pub struct RsaKeyPair {
    pub private: RsaPrivateKey,
    pub public: RsaPublicKey,
}

impl RsaKeyPair {
    pub fn generate() -> Result<Self, ()> {
        let private = RsaPrivateKey::new(&mut OsRng, 2048).map_err(|_| ())?;
        Ok(Self {
            public: RsaPublicKey::from(&private),
            private,
        })
    }
}

/// Storage for trusted server public keys
pub struct ServerPublicKeyValidator {
    key: Option<RsaPublicKey>,
}

impl ServerPublicKeyValidator {
    /// Constructs a new TrustedServerKeyStore
    pub fn new() -> Self {
        Self { key: None }
    }

    /// Check if key is trusted
    pub fn validate(&mut self, key: &RsaPublicKey) -> Result<(), EncChannelSetupError> {
        if let Some(k) = &self.key {
            if key == k {
                Ok(())
            } else {
                Err(EncChannelSetupError::ServerPublicKeyChanged)
            }
        } else {
            // First connection, trust key
            self.key = Some(key.clone());
            Ok(())
        }
    }
}

// Encrypted channel setup error
#[derive(Debug)]
pub enum EncChannelSetupError {
    ServerPublicKeyChanged,
    Timeout,
    IoError(io::Error),
}

impl From<io::Error> for EncChannelSetupError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<Elapsed> for EncChannelSetupError {
    fn from(_: Elapsed) -> Self {
        Self::Timeout
    }
}

impl Display for EncChannelSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            &EncChannelSetupError::ServerPublicKeyChanged => {
                write!(f, "server public key changed!")
            }
            &EncChannelSetupError::Timeout => {
                write!(f, "timeout error")
            }
            &EncChannelSetupError::IoError(err) => err.fmt(f),
        }
    }
}

/// Encrypted channel setup result
pub type EncChannelSetupResult<S, R> =
    Result<(AES256GCMMsgSender<S>, AES256GCMMsgReceiver<R>), EncChannelSetupError>;

/// Handles performing the initial key exchange phase and constructing an encrypted message channel
/// on the client side
/// TODO: implement first-use key trusting
pub async fn client_setup_encrypted_channel<S, R>(
    mut sender: S,
    mut receiver: R,
    timeout: Duration,
    key_validator: &mut ServerPublicKeyValidator,
) -> EncChannelSetupResult<S, R>
where
    S: AsyncMsgSend,
    R: AsyncMsgRecv,
{
    // Generate new symmetric encryption initializers
    let sym_init = AES256GCMInitializerPair::new_rand();

    // Wait for the server's public key
    let pub_key_bytes = time::timeout(timeout, receiver.recv()).await??;
    let pub_key = RsaPublicKey::from_pkcs1_der(&pub_key_bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid public key"))?;

    // Check server public key
    key_validator.validate(&pub_key)?;

    // Serialize, encrypt with public key and send symmetric encryption initializers
    let sym_init_bytes = rkyv::to_bytes::<_, 128>(&sym_init)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "symmetric key serialization error"))?;
    let sym_init_bytes_enc = pub_key
        .encrypt(&mut OsRng, Pkcs1v15Encrypt, &sym_init_bytes)
        .map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "symmetric key encryption error")
        })?;
    sender.send(&sym_init_bytes_enc).await?;

    // We have enstablished an encrypted channel to the server
    Ok((
        AES256GCMMsgSender::new(sender, &sym_init.cts),
        AES256GCMMsgReceiver::new(receiver, &sym_init.stc),
    ))
}

/// Handles performing the initial key exchange phase and constructing an encrypted message channel
/// on the server side
pub async fn server_setup_encrypted_channel<S, R>(
    mut sender: S,
    mut receiver: R,
    keypair: &RsaKeyPair,
    timeout: Duration,
) -> EncChannelSetupResult<S, R>
where
    S: AsyncMsgSend,
    R: AsyncMsgRecv,
{
    // Send public key to client
    let pub_key_der = keypair
        .public
        .to_pkcs1_der()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "public key serialization error"))?;
    sender.send(pub_key_der.as_bytes()).await?;

    // Wait for symmetric key from client, decrypt and deserialize
    let sym_init_bytes = time::timeout(timeout, receiver.recv()).await??;
    let sym_init_bytes = keypair
        .private
        .decrypt(Pkcs1v15Encrypt, &sym_init_bytes)
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "symmetric key initializer decryption error",
            )
        })?;

    let sym_init = rkyv::from_bytes::<AES256GCMInitializerPair>(&sym_init_bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid symmetric key initializer",
        )
    })?;

    // We have enstablished an encrypted channel to the server
    Ok((
        AES256GCMMsgSender::new(sender, &sym_init.stc),
        AES256GCMMsgReceiver::new(receiver, &sym_init.cts),
    ))
}

#[cfg(test)]
mod tests {
    use rsa::BigUint;

    use super::*;

    #[test]
    fn test_inc_multibyte() {
        let tests = [
            (vec![0x10], vec![0x11]),
            (vec![0xFF], vec![0x00]),
            (vec![0x00, 0x00], vec![0x00, 0x01]),
            (vec![0x00, 0xFF], vec![0x01, 0x00]),
            (vec![0xFF, 0xFF], vec![0x00, 0x00]),
            (
                vec![0xDA; 12],
                vec![
                    0xDA, 0xDA, 0xDA, 0xDA, 0xDA, 0xDA, 0xDA, 0xDA, 0xDA, 0xDA, 0xDA, 0xDB,
                ],
            ),
        ];

        for (val, exp) in tests {
            let mut val = val;
            inc_multibyte(&mut val);
            assert_eq!(val, exp);
        }
    }

    #[test]
    fn server_key_validation() {
        let mut key_validator = ServerPublicKeyValidator::new();

        let key1 = RsaPrivateKey::from_p_q(
            BigUint::from_bytes_be(&vec![0x02]),
            BigUint::from_bytes_be(&vec![0x03]),
            BigUint::from_bytes_be(&vec![0x01]),
        )
        .unwrap();
        let key1 = RsaPublicKey::from(key1);

        let key2 = RsaPrivateKey::from_p_q(
            BigUint::from_bytes_be(&vec![0x05]),
            BigUint::from_bytes_be(&vec![0x07]),
            BigUint::from_bytes_be(&vec![0x01]),
        )
        .unwrap();
        let key2 = RsaPublicKey::from(key2);

        assert_eq!(key1, key1);
        assert_ne!(key1, key2);

        // First time validation
        key_validator.validate(&key1).unwrap();

        // Second validation with correct key
        key_validator.validate(&key1).unwrap();

        // Validation with incorrect key
        key_validator.validate(&key2).unwrap_err();
    }
}
