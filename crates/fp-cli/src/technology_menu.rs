//! Seletor de tecnologias com itens desabilitados. Cliclack 0.5 não expõe esse
//! estado; reutilizamos seu Theme e o console já utilizado pela própria crate.
use crate::{Result, create::Technology, processes};
use cliclack::{Theme, ThemeState};
use console::{Key, Term, measure_text_width};
use std::{
    ffi::OsStr,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Debug, Clone)]
pub struct Choice {
    pub technology: Technology,
    pub tool: Option<PathBuf>,
    label: &'static str,
    hint: &'static str,
}
impl Choice {
    pub fn enabled(&self) -> bool {
        self.technology.tool().is_none() || self.tool.is_some()
    }
    fn label(&self) -> String {
        if self.enabled() {
            self.label.to_string()
        } else {
            format!("{} (não configurado)", self.label)
        }
    }
}

pub fn choices(path: &OsStr) -> Vec<Choice> {
    let specs = [
        (Technology::Python, "Python", "uv"),
        (Technology::Web, "TypeScript / JavaScript", "Vite+ (vp)"),
        (Technology::Rust, "Rust", "Cargo"),
        (Technology::Iot, "ESP32 / IoT", "PlatformIO"),
        (Technology::Sql, "SQL / Database", "Estrutura mínima"),
        (Technology::Folder, "Somente pasta", "Nenhum inicializador"),
    ];
    specs
        .into_iter()
        .map(|(technology, label, hint)| {
            let tool = technology
                .tool()
                .and_then(|name| processes::find_in(name, path))
                .or_else(|| {
                    (technology == Technology::Iot)
                        .then(|| processes::find_in("platformio", path))
                        .flatten()
                });
            Choice {
                technology,
                tool,
                label,
                hint,
            }
        })
        .collect()
}

fn next(choices: &[Choice], cursor: usize, down: bool) -> usize {
    if down {
        (cursor + 1..choices.len()).find(|&i| choices[i].enabled())
    } else {
        (0..cursor).rev().find(|&i| choices[i].enabled())
    }
    .unwrap_or(cursor)
}

struct MenuTheme;
impl Theme for MenuTheme {}

fn render(choices: &[Choice], cursor: usize, state: &ThemeState) -> String {
    let theme = MenuTheme;
    let mut frame = theme.format_header(state, "Qual tecnologia deseja utilizar?");
    for (i, choice) in choices.iter().enumerate() {
        // O estilo inativo do Theme mantém as opções indisponíveis esmaecidas.
        frame.push_str(&theme.format_select_item(state, i == cursor, &choice.label(), choice.hint));
    }
    frame.push_str(&theme.format_footer(state));
    frame
}

fn draw(term: &mut Term, previous: &mut String, frame: String) -> io::Result<()> {
    if frame != *previous {
        let width = usize::from(term.size().1).max(1);
        let rows = previous
            .lines()
            .map(|line| measure_text_width(line).max(1).div_ceil(width))
            .sum();
        term.clear_last_lines(rows)?;
        term.write_all(frame.as_bytes())?;
        term.flush()?;
        *previous = frame;
    }
    Ok(())
}

fn interact(term: &mut Term, choices: &[Choice]) -> io::Result<usize> {
    let mut cursor = choices.iter().position(Choice::enabled).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "Nenhuma tecnologia disponível")
    })?;
    let mut previous = String::new();
    loop {
        draw(
            term,
            &mut previous,
            render(choices, cursor, &ThemeState::Active),
        )?;
        match term.read_key_raw()? {
            Key::ArrowUp | Key::ArrowLeft | Key::Char('k' | 'h' | '\x10') => {
                cursor = next(choices, cursor, false)
            }
            Key::ArrowDown | Key::ArrowRight | Key::Char('j' | 'l' | '\x0e') => {
                cursor = next(choices, cursor, true)
            }
            Key::Enter if choices[cursor].enabled() => {
                draw(
                    term,
                    &mut previous,
                    render(choices, cursor, &ThemeState::Submit),
                )?;
                return Ok(cursor);
            }
            Key::Escape | Key::CtrlC => {
                draw(
                    term,
                    &mut previous,
                    render(choices, cursor, &ThemeState::Cancel),
                )?;
                return Err(io::ErrorKind::Interrupted.into());
            }
            _ => {}
        }
    }
}

pub fn select() -> Result<Choice> {
    let choices = choices(&std::env::var_os("PATH").unwrap_or_default());
    let mut term = Term::stderr();
    if !term.is_term() {
        return Err("O seletor de tecnologias requer um terminal interativo".into());
    }
    term.hide_cursor()?;
    let result = interact(&mut term, &choices);
    let restored = term.show_cursor();
    let index = result?;
    restored?;
    Ok(choices[index].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::{fs, os::unix::fs::PermissionsExt};

    #[test]
    fn missing_tools_disable_only_programming_ecosystems() {
        let dir = tempfile::tempdir().unwrap();
        let items = choices(dir.path().as_os_str());
        assert_eq!(
            items.iter().map(Choice::enabled).collect::<Vec<_>>(),
            [false, false, false, false, true, true]
        );
        let frame = render(&items, 4, &ThemeState::Active);
        for label in ["Python", "TypeScript / JavaScript", "Rust", "ESP32 / IoT"] {
            assert!(frame.contains(&format!("{label} (não configurado)")));
        }
        assert!(!frame.contains("SQL / Database (não configurado)"));
    }
    #[test]
    fn navigation_skips_disabled_entries_in_both_directions() {
        let dir = tempfile::tempdir().unwrap();
        let items = choices(dir.path().as_os_str());
        assert_eq!(next(&items, 4, true), 5);
        assert_eq!(next(&items, 5, false), 4);
        assert_eq!(next(&items, 4, false), 4);
        assert_eq!(next(&items, 5, true), 5);
    }
    #[test]
    #[cfg(unix)]
    fn only_executable_tools_on_path_enable_their_technology() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["uv", "vp", "cargo", "platformio"] {
            fs::write(dir.path().join(name), "#!/bin/sh\nexit 0\n").unwrap();
        }
        assert!(!choices(dir.path().as_os_str())[1].enabled());
        for name in ["uv", "vp", "cargo", "platformio"] {
            fs::set_permissions(dir.path().join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        let items = choices(dir.path().as_os_str());
        assert!(items.iter().all(Choice::enabled));
        assert_eq!(
            items[1].tool.as_deref(),
            Some(dir.path().join("vp").as_path())
        );
        assert_eq!(
            items[3].tool.as_deref(),
            Some(dir.path().join("platformio").as_path())
        );
        fs::remove_file(dir.path().join("vp")).unwrap();
        let items = choices(dir.path().as_os_str());
        assert!(!items[1].enabled());
        assert_eq!(next(&items, 0, true), 2);
        assert_eq!(next(&items, 2, false), 0);
    }
    #[test]
    #[cfg(unix)]
    fn pio_alias_and_broken_symlink_detection() {
        let dir = tempfile::tempdir().unwrap();
        let pio = dir.path().join("pio");
        fs::write(&pio, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&pio, fs::Permissions::from_mode(0o755)).unwrap();
        std::os::unix::fs::symlink(dir.path().join("missing"), dir.path().join("vp")).unwrap();
        let items = choices(dir.path().as_os_str());
        assert!(items[3].enabled());
        assert!(!items[1].enabled());
    }
}
