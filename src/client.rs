use std::{io, ops::DivAssign, sync::mpsc::Receiver, time::Duration};

use log::{debug, error, info, warn};
use stderrlog::new;
use tokio::{io::AsyncSeek, net::TcpStream, sync::broadcast::error, time};

use crate::{
    comm::{
        crypto::{client_setup_encrypted_channel, ServerPublicKeyValidator},
        encaps::{AsyncMsgRecv, AsyncMsgSend, LenU64EncapsMsgReceiver, LenU64EncapsMsgSender},
        timer::DoublingTimer,
    },
    config::ClusterClientConfig,
};

/// Pomegranate Cluster Client
pub struct ClusterClient {
    config: ClusterClientConfig,
}

impl ClusterClient {
    /// Creates new ClusterClient
    pub fn new(config: ClusterClientConfig) -> Self {
        Self { config }
    }

    /// Run Client
    pub async fn run(&self) {
        let mut key_validator = ServerPublicKeyValidator::new(self.config.bypass_pk_check);
        let mut retry_timer =
            DoublingTimer::new(5, Duration::from_secs(1), Duration::from_secs(30));

        loop {
            debug!("Attempting connection to {}", self.config.coord_addr);
            match self.connect_to_cluster(&mut key_validator).await {
                Err(e) => {
                    let delay = retry_timer.next();
                    error!(
                        "Error connecting to cluster: {}. Retrying in {}s",
                        e,
                        delay.as_secs()
                    );
                    time::sleep(delay).await;
                }
                Ok((sender, mut receiver)) => {
                    info!("Connected!");
                    retry_timer.reset();
                    loop {
                        let msg = match receiver.recv().await {
                            Ok(msg) => msg,
                            Err(e) => {
                                error!("Connection terminated: {}", e);
                                break;
                            }
                        };

                        println!("Received message: {}", String::from_utf8_lossy(&msg));
                    }
                    // Do clustery stuff
                }
            }
        }
    }

    /// Connect to Cluster Controller and do Onboarding
    async fn connect_to_cluster(
        &self,
        key_validator: &mut ServerPublicKeyValidator,
    ) -> io::Result<(impl AsyncMsgSend, impl AsyncMsgRecv)> {
        // Connect to server
        let socket = TcpStream::connect(self.config.coord_addr).await?;
        let (reader, writer) = socket.into_split();
        let sender = LenU64EncapsMsgSender::new(writer);
        let receiver = LenU64EncapsMsgReceiver::new(reader);

        // Setup encrypted channel
        let (sender, receiver) = client_setup_encrypted_channel(
            sender,
            receiver,
            Duration::from_millis(1000),
            key_validator,
        )
        .await?;

        Ok((sender, receiver))
    }
}
