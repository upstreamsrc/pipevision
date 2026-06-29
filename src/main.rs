use std::env;
use std::io;
use std::io::Stdout;

use crate::app::App;

mod app;
mod pipeline;
mod stats;
mod ui;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let has_stage: bool = args.iter().any(|arg| arg.starts_with("--stage="));

    if has_stage {
        let quiet: bool = args.iter().any(|arg| arg == "--quiet");
        return pipeline::run_pipeline_injected(quiet);
    }

    let file_arg = args.iter().skip(1).find(|arg| !arg.starts_with("--"));
    run_tui(file_arg.map(|s| s.as_str()))
}

fn run_tui(file_path: Option<&str>) -> io::Result<()> {
    use crossterm::event::{self, Event};
    use crossterm::execute;
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;
    use std::time::Duration;

    let mut app: App = app::App::new(file_path)?;

    if file_path.is_some() {
        app.start_pipeline();
    }

    enable_raw_mode()?;
    let mut stdout: Stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend: CrosstermBackend<Stdout> = CrosstermBackend::new(stdout);
    let mut terminal: Terminal<CrosstermBackend<Stdout>> = Terminal::new(backend)?;
    terminal.clear()?;

    let result: Result<(), io::Error> = loop {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
                if app.should_exit {
                    break Ok(());
                }
            }
        }

        app.update();
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
