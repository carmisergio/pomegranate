use std::io::stderr;

use log::Level;
use pomegranate::{client::ClusterClient, config::ClusterClientConfig};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Initialize logging to stderr
    stderrlog::new()
        .verbosity(Level::Debug)
        .init()
        .expect("log initialization");

    let cclient_conf = ClusterClientConfig::new("127.0.0.1:1234").bypass_pk_check(false);
    let cclient = ClusterClient::new(cclient_conf);

    cclient.run().await;
}
