use std::{str::from_utf8, time::Duration};

use pomegranate::comm::{
    crypto::{client_setup_encrypted_channel, ServerPublicKeyValidator},
    encaps::{AsyncMsgRecv, AsyncMsgSend, LenU64EncapsMsgReceiver, LenU64EncapsMsgSender},
};
use tokio::net::TcpStream;

const PORT: u16 = 1234;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Connect to server
    let mut socket = TcpStream::connect(("127.0.0.1", PORT)).await.unwrap();

    println!("Connected!");
    let (reader, writer) = socket.split();
    let sender = LenU64EncapsMsgSender::new(writer);
    let receiver = LenU64EncapsMsgReceiver::new(reader);

    // Enstablish a secure channel
    let mut key_validator = ServerPublicKeyValidator::new();
    let (mut sender, mut receiver) = client_setup_encrypted_channel(
        sender,
        receiver,
        Duration::from_millis(1000),
        &mut key_validator,
    )
    .await
    .unwrap();

    println!("Encrypted channel enstablished!");

    sender.send("Hello from client".as_bytes()).await.unwrap();
    sender.send("Hello from client 2".as_bytes()).await.unwrap();

    let msg = receiver.recv().await.unwrap();
    println!("Message from server: {}", from_utf8(&msg).unwrap());

    let msg = receiver.recv().await.unwrap();
    println!("Message from server: {}", from_utf8(&msg).unwrap());
}
