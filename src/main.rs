use std::{env, io::{BufRead, BufReader, BufWriter, Lines, Stdin, Stdout, Write}, time::Instant};

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

    // let stage: usize = argv.iter()
    //                        .position(|arg: &String| arg == "--stage")
    //                        .and_then(|index: usize| argv.get(index + 1)
    //                                                     .and_then(|arg: &String| arg.parse().ok()))
    //                        .unwrap_or(1); 

    let stage: usize = env::var("PIPE_STAGE")
                       .ok().and_then(|s: String| s.parse().ok())
                       .or_else(|| {
                            argv.iter().position(|arg: &String| arg == "--stage")
                            .and_then(|index: usize| argv.get(index + 1))
                                                         .and_then(|arg: &String| arg.parse().ok())
                       }).unwrap_or(1);

    // cat test.txt | ./target/release/pipevision --stage 1 | grep -v "DEBUG" | ./target/release/pipevision --stage 2 | sort | ./target/release/pipevision --stage 3

    let input_lines: usize = argv.iter()
        .position(|arg: &String| arg == "--input")
        .and_then(|index: usize| argv.get(index + 1).and_then(|arg_str: &String| arg_str.parse().ok()))
        .unwrap_or(0);

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

    let filtered_lines: usize = if input_lines > 0 {
        input_lines.saturating_sub(piping_statistics.total_lines_amount) // min: 0
    } else {0};

    let elapsed_time: f64 = piping_statistics.start_time.elapsed().as_secs_f64();
    if elapsed_time > 0.0 {
        eprintln!();
        eprintln!("-[ STAGE {} ]-", stage);
        eprintln!("Lines       : {}", piping_statistics.total_lines_amount);
        eprintln!("Filtered    : {}", filtered_lines);  
        eprintln!("Bytes       : {}", piping_statistics.total_bytes_amount);
        eprintln!("L/s         : {:.4}", piping_statistics.total_lines_amount as f64 / elapsed_time);
        eprintln!("B/s         : {:.4}", piping_statistics.total_bytes_amount as f64 / elapsed_time);
        eprintln!();
    } else {
        eprintln!("Rate: you cannot time travel to the past, ET is <0, try again");
    }
    
    Ok(())
}
