use std::{str::from_utf8, time::Duration};

use pomegranate::comm::{
    crypto::{server_setup_encrypted_channel, RsaKeyPair},
    encaps::{AsyncMsgRecv, AsyncMsgSend, LenU64EncapsMsgReceiver, LenU64EncapsMsgSender},
};
use tokio::{net::TcpListener, time};

const PORT: u16 = 1234;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Generate public asymmetric key pair
    println!("Generating RSA key pair...");
    let keypair = RsaKeyPair::generate().unwrap();

    // Start listening
    let listener = TcpListener::bind(("0.0.0.0", PORT)).await.unwrap();
    println!("Listening on port {}", PORT);

    // Listen for connection
    let (mut socket, addr) = listener.accept().await.unwrap();
    println!("New connection from {}", addr);
    let (reader, writer) = socket.split();
    let sender = LenU64EncapsMsgSender::new(writer);
    let receiver = LenU64EncapsMsgReceiver::new(reader);

    // Enstablish a secure channel
    let (mut sender, mut receiver) =
        server_setup_encrypted_channel(sender, receiver, &keypair, Duration::from_millis(1000))
            .await
            .unwrap_or_else(|err| {
                println!("Unable to enstablish encyprted channel: {}", err);
                std::process::exit(1);
            });

    println!("Encrypted channel enstablished!");

    for i in 0..1000 {
        sender
            .send(format!("Hello from server, {}", i).as_bytes())
            .await
            .unwrap();

        time::sleep(Duration::from_millis(1000)).await;
    }
    // sender.send("Hello from server 2".as_bytes()).await.unwrap();

    // let msg = receiver.recv().await.unwrap();
    // println!("Message from client: {}", from_utf8(&msg).unwrap());

    // let msg = receiver.recv().await.unwrap();
    // println!("Message from client: {}", from_utf8(&msg).unwrap());
}
