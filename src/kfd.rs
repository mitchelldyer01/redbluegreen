//! kfd - the /dev/kfd interface, by hand.
//!
//! open, ioctl, mmap and munmap are declared here. The ioctl
//! structs and call wrappers live here too, so main.rs stays
//! thin. No crates.

use std::ffi::{c_char, CString};

extern "C" {
    fn open(path: *const c_char, flags: i32) -> i32;
    fn ioctl(fd: i32, req: u64, ...) -> i32;
    fn mmap(
        addr: *mut u8,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

const O_RDWR: i32 = 2;
const O_CLOEXEC: i32 = 0o2000000;

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_SHARED: i32 = 1;
const MAP_PRIVATE: i32 = 2;
const MAP_FIXED: i32 = 0x10;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_NORESERVE: i32 = 0x4000;

const MAGIC_K: u64 = 0x4B; // 'K'
const IOC_READ: u64 = 2;
const IOC_WRITE: u64 = 1;

/// ALLOC flags: GTT, and writable from the GPU.
pub const ALLOC_GTT: u32 = 1 << 1;
pub const ALLOC_WRITABLE: u32 = 1 << 31;

pub fn ioc(dir: u64, size: u64, nr: u64) -> u64 {
    dir << 30 | size << 16 | MAGIC_K << 8 | nr
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct KfdGetVersion {
    pub major: u32,
    pub minor: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct KfdAcquireVM {
    pub drm_fd: u32,
    pub gpu_id: u32,
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct KfdAllocMemory {
    pub va_addr: u64,
    pub size: u64,
    pub handle: u64,
    pub mmap_offset: u64,
    pub gpu_id: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct KfdFreeMemory {
    pub handle: u64,
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct KfdMapMemory {
    pub handle: u64,
    pub device_ids_array_ptr: u64,
    pub n_devices: u32,
    pub n_success: u32,
}

const _: () = assert!(std::mem::size_of::<KfdGetVersion>() == 8);
const _: () = assert!(std::mem::size_of::<KfdAcquireVM>() == 8);
const _: () = assert!(std::mem::size_of::<KfdAllocMemory>() == 40);
const _: () = assert!(std::mem::size_of::<KfdFreeMemory>() == 8);
const _: () = assert!(std::mem::size_of::<KfdMapMemory>() == 24);

pub fn die(call: &str, errno: i32) -> ! {
    eprintln!("{}: {}", call, std::io::Error::from_raw_os_error(errno));
    std::process::exit(1);
}

pub fn open_rw(path: &str) -> i32 {
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

fn ioctl_check(fd: i32, req: u64, name: &str, arg: *mut u8) {
    let rc = unsafe { ioctl(fd, req, arg) };
    if rc != 0 {
        die(
            name,
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
        );
    }
}

/// mmap that dies on MAP_FAILED.
pub fn mmap_or_die(
    addr: *mut u8,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    off: i64,
    call: &str,
) -> *mut u8 {
    let p = unsafe { mmap(addr, len, prot, flags, fd, off) };
    if p as isize == -1 {
        die(
            call,
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
        );
    }
    p
}

pub fn munmap_or_die(addr: *mut u8, len: usize) {
    let rc = unsafe { munmap(addr, len) };
    if rc != 0 {
        die(
            "munmap",
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
        );
    }
}

/// Reserve `size` bytes of VA. No physical pages.
pub fn reserve_va(size: usize) -> *mut u8 {
    mmap_or_die(
        std::ptr::null_mut(),
        size,
        PROT_NONE,
        MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE,
        -1,
        0,
        "mmap reserve",
    )
}

/// Map a kfd allocation into the VA we chose.
///
/// On this kernel (7.0) the mmap_offset from ALLOC is a DRM
/// offset, so the renderD fd goes here, not the kfd fd.
pub fn host_view(
    drm_fd: i32,
    va: *mut u8,
    size: usize,
    mmap_offset: i64,
) -> *mut u8 {
    let p = mmap_or_die(
        va,
        size,
        PROT_READ | PROT_WRITE,
        MAP_SHARED | MAP_FIXED,
        drm_fd,
        mmap_offset,
        "mmap host view",
    );
    if p != va {
        die("mmap host view", 0);
    }
    p
}

pub fn get_version(kfd_fd: i32) -> KfdGetVersion {
    let mut v = KfdGetVersion::default();
    ioctl_check(
        kfd_fd,
        ioc(IOC_READ, 8, 0x01), // GET_VERSION
        "ioctl GET_VERSION",
        &mut v as *mut _ as *mut u8,
    );
    v
}

pub fn acquire_vm(kfd_fd: i32, drm_fd: i32, gpu_id: u32) {
    let mut arg = KfdAcquireVM {
        drm_fd: drm_fd as u32,
        gpu_id,
    };
    ioctl_check(
        kfd_fd,
        ioc(IOC_WRITE, 8, 0x15), // ACQUIRE_VM
        "ioctl ACQUIRE_VM",
        &mut arg as *mut _ as *mut u8,
    );
}

/// ALLOC_MEMORY_OF_GPU. Returns (handle, mmap_offset).
pub fn alloc_memory(
    kfd_fd: i32,
    gpu_id: u32,
    va: *mut u8,
    size: u64,
) -> (u64, u64) {
    let mut arg = KfdAllocMemory {
        va_addr: va as u64,
        size,
        handle: 0,
        mmap_offset: 0,
        gpu_id,
        flags: ALLOC_GTT | ALLOC_WRITABLE,
    };
    ioctl_check(
        kfd_fd,
        ioc(IOC_READ | IOC_WRITE, 40, 0x16), // ALLOC_MEMORY_OF_GPU
        "ioctl ALLOC_MEMORY_OF_GPU",
        &mut arg as *mut _ as *mut u8,
    );
    (arg.handle, arg.mmap_offset)
}

pub fn free_memory(kfd_fd: i32, handle: u64) {
    let arg = KfdFreeMemory { handle };
    ioctl_check(
        kfd_fd,
        ioc(IOC_WRITE, 8, 0x17), // FREE_MEMORY
        "ioctl FREE_MEMORY",
        &arg as *const _ as *mut u8,
    );
}

pub fn map_to_gpu(kfd_fd: i32, handle: u64, gpu_id: u32) {
    // gpu_id is a local, so the pointer below is valid for the
    // whole call.
    let mut arg = KfdMapMemory {
        handle,
        device_ids_array_ptr: &gpu_id as *const u32 as u64,
        n_devices: 1,
        n_success: 0,
    };
    ioctl_check(
        kfd_fd,
        ioc(IOC_READ | IOC_WRITE, 24, 0x18), // MAP_MEMORY_TO_GPU
        "ioctl MAP_MEMORY_TO_GPU",
        &mut arg as *mut _ as *mut u8,
    );
    if arg.n_success != 1 {
        die("ioctl MAP_MEMORY_TO_GPU", 0);
    }
}

pub fn unmap_from_gpu(kfd_fd: i32, handle: u64, gpu_id: u32) {
    let mut arg = KfdMapMemory {
        handle,
        device_ids_array_ptr: &gpu_id as *const u32 as u64,
        n_devices: 1,
        n_success: 0,
    };
    ioctl_check(
        kfd_fd,
        ioc(IOC_READ | IOC_WRITE, 24, 0x19), // UNMAP_MEMORY
        "ioctl UNMAP_MEMORY_FROM_GPU",
        &mut arg as *mut _ as *mut u8,
    );
    if arg.n_success != 1 {
        die("ioctl UNMAP_MEMORY_FROM_GPU", 0);
    }
}
