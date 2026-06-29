use std::{
    env, fs,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Instant,
};

pub const STATS_UPDATE_INTERVAL: usize = 100;
pub const MAX_OUTPUT_LINES: usize = 5000;

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub original: String,
    pub stages: Vec<String>,
}

impl Pipeline {
    pub fn from_file(path: &str) -> io::Result<Self> {
        let content: String = fs::read_to_string(path)?;
        let original: String = content.trim().to_string();

        if original.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("'{}' is empty", path),
            ));
        }

        let stages: Vec<String> = original.split('|').map(|s| s.trim().to_string()).collect();

        if stages.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("'{}' contains no pipe stages", path),
            ));
        }

        for (index, stage) in stages.iter().enumerate() {
            if stage.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "'{}' stage {} is empty (double '|' or trailing pipe?)",
                        path,
                        index + 1
                    ),
                ));
            }
        }

        Ok(Self { original, stages })
    }

    pub fn num_stages(&self) -> usize {
        self.stages.len()
    }

    pub fn build_injected_command(&self, run_id: u32, executable: &str) -> String {
        let mut injected_parts: Vec<String> = Vec::new();
        let mut stage_counter: u32 = 1u32;

        for (inx, stage_cmd) in self.stages.iter().enumerate() {
            injected_parts.push(stage_cmd.clone());

            if inx + 1 < self.stages.len() {
                injected_parts.push(format!(
                    "{} --stage={} --run-id={} --quiet",
                    executable, stage_counter, run_id
                ));
                stage_counter += 1;
            }
        }

        injected_parts.join(" | ")
    }

    pub fn spawn(command: &str) -> io::Result<Child> {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }
}

pub struct PipingStatistics {
    pub total_bytes_amount: usize,
    pub total_lines_amount: usize,
    pub start_time: Instant,
    pub finished: bool,
    last_update_line: usize,
}

impl PipingStatistics {
    pub fn new() -> Self {
        Self {
            total_bytes_amount: 0,
            total_lines_amount: 0,
            start_time: Instant::now(),
            finished: false,
            last_update_line: 0,
        }
    }

    pub fn write_stats_file(&self, run_id: u32, stage: u32) {
        let elapsed: f64 = self.start_time.elapsed().as_secs_f64();
        let lines_per_sec: f64 = self.total_lines_amount as f64 / elapsed.max(0.0001);
        let bytes_per_sec: f64 = self.total_bytes_amount as f64 / elapsed.max(0.0001);
        let status: &str = if self.finished {
            "completed"
        } else {
            "running"
        };

        let data: String = format!(
            "{}\n{}\n{:.4}\n{:.4}\n{}\n",
            self.total_lines_amount, self.total_bytes_amount, lines_per_sec, bytes_per_sec, status,
        );

        let stats_file: PathBuf = stats_path(run_id, stage);
        let _ = fs::write(&stats_file, data);
    }

    pub fn try_write_update(&mut self, run_id: u32, stage: u32) {
        if self.total_lines_amount - self.last_update_line >= STATS_UPDATE_INTERVAL {
            self.write_stats_file(run_id, stage);
            self.last_update_line = self.total_lines_amount;
        }
    }
}

pub fn stats_path(run_id: u32, stage: u32) -> PathBuf {
    env::temp_dir().join(format!("pipevision_{}_{}.txt", run_id, stage))
}

pub fn output_path(run_id: u32, stage: u32) -> PathBuf {
    env::temp_dir().join(format!("pipevision_{}_{}_output.txt", run_id, stage))
}

pub fn write_output_file(run_id: u32, stage: u32, lines: &[String]) {
    let path: PathBuf = output_path(run_id, stage);
    let _ = fs::write(&path, lines.join("\n"));
}

pub fn parse_stage_arg() -> u32 {
    env::args()
        .find_map(|arg| arg.strip_prefix("--stage=")?.parse::<u32>().ok())
        .unwrap_or(1)
}

pub fn parse_run_id_arg() -> u32 {
    env::args()
        .find_map(|arg| arg.strip_prefix("--run-id=")?.parse::<u32>().ok())
        .unwrap_or(0)
}

pub fn run_pipeline_injected(dont_show_output: bool) -> io::Result<()> {
    let stage: u32 = parse_stage_arg();
    let run_id: u32 = parse_run_id_arg();
    let mut stats: PipingStatistics = PipingStatistics::new();
    let mut output_buffer: Vec<String> = Vec::new();

    let stdin = io::stdin();
    let reader: BufReader<std::io::StdinLock<'_>> = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer: BufWriter<std::io::StdoutLock<'_>> = BufWriter::new(stdout.lock());

    for line_result in reader.lines() {
        let line: String = line_result?;

        stats.total_lines_amount += 1;
        stats.total_bytes_amount += line.as_bytes().len() + 1;

        writeln!(writer, "{}", line)?;

        if output_buffer.len() < MAX_OUTPUT_LINES {
            output_buffer.push(line.clone());
        }

        if !dont_show_output {
            eprintln!("[Stage {} Output] {}", stage, line);
        }

        stats.try_write_update(run_id, stage);

        if output_buffer.len() % STATS_UPDATE_INTERVAL == 0 && !output_buffer.is_empty() {
            write_output_file(run_id, stage, &output_buffer);
        }
    }

    writer.flush()?;
    stats.finished = true;
    stats.write_stats_file(run_id, stage);
    write_output_file(run_id, stage, &output_buffer);
    Ok(())
}
