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
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ErrorKind;
use crossterm::ExecutableCommand;
use crossterm::QueueableCommand;

pub struct Terminal {
    stdout: Stdout,
    lines: Vec<Vec<(Pixel, char)>>,
    // TODO: Support changing this at runtime.
    width: usize,
    height: usize,
}

impl Terminal {
    pub fn new(width: usize, height: usize) -> Self {
        let mut stdout = stdout();

        // This causes the terminal to be output on an alternate buffer.
        stdout.execute(EnterAlternateScreen).unwrap();
        stdout.execute(Hide).unwrap();

        enable_raw_mode().unwrap();
        stdout.execute(EnableMouseCapture).unwrap();

        Self {
            stdout,
            lines: vec![vec![]; height / 2],
            width,
            height,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Pixel {
    pub fn zero() -> Self {
        Pixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
    fn is_lit(&self) -> bool {
        if self.a == 0 {
            return false;
        }

        return self.r != 0 && self.g != 0 && self.b != 0;
    }
}

impl From<Pixel> for Color {
    fn from(Pixel { r, g, b, a: _ }: Pixel) -> Self {
        Color::Rgb { r, g, b }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.stdout.execute(DisableMouseCapture).unwrap();
        disable_raw_mode().unwrap();
        self.stdout.execute(Show).unwrap();
        self.stdout.execute(LeaveAlternateScreen).unwrap();
    }
}

impl Terminal {
    pub fn update(&mut self, buffer: &Vec<Pixel>) -> Result<(), Error> {
        let mut buffer = buffer.to_vec();
        // Always process an even number of rows.
        if buffer.len() % 2 != 0 {
            buffer.extend(vec![Pixel::zero(); self.width]);
        }

        // Each element is one pixel, but when it is rendered to the terminal,
        // two rows share one character, using the unicode BLOCK characters.

        // Group alternate rows together, so zipping them allows two consecutive
        // rows to be processed into terminal characters at the same time.
        let tops = buffer
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let row = i / self.width;

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
                let row = i / self.width;

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
                if i % self.width == 0 {
                    acc.push(vec![]);
                }
                let character = match (top.is_lit(), bottom.is_lit()) {
                    // Just use the top for the color for now.
                    (true, true) => (top, BLOCK_FULL),
                    (true, false) => (top, BLOCK_UPPER),
                    (false, true) => (bottom, BLOCK_LOWER),
                    (false, false) => (Pixel::zero(), BLOCK_EMPTY),
                };

                let current_line = acc.last_mut().unwrap();
                current_line.push(character);

                acc
            });

        assert_eq!(lines.len(), self.height / 2);
        assert_eq!(lines.len(), self.lines.len());

        // Refreshing the entire terminal (with the clear char) and outputting
        // everything on every iteration is costly and causes the terminal to
        // flicker.
        //
        // Instead, only "re-render" the current line, if it is different from
        // the previous frame.

        for (i, (prev, current)) in zip(&self.lines, &lines).enumerate() {
            if !do_vecs_match(prev, current) {
                self.stdout.queue(MoveTo(0, i as u16))?;
                self.stdout.queue(Clear(ClearType::CurrentLine))?;

                for (pixel, char) in current {
                    let color: Color = Color::from(*pixel);

                    self.stdout
                        .queue(PrintStyledContent(char.to_string().with(color)))?;
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
const BLOCK_FULL: char = '█';
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
