# Chess Analyzer (Rust) – Architecture & Design Specification

## Project Goal

Design and implement a high-performance chess analysis toolkit in Rust. The project is intended to analyze personal PGN databases (typically between a few games and approximately 5,000 games), annotate them using Stockfish, classify mistakes, detect tactical opportunities, and later serve as the backend for a chess statistics/dashboard application.

The project should prioritize:

- Performance
- Clean architecture
- Concurrency
- Extensibility
- Maintainability

The implementation should avoid over-engineering while remaining easy to extend.

---

# Overall Architecture

The project should be organized into independent modules (or Cargo workspace crates).

Suggested architecture:

- analyzer-core
- engine
- scheduler
- pgn
- annotations
- tactics
- cli

Each module should have a single responsibility.

The entire application should operate on an internal analysis model rather than directly modifying PGNs.

PGNs are simply an input/output format.

---

# Pipeline 1 – Engine Analysis

Responsibilities:

- Read one or more PGN files.
- Parse every game.
- Generate every board position.
- Send positions to Stockfish.
- Receive engine evaluations.
- Store evaluations in the internal analysis model.
- Export annotated games.

This module is responsible ONLY for engine evaluation.

It should not classify mistakes or detect tactics.

---

# Pipeline 2 – Move Classification

This pipeline operates after evaluations already exist.

Responsibilities:

Classify moves as:

- Best Move
- Excellent
- Good
- Inaccuracy
- Mistake
- Blunder
- Brilliant (future)

Classification should be based on configurable evaluation thresholds.

This module should not perform engine analysis except in future optional detectors.

---

# Pipeline 3 – Tactical Detection

Implement independent tactical detectors.

Each detector should implement a common interface.

Examples:

- Missed Mate
- Missed Forced Mate
- Missed Fork
- Missed Skewer
- Missed Pin
- Missed Discovered Attack
- Hanging Pieces
- Free Pieces Not Captured
- Passed Pawn Opportunities
- Promotion Opportunities
- Only Winning Move
- Additional tactical patterns in the future

Each detector should return structured findings.

Detectors should NOT directly modify PGNs.

---

# Shared Position Feature Layer

Avoid recomputing chess information multiple times.

Create a shared feature extraction layer.

For every position compute reusable information once.

Examples:

- Attack maps
- Defended pieces
- Hanging pieces
- Pins
- Checks
- Legal moves
- King safety
- Passed pawns

Whenever possible leverage existing functionality from the Shakmaty library instead of reimplementing chess logic.

Every detector should reuse these computed features.

---

# Scheduler

The scheduler is responsible for managing Stockfish processes.

Responsibilities:

- Maintain a queue of positions
- Spawn Stockfish processes
- Distribute work
- Collect results
- Report progress
- Handle failures

Workers should process positions independently.

---

# Parallelism

Support configurable:

- Number of Stockfish processes
- Threads per Stockfish process
- Hash size (MB) per Stockfish process
- Search depth
- MultiPV (future)

Every option must be manually configurable.

The scheduler should also provide an Auto mode.

Auto mode should:

- Detect logical CPU count.
- Detect available RAM.
- Reserve resources for:
    - the operating system
    - the analyzer
    - the in-memory cache
    - Stockfish hash tables

The scheduler should intentionally avoid consuming 100% of available CPUs.

It should leave spare CPU capacity so the operating system and the analyzer remain responsive.

Likewise it should avoid exhausting available memory.

---

# In-Memory Position Cache

Because the intended workload is approximately 5,000 games or fewer, the cache should remain entirely in memory.

No persistent database cache is required.

The cache exists only during the current execution.

Purpose:

Avoid sending identical positions to Stockfish multiple times.

Many positions appear repeatedly across games because of:

- Opening theory
- Transpositions
- Common move orders

These positions should only be analyzed once.

Suggested implementation:

- Use a Zobrist hash as the primary lookup key.
- Store the canonical FEN alongside the hash to verify correctness in the extremely unlikely event of a hash collision.

The cache should store:

- Position
- Evaluation
- Best Move
- Principal Variation
- Search Depth
- Any additional engine metadata

If the cache already contains a position analyzed at depth 40 and the user requests depth 30, reuse the cached result.

If the user later requests depth 50, analyze again and replace the cached entry with the deeper result.

The cache should be shared safely among all workers.

---

# Internal Analysis Model

Everything should operate on a structured internal representation instead of directly writing to PGNs.

Suggested model:

Game
    Metadata
    Moves
    Positions
    Evaluations
    Engine Metadata
    Principal Variations
    Tactical Findings
    Move Classifications

This becomes the single source of truth.

---

# Exporters

Implement two exporters.

## PGN Exporter

Generate annotated PGNs.

Comments should follow a structured and machine-readable format.

For example:

[%eval 0.82]

[%bestmove Nf3]

[%analysis
classification=Blunder
score_loss=2.8
missed=Fork
]

The exact syntax may evolve, but it should remain easy to parse.

This allows future tools to recover analysis directly from PGNs.

---

## JSON Exporter

Export the complete structured analysis model.

The JSON should preserve every piece of information produced during analysis.

The future dashboard should be able to consume this JSON directly without reparsing PGNs.

Both exporters should operate from the same internal model.

---

# Configuration

Support both:

- Command-line arguments
- Configuration JSON file

Example:

{
  "engine": {
    "path": "stockfish",
    "depth": 35,
    "processes": "auto",
    "threads_per_process": 2,
    "hash_mb": 1024
  },
  "output": {
    "format": ["pgn", "json"]
  }
}

CLI arguments should override values from the configuration file.

---

# Design Principles

- Modular architecture.
- Strong separation of concerns.
- Thread-safe implementation.
- Efficient concurrency.
- Clean abstractions.
- Extensible detector framework.
- Minimal code duplication.
- Favor readability over clever optimizations.

---

# Future Dashboard Compatibility

Although this project only implements the backend analyzer, it should be designed so a future dashboard can easily consume its output.

Examples of future dashboard capabilities:

- Evaluation graph
- Blunder explorer
- Missed mate explorer
- Tactical motif explorer
- Free piece opportunities
- Hanging piece statistics
- Passed pawn opportunities
- Opening performance
- Classification statistics
- Move quality distributions

The analyzer should expose enough structured information to support these future features without requiring major architectural changes.

---

# Overall Objective

Build a robust, scalable, and modular Rust chess analysis toolkit capable of efficiently analyzing thousands of personal games using Stockfish. The system should maximize engine throughput through intelligent scheduling, configurable parallelism, and an in-memory position cache while producing rich structured analysis that can be exported as annotated PGNs and JSON. The architecture should make it straightforward to add new tactical detectors, exporters, analysis modules, and future dashboard integrations without requiring significant redesign.