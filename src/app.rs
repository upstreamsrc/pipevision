use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::PathBuf,
    process::{Child, ExitStatus},
    time::Instant,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    pipeline::Pipeline,
    stats::{self, StageMetrics},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppStatus {
    Idle,      // 0
    Running,   // 1
    Completed, // 2
}

pub struct App {
    pub pipeline: Option<Pipeline>,
    pub pipeline_file: Option<String>,

    pub metrics: Vec<StageMetrics>,
    pub stage_output: Vec<String>,
    pub output_scroll: usize,
    pub run_id: u32,
    pub status: AppStatus,
    pub selected: usize,
    pub should_exit: bool,

    pub status_message: String,
    pub error_message: Option<String>,

    pub start_time: Option<Instant>,
    pub elapsed_seconds: f64,
    pub exit_status: Option<i32>,

    pub file_input_active: bool,
    pub file_input_buffer: String,
    pub file_suggestions: Vec<String>,
    pub selected_suggestion: usize,

    all_files_cache: Vec<String>,
    child: Option<Child>,
}

impl App {
    pub fn new(file_path: Option<&str>) -> io::Result<Self> {
        let mut app: App = Self {
            pipeline: None,
            pipeline_file: None,

            metrics: Vec::new(),
            stage_output: Vec::new(),
            output_scroll: 0,
            run_id: 0,
            status: AppStatus::Idle,
            selected: 0,
            should_exit: false,
            status_message: "> Press [L] to load a pipeline file!".to_string(),
            error_message: None,
            start_time: None,
            elapsed_seconds: 0.0,
            exit_status: None,

            file_input_active: false,
            file_input_buffer: String::new(),
            file_suggestions: Vec::new(),

            selected_suggestion: 0,
            all_files_cache: Vec::new(),
            child: None,
        };

        if let Some(path) = file_path {
            app.load_pipeline(path);
        }

        Ok(app)
    }

    pub fn load_pipeline(&mut self, path: &str) {
        match Pipeline::from_file(path) {
            Ok(pipeline) => {
                self.metrics = make_pending_metrics(&pipeline);
                self.pipeline = Some(pipeline);
                self.pipeline_file = Some(path.to_string());
                self.status = AppStatus::Idle; // default state
                self.status_message = "Press [R] to run the pipeline".to_string();
                self.error_message = None;
                self.selected = 0;
                self.stage_output = Vec::new();
                self.output_scroll = 0;
                self.file_input_active = false;
                self.file_input_buffer.clear();
            }

            Err(error_in_pipeline) => {
                self.error_message = Some(format!("{}", error_in_pipeline));
                self.status_message =
                    "Some error loading that file... Press [L] to try again.".to_string();
                self.file_input_active = false;
                self.file_input_buffer.clear();
            }
        }
    }

    pub fn activate_file_input(&mut self) {
        self.file_input_active = true;
        self.file_input_buffer.clear();
        self.selected_suggestion = 0;
        self.scan_directory(".");
        self.update_suggestions();
    }

    fn scan_directory(&mut self, directory: &str) {
        self.all_files_cache.clear();

        if let Ok(all_files) = fs::read_dir(directory) {
            for file in all_files.flatten() {
                let path: PathBuf = file.path();
                if path.is_file() {
                    // if found to be a symlink or subdirectory, this will not work
                    if let Some(name) = path.file_name().and_then(|x: &OsStr| x.to_str()) {
                        self.all_files_cache.push(name.to_string());
                    }
                }
            }
        }

        self.all_files_cache.sort();
    }

    fn update_suggestions(&mut self) {
        let query: String = self.file_input_buffer.to_lowercase();

        self.file_suggestions = self
            .all_files_cache
            .iter()
            .filter(|file| file.to_lowercase().contains(&query))
            .cloned()
            .collect();

        if self.selected_suggestion >= self.file_suggestions.len() {
            self.selected_suggestion = self.file_suggestions.len().saturating_sub(1);
        }
    }

