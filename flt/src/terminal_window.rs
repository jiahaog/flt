//! This should be the only file in this crate which depends on [crossterm]
//! functionality beyond the data classes.

use crate::event::PlatformEvent;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::style::{Color, Print, PrintStyledContent, Stylize};
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{ErrorKind, ExecutableCommand, QueueableCommand};
use flutter_sys::Pixel;
use std::collections::{HashMap, VecDeque};
use std::io::{stdout, Stdout, Write};
use std::ops::Add;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

/// Lines to reserve the terminal for logging.
const LOGGING_WINDOW_HEIGHT: usize = 4;

pub struct TerminalWindow {
    stdout: Stdout,
    // Tuple represents the (top, bottom).
    prev_drawn: Vec<Vec<Option<(Pixel, Pixel)>>>,
    prev_drawn_semantics: HashMap<(usize, usize), String>,
    logs: VecDeque<String>,
    // Coordinates of semantics is represented in the "external" height.
    // See [to_external_height].
    semantics: HashMap<(usize, usize), String>,

    // Switches for debugging.
    simple_output: bool,
    alternate_screen: bool,
    pub(crate) log_events: bool,
}

impl Drop for TerminalWindow {
    fn drop(&mut self) {
        if !self.simple_output {
            self.stdout.execute(DisableMouseCapture).unwrap();
            disable_raw_mode().unwrap();

            // Show cursor.
            self.stdout.execute(Show).unwrap();

            if self.alternate_screen {
                self.stdout.execute(LeaveAlternateScreen).unwrap();
            }
            // Add a newline char so any other subsequent logs appear on the next line.
            self.stdout.execute(Print("\n")).unwrap();
        }
    }
}

impl TerminalWindow {
    pub(crate) fn new(
        simple_output: bool,
        alternate_screen: bool,
        log_events: bool,
        event_sender: Sender<PlatformEvent>,
    ) -> Self {
        let mut stdout = stdout();

        if !simple_output {
            if alternate_screen {
                // This causes the terminal to be output on an alternate buffer.
                stdout.execute(EnterAlternateScreen).unwrap();
            }

            // Hide cursor.
            stdout.execute(Hide).unwrap();

            enable_raw_mode().unwrap();
            stdout.execute(EnableMouseCapture).unwrap();
        }

        thread::spawn(move || {
            let mut should_run = true;
            while should_run {
                let event = read().unwrap();
                let event = normalize_event_height(event);
                should_run = event_sender
                    .send(PlatformEvent::TerminalEvent(event))
                    .is_ok();
            }
        });

        Self {
            stdout,
            prev_drawn: vec![],
            prev_drawn_semantics: HashMap::new(),
            logs: VecDeque::new(),
            semantics: HashMap::new(),
            simple_output,
            alternate_screen,
            log_events,
        }
    }

    pub(crate) fn size(&self) -> (usize, usize) {
        let (width, height) = terminal::size().unwrap();
        let (width, height) = (width as usize, height as usize);

        // Space for the logging window.
        let height = height - LOGGING_WINDOW_HEIGHT;

        (width, to_external_height(height))
    }

    pub(crate) fn update_semantics(&mut self, label_positions: Vec<((usize, usize), String)>) {
        // TODO(jiahaog): This is slow.
        self.semantics = label_positions.into_iter().collect();
    }

