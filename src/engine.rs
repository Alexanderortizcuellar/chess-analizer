use tokio::process::{Command, Child};
use tokio::io::{BufReader, AsyncBufReadExt, AsyncWriteExt};
use std::process::Stdio;
use crate::model::{Evaluation, EngineMetadata};

pub struct EngineConfig {
    pub path: String,
    pub threads: u32,
    pub hash_mb: u32,
}

pub struct EngineInstance {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_reader: BufReader<tokio::process::ChildStdout>,
}

impl EngineInstance {
    pub async fn spawn(config: &EngineConfig) -> Result<Self, String> {
        let mut child = Command::new(&config.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn Stockfish (path: {}): {}", config.path, e))?;

        let stdin = child.stdin.take().ok_or("Failed to open engine stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open engine stdout")?;
        let stdout_reader = BufReader::new(stdout);

        let mut instance = Self {
            child,
            stdin,
            stdout_reader,
        };

        // Initialize UCI
        instance.send_line("uci").await?;
        instance.wait_for_line("uciok").await?;

        // Configure threads and hash
        instance.send_line(&format!("setoption name Threads value {}", config.threads)).await?;
        instance.send_line(&format!("setoption name Hash value {}", config.hash_mb)).await?;
        instance.send_line("isready").await?;
        instance.wait_for_line("readyok").await?;

        Ok(instance)
    }

    async fn send_line(&mut self, line: &str) -> Result<(), String> {
        self.stdin.write_all(format!("{}\n", line).as_bytes()).await
            .map_err(|e| format!("Failed to write to engine: {}", e))?;
        self.stdin.flush().await
            .map_err(|e| format!("Failed to flush engine stdin: {}", e))?;
        Ok(())
    }

    async fn wait_for_line(&mut self, expected: &str) -> Result<String, String> {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = self.stdout_reader.read_line(&mut line).await
                .map_err(|e| format!("Failed to read from engine: {}", e))?;
            if bytes_read == 0 {
                return Err("Engine process terminated prematurely".to_string());
            }
            if line.trim() == expected {
                return Ok(line);
            }
        }
    }

    pub async fn analyze_position(&mut self, fen: &str, depth: u32) -> Result<(Evaluation, EngineMetadata), String> {
        // Send FEN position
        self.send_line(&format!("position fen {}", fen)).await?;
        self.send_line(&format!("go depth {}", depth)).await?;

        // Parse FEN to see who is the active player ('w' or 'b')
        // Standard FEN: [Board] [ActivePlayer] [Castling] [EnPassant] [Halfmove] [Fullmove]
        let active_player = fen.split_whitespace()
            .nth(1)
            .and_then(|s| s.chars().next())
            .unwrap_or('w');

        let mut current_eval = Evaluation::Centipawns(0);
        let mut current_best = String::new();
        let mut current_pv = Vec::new();
        let mut current_nodes = None;

        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = self.stdout_reader.read_line(&mut line).await
                .map_err(|e| format!("Failed to read from engine: {}", e))?;
            if bytes_read == 0 {
                return Err("Engine process terminated prematurely during analysis".to_string());
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            if parts[0] == "bestmove" {
                if parts.len() > 1 {
                    current_best = parts[1].to_string();
                }
                break;
            }

            if parts[0] == "info" {


                // Parse score
                if let Some(score_idx) = parts.iter().position(|&x| x == "score") {
                    if score_idx + 2 < parts.len() {
                        let score_type = parts[score_idx + 1];
                        let score_val = parts[score_idx + 2].parse::<i32>().ok();

                        if let Some(val) = score_val {
                            let rel_eval = match score_type {
                                "cp" => Evaluation::Centipawns(val),
                                "mate" => Evaluation::Mate(val),
                                _ => continue,
                            };

                            // Convert relative evaluation to absolute (White's perspective)
                            current_eval = if active_player == 'b' {
                                match rel_eval {
                                    Evaluation::Centipawns(cp) => Evaluation::Centipawns(-cp),
                                    Evaluation::Mate(m) => {
                                        if m == 0 {
                                            Evaluation::Mate(0) // Black is checkmated (White wins)
                                        } else {
                                            Evaluation::Mate(-m)
                                        }
                                    }
                                }
                            } else {
                                match rel_eval {
                                    Evaluation::Centipawns(cp) => Evaluation::Centipawns(cp),
                                    Evaluation::Mate(m) => {
                                        if m == 0 {
                                            Evaluation::Mate(-1) // White is checkmated (Black wins)
                                        } else {
                                            Evaluation::Mate(m)
                                        }
                                    }
                                }
                            };
                        }
                    }
                }

                // Parse nodes
                if let Some(nodes_idx) = parts.iter().position(|&x| x == "nodes") {
                    if nodes_idx + 1 < parts.len() {
                        current_nodes = parts[nodes_idx + 1].parse::<u64>().ok();
                    }
                }

                // Parse PV (everything after "pv")
                if let Some(pv_idx) = parts.iter().position(|&x| x == "pv") {
                    let mut pv = Vec::new();
                    for &p in &parts[pv_idx + 1..] {
                        pv.push(p.to_string());
                    }
                    if !pv.is_empty() {
                        current_pv = pv;
                    }
                }
            }
        }

        if current_best.is_empty() && !current_pv.is_empty() {
            current_best = current_pv[0].clone();
        }

        let meta = EngineMetadata {
            best_move: current_best,
            principal_variation: current_pv,
            depth,
            nodes: current_nodes,
        };

        Ok((current_eval, meta))
    }

    pub async fn shutdown(mut self) {
        let _ = self.send_line("quit").await;
        let _ = self.child.wait().await;
    }
}