    pub fn is_loaded(&self) -> bool {
        return self.pipeline.is_some();
    }

    pub fn start_pipeline(&mut self) {
        let pipeline: &Pipeline = match &self.pipeline {
            Some(p) => p,
            None => return,
        };

        let executable: String = env::current_exe()
            .expect("Failed to get executable path")
            .display()
            .to_string();

        let run_pid: u32 = std::process::id();
        let command: String = pipeline.build_injected_command(run_pid, &executable);

        stats::cleanup(run_pid, pipeline.num_stages());

        let child: Child = Pipeline::spawn(&command).unwrap_or_else(|error| {
            panic!("Failed to spawn pipeline: {}", error);
        });

        self.run_id = run_pid;
        self.child = Some(child);
        self.status = AppStatus::Running;
        self.start_time = Some(Instant::now());
        self.elapsed_seconds = 0.0;
        self.exit_status = None;
        self.error_message = None;
        self.status_message = "Working/Running...".to_string();
        self.metrics = make_pending_metrics(pipeline);
        self.stage_output = Vec::new();
        self.output_scroll = 0;
    }

    pub fn rerun(&mut self) {
        if !self.is_loaded() {
            return;
        }

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(ref pipeline) = self.pipeline {
            stats::cleanup(self.run_id, pipeline.num_stages());
        }

        self.start_pipeline();
    }

