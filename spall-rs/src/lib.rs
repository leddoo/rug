use core::sync::atomic::{AtomicBool, AtomicUsize, AtomicPtr, Ordering};
use core::cell::{Cell, UnsafeCell};
use core::mem::{ManuallyDrop, size_of};

use std::sync::Arc;
use std::sync::mpsc;
use std::thread::JoinHandle;



pub static CTX: Ctx = Ctx::new();

thread_local!(
    static THREAD_CTX: ThreadCtx = ThreadCtx::new_timed();
);


/// force initialization on the current thread.
pub fn touch() {
    THREAD_CTX.with(|cx| {
        // just use the context in some way.
        // not sure whether `with` is enough to trigger init.
        unsafe { (&cx.pid as *const u32).read_volatile(); }
    });
}



// events

#[repr(C, packed)]
pub struct SpallHeader {
    pub magic_header:   u64, // = 0x0BADF00D
    pub version:        u64, // = 1
    pub timestamp_unit: f64,
    pub must_be_0:      u64, // = 0
}

pub enum EventType {
    Invalid            = 0,
    CustomData         = 1, // Basic readers can skip this.
    StreamOver         = 2,

    Begin              = 3,
    End                = 4,
    Instant            = 5,

    OverwriteTimestamp = 6, // Retroactively change timestamp units - useful for incrementally improving RDTSC frequency.
    PadSkip            = 7,
}

#[repr(C, packed)]
pub struct BeginEvent {
    pub ty:       u8, // = SpallEventType_Begin
    pub category: u8,

    pub pid:  u32,
    pub tid:  u32,
    pub when: f64,

    pub name_length: u8,
    pub args_length: u8,
}

impl BeginEvent {
    #[inline(always)]
    fn size(name_len: u8, args_len: u8) -> usize {
        size_of::<Self>() + name_len as usize + args_len as usize
    }
}

#[repr(C, packed)]
pub struct BeginEventMax {
    pub event:      BeginEvent,
    pub name_bytes: [u8; 255],
    pub args_bytes: [u8; 255],
}

#[repr(C, packed)]
pub struct EndEvent {
    pub ty:   u8, // = SpallEventType_End
    pub pid:  u32,
    pub tid:  u32,
    pub when: f64,
}

#[repr(C, packed)]
pub struct PadSkipEvent {
    pub ty:   u8, // = SpallEventType_Pad_Skip
    pub size: u32,
}



// global context.

pub const DEFAULT_BUFFER_SIZE: usize = 2*1024*1024;

pub struct Ctx {
    pub default_buffer_size: AtomicUsize,

    /// whether any event names or args were truncated.
    pub truncated: AtomicBool,
    /// whether any events were dropped.
    pub dropped: AtomicBool,
}

impl Ctx {
    const fn new() -> Self {
        Self {
            default_buffer_size: AtomicUsize::new(DEFAULT_BUFFER_SIZE),

            truncated: AtomicBool::new(false),
            dropped:   AtomicBool::new(false),
        }
    }
}



// timing.

#[cfg(target_arch = "aarch64")]
mod timing {
    #[inline(always)]
    pub fn rdtsc() -> u64 {
        let tsc: u64;
        unsafe {
            core::arch::asm!(
                "mrs {tsc}, cntvct_el0",
                tsc = out(reg) tsc,
            );
        }
        tsc
    }

    #[inline(always)]
    pub fn tsc_freq() -> u64 {
        let freq: u64;
        unsafe {
            core::arch::asm!(
                "mrs {freq}, cntfrq_el0",
                freq = out(reg) freq,
            );
        }
        freq
    }
}

pub use timing::*;



// thread context.

struct ThreadCtx {
    sender: ManuallyDrop<mpsc::Sender<()>>,
    writer: ManuallyDrop<JoinHandle<()>>,

    pid: u32,
    tid: u32,

    buffer: Arc<Buffer>,
}

impl ThreadCtx {
    fn new() -> Self {
        let buffer = Arc::new({
            let size = CTX.default_buffer_size.load(Ordering::Relaxed);

            let mut data = UnsafeCell::new(vec![0u8; size].into_boxed_slice());
            let data_ptr = data.get_mut().as_mut_ptr();
            Buffer {
                _data: data,
                data_ptr: data_ptr.into(),
                half_len: size/2,

                head:      data_ptr.into(),
                remaining: (size/2).into(),
                top_half:  false.into(),

                writer_ptr: AtomicPtr::new(core::ptr::null_mut()),
                writer_len: 0.into(),
            }
        });

        let (sender, receiver) = mpsc::channel();

        let writer = std::thread::spawn({
            let buffer = buffer.clone();
            move || { writer(buffer, receiver); }
        });

        let pid = std::process::id();
        let tid = unsafe {
            core::mem::transmute::<_, u64>(
                std::thread::current().id())
            as u32
        };

        Self {
            sender: ManuallyDrop::new(sender),
            writer: ManuallyDrop::new(writer),
            pid, tid,
            buffer,
        }
    }

    fn new_timed() -> Self {
        let t0 = rdtsc();
        let result = Self::new();
        let t1 = rdtsc();

        result.ev_begin(t0, "spall thread startup", "");
        result.ev_end(t1);

        result
    }

