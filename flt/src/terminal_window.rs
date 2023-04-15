//! Implementation of [crate::window::Window], rendering to the terminal output.
//!
//! This should be the only file in this crate which depends on [crossterm].

use std::io::stdout;
use std::io::Stdout;
use std::io::Write;
use std::iter::zip;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::style::{Color, PrintStyledContent, Stylize};
use crossterm::terminal::size;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ErrorKind;
use crossterm::ExecutableCommand;
use crossterm::QueueableCommand;
use flutter_sys::Pixel;

pub struct TerminalWindow {
    stdout: Stdout,
    lines: Vec<Vec<TerminalChar>>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct TerminalChar {
    foreground: Color,
    background: Option<Color>,
    char: char,
}

impl TerminalWindow {
    pub fn new() -> Self {
        let mut stdout = stdout();

        // This causes the terminal to be output on an alternate buffer.
        stdout.execute(EnterAlternateScreen).unwrap();
        stdout.execute(Hide).unwrap();

        enable_raw_mode().unwrap();
        stdout.execute(EnableMouseCapture).unwrap();

        Self {
            stdout,
            lines: vec![],
        }
    }

    pub fn size(&self) -> (usize, usize) {
        let (width, height) = size().unwrap();
        (
            width as usize,
            // The terminal renderer merges two pixels (top and bottom) into one.
            (height * 2) as usize,
        )
    }
}

fn is_lit(pixel: &Pixel) -> bool {
    if pixel.a == 0 {
        return false;
    }

    return pixel.r != 0 && pixel.g != 0 && pixel.b != 0;
}

fn to_color(Pixel { r, g, b, a: _ }: &Pixel) -> Color {
    Color::Rgb {
        r: *r,
        g: *g,
        b: *b,
    }
}

impl Drop for TerminalWindow {
    fn drop(&mut self) {
        self.stdout.execute(DisableMouseCapture).unwrap();
        disable_raw_mode().unwrap();
        self.stdout.execute(Show).unwrap();
        self.stdout.execute(LeaveAlternateScreen).unwrap();
    }
}

impl TerminalWindow {
    pub fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>) -> Result<(), Error> {
        let mut buffer = buffer.to_vec();
        // Always process an even number of rows.
        if buffer.len() % 2 != 0 {
            buffer.extend(vec![Pixel::zero(); width]);
        }

        // Each element is one pixel, but when it is rendered to the terminal,
        // two rows share one character, using the unicode BLOCK characters.

        // Group alternate rows together, so zipping them allows two consecutive
        // rows to be processed into terminal characters at the same time.
        let tops = buffer
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let row = i / width;

                if row % 2 == 0 {
                    true
                } else {
                    false
                }
            })
            .map(|(_, val)| *val);
        let bottoms = buffer
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let row = i / width;

                if row % 2 == 1 {
                    true
                } else {
                    false
                }
            })
            .map(|(_, val)| *val);

        let lines = zip(tops, bottoms)
            .enumerate()
            .fold(vec![], |mut acc, (i, (top, bottom))| {
                if i % width == 0 {
                    acc.push(vec![]);
                }
                let character = match (is_lit(&top), is_lit(&bottom)) {
                    (true, true) => TerminalChar {
                        // Upper is the foreground, lower is the background.
                        foreground: to_color(&top),
                        background: Some(to_color(&bottom)),
                        char: BLOCK_UPPER,
                    },
                    (true, false) => TerminalChar {
                        foreground: to_color(&top),
                        background: None,
                        char: BLOCK_UPPER,
                    },
                    (false, true) => TerminalChar {
                        foreground: to_color(&bottom),
                        background: None,
                        char: BLOCK_LOWER,
                    },
                    (false, false) => TerminalChar {
                        foreground: to_color(&Pixel::zero()),
                        background: None,
                        char: BLOCK_EMPTY,
                    },
                };

                let current_line = acc.last_mut().unwrap();
                current_line.push(character);

                acc
            });

        assert_eq!(lines.len(), height / 2);

        // Refreshing the entire terminal (with the clear char) and outputting
        // everything on every iteration is costly and causes the terminal to
        // flicker.
        //
        // Instead, only "re-render" the current line, if it is different from
        // the previous frame.

        // Means that the screen dimensions has changed.
        if self.lines.len() != lines.len() {
            // TODO(jiahaog): Find a faster way to do this.
            // Use empty values so the diffing check below always fail.
            self.lines = vec![vec![]; lines.len()];
        }

        for (i, (prev, current)) in zip(&self.lines, &lines).enumerate() {
            if !do_vecs_match(prev, current) {
                self.stdout.queue(MoveTo(0, i as u16))?;
                self.stdout.queue(Clear(ClearType::CurrentLine))?;

                for TerminalChar {
                    foreground,
                    background,
                    char,
                } in current
                {
                    let style = char.to_string().with(*foreground);
                    let style = match background {
                        Some(background) => style.on(*background),
                        None => style,
                    };
                    self.stdout.queue(PrintStyledContent(style))?;
                }
            }
        }

        self.stdout.flush()?;

        self.lines = lines;

        Ok(())
    }
}

fn do_vecs_match<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    let matching = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
    matching == a.len() && matching == b.len()
}

const BLOCK_LOWER: char = '▄';
const BLOCK_UPPER: char = '▀';
const BLOCK_EMPTY: char = ' ';

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Error::ErrorStr(value.to_string())
    }
}

#[derive(Debug)]
pub enum Error {
    ErrorStr(String),
}
