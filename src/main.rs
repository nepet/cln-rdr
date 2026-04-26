use anyhow::{Context, Error, Result};
use clap::Parser;
use lnsocket::{
    CommandoClient, LNSocket,
    bitcoin::secp256k1::{PublicKey, SecretKey, rand},
};
use serde_json::{Map, Value};
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(name = "rdr")]
#[command(about = "CLN-RADAR: Tactical Node Uplink", long_about = None)]
#[command(after_help = "\
Examples:
  rdr -R AUTH 02abc...@cln.example.com:9735 getinfo
  rdr -R AUTH 02abc...@cln.example.com:9735 -k showrunes rune=xyz
  rdr -R AUTH 02abc...@cln.example.com:9735 showrunes --params-json '{\"rune\":\"xyz\"}'
")]
pub struct Args {
    /// Connection target in the form <nodeid@host:port>
    #[arg(value_name = "NODEID@HOST:PORT")]
    pub connect: ConnectInfo,

    /// RPC method name
    pub method: String,

    /// Full JSON params payload, passed through as-is
    #[arg(
            long = "params-json",
            value_name = "JSON",
            conflicts_with_all(["named", "text", "strict_json", "params"])
        )]
    pub params_json: Option<String>,

    /// Positional params, or key=value pairs with -k
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        conflicts_with = "params_json"
    )]
    pub params: Vec<String>,

    /// Commando auth rune
    #[arg(
        short = 'R',
        long = "auth",
        env = "CLN_COMMANDO_RUNE",
        hide_env_values = true
    )]
    pub auth: String,

    /// Treat params as key=value pairs
    #[arg(short = 'k', long = "named")]
    pub named: bool,

    /// Treat every param value as plain text
    #[arg(long)]
    pub text: bool,

    /// Require every param value to be valid JSON
    #[arg(long, conflicts_with = "text")]
    pub strict_json: bool,
}

#[derive(Debug, Clone)]
pub struct ConnectInfo {
    pub node_id: PublicKey,
    pub addr: String,
}

impl FromStr for ConnectInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (node, addr) = s.split_once('@').ok_or_else(|| {
            "invalid CONNECT: expected <nodeid@host:port>, for example 02abc...@example.com:9735"
                .to_owned()
        })?;

        if node.is_empty() {
            return Err("invalid CONNECT: missing node id before '@'".to_owned());
        }

        if addr.is_empty() {
            return Err("invalid CONNECT: missing host:port after '@'".to_owned());
        }

        if !addr.contains(':') {
            return Err("invalid CONNECT: expected host:port after '@'".to_owned());
        }

        let node_id = PublicKey::from_str(node)
            .map_err(|e| format!("invalid CONNECT: bad node pubkey: {e}"))?;

        Ok(ConnectInfo {
            node_id,
            addr: addr.to_owned(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum ParamMode {
    Auto,
    Text,
    StrictJson,
}

impl Args {
    fn param_mode(&self) -> ParamMode {
        if self.text {
            ParamMode::Text
        } else if self.strict_json {
            ParamMode::StrictJson
        } else {
            ParamMode::Auto
        }
    }
}

fn parse_value(s: &str, mode: ParamMode) -> Result<Value, String> {
    match mode {
        ParamMode::Text => Ok(Value::String(s.to_owned())),
        ParamMode::Auto => {
            Ok(serde_json::from_str::<Value>(s).unwrap_or_else(|_| Value::String(s.to_owned())))
        }
        ParamMode::StrictJson => {
            serde_json::from_str::<Value>(s).map_err(|e| format!("invalid JSON value `{s}`: {e}"))
        }
    }
}

fn parse_params(
    params_json: Option<&str>,
    force_named: bool,
    mode: ParamMode,
    raw: &[String],
) -> Result<Value, String> {
    if let Some(json) = params_json {
        return serde_json::from_str::<Value>(json)
            .map_err(|e| format!("invalid JSON for --params-json: {e}"));
    }

    let named = force_named || raw.first().is_some_and(|s| s.contains('='));

    if named {
        let mut obj = Map::new();

        for item in raw {
            let (k, v) = item
                .split_once('=')
                .ok_or_else(|| format!("expected key=value, got `{item}`"))?;

            if k.is_empty() {
                return Err(format!("empty key in `{item}`"));
            }

            obj.insert(k.to_owned(), parse_value(v, mode)?);
        }

        Ok(Value::Object(obj))
    } else {
        raw.iter()
            .map(|s| parse_value(s, mode))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array)
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("error: {:#}", err);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();

    let target = format!("{}@{}", args.connect.node_id, args.connect.addr);
    let method = args.method.clone();

    let params = parse_params(
        args.params_json.as_deref(),
        args.named,
        args.param_mode(),
        &args.params,
    )
    .map_err(Error::msg)
    .with_context(|| format!("invalid parameters for RPC `{method}`"))?;

    let local_key = SecretKey::new(&mut rand::thread_rng());

    let sock = LNSocket::connect_and_init(local_key, args.connect.node_id, &args.connect.addr)
        .await
        .map_err(Error::msg)
        .with_context(|| format!("failed to connect to remote node `{target}`"))?;

    let client = CommandoClient::spawn(sock, args.auth);

    let result = client
        .call(method.clone(), params)
        .await
        .map_err(Error::msg)
        .with_context(|| format!("RPC `{method}` failed on `{target}`"))?;

    let pretty = serde_json::to_string_pretty(&result)
        .context("failed to render RPC response as pretty JSON")?;

    println!("{pretty}");
    Ok(())
}
