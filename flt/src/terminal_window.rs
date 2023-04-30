//! Implementation of [crate::window::Window], rendering to the terminal output.
//!
//! This should be the only file in this crate which depends on [crossterm].

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::style::{Color, Print, PrintStyledContent, Stylize};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{ErrorKind, ExecutableCommand, QueueableCommand};
use flutter_sys::Pixel;
use std::io::{stdout, Stdout, Write};
use std::iter::zip;

pub struct TerminalWindow {
    stdout: Stdout,
    lines: Vec<Vec<TerminalCell>>,
    simple_output: bool,
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct TerminalCell {
    top: Color,
    bottom: Color,
}

impl TerminalWindow {
    pub fn new(simple_output: bool) -> Self {
        let mut stdout = stdout();

        if !simple_output {
            // This causes the terminal to be output on an alternate buffer.
            stdout.execute(EnterAlternateScreen).unwrap();
            stdout.execute(Hide).unwrap();

            enable_raw_mode().unwrap();
            stdout.execute(EnableMouseCapture).unwrap();
        }

        Self {
            stdout,
            lines: vec![],
            simple_output,
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

fn to_color(Pixel { r, g, b, a: _ }: &Pixel) -> Color {
    Color::Rgb {
        r: *r,
        g: *g,
        b: *b,
    }
}

impl Drop for TerminalWindow {
    fn drop(&mut self) {
        if !self.simple_output {
            self.stdout.execute(DisableMouseCapture).unwrap();
            disable_raw_mode().unwrap();
            self.stdout.execute(Show).unwrap();
            self.stdout.execute(LeaveAlternateScreen).unwrap();
        }
    }
}

impl TerminalWindow {
    pub fn draw_text(&mut self, x: usize, y: usize, text: &str) -> Result<(), ErrorKind> {
        self.stdout.queue(MoveTo(x as u16, y as u16))?;
        self.stdout.queue(Print(text))?;
        self.stdout.flush()?;

        Ok(())
    }

    pub fn draw(
        &mut self,
        width: usize,
        height: usize,
        buffer: Vec<Pixel>,
    ) -> Result<(), ErrorKind> {
        if self.simple_output {
            return Ok(());
        }

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
                let cell = TerminalCell {
                    top: to_color(&top),
                    bottom: to_color(&bottom),
                };

                let current_line = acc.last_mut().unwrap();
                current_line.push(cell);

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

                for TerminalCell { top, bottom } in current {
                    self.stdout.queue(PrintStyledContent(
                        BLOCK_UPPER.to_string().with(*top).on(*bottom),
                    ))?;
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

const BLOCK_UPPER: char = '▀';
