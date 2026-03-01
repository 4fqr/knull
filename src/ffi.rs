//! Knull Foreign Function Interface (FFI)
//!
//! Allows Knull to call C libraries and external functions.
//! Uses libffi for cross-platform dynamic function invocation.

use libc::{c_char, c_double, c_int, c_void, size_t};
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

// Global library registry
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

// ============================================
// Enhanced FFI - extern "C" and extern "C++"
// ============================================

#[derive(Debug, Clone)]
pub enum CallingConvention {
    C,
    CPlusPlus,
    Raw,
}

#[derive(Debug, Clone)]
pub struct ExternFunction {
    pub name: String,
    pub mangled_name: Option<String>,
    pub return_type: FFIType,
    pub param_types: Vec<FFIType>,
    pub calling_convention: CallingConvention,
    pub is_variadic: bool,
}

impl ExternFunction {
    pub fn c(name: &str, return_type: FFIType, param_types: Vec<FFIType>) -> Self {
        ExternFunction {
            name: name.to_string(),
            mangled_name: None,
            return_type,
            param_types,
            calling_convention: CallingConvention::C,
            is_variadic: false,
        }
    }

    pub fn c_plus_plus(name: &str, return_type: FFIType, param_types: Vec<FFIType>) -> Self {
        let mangled = cpp_name_mangle(name, &param_types);
        ExternFunction {
            name: name.to_string(),
            mangled_name: Some(mangled),
            return_type,
            param_types,
            calling_convention: CallingConvention::CPlusPlus,
            is_variadic: false,
        }
    }

    pub fn variadic(mut self) -> Self {
        self.is_variadic = true;
        self
    }

    pub fn get_symbol_name(&self) -> &str {
        self.mangled_name.as_deref().unwrap_or(&self.name)
    }
}

pub struct ExternBlock {
    pub convention: CallingConvention,
    pub functions: Vec<ExternFunction>,
    pub library_path: Option<String>,
}

impl ExternBlock {
    pub fn c() -> Self {
        ExternBlock {
            convention: CallingConvention::C,
            functions: Vec::new(),
            library_path: None,
        }
    }

    pub fn c_plus_plus() -> Self {
        ExternBlock {
            convention: CallingConvention::CPlusPlus,
            functions: Vec::new(),
            library_path: None,
        }
    }

    pub fn with_library(mut self, path: &str) -> Self {
        self.library_path = Some(path.to_string());
        self
    }

    pub fn add_function(mut self, func: ExternFunction) -> Self {
        self.functions.push(func);
        self
    }
}

fn cpp_name_mangle(name: &str, params: &[FFIType]) -> String {
    let mut mangled = String::from("_Z");
    mangled.push_str(&encode_length(name));
    mangled.push_str(name);

    for param in params {
        mangled.push_str(&match param {
            FFIType::Void => "v".to_string(),
            FFIType::Int => "i".to_string(),
            FFIType::Float => "d".to_string(),
            FFIType::Bool => "b".to_string(),
            FFIType::String => "PKc".to_string(),
            FFIType::Pointer => "Pv".to_string(),
        });
    }

    mangled
}

fn encode_length(s: &str) -> String {
    let len = s.len();
    let mut result = String::new();
    let digits = len.to_string();
    for c in digits.chars() {
        result.push(c);
    }
    result
}

pub struct BindingGenerator {
    extern_blocks: Vec<ExternBlock>,
}

impl BindingGenerator {
    pub fn new() -> Self {
        BindingGenerator {
            extern_blocks: Vec::new(),
        }
    }

    pub fn add_extern_block(&mut self, block: ExternBlock) {
        self.extern_blocks.push(block);
    }

    pub fn generate_knull_bindings(&self) -> String {
        let mut output = String::new();

        for block in &self.extern_blocks {
            let keyword = match block.convention {
                CallingConvention::C => "extern \"C\"",
                CallingConvention::CPlusPlus => "extern \"C++\"",
                CallingConvention::Raw => "extern",
            };

            output.push_str(&format!("{} {{\n", keyword));

            for func in &block.functions {
                output.push_str(&format!("    fn {}(", func.name));

                for (i, param) in func.param_types.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&format!("_: {}", self.ffi_type_to_knull(param)));
                }

                output.push_str(")");

                if let Some(ret) = self.ffi_type_to_knull_opt(&func.return_type) {
                    output.push_str(&format!(" -> {}", ret));
                }

                output.push_str("\n");
            }

            output.push_str("}\n\n");
        }

        output
    }

    fn ffi_type_to_knull(&self, ty: &FFIType) -> &'static str {
        match ty {
            FFIType::Void => "()",
            FFIType::Int => "i64",
            FFIType::Float => "f64",
            FFIType::Bool => "bool",
            FFIType::String => "&str",
            FFIType::Pointer => "usize",
        }
    }

    fn ffi_type_to_knull_opt(&self, ty: &FFIType) -> Option<&'static str> {
        match ty {
            FFIType::Void => None,
            _ => Some(self.ffi_type_to_knull(ty)),
        }
    }

    pub fn auto_generate_bindings(
        header_path: &str,
        convention: CallingConvention,
    ) -> Result<String, String> {
        let source = std::fs::read_to_string(header_path)
            .map_err(|e| format!("Failed to read header: {}", e))?;

        let mut bindings = String::new();
        let keyword = match convention {
            CallingConvention::C => "extern \"C\"",
            CallingConvention::CPlusPlus => "extern \"C++\"",
            CallingConvention::Raw => "extern",
        };

        bindings.push_str(&format!("{} {{\n", keyword));

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("fn ")
                || (!trimmed.starts_with("#include") && trimmed.contains("("))
            {
                if trimmed.contains(";") {
                    bindings.push_str(&format!("    {}\n", trimmed));
                }
            }
        }

        bindings.push_str("}\n");

        Ok(bindings)
    }
}

pub mod c_functions {
    use super::*;

    pub fn printf(format: &str, args: &[FFIValue]) -> i32 {
        println!("[C] printf: {}", format);
        0
    }

    pub fn malloc(size: usize) -> usize {
        unsafe { libc::malloc(size as size_t) as usize }
    }

    pub fn free(ptr: usize) {
        unsafe { libc::free(ptr as *mut c_void) }
    }

    pub fn memcpy(dest: usize, src: usize, n: usize) -> usize {
        unsafe {
            std::ptr::copy_nonoverlapping(src as *const c_void, dest as *mut c_void, n);
        }
        dest
    }

    pub fn strlen(s: *const c_char) -> usize {
        unsafe {
            let mut len = 0;
            while *s.add(len) != 0 {
                len += 1;
            }
            len
        }
    }
}

pub mod cpp_functions {
    use super::*;

    pub fn std__cout__flush() {
        println!("[C++] std::cout::flush()");
    }

    pub fn std__cout__operator_lt_lt(value: i64) {
        print!("{}", value);
    }

    pub fn std__string__constructor() -> usize {
        0
    }

    pub fn std__string__destructor(_ptr: usize) {}
}
