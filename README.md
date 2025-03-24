# mempool-rs

A command-line tool for parsing Bitcoin Core's mempool.dat files.

These files are written by Bitcoin Core on shutdown and with the RPC `savemempool`.

## Features

- Parse both V1 and V2 mempool.dat formats
- Automatic detection and handling of XOR encryption in V2 format files
- View header info (version, transaction count)
- View transaction details
- Optional interactive TUI with vim-like navigation and search
    - filter by TxID or WTxID

## mempool.dat Format

mempool.dat files can come in two formats:

- **V1 format**: [version (u64)] → [tx count (u64)] → [transactions]
- **V2 format**: [version (u64)] → [xor key size (u8)] → [xor key (key size bytes)] → [tx count (u64)] → [transactions]

The main difference between V1 and V2 is that V2 includes an [XOR key](https://github.com/bitcoin/bitcoin/pull/28207/) which is used to decrypt the remainder of the file.

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

# Display using more compact transaction output (single line)
mempool-rs -f /path/to/mempool.dat decode --compact

# Use the TUI
mempool-rs -f /path/to/mempool.dat interact
```

## Interactive TUI

Interactively explore the mempool dump with vim-style navigation.
It features a two-panel layout:

- Left: Transaction list by (w)txid with search
- Right: Transaction inspector

### TUI Navigation

The TUI uses vim-like modal editing with Normal and Insert modes:

#### Normal Mode Commands

| Key | Function |
|-----|----------|
| `q` | Quit the application |
| `Tab` | Switch focus between transaction list and transaction details |
| `i` | Enter Insert mode (for searching) |
| `j` or `Down` | Navigate down in the transaction list or scroll details |
| `k` or `Up` | Navigate up in the transaction list or scroll details |
| `PgDn` or `f` | Jump down 10 entries |
| `PgUp` or `b` | Jump up 10 entries |
| `gg` | Go to the top of the transaction list |
| `G` | Go to the bottom of the transaction list |
| `m` | Toggle between TxID and WTxID search/display |
| `c` | Clear the current search |
| `h` | Show mempool header information popup |
| `Esc` | Close popup or return focus to transaction list |

#### Insert Mode Commands

| Key | Function |
|-----|----------|
| Any character | Type to search for transactions by TxID/WTxID |
| `Backspace` | Delete character from search |
| `Esc` | Return to Normal mode |

The search performs an exact substring match on either TxID or WTxID depending on the current mode.

## TODO

- Complete implementation of mapDeltas parsing
- Add more transaction information display options
- Support for exporting specific transactions
