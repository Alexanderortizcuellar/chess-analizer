# Chess Analyzer

A high-performance, concurrent chess analysis and annotation toolkit written in Rust. It utilizes Stockfish via the UCI protocol to parse PGN files, perform asynchronous position analysis, classify moves based on centipawn loss, identify tactical patterns, and export annotated PGNs/JSONs.

## Features

- **High Concurrency**: Automatically distributes analysis jobs across a pool of Stockfish worker processes using Tokio.
- **De-duplication & Caching**: Employs an in-memory Zobrist transposition cache to avoid re-evaluating duplicate chess positions across games.
- **Advanced Move Classification**: Assigns quality tags (*Best Move*, *Excellent*, *Good*, *Inaccuracy*, *Mistake*, *Blunder*, *Miss*) based on centipawn loss.
- **Missed Mate Tactics**: Detects when winning mate opportunities are missed.
- **PV Variations**: Automatically exports the entire principal variation (PV) in standard PGN variation format for forced mate or forced win paths.
- **Highly Configurable**: Supports JSON config files and command-line overrides for processes, threads, hash size, search depth, and output directories.

## Installation

### Prerequisites
1. **Rust Toolchain**: Install Rust and Cargo via [rustup](https://rustup.rs/).
2. **Stockfish Engine**: Download the [Stockfish executable](https://stockfishchess.org/download/) matching your OS.

### Building
Compile the project in release mode for maximum execution speed:
```bash
cargo build --release
```

## Configuration

You can configure the analyzer using a `config.json` file in your working directory:
```json
{
  "engine": {
    "path": "C:\\Users\\ASUS\\chess\\engines\\stockfish.exe",
    "depth": 20,
    "processes": "auto",
    "threads_per_process": 1,
    "hash_mb": 1024
  },
  "output": {
    "format": ["pgn", "json"],
    "output_dir": "./output"
  }
}
```

## CLI Usage

Run the executable by specifying the PGN file(s) and any overrides:
```bash
# Using config.json settings
cargo run --release -- games.pgn

# Overriding settings for high-performance multi-core VMs (e.g., 64-core VM)
cargo run --release -- games.pgn -d 20 -p 60 -t 1 -m 1024 -o ./output
```

### Supported Arguments:
- `-c, --config <FILE>`: Path to a configuration JSON file.
- `-e, --engine <PATH>`: Path to the Stockfish engine executable.
- `-d, --depth <INT>`: Search depth.
- `-p, --processes <INT|auto>`: Number of concurrent Stockfish processes.
- `-t, --threads <INT>`: Threads per Stockfish process.
- `-m, --hash <INT>`: Hash size (MB) per Stockfish process.
- `-o, --output-dir <DIR>`: Output directory for annotated PGN/JSON files.
- `--format <FORMATS>`: Comma-separated formats (e.g., `pgn,json`).

## Move Classifications
The analyzer classifies played moves using the following centipawn loss and tactical criteria:
- **Best Move**: The played move matches the engine's top choice, or loses $\le 0.10$ pawns.
- **Excellent**: Loss $\le 0.20$ pawns.
- **Good**: Loss $\le 0.50$ pawns.
- **Inaccuracy**: Loss $\le 1.00$ pawns.
- **Mistake**: Loss $\le 2.00$ pawns.
- **Blunder**: Loss $> 2.00$ pawns, resulting in a losing position (advantage drops below $-1.00$).
- **Miss**: A missed win/tactic. The player had a winning advantage ($\ge 1.50$ pawns or forced mate), but played a move that let the opponent escape to an equal/playable position ($\ge -1.00$ advantage).
