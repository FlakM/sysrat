use crate::app::App;

pub mod app;
pub mod event;
pub mod process_service;
pub mod ui;

use chrono::NaiveDateTime;
use colored::{Color, Colorize};
//use colored::{Color, Colorize};
use duct::cmd;
use process_service::ProcessService;
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

    let _thread = std::thread::spawn(move || {
        // Spawn the dtrace command.
        // Redirect stderr to stdout so we can read both from the same stream.
        #[cfg(target_os = "macos")]
        let child_expression = cmd!("bash", "-c", "dtrace -s ./execsnoop.d");
        #[cfg(target_os = "linux")]
        let child_expression = cmd!("bash", "-c", "bpftrace -q execsnoop.bpf");

        let reader = child_expression.reader().unwrap();
        let reader = BufReader::new(reader);

        let process_service = ProcessService::new();
        // Print each line as soon as it is received.
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let process = match parse_line(&l, &process_service) {
                        Ok(p) => p,
                        Err(_e) => {
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
    pub comm: String,
    pub args: String,
    pub timestamp: NaiveDateTime,
    pub username: Option<String>,
}

impl ProcessExecution {
    fn ref_array(&self) -> [String; 6] {
        [
            self.timestamp.to_string(),
            self.username.clone().unwrap_or_default(),
            self.pid.to_string(),
            self.ppid.to_string(),
            self.comm.to_string(),
            self.args.bold().to_string(),
        ]
    }
}

impl Display for ProcessExecution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ref_array = self.ref_array();
        write!(
            f,
            "{} {} {} {} {} {}",
            ref_array[0].blue(),
            ref_array[1].blue(),
            ref_array[2].blue(),
            ref_array[3].blue(),
            ref_array[4].blue(),
            ref_array[5].blue()
        )
    }
}

// time,uid,pid,ppid,comm,args
fn parse_line(line: &str, process_service: &ProcessService) -> anyhow::Result<ProcessExecution> {
    let parts: Vec<&str> = line.split(",").collect();
    let timestamp = NaiveDateTime::parse_from_str(parts[0], FORMAT)?;
    let uid: u32 = parts[1].parse()?;
    let pid = parts[2].parse()?;
    let ppid = parts[3].parse()?;
    let comm = parts[4].trim().to_string();
    let args = parts[5].trim().to_string();

    let username = process_service
        .get_user_by_id(uid as usize)
        .map(|s| s.to_string());

    Ok(ProcessExecution {
        pid,
        ppid,
        comm,
        args,
        timestamp,
        username,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing_line() {
        let process_service = ProcessService::new();
        let line = "2025 Mar 25 21:16:01,1000,12681,3784,systemd,/nix/store/w9qcpyhjrxsqrps91wkz8r4mqvg9zrxc-systemd-256.10/lib/systemd/systemd-executor --deserialize 47 --log-level info --log-target auto";
        let process = parse_line(line, &process_service).unwrap();
        assert_eq!(process.pid, 12681);
        assert_eq!(process.ppid, 3784);
        assert_eq!(process.comm, "systemd");
        assert_eq!(
            process.args,
            "/nix/store/w9qcpyhjrxsqrps91wkz8r4mqvg9zrxc-systemd-256.10/lib/systemd/systemd-executor --deserialize 47 --log-level info --log-target auto"
        );
    }
}
