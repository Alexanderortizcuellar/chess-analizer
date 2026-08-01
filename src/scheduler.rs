use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use sysinfo::System;
use tokio::sync::mpsc;
use shakmaty::fen::Fen;
use shakmaty::{Chess, Position, EnPassantMode};
use shakmaty::zobrist::Zobrist64;
use crate::model::{Game, Evaluation};
use crate::engine::{EngineConfig, EngineInstance};
use crate::cache::{SharedPositionCache, normalize_fen};

pub struct SchedulerConfig {
    pub engine_path: String,
    pub depth: u32,
    pub num_processes: String, // "auto" or integer string
    pub threads_per_process: Option<u32>,
    pub hash_mb_per_process: Option<u32>,
    pub movetime_sec: u32,
}

#[derive(Debug, Clone)]
pub struct AnalysisJob {
    pub fen: String,
    pub hash: Zobrist64,
}

pub struct ResourceProfile {
    pub num_processes: u32,
    pub threads_per_process: u32,
    pub hash_mb: u32,
}

pub fn determine_resources(config: &SchedulerConfig) -> ResourceProfile {
    let mut sys = System::new_all();
    sys.refresh_all();

    let num_cores = sys.cpus().len() as u32;
    let total_ram_mb = sys.total_memory() / (1024 * 1024);

    let (num_proc, threads) = match config.num_processes.to_lowercase().as_str() {
        "auto" => {
            // Auto mode: leave at least 2 cores free if possible, minimum 1 worker
            let target_threads = (num_cores - 2).max(1);
            // Default to 1 thread per process for maximum independent parallelism
            (target_threads, 1)
        }
        val => {
            let p = val.parse::<u32>().unwrap_or(1).max(1);
            let t = config.threads_per_process.unwrap_or(1).max(1);
            (p, t)
        }
    };

    let hash = match config.hash_mb_per_process {
        Some(h) => h,
        None => {
            // Reserve 2GB for OS & analyzer
            let reserved_ram = 2048;
            let available_ram = if total_ram_mb > reserved_ram {
                total_ram_mb - reserved_ram
            } else {
                512 // fallback
            };
            // Distribute remaining RAM across processes, clamping between 64MB and 1024MB
            let share = available_ram / num_proc as u64;
            share.clamp(64, 1024) as u32
        }
    };

    ResourceProfile {
        num_processes: num_proc,
        threads_per_process: threads,
        hash_mb: hash,
    }
}

