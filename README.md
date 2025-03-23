# mempool-rs

A command-line tool for parsing Bitcoin Core's mempool.dat files.

These files are written by Bitcoin Core on shutdown and with the RPC `savemempool`.

## Features

- Parse both V1 and V2 mempool.dat formats
- Automatic detection and handling of XOR encryption in V2 format files
- View header information (version, transaction count)
- View transaction details including timestamp (fee Delta TODO)

## mempool.dat Format

mempool.dat files can come in two formats:

- **V1 format**: [u64 (8 bytes)] → [tx count (u64)] → [transactions]
- **V2 format**: [u64 (8 bytes)] → [xor key size (u8)] → [xor key (key size bytes)] → [tx count (u64)] → [transactions]

The main difference between V1 and V2 is that V2 includes an XOR key after the version, which is used to decrypt the rest of the file.

## Installation

```shell
cargo install --path .
```

## Usage

```
# Show help
mempool-rs --help

# Specify a mempool.dat file path (default: mempool.dat in current dir)
mempool-rs --file /path/to/mempool.dat decode
```

### Commands

```shell
# Show only the header information (version and transaction count)
mempool-rs -f /path/to/mempool.dat header

# Decode and display transactions (default: first 10)
mempool-rs -f /path/to/mempool.dat decode -l 5

# Display compact transaction output
mempool-rs -f /path/to/mempool.dat decode --compact
```

## TODO

- Complete implementation of mapDeltas parsing
- Add transaction searching by txid
- Improve transaction display and add more output formats
