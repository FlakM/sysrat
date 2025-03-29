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
        bpf_get_current_uid_gid, bpf_ktime_get_ns, bpf_probe_read_user_str_bytes,
    },
    macros::tracepoint,
    programs::TracePointContext,
};
use aya_log_ebpf::info;
use core::ptr;

// Constants – adjust as needed.
const MAX_ARGS: usize = 20;
const ARGS_BUF_SIZE: usize = 256;

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

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub timestamp: u64, // nanoseconds since boot
    pub uid: u32,
    pub pid: u32,
    pub ppid: u32,
    pub comm: [u8; 16],
    pub args: [u8; ARGS_BUF_SIZE],
}

#[tracepoint(name = "sys_enter_execve", category = "syscalls")]
pub fn sys_enter_execve(ctx: TracePointContext) -> u32 {
    match try_enter_execve(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret as u32,
    }
}

fn try_enter_execve(ctx: TracePointContext) -> Result<u32, i64> {
    // Read the tracepoint data into our SysEnterExecve struct.
    let data: SysEnterExecve = unsafe { ctx.read_at(0).map_err(|_| -1)? };

    // Get process info.
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let uid = bpf_get_current_uid_gid() as u32;
    let task = unsafe { bpf_get_current_task_btf() as *const vmlinux::task_struct };
    let real_parent = unsafe { (*task).real_parent };
    let ppid = unsafe { (*real_parent).pid };
    let timestamp = unsafe { bpf_ktime_get_ns() };

    // Get the comm (process name).
    let comm = match bpf_get_current_comm() {
        Ok(c) => c,
        Err(ret) => return Err(ret),
    };

    // Prepare a buffer for the concatenated arguments.
    let mut args_buf = [0u8; ARGS_BUF_SIZE];
    let mut off: usize = 0;

    // Instead of parsing filename from data.filename,
    // we parse the entire argv, taking argv[0] as the filename.
    for i in 0..MAX_ARGS {
        let arg_ptr = unsafe { ptr::read(data.argv.offset(i as isize)) };
        if arg_ptr.is_null() {
            break;
        }

        // For i > 0 (i.e. for arguments after argv[0]), insert a space separator.
        if i > 0 {
            if off >= ARGS_BUF_SIZE - 1 {
                break;
            }
            args_buf[off] = b' ';
            off += 1;
        }

        // Calculate remaining buffer space.
        let remaining = ARGS_BUF_SIZE - off;
        if remaining == 0 {
            break;
        }

        // Read the argument string directly into the remaining buffer.
        let arg_slice = match unsafe { bpf_probe_read_user_str_bytes(arg_ptr, &mut args_buf[off..]) } {
            Ok(slice) => slice,
            Err(_) => break,
        };

        // If the argument is empty or only a null terminator, break.
        if arg_slice.len() <= 1 {
            break;
        }

        // Exclude the null terminator from the length.
        let arg_len = arg_slice.len().saturating_sub(1);
        if off + arg_len > ARGS_BUF_SIZE {
            off = ARGS_BUF_SIZE;
            break;
        }
        off += arg_len;
    }

    // Log the event.
    let args_str = unsafe { core::str::from_utf8_unchecked(&args_buf[..off]) };
    let comm_str = unsafe { core::str::from_utf8_unchecked(&comm) };

    info!(
        &ctx,
        "{},{},{},{},{} {}", timestamp, uid, pid, ppid, comm_str, args_str
    );
    // Explicitly return here so that every code path terminates with this return.
    Ok(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
