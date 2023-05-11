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
use std::iter::zip;
use std::ops::Add;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Instant;

/// Lines to reserve the terminal for logging.
const LOGGING_WINDOW_HEIGHT: usize = 4;

pub struct TerminalWindow {
    stdout: Stdout,
    lines: Vec<Vec<TerminalCell>>,
    logs: VecDeque<String>,
    // Coordinates of semantics is represented in the "external" height.
    // See [to_external_height].
    semantics: HashMap<(usize, usize), String>,

    // Switches for debugging.
    simple_output: bool,
    alternate_screen: bool,
    showing_help: bool,
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
            lines: vec![],
            logs: VecDeque::new(),
            semantics: HashMap::new(),
            simple_output,
            showing_help: false,
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
        pixel_grid: Vec<Vec<Pixel>>,
        (x_offset, y_offset): (isize, isize),
    ) -> Result<(), ErrorKind> {
        // TODO(jiahaog): Stub out stdout instead so more things actually happen.
        if self.simple_output {
            return Ok(());
        }

        if self.showing_help {
            return Ok(());
        }

        let start_instant = Instant::now();

        // TODO(jiahaog): Enable this assertion. This breaks when zooming out.
        // assert_eq!(pixel_grid.len() % 2, 0, "Drawn pixels should always be a multiple of two as the terminal height is multiplied by two before being provided to flutter.");

        // TODO(jiahaog): This implementation is horrible and should be rewritten.

        let grid_with_semantics: Vec<Vec<(Pixel, Option<String>)>> = pixel_grid
            .into_iter()
            .enumerate()
            .map(|(y, row)| {
                row.into_iter()
                    .enumerate()
                    .map(|(x, pixel)| (pixel, self.semantics.get(&(x, y)).cloned()))
                    .collect()
            })
            .collect();

        let (pixel_width, pixel_height) = self.size();

        let grid_for_terminal: Vec<Vec<(Pixel, Option<String>)>> = (0..pixel_height)
            .map(|y| {
                let y = y_offset + y as isize;
                if y < 0 || y >= grid_with_semantics.len() as isize {
                    return vec![(Pixel::zero(), None); pixel_width];
                }

                let y = y as usize;

                (0..pixel_width)
                    .map(|x| {
                        let x = x_offset + x as isize;

                        if x < 0 || x >= grid_with_semantics.first().unwrap().len() as isize {
                            return (Pixel::zero(), None);
                        }

                        let x = x as usize;

                        grid_with_semantics[y][x].clone()
                    })
                    .collect()
            })
            .collect();

        assert!(grid_for_terminal.len() == pixel_height);
        assert!(grid_for_terminal.first().unwrap().len() == pixel_width);

        // Each element is one pixel, but when it is rendered to the terminal,
        // two rows share one character, using the unicode BLOCK characters.
        let lines = (0..grid_for_terminal.len())
            .step_by(2)
            .map(|y| {
                // TODO(jiahaog): Avoid the borrow here.
                let tops = &grid_for_terminal[y];
                let bottoms = &grid_for_terminal[y + 1];

                zip(tops, bottoms)
                    .map(
                        |((top_pixel, top_semantics), (bottom_pixel, bottom_semantics))| {
                            // Find the semantic labels for the current cell.
                            let semantics = match (top_semantics, bottom_semantics) {
                                (None, None) => None,
                                (None, right) => right.clone(),
                                (left, None) => left.clone(),
                                // Use the longest.
                                (Some(left), Some(right)) => Some(if left.len() > right.len() {
                                    left.clone()
                                } else {
                                    right.clone()
                                }),
                            };

                            TerminalCell {
                                top: to_color(&top_pixel),
                                bottom: to_color(&bottom_pixel),
                                semantics,
                            }
                        },
                    )
                    .collect::<Vec<TerminalCell>>()
            })
            .collect::<Vec<Vec<TerminalCell>>>();

        // Refreshing the entire terminal (with the clear char) and outputting
        // everything on every iteration is costly and causes the terminal to
        // flicker.
        //
        // Instead, only "re-render" different characters, if it is different from
        // the previous frame.

        // Means that the screen dimensions has changed.
        if self.lines.len() != lines.len() {
            // As the next zip needs to be a zip_longest.
            self.lines = vec![vec![]; lines.len()];
        }

        for (y, (prev, current)) in zip(&self.lines, &lines).enumerate() {
            for (
                x,
                current_cell @ TerminalCell {
                    top,
                    bottom,
                    semantics: _,
                },
            ) in current.into_iter().enumerate()
            {
                if prev
                    .get(x)
                    .filter(|prev_cell| prev_cell == &current_cell)
                    .is_some()
                {
                    continue;
                }
                self.stdout.queue(MoveTo(x as u16, y as u16))?;
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

        {
            assert!(self.logs.len() <= LOGGING_WINDOW_HEIGHT);

            let (_, terminal_height) = terminal::size()?;

            for i in 0..LOGGING_WINDOW_HEIGHT {
                let y = terminal_height as usize - LOGGING_WINDOW_HEIGHT + i;

                self.stdout.queue(MoveTo(0, y as u16))?;
                self.stdout
                    .queue(Clear(crossterm::terminal::ClearType::CurrentLine))?;
                if let Some(line) = self.logs.get(i) {
                    self.stdout.queue(Print(line))?;
                }
            }

            let draw_duration = Instant::now().duration_since(start_instant);

            let hint_and_fps = format!("{HELP_HINT} [{}]", draw_duration.as_millis());
            self.stdout.queue(MoveTo(
                (pixel_width - hint_and_fps.len()) as u16,
                (terminal_height - 1) as u16,
            ))?;
            self.stdout.queue(Print(hint_and_fps))?;
        }

        self.stdout.flush()?;
        self.lines = lines;

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

    pub(crate) fn toggle_show_help(&mut self) -> Result<(), ErrorKind> {
        self.showing_help = !self.showing_help;

        self.stdout.execute(Clear(ClearType::All))?;
        self.mark_dirty();

        if self.showing_help {
            self.stdout.queue(MoveTo(0, 0))?;
            self.stdout.queue(Print("Ctrl + r: Reset the viewport."))?;
            self.stdout.queue(MoveTo(0, 2))?;
            self.stdout
                .queue(Print("Ctrl + 5: Increase the pixel ratio."))?;
            self.stdout.queue(MoveTo(0, 4))?;
            self.stdout
                .queue(Print("Ctrl + 4: Decrease the pixel ratio."))?;
            self.stdout.queue(MoveTo(0, 6))?;
            self.stdout
                .queue(Print("Ctrl + Mouse Scroll: Zoom in / out."))?;
            self.stdout.queue(MoveTo(0, 8))?;
            self.stdout
                .queue(Print("Ctrl + Mouse Click and Drag: Pan the viewport. Some terminals might not allow this."))?;
            self.stdout.queue(MoveTo(0, 10))?;
            self.stdout.queue(Print(
                "Ctrl + z: Show semantic labels (very experimental and jank).",
            ))?;
            self.stdout.queue(MoveTo(0, 12))?;
            self.stdout.queue(Print("?: Toggle help."))?;

            self.stdout.queue(MoveTo(0, 14))?;
            self.stdout.queue(Print("Tips: Changing the current terminal emulator's text size will make things look a lot better. "))?;
            self.stdout.queue(MoveTo(0, 15))?;
            self.stdout.queue(Print(
                "But the code is suboptimal and it might lead to more jank.",
            ))?;
            self.stdout.flush()?;
        }
        Ok(())
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.lines.clear();
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

const HELP_HINT: &str = "? for help";
