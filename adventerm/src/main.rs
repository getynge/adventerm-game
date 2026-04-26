mod app;
mod config;
mod dump;
mod input;
mod menu;
mod relaunch;
mod ui;

use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use app::App;
use ratatui::{DefaultTerminal, Frame};

const FLAG_DUMP: &str = "--dump-rooms";
const FLAG_SEED: &str = "--seed";
const DUMP_USAGE: &str = "usage: adventerm --dump-rooms <count> <path> [--seed <n>]";

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let argv: Vec<String> = std::env::args().collect();
    if let Some((seed, count, path)) = parse_dump_args(&argv) {
        dump::run(seed, count, &path)?;
        return Ok(());
    }
    if let relaunch::Relaunch::Spawned = relaunch::maybe_relaunch_in_terminal() {
        return Ok(());
    }
    ratatui::run(run)?;
    Ok(())
}

fn parse_dump_args(args: &[String]) -> Option<(u64, usize, PathBuf)> {
    let dump_idx = args.iter().position(|a| a == FLAG_DUMP)?;

    let count = match args.get(dump_idx + 1).and_then(|s| s.parse::<usize>().ok()) {
        Some(n) => n,
        None => fail_dump("missing or non-numeric <count>"),
    };
    let path = match args.get(dump_idx + 2) {
        Some(p) if !p.starts_with("--") => PathBuf::from(p),
        _ => fail_dump("missing <path>"),
    };
    let seed = match args.iter().position(|a| a == FLAG_SEED) {
        Some(i) => match args.get(i + 1).and_then(|s| s.parse::<u64>().ok()) {
            Some(n) => n,
            None => fail_dump("missing or non-numeric <seed>"),
        },
        None => clock_seed(),
    };

    Some((seed, count, path))
}

fn clock_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn fail_dump(msg: &str) -> ! {
    eprintln!("error: {msg}\n{DUMP_USAGE}");
    process::exit(2);
}

fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app = App::new();
    loop {
        terminal.draw(|frame: &mut Frame| ui::render(frame, &app))?;
        if app.should_quit() {
            return Ok(());
        }
        let event = crossterm::event::read()?;
        app.handle_event(&event);
    }
}
