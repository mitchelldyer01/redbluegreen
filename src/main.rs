//! rbg - talk to the GPU driver through /dev/kfd.
//!
//! The syscalls and ioctls are hand-declared in the kfd module.
//! Every failure is a Result; main prints it and exits 1. No
//! crates.

mod kfd;

const MEM_SIZE: usize = 1024 * 1024;
const RING_SIZE: u64 = 64 * 1024;

fn usage() -> ! {
    eprintln!("usage: rbg probe [--acquire] | rbg mem");
    eprintln!("        | rbg queue [--cwsr-short]");
    eprintln!("        | rbg dispatch [--no-doorbell]");
    eprintln!("        | rbg add N [--fine] [--reps R] [--wg W] [--kernel K]");
    eprintln!("  queue, dispatch and add also take --debug");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let known = matches!(
        args.first().map(String::as_str),
        Some("probe")
            | Some("mem")
            | Some("queue")
            | Some("dispatch")
            | Some("add")
    );
    if !known {
        usage();
    }
    let result: kfd::Result<()> = match args.first().map(String::as_str) {
        Some("probe") => run_probe(&args),
        Some("mem") => run_mem(),
        Some("queue") => run_queue(&args),
        Some("dispatch") => run_dispatch(&args),
        Some("add") => run_add(&args),
        _ => unreachable!("checked above"),
    };
    if let Err(e) = result {
        eprintln!("rbg: {e}");
        std::process::exit(1);
    }
}

fn open_kfd() -> kfd::Result<kfd::Kfd> {
    let node = kfd::find_node()?;
    let render = kfd::render_path(&node);
    kfd::Kfd::new(&node, &render)
}

fn run_probe(args: &[String]) -> kfd::Result<()> {
    let acquire = args.iter().skip(1).any(|a| a == "--acquire");
    let node = kfd::find_node()?;
    let kfd_fd = kfd::open_rw("/dev/kfd")?;
    let version = kfd::get_version(kfd_fd)?;
    println!("kfd version: {}.{}", version.major, version.minor);
    println!("gpu_id: {}", node.gpu_id);
    println!("gfx_target_version: {}", node.gfx);
    if acquire {
        let render = kfd::render_path(&node);
        let render_fd = kfd::open_rw(&render)?;
        kfd::acquire_vm(kfd_fd, render_fd, node.gpu_id)?;
        println!("acquire_vm: ok");
        kfd::close_fd(render_fd);
    }
    kfd::close_fd(kfd_fd);
    Ok(())
}

fn run_mem() -> kfd::Result<()> {
    let k = open_kfd()?;
    let flags = kfd::ALLOC_GTT | kfd::ALLOC_WRITABLE;
    let mut buf = kfd::Buffer::new(&k, MEM_SIZE as u64, flags)?;
    println!("handle: {}", buf.handle);
    println!("mmap_offset: {}", buf.mmap_offset);
    println!("va: 0x{:x}", buf.va as u64);

    let n = MEM_SIZE / 4;
    for (i, w) in buf.as_slice_mut::<u32>().iter_mut().enumerate() {
        *w = i as u32;
    }

    // Drop the host view, take a fresh one at the same VA.
    buf.remap()?;
    let bad = buf
        .as_slice_mut::<u32>()
        .iter()
        .enumerate()
        .filter(|(i, w)| **w != *i as u32)
        .count();

    // Teardown (munmap, unmap from gpu, free) runs even on a
    // verify failure: the Buffer and the Kfd drop here.
    if bad != 0 {
        return Err(kfd::Error::other(
            "verify",
            &format!("{bad} of {n} values wrong"),
        ));
    }
    println!("verify: {n} ok");
    Ok(())
}

fn queue_args(short: bool, node: &kfd::NodeInfo) -> (u64, u64, u64, u32) {
    // CWSR: ALIGN(cwsr_size + debug, 4096), with
    // debug = ALIGN(cu * 32 * 32, 64) and cu = simd_count /
    // simd_per_cu. The debug size fills the header's DebugSize.
    let cu = node.simd_count / node.simd_per_cu;
    let debug = (cu * 32 * 32 + 63) & !63;
    let mut cwsr_bo = (node.cwsr_size + debug + 4095) & !4095;
    if short {
        cwsr_bo -= 4096;
    }
    (node.cwsr_size, cwsr_bo, debug, node.ctl_stack_size as u32)
}

/// The --debug switch, shared by queue, dispatch and add.
/// Prints the CWSR header words once at creation.
fn debug_switch(args: &[String]) -> bool {
    args.iter().any(|a| a == "--debug")
}

