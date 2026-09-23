//! kfd - the /dev/kfd interface, by hand.
//!
//! open, close, ioctl, mmap and munmap are declared here. The
//! ioctl structs and call wrappers live here too, so main.rs
//! stays thin. Every call returns Result. No crates.

use std::ffi::{c_char, CString};
use std::fmt;
use std::io;
use std::sync::atomic::{fence, Ordering};
use std::time::{Duration, Instant};

extern "C" {
    fn open(path: *const c_char, flags: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn ioctl(fd: i32, req: u64, ...) -> i32;
    fn mmap(
        addr: *mut u8,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut u8;
    #[link_name = "munmap"]
    fn sys_munmap(addr: *mut u8, len: usize) -> i32;
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

/// ALLOC flags.
pub const ALLOC_GTT: u32 = 1 << 1;
const ALLOC_EXECUTABLE: u32 = 1 << 30;
/// Fine-grained: the CP's own reads, atomics and EOP path see
/// the same bytes the CPU does. ROCr sets it on every buffer the
/// CP touches except EOP and CWSR (2026-09-22, unit 4).
pub const ALLOC_COHERENT: u32 = 1 << 26;
pub const ALLOC_WRITABLE: u32 = 1 << 31;

const QUEUE_TYPE_COMPUTE_AQL: u32 = 2;
const EVENT_TYPE_SIGNAL: u32 = 0;
const DOORBELL_PAGE: u64 = 8192;
/// EOP ring size the kernel expects on this gfx (kfd_queue.c).
const EOP_SIZE: u64 = 4096;
const DOORBELL_MASK: u64 = 0x1FFF;
const EVENT_PAGE: u64 = 32 * 1024;

/// The error of this program. It holds the name of the call
/// that failed, the errno the call returned, and the message
/// for that errno.
pub struct Error {
    pub call: String,
    pub errno: i32,
    pub message: String,
}

impl Error {
    /// A syscall or ioctl failed. The errno and the message
    /// come from the kernel.
    fn syscall(call: &str) -> Error {
        let e = io::Error::last_os_error();
        Error {
            call: call.to_string(),
            errno: e.raw_os_error().unwrap_or(0),
            message: e.to_string(),
        }
    }

    /// A check in our own code failed. The errno is 0.
    pub fn other(call: &str, message: &str) -> Error {
        Error {
            call: call.to_string(),
            errno: 0,
            message: message.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.call, self.message)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error({:?}, errno {}, {:?})",
            self.call, self.errno, self.message
        )
    }
}

impl std::error::Error for Error {}

/// The result of every call in this file.
pub type Result<T> = std::result::Result<T, Error>;

fn ioc(dir: u64, size: u64, nr: u64) -> u64 {
    dir << 30 | size << 16 | MAGIC_K << 8 | nr
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct KfdGetVersion {
    pub major: u32,
    pub minor: u32,
}

// FFI structs below: the kernel reads the fields, Rust writes
// them, so `dead_code` is allowed at the struct level.

#[derive(Copy, Clone)]
#[allow(dead_code)]
#[repr(C)]
pub struct KfdAcquireVM {
    pub drm_fd: u32,
    pub gpu_id: u32,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
#[repr(C)]
pub struct KfdAllocMemory {
    pub va_addr: u64,
    pub size: u64,
    pub handle: u64,
    pub mmap_offset: u64,
    pub gpu_id: u32,
    pub flags: u32,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
#[repr(C)]
pub struct KfdFreeMemory {
    pub handle: u64,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
#[repr(C)]
pub struct KfdMapMemory {
    pub handle: u64,
    pub device_ids_array_ptr: u64,
    pub n_devices: u32,
    pub n_success: u32,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
#[repr(C)]
pub struct KfdCreateQueue {
    pub ring_base_address: u64,
    pub write_pointer_address: u64,
    pub read_pointer_address: u64,
    pub doorbell_offset: u64,
    pub ring_size: u32,
    pub gpu_id: u32,
    pub queue_type: u32,
    pub queue_percentage: u32,
    pub queue_priority: u32,
    pub queue_id: u32,
    pub eop_buffer_address: u64,
    pub eop_buffer_size: u64,
    pub ctx_save_restore_addr: u64,
    pub ctx_save_restore_size: u32,
    pub ctl_stack_size: u32,
    pub sdma_engine_id: u32,
    pub _pad: u32,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
#[repr(C)]
pub struct KfdDestroyQueue {
    pub queue_id: u32,
    pub _pad: u32,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
#[repr(C)]
pub struct KfdCreateEvent {
    pub event_page_offset: u64,
    pub event_trigger_data: u32,
    pub event_type: u32,
    pub auto_reset: u32,
    pub node_id: u32,
    pub event_id: u32,
    pub event_slot_index: u32,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
#[repr(C)]
pub struct KfdDestroyEvent {
    pub event_id: u32,
    pub _pad: u32,
}

/// RUNTIME_ENABLE (0x25). ROCr calls it once per process before any
/// queue. The kernel then tells MES to clear stale process context;
/// without it a queue created while another process is busy is
/// never scheduled (unit 5b, 2026-09-22).
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct KfdRuntimeEnable {
    pub r_debug: u64,
    pub mode_mask: u32,
    pub capabilities_mask: u32,
}
const _: () = assert!(std::mem::size_of::<KfdRuntimeEnable>() == 16);

const _: () = assert!(std::mem::size_of::<KfdGetVersion>() == 8);
const _: () = assert!(std::mem::size_of::<KfdAcquireVM>() == 8);
const _: () = assert!(std::mem::size_of::<KfdAllocMemory>() == 40);
const _: () = assert!(std::mem::size_of::<KfdFreeMemory>() == 8);
const _: () = assert!(std::mem::size_of::<KfdMapMemory>() == 24);
const _: () = assert!(std::mem::size_of::<KfdCreateQueue>() == 96);
const _: () = assert!(std::mem::size_of::<KfdDestroyQueue>() == 8);
const _: () = assert!(std::mem::size_of::<KfdCreateEvent>() == 32);
const _: () = assert!(std::mem::size_of::<KfdDestroyEvent>() == 8);

pub fn open_rw(path: &str) -> Result<i32> {
    let c = CString::new(path).expect("no nul byte in path");
    let fd = unsafe { open(c.as_ptr(), O_RDWR | O_CLOEXEC) };
    if fd < 0 {
        return Err(Error::syscall(&format!("open {path}")));
    }
    Ok(fd)
}

fn ioctl_check(fd: i32, req: u64, name: &str, arg: *mut u8) -> Result<()> {
    let rc = unsafe { ioctl(fd, req, arg) };
    if rc != 0 {
        return Err(Error::syscall(name));
    }
    Ok(())
}

fn mmap_raw(
    addr: *mut u8,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    off: i64,
    call: &str,
) -> Result<*mut u8> {
    let p = unsafe { mmap(addr, len, prot, flags, fd, off) };
    if p as isize == -1 {
        return Err(Error::syscall(call));
    }
    Ok(p)
}

/// Reserve `size` bytes of VA. No physical pages.
fn reserve_va(size: usize) -> Result<*mut u8> {
    mmap_raw(
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
fn host_view(
    drm_fd: i32,
    va: *mut u8,
    size: usize,
    mmap_offset: i64,
) -> Result<*mut u8> {
    let p = mmap_raw(
        va,
        size,
        PROT_READ | PROT_WRITE,
        MAP_SHARED | MAP_FIXED,
        drm_fd,
        mmap_offset,
        "mmap host view",
    )?;
    if p != va {
        return Err(Error::other("mmap host view", "wrong address"));
    }
    Ok(p)
}

fn munmap(addr: *mut u8, len: usize) -> Result<()> {
    let rc = unsafe { sys_munmap(addr, len) };
    if rc != 0 {
        return Err(Error::syscall("munmap"));
    }
    Ok(())
}

pub fn close_fd(fd: i32) {
    let _ = unsafe { close(fd) };
}

pub fn get_version(kfd_fd: i32) -> Result<KfdGetVersion> {
    let mut v = KfdGetVersion { major: 0, minor: 0 };
    ioctl_check(
        kfd_fd,
        ioc(IOC_READ, 8, 0x01), // GET_VERSION
        "ioctl GET_VERSION",
        &mut v as *mut _ as *mut u8,
    )?;
    Ok(v)
}

/// Enable the runtime for this process, mode ENABLE (1), no TTMP
/// setup. `r_debug` must outlive the process; the kernel keeps
/// the address for debuggers and never reads it otherwise.
pub fn runtime_enable(kfd_fd: i32, r_debug: &'static u64) -> Result<()> {
    let mut arg = KfdRuntimeEnable {
        r_debug: r_debug as *const u64 as u64,
        mode_mask: 1,
        capabilities_mask: 0,
    };
    ioctl_check(
        kfd_fd,
        ioc(IOC_READ | IOC_WRITE, 16, 0x25), // RUNTIME_ENABLE
        "ioctl RUNTIME_ENABLE",
        &mut arg as *mut _ as *mut u8,
    )
}

pub fn acquire_vm(kfd_fd: i32, drm_fd: i32, gpu_id: u32) -> Result<()> {
    let mut arg = KfdAcquireVM {
        drm_fd: drm_fd as u32,
        gpu_id,
    };
    ioctl_check(
        kfd_fd,
        ioc(IOC_WRITE, 8, 0x15), // ACQUIRE_VM
        "ioctl ACQUIRE_VM",
        &mut arg as *mut _ as *mut u8,
    )
}

/// ALLOC_MEMORY_OF_GPU. Returns (handle, mmap_offset).
fn alloc_memory(
    kfd_fd: i32,
    gpu_id: u32,
    va: *mut u8,
    size: u64,
    flags: u32,
) -> Result<(u64, u64)> {
    let mut arg = KfdAllocMemory {
        va_addr: va as u64,
        size,
        handle: 0,
        mmap_offset: 0,
        gpu_id,
        flags,
    };
    ioctl_check(
        kfd_fd,
        ioc(IOC_READ | IOC_WRITE, 40, 0x16), // ALLOC_MEMORY_OF_GPU
        "ioctl ALLOC_MEMORY_OF_GPU",
        &mut arg as *mut _ as *mut u8,
    )?;
    Ok((arg.handle, arg.mmap_offset))
}

fn free_memory(kfd_fd: i32, handle: u64) -> Result<()> {
    let arg = KfdFreeMemory { handle };
    ioctl_check(
        kfd_fd,
        ioc(IOC_WRITE, 8, 0x17), // FREE_MEMORY
        "ioctl FREE_MEMORY",
        &arg as *const _ as *mut u8,
    )
}

fn map_to_gpu(kfd_fd: i32, handle: u64, gpu_id: u32) -> Result<()> {
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
    )?;
    if arg.n_success != 1 {
        return Err(Error::other("ioctl MAP_MEMORY_TO_GPU", "n_success != 1"));
    }
    Ok(())
}

fn unmap_from_gpu(kfd_fd: i32, handle: u64, gpu_id: u32) -> Result<()> {
    let mut arg = KfdMapMemory {
        handle,
        device_ids_array_ptr: &gpu_id as *const u32 as u64,
        n_devices: 1,
        n_success: 0,
    };
    ioctl_check(
        kfd_fd,
        ioc(IOC_READ | IOC_WRITE, 24, 0x19), // UNMAP_MEMORY_FROM_GPU
        "ioctl UNMAP_MEMORY_FROM_GPU",
        &mut arg as *mut _ as *mut u8,
    )?;
    if arg.n_success != 1 {
        return Err(Error::other(
            "ioctl UNMAP_MEMORY_FROM_GPU",
            "n_success != 1",
        ));
    }
    Ok(())
}

/// Owns the kfd fd, the render fd, the gpu_id and the gfx
/// target version.
///
/// `new` opens both nodes and acquires the VM. Drop closes the
/// fds.
pub struct Kfd {
    pub kfd_fd: i32,
    pub render_fd: i32,
    pub gpu_id: u32,
    /// The gfx target version. No caller reads it yet.
    #[allow(dead_code)]
    pub gfx: u32,
}

impl Kfd {
    /// Open /dev/kfd and the render node, then ACQUIRE_VM.
    pub fn new(node: &NodeInfo, render_path: &str) -> Result<Kfd> {
        let kfd_fd = open_rw("/dev/kfd")?;
        let render_fd = match open_rw(render_path) {
            Ok(fd) => fd,
            Err(e) => {
                close_fd(kfd_fd);
                return Err(e);
            }
        };
        static R_DEBUG: u64 = 0;
        if let Err(e) = runtime_enable(kfd_fd, &R_DEBUG) {
            close_fd(kfd_fd);
            close_fd(render_fd);
            return Err(e);
        }
        if let Err(e) = acquire_vm(kfd_fd, render_fd, node.gpu_id) {
            close_fd(kfd_fd);
            close_fd(render_fd);
            return Err(e);
        }
        Ok(Kfd {
            kfd_fd,
            render_fd,
            gpu_id: node.gpu_id,
            gfx: node.gfx,
        })
    }
}

impl Drop for Kfd {
    fn drop(&mut self) {
        close_fd(self.kfd_fd);
        close_fd(self.render_fd);
    }
}

/// A GTT buffer, mapped to the GPU, with a host view at `va`.
///
/// `new` reserves the VA, ALLOCs the BO, maps it to the GPU, and
/// takes the host view, in that order. The host view can be
/// dropped and retaken at the same VA with `remap`.
///
/// Drop cannot fail: munmap, unmap, free. On error it prints to
/// stderr and goes on.
pub struct Buffer<'a> {
    kfd: &'a Kfd,
    pub va: *mut u8,
    pub size: u64,
    pub handle: u64,
    pub mmap_offset: u64,
    /// The flags the BO was ALLOCed with. Held for the record;
    /// the kernel reads them, Rust does not.
    #[allow(dead_code)]
    pub flags: u32,
    /// True while the host view is mapped at `va`.
    view_mapped: bool,
}

impl<'a> Buffer<'a> {
    /// Reserve VA, ALLOC, map to gpu, host view, in that order.
    pub fn new(kfd: &'a Kfd, size: u64, flags: u32) -> Result<Buffer<'a>> {
        let va = reserve_va(size as usize)?;
        let (handle, mmap_offset) =
            alloc_memory(kfd.kfd_fd, kfd.gpu_id, va, size, flags)?;
        if let Err(e) = map_to_gpu(kfd.kfd_fd, handle, kfd.gpu_id) {
            let _ = free_memory(kfd.kfd_fd, handle);
            let _ = munmap(va, size as usize);
            return Err(e);
        }
        if let Err(e) =
            host_view(kfd.render_fd, va, size as usize, mmap_offset as i64)
        {
            let _ = unmap_from_gpu(kfd.kfd_fd, handle, kfd.gpu_id);
            let _ = free_memory(kfd.kfd_fd, handle);
            let _ = munmap(va, size as usize);
            return Err(e);
        }
        Ok(Buffer {
            kfd,
            va,
            size,
            handle,
            mmap_offset,
            flags,
            view_mapped: true,
        })
    }

    /// Borrow the mapped memory. The slice borrows through the
    /// Buffer, so it cannot outlive it. The host view must be
    /// mapped.
    pub fn as_slice_mut<T>(&mut self) -> &mut [T] {
        debug_assert!(self.view_mapped);
        unsafe {
            std::slice::from_raw_parts_mut(
                self.va as *mut T,
                self.size as usize / std::mem::size_of::<T>(),
            )
        }
    }

    /// Drop the host view, then take a fresh one at the same VA.
    pub fn remap(&mut self) -> Result<()> {
        self.view_mapped = false;
        munmap(self.va, self.size as usize)?;
        host_view(
            self.kfd.render_fd,
            self.va,
            self.size as usize,
            self.mmap_offset as i64,
        )?;
        self.view_mapped = true;
        Ok(())
    }
}

impl<'a> Drop for Buffer<'a> {
    fn drop(&mut self) {
        if self.view_mapped {
            if let Err(e) = munmap(self.va, self.size as usize) {
                eprintln!("drop Buffer: munmap: {e}");
            }
        }
        if let Err(e) =
            unmap_from_gpu(self.kfd.kfd_fd, self.handle, self.kfd.gpu_id)
        {
            eprintln!("drop Buffer: unmap: {e}");
        }
        if let Err(e) = free_memory(self.kfd.kfd_fd, self.handle) {
            eprintln!("drop Buffer: free: {e}");
        }
    }
}

/// A kernel object: descriptor plus code, in one Buffer.
///
/// Bytes 0..64 hold the descriptor, the code starts at byte 256.
/// The descriptor's entry offset (i64 at 16) is patched to 256;
/// the assembler leaves it at zero. docs/dispatch.md.
pub struct Kernel<'a> {
    buf: Buffer<'a>,
}

impl<'a> Kernel<'a> {
    /// Load a kernel the caller named. `kd` is the 64-byte
    /// descriptor, `text` the machine code, each an
    /// `include_bytes!` of a file tools/asm wrote.
    pub fn new(kfd: &'a Kfd, kd: &[u8; 64], text: &[u8]) -> Result<Kernel<'a>> {
        // 256 plus the code, rounded up to a whole page.
        let size = (256 + text.len() as u64 + 4095) & !4095;
        let flags =
            ALLOC_GTT | ALLOC_WRITABLE | ALLOC_EXECUTABLE | ALLOC_COHERENT;
        let mut buf = Buffer::new(kfd, size, flags)?;
        let b = buf.as_slice_mut::<u8>();
        b[..64].copy_from_slice(kd);
        b[256..256 + text.len()].copy_from_slice(text);
        b[16..24].copy_from_slice(&256i64.to_le_bytes());
        Ok(Kernel { buf })
    }

    /// The va the packet's kernel_object field takes.
    pub fn object(&self) -> u64 {
        self.buf.va as u64
    }
}

/// A dispatch signal: the 64-byte struct in its own Buffer.
///
/// kind 1 (USER) at 0, value at 8. Set to 1 in new; the CP
/// decrements it to 0 when the packet retires. We poll, no
/// Event. docs/dispatch.md.
pub struct Signal<'a> {
    buf: Buffer<'a>,
}

impl<'a> Signal<'a> {
    /// Allocate zero-filled, then set kind 1 and value 1.
    pub fn new(kfd: &'a Kfd) -> Result<Signal<'a>> {
        let flags = ALLOC_GTT | ALLOC_WRITABLE | ALLOC_COHERENT;
        let mut buf = Buffer::new(kfd, 4096, flags)?;
        let b = buf.as_slice_mut::<u8>();
        for x in b.iter_mut() {
            *x = 0;
        }
        b[0..8].copy_from_slice(&1i64.to_le_bytes()); // kind USER
        b[8..16].copy_from_slice(&1i64.to_le_bytes()); // value
        Ok(Signal { buf })
    }

    /// The va the packet's completion_signal field takes.
    pub fn va(&self) -> u64 {
        self.buf.va as u64
    }

    /// The value field, by a volatile read.
    pub fn value(&self) -> i64 {
        unsafe { std::ptr::read_volatile(self.buf.va.add(8) as *const i64) }
    }

    /// Poll the value every 100 us. Ok at 0, else an error
    /// naming "signal timeout".
    pub fn wait(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        loop {
            if self.value() == 0 {
                return Ok(());
            }
            if start.elapsed() > timeout {
                return Err(Error::other("signal wait", "signal timeout"));
            }
            std::thread::sleep(Duration::from_micros(100));
        }
    }
}

/// The non-geometry inputs to `Queue::new`.
///
/// `event` and `error_payload` must outlive the queue: the
/// CWSR header names both.
pub struct QueueOpts<'a> {
    pub event: &'a Event<'a>,
    pub error_payload: &'a Buffer<'a>,
    /// EXECUTABLE on the CWSR BO (the --cwsr-exec switch).
    pub cwsr_exec: bool,
    /// Fill the CWSR header before CREATE_QUEUE. --no-header
    /// turns this off.
    pub fill_header: bool,
    /// Print the header words once at creation (--debug).
    pub verbose: bool,
}

/// An AQL compute queue.
///
/// Owns the ring, write pointer, read pointer, CWSR and EOP
/// Buffers plus the 8 KiB doorbell mapping. Drop calls DESTROY_QUEUE and
/// then munmaps the doorbell; the Buffers drop after that.
pub struct Queue<'a> {
    kfd: &'a Kfd,
    /// Held so the BOs outlive the queue; the kernel reads them.
    #[allow(dead_code)]
    pub ring: Buffer<'a>,
    #[allow(dead_code)]
    pub write_ptr: Buffer<'a>,
    pub read_ptr: Buffer<'a>,
    #[allow(dead_code)]
    pub cwsr: Buffer<'a>,
    /// EOP ring, 4 KiB. gfx11 programs it into the MQD without a
    /// zero check; a missing one hangs MES on REMOVE_QUEUE.
    #[allow(dead_code)]
    pub eop: Buffer<'a>,
    pub doorbell: *mut u8,
    pub doorbell_offset: u64,
    pub queue_id: u32,
    /// The index of the next free ring slot.
    write_index: u64,
}

impl<'a> Queue<'a> {
    /// Build the five Buffers, CREATE_QUEUE, mmap the doorbell.
    pub fn new(
        kfd: &'a Kfd,
        ring_size: u64,
        cwsr_size: u64,
        cwsr_bo_size: u64,
        debug_size: u64,
        ctl_stack_size: u32,
        opts: &QueueOpts<'a>,
    ) -> Result<Queue<'a>> {
        let event = opts.event;
        let error_payload = opts.error_payload;
        let cwsr_exec = opts.cwsr_exec;
        let fill_header = opts.fill_header;
        let verbose = opts.verbose;

        // The ring must be EXECUTABLE. It must not be
        // AQL_QUEUE_MEM: on this kernel that flag makes the BO
        // un-mappable on the render fd, so no host view.
        let flags_w = ALLOC_GTT | ALLOC_WRITABLE;
        let flags_fine = flags_w | ALLOC_COHERENT;
        let ring = Buffer::new(kfd, ring_size, flags_fine | ALLOC_EXECUTABLE)?;
        let write_ptr = Buffer::new(kfd, 4096, flags_fine)?;
        let read_ptr = Buffer::new(kfd, 4096, flags_fine)?;
        let cwsr_flags = if cwsr_exec {
            flags_w | ALLOC_EXECUTABLE
        } else {
            flags_w
        };
        let mut cwsr = Buffer::new(kfd, cwsr_bo_size, cwsr_flags)?;
        let eop = Buffer::new(kfd, EOP_SIZE, flags_w | ALLOC_EXECUTABLE)?;

        // The CWSR header, at offset 0 of the area, as
        // libhsakmt fills it before CREATE_QUEUE. The area is
        // zero-filled, so the first four words and Reserved
        // stay 0.
        if fill_header {
            let h = cwsr.as_slice_mut::<u8>();
            h[16..20].copy_from_slice(&(cwsr_size as u32).to_le_bytes());
            h[20..24].copy_from_slice(&(debug_size as u32).to_le_bytes());
            h[24..32].copy_from_slice(&(error_payload.va as u64).to_le_bytes());
            h[32..36].copy_from_slice(&event.event_id.to_le_bytes());
        }
        if verbose {
            let h = cwsr.as_slice_mut::<u8>();
            let words = (0..9)
                .map(|i| {
                    u32::from_le_bytes(h[i * 4..i * 4 + 4].try_into().unwrap())
                })
                .map(|w| format!("{w:#010x}"))
                .collect::<Vec<_>>();
            println!("cwsr header: {}", words.join(" "));
        }

        let mut arg = KfdCreateQueue {
            ring_base_address: ring.va as u64,
            write_pointer_address: write_ptr.va as u64,
            read_pointer_address: read_ptr.va as u64,
            doorbell_offset: 0,
            ring_size: ring_size as u32,
            gpu_id: kfd.gpu_id,
            queue_type: QUEUE_TYPE_COMPUTE_AQL,
            queue_percentage: 100,
            queue_priority: 7,
            queue_id: 0,
            eop_buffer_address: eop.va as u64,
            eop_buffer_size: EOP_SIZE,
            ctx_save_restore_addr: cwsr.va as u64,
            ctx_save_restore_size: cwsr_size as u32,
            ctl_stack_size,
            sdma_engine_id: 0,
            _pad: 0,
        };
        ioctl_check(
            kfd.kfd_fd,
            ioc(IOC_READ | IOC_WRITE, 96, 0x02), // CREATE_QUEUE
            "ioctl CREATE_QUEUE",
            &mut arg as *mut _ as *mut u8,
        )?;
        let off = arg.doorbell_offset;
        let doorbell = mmap_raw(
            std::ptr::null_mut(),
            DOORBELL_PAGE as usize,
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            kfd.kfd_fd,
            (off & !DOORBELL_MASK) as i64,
            "mmap doorbell",
        )?;
        let mut q = Queue {
            kfd,
            ring,
            write_ptr,
            read_ptr,
            cwsr,
            eop,
            doorbell,
            doorbell_offset: off,
            queue_id: arg.queue_id,
            write_index: 0,
        };
        q.init_ring();
        Ok(q)
    }

    /// Write header 1 (INVALID) into every ring slot. The CP
    /// waits on an INVALID header and never runs one.
    pub fn init_ring(&mut self) {
        let slots = self.ring.size / 64;
        let b = self.ring.as_slice_mut::<u8>();
        for i in 0..slots {
            let off = (i * 64) as usize;
            b[off..off + 2].copy_from_slice(&1u16.to_le_bytes());
        }
    }

    /// Steps 2 to 4 of the write order in docs/dispatch.md:
    /// the packet body, the header last, the write pointer, the
    /// doorbell. A release fence before the header store and
    /// before the doorbell store.
    pub fn dispatch(&mut self, packet: &[u8; 64], ring_doorbell: bool) {
        let index = self.write_index;
        let slot = index % (self.ring.size / 64);
        let base = (slot * 64) as usize;
        let b = self.ring.as_slice_mut::<u8>();
        // Step 2, first half: the body, without the header.
        b[base + 4..base + 64].copy_from_slice(&packet[4..64]);
        // Step 2, second half: the header, last, as one u32
        // store of header | setup << 16.
        let h = u16::from_le_bytes([packet[0], packet[1]]) as u32
            | (u16::from_le_bytes([packet[2], packet[3]]) as u32) << 16;
        fence(Ordering::Release);
        unsafe {
            std::ptr::write_volatile(
                (self.ring.va as *mut u32).add(base / 4),
                h,
            );
        }
        // Step 3: index + 1 into the write-pointer Buffer, u64.
        let w = self.write_ptr.as_slice_mut::<u8>();
        w[0..8].copy_from_slice(&(self.write_index + 1).to_le_bytes());
        self.write_index += 1;
        if ring_doorbell {
            // Step 4: the packet's own index (not its slot) into
            // the doorbell. This doorbell write is the one place
            // in this program that starts the hardware.
            fence(Ordering::Release);
            unsafe {
                std::ptr::write_volatile(self.doorbell as *mut u64, index);
            }
        }
    }
}

impl<'a> Queue<'a> {
    /// Ring the doorbell again with the last packet's index. For
    /// the lost-doorbell experiment (unit 5b).
    pub fn reknock(&self) {
        if self.write_index == 0 {
            return;
        }
        fence(Ordering::Release);
        unsafe {
            std::ptr::write_volatile(
                self.doorbell as *mut u64,
                self.write_index - 1,
            );
        }
    }
}

impl<'a> Drop for Queue<'a> {
    fn drop(&mut self) {
        let arg = KfdDestroyQueue {
            queue_id: self.queue_id,
            _pad: 0,
        };
        if let Err(e) = ioctl_check(
            self.kfd.kfd_fd,
            ioc(IOC_WRITE, 8, 0x03), // DESTROY_QUEUE
            "ioctl DESTROY_QUEUE",
            &arg as *const _ as *mut u8,
        ) {
            eprintln!("drop Queue: destroy: {e}");
        }
        if let Err(e) = munmap(self.doorbell, DOORBELL_PAGE as usize) {
            eprintln!("drop Queue: munmap doorbell: {e}");
        }
        // The Buffers drop here, in declaration order.
    }
}

/// A SIGNAL event, with its event page mapped.
///
/// Owns the event_id and the 32 KiB event page mapping. The slot
/// is the u64 at index `event_slot_index`. Drop munmaps the page
/// then DESTROY_EVENT.
pub struct Event<'a> {
    kfd: &'a Kfd,
    pub event_id: u32,
    pub event_slot_index: u32,
    pub page: *mut u8,
}

impl<'a> Event<'a> {
    /// CREATE_EVENT, then mmap the event page.
    pub fn new(kfd: &'a Kfd) -> Result<Event<'a>> {
        let mut arg = KfdCreateEvent {
            event_page_offset: 0,
            event_trigger_data: 0,
            event_type: EVENT_TYPE_SIGNAL,
            auto_reset: 0,
            node_id: 0,
            event_id: 0,
            event_slot_index: 0,
        };
        ioctl_check(
            kfd.kfd_fd,
            ioc(IOC_READ | IOC_WRITE, 32, 0x08), // CREATE_EVENT
            "ioctl CREATE_EVENT",
            &mut arg as *mut _ as *mut u8,
        )?;
        let page = match mmap_raw(
            std::ptr::null_mut(),
            EVENT_PAGE as usize,
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            kfd.kfd_fd,
            arg.event_page_offset as i64,
            "mmap event page",
        ) {
            Ok(p) => p,
            Err(e) => {
                let d = KfdDestroyEvent {
                    event_id: arg.event_id,
                    _pad: 0,
                };
                let _ = ioctl_check(
                    kfd.kfd_fd,
                    ioc(IOC_WRITE, 8, 0x09), // DESTROY_EVENT
                    "ioctl DESTROY_EVENT",
                    &d as *const _ as *mut u8,
                );
                return Err(e);
            }
        };
        Ok(Event {
            kfd,
            event_id: arg.event_id,
            event_slot_index: arg.event_slot_index,
            page,
        })
    }
}

impl<'a> Drop for Event<'a> {
    fn drop(&mut self) {
        if let Err(e) = munmap(self.page, EVENT_PAGE as usize) {
            eprintln!("drop Event: munmap: {e}");
        }
        let arg = KfdDestroyEvent {
            event_id: self.event_id,
            _pad: 0,
        };
        if let Err(e) = ioctl_check(
            self.kfd.kfd_fd,
            ioc(IOC_WRITE, 8, 0x09), // DESTROY_EVENT
            "ioctl DESTROY_EVENT",
            &arg as *const _ as *mut u8,
        ) {
            eprintln!("drop Event: destroy: {e}");
        }
    }
}

/// Facts read from the node's sysfs directory.
pub struct NodeInfo {
    pub gpu_id: u32,
    pub gfx: u32,
    pub cwsr_size: u64,
    pub ctl_stack_size: u64,
    pub simd_count: u64,
    pub simd_per_cu: u64,
    pub drm_render_minor: u64,
}

fn read_prop(
    props: &std::collections::HashMap<String, String>,
    key: &str,
) -> Result<u64> {
    props.get(key).and_then(|s| s.parse().ok()).ok_or_else(|| {
        Error::other("read properties", &format!("no {key} in node properties"))
    })
}

/// Find the first kfd node whose gpu_id is nonzero, and read its
/// properties.
pub fn find_node() -> Result<NodeInfo> {
    let base = std::path::Path::new("/sys/class/kfd/kfd/topology/nodes");
    let entries = std::fs::read_dir(base)
        .map_err(|e| Error::other("read_dir", &format!("{e}")))?;
    for entry in entries.flatten() {
        let dir = entry.path();
        let id: u32 = match std::fs::read_to_string(dir.join("gpu_id"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
        {
            Some(v) => v,
            None => continue,
        };
        if id == 0 {
            continue;
        }
        let props_text = std::fs::read_to_string(dir.join("properties"))
            .map_err(|e| Error::other("read properties", &format!("{e}")))?;
        let mut props = std::collections::HashMap::new();
        for line in props_text.lines() {
            let mut it = line.split_whitespace();
            if let (Some(k), Some(v)) = (it.next(), it.next()) {
                props.insert(k.to_string(), v.to_string());
            }
        }
        let gfx: u32 = props
            .get("gfx_target_version")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                Error::other("read properties", "no gfx_target_version")
            })?;
        let cwsr_size = read_prop(&props, "cwsr_size")?;
        let ctl_stack_size = read_prop(&props, "ctl_stack_size")?;
        let simd_count = read_prop(&props, "simd_count")?;
        let simd_per_cu = read_prop(&props, "simd_per_cu")?;
        let drm_render_minor = read_prop(&props, "drm_render_minor")?;
        return Ok(NodeInfo {
            gpu_id: id,
            gfx,
            cwsr_size,
            ctl_stack_size,
            simd_count,
            simd_per_cu,
            drm_render_minor,
        });
    }
    Err(Error::other("find_node", "no nonzero gpu_id"))
}

/// Find the render node path for this kfd node.
///
/// The node's properties name drm_render_minor directly; the
/// render node is renderD(minor).
pub fn render_path(node: &NodeInfo) -> String {
    format!("/dev/dri/renderD{}", node.drm_render_minor)
}
