//! This should be the only file in this crate which depends on [crossterm]
//! functionality beyond the data classes.

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::style::{Color, Print, PrintStyledContent, Stylize};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{ErrorKind, ExecutableCommand, QueueableCommand};
use flutter_sys::Pixel;
use std::collections::HashMap;
use std::io::{stdout, Stdout, Write};
use std::iter::zip;
use std::ops::Add;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

pub struct TerminalWindow {
    stdout: Stdout,
    lines: Vec<Vec<TerminalCell>>,
    // y of semantics is represented in the "external" height.
    semantics: HashMap<(usize, usize), String>,
    simple_output: bool,
    event_channel: Receiver<Event>,
}

impl TerminalWindow {
    pub(crate) fn new(simple_output: bool) -> Self {
        let mut stdout = stdout();

        if !simple_output {
            // This causes the terminal to be output on an alternate buffer.
            stdout.execute(EnterAlternateScreen).unwrap();
            stdout.execute(Hide).unwrap();

            enable_raw_mode().unwrap();
            stdout.execute(EnableMouseCapture).unwrap();
        }

        let (sender, receiver) = channel();

        thread::spawn(move || {
            let mut should_run = true;
            while should_run {
                let event = read().unwrap();
                let event = normalize_event_height(event);
                should_run = sender.send(event).is_ok();
            }
        });

        Self {
            stdout,
            lines: vec![],
            semantics: HashMap::new(),
            simple_output,
            event_channel: receiver,
        }
    }

    pub(crate) fn event_channel(&self) -> &Receiver<Event> {
        &self.event_channel
    }

    pub(crate) fn size(&self) -> (usize, usize) {
        let (width, height) = size().unwrap();
        (width as usize, to_external_height(height) as usize)
    }
}

fn to_external_height<T: Add<Output = T> + Copy>(internal_height: T) -> T {
    internal_height + internal_height
}

fn normalize_event_height(event: Event) -> Event {
    match event {
        Event::Resize(columns, rows) => Event::Resize(columns, to_external_height(rows)),
        Event::Mouse(mut mouse_event) => {
            mouse_event.row = to_external_height(mouse_event.row);
            Event::Mouse(mouse_event)
        }
        x => x,
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
    pub(crate) fn update_semantics(&mut self, label_positions: Vec<((usize, usize), String)>) {
        // TODO(jiahaog): This is slow.
        self.semantics = label_positions
            .into_iter()
            // .map(|((x, y), label)| ((x, to_internal_height(y)), label))
            .collect();
    }

    pub(crate) fn draw(
        &mut self,
        mut pixel_grid: Vec<Vec<Pixel>>,
        (x_offset, y_offset): (usize, usize),
    ) -> Result<(), ErrorKind> {
        // TODO(jiahaog): Stub out stdout instead so more things actually happen.
        if self.simple_output {
            return Ok(());
        }

        let width = pixel_grid.first().map_or(0, |row| row.len());
        let height = pixel_grid.len();

        // Always process an even number of rows.
        if pixel_grid.len() % 2 != 0 {
            pixel_grid.extend(vec![vec![Pixel::zero(); width]]);
        }

        // Extend the bounds of the grid to handle the offset.
        // TODO(jiahaog): This is slow.

        let (terminal_width, terminal_height) = self.size();
        let terminal_height = to_external_height(terminal_height);

        let height_shortfall = y_offset + terminal_height - height;
        if height_shortfall > 0 {
            for _ in 0..height_shortfall {
                pixel_grid.extend(vec![vec![Pixel::zero(); width]]);
            }
        }

        let row_shortfall = x_offset + terminal_width - width;
        if row_shortfall > 0 {
            for row in pixel_grid.iter_mut() {
                row.extend(vec![Pixel::zero(); row_shortfall]);
            }
        }

        // Each element is one pixel, but when it is rendered to the terminal,
        // two rows share one character, using the unicode BLOCK characters.
        let lines = (y_offset..pixel_grid.len())
            .step_by(2)
            .map(|y| {
                let tops = &pixel_grid[y];
                let bottoms = &pixel_grid[y + 1];

                zip(tops, bottoms)
                    .enumerate()
                    .skip(x_offset)
                    .map(|(x, (top, bottom))| {
                        // Find the semantic labels for the current cell.
                        let semantics = match (
                            self.semantics.get(&(x, y)).cloned(),
                            // As two rows are merged together, so check the next row as well.
                            self.semantics.get(&(x, y + 1)).cloned(),
                        ) {
                            (None, None) => None,
                            (None, right) => right,
                            (left, None) => left,
                            // Use the longest.
                            (Some(left), Some(right)) => Some(if left.len() > right.len() {
                                left
                            } else {
                                right
                            }),
                        };

                        TerminalCell {
                            top: to_color(&top),
                            bottom: to_color(&bottom),
                            semantics,
                        }
                    })
                    .collect::<Vec<TerminalCell>>()
            })
            .collect::<Vec<Vec<TerminalCell>>>();

        // Refreshing the entire terminal (with the clear char) and outputting
        // everything on every iteration is costly and causes the terminal to
        // flicker.
        //
        // Instead, only "re-render" the current line, if it is different from
        // the previous frame.

        // Means that the screen dimensions has changed.
        if self.lines.len() != lines.len() {
            // Use empty values so the diffing check below always fail.
            self.lines = vec![vec![]; lines.len()];
        }

        for (y, (prev, current)) in zip(&self.lines, &lines).enumerate() {
            if !do_vecs_match(prev, current) {
                self.stdout.queue(MoveTo(0, y as u16))?;

                for TerminalCell {
                    top,
                    bottom,
                    semantics: _,
                } in current
                {
                    self.stdout.queue(PrintStyledContent(
                        BLOCK_UPPER.to_string().with(*top).on(*bottom),
                    ))?;
                }

                // Second pass to put semantics over pixels.
                for (
                    x,
                    TerminalCell {
                        top: _,
                        bottom: _,
                        semantics,
                    },
                ) in current.into_iter().enumerate()
                {
                    if semantics.is_none() {
                        continue;
                    }
                    self.stdout.queue(MoveTo(x as u16, y as u16))?;
                    // TODO(jiahaog): Reflow within bounding box, or otherwise truncate.
                    self.stdout.queue(Print(semantics.as_ref().unwrap()))?;
                }
            }
        }

        self.stdout.flush()?;

        self.lines = lines;

        Ok(())
    }
}

#[derive(PartialEq, Eq, Clone)]
struct TerminalCell {
    top: Color,
    bottom: Color,
    semantics: Option<String>,
}

fn do_vecs_match<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    let matching = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
    matching == a.len() && matching == b.len()
}

const BLOCK_UPPER: char = '▀';
