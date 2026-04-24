use clap::Parser;
use lnsocket::{
    bitcoin::secp256k1::{rand, PublicKey, SecretKey},
    CommandoClient, LNSocket,
};
use serde_json::Value;
use std::str::FromStr;
use anyhow::{Context, Result};

#[derive(Parser, Debug)]
#[command(name = "rdr")]
#[command(about = "CLN-RADAR: Tactical Node Uplink", long_about = None)]
struct Args {
    /// Remote CLN node pubkey (33-byte compressed hex)
    #[arg(long)]
    node: String,

    /// Remote address, e.g. example.com:9735 or abcdef.onion:9735
    #[arg(long)]
    addr: String,

    /// Commando rune
    #[arg(long)]
    rune: String,

    /// RPC method, e.g. getinfo
    method: String,

    /// JSON params object, e.g. '{}' or '{"id":"..."}'
    #[arg(default_value = "{}")]
    params: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let remote_pubkey = PublicKey::from_str(&args.node).with_context(|| format!("node={} is not a valid public key.", &args.node))?;
    let params: Value = serde_json::from_str(&args.params)?;

    // Ephemeral local key for the LN transport session.
    let local_key = SecretKey::new(&mut rand::thread_rng());

    let sock = LNSocket::connect_and_init(local_key, remote_pubkey, &args.addr).await.context("failed to connect to remote node.");
    let client = CommandoClient::spawn(sock, args.rune);

    let result = client.call(args.method, params).await.with_context(|| format!("failed to call {} with params {}", &args.method, &params));
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
