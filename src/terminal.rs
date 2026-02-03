use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct EmbeddedTerminal {
    pty_pair: PtyPair,
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Box<dyn Write + Send>,
    running: Arc<Mutex<bool>>,
}

impl EmbeddedTerminal {
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let pty_pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 1000)));
        let writer = pty_pair.master.take_writer()?;
        let running = Arc::new(Mutex::new(false));

        Ok(Self {
            pty_pair,
            parser,
            writer,
            running,
        })
    }

    pub fn spawn_claude(&mut self, project_dir: &str, session_id: &str) -> Result<()> {
        let mut cmd = CommandBuilder::new("bash");
        cmd.args([
            "-c",
            &format!(
                "cd '{}' 2>/dev/null || cd ~; claude --resume {} --dangerously-skip-permissions",
                project_dir, session_id
            ),
        ]);

        let child = self.pty_pair.slave.spawn_command(cmd)?;
        *self.running.lock().unwrap() = true;

        // Start reader thread
        let mut reader = self.pty_pair.master.try_clone_reader()?;
        let parser = Arc::clone(&self.parser);
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut p) = parser.lock() {
                            p.process(&buf[..n]);
                        }
                    }
                    Err(_) => break,
                }
                if !*running.lock().unwrap() {
                    break;
                }
            }
            *running.lock().unwrap() = false;
        });

        // Don't wait for child - let it run in background
        drop(child);

        Ok(())
    }

    pub fn spawn_new_claude(&mut self) -> Result<()> {
        let mut cmd = CommandBuilder::new("claude");
        cmd.arg("--dangerously-skip-permissions");

        let child = self.pty_pair.slave.spawn_command(cmd)?;
        *self.running.lock().unwrap() = true;

        // Start reader thread
        let mut reader = self.pty_pair.master.try_clone_reader()?;
        let parser = Arc::clone(&self.parser);
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut p) = parser.lock() {
                            p.process(&buf[..n]);
                        }
                    }
                    Err(_) => break,
                }
                if !*running.lock().unwrap() {
                    break;
                }
            }
            *running.lock().unwrap() = false;
        });

        drop(child);

        Ok(())
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.pty_pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        if let Ok(mut p) = self.parser.lock() {
            p.set_size(rows, cols);
        }
        Ok(())
    }

    pub fn get_screen_with_styles(&self) -> Option<Vec<Vec<(char, vt100::Color, vt100::Color, bool)>>> {
        self.parser.lock().ok().map(|p| {
            let screen = p.screen();
            (0..screen.size().0)
                .map(|row| {
                    (0..screen.size().1)
                        .map(|col| {
                            let cell = screen.cell(row, col).unwrap();
                            let ch = cell.contents().chars().next().unwrap_or(' ');
                            let fg = cell.fgcolor();
                            let bg = cell.bgcolor();
                            let bold = cell.bold();
                            (ch, fg, bg, bold)
                        })
                        .collect()
                })
                .collect()
        })
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        self.parser.lock().ok().map(|p| p.screen().cursor_position())
    }

    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
        // Send Ctrl+C to terminate
        let _ = self.write(&[3]);
    }
}

impl Drop for EmbeddedTerminal {
    fn drop(&mut self) {
        self.stop();
    }
}
