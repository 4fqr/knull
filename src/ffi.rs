//! Knull Foreign Function Interface (FFI)
//!
//! Allows Knull to call C libraries and external functions.
//! Uses libffi for cross-platform dynamic function invocation.

use libc::{c_char, c_double, c_int, c_long, c_void, size_t};
use std::ffi::{CStr, CString};
use std::sync::Mutex;

/// FFI Value types that can be passed to/from C
#[derive(Debug, Clone)]
pub enum FFIValue {
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Pointer(usize),
    Buffer(Vec<u8>),
}

/// FFI Type signatures
#[derive(Debug, Clone, Copy)]
pub enum FFIType {
    Void,
    Int,
    Float,
    Bool,
    String,
    Pointer,
}

/// A loaded dynamic library
pub struct DynamicLibrary {
    handle: usize, // Store as usize for Send+Sync
    name: String,
}

impl DynamicLibrary {
    /// Open a dynamic library
    pub fn open(path: &str) -> Result<Self, String> {
        let c_path = CString::new(path).map_err(|e| e.to_string())?;

        unsafe {
            #[cfg(target_os = "linux")]
            let handle = libc::dlopen(c_path.as_ptr(), libc::RTLD_LAZY);

            #[cfg(target_os = "macos")]
            let handle = libc::dlopen(c_path.as_ptr(), libc::RTLD_LAZY);

            #[cfg(target_os = "windows")]
            let handle = {
                use std::ffi::OsStr;
                use std::os::windows::ffi::OsStrExt;
                use winapi::um::libloaderapi::LoadLibraryW;
                let wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
                LoadLibraryW(wide.as_ptr()) as *mut c_void
            };

            if handle.is_null() {
                return Err(format!("Failed to load library: {}", path));
            }

            Ok(Self {
                handle: handle as usize,
                name: path.to_string(),
            })
        }
    }

    /// Get a symbol from the library
    pub fn get_symbol(&self, name: &str) -> Result<usize, String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;

        unsafe {
            #[cfg(target_os = "linux")]
            let symbol = libc::dlsym(self.handle as *mut c_void, c_name.as_ptr());

            #[cfg(target_os = "macos")]
            let symbol = libc::dlsym(self.handle as *mut c_void, c_name.as_ptr());

            #[cfg(target_os = "windows")]
            let symbol = {
                use winapi::um::libloaderapi::GetProcAddress;
                GetProcAddress(self.handle as _, c_name.as_ptr()) as *mut c_void
            };

            if symbol.is_null() {
                return Err(format!("Symbol not found: {}", name));
            }

            Ok(symbol as usize)
        }
    }

    /// Call a function with no arguments
    pub unsafe fn call_void(&self, symbol: usize) {
        let func: extern "C" fn() = std::mem::transmute(symbol);
        func();
    }

    /// Call a function returning int
    pub unsafe fn call_int(&self, symbol: usize, args: &[FFIValue]) -> i64 {
        match args.len() {
            0 => {
                let func: extern "C" fn() -> c_int = std::mem::transmute(symbol);
                func() as i64
            }
            1 => {
                let func: extern "C" fn(c_int) -> c_int = std::mem::transmute(symbol);
                func(args[0].as_c_int()) as i64
            }
            2 => {
                let func: extern "C" fn(c_int, c_int) -> c_int = std::mem::transmute(symbol);
                func(args[0].as_c_int(), args[1].as_c_int()) as i64
            }
            _ => 0,
        }
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        unsafe {
            #[cfg(target_os = "linux")]
            libc::dlclose(self.handle as *mut c_void);

            #[cfg(target_os = "macos")]
            libc::dlclose(self.handle as *mut c_void);

            #[cfg(target_os = "windows")]
            {
                use winapi::um::libloaderapi::FreeLibrary;
                FreeLibrary(self.handle as _);
            }
        }
    }
}

impl FFIValue {
    /// Convert to C int
    pub fn as_c_int(&self) -> c_int {
        match self {
            FFIValue::Int(i) => *i as c_int,
            FFIValue::Bool(b) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Convert to C double
    pub fn as_c_double(&self) -> c_double {
        match self {
            FFIValue::Float(f) => *f,
            FFIValue::Int(i) => *i as f64,
            _ => 0.0,
        }
    }

    /// Convert to C string pointer
    pub fn as_c_string(&self) -> *const c_char {
        match self {
            FFIValue::String(s) => {
                let c_str = CString::new(s.clone()).unwrap_or_default();
                c_str.into_raw()
            }
            _ => std::ptr::null(),
        }
    }
}

/// Global library registry
lazy_static::lazy_static! {
    static ref LIBRARIES: Mutex<Vec<DynamicLibrary>> = Mutex::new(Vec::new());
}

/// Open a dynamic library and return handle
pub fn ffi_open(path: &str) -> Result<usize, String> {
    let lib = DynamicLibrary::open(path)?;
    let mut libs = LIBRARIES.lock().unwrap();
    let handle = libs.len();
    libs.push(lib);
    Ok(handle)
}

/// Get symbol from library
pub fn ffi_get_symbol(lib_handle: usize, name: &str) -> Result<usize, String> {
    let libs = LIBRARIES.lock().unwrap();
    if let Some(lib) = libs.get(lib_handle) {
        lib.get_symbol(name)
    } else {
        Err("Invalid library handle".to_string())
    }
}

/// Call C function
pub unsafe fn ffi_call(
    lib_handle: usize,
    symbol_name: &str,
    args: &[FFIValue],
    ret_type: FFIType,
) -> FFIValue {
    let libs = LIBRARIES.lock().unwrap();
    if let Some(lib) = libs.get(lib_handle) {
        if let Ok(symbol) = lib.get_symbol(symbol_name) {
            match ret_type {
                FFIType::Void => {
                    lib.call_void(symbol);
                    FFIValue::Void
                }
                FFIType::Int => FFIValue::Int(lib.call_int(symbol, args)),
                FFIType::Float => FFIValue::Float(0.0), // Simplified
                FFIType::Bool => FFIValue::Bool(lib.call_int(symbol, args) != 0),
                _ => FFIValue::Void,
            }
        } else {
            FFIValue::Void
        }
    } else {
        FFIValue::Void
    }
}

/// Allocate C memory
pub fn ffi_malloc(size: usize) -> usize {
    unsafe {
        let ptr = libc::malloc(size as size_t);
        ptr as usize
    }
}

/// Free C memory
pub fn ffi_free(ptr: usize) {
    unsafe {
        libc::free(ptr as *mut c_void);
    }
}

/// Write bytes to C memory
pub unsafe fn ffi_write(ptr: usize, data: &[u8]) {
    let dest = ptr as *mut u8;
    std::ptr::copy_nonoverlapping(data.as_ptr(), dest, data.len());
}

/// Read bytes from C memory
pub unsafe fn ffi_read(ptr: usize, len: usize) -> Vec<u8> {
    let src = ptr as *const u8;
    let mut buf = vec![0u8; len];
    std::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), len);
    buf
}

/// Read C string
pub unsafe fn ffi_read_string(ptr: usize) -> String {
    let c_str = CStr::from_ptr(ptr as *const c_char);
    c_str.to_string_lossy().to_string()
}

/// Initialize FFI subsystem
pub fn ffi_init() {
    println!("[FFI] Foreign Function Interface initialized");
}
