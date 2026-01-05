use crossterm::{style::Print, ExecutableCommand};
use libc::{poll, pollfd, POLLIN, STDIN_FILENO};
use std::io::{Read, Stdout, Write};
use std::time::{Duration, Instant};

/// Checks if the terminal supports the Kitty graphics protocol.
///
/// This is done by sending a "query" command to the terminal and waiting for a response.
/// If the terminal supports the protocol, it will reply with a confirmation containing
/// the ID we sent.
pub fn kitty_graphics_supported(stdout: &mut Stdout) -> bool {
    // Check for Kitty support by sending a graphics query.
    // Breakdown of the escape sequence:
    // \x1b_G   : Start of Kitty graphics command (APC - Application Program Command).
    // i=31     : ID = 31. An arbitrary integer ID to identify this specific command/response.
    // a=q      : Action = query. We are asking the terminal if it supports graphics.
    // ;        : Separator between keys and payload (payload is empty here).
    // \x1b\\   : ST (String Terminator). Ends the command.
    let _ = stdout.execute(Print("\x1b_Gi=31,a=q;\x1b\\"));
    let _ = stdout.flush();

    let mut buf = [0u8; 1024];
    let mut idx = 0;
    let start = Instant::now();
    let mut detected = false;

    // We wait up to 100ms for a response.
    loop {
        if start.elapsed() > Duration::from_millis(100) {
            break;
        }

        let mut fds = [pollfd {
            fd: STDIN_FILENO,
            events: POLLIN,
            revents: 0,
        }];

        let ret = unsafe { poll(fds.as_mut_ptr(), 1, 10) };
        if ret > 0 {
            // Data available
            let read_bytes = std::io::stdin().read(&mut buf[idx..]);
            match read_bytes {
                Ok(n) if n > 0 => {
                    idx += n;
                    let s = String::from_utf8_lossy(&buf[..idx]);
                    // The terminal should respond with something containing "i=31" if it understood the query.
                    // e.g., \x1b_Gi=31;OK\x1b\
                    if s.contains("i=31") {
                        detected = true;
                        break;
                    }
                    // If we encounter the String Terminator, we stop reading.
                    if s.ends_with("\x1b\\") {
                        break;
                    }
                }
                _ => break,
            }
        }
    }
    detected
}
