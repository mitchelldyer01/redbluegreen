//! rbg - talk to the GPU driver through /dev/kfd.
//!
//! The syscalls and ioctls are hand-declared in the kfd module.
//! Every failure is a Result; main prints it and exits 1. No
//! crates.

mod kfd;

const MEM_SIZE: usize = 1024 * 1024;
const RING_SIZE: u64 = 64 * 1024;

fn usage() -> ! {
    eprintln!("usage: rbg probe [--acquire] | rbg mem | rbg queue");
    eprintln!("        [--cwsr-short] | rbg dispatch [--no-doorbell]");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let known = matches!(
        args.first().map(String::as_str),
        Some("probe") | Some("mem") | Some("queue") | Some("dispatch")
    );
    if !known {
        usage();
    }
    let result: kfd::Result<()> = match args.first().map(String::as_str) {
        Some("probe") => run_probe(&args),
        Some("mem") => run_mem(),
        Some("queue") => run_queue(&args),
        Some("dispatch") => run_dispatch(&args),
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

fn queue_args(short: bool, node: &kfd::NodeInfo) -> (u64, u64, u32) {
    // CWSR: ALIGN(cwsr_size + debug, 4096), with
    // debug = ALIGN(cu * 32 * 32, 64) and cu = simd_count /
    // simd_per_cu.
    let cu = node.simd_count / node.simd_per_cu;
    let debug = (cu * 32 * 32 + 63) & !63;
    let mut cwsr_bo = (node.cwsr_size + debug + 4095) & !4095;
    if short {
        cwsr_bo -= 4096;
    }
    (node.cwsr_size, cwsr_bo, node.ctl_stack_size as u32)
}

fn run_queue(args: &[String]) -> kfd::Result<()> {
    // --cwsr-short: pass a context area 4 KiB too small, so the
    // kernel answers EINVAL. Tests that every Drop runs on error.
    let short = args.iter().skip(1).any(|a| a == "--cwsr-short");
    let k = open_kfd()?;
    let node = kfd::find_node()?;

    let (cwsr_size, cwsr_bo, ctl) = queue_args(short, &node);

    let mut q = kfd::Queue::new(&k, RING_SIZE, cwsr_size, cwsr_bo, ctl)?;
    let ev = kfd::Event::new(&k)?;

    println!("queue_id: {}", q.queue_id);
    println!("doorbell_offset: {:#x}", q.doorbell_offset);
    println!("event_id: {}", ev.event_id);
    println!("event_slot_index: {}", ev.event_slot_index);

    // The read pointer is a u64 the hardware advances; it starts
    // at zero.
    let rptr = q.read_ptr.as_slice_mut::<u64>()[0];
    println!("rptr: {rptr}");

    // Event, then Queue, drop here. The Queue drops its Buffers
    // after DESTROY_QUEUE and the doorbell munmap.
    drop(ev);
    drop(q);
    Ok(())
}

fn run_dispatch(args: &[String]) -> kfd::Result<()> {
    // --no-doorbell: write the packet and the write pointer,
    // skip the doorbell. The signal must time out.
    let no_doorbell = args.iter().skip(1).any(|a| a == "--no-doorbell");
    let k = open_kfd()?;
    let node = kfd::find_node()?;
    let (cwsr_size, cwsr_bo, ctl) = queue_args(false, &node);
    let mut q = kfd::Queue::new(&k, RING_SIZE, cwsr_size, cwsr_bo, ctl)?;

    let kern = kfd::Kernel::new(&k)?;
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
    let wait_err = sig.wait(std::time::Duration::from_secs(5)).err();

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