    pub fn update(&mut self) {
        if self.status != AppStatus::Running {
            return;
        }

        self.refresh_metrics();
        self.load_stage_output();

        let none_running: bool = self
            .metrics
            .iter()
            .all(|m| m.status != stats::StageStatus::Running);
        let any_completed: bool = self
            .metrics
            .iter()
            .any(|m| m.status == stats::StageStatus::Completed);
        let last_injected_stalled = self.num_stages() >= 2
            && self.metrics.len() >= self.num_stages()
            && self.metrics[self.num_stages() - 2].status == stats::StageStatus::Completed
            && self.metrics[self.num_stages() - 2].total_lines == 0;

        if none_running && any_completed && last_injected_stalled {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }

            self.complete(
                None,
                "COMPLETED (final stage was stalled, killed)".to_string(),
            );
            return;
        }

        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.on_pipeline_exit(status);
                    return;
                }

                Ok(None) => {}
                Err(errors) => {
                    self.error_message = Some(format!("Pipeline error: {}", errors));
                    self.complete(None, "FAILED".to_string());
                    return;
                }
            }
        }

        if let Some(start) = self.start_time {
            self.status_message =
                format!("Working/Running: [{:.1}s]", start.elapsed().as_secs_f64());
        }
    }

    fn complete(&mut self, exit: Option<i32>, message: String) {
        self.elapsed_seconds = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        self.start_time = None;
        self.status = AppStatus::Completed;
        self.exit_status = exit;
        self.status_message = message;
    }

    pub fn force_stop(&mut self) {
        if self.status != AppStatus::Running {
            return;
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.complete(None, "ABORTED (force stopped)".to_string());
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.file_input_active {
            match key.code {
                KeyCode::Char(c) => {
                    self.file_input_buffer.push(c);
                    self.update_suggestions();
                }

                KeyCode::Backspace => {
                    self.file_input_buffer.pop();
                    self.update_suggestions();
                }

                KeyCode::Enter => {
                    let path = if self.file_suggestions.is_empty() {
                        self.file_input_buffer.clone()
                    } else {
                        self.file_suggestions[self.selected_suggestion].clone()
                    };
                    self.load_pipeline(&path);
                }

                KeyCode::Tab | KeyCode::Down => {
                    if !self.file_suggestions.is_empty() {
                        self.selected_suggestion =
                            (self.selected_suggestion + 1) % self.file_suggestions.len();
                    }
                }

                KeyCode::Up => {
                    if !self.file_suggestions.is_empty() {
                        self.selected_suggestion = self.selected_suggestion.saturating_sub(1);
                    }
                }

                KeyCode::Esc => {
                    self.file_input_active = false;
                    self.file_input_buffer.clear();
                    self.file_suggestions.clear();
                }

                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => self.should_exit = true,
            KeyCode::Char('x') | KeyCode::Char('X') => self.force_stop(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.force_stop()
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.activate_file_input();
            }

            KeyCode::Char('r') | KeyCode::Char('R') => {
                if self.is_loaded() && self.status != AppStatus::Running {
                    self.rerun();
                }
            }

            KeyCode::Up | KeyCode::Char('k') => {
                if self.is_loaded() {
                    let new = self.selected.saturating_sub(1);
                    if new != self.selected {
                        self.select_stage(new);
                    }
                }
            }

            KeyCode::Down | KeyCode::Char('j') => {
                if self.is_loaded() {
                    let max = self.num_stages().saturating_sub(1);
                    let new = (self.selected + 1).min(max);
                    if new != self.selected {
                        self.select_stage(new);
                    }
                }
            }

            KeyCode::PageUp => {
                self.output_scroll = self.output_scroll.saturating_sub(10);
            }

            KeyCode::PageDown => {
                let max = self.output_total_lines().saturating_sub(1);
                self.output_scroll = (self.output_scroll + 10).min(max);
            }

            _ => {}
        }
    }

    fn on_pipeline_exit(&mut self, status: ExitStatus) {
        let msg = if status.success() {
            "SUCCESS: Completed successfully".to_string()
        } else {
            format!("FAILURE: Exited with code {}", status.code().unwrap_or(-1))
        };
        self.complete(status.code(), msg);
        self.refresh_metrics();
        self.load_stage_output();
    }

    fn refresh_metrics(&mut self) {
        let pipeline = match &self.pipeline {
            Some(p) => p,
            None => return,
        };
        self.metrics = stats::collect_all(self.run_id, pipeline.num_stages(), &pipeline.stages);
    }

    pub fn num_stages(&self) -> usize {
        self.pipeline.as_ref().map_or(0, |p| p.num_stages())
    }

    pub fn select_stage(&mut self, idx: usize) {
        if !self.is_loaded() {
            return;
        }

        self.selected = idx.min(self.num_stages().saturating_sub(1));
        self.output_scroll = 0;
        self.load_stage_output();
    }

    pub fn load_stage_output(&mut self) {
        if !self.is_loaded() {
            return;
        }

        let stage_number: u32 = (self.selected + 1) as u32;
        self.stage_output = stats::read_stage_output(self.run_id, stage_number);
    }

    pub fn output_total_lines(&self) -> usize {
        self.stage_output.len()
    }

    pub fn filtered_lines(&self, stage_index: usize) -> Option<usize> {
        if stage_index == 0 || !self.is_loaded() {
            return None;
        }

        let previous = self.metrics.get(stage_index - 1)?;
        let current = self.metrics.get(stage_index)?;

        if previous.status == stats::StageStatus::Pending {
            return None;
        }

        Some(previous.total_lines.saturating_sub(current.total_lines))
    }

    pub fn reduction_pct(&self, stage_index: usize) -> Option<f64> {
        if stage_index == 0 || !self.is_loaded() {
            return None;
        }

        let previous = self.metrics.get(stage_index - 1)?;

        if previous.total_lines == 0 {
            return None;
        }

        let filtered: usize = self.filtered_lines(stage_index)?;
        Some((filtered as f64 / previous.total_lines as f64) * 100.0)
    }
}

fn make_pending_metrics(pipeline: &Pipeline) -> Vec<StageMetrics> {
    (0..pipeline.num_stages())
        .map(|i| {
            let cmd = pipeline.stages.get(i).cloned().unwrap_or_default();
            StageMetrics::pending(i + 1, cmd)
        })
        .collect()
}
