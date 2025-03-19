use crate::app::App;

pub mod app;
pub mod event;
pub mod ui;

use chrono::NaiveDateTime;
use colored::Colorize;
//use colored::{Color, Colorize};
use duct::cmd;
use std::{
    fmt::{self, Display},
    io::{BufRead, BufReader},
};

const FORMAT: &str = "%Y %b %d %H:%M:%S";

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app = App::new();
    let sender = app.events.sender.clone();

    let thread = std::thread::spawn(move || {
        // Spawn the dtrace command.
        // Redirect stderr to stdout so we can read both from the same stream.
        #[cfg(target_os = "macos")]
        let child_expression = cmd!("bash", "-c", "dtrace -s ./execsnoop.d");
        #[cfg(target_os = "linux")]
        let child_expression = cmd!("bash", "-c", "bpftrace -q execsnoop.bpf");

        let reader = child_expression.reader().unwrap();
        let reader = BufReader::new(reader);

        // Print each line as soon as it is received.
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let process = match parse_line(&l) {
                        Ok(p) => p,
                        Err(e) => {
                            panic!("Error parsing line: [{}] \n {}", l, e);
                            continue;
                        }
                    };
                    sender
                        .send(event::Event::App(event::AppEvent::NewProcess(process)))
                        .unwrap();
                }
                Err(e) => {
                    eprintln!("Error reading line: {}", e);
                    break;
                }
            }
        }
    });

    let result = app.run(terminal);

    ratatui::restore();

    result
}

#[derive(Debug, Clone)]
pub struct ProcessExecution {
    pub pid: u32,
    pub ppid: u32,
    pub command: String,
    pub timestamp: NaiveDateTime,
}

const COLORS: [(u8, u8, u8); 50] = [
    (255, 0, 0),     // Red
    (0, 255, 0),     // Lime
    (0, 0, 255),     // Blue
    (255, 255, 0),   // Yellow
    (0, 255, 255),   // Cyan
    (255, 0, 255),   // Magenta
    (192, 192, 192), // Silver
    (128, 128, 128), // Gray
    (128, 0, 0),     // Maroon
    (128, 128, 0),   // Olive
    (0, 128, 0),     // Green
    (128, 0, 128),   // Purple
    (0, 128, 128),   // Teal
    (0, 0, 128),     // Navy
    (255, 165, 0),   // Orange
    (255, 20, 147),  // Deep Pink
    (218, 112, 214), // Orchid
    (75, 0, 130),    // Indigo
    (240, 230, 140), // Khaki
    (173, 216, 230), // Light Blue
    (0, 191, 255),   // Deep Sky Blue
    (70, 130, 180),  // Steel Blue
    (100, 149, 237), // Cornflower Blue
    (123, 104, 238), // Medium Slate Blue
    (72, 61, 139),   // Dark Slate Blue
    (138, 43, 226),  // Blue Violet
    (199, 21, 133),  // Medium Violet Red
    (255, 105, 180), // Hot Pink
    (255, 182, 193), // Light Pink
    (205, 92, 92),   // Indian Red
    (244, 164, 96),  // Sandy Brown
    (210, 105, 30),  // Chocolate
    (160, 82, 45),   // Sienna
    (255, 228, 196), // Bisque
    (255, 222, 173), // Navajo White
    (255, 160, 122), // Light Salmon
    (250, 128, 114), // Salmon
    (233, 150, 122), // Dark Salmon
    (255, 127, 80),  // Coral
    (240, 128, 128), // Light Coral
    (255, 99, 71),   // Tomato
    (255, 69, 0),    // Orange Red
    (220, 20, 60),   // Crimson
    (178, 34, 34),   // Firebrick
    (139, 0, 0),     // Dark Red
    (0, 100, 0),     // Dark Green
    (46, 139, 87),   // Sea Green
    (60, 179, 113),  // Medium Sea Green
    (34, 139, 34),   // Forest Green
    (50, 205, 50),   // Lime Green
];

impl ProcessExecution {
    fn ref_array(&self) -> [String; 4] {
        let ppid = self.ppid;
        let (r, g, b) = COLORS[(ppid % COLORS.len() as u32) as usize];
        [
            self.timestamp.to_string(),
            self.pid.to_string(),
            self.ppid.to_string().truecolor(r, g, b).to_string(),
            self.command.bold().to_string(),
        ]
    }
}

impl Display for ProcessExecution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {} (pid: {}, ppid: {})",
            self.timestamp.format(FORMAT).to_string().green(),
            self.command,
            self.pid,
            self.ppid
        )
    }
}

fn parse_line(line: &str) -> anyhow::Result<ProcessExecution> {
    let parts: Vec<&str> = line.split(",").collect();
    let timestamp = NaiveDateTime::parse_from_str(parts[0], FORMAT)?;
    let pid = parts[1].parse()?;
    let ppid = parts[2].parse()?;
    let command = parts[3].trim().to_string();
    Ok(ProcessExecution {
        pid,
        ppid,
        command,
        timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing_line() {
        let line = "2025 Mar 16 13:42:53,3925,341,/usr/sbin/netstat -na";
        let process = parse_line(line).unwrap();
        assert_eq!(process.pid, 3925);
        assert_eq!(process.ppid, 341);
        assert_eq!(process.command, "/usr/sbin/netstat -na");
    }
}
