use std::env;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

const GUARD_ENV: &str = "ADVENTERM_RELAUNCHED";
const GUARD_FLAG: &str = "--no-relaunch";

pub enum Relaunch {
    Continue,
    Spawned,
}

pub fn maybe_relaunch_in_terminal() -> Relaunch {
    if already_attempted() {
        return Relaunch::Continue;
    }
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        return Relaunch::Continue;
    }
    let Ok(exe) = env::current_exe() else {
        return Relaunch::Continue;
    };
    if try_spawn_terminal(&exe) {
        Relaunch::Spawned
    } else {
        Relaunch::Continue
    }
}

fn already_attempted() -> bool {
    env::var_os(GUARD_ENV).is_some() || env::args().any(|a| a == GUARD_FLAG)
}

#[cfg(target_os = "macos")]
fn try_spawn_terminal(exe: &Path) -> bool {
    let escaped = exe
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let script = format!(
        "tell application \"Terminal\" to do script \"{} {}\"",
        escaped, GUARD_FLAG
    );
    Command::new("osascript")
        .arg("-e")
        .arg(script)
        .spawn()
        .is_ok()
}

#[cfg(target_os = "windows")]
fn try_spawn_terminal(exe: &Path) -> bool {
    if Command::new("wt.exe")
        .arg(exe)
        .arg(GUARD_FLAG)
        .env(GUARD_ENV, "1")
        .spawn()
        .is_ok()
    {
        return true;
    }
    Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(exe)
        .arg(GUARD_FLAG)
        .env(GUARD_ENV, "1")
        .spawn()
        .is_ok()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn try_spawn_terminal(exe: &Path) -> bool {
    let candidates: &[(&str, &[&str])] = &[
        ("gnome-terminal", &["--"]),
        ("konsole", &["-e"]),
        ("xfce4-terminal", &["-x"]),
        ("alacritty", &["-e"]),
        ("kitty", &[]),
        ("wezterm", &["start", "--"]),
        ("foot", &[]),
        ("xterm", &["-e"]),
        ("x-terminal-emulator", &["-e"]),
    ];
    for (prog, args) in candidates {
        if !is_on_path(prog) {
            continue;
        }
        let spawned = Command::new(prog)
            .args(*args)
            .arg(exe)
            .arg(GUARD_FLAG)
            .env(GUARD_ENV, "1")
            .spawn()
            .is_ok();
        if spawned {
            return true;
        }
    }
    false
}

#[cfg(all(unix, not(target_os = "macos")))]
fn is_on_path(prog: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| dir.join(prog).is_file())
}
