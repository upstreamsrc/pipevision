use std::{io::{BufRead, BufReader, BufWriter, Lines, Stdin, Stdout, Write}, time::Instant};

struct PipingStatistics {
    total_bytes_amount: usize,
    total_lines_amount: usize,
    start_time: Instant
}

impl PipingStatistics {
    fn new() -> Self {
        Self {
            total_bytes_amount: 0,
            total_lines_amount: 0,
            start_time: Instant::now()
        }
    }
}

fn main() -> std::io::Result<()> {
    let argv: Vec<String> = std::env::args().collect(); 
    let quiet: bool = argv.iter().any(|arg: &String| arg == "--quiet");

    let mut piping_statistics: PipingStatistics = PipingStatistics::new();

    let stdin: BufReader<Stdin> = std::io::BufReader::new(std::io::stdin());
    let mut lines: Lines<BufReader<Stdin>> = stdin.lines();

    let stdout: Stdout = std::io::stdout();
    let mut writer: BufWriter<Stdout> = BufWriter::new(stdout);

    while let Some(line) = lines.next() {
        let line: String = line?;
        piping_statistics.total_lines_amount += 1;
        piping_statistics.total_bytes_amount += line.as_bytes().len() + 1;

        writeln!(writer, "{}", line)?;

        if !quiet {
            eprintln!("{}", line);
        }
    }

    writer.flush()?;

    let elapsed_time: f64 = piping_statistics.start_time.elapsed().as_secs_f64();
    eprintln!("- Lines: {:>14}", piping_statistics.total_lines_amount);
    eprintln!("- Bytes: {:>14}", piping_statistics.total_bytes_amount);
    if elapsed_time > 0.0 {
        eprintln!("- Rate: {:>12.4} L/s", piping_statistics.total_lines_amount as f64 / elapsed_time); // lines per second
        eprintln!("- Rate: {:>12.4} B/s", piping_statistics.total_bytes_amount as f64 / elapsed_time); // bytes per second
    } else {
        eprintln!("Rate: so fast that it actually time travelled to the past with negative time elapsed somehow");
    }

    Ok(())
}
