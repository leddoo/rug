use core::ffi::c_void;

use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{
                CreateThread,
                ExitProcess,
            },
        },
        UI::{
            Input::KeyboardAndMouse::VIRTUAL_KEY,
            WindowsAndMessaging::*,
        },
    },
};

use std::sync::mpsc;


pub fn run(main: fn()) {
    unsafe { _run(main) }
}

pub fn exit() -> ! {
    unsafe { ExitProcess(0) }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window (HWND);

impl Window {
    pub fn new() -> Window {
        unsafe { Window(_create_window()) }
    }

    pub fn size(self) -> (u32, u32) {
        unsafe { _get_window_size(self.0) }
    }

    pub fn fill_pixels(self, buffer: &[u32], x: i32, y: i32, w: u32, h: u32) {
        unsafe { _fill_pixels(self.0, buffer, x, y, w, h) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Close (HWND),
    MouseMove (HWND, u32, u32),
    MouseDown (HWND, u32, u32, MouseButton),
    MouseUp   (HWND, u32, u32, MouseButton),
    MouseWheel (HWND, i32),
    KeyDown (HWND, VIRTUAL_KEY),
    KeyUp   (HWND, VIRTUAL_KEY),
    Char (HWND, u16),
    Size (HWND, u32, u32),
    Paint (HWND),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}


pub fn peek_event() -> Option<Event> {
    unsafe { _peek_event() }
}

#[allow(dead_code)]
pub fn next_event() -> Event {
    unsafe { _next_event() }
}

pub fn next_event_timeout(timeout: std::time::Duration) -> Option<Event> {
    unsafe { _next_event_timeout(timeout) }
}


unsafe fn _run(main: fn()) {
    let instance = GetModuleHandleW(None).unwrap();

    // create message window
    {
        let wc = WNDCLASSW {
            hInstance: instance,
            lpszClassName: MSG_WINDOW_CLASS_NAME,
            lpfnWndProc: Some(msg_window_proc),
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        assert!(atom != 0);

        let window = CreateWindowExW(
            Default::default(),
            MSG_WINDOW_CLASS_NAME,
            "message_window",
            Default::default(),
            CW_USEDEFAULT, CW_USEDEFAULT,
            CW_USEDEFAULT, CW_USEDEFAULT,
            Some(HWND_MESSAGE),
            None,
            instance,
            core::ptr::null(),
        );
        assert!(window.0 != 0);

        MESSAGE_WINDOW = window;
    };

    // set up user window class
    {
        let wc = WNDCLASSW {
            hInstance: instance,
            lpszClassName: USER_WINDOW_CLASS_NAME,
            lpfnWndProc: Some(user_window_proc),
            hIcon: LoadIconW(None, IDI_APPLICATION).unwrap(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        assert!(atom != 0);
    }

    // create user thread
    {
        CreateThread(
            core::ptr::null(),
            0,
            Some(main_thread_proc),
            main as *const c_void,
            Default::default(),
            &mut USER_THREAD_ID,
        ).unwrap();
    }

    let (sender, receiver) = mpsc::channel();
    EVENT_QUEUE_SENDER   = Some(sender);
    EVENT_QUEUE_RECEIVER = Some(receiver);

    INITIALIZED = true;

    // event loop
    loop {
        let mut message = MSG::default();
        GetMessageW(&mut message, HWND(0), 0, 0);
        TranslateMessage(&mut message);
        DispatchMessageW(&message);
    }
}


// "message_window_class"
const MSG_WINDOW_CLASS_NAME: PCWSTR = PCWSTR([109, 101, 115, 115, 97, 103, 101, 95, 119, 105, 110, 100, 111, 119, 95, 99, 108, 97, 115, 115, 0].as_ptr());

// "user_window_class"
const USER_WINDOW_CLASS_NAME: PCWSTR = PCWSTR([117, 115, 101, 114, 95, 119, 105, 110, 100, 111, 119, 95, 99, 108, 97, 115, 115, 0].as_ptr());

const MSG_CREATE_WINDOW:  u32 = WM_USER + 42;
const MSG_DESTROY_WINDOW: u32 = WM_USER + 69;


static mut INITIALIZED: bool = false;
static mut USER_THREAD_ID: u32 = 0;
static mut MESSAGE_WINDOW: HWND = HWND(0);
static mut EVENT_QUEUE_SENDER:   Option<mpsc::Sender<Event>> = None;
static mut EVENT_QUEUE_RECEIVER: Option<mpsc::Receiver<Event>> = None;


unsafe fn _create_window() -> HWND {
    assert!(INITIALIZED);
    HWND(SendMessageW(MESSAGE_WINDOW, MSG_CREATE_WINDOW, WPARAM(0), LPARAM(0)).0)
}

unsafe fn _get_window_size(window: HWND) -> (u32, u32) {
    let mut rect = RECT::default();
    let r = GetClientRect(window, &mut rect);
    assert!(r.0 != 0);

    ((rect.right - rect.left) as u32, (rect.bottom - rect.top) as u32)
}

unsafe fn _fill_pixels(window: HWND, buffer: &[u32], x: i32, y: i32, w: u32, h: u32) {
    if w == 0 || h == 0 {
        return;
    }

    let window_height = _get_window_size(window).1 as i32;
    let y = window_height - (y + h as i32);

    let dc = GetDC(window);
    assert!(dc.0 != 0);

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize:        core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth:       w as i32,
            biHeight:      h as i32,
            biPlanes:      1,
            biBitCount:    32,
            biCompression: BI_RGB as u32,
            ..Default::default()
        },
        ..Default::default()
    };

    let r = SetDIBitsToDevice(
        dc,
        x, y,
        w, h,
        0, 0,
        0, h,
        buffer.as_ptr() as *const c_void,
        &bmi,
        DIB_RGB_COLORS,
    );
    assert!(r == h as i32);

    let r = ReleaseDC(window, dc);
    assert!(r == 1);
}


unsafe fn _peek_event() -> Option<Event> {
    assert!(INITIALIZED);
    let receiver = EVENT_QUEUE_RECEIVER.as_mut().unwrap();
    match receiver.try_recv() {
        Ok(event) => Some(event),
        Err(mpsc::TryRecvError::Empty) => None,
        _ => panic!(),
    }
}

unsafe fn _next_event() -> Event {
    assert!(INITIALIZED);
    let receiver = EVENT_QUEUE_RECEIVER.as_mut().unwrap();
    receiver.recv().unwrap()
}

unsafe fn _next_event_timeout(timeout: std::time::Duration) -> Option<Event> {
    assert!(INITIALIZED);
    let receiver = EVENT_QUEUE_RECEIVER.as_mut().unwrap();
    match receiver.recv_timeout(timeout) {
        Ok(event) => Some(event),
        Err(mpsc::RecvTimeoutError::Timeout) => None,
        _ => panic!(),
    }
}


unsafe extern "system" fn msg_window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message as u32 {
        MSG_CREATE_WINDOW => {
            let window = CreateWindowExW(
                Default::default(),
                USER_WINDOW_CLASS_NAME,
                "window",
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT, CW_USEDEFAULT,
                CW_USEDEFAULT, CW_USEDEFAULT,
                None,
                None,
                GetModuleHandleW(None).unwrap(),
                core::ptr::null(),
            );
            assert!(window.0 != 0);

            LRESULT(window.0)
        },

        MSG_DESTROY_WINDOW => {
            LRESULT(0)
        },

        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

unsafe extern "system" fn user_window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    assert!(INITIALIZED);
    let queue = EVENT_QUEUE_SENDER.as_mut().unwrap();

    fn low_u16(a: isize) -> u32 {
        (a as usize as u32) & 0xffff
    }

    fn high_u16(a: isize) -> u32 {
        ((a as usize as u32) >> 16) & 0xffff
    }

    let message = message as u32;
    match message {
        WM_CLOSE => {
            queue.send(Event::Close(window)).unwrap();
            LRESULT(0)
        },

        WM_MOUSEMOVE => {
            let x = low_u16(lparam.0);
            let y = high_u16(lparam.0);
            queue.send(Event::MouseMove(window, x, y)).unwrap();
            LRESULT(0)
        },

        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            let x = low_u16(lparam.0);
            let y = high_u16(lparam.0);
            let button = match message {
                WM_LBUTTONDOWN => MouseButton::Left,
                WM_RBUTTONDOWN => MouseButton::Right,
                WM_MBUTTONDOWN => MouseButton::Middle,
                _ => unreachable!(),
            };
            queue.send(Event::MouseDown(window, x, y, button)).unwrap();
            LRESULT(0)
        },

        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
            let x = low_u16(lparam.0);
            let y = high_u16(lparam.0);
            let button = match message {
                WM_LBUTTONUP => MouseButton::Left,
                WM_RBUTTONUP => MouseButton::Right,
                WM_MBUTTONUP => MouseButton::Middle,
                _ => unreachable!(),
            };
            queue.send(Event::MouseUp(window, x, y, button)).unwrap();
            LRESULT(0)
        },

        WM_MOUSEWHEEL => {
            let delta = high_u16(wparam.0 as isize) as i16 as i32 / 120;
            queue.send(Event::MouseWheel(window, delta)).unwrap();
            LRESULT(0)
        }

        WM_KEYDOWN => {
            let key = VIRTUAL_KEY(wparam.0 as usize as u16);
            queue.send(Event::KeyDown(window, key)).unwrap();
            LRESULT(0)
        },

        WM_KEYUP => {
            let key = VIRTUAL_KEY(wparam.0 as usize as u16);
            queue.send(Event::KeyUp(window, key)).unwrap();
            LRESULT(0)
        },

        WM_CHAR => {
            let chr = wparam.0 as usize as u16;
            queue.send(Event::Char(window, chr)).unwrap();
            LRESULT(0)
        },

        WM_SIZE => {
            let w = low_u16(lparam.0);
            let h = high_u16(lparam.0);
            queue.send(Event::Size(window, w, h)).unwrap();
            LRESULT(0)
        },

        WM_PAINT => {
            queue.send(Event::Paint(window)).unwrap();
            ValidateRect(window, std::ptr::null());
            LRESULT(0)
        },

        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}


unsafe extern "system" fn main_thread_proc(thread_parameter: *mut c_void) -> u32 {
    let main: fn() = core::mem::transmute(thread_parameter);

    let result = std::panic::catch_unwind(main);
    if let Err(_) = result {
        ExitProcess(1);
    }

    ExitProcess(0);
}

