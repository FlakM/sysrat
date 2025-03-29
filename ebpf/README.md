## About this module

This module uses aya-tool to generate bindings to `task_struct` in the Linux kernel.

```bash
aya-tool generate task_struct > ebpf/ebpf-kernel/src/vmlinux.rs
aya-tool generate execve_args > ebpf/ebpf-kernel/src/execve_args.rs

```