    fn shutdown(&mut self) {
        let sender = unsafe { ManuallyDrop::take(&mut self.sender) };
        let writer = unsafe { ManuallyDrop::take(&mut self.writer) };

        // signal quit to writer.
        drop(sender);

        // wait for writer to terminate.
        writer.join().unwrap();
    }


    #[must_use]
    #[inline(always)]
    unsafe fn write(&self, size: usize) -> Option<*mut u8> {
        let this = &self.buffer;

        let remaining = this.remaining.get();
        if remaining < size {
            // writer still busy?
            if !this.writer_ptr.load(Ordering::SeqCst).is_null() {
                println!("!!write dropped!!");
                CTX.dropped.store(true, Ordering::Relaxed);
                return None;
            }

            let old_offset = if this.top_half.get() { this.half_len } else { 0 };
            let new_offset = if this.top_half.get() { 0 } else { this.half_len };

            let data_ptr = unsafe { *this.data_ptr.as_ptr() };

            let old_ptr = unsafe { data_ptr.add(old_offset) };
            let new_ptr = unsafe { data_ptr.add(new_offset) };

            // notify writer.
            this.writer_len.store(this.half_len - remaining, Ordering::SeqCst);
            this.writer_ptr.store(old_ptr, Ordering::SeqCst);
            self.sender.send(()).unwrap();

            // swap buffers.
            unsafe {
                *this.head.as_ptr() = new_ptr;
                this.remaining.set(this.half_len);
                this.top_half.set(!this.top_half.get());
            }
        }

        unsafe {
            let head = &mut *this.head.as_ptr();
            let result = *head;

            *head = head.add(size);
            this.remaining.set(this.remaining.get() - size);

            return Some(result);
        }
    }

    #[inline(always)]
    fn ev_begin(&self, time: u64, name: &str, args: &str) {
        if name.len() > 255 || args.len() > 255 {
            CTX.truncated.store(true, Ordering::Relaxed);
        }

        let trunc_name_len = name.len().min(255);
        let trunc_args_len = name.len().min(255);

        let size = BeginEvent::size(trunc_name_len as u8, trunc_args_len as u8);

        if let Some(ptr) = unsafe { self.write(size) } {
            unsafe {
                (ptr as *mut BeginEvent).write(BeginEvent {
                    ty: EventType::Begin as u8,
                    category: 0,
                    pid:  self.pid,
                    tid:  self.tid,
                    when: time as f64,
                    name_length: trunc_name_len as u8,
                    args_length: trunc_args_len as u8,
                });

                core::ptr::copy_nonoverlapping(
                    name.as_ptr(),
                    ptr.add(size_of::<BeginEvent>()),
                    trunc_name_len);

                core::ptr::copy_nonoverlapping(
                    args.as_ptr(),
                    ptr.add(size_of::<BeginEvent>() + trunc_name_len),
                    trunc_args_len);
            }
        }
    }

    #[inline(always)]
    fn ev_end(&self, time: u64) {
        if let Some(ptr) = unsafe { self.write(size_of::<EndEvent>()) } {
            unsafe {
                (ptr as *mut EndEvent).write(EndEvent {
                    ty: EventType::End as u8,
                    pid:  self.pid,
                    tid:  self.tid,
                    when: time as f64,
                });
            }
        }
    }
}

impl Drop for ThreadCtx {
    fn drop(&mut self) {
        self.shutdown();
    }
}


struct Buffer {
    _data: UnsafeCell<Box<[u8]>>,
    data_ptr: AtomicPtr<u8>,
    half_len: usize,

    head:      AtomicPtr<u8>,
    remaining: Cell<usize>,
    top_half:  Cell<bool>,

    writer_ptr: AtomicPtr<u8>,
    writer_len: AtomicUsize,
}

unsafe impl Sync for Buffer {}


fn writer(buffer: Arc<Buffer>, receiver: mpsc::Receiver<()>) {
    while let Ok(()) = receiver.recv() {
        let ptr = buffer.writer_ptr.load(Ordering::SeqCst);
        if ptr.is_null() {
            continue;
        }

        //let len = buffer.writer_len.load(Ordering::SeqCst);

        std::thread::sleep(std::time::Duration::from_millis(1));

        buffer.writer_len.store(0, Ordering::SeqCst);
        buffer.writer_ptr.store(core::ptr::null_mut(), Ordering::SeqCst);
    }
}



// macro api.

#[macro_export]
macro_rules! trace_scope {
    ($name:expr) => {
        let _trace_scope_ = ::spall::trace_scope_impl($name, "");
    };

    ($name:expr , $arg:expr) => {
        let _trace_scope_ = ::spall::trace_scope_impl($name, $arg);
    };

    ($name:expr ; $($args:tt)+) => {
        // @temp!!!!
        let _trace_scope_ = ::spall::trace_scope_impl($name, &format!($($args)+));
    };
}


#[inline(always)]
pub fn trace_scope_impl(name: &str, args: &str) -> TraceScope {
    THREAD_CTX.with(|cx| {
        cx.ev_begin(rdtsc(), name, args);
    });
    TraceScope {}
}


pub struct TraceScope;

impl Drop for TraceScope {
    #[inline(always)]
    fn drop(&mut self) {
        THREAD_CTX.with(|cx| {
            cx.ev_end(rdtsc());
        });
    }
}

