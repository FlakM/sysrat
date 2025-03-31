#![no_std]

use core::fmt::{self, Formatter};


pub const MAX_PATH_LEN: usize = 512;


pub const ARG_SIZE: usize = 64;
pub const ARG_COUNT: usize = 20;


pub const ENV_SIZE: usize = 100;
pub const ENV_COUNT: usize = 20;

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub timestamp: u64, // nanoseconds since boot
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
    pub ppid: u32,
    pub comm: [u8; 16],
    pub args: [[u8; ARG_SIZE]; ARG_COUNT],
    pub envs: [[u8; ENV_SIZE]; ENV_COUNT],
}

impl core::fmt::Display for Event {
    // include envs and args
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let comm = core::str::from_utf8(&self.comm).unwrap_or_default();
        write!(f, "{} ({}): ", comm, self.pid)?;
        for arg in &self.args {
            let arg = core::str::from_utf8(arg).unwrap_or_default();
            write!(f, "{} ", arg)?;
        }
        for env in &self.envs {
            let env = core::str::from_utf8(env).unwrap_or_default();
            write!(f, "{} ", env)?;
        }
        Ok(())
    }
}