    pub(crate) fn draw(
        &mut self,
        mut pixel_grid: Vec<Vec<Pixel>>,
        (x_offset, y_offset): (isize, isize),
        prev_frame_duration: Duration,
    ) -> Result<(), ErrorKind> {
        // TODO(jiahaog): Stub out stdout instead so more things actually happen.
        if self.simple_output {
            return Ok(());
        }

        let width = pixel_grid.first().map_or(0, |row| row.len());

        // Always process an even number of rows.
        if pixel_grid.len() % 2 != 0 {
            pixel_grid.extend(vec![vec![Pixel::zero(); width]]);
        }

        let (width, height) = self.size();
        // Internal height
        let height = height / 2;

        let mut current: Vec<Vec<Option<(Pixel, Pixel)>>> = vec![];
        let mut current_semantics = HashMap::new();

        for term_y in 0..height {
            current.push(vec![]);

            let pixel_y = (y_offset + term_y as isize) * 2;
            if pixel_y < 0 || pixel_y as usize >= pixel_grid.len() {
                self.stdout.queue(MoveTo(0, term_y as u16))?;
                self.stdout.queue(Clear(ClearType::CurrentLine))?;
                current.last_mut().unwrap().push(None);
                continue;
            }
            let pixel_y = pixel_y as usize;

            let current_row = current.last_mut().unwrap();

            for term_x in 0..width {
                let pixel_x = x_offset + term_x as isize;

                if pixel_x < 0 || pixel_x as usize >= pixel_grid[pixel_y].len() {
                    self.stdout.queue(MoveTo(term_x as u16, term_y as u16))?;
                    self.stdout.queue(Print(" "))?;
                    current_row.push(None);
                    continue;
                }
                let pixel_x = pixel_x as usize;

                self.stdout.queue(MoveTo(term_x as u16, term_y as u16))?;

                let top_pixel = pixel_grid[pixel_y][pixel_x];
                let bottom_pixel = pixel_grid[pixel_y + 1][pixel_x];
                current_row.push(Some((top_pixel, bottom_pixel)));

                let block = BLOCK_UPPER
                    .with(to_color(&top_pixel))
                    .on(to_color(&bottom_pixel));

                // TODO(jiahaog): Skipping the optimizations when semantics are enabled is horribly slow.
                if self.semantics.len() == 0
                    && term_y < self.prev_drawn.len()
                    && term_x < self.prev_drawn[term_y].len()
                {
                    let prev = self.prev_drawn[term_y][term_x];
                    if prev.is_some() && prev.unwrap() == (top_pixel, bottom_pixel) {
                        continue;
                    }
                }

                self.stdout.queue(PrintStyledContent(block))?;
            }

            for ((pixel_x, pixel_y), label) in self.semantics.iter() {
                let term_x = *pixel_x as isize - x_offset;
                // Division here actually truncates and may cause labels to be hidden if they
                // are on consecutive rows.
                let term_y = (*pixel_y as isize - y_offset) / 2;

                if term_x < 0 || term_x as usize >= width || term_y < 0 || term_y as usize >= height
                {
                    continue;
                }
                let term_x = term_x as usize;
                let term_y = term_y as usize;

                current_semantics.insert((term_x, term_y), label);

                if let Some(prev_label) = self.prev_drawn_semantics.get(&(term_x, term_y)) {
                    if prev_label == label {
                        continue;
                    }
                }

                self.stdout.queue(MoveTo(term_x as u16, term_y as u16))?;
                self.stdout.queue(Print(label))?;
            }
        }

        self.stdout.queue(MoveTo(0, 0))?;
        self.stdout
            .queue(Print(format!("{:>3?}", prev_frame_duration.as_millis())))?;

        self.stdout.flush()?;
        self.prev_drawn = current;

        Ok(())
    }

    pub(crate) fn log(&mut self, message: String) {
        if self.simple_output {
            println!("{message}");
        }
        if self.logs.len() == LOGGING_WINDOW_HEIGHT {
            self.logs.pop_front();
        }
        self.logs.push_back(message);
    }
}

#[derive(PartialEq, Eq, Clone)]
struct TerminalCell {
    top: Color,
    bottom: Color,
    semantics: Option<String>,
}

const BLOCK_UPPER: char = '▀';

/// Translates from a "Internal" height to a "External" height.
///
/// "External" height is the height seen by users of this class.
/// "Internal" height is the height actually used when reading / writing to the
/// terminal.
///
/// Translation is needed as the terminal drawing strategy merges two lines of
/// pixels (seen to external users) into one line when written to the terminal.
fn to_external_height<T: Add<Output = T> + Copy>(internal_height: T) -> T {
    internal_height + internal_height
}

fn normalize_event_height(event: Event) -> Event {
    match event {
        Event::Resize(columns, rows) => {
            let rows = rows - LOGGING_WINDOW_HEIGHT as u16;
            Event::Resize(columns, to_external_height(rows))
        }
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
