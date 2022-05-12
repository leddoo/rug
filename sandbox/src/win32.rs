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
        UI::WindowsAndMessaging::*,
    },
};


pub fn run(main: fn()) {
    unsafe { _run(main) }
}

#[allow(dead_code)]
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
            biHeight:      -(h as i32),
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
    match message as u32 {
        WM_PAINT => {
            ValidateRect(window, std::ptr::null());
            LRESULT(0)
        },

        WM_CLOSE => { LRESULT(0) },

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

