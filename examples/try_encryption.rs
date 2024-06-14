use aes_gcm_siv::{
    aead::{
        generic_array::{GenericArray, GenericArrayIter},
        Aead, OsRng,
    },
    Aes128GcmSiv, Aes256GcmSiv, KeyInit, Nonce,
};

fn main() {
    let message = "HI!";

    // let key = Aes128GcmSiv::generate_key(&mut OsRng);
    let key = [0u8; 16];

    let cipher = Aes128GcmSiv::new(&GenericArray::from(key));
    let nonce = Nonce::from_slice(b"1(kdj3939fhj");

    let mut ciphertext = cipher
        .encrypt(nonce, message.as_bytes().as_ref())
        .expect("Encrypt error");

    // ciphertext[2] += 1;

    println!("Cyphertext: {}", String::from_utf8_lossy(&ciphertext));

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .expect("Decrypt error");

    let message_decrypted = String::from_utf8(plaintext).expect("Wrong utf-8");

    println!("Decryption result: {}", message_decrypted);
}
