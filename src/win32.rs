// Ventana de Windows: se crea, se lee el teclado y el raton y se dibuja la
// imagen. Usa directamente la API del sistema (no son librerias externas).

#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

type HWND = *mut c_void;
type HINSTANCE = *mut c_void;
type HDC = *mut c_void;
type HANDLE = *mut c_void;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[repr(C)]
struct WNDCLASSW {
    style: u32,
    lpfnWndProc: Option<WNDPROC>,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: HINSTANCE,
    hIcon: HANDLE,
    hCursor: HANDLE,
    hbrBackground: HANDLE,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
}

#[repr(C)]
#[derive(Default)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: u32,
    wParam: WPARAM,
    lParam: LPARAM,
    time: u32,
    pt: POINT,
    lPrivate: u32,
}

#[repr(C)]
#[derive(Default)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct BITMAPINFOHEADER {
    biSize: u32,
    biWidth: i32,
    biHeight: i32,
    biPlanes: u16,
    biBitCount: u16,
    biCompression: u32,
    biSizeImage: u32,
    biXPelsPerMeter: i32,
    biYPelsPerMeter: i32,
    biClrUsed: u32,
    biClrImportant: u32,
}

#[repr(C)]
struct BITMAPINFO {
    bmiHeader: BITMAPINFOHEADER,
    bmiColors: [u32; 1],
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> u16;
    fn CreateWindowExW(
        dwExStyle: u32,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: u32,
        x: i32,
        y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: HWND,
        hMenu: HANDLE,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    fn DefWindowProcW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn DestroyWindow(hWnd: HWND) -> i32;
    fn PeekMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: u32, wMsgFilterMax: u32, wRemoveMsg: u32) -> i32;
    fn TranslateMessage(lpMsg: *const MSG) -> i32;
    fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    fn PostQuitMessage(nExitCode: i32);
    fn GetDC(hWnd: HWND) -> HDC;
    fn ReleaseDC(hWnd: HWND, hDC: HDC) -> i32;
    fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HANDLE;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> i32;
    fn AdjustWindowRect(lpRect: *mut RECT, dwStyle: u32, bMenu: i32) -> i32;
    fn SetWindowTextW(hWnd: HWND, lpString: *const u16) -> i32;
    fn GetAsyncKeyState(vKey: i32) -> i16;
    fn GetForegroundWindow() -> HWND;
    fn SetCapture(hWnd: HWND) -> HWND;
    fn ReleaseCapture() -> i32;
    fn SetProcessDPIAware() -> i32;
    fn ValidateRect(hWnd: HWND, lpRect: *const RECT) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn StretchDIBits(
        hdc: HDC,
        xDest: i32,
        yDest: i32,
        DestWidth: i32,
        DestHeight: i32,
        xSrc: i32,
        ySrc: i32,
        SrcWidth: i32,
        SrcHeight: i32,
        lpBits: *const c_void,
        lpbmi: *const BITMAPINFO,
        iUsage: u32,
        rop: u32,
    ) -> i32;
    fn SetStretchBltMode(hdc: HDC, mode: i32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
}

const WM_DESTROY: u32 = 0x0002;
const WM_PAINT: u32 = 0x000F;
const WM_CLOSE: u32 = 0x0010;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_KEYDOWN: u32 = 0x0100;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONDOWN: u32 = 0x0204;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_MOUSEWHEEL: u32 = 0x020A;
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;
const PM_REMOVE: u32 = 0x0001;
const SRCCOPY: u32 = 0x00CC_0020;
const COLORONCOLOR: i32 = 3;
const IDC_ARROW: usize = 32512;

pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
pub const VK_ESCAPE: i32 = 0x1B;
pub const VK_SPACE: i32 = 0x20;
pub const VK_PRIOR: i32 = 0x21;
pub const VK_NEXT: i32 = 0x22;
pub const VK_LEFT: i32 = 0x25;
pub const VK_UP: i32 = 0x26;
pub const VK_RIGHT: i32 = 0x27;
pub const VK_DOWN: i32 = 0x28;
pub const VK_ADD: i32 = 0x6B;
pub const VK_SUBTRACT: i32 = 0x6D;
pub const VK_OEM_PLUS: i32 = 0xBB;
pub const VK_OEM_MINUS: i32 = 0xBD;

static RUNNING: AtomicBool = AtomicBool::new(true);
static WHEEL: AtomicI32 = AtomicI32::new(0);
static DRAGGING: AtomicBool = AtomicBool::new(false);
static LAST_X: AtomicI32 = AtomicI32::new(0);
static LAST_Y: AtomicI32 = AtomicI32::new(0);
static DRAG_DX: AtomicI32 = AtomicI32::new(0);
static DRAG_DY: AtomicI32 = AtomicI32::new(0);
static KEYS: Mutex<Vec<i32>> = Mutex::new(Vec::new());

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn lparam_xy(l: LPARAM) -> (i32, i32) {
    ((l & 0xFFFF) as u16 as i16 as i32, ((l >> 16) & 0xFFFF) as u16 as i16 as i32)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            RUNNING.store(false, Ordering::SeqCst);
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            RUNNING.store(false, Ordering::SeqCst);
            PostQuitMessage(0);
            0
        }
        WM_PAINT => {
            ValidateRect(hwnd, std::ptr::null());
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) & 0xFFFF) as u16 as i16;
            WHEEL.fetch_add(delta as i32, Ordering::SeqCst);
            0
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
            let (x, y) = lparam_xy(lparam);
            LAST_X.store(x, Ordering::SeqCst);
            LAST_Y.store(y, Ordering::SeqCst);
            DRAGGING.store(true, Ordering::SeqCst);
            SetCapture(hwnd);
            0
        }
        WM_LBUTTONUP | WM_RBUTTONUP => {
            DRAGGING.store(false, Ordering::SeqCst);
            ReleaseCapture();
            0
        }
        WM_MOUSEMOVE => {
            if DRAGGING.load(Ordering::SeqCst) {
                let (x, y) = lparam_xy(lparam);
                DRAG_DX.fetch_add(x - LAST_X.swap(x, Ordering::SeqCst), Ordering::SeqCst);
                DRAG_DY.fetch_add(y - LAST_Y.swap(y, Ordering::SeqCst), Ordering::SeqCst);
            }
            0
        }
        WM_KEYDOWN => {
            // Ignorar la repeticion al mantener una tecla presionada
            if lparam & (1 << 30) == 0 {
                if let Ok(mut k) = KEYS.lock() {
                    k.push(wparam as i32);
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub struct Input {
    pub wheel: i32,
    pub drag_dx: i32,
    pub drag_dy: i32,
    pub pressed: Vec<i32>,
}

pub struct Window {
    hwnd: HWND,
}

impl Window {
    pub fn new(title: &str, width: usize, height: usize) -> Result<Self, String> {
        unsafe {
            SetProcessDPIAware();
            let instance = GetModuleHandleW(std::ptr::null());
            let class_name = wide("DioramaRaytracer");
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: null_mut(),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16),
                hbrBackground: null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };
            if RegisterClassW(&wc) == 0 {
                return Err("RegisterClassW fallo".into());
            }
            let style = WS_OVERLAPPEDWINDOW | WS_VISIBLE;
            let mut rect = RECT { left: 0, top: 0, right: width as i32, bottom: height as i32 };
            AdjustWindowRect(&mut rect, style, 0);
            let title_w = wide(title);
            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title_w.as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            );
            if hwnd.is_null() {
                return Err("CreateWindowExW fallo".into());
            }
            Ok(Window { hwnd })
        }
    }

    /// Procesa los mensajes pendientes. Devuelve false cuando se cierra la ventana.
    pub fn pump(&self) -> bool {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        RUNNING.load(Ordering::SeqCst)
    }

    pub fn take_input(&self) -> Input {
        Input {
            wheel: WHEEL.swap(0, Ordering::SeqCst),
            drag_dx: DRAG_DX.swap(0, Ordering::SeqCst),
            drag_dy: DRAG_DY.swap(0, Ordering::SeqCst),
            pressed: KEYS.lock().map(|mut k| std::mem::take(&mut *k)).unwrap_or_default(),
        }
    }

    /// Tecla mantenida (solo si la ventana tiene el foco).
    pub fn key_down(&self, vk: i32) -> bool {
        unsafe { GetForegroundWindow() == self.hwnd && (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
    }

    pub fn set_title(&self, title: &str) {
        let t = wide(title);
        unsafe {
            SetWindowTextW(self.hwnd, t.as_ptr());
        }
    }

    /// Dibuja la imagen estirada a toda la ventana.
    pub fn present(&self, pixels: &[u32], bw: usize, bh: usize) {
        unsafe {
            let mut rc = RECT::default();
            GetClientRect(self.hwnd, &mut rc);
            let hdc = GetDC(self.hwnd);
            if hdc.is_null() {
                return;
            }
            SetStretchBltMode(hdc, COLORONCOLOR);
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: bw as i32,
                    biHeight: -(bh as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [0],
            };
            StretchDIBits(
                hdc,
                0,
                0,
                rc.right - rc.left,
                rc.bottom - rc.top,
                0,
                0,
                bw as i32,
                bh as i32,
                pixels.as_ptr() as *const c_void,
                &bmi,
                0,
                SRCCOPY,
            );
            ReleaseDC(self.hwnd, hdc);
        }
    }
}
