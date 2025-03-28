use std::{ffi::OsStr, path::Path};

use sysinfo::{Pid, System, Uid, Users};

#[derive(Debug)]
pub struct ProcessService {
    system: System,
    users: Users,
}

impl ProcessService {
    pub fn new() -> Self {
        let mut ret = Self {
            system: System::new_all(),
            users: Users::new(),
        };
        ret.update();
        ret
    }

    fn update(&mut self) {
        self.system.refresh_all();
        self.users.refresh();
    }

    pub fn get_process(&self, pid: usize) -> Option<RenderedProcess> {
        let p = self.system.process(Pid::from(pid))?;

        let cwd = p.cwd();

        let user_id = p
            .user_id()
            .and_then(|u| self.users.get_user_by_id(u))
            .map(|u| u.name());
        let env = p
            .environ()
            .into_iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        Some(RenderedProcess {
            cwd,
            user: user_id.unwrap_or(""),
            env,
        })
    }

    pub fn get_user_by_id(&self, uid: usize) -> Option<&str> {
        let uid = Uid::try_from(uid).ok()?;
        self.users.get_user_by_id(&uid).map(|u| u.name())
    }
}

#[derive(Debug)]
pub struct RenderedProcess<'a> {
    cwd: Option<&'a Path>,
    user: &'a str,
    env: Vec<String>,
}