pub async fn analyze_games(
    games: &mut [Game],
    config: &SchedulerConfig,
    cache: &SharedPositionCache,
) -> Result<(), String> {
    // 1. Gather all unique positions to analyze
    let mut unique_jobs = Vec::new();
    let mut seen_fens = HashSet::new();

    for game in games.iter() {
        for mv in game.moves.iter() {
            let normalized = normalize_fen(&mv.fen_before);
            if seen_fens.insert(normalized) {
                // Parse FEN to compute Zobrist hash
                if let Ok(fen) = mv.fen_before.parse::<Fen>() {
                    if let Ok(pos) = fen.into_position::<Chess>(shakmaty::CastlingMode::Standard) {
                        let hash = pos.zobrist_hash::<Zobrist64>(EnPassantMode::Legal);
                        unique_jobs.push(AnalysisJob {
                            fen: mv.fen_before.clone(),
                            hash,
                        });
                    }
                }
            }
        }
        // Also add the fen_after of the last move to analyze the final game state
        if let Some(last_mv) = game.moves.last() {
            let normalized = normalize_fen(&last_mv.fen_after);
            if seen_fens.insert(normalized) {
                if let Ok(fen) = last_mv.fen_after.parse::<Fen>() {
                    if let Ok(pos) = fen.into_position::<Chess>(shakmaty::CastlingMode::Standard) {
                        let hash = pos.zobrist_hash::<Zobrist64>(EnPassantMode::Legal);
                        unique_jobs.push(AnalysisJob {
                            fen: last_mv.fen_after.clone(),
                            hash,
                        });
                    }
                }
            }
        }
    }

    if unique_jobs.is_empty() {
        println!("No positions to analyze.");
        return Ok(());
    }

    // 2. Filter jobs that are already in the cache at the required depth
    let mut pending_jobs = Vec::new();
    for job in unique_jobs {
        if cache.lookup(job.hash, &job.fen, config.depth).is_none() {
            pending_jobs.push(job);
        }
    }

    let total_pending = pending_jobs.len();
    println!("Total unique positions: {}", seen_fens.len());
    println!("Cached positions: {}", seen_fens.len() - total_pending);
    println!("Positions needing analysis: {}", total_pending);

    if total_pending == 0 {
        println!("All positions are already cached. Skipping engine run.");
    } else {
        // 3. Determine resources
        let profile = determine_resources(config);
        println!(
            "Spawning {} Stockfish process(es) ({} thread(s), {} MB hash each)",
            profile.num_processes, profile.threads_per_process, profile.hash_mb
        );

        // 4. Setup channels and progress tracking
        let (tx, rx) = mpsc::channel(total_pending);
        for job in pending_jobs {
            let _ = tx.send(job).await;
        }
        drop(tx); // Close channel sender so workers know when done

        let rx = Arc::new(tokio::sync::Mutex::new(rx));
        let progress = Arc::new(Mutex::new(0));

        let mut workers = Vec::new();
        for i in 0..profile.num_processes {
            let rx = rx.clone();
            let progress = progress.clone();
            let cache = cache.clone();
            let engine_path = config.engine_path.clone();
            let threads = profile.threads_per_process;
            let hash_mb = profile.hash_mb;
            let depth = config.depth;
            let movetime_sec = config.movetime_sec;

            let worker = tokio::spawn(async move {
                let engine_config = EngineConfig {
                    path: engine_path,
                    threads,
                    hash_mb,
                };
                
                let mut engine = match EngineInstance::spawn(&engine_config).await {
                    Ok(eng) => eng,
                    Err(e) => {
                        eprintln!("Worker {} failed to start engine: {}", i, e);
                        return;
                    }
                };

                loop {
                    let job = {
                        let mut rx_lock = rx.lock().await;
                        rx_lock.recv().await
                    };

                    let job = match job {
                        Some(j) => j,
                        None => break, // Channel empty
                    };

                    match engine.analyze_position(&job.fen, depth, movetime_sec).await {
                        Ok((eval, meta)) => {
                            cache.insert(job.hash, job.fen, eval, meta);
                        }
                        Err(e) => {
                            eprintln!("Worker {} analysis error: {}", i, e);
                        }
                    }

                    // Update and print progress
                    let done = {
                        let mut p_lock = progress.lock().unwrap();
                        *p_lock += 1;
                        *p_lock
                    };

                    if done % 10 == 0 || done == total_pending {
                        let pct = (done as f64 / total_pending as f64) * 100.0;
                        println!("Progress: {} / {} positions analyzed ({:.1}%)", done, total_pending, pct);
                    }
                }

                engine.shutdown().await;
            });

            workers.push(worker);
        }

        // Wait for all workers to finish
        for worker in workers {
            let _ = worker.await;
        }
        println!("Analysis completed.");
    }

    // 5. Populate the game structures from the cache
    for game in games.iter_mut() {
        for mv in game.moves.iter_mut() {
            if let Ok(fen) = mv.fen_before.parse::<Fen>() {
                if let Ok(pos) = fen.into_position::<Chess>(shakmaty::CastlingMode::Standard) {
                    let hash = pos.zobrist_hash::<Zobrist64>(EnPassantMode::Legal);
                    if let Some((eval, meta)) = cache.lookup(hash, &mv.fen_before, config.depth) {
                        mv.evaluation = Some(eval);
                        mv.engine_metadata = Some(meta);
                    }
                }
            }
        }
        
        // Populate final evaluation
        if let Some(last_mv) = game.moves.last() {
            if let Ok(fen) = last_mv.fen_after.parse::<Fen>() {
                if let Ok(pos) = fen.into_position::<Chess>(shakmaty::CastlingMode::Standard) {
                    if pos.is_checkmate() {
                        if last_mv.is_white {
                            game.final_evaluation = Some(Evaluation::Mate(0)); // White checkmated Black (White wins)
                        } else {
                            game.final_evaluation = Some(Evaluation::Mate(-1)); // Black checkmated White (Black wins)
                        }
                    } else {
                        let hash = pos.zobrist_hash::<Zobrist64>(EnPassantMode::Legal);
                        if let Some((eval, _)) = cache.lookup(hash, &last_mv.fen_after, config.depth) {
                            game.final_evaluation = Some(eval);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
