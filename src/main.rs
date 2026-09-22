//! rbg - talk to the GPU driver through /dev/kfd.
//!
//! The syscalls and ioctls are hand-declared in the kfd module.
//! No crates.

mod kfd;

use std::fs;
use std::path::Path;

/// Find the first kfd node whose gpu_id is nonzero.
fn find_gpu_id() -> (u32, u32) {
    let base = Path::new("/sys/class/kfd/kfd/topology/nodes");
    let entries = fs::read_dir(base).unwrap_or_else(|e| {
        kfd::die("read_dir", e.raw_os_error().unwrap_or(-1))
    });
    for entry in entries.flatten() {
        let dir = entry.path();
        let id: u32 = match fs::read_to_string(dir.join("gpu_id"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
        {
            Some(v) => v,
            None => continue,
        };
        if id == 0 {
            continue;
        }
        let props =
            fs::read_to_string(dir.join("properties")).unwrap_or_else(|e| {
                kfd::die("read properties", e.raw_os_error().unwrap_or(-1))
            });
        let gfx: u32 = props
            .lines()
            .find_map(|l| {
                let mut it = l.split_whitespace();
                match (it.next(), it.next()) {
                    (Some("gfx_target_version"), Some(v)) => v.parse().ok(),
                    _ => None,
                }
            })
            .unwrap_or_else(|| kfd::die("no gfx_target_version", 0));
        return (id, gfx);
    }
    kfd::die("no nonzero gpu_id", 0)
}

fn usage() -> ! {
    eprintln!("usage: rbg probe [--acquire] | rbg mem");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("probe") => run_probe(&args),
        Some("mem") => run_mem(),
        _ => usage(),
    }
}

fn run_probe(args: &[String]) {
    let acquire = args.iter().skip(1).any(|a| a == "--acquire");
    let (gpu_id, gfx) = find_gpu_id();
    let kfd_fd = kfd::open_rw("/dev/kfd");
    let render_fd = kfd::open_rw("/dev/dri/renderD128");

    let version = kfd::get_version(kfd_fd);
    println!("kfd version: {}.{}", version.major, version.minor);
    println!("gpu_id: {}", gpu_id);
    println!("gfx_target_version: {}", gfx);

    if acquire {
        kfd::acquire_vm(kfd_fd, render_fd, gpu_id);
        println!("acquire_vm: ok");
    }
}

const MEM_SIZE: usize = 1024 * 1024;

fn run_mem() {
    let (gpu_id, _gfx) = find_gpu_id();
    let kfd_fd = kfd::open_rw("/dev/kfd");
    let render_fd = kfd::open_rw("/dev/dri/renderD128");
    kfd::acquire_vm(kfd_fd, render_fd, gpu_id);

    // Order: acquire_vm, reserve VA, alloc, map to gpu, host view.
    let va = kfd::reserve_va(MEM_SIZE);
    let (handle, mmap_offset) =
        kfd::alloc_memory(kfd_fd, gpu_id, va, MEM_SIZE as u64);
    kfd::map_to_gpu(kfd_fd, handle, gpu_id);
    let view = kfd::host_view(render_fd, va, MEM_SIZE, mmap_offset as i64);
    println!("handle: {}", handle);
    println!("mmap_offset: {}", mmap_offset);
    println!("va: 0x{:x}", va as u64);

    let n = MEM_SIZE / 4;
    let words = unsafe { std::slice::from_raw_parts_mut(view as *mut u32, n) };
    for (i, w) in words.iter_mut().enumerate() {
        *w = i as u32;
    }

    // Drop the host view, take a fresh one at the same VA.
    kfd::munmap_or_die(va, MEM_SIZE);
    let view2 = kfd::host_view(render_fd, va, MEM_SIZE, mmap_offset as i64);
    let back = unsafe { std::slice::from_raw_parts(view2 as *const u32, n) };
    let bad = back
        .iter()
        .enumerate()
        .filter(|(i, w)| **w != *i as u32)
        .count();

    // Teardown in reverse, even on a verify failure.
    kfd::munmap_or_die(va, MEM_SIZE);
    kfd::unmap_from_gpu(kfd_fd, handle, gpu_id);
    kfd::free_memory(kfd_fd, handle);

    if bad != 0 {
        eprintln!("verify: {} of {} values wrong", bad, n);
        std::process::exit(1);
    }
    println!("verify: {} ok", n);
}
