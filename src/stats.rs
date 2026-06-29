use std::fs;

use crate::pipeline::{output_path, stats_path};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
}

#[derive(Debug, Clone)]
pub struct StageMetrics {
    pub stage: usize,
    pub command: String,
    pub total_lines: usize,
    pub total_bytes: usize,
    pub lines_per_second: f64,
    pub bytes_per_second: f64,
    pub status: StageStatus,
}

impl StageMetrics {
    pub fn pending(stage: usize, command: String) -> Self {
        Self {
            stage,
            command,
            total_lines: 0,
            total_bytes: 0,
            lines_per_second: 0.0,
            bytes_per_second: 0.0,
            status: StageStatus::Pending,
        }
    }
}

pub fn collect_all(run_id: u32, num_stages: usize, commands: &[String]) -> Vec<StageMetrics> {
    let mut results = Vec::new();
    for inx in 0..num_stages {
        let stage_num = (inx + 1) as u32;
        let command = commands.get(inx).cloned().unwrap_or_default();
        match read_one(run_id, stage_num) {
            Some(stage) => {
                let mut stage_metrics = stage;
                stage_metrics.command = command;
                results.push(stage_metrics);
            }
            None => {
                results.push(StageMetrics::pending(stage_num as usize, command));
            }
        }
    }
    results
}

fn read_one(run_id: u32, stage: u32) -> Option<StageMetrics> {
    let path = stats_path(run_id, stage);
    let content = fs::read_to_string(&path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 4 {
        return None;
    }

    let total_lines: usize = lines[0].parse().unwrap_or(0);
    let total_bytes: usize = lines[1].parse().unwrap_or(0);
    let lines_per_second: f64 = lines[2].parse().unwrap_or(0.0);
    let bytes_per_second: f64 = lines[3].parse().unwrap_or(0.0);
    let status = if lines.len() >= 5 && lines[4] == "completed" {
        StageStatus::Completed
    } else {
        StageStatus::Running
    };

    Some(StageMetrics {
        stage: stage as usize,
        command: String::new(),
        total_lines,
        total_bytes,
        lines_per_second,
        bytes_per_second,
        status,
    })
}

pub fn read_stage_output(run_id: u32, stage: u32) -> Vec<String> {
    let path = output_path(run_id, stage);
    fs::read_to_string(&path)
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default()
}

pub fn cleanup(run_id: u32, num_stages: usize) {
    for i in 1..=num_stages as u32 {
        let _ = fs::remove_file(&stats_path(run_id, i));
        let _ = fs::remove_file(&output_path(run_id, i));
    }
}