/// `--wait S`: seconds to wait on a completion signal (default 5).
/// A long wait leaves time to read /sys/kernel/debug/kfd during a hang.
fn wait_secs(args: &[String]) -> std::time::Duration {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--wait" {
            if let Some(v) = it.next().and_then(|v| v.parse::<u64>().ok()) {
                return std::time::Duration::from_secs(v);
            }
        }
    }
    std::time::Duration::from_secs(5)
}

/// The 4 KiB COHERENT error payload the CWSR header names.
fn error_payload(k: &kfd::Kfd) -> kfd::Result<kfd::Buffer<'_>> {
    kfd::Buffer::new(
        k,
        4096,
        kfd::ALLOC_GTT | kfd::ALLOC_WRITABLE | kfd::ALLOC_COHERENT,
    )
}

fn run_queue(args: &[String]) -> kfd::Result<()> {
    // --cwsr-short: pass a context area 4 KiB too small, so the
    // kernel answers EINVAL. Tests that every Drop runs on error.
    let short = args.iter().skip(1).any(|a| a == "--cwsr-short");
    let debug = debug_switch(args);
    let k = open_kfd()?;
    let node = kfd::find_node()?;

    let (cwsr_size, cwsr_bo, dbg, ctl) = queue_args(short, &node);

    // Event and error payload before the Queue: the CWSR
    // header names both, so they must outlive it.
    let ev = kfd::Event::new(&k)?;
    let ep = error_payload(&k)?;
    let opts = kfd::QueueOpts {
        event: &ev,
        error_payload: &ep,
        verbose: debug,
    };
    let mut q =
        kfd::Queue::new(&k, RING_SIZE, cwsr_size, cwsr_bo, dbg, ctl, &opts)?;

    println!("queue_id: {}", q.queue_id);
    println!("doorbell_offset: {:#x}", q.doorbell_offset);
    println!("event_id: {}", ev.event_id);
    println!("event_slot_index: {}", ev.event_slot_index);

    // The read pointer is a u64 the hardware advances; it starts
    // at zero.
    let rptr = q.read_ptr.as_slice_mut::<u64>()[0];
    println!("rptr: {rptr}");

    // Queue first, then Event, then payload, drop here. The
    // Queue drops its Buffers after DESTROY_QUEUE and the
    // doorbell munmap.
    drop(q);
    drop(ev);
    drop(ep);
    Ok(())
}

fn run_dispatch(args: &[String]) -> kfd::Result<()> {
    // --no-doorbell: write the packet and the write pointer,
    // skip the doorbell. The signal must time out.
    let no_doorbell = args.iter().skip(1).any(|a| a == "--no-doorbell");
    let debug = debug_switch(args);
    let k = open_kfd()?;
    let node = kfd::find_node()?;
    let (cwsr_size, cwsr_bo, dbg, ctl) = queue_args(false, &node);
    let ev = kfd::Event::new(&k)?;
    let ep = error_payload(&k)?;
    let opts = kfd::QueueOpts {
        event: &ev,
        error_payload: &ep,
        verbose: debug,
    };
    let mut q =
        kfd::Queue::new(&k, RING_SIZE, cwsr_size, cwsr_bo, dbg, ctl, &opts)?;

    let kd: &[u8; 64] = include_bytes!("../kernels/store42.kd");
    let text: &[u8] = include_bytes!("../kernels/store42.text");
    let kern = kfd::Kernel::new(&k, kd, text)?;
    let sig = kfd::Signal::new(&k)?;
    let flags = kfd::ALLOC_GTT | kfd::ALLOC_WRITABLE | kfd::ALLOC_COHERENT;
    let mut kernarg = kfd::Buffer::new(&k, 4096, flags)?;
    let mut target = kfd::Buffer::new(&k, 4096, flags)?;
    for x in target.as_slice_mut::<u8>().iter_mut() {
        *x = 0;
    }

    // The kernarg buffer holds the u64 the kernel writes to.
    kernarg.as_slice_mut::<u8>()[0..8]
        .copy_from_slice(&(target.va as u64).to_le_bytes());

    // The 64-byte packet, docs/dispatch.md.
    let mut p = [0u8; 64];
    p[0..2].copy_from_slice(&0x1502u16.to_le_bytes()); // header
    p[2..4].copy_from_slice(&1u16.to_le_bytes()); // setup, 1 dim
    p[4..6].copy_from_slice(&1u16.to_le_bytes()); // workgroup x
    p[6..8].copy_from_slice(&1u16.to_le_bytes()); // workgroup y
    p[8..10].copy_from_slice(&1u16.to_le_bytes()); // workgroup z
    p[12..16].copy_from_slice(&1u32.to_le_bytes()); // grid x
    p[16..20].copy_from_slice(&1u32.to_le_bytes()); // grid y
    p[20..24].copy_from_slice(&1u32.to_le_bytes()); // grid z
    p[32..40].copy_from_slice(&kern.object().to_le_bytes());
    p[40..48].copy_from_slice(&(kernarg.va as u64).to_le_bytes());
    p[56..64].copy_from_slice(&sig.va().to_le_bytes());

    q.dispatch(&p, !no_doorbell);

    // Poll up to 5 s. On timeout we still print both values
    // below and then return the timeout error; --no-doorbell
    // expects exactly this.
    let wait_err = sig.wait(wait_secs(args)).err();

    // Take a fresh host view, as run_mem does, then read.
    target.remap()?;
    let v = unsafe { std::ptr::read_volatile(target.va as *const u32) };
    let s = sig.value();
    println!("target[0]: {v}");
    println!("signal: {s}");
    if let Some(e) = wait_err {
        return Err(e);
    }
    if v == 42 {
        println!("dispatch: ok");
        Ok(())
    } else {
        Err(kfd::Error::other("dispatch", "target is not 42"))
    }
}

