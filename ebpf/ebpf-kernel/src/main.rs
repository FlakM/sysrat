#![no_std]
#![no_main]

#[allow(clippy::all)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[rustfmt::skip]
mod vmlinux;

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_task_btf,
        bpf_get_current_uid_gid, bpf_ktime_get_ns, bpf_probe_read_user,
        bpf_probe_read_user_str_bytes,
    },
    macros::{map, tracepoint},
    maps::{PerCpuArray, RingBuf},
    programs::TracePointContext,
};
use ebpf_common::{Event, ARG_COUNT, ARG_SIZE, ENV_COUNT};

#[repr(C)]
pub struct SysEnterExecve {
    // Tracepoint header fields.
    pub common_type: u16,
    pub common_flags: u8,
    pub common_preempt_count: u8,
    pub common_pid: i32,
    // Additional syscall-specific field.
    pub __syscall_nr: i32,
    // Execve-specific fields:
    pub filename: *const u8,
    pub argv: *const *const u8,
    pub envp: *const *const u8,
}

#[tracepoint(name = "sys_enter_execve", category = "syscalls")]
pub fn sys_enter_execve(ctx: TracePointContext) -> u32 {
    match try_enter_execve(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret as u32,
    }
}

#[map]
pub static BUF: PerCpuArray<Event> = PerCpuArray::with_max_entries(1, 0);

#[map(name = "RINGBUF")]
static mut RINGBUF: RingBuf = RingBuf::with_byte_size(256, 0);

// Implemention based on the suspection from here: https://github.com/notashes/syspection/blob/e5756aec507c2a9097331393b534392412c63d9b/syspection-ebpf/src/main.rs#L70
fn try_enter_execve(ctx: TracePointContext) -> Result<u32, i64> {
    // Get process info.
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let uid = bpf_get_current_uid_gid() as u32;
    let gid = (bpf_get_current_uid_gid() >> 32) as u32;
    let task = unsafe { bpf_get_current_task_btf() as *const vmlinux::task_struct };
    let real_parent = unsafe { (*task).real_parent };
    let ppid = unsafe { (*real_parent).pid } as u32;
    let timestamp = unsafe { bpf_ktime_get_ns() };

    // Get the comm (process name).
    let comm = match bpf_get_current_comm() {
        Ok(c) => c,
        Err(ret) => return Err(ret),
    };

    // Read the tracepoint data into our SysEnterExecve struct.
    let data: SysEnterExecve = unsafe { ctx.read_at(0).map_err(|_| -1)? };

    let event_ref = unsafe {
        let ptr = BUF.get_ptr_mut(0).ok_or(0)?;
        &mut *ptr
    };

    let argv = data.argv;
    for i in 0..ARG_COUNT {
        let arg_ptr = unsafe { bpf_probe_read_user(argv.offset(i as isize)) }?;

        if arg_ptr.is_null() {
            break;
        }

        unsafe {
            bpf_probe_read_user_str_bytes(arg_ptr, &mut event_ref.args[i as usize])
                .unwrap_or_default()
        };
    }

    let envp = data.envp;
    for env in 0..ENV_COUNT {
        let env_ptr = unsafe { bpf_probe_read_user(envp.offset(env as isize)) }?;

        if env_ptr.is_null() {
            break;
        }

        unsafe {
            bpf_probe_read_user_str_bytes(env_ptr, &mut event_ref.envs[env as usize])
                .unwrap_or_default()
        };
    }

    event_ref.timestamp = timestamp;
    event_ref.uid = uid;
    event_ref.gid = gid;
    event_ref.pid = pid;
    event_ref.ppid = ppid;
    event_ref.comm = comm;

    unsafe {
        submit(event_ref);
    }

    Ok(0)
}

#[inline]
unsafe fn submit(event: &mut Event) {
    if let Some(mut buf) = RINGBUF.reserve::<Event>(0) {
        buf.write(*event);
        buf.submit(0);
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
