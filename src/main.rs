//! rbg - talk to the GPU driver through /dev/kfd.
//!
//! No crates. Only ioctl (and open, read via libc) are declared
//! by hand. Everything else is std.

use std::ffi::{c_char, CString};
use std::fs;
use std::path::Path;

extern "C" {
    fn open(path: *const c_char, flags: i32) -> i32;
    fn ioctl(fd: i32, req: u64, ...) -> i32;
}

const O_RDWR: i32 = 2;
const O_CLOEXEC: i32 = 0o2000000;

const MAGIC_K: u64 = 0x4B; // 'K'
const IOC_READ: u64 = 2;
const IOC_WRITE: u64 = 1;

fn ioc(dir: u64, size: u64, nr: u64) -> u64 {
    dir << 30 | size << 16 | MAGIC_K << 8 | nr
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct KfdGetVersion {
    major: u32,
    minor: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct KfdAcquireVM {
    drm_fd: u32,
    gpu_id: u32,
}

const _: () = assert!(std::mem::size_of::<KfdGetVersion>() == 8);
const _: () = assert!(std::mem::size_of::<KfdAcquireVM>() == 8);

fn die(call: &str, errno: i32) -> ! {
    eprintln!("{}: {}", call, std::io::Error::from_raw_os_error(errno));
    std::process::exit(1);
}

fn open_rw(path: &str) -> i32 {
    let c = CString::new(path).expect("no nul byte in path");
    let fd = unsafe { open(c.as_ptr(), O_RDWR | O_CLOEXEC) };
    if fd < 0 {
        die(
            "open",
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
        );
    }
    fd
}

/// Find the first kfd node whose gpu_id is nonzero.
fn find_gpu_id() -> (u32, u32) {
    let base = Path::new("/sys/class/kfd/kfd/topology/nodes");
    let entries = fs::read_dir(base)
        .unwrap_or_else(|e| die("read_dir", e.raw_os_error().unwrap_or(-1)));
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
                die("read properties", e.raw_os_error().unwrap_or(-1))
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
            .unwrap_or_else(|| die("no gfx_target_version", 0));
        return (id, gfx);
    }
    die("no nonzero gpu_id", 0)
}

fn usage() -> ! {
    eprintln!("usage: rbg probe [--acquire]");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("probe") => {}
        _ => usage(),
    }
    let acquire = args.iter().skip(1).any(|a| a == "--acquire");

    let (gpu_id, gfx) = find_gpu_id();
    let kfd_fd = open_rw("/dev/kfd");
    let render_fd = open_rw("/dev/dri/renderD128");

    let mut version = KfdGetVersion::default();
    let req = ioc(IOC_READ, 8, 0x01); // GET_VERSION
    let rc = unsafe { ioctl(kfd_fd, req, &mut version) };
    if rc != 0 {
        die(
            "ioctl GET_VERSION",
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
        );
    }
    println!("kfd version: {}.{}", version.major, version.minor);
    println!("gpu_id: {}", gpu_id);
    println!("gfx_target_version: {}", gfx);

    if acquire {
        let arg = KfdAcquireVM {
            drm_fd: render_fd as u32,
            gpu_id,
        };
        let req = ioc(IOC_WRITE, 8, 0x15); // ACQUIRE_VM
        let rc = unsafe { ioctl(kfd_fd, req, &arg) };
        if rc != 0 {
            die(
                "ioctl ACQUIRE_VM",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
            );
        }
        println!("acquire_vm: ok");
    }
}
