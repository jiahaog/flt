//! This should be the only file in this crate which depends on [crossterm]
//! functionality beyond the data classes.

use crate::event::PlatformEvent;
use base64::prelude::*;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::style::{Color, Print, PrintStyledContent, Stylize};
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, window_size, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{ExecutableCommand, QueueableCommand};
use libc::{ftruncate, shm_open, shm_unlink, O_CREAT, O_RDWR, O_TRUNC};
use memmap2::MmapMut;
use std::collections::{HashMap, VecDeque};
use std::io::{stdout, Stdout, Write};
use std::iter::zip;
use std::os::unix::io::FromRawFd;
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
    kitty_mode: bool,
    pixels_per_col: f64,
    pixels_per_row: f64,
    device_pixel_ratio: f64,
    shm_buffer: Option<SharedMemoryBuffer>,
    frame_count: u64,
}

struct SharedMemoryBuffer {
    name: String,
    raw_name: String,
    // We keep file to keep the FD open (though shm persists until unlinked/closed)
    // and to allow resizing (if we used File methods).
    // But we used libc::shm_open which returns raw fd.
    // We can wrap it in ManuallyDrop<File> or just raw fd.
    // memmap2 `map_mut` takes &File.
    #[allow(dead_code)]
    file: std::fs::File,
    map: Option<MmapMut>,
}