/// Parse `add N [--fine] [--reps R] [--wg W] [--kernel K]`.
/// --wg sets the workgroup size (default 256; vadd assumes 256,
/// so other values are only valid with N = 1). --kernel names
/// the kernel: vadd by default, or vadd-v4hang, the kernel
/// that hung with next_free_vgpr 5.
fn add_args(args: &[String]) -> kfd::Result<(u32, bool, u32, u32, String)> {
    let mut n: u32 = 0;
    let mut fine = false;
    let mut reps = 5u32;
    let mut wg = 256u32;
    let mut kernel = String::from("vadd");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--fine" => fine = true,
            // Read by debug_switch; accepted here so it parses.
            "--debug" => {}
            "--wait" => {
                i += 1; // value read by wait_secs
            }
            "--kernel" => {
                i += 1;
                if i >= args.len() {
                    return Err(kfd::Error::other(
                        "add",
                        "--kernel takes a name",
                    ));
                }
                kernel = args[i].clone();
            }
            "--wg" => {
                i += 1;
                if i >= args.len() {
                    return Err(kfd::Error::other("add", "--wg takes a value"));
                }
                wg = args[i]
                    .parse()
                    .map_err(|_| kfd::Error::other("add", "bad --wg"))?;
            }
            "--reps" => {
                i += 1;
                if i >= args.len() {
                    return Err(kfd::Error::other(
                        "add",
                        "--reps takes a value",
                    ));
                }
                reps = args[i]
                    .parse()
                    .map_err(|_| kfd::Error::other("add", "bad --reps"))?;
            }
            s => {
                n = s.parse().map_err(|_| kfd::Error::other("add", "bad N"))?;
            }
        }
        i += 1;
    }
    if n == 0 || reps == 0 || wg == 0 || wg > 1024 {
        return Err(kfd::Error::other("add", "N, reps, wg out of range"));
    }
    Ok((n, fine, reps, wg, kernel))
}

