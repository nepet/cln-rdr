# rdr

**CLN-RADAR: Tactical Node Uplink**

`rdr` is a small command-line client for calling remote [Core Lightning](https://github.com/ElementsProject/lightning) RPC methods over the Lightning transport using `lnsocket` and Commando.

It is designed around three explicit ways to supply RPC parameters:

- positional parameters → JSON array
- `-k` / `--named` parameters → JSON object from `key=value`
- `--params-json` → pass the full JSON payload through as-is

## Features

- Connect to a remote Core Lightning node using `NODEID@HOST:PORT`
- Authenticate with a Commando rune via `-R, --auth` or `CLN_COMMANDO_RUNE`
- Call any RPC method supported by the remote node
- Support positional, named, and raw JSON parameter input
- Produce readable step-specific errors for parameter parsing, transport setup, and RPC failures

## Requirements

- Rust and Cargo
- Access to a remote Core Lightning node
- A valid Commando rune for that node
- Network reachability to the node's Lightning port

## Installation

### From crates.io

```bash
cargo install cln-rdr
```

Cargo installs binaries into Cargo's bin directory, which is typically:

```text
$HOME/.cargo/bin
```

Make sure that directory is on your `PATH`.

After installation, you can run:

```bash
rdr --help
```

### From source

```bash
cargo install --path .
```

or build a release binary directly:

```bash
cargo build --release
```

The resulting binary will be at:

```bash
./target/release/rdr
```

## Usage

```text
rdr [OPTIONS] <NODEID@HOST:PORT> <METHOD> [PARAMS...]
```

### Positional arguments

`<NODEID@HOST:PORT>`
: Remote node public key and address in a single value.

`<METHOD>`
: RPC method name, for example `getinfo`, `listfunds`, or `showrunes`.

`[PARAMS...]`
: Positional RPC parameters. By default these are converted into a JSON array unless `-k` or `--params-json` is used.

### Options

`-R, --auth <RUNE>`
: Commando authentication rune. Can also be supplied through `CLN_COMMANDO_RUNE`.

`-k, --named`
: Treat trailing parameters as `key=value` pairs and build a JSON object.

`--text`
: Treat every parameter value as plain text.

`--strict-json`
: Require every parameter value to be valid JSON.

`--params-json <JSON>`
: Pass a complete JSON params payload as-is. This conflicts with `-k`, `--text`, `--strict-json`, and positional params.

## Connection format

The first argument must be in this form:

```text
NODEID@HOST:PORT
```

Example:

```bash
rdr -R AUTH 02abc...@cln.example.com:9735 getinfo
```

If the value is malformed, `rdr` rejects it before making any network call.

## Authentication

You can supply the Commando rune directly:

```bash
rdr -R AUTH 02abc...@cln.example.com:9735 getinfo
```

Or set it once in your environment:

```bash
export CLN_COMMANDO_RUNE=AUTH
rdr 02abc...@cln.example.com:9735 getinfo
```

Using the environment variable is usually the better day-to-day workflow because it keeps the command shorter and avoids repeating the rune in shell history.

## Parameter handling

`rdr` supports three explicit parameter styles.

### 1. Positional parameters

Without `-k`, trailing arguments are collected into a JSON array.

```bash
rdr -R AUTH 02abc...@cln.example.com:9735 somecmd 1 true hello
```

Default behavior is **smart parsing**:

- valid JSON values are parsed as JSON
- everything else is treated as a string

So the example above becomes:

```json
[1, true, "hello"]
```

### 2. Named parameters with `-k`

With `-k`, trailing arguments must be `key=value` pairs and are converted into a JSON object.

```bash
rdr -R AUTH 02abc...@cln.example.com:9735 -k showrunes rune=xyz
```

This becomes:

```json
{"rune": "xyz"}
```

This mode is the closest match to `lightning-cli -k`.

### 3. Full JSON with `--params-json`

If you want exact control over the payload, pass the whole JSON object or array explicitly:

```bash
rdr -R AUTH 02abc...@cln.example.com:9735 showrunes --params-json '{"rune":"xyz"}'
```

This is the best option when you already have a JSON payload or want to avoid shell parsing ambiguity.

## `--text` vs `--strict-json`

### Default behavior

By default, `rdr` uses smart parsing:

- `3` becomes JSON number `3`
- `true` becomes JSON boolean `true`
- `{"a":1}` becomes a JSON object
- `hello` stays the string `"hello"`

### `--text`

Force every value to be treated as plain text.

```bash
rdr -R AUTH --text 02abc...@cln.example.com:9735 -k somecmd count=3 active=true
```

This becomes:

```json
{"count":"3","active":"true"}
```

### `--strict-json`

Require every value to already be valid JSON.

```bash
rdr -R AUTH --strict-json 02abc...@cln.example.com:9735 -k somecmd count=3 label='"hello"'
```

This becomes:

```json
{"count":3,"label":"hello"}
```

Bare `label=hello` would fail in `--strict-json` mode because `hello` is not valid JSON.

## Examples

### No parameters

```bash
rdr -R AUTH 02abc...@cln.example.com:9735 getinfo
```

### Named parameters

```bash
rdr -R AUTH 02abc...@cln.example.com:9735 -k showrunes rune=xyz
```

### Full JSON parameters

```bash
rdr -R AUTH 02abc...@cln.example.com:9735 showrunes --params-json '{"rune":"xyz"}'
```

### Use environment auth

```bash
export CLN_COMMANDO_RUNE=AUTH
rdr 02abc...@cln.example.com:9735 listfunds
```

### Force text values

```bash
rdr -R AUTH --text 02abc...@cln.example.com:9735 -k somecmd note=hello count=3
```

### Enforce strict JSON values

```bash
rdr -R AUTH --strict-json 02abc...@cln.example.com:9735 -k somecmd enabled=true retries=3
```

## Packaging notes

Publishing to crates.io is the primary distribution path for `rdr`. For Rust users, `cargo install cln-rdr` is the lowest-friction way to install and update the tool.

Homebrew can also make sense if you expect non-Rust macOS or Linux users who already install CLIs with `brew`, but it is best treated as a secondary distribution channel after crates.io. In practice, that usually means maintaining a small custom tap rather than blocking the initial release on Homebrew packaging.

## Error behavior

`rdr` tries to fail with context that identifies the stage that broke:

- invalid parameters for an RPC
- failed to connect to the remote node
- RPC failure on the remote node
- JSON rendering failures for the final output

Examples of rejected input include:

- malformed `NODEID@HOST:PORT`
- `-k` arguments that are missing `=`
- invalid JSON with `--strict-json`
- invalid JSON in `--params-json`

## Output

Successful responses are printed as pretty-formatted JSON to standard output.

This makes `rdr` easy to use interactively and easy to pipe into tools like `jq`.

## Security notes

- Treat Commando runes like credentials.
- Prefer `CLN_COMMANDO_RUNE` over putting the rune directly in shell history when possible.
- Be careful when using `--params-json` or shell-quoted parameters in shared terminals and logs.

## License

MIT. See [LICENSE](./LICENSE).
