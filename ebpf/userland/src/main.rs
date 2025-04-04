use aya::programs::TracePoint;
use ebpf_common::Event;
use log::info;
#[rustfmt::skip]
use log::{debug, warn};

use aya::maps::RingBuf;
use std::convert::TryFrom;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Bump the memlock rlimit. This is needed for older kernels that don't use the
    // new memcg based accounting, see https://lwn.net/Articles/837122/
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        debug!("remove limit on locked memory failed, ret is: {}", ret);
    }

    // This will include your eBPF object file as raw bytes at compile-time and load it at
    // runtime. This approach is recommended for most real-world use cases. If you would
    // like to specify the eBPF program at runtime rather than at compile-time, you can
    // reach for `Bpf::load_file` instead.
    let mut ebpf = aya::Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/ebpf-kernel-bpf"
    )))?;
    if let Err(e) = aya_log::EbpfLogger::init(&mut ebpf) {
        // This can happen if you remove all log statements from your eBPF program.
        warn!("failed to initialize eBPF logger: {}", e);
    }
    let program: &mut TracePoint = ebpf.program_mut("sys_enter_execve").unwrap().try_into()?;
    program.load()?;
    program.attach("syscalls", "sys_enter_execve")?;

    let mut ring_buf = RingBuf::try_from(ebpf.map_mut("RINGBUF").unwrap()).unwrap();

    // TODO: use async fd polling like here: https://github.com/zz85/profile-bee/blob/c311ffa6833ee408ee62cf75d23620480e0a97ee/profile-bee/bin/profile-bee.rs#L232-L260
    loop {
        if let Some(item) = ring_buf.next() {
           let event: Event =  unsafe { *item.as_ptr().cast() };
           info!("event: {}", event);
        }
    }

    // await Ctrl-C without tokio
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })?;
    info!("waiting for Ctrl-C");

    rx.recv().unwrap();

    Ok(())
}