fn run_add(args: &[String]) -> kfd::Result<()> {
    let (n, fine, reps, wg, kernel) = add_args(args)?;
    let kernel = kernel.as_str();
    let debug = debug_switch(args);
    let k = open_kfd()?;
    let node = kfd::find_node()?;
    let (cwsr_size, cwsr_bo, dbg, ctl) = queue_args(false, &node);
    let ev = kfd::Event::new(&k)?;
    let ep = error_payload(&k)?;
    let opts = kfd::QueueOpts {
        event: &ev,
        error_payload: &ep,
        verbose: debug,
    };
    let mut q =
        kfd::Queue::new(&k, RING_SIZE, cwsr_size, cwsr_bo, dbg, ctl, &opts)?;

    let (kd, text): (&[u8; 64], &[u8]) = match kernel {
        "vadd-v4hang" => (
            include_bytes!("../kernels/vadd-v4hang.kd"),
            include_bytes!("../kernels/vadd-v4hang.text"),
        ),
        _ => (
            include_bytes!("../kernels/vadd.kd"),
            include_bytes!("../kernels/vadd.text"),
        ),
    };
    let kern = kfd::Kernel::new(&k, kd, text)?;

    // Coarse by default, the ROCr choice for compute data; the
    // packet's system-scope fences make the writes visible.
    // --fine adds COHERENT.
    let mut flags = kfd::ALLOC_GTT | kfd::ALLOC_WRITABLE;
    if fine {
        flags |= kfd::ALLOC_COHERENT;
    }
    let size = (n as u64 * 4 + 4095) & !4095; // page-rounded
    let mut a = kfd::Buffer::new(&k, size, flags)?;
    let mut b = kfd::Buffer::new(&k, size, flags)?;
    let mut c = kfd::Buffer::new(&k, size, flags)?;

    for (i, x) in a.as_slice_mut::<f32>()[..n as usize].iter_mut().enumerate() {
        *x = (i & 0xFFFF) as f32;
    }
    for x in b.as_slice_mut::<f32>()[..n as usize].iter_mut() {
        *x = 1.0;
    }
    for x in c.as_slice_mut::<u32>()[..n as usize].iter_mut() {
        *x = 0;
    }
    // kernarg, 32 bytes: a u64 @0, b u64 @8, c u64 @16, n u32 @24.
    let ka = kfd::ALLOC_GTT | kfd::ALLOC_WRITABLE | kfd::ALLOC_COHERENT;
    let mut kernarg = kfd::Buffer::new(&k, 4096, ka)?;
    kernarg.as_slice_mut::<u8>()[0..8]
        .copy_from_slice(&(a.va as u64).to_le_bytes());
    kernarg.as_slice_mut::<u8>()[8..16]
        .copy_from_slice(&(b.va as u64).to_le_bytes());
    kernarg.as_slice_mut::<u8>()[16..24]
        .copy_from_slice(&(c.va as u64).to_le_bytes());
    kernarg.as_slice_mut::<u8>()[24..28].copy_from_slice(&n.to_le_bytes());

    // The 64-byte packet: workgroup 256, grid
    // ((n + 255) / 256) * 256, the rest as in run_dispatch.
    let grid_x = ((n as u64).div_ceil(wg as u64) * wg as u64) as u32;
    let mut p = [0u8; 64];
    p[0..2].copy_from_slice(&0x1502u16.to_le_bytes()); // header
    p[2..4].copy_from_slice(&1u16.to_le_bytes()); // setup, 1 dim
    p[4..6].copy_from_slice(&(wg as u16).to_le_bytes()); // workgroup x
    p[6..8].copy_from_slice(&1u16.to_le_bytes()); // workgroup y
    p[8..10].copy_from_slice(&1u16.to_le_bytes()); // workgroup z
    p[12..16].copy_from_slice(&grid_x.to_le_bytes()); // grid x
    p[16..20].copy_from_slice(&1u32.to_le_bytes()); // grid y
    p[20..24].copy_from_slice(&1u32.to_le_bytes()); // grid z
    p[32..40].copy_from_slice(&kern.object().to_le_bytes());
    p[40..48].copy_from_slice(&(kernarg.va as u64).to_le_bytes());

    let mut best: u64 = u64::MAX;
    for rep in 1..=reps {
        // The signal is fresh each rep; the buffers are reused.
        let sig = kfd::Signal::new(&k)?;
        p[56..64].copy_from_slice(&sig.va().to_le_bytes());
        let start = std::time::Instant::now();
        q.dispatch(&p, true);
        let waited = sig.wait(wait_secs(args));
        if let Err(e) = waited {
            // Diagnostics before the process exits into a hung
            // DESTROY_QUEUE: did the CP retire the packet, and
            // did the kernel write anything?
            let rptr = q.read_ptr.as_slice_mut::<u64>()[0];
            c.remap()?;
            let c0 = unsafe { std::ptr::read_volatile(c.va as *const u32) };
            println!(
                "timeout: rptr {rptr}, signal {}, c[0] bits {c0:#x}",
                sig.value()
            );
            return Err(e);
        }
        let us = start.elapsed().as_micros() as u64;
        println!("rep {rep}: {us} us");
        best = best.min(us);
    }
    // 12 * n bytes moved: a and b read, c written.
    let gbs = 12.0 * n as f64 / (best as f64 * 1e-6) / 1e9;
    println!("best: {best} us, GB/s: {gbs:.2}");

    // Take a fresh host view of c, then check every element.
    c.remap()?;
    let bad = (0..n as usize).find(|&i| {
        let bits =
            unsafe { std::ptr::read_volatile(c.va.add(i * 4) as *const u32) };
        f32::from_bits(bits) != (i & 0xFFFF) as f32 + 1.0
    });
    match bad {
        None => {
            println!("verify: {n} ok");
            Ok(())
        }
        Some(i) => {
            Err(kfd::Error::other("verify", &format!("first bad index {i}")))
        }
    }
}