impl SharedMemoryBuffer {
    fn new(size: usize, suffix: u64) -> std::io::Result<Self> {
        let pid = std::process::id();
        let raw_name = format!("/flt_{}_{}", pid, suffix);
        let c_name = std::ffi::CString::new(raw_name.clone())?;

        // Ensure clean state
        unsafe { shm_unlink(c_name.as_ptr()) };

        let fd = unsafe { shm_open(c_name.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let file = unsafe { std::fs::File::from_raw_fd(fd) };

        if unsafe { ftruncate(fd, size as i64) } < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let map = unsafe { MmapMut::map_mut(&file)? };

        Ok(Self {
            name: BASE64_STANDARD.encode(&raw_name),
            raw_name,
            file,
            map: Some(map),
        })
    }
}

impl Drop for SharedMemoryBuffer {
    fn drop(&mut self) {
        if let Ok(c_name) = std::ffi::CString::new(self.raw_name.clone()) {
            unsafe { shm_unlink(c_name.as_ptr()) };
        }
    }
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
        kitty_mode: bool,
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

        let (pixels_per_col, pixels_per_row) = if kitty_mode {
            match window_size() {
                Ok(crossterm::terminal::WindowSize {
                    width: w_px,
                    height: h_px,
                    rows,
                    columns,
                }) if w_px > 0 && h_px > 0 && rows > 0 && columns > 0 => {
                    (w_px as f64 / columns as f64, h_px as f64 / rows as f64)
                }
                _ => (10.0, 20.0), // Fallback to a high-res default
            }
        } else {
            (1.0, 2.0)
        };

        let device_pixel_ratio = if kitty_mode {
            pixels_per_row.max(1.0) / 22.0
        } else {
            crate::constants::DEFAULT_PIXEL_RATIO
        };

        thread::spawn(move || {
            let mut should_run = true;
            while should_run {
                let event = read().unwrap();
                let event = normalize_event_height(event, pixels_per_col, pixels_per_row);
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
            kitty_mode,
            pixels_per_col,
            pixels_per_row,
            device_pixel_ratio,
            shm_buffer: None,
            frame_count: 0,
        }
    }

    pub(crate) fn device_pixel_ratio(&self) -> f64 {
        self.device_pixel_ratio
    }

    pub(crate) fn size(&self) -> (usize, usize) {
        let (width, height) = terminal::size().unwrap();
        let (width, height) = (width as usize, height as usize);

        // Space for the logging window.
        let height = height - LOGGING_WINDOW_HEIGHT;

        (
            (width as f64 * self.pixels_per_col).round() as usize,
            (height as f64 * self.pixels_per_row).round() as usize,
        )
    }

    pub(crate) fn update_semantics(&mut self, label_positions: Vec<((usize, usize), String)>) {
        // TODO(jiahaog): This is slow.
        self.semantics = label_positions.into_iter().collect();
    }
    pub(crate) fn draw(
        &mut self,
        buffer: Vec<u8>,
        width: usize,
        height: usize,
        (x_offset, y_offset): (isize, isize),
    ) -> Result<(), std::io::Error> {
        // TODO(jiahaog): Stub out stdout instead so more things actually happen.
        if self.simple_output {
            return Ok(());
        }

        if self.showing_help {
            return Ok(());
        }

        if self.kitty_mode {
            return self.draw_kitty(buffer, width, height);
        }

        let start_instant = Instant::now();

        let (cell_cols, cell_rows) = terminal::size().unwrap();
        let cell_rows = cell_rows as usize - LOGGING_WINDOW_HEIGHT;
        let cell_cols = cell_cols as usize;

        let mut lines = Vec::with_capacity(cell_rows);

        for y in (0..cell_rows).step_by(1) {
            let mut row_cells = Vec::with_capacity(cell_cols);

            for x in 0..cell_cols {
                let pixel_x = x_offset + x as isize;
                let pixel_y_top = y_offset + (y * 2) as isize;
                let pixel_y_bot = y_offset + (y * 2 + 1) as isize;

                let top_pixel = get_pixel(&buffer, width, height, pixel_x, pixel_y_top);
                let bot_pixel = get_pixel(&buffer, width, height, pixel_x, pixel_y_bot);

                let semantics = None;

                row_cells.push(TerminalCell {
                    top: to_color_from_bytes(top_pixel),
                    bottom: to_color_from_bytes(bot_pixel),
                    semantics,
                });
            }
            lines.push(row_cells);
        }

        if self.lines.len() != lines.len() {
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
                (cell_cols - hint_and_fps.len()) as u16,
                (terminal_height - 1) as u16,
            ))?;
            self.stdout.queue(Print(hint_and_fps))?;
        }

        self.stdout.flush()?;
        self.lines = lines;

        Ok(())
    }
    fn draw_kitty(
        &mut self,
        mut buffer: Vec<u8>,
        width: usize,
        height: usize,
    ) -> Result<(), std::io::Error> {
        if buffer.is_empty() {
            return Ok(());
        }

        let start = Instant::now();

        // Color swap (BGRA -> RGBA)
        if !cfg!(target_os = "macos") {
            for chunk in buffer.chunks_exact_mut(4) {
                chunk.swap(0, 2);
            }
        }

        // Generate unique SHM segment for this frame
        self.frame_count += 1;
        let needed_size = buffer.len();

        let mut new_shm = SharedMemoryBuffer::new(needed_size, self.frame_count)?;

        // Write to SHM
        if let Some(map) = &mut new_shm.map {
            map[0..needed_size].copy_from_slice(&buffer);
            let _ = map.flush();
        }

        // Send Command
        self.stdout.queue(MoveTo(0, 0))?;

        // Initialize transfer.
        // f=32: 32-bit RGBA
        // s={pixel_width},v={pixel_height}: dimensions
        // a=T: transmit and display
        // q=2: quiet mode (no response)
        // i=1: image ID for overwriting
        // t=s (Shared Memory)
        // Payload is the encoded name of the NEW SHM segment
        let code = format!(
            "\x1b_Gf=32,s={},v={},a=T,q=2,i=1,t=s;{}\x1b\\",
            width, height, &new_shm.name
        );
        self.stdout.queue(Print(code))?;
        self.stdout.flush()?;

        // Store the new SHM buffer.
        // This drops the previous one (if any), which triggers shm_unlink.
        self.shm_buffer = Some(new_shm);

        {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open("/tmp/flt_frame_timings.log")
            {
                let _ = writeln!(f, "Frame time: {}ms", start.elapsed().as_millis());
            }
        }
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

    pub(crate) fn toggle_show_help(&mut self) -> Result<(), std::io::Error> {
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

fn normalize_event_height(event: Event, x_scale: f64, y_scale: f64) -> Event {
    match event {
        Event::Resize(columns, rows) => {
            let rows = rows - LOGGING_WINDOW_HEIGHT as u16;
            Event::Resize(
                (columns as f64 * x_scale).round() as u16,
                (rows as f64 * y_scale).round() as u16,
            )
        }
        Event::Mouse(mut mouse_event) => {
            mouse_event.column = (mouse_event.column as f64 * x_scale).round() as u16;
            mouse_event.row = (mouse_event.row as f64 * y_scale).round() as u16;
            Event::Mouse(mouse_event)
        }
        x => x,
    }
}

// Helper to get pixel from flat buffer safely
fn get_pixel(buffer: &[u8], width: usize, height: usize, x: isize, y: isize) -> Option<&[u8]> {
    if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
        return None;
    }
    let idx = (y as usize * width + x as usize) * 4;
    if idx + 4 <= buffer.len() {
        Some(&buffer[idx..idx + 4])
    } else {
        None
    }
}

fn to_color_from_bytes(pixel_bytes: Option<&[u8]>) -> Color {
    match pixel_bytes {
        Some(slice) => {
            // Assume engine format matches platform (BGRA on Linux, RGBA on Mac)
            // But we want Color::Rgb which is r, g, b.
            if cfg!(target_os = "macos") {
                Color::Rgb {
                    r: slice[0],
                    g: slice[1],
                    b: slice[2],
                }
            } else {
                Color::Rgb {
                    r: slice[2],
                    g: slice[1],
                    b: slice[0],
                }
            }
        }
        None => Color::Rgb { r: 0, g: 0, b: 0 },
    }
}

const HELP_HINT: &str = "? for help";
