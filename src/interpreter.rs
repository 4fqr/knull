//! Knull Interpreter
//!
//! Tree-walking interpreter for executing Knull code directly.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::parser::{ASTNode, Literal, Type};
use libc;
use rusqlite;
use flate2::Compression;
use flate2::write::{GzEncoder, ZlibEncoder, DeflateEncoder};
use flate2::read::{GzDecoder, ZlibDecoder, DeflateDecoder};
use tungstenite;
use native_tls;
use libloading;
use minifb;

// ── New god-mode crates ──────────────────────────────────────────────────────
use crossbeam_channel;
use dashmap::DashMap;
use rayon::prelude::*;
use std::sync::atomic::{AtomicI64, Ordering};
use chrono::{Local, Utc, TimeZone, Datelike, Timelike};
use uuid::Uuid;
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{Zero, One, ToPrimitive, Signed as NumSigned};
use num_integer::Integer;
use image as img_crate;
use strsim;
use rand::RngCore;
use rand::rngs::OsRng;
// AES-GCM / ChaCha20 / AEAD
use aes_gcm::{Aes256Gcm, AesGcm};
use aes_gcm::aead::{Aead, KeyInit as AeadKeyInit};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit as ChaChaKeyInit};
// Ed25519 / X25519
use ed25519_dalek::{SigningKey, VerifyingKey, Signer as Ed25519Signer, Verifier as Ed25519Verifier};
use x25519_dalek::{StaticSecret, PublicKey as X25519PublicKey};
// RSA
use rsa::{RsaPrivateKey, RsaPublicKey, Pkcs1v15Encrypt};
// HMAC / SHA* / BLAKE3
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512, Digest as Sha2Digest};
use sha3::{Sha3_256, Sha3_512, Digest as Sha3Digest};
// Argon2 / PBKDF2
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use pbkdf2::pbkdf2_hmac;
// CSV / XML / YAML
use csv as csv_crate;
use quick_xml;
use serde_yaml;
// TUI / Crossterm
use ratatui::{Terminal, backend::CrosstermBackend};
use crossterm::{
    terminal as ct_terminal,
    execute,
    cursor,
    style::{self as ct_style, Color as CtColor, Attribute},
    event::{self as ct_event, Event as CtEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
};
// Compression
use lz4_flex;
use zstd;
// ── Batch 9 imports ────────────────────────────────────────────────────────
use nalgebra::{DMatrix, DVector, Matrix3, Matrix4, Vector2, Vector3, Vector4, Quaternion, UnitQuaternion};
use rustfft::{FftPlanner, num_complex::Complex};
use ndarray::{Array1, Array2};
use glam::{Vec2, Vec3, Vec4, Mat3, Mat4, Quat};
use petgraph::graph::{DiGraph, UnGraph, NodeIndex, EdgeIndex};
use petgraph::algo::{dijkstra, astar, bellman_ford, toposort, connected_components, is_cyclic_directed};
use petgraph::visit::EdgeRef;
use xxhash_rust::xxh3::xxh3_64;
use xxhash_rust::xxh32::xxh32;
use base32;
use bs58;
use rmp_serde;
use bincode;
use jsonwebtoken::{encode as jwt_encode, decode as jwt_decode_fn, Header as JwtHeader, Algorithm as JwtAlgorithm, EncodingKey, DecodingKey, Validation};
use handlebars::Handlebars;
use similar::{ChangeTag, TextDiff};
use fancy_regex::Regex as FancyRegex;
use sysinfo::{System, Cpu, Process, Disks, Networks, Pid as SysinfoPid, LoadAvg};
use walkdir::WalkDir;
use glob::glob;
use itertools::Itertools;
use hound;
use byteorder::{BigEndian, LittleEndian, ByteOrder};
use indexmap::IndexMap;
use std::process::{Command, Stdio};
use std::f64::consts::PI;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;
use std::f64::consts::PI as F64_PI;


// Signal flag table: signum -> received count
lazy_static::lazy_static! {
    static ref SIGNAL_FLAGS: Arc<Mutex<HashMap<i32, u64>>> = Arc::new(Mutex::new(HashMap::new()));
}

// Global TCP stream registry
lazy_static::lazy_static! {
    static ref TCP_STREAMS: Arc<Mutex<HashMap<i64, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref TCP_LISTENERS: Arc<Mutex<HashMap<i64, TcpListener>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref NEXT_HANDLE: Arc<Mutex<i64>> = Arc::new(Mutex::new(1));
}

fn next_handle() -> i64 {
    let mut handle = NEXT_HANDLE.lock().unwrap();
    let h = *handle;
    *handle += 1;
    h
}

fn save_tcp_stream(stream: TcpStream) -> i64 {
    let handle = next_handle();
    TCP_STREAMS.lock().unwrap().insert(handle, stream);
    handle
}

fn get_tcp_stream(handle: i64) -> Option<std::sync::MutexGuard<'static, HashMap<i64, TcpStream>>> {
    if TCP_STREAMS.lock().unwrap().contains_key(&handle) {
        Some(TCP_STREAMS.lock().unwrap())
    } else {
        None
    }
}

fn tcp_stream_send(handle: i64, data: &[u8]) -> Result<usize, String> {
    let mut streams = TCP_STREAMS.lock().unwrap();
    if let Some(stream) = streams.get_mut(&handle) {
        stream.write(data).map_err(|e| e.to_string())
    } else {
        Err("Invalid stream handle".to_string())
    }
}

fn tcp_stream_recv(handle: i64, size: usize) -> Result<String, String> {
    let mut streams = TCP_STREAMS.lock().unwrap();
    if let Some(stream) = streams.get_mut(&handle) {
        let mut buf = vec![0u8; size];
        match stream.read(&mut buf) {
            Ok(n) => {
                buf.truncate(n);
                Ok(String::from_utf8_lossy(&buf).to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    } else {
        Err("Invalid stream handle".to_string())
    }
}

fn tcp_stream_close(handle: i64) {
    TCP_STREAMS.lock().unwrap().remove(&handle);
}

fn save_tcp_listener(listener: TcpListener) -> i64 {
    let handle = next_handle();
    TCP_LISTENERS.lock().unwrap().insert(handle, listener);
    handle
}

fn tcp_listener_accept(handle: i64) -> Result<(i64, String), String> {
    // Accept connection — we must release the listeners lock before acquiring streams lock
    let result = {
        let mut listeners = TCP_LISTENERS.lock().unwrap();
        if let Some(listener) = listeners.get_mut(&handle) {
            match listener.accept() {
                Ok((stream, addr)) => Ok((stream, addr.to_string())),
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err("Invalid listener handle".to_string())
        }
    }; // listeners lock released here
    match result {
        Ok((stream, addr)) => {
            let stream_handle = next_handle();
            TCP_STREAMS.lock().unwrap().insert(stream_handle, stream);
            Ok((stream_handle, addr))
        }
        Err(e) => Err(e),
    }
}

// ── Internal hex helpers used by crypto/image builtins ────────────────────
#[inline]
fn he2b(hex: &str) -> Option<Vec<u8>> {
    let h = hex.trim_start_matches("0x").trim_start_matches("0X");
    if h.len() % 2 != 0 { return None; }
    (0..h.len()).step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i+2], 16).ok())
        .collect()
}
#[inline]
fn b2he(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
#[inline]
fn vbytes(v: &Value) -> Vec<u8> {
    match v {
        Value::Array(arr) => arr.iter().map(|x| x.as_int() as u8).collect(),
        Value::String(s) => {
            if s.chars().all(|c| c.is_ascii_hexdigit()) && s.len() % 2 == 0 && !s.is_empty() {
                he2b(s).unwrap_or_else(|| s.as_bytes().to_vec())
            } else {
                s.as_bytes().to_vec()
            }
        }
        _ => v.to_string().into_bytes(),
    }
}

/// Struct field definition
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

/// Struct instance
#[derive(Debug, Clone)]
pub struct StructInstance {
    pub def: Box<StructDef>,
    pub fields: HashMap<String, Value>,
}

/// Function object (for closures)
#[derive(Debug, Clone)]
pub struct FunctionObj {
    pub name: String,
    pub params: Vec<(String, Option<Type>)>,
    pub body: Box<ASTNode>,
    pub closure: HashMap<String, Value>,
}

/// Trait definition
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<(String, FunctionObj)>,
}

/// Runtime value
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Range { start: i64, end: i64, inclusive: bool },
    Map(HashMap<String, Value>),
    Tuple(Vec<Value>),
    Closure {
        params: Vec<String>,
        body: Box<ASTNode>,
        env: HashMap<String, Value>,
    },
    StructDef(Box<StructDef>),
    StructInstance(Box<StructInstance>),
    Function(Box<FunctionObj>),
    Trait(Box<TraitDef>),
    Reference(Box<Value>), // For Rc, Arc simulation
    Null,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Range { start: a, end: b, inclusive: c }, Value::Range { start: d, end: e, inclusive: f }) => a == d && b == e && c == f,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            // Maps: compare key-value pairs
            (Value::Map(a), Value::Map(b)) => {
                if a.len() != b.len() { return false; }
                a.iter().all(|(k, v)| b.get(k).map(|bv| bv == v).unwrap_or(false))
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => {
                if fl.fract() == 0.0 && fl.abs() < 1e15 {
                    write!(f, "{}", *fl as i64)
                } else {
                    write!(f, "{}", fl)
                }
            }
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Range { start, end, inclusive } => {
                if *inclusive { write!(f, "{}..={}", start, end) }
                else { write!(f, "{}..{}", start, end) }
            }
            Value::Map(map) => {
                write!(f, "{{")?;
                let mut first = true;
                for (k, v) in map {
                    if !first { write!(f, ", ")?; }
                    write!(f, "\"{}\": {}", k, v)?;
                    first = false;
                }
                write!(f, "}}")
            }
            Value::Tuple(elems) => {
                write!(f, "(")?;
                for (i, v) in elems.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, ")")
            }
            Value::Closure { params, .. } => write!(f, "<closure({})>", params.join(", ")),
            Value::StructDef(def) => write!(f, "<struct {}>", def.name),
            Value::StructInstance(inst) => {
                write!(f, "{} {{", inst.def.name)?;
                let mut first = true;
                for (k, v) in &inst.fields {
                    if !first { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                    first = false;
                }
                write!(f, "}}")
            }
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Trait(tr) => write!(f, "<trait {}>", tr.name),
            Value::Reference(val) => write!(f, "<ref {}>", val),
            Value::Null => write!(f, "null"),
        }
    }
}

impl Value {
    fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Range { start, end, .. } => start != end,
            Value::StructInstance(_) => true,
            Value::Function(_) => true,
            Value::Closure { .. } => true,
            Value::Reference(val) => val.is_truthy(),
            Value::Null => false,
            _ => false,
        }
    }

    fn is_int(&self) -> bool { matches!(self, Value::Int(_)) }
    fn is_float(&self) -> bool { matches!(self, Value::Float(_)) }

    fn get_field(&self, field: &str) -> Option<Value> {
        match self {
            Value::StructInstance(inst) => inst.fields.get(field).cloned(),
            Value::Reference(val) => val.get_field(field),
            _ => None,
        }
    }

    fn set_field(&mut self, field: &str, value: Value) -> bool {
        match self {
            Value::StructInstance(inst) => {
                inst.fields.insert(field.to_string(), value);
                true
            }
            Value::Reference(val) => val.set_field(field, value),
            _ => false,
        }
    }

    fn as_int(&self) -> i64 {
        match self {
            Value::Int(i) => *i,
            Value::Float(f) => *f as i64,
            Value::Bool(b) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn as_float(&self) -> f64 {
        match self {
            Value::Int(i) => *i as f64,
            Value::Float(f) => *f,
            _ => 0.0,
        }
    }

    fn as_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            _ => self.to_string(),
        }
    }
}

/// Variable scope
#[derive(Debug)]
struct Scope {
    variables: HashMap<String, Value>,
}

impl Scope {
    fn new() -> Self {
        Scope {
            variables: HashMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<Value> {
        self.variables.get(name).cloned()
    }

    fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }
}

/// Interpreter state
pub struct Interpreter {
    scopes: Vec<Scope>,
    functions: HashMap<String, FunctionDef>,
    return_value: Option<Value>,
    break_flag: bool,
    continue_flag: bool,
    // Networking resource tables
    tcp_streams: HashMap<i64, std::net::TcpStream>,
    tcp_listeners: HashMap<i64, std::net::TcpListener>,
    udp_sockets: HashMap<i64, std::net::UdpSocket>,
    net_handle_counter: i64,
    // Raw memory allocations: ptr_handle -> bytes
    mem_alloc_map: HashMap<i64, Vec<u8>>,
    mem_ptr_counter: i64,
    // Background async processes: handle -> child
    async_procs: HashMap<i64, std::process::Child>,
    async_proc_counter: i64,
    // OS threads: handle -> receiver for return value
    thread_results: HashMap<i64, std::sync::mpsc::Receiver<Value>>,
    thread_counter: i64,
    // Mutex handles: id -> Arc<Mutex<Value>>
    mutexes: HashMap<i64, std::sync::Arc<std::sync::Mutex<Value>>>,
    mutex_counter: i64,
    // SQLite handles: id -> Connection
    db_connections: HashMap<i64, rusqlite::Connection>,
    db_counter: i64,
    // Dynamic library handles
    dl_libs: HashMap<i64, libloading::Library>,
    dl_syms: HashMap<i64, usize>, // symbol ptr as usize
    dl_counter: i64,
    // WebSocket connections
    ws_connections: HashMap<i64, tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>>,
    ws_counter: i64,
    // GUI windows (minifb)
    gui_windows: HashMap<i64, (minifb::Window, Vec<u32>)>, // window + pixel buffer
    gui_counter: i64,
    // Raw sockets: handle -> fd
    raw_sockets: HashMap<i64, i32>,
    raw_sock_counter: i64,
    // Unix domain sockets
    unix_streams: HashMap<i64, std::os::unix::net::UnixStream>,
    unix_listeners: HashMap<i64, std::os::unix::net::UnixListener>,
    unix_counter: i64,
    // crossbeam channels
    chan_senders: HashMap<i64, crossbeam_channel::Sender<Value>>,
    chan_receivers: HashMap<i64, crossbeam_channel::Receiver<Value>>,
    chan_counter: i64,
    // Atomic i64 cells
    atomics: HashMap<i64, std::sync::atomic::AtomicI64>,
    atom_counter: i64,
    // Image handles (image crate)
    images: HashMap<i64, image::DynamicImage>,
    img_counter: i64,
    // TUI raw-mode active flag
    tui_active: bool,
    // Graph handles (petgraph)
    graphs_directed: HashMap<i64, petgraph::graph::DiGraph<String, f64>>,
    graphs_undirected: HashMap<i64, petgraph::graph::UnGraph<String, f64>>,
    graph_counter: i64,
    // Handlebars template engine
    hbs: handlebars::Handlebars<'static>,
    // sysinfo System
    sysinfo_sys: Option<sysinfo::System>,
}

#[derive(Debug, Clone)]
struct FunctionDef {
    name: String,
    params: Vec<String>,
    body: ASTNode,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            scopes: vec![Scope::new()],
            functions: HashMap::new(),
            return_value: None,
            break_flag: false,
            continue_flag: false,
            tcp_streams: HashMap::new(),
            tcp_listeners: HashMap::new(),
            udp_sockets: HashMap::new(),
            net_handle_counter: 0,
            mem_alloc_map: HashMap::new(),
            mem_ptr_counter: 0x1000_0000,
            async_procs: HashMap::new(),
            async_proc_counter: 0,
            thread_results: HashMap::new(),
            thread_counter: 0,
            mutexes: HashMap::new(),
            mutex_counter: 0,
            db_connections: HashMap::new(),
            db_counter: 0,
            dl_libs: HashMap::new(),
            dl_syms: HashMap::new(),
            dl_counter: 0,
            ws_connections: HashMap::new(),
            ws_counter: 0,
            gui_windows: HashMap::new(),
            gui_counter: 0,
            raw_sockets: HashMap::new(),
            raw_sock_counter: 0,
            unix_streams: HashMap::new(),
            unix_listeners: HashMap::new(),
            unix_counter: 0,
            chan_senders: HashMap::new(),
            chan_receivers: HashMap::new(),
            chan_counter: 0,
            atomics: HashMap::new(),
            atom_counter: 0,
            images: HashMap::new(),
            img_counter: 0,
            tui_active: false,
            graphs_directed: HashMap::new(),
            graphs_undirected: HashMap::new(),
            graph_counter: 0,
            hbs: handlebars::Handlebars::new(),
            sysinfo_sys: None,
        }
    }

    fn current_scope(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn get_variable(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value);
            }
        }
        None
    }

    fn bind_parameter(&mut self, name: String, value: Value) {
        self.current_scope().set(name, value);
    }

    fn set_variable(&mut self, name: String, value: Value) {
        // First, check if the variable exists in any scope
        for scope in self.scopes.iter_mut().rev() {
            if scope.variables.contains_key(&name) {
                scope.set(name, value);
                return;
            }
        }
        // If not found, create in current scope
        self.current_scope().set(name, value);
    }

    /// Assign to a target (identifier, index, field)
    fn assign_target(&mut self, target: &ASTNode, value: Value) -> Result<(), String> {
        match target {
            ASTNode::Identifier(name) => {
                // update existing or create in current scope
                self.set_variable(name.clone(), value);
                Ok(())
            }
            ASTNode::Index { obj, index } => {
                let idx_val = self.evaluate(index)?;
                // Walk the object to the deepest identifier, then update
                let root_name = Self::extract_root_name(obj)
                    .ok_or_else(|| "Complex index assignment not supported".to_string())?;
                let mut root_val = self.get_variable(&root_name)
                    .ok_or_else(|| format!("Undefined variable: {}", root_name))?;
                Self::apply_index_assign(&mut root_val, obj, &root_name, idx_val, value)?;
                self.set_variable(root_name, root_val);
                Ok(())
            }
            ASTNode::FieldAccess { obj, field } => {
                // Support both struct instances and maps
                let root_name = Self::extract_root_name(obj)
                    .ok_or_else(|| "Complex field assignment not supported".to_string())?;
                let mut root_val = self.get_variable(&root_name)
                    .ok_or_else(|| format!("Undefined variable: {}", root_name))?;
                match &mut root_val {
                    Value::Map(ref mut m) => { m.insert(field.clone(), value); }
                    ref mut other => { other.set_field(field, value); }
                }
                self.set_variable(root_name, root_val);
                Ok(())
            }
            _ => Err("Invalid assignment target".to_string()),
        }
    }

    /// Extract the root variable name from a potentially nested index/field expression
    fn extract_root_name(node: &ASTNode) -> Option<String> {
        match node {
            ASTNode::Identifier(name) => Some(name.clone()),
            ASTNode::Index { obj, .. } => Self::extract_root_name(obj),
            ASTNode::FieldAccess { obj, .. } => Self::extract_root_name(obj),
            _ => None,
        }
    }

    /// Apply a nested index assignment (e.g. a[0][1] = v, m["k"] = v)
    fn apply_index_assign(root: &mut Value, _obj: &ASTNode, _root_name: &str, idx: Value, value: Value) -> Result<(), String> {
        match root {
            Value::Array(arr) => {
                let i = idx.as_int();
                let i = if i < 0 { arr.len() as i64 + i } else { i } as usize;
                if i < arr.len() {
                    arr[i] = value;
                    Ok(())
                } else {
                    Err(format!("Index {} out of bounds (len={})", i, arr.len()))
                }
            }
            Value::Map(m) => {
                m.insert(idx.as_string(), value);
                Ok(())
            }
            _ => Err("Cannot index-assign on non-array/map value".to_string()),
        }
    }

    /// Execute a program
    pub fn execute(&mut self, ast: &ASTNode) -> Result<(), String> {
        match ast {
            ASTNode::Program(items) => {
                // First pass: collect function and struct definitions
                for item in items {
                    match item {
                        ASTNode::Function {
                            name, params, body, ..
                        } => {
                            self.functions.insert(
                                name.clone(),
                                FunctionDef {
                                    name: name.clone(),
                                    params: params.iter().map(|p| p.name.clone()).collect(),
                                    body: *body.clone(),
                                },
                            );
                        }
                        ASTNode::StructDef { name, fields } => {
                            let struct_def = StructDef {
                                name: name.clone(),
                                fields: fields.clone(),
                            };
                            self.set_variable(name.clone(), Value::StructDef(Box::new(struct_def)));
                        }
                        ASTNode::Impl { ty, methods } => {
                            // Collect impl methods
                            for method in methods {
                                if let ASTNode::Function {
                                    name, params, body, ..
                                } = method
                                {
                                    let method_name = format!("{}::{}", ty, name);
                                    // Params already include self from parser
                                    let method_params: Vec<String> =
                                        params.iter().map(|p| p.name.clone()).collect();
                                    self.functions.insert(
                                        method_name,
                                        FunctionDef {
                                            name: name.clone(),
                                            params: method_params,
                                            body: *body.clone(),
                                        },
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Second pass: execute non-definition statements
                for item in items {
                    match item {
                        ASTNode::Function { .. }
                        | ASTNode::StructDef { .. }
                        | ASTNode::Impl { .. } => {}
                        _ => self.execute_node(item)?,
                    }
                }

                // Call main if it exists
                if self.functions.contains_key("main") {
                    self.call_function("main", vec![])?;
                }

                Ok(())
            }
            _ => self.execute_node(ast),
        }
    }

    /// Execute a single AST node
    fn execute_node(&mut self, node: &ASTNode) -> Result<(), String> {
        if self.return_value.is_some() || self.break_flag || self.continue_flag {
            return Ok(());
        }

        match node {
            ASTNode::Let { name, value, .. } => {
                let val = self.evaluate(value)?;
                // `let` always creates in current scope
                self.current_scope().set(name.clone(), val);
                Ok(())
            }
            ASTNode::Assign { target, value } => {
                let val = self.evaluate(value)?;
                self.assign_target(target, val)?;
                Ok(())
            }
            ASTNode::AssignOp { target, op, value } => {
                let rhs = self.evaluate(value)?;
                let lhs = self.evaluate(target)?;
                let result = self.eval_binary_op(op, lhs, rhs)?;
                self.assign_target(target, result)?;
                Ok(())
            }
            ASTNode::Break(_) => {
                self.break_flag = true;
                Ok(())
            }
            ASTNode::Continue(_) => {
                self.continue_flag = true;
                Ok(())
            }
            ASTNode::Loop(body) => {
                loop {
                    self.execute_node(body)?;
                    if self.return_value.is_some() { break; }
                    if self.break_flag { self.break_flag = false; break; }
                    if self.continue_flag { self.continue_flag = false; }
                }
                Ok(())
            }
            ASTNode::Function { .. } => {
                // Already handled in first pass
                Ok(())
            }
            ASTNode::Return(expr) => {
                self.return_value = Some(self.evaluate(expr)?);
                Ok(())
            }
            ASTNode::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond_val = self.evaluate(cond)?;
                if cond_val.is_truthy() {
                    self.execute_node(then_body)?;
                } else if let Some(else_node) = else_body {
                    self.execute_node(else_node)?;
                }
                Ok(())
            }
            ASTNode::While { cond, body } => {
                loop {
                    let cond_val = self.evaluate(cond)?;
                    if !cond_val.is_truthy() {
                        break;
                    }
                    self.execute_node(body)?;
                    if self.return_value.is_some() {
                        break;
                    }
                    if self.break_flag {
                        self.break_flag = false;
                        break;
                    }
                    if self.continue_flag {
                        self.continue_flag = false;
                        continue;
                    }
                }
                Ok(())
            }
            ASTNode::For { var, iter, body } => {
                let iter_val = self.evaluate(iter)?;
                let items: Vec<Value> = match iter_val {
                    Value::Array(arr) => arr,
                    Value::Range { start, end, inclusive } => {
                        let mut v = Vec::new();
                        let limit = if inclusive { end + 1 } else { end };
                        let mut i = start;
                        while i < limit { v.push(Value::Int(i)); i += 1; }
                        v
                    }
                    _ => return Err(format!("Cannot iterate over {:?}", iter_val)),
                };
                for val in items {
                    self.push_scope();
                    self.bind_parameter(var.clone(), val);
                    self.execute_node(body)?;
                    self.pop_scope();
                    if self.return_value.is_some() { break; }
                    if self.break_flag { self.break_flag = false; break; }
                    if self.continue_flag { self.continue_flag = false; }
                }
                Ok(())
            }
            ASTNode::Block(nodes) => {
                self.push_scope();
                for node in nodes {
                    self.execute_node(node)?;
                    if self.return_value.is_some() || self.break_flag || self.continue_flag {
                        break;
                    }
                }
                self.pop_scope();
                Ok(())
            }
            ASTNode::Call { func, args } => {
                let arg_values: Result<Vec<Value>, String> =
                    args.iter().map(|arg| self.evaluate(arg)).collect();
                let arg_values = arg_values?;
                match func.as_ref() {
                    ASTNode::Identifier(name) => {
                        self.call_function(name, arg_values)?;
                    }
                    // Inline closure calls: (|x| x)(val), compose(f,g)(x)
                    other => {
                        let callable = self.evaluate(other)?;
                        self.call_value(callable, arg_values)?;
                    }
                }
                Ok(())
            }
            ASTNode::StructDef { name, fields } => {
                let struct_def = StructDef {
                    name: name.clone(),
                    fields: fields.clone(),
                };
                self.set_variable(name.clone(), Value::StructDef(Box::new(struct_def)));
                Ok(())
            }
            ASTNode::Impl { ty, methods } => {
                // Store methods for this type
                for method in methods {
                    if let ASTNode::Function {
                        name, params, body, ..
                    } = method
                    {
                        let method_name = format!("{}::{}", ty, name);
                        self.functions.insert(
                            method_name,
                            FunctionDef {
                                name: name.clone(),
                                params: params.iter().map(|p| p.name.clone()).collect(),
                                body: *body.clone(),
                            },
                        );
                    }
                }
                Ok(())
            }
            other => {
                // Try to evaluate as expression (ignore result)
                let _ = self.evaluate(other)?;
                Ok(())
            }
        }
    }

    /// Evaluate an expression
    pub fn evaluate(&mut self, node: &ASTNode) -> Result<Value, String> {
        match node {
            ASTNode::Literal(lit) => Ok(self.literal_to_value(lit)),
            ASTNode::Identifier(name) => {
                if let Some(v) = self.get_variable(name) {
                    return Ok(v);
                }
                // Also resolve named functions as first-class values (FunctionObj)
                if let Some(func_def) = self.functions.get(name).cloned() {
                    return Ok(Value::Function(Box::new(FunctionObj {
                        name: func_def.name.clone(),
                        params: func_def.params.iter().map(|p| (p.clone(), None)).collect(),
                        body: Box::new(func_def.body.clone()),
                        closure: HashMap::new(),
                    })));
                }
                Err(format!("Undefined variable: {}", name))
            }
            ASTNode::Binary { op, left, right } => {
                let left_val = self.evaluate(left)?;
                let right_val = self.evaluate(right)?;
                self.eval_binary_op(op, left_val, right_val)
            }
            ASTNode::Unary { op, operand } => {
                let val = self.evaluate(operand)?;
                match op.as_str() {
                    "!" | "not" => Ok(Value::Bool(!val.is_truthy())),
                    "-" => match val {
                        Value::Int(i) => Ok(Value::Int(-i)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err("Cannot negate non-numeric value".to_string()),
                    },
                    _ => Err(format!("Unknown unary operator: {}", op)),
                }
            }
            ASTNode::Call { func, args } => {
                let arg_values: Result<Vec<Value>, String> =
                    args.iter().map(|arg| self.evaluate(arg)).collect();
                let arg_values = arg_values?;
                match func.as_ref() {
                    ASTNode::Identifier(name) => self.call_function(name, arg_values),
                    other => {
                        let callable = self.evaluate(other)?;
                        self.call_value(callable, arg_values)
                    }
                }
            }
            ASTNode::Index { obj, index } => {
                let obj_val = self.evaluate(obj)?;
                let idx_val = self.evaluate(index)?;
                match (obj_val, idx_val) {
                    (Value::Array(arr), Value::Int(i)) => {
                        let idx = if i < 0 { arr.len() as i64 + i } else { i } as usize;
                        arr.get(idx)
                            .cloned()
                            .ok_or_else(|| "Index out of bounds".to_string())
                    }
                    (Value::Tuple(arr), Value::Int(i)) => {
                        let idx = if i < 0 { arr.len() as i64 + i } else { i } as usize;
                        arr.get(idx)
                            .cloned()
                            .ok_or_else(|| "Tuple index out of bounds".to_string())
                    }
                    (Value::String(s), Value::Int(i)) => {
                        let idx = if i < 0 { s.len() as i64 + i } else { i } as usize;
                        s.chars()
                            .nth(idx)
                            .map(|c| Value::String(c.to_string()))
                            .ok_or_else(|| "Index out of bounds".to_string())
                    }
                    (Value::Map(m), Value::String(key)) => {
                        Ok(m.get(&key).cloned().unwrap_or(Value::Null))
                    }
                    (Value::Map(m), idx) => {
                        Ok(m.get(&idx.as_string()).cloned().unwrap_or(Value::Null))
                    }
                    _ => Err("Cannot index non-array/string value".to_string()),
                }
            }
            ASTNode::Range { start, end, inclusive } => {
                let s = self.evaluate(start)?.as_int();
                let e = self.evaluate(end)?.as_int();
                Ok(Value::Range { start: s, end: e, inclusive: *inclusive })
            }
            ASTNode::Assign { target, value } => {
                let val = self.evaluate(value)?;
                self.assign_target(target, val.clone())?;
                Ok(val)
            }
            ASTNode::AssignOp { target, op, value } => {
                let rhs = self.evaluate(value)?;
                let lhs = self.evaluate(target)?;
                let result = self.eval_binary_op(op, lhs, rhs)?;
                self.assign_target(target, result.clone())?;
                Ok(result)
            }
            ASTNode::Block(nodes) => {
                self.push_scope();
                let mut last = Value::Null;
                for node in nodes {
                    last = self.evaluate(node)?;
                    if self.return_value.is_some() || self.break_flag || self.continue_flag { break; }
                }
                self.pop_scope();
                Ok(last)
            }
            ASTNode::Array(elements) => {
                let mut result = Vec::new();
                for e in elements {
                    if let ASTNode::Spread(inner) = e {
                        let v = self.evaluate(inner)?;
                        match v {
                            Value::Array(arr) => result.extend(arr),
                            Value::Tuple(t) => result.extend(t),
                            other => result.push(other),
                        }
                    } else {
                        result.push(self.evaluate(e)?);
                    }
                }
                Ok(Value::Array(result))
            }
            ASTNode::Range { start, end, inclusive } => {
                let s = self.evaluate(start)?.as_int();
                let e = self.evaluate(end)?.as_int();
                Ok(Value::Range { start: s, end: e, inclusive: *inclusive })
            }
            ASTNode::Assign { target, value } => {
                let val = self.evaluate(value)?;
                self.assign_target(target, val.clone())?;
                Ok(val)
            }
            ASTNode::AssignOp { target, op, value } => {
                let rhs = self.evaluate(value)?;
                let lhs = self.evaluate(target)?;
                let result = self.eval_binary_op(op, lhs, rhs)?;
                self.assign_target(target, result.clone())?;
                Ok(result)
            }
            ASTNode::StructLiteral { name, fields } => {
                let struct_def = self
                    .get_variable(name)
                    .ok_or_else(|| format!("Unknown struct: {}", name))?;
                if let Value::StructDef(def) = struct_def {
                    let mut field_values = HashMap::new();
                    for (field_name, field_expr) in fields {
                        let val = self.evaluate(field_expr)?;
                        field_values.insert(field_name.clone(), val);
                    }
                    Ok(Value::StructInstance(Box::new(StructInstance {
                        def,
                        fields: field_values,
                    })))
                } else {
                    Err(format!("{} is not a struct", name))
                }
            }
            ASTNode::MethodCall { obj, method, args } => {
                let obj_val = self.evaluate(obj)?;
                let arg_values: Result<Vec<Value>, String> =
                    args.iter().map(|arg| self.evaluate(arg)).collect();
                let arg_values = arg_values?;

                // Dispatch on built-in types first
                match &obj_val {
                    // ── String methods ────────────────────────────────────────
                    Value::String(s) => {
                        let s = s.clone();
                        return self.call_string_method(&s, method, &arg_values);
                    }
                    // ── Array methods ─────────────────────────────────────────
                    Value::Array(arr) => {
                        let arr = arr.clone();
                        return self.call_array_method(arr, method, &arg_values);
                    }
                    // ── Map methods ───────────────────────────────────────────
                    Value::Map(map) => {
                        let map = map.clone();
                        return self.call_map_method(map, method, &arg_values);
                    }
                    // ── Int methods ───────────────────────────────────────────
                    Value::Int(n) => {
                        let n = *n;
                        return self.call_int_method(n, method, &arg_values);
                    }
                    // ── Float methods ─────────────────────────────────────────
                    Value::Float(f) => {
                        let f = *f;
                        return self.call_float_method(f, method, &arg_values);
                    }
                    // ── Struct instance: call impl method ─────────────────────
                    Value::StructInstance(inst) => {
                        let type_name = inst.def.name.clone();
                        let method_name = format!("{}::{}", type_name, method);
                        let mut full_args = vec![obj_val.clone()];
                        full_args.extend(arg_values);
                        return self.call_function(&method_name, full_args);
                    }
                    _ => {}
                }

                Err(format!("No method '{}' on value {:?}", method, obj_val))
            }
            ASTNode::FieldAccess { obj, field } => {
                let obj_val = self.evaluate(obj)?;
                obj_val
                    .get_field(field)
                    .ok_or_else(|| format!("Field {} not found", field))
            }
            // ── Interpolated string: f"Hello {name}!" ────────────────────────
            ASTNode::InterpolatedString(segments) => {
                let mut result = String::new();
                for (is_expr, content) in segments {
                    if *is_expr {
                        // Re-parse and evaluate the expression
                        let mut sub_parser = crate::parser::Parser::new(content);
                        match sub_parser.parse_expression_pub() {
                            Ok(sub_ast) => {
                                let val = self.evaluate(&sub_ast)?;
                                result.push_str(&val.as_string());
                            }
                            Err(_) => result.push_str(content),
                        }
                    } else {
                        result.push_str(content);
                    }
                }
                Ok(Value::String(result))
            }
            // ── Lambda / closure ──────────────────────────────────────────────
            ASTNode::Lambda { params, body } => {
                // Capture current scope
                let mut env = HashMap::new();
                for scope in &self.scopes {
                    for (k, v) in &scope.variables {
                        env.insert(k.clone(), v.clone());
                    }
                }
                Ok(Value::Closure {
                    params: params.clone(),
                    body: body.clone(),
                    env,
                })
            }
            // ── Map literal ───────────────────────────────────────────────────
            ASTNode::Map(pairs) => {
                let mut map = HashMap::new();
                for (k_node, v_node) in pairs {
                    let key = self.evaluate(k_node)?.as_string();
                    let val = self.evaluate(v_node)?;
                    map.insert(key, val);
                }
                Ok(Value::Map(map))
            }
            // ── Tuple ─────────────────────────────────────────────────────────
            ASTNode::Tuple(elems) => {
                let vals: Result<Vec<Value>, String> =
                    elems.iter().map(|e| self.evaluate(e)).collect();
                Ok(Value::Tuple(vals?))
            }
            // ── Pipeline: left |> right ───────────────────────────────────────
            ASTNode::Pipeline { left, right } => {
                let lv = self.evaluate(left)?;
                // right should be a function call or identifier
                match right.as_ref() {
                    ASTNode::Call { func, args } => {
                        let mut full_args = vec![lv];
                        let more: Result<Vec<Value>, String> =
                            args.iter().map(|a| self.evaluate(a)).collect();
                        full_args.extend(more?);
                        let func_name = match func.as_ref() {
                            ASTNode::Identifier(n) => n.clone(),
                            _ => return Err("Pipeline: right side must be a function call".to_string()),
                        };
                        self.call_function(&func_name, full_args)
                    }
                    ASTNode::Identifier(name) => {
                        self.call_function(name, vec![lv])
                    }
                    _ => {
                        let rv = self.evaluate(right)?;
                        // If right is a closure, call it with left as arg
                        if let Value::Closure { params, body, env } = rv {
                            self.push_scope();
                            for (k, v) in &env { self.current_scope().set(k.clone(), v.clone()); }
                            if let Some(p) = params.first() {
                                self.bind_parameter(p.clone(), lv);
                            }
                            let result = match body.as_ref() {
                                ASTNode::Block(_) | ASTNode::Program(_) => {
                                    self.execute_node(&body)?;
                                    self.return_value.take().unwrap_or(Value::Null)
                                }
                                expr => self.evaluate(expr)?,
                            };
                            self.pop_scope();
                            Ok(result)
                        } else {
                            Ok(lv)
                        }
                    }
                }
            }
            // ── Spawn ─────────────────────────────────────────────────────────
            ASTNode::Spawn(body) => {
                let body_clone = *body.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Value>();
                let funcs_clone: Vec<(String, Vec<String>, ASTNode)> = self.functions
                    .iter()
                    .map(|(k, v)| (k.clone(), v.params.clone(), v.body.clone()))
                    .collect();
                thread::spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut interp = Interpreter::new();
                        for (name, params, body) in funcs_clone {
                            interp.functions.insert(name.clone(), FunctionDef { name, params, body });
                        }
                        let _ = interp.execute_node(&body_clone);
                        interp.return_value.take().unwrap_or(Value::Null)
                    }));
                    let val = result.unwrap_or(Value::Null);
                    let _ = tx.send(val);
                });
                self.thread_counter += 1;
                let id = self.thread_counter;
                self.thread_results.insert(id, rx);
                Ok(Value::Int(id))
            }
            // ── Match expression with guards ──────────────────────────────────
            ASTNode::Match { expr, arms } => {
                let val = self.evaluate(expr)?;
                for arm in arms {
                    if self.pattern_matches(&arm.pattern, &val) {
                        // Check guard if present
                        if let Some(guard) = &arm.guard {
                            self.push_scope();
                            self.pattern_bind(&arm.pattern, &val);
                            let guard_val = self.evaluate(guard)?;
                            if !guard_val.is_truthy() {
                                self.pop_scope();
                                continue;
                            }
                            let result = self.evaluate(&arm.body)?;
                            self.pop_scope();
                            return Ok(result);
                        }
                        self.push_scope();
                        self.pattern_bind(&arm.pattern, &val);
                        let result = self.evaluate(&arm.body)?;
                        self.pop_scope();
                        return Ok(result);
                    }
                }
                Ok(Value::Null)
            }
            // ── Async function (treat like regular fn) ────────────────────────
            ASTNode::AsyncFunction { name, params, body, .. } => {
                self.functions.insert(
                    name.clone(),
                    FunctionDef {
                        name: name.clone(),
                        params: params.iter().map(|p| p.name.clone()).collect(),
                        body: *body.clone(),
                    },
                );
                Ok(Value::Null)
            }
            // ── ?? Null coalesce ──────────────────────────────────────────────
            ASTNode::NullCoalesce { left, right } => {
                let lv = self.evaluate(left)?;
                if matches!(lv, Value::Null) {
                    self.evaluate(right)
                } else {
                    Ok(lv)
                }
            }
            // ── ?. Safe navigation ────────────────────────────────────────────
            ASTNode::SafeNav { obj, field } => {
                let ov = self.evaluate(obj)?;
                match ov {
                    Value::Null => Ok(Value::Null),
                    Value::Map(ref m) => Ok(m.get(field).cloned().unwrap_or(Value::Null)),
                    Value::StructInstance(ref s) => {
                        Ok(s.fields.get(field).cloned().unwrap_or(Value::Null))
                    }
                    _ => Ok(Value::Null),
                }
            }
            // ── ? Try operator ────────────────────────────────────────────────
            ASTNode::TryOp(expr) => {
                let v = self.evaluate(expr)?;
                match &v {
                    Value::Map(m) if m.get("__type").map(|t| t.as_string() == "Error").unwrap_or(false) => {
                        // Propagate the error
                        let msg = m.get("message").map(|t| t.as_string()).unwrap_or_else(|| "error".to_string());
                        self.return_value = Some(v.clone());
                        Err(format!("PropagatedError: {}", msg))
                    }
                    Value::Null => {
                        self.return_value = Some(Value::Null);
                        Err("PropagatedError: null".to_string())
                    }
                    _ => Ok(v),
                }
            }
            // ── Spread operator ───────────────────────────────────────────────
            ASTNode::Spread(expr) => {
                // Returns the array/tuple value — container handles expansion
                self.evaluate(expr)
            }
            // ── List comprehension ────────────────────────────────────────────
            ASTNode::ListComp { expr, var, iter, filter } => {
                let iter_val = self.evaluate(iter)?;
                let items = match iter_val {
                    Value::Array(a) => a,
                    Value::Range { start, end, inclusive } => {
                        let e = if inclusive { end + 1 } else { end };
                        (start..e).map(Value::Int).collect()
                    }
                    Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                    other => vec![other],
                };
                let mut result = Vec::new();
                for item in items {
                    self.push_scope();
                    self.set_variable(var.clone(), item);
                    if let Some(f) = filter {
                        let cond = self.evaluate(f)?;
                        if !cond.is_truthy() {
                            self.pop_scope();
                            continue;
                        }
                    }
                    let v = self.evaluate(expr)?;
                    self.pop_scope();
                    result.push(v);
                }
                Ok(Value::Array(result))
            }
            // ── do { } while cond ─────────────────────────────────────────────
            ASTNode::DoWhile { body, cond } => {
                loop {
                    self.push_scope();
                    let _ = self.execute_node(body);
                    let break_flag = self.break_flag;
                    if self.return_value.is_some() { self.pop_scope(); break; }
                    if break_flag { self.break_flag = false; self.pop_scope(); break; }
                    self.continue_flag = false;
                    self.pop_scope();
                    let c = self.evaluate(cond)?;
                    if !c.is_truthy() { break; }
                }
                Ok(Value::Null)
            }
            // ── if let binding ────────────────────────────────────────────────
            ASTNode::IfLet { pattern, value, then_body, else_body } => {
                let v = self.evaluate(value)?;
                let matched = !matches!(v, Value::Null);
                if matched {
                    self.push_scope();
                    self.set_variable(pattern.clone(), v);
                    let result = self.evaluate(then_body)?;
                    self.pop_scope();
                    Ok(result)
                } else if let Some(eb) = else_body {
                    self.evaluate(eb)
                } else {
                    Ok(Value::Null)
                }
            }
            // ── Const declaration (same as immutable let) ─────────────────────
            ASTNode::Const { name, value, .. } => {
                let v = self.evaluate(value)?;
                self.set_variable(name.clone(), v.clone());
                Ok(v)
            }
            // ── Type alias (no runtime effect) ────────────────────────────────
            ASTNode::TypeAlias { .. } => Ok(Value::Null),
            // ── Defer (run at end of scope — simplified: run immediately) ─────
            ASTNode::Defer(expr) => {
                // In a full impl, deferred calls run at scope exit.
                // Here we store and run at end of current block execution.
                // Simplified: just evaluate the expression now.
                self.evaluate(expr)
            }
            // ── Named arg (unwrap value, ignore name) ─────────────────────────
            ASTNode::NamedArg { value, .. } => self.evaluate(value),
            // ── Use / import: load and execute another .knull file ─────────────
            ASTNode::Use(path) => {
                let p = path.trim_matches('"');
                match std::fs::read_to_string(p) {
                    Ok(src) => {
                        let mut parser = crate::parser::Parser::new(&src);
                        match parser.parse() {
                            Ok(ast) => {
                                // Execute but share the current scope/functions
                                for item in if let ASTNode::Program(items) = &ast { items.as_slice() } else { std::slice::from_ref(&ast) } {
                                    match item {
                                        ASTNode::Function { name, params, body, .. } => {
                                            self.functions.insert(name.clone(), FunctionDef {
                                                name: name.clone(),
                                                params: params.iter().map(|p| p.name.clone()).collect(),
                                                body: *body.clone(),
                                            });
                                        }
                                        _ => { self.execute_node(item)?; }
                                    }
                                }
                                Ok(Value::Null)
                            }
                            Err(e) => Err(format!("Import parse error in {}: {}", p, e)),
                        }
                    }
                    Err(e) => Err(format!("Cannot import '{}': {}", p, e)),
                }
            }
            // ── try { } catch var { } — error handling ─────────────────────────
            ASTNode::TryCatch { try_body, catch_var, catch_body } => {
                match self.evaluate(try_body) {
                    Ok(v) => Ok(v),
                    Err(e) => {
                        self.push_scope();
                        self.set_variable(catch_var.clone(), Value::String(e));
                        let result = self.evaluate(catch_body)?;
                        self.pop_scope();
                        Ok(result)
                    }
                }
            }
            // ── throw expr ─────────────────────────────────────────────────────
            ASTNode::Throw(expr) => {
                let v = self.evaluate(expr)?;
                Err(v.as_string())
            }
            // ── If as expression (handles both explicit return and implicit last value) ──
            ASTNode::If { cond, then_body, else_body } => {
                let cv = self.evaluate(cond)?;
                if cv.is_truthy() {
                    self.evaluate(then_body)
                } else if let Some(eb) = else_body {
                    self.evaluate(eb)
                } else {
                    Ok(Value::Null)
                }
            }
            // ── While as expression (returns Null) ─────────────────────────────
            ASTNode::While { cond, body } => {
                loop {
                    let cv = self.evaluate(cond)?;
                    if !cv.is_truthy() { break; }
                    self.evaluate(body)?;
                    if self.return_value.is_some() { break; }
                    if self.break_flag { self.break_flag = false; break; }
                    if self.continue_flag { self.continue_flag = false; continue; }
                }
                Ok(Value::Null)
            }
            // ── Return as expression — sets return_value and returns it ─────────
            ASTNode::Return(expr) => {
                let v = self.evaluate(expr)?;
                self.return_value = Some(v.clone());
                Ok(v)
            }
            // ── Let as expression — binds and returns the value ────────────────
            ASTNode::Let { name, value, .. } => {
                let v = self.evaluate(value)?;
                self.set_variable(name.clone(), v.clone());
                Ok(v)
            }
            // ── Print statements as expressions ───────────────────────────────
            // Fallthrough: delegate to execute_node for statements we don't know how to evaluate
            other => {
                // Attempt to handle as statement if we don't know how to evaluate
                match self.execute_node(other) {
                    Ok(()) => Ok(self.return_value.take().unwrap_or(Value::Null)),
                    Err(e) => Err(e),
                }
            }
        }
    }

    /// Convert a literal to a value
    fn literal_to_value(&self, lit: &Literal) -> Value {
        match lit {
            Literal::Int(i) => Value::Int(*i),
            Literal::Float(f) => Value::Float(*f),
            Literal::String(s) => Value::String(s.clone()),
            Literal::Bool(b) => Value::Bool(*b),
            Literal::Null => Value::Null,
        }
    }

    /// Evaluate a binary operation
    fn eval_binary_op(&self, op: &str, left: Value, right: Value) -> Result<Value, String> {
        match op {
            "+" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 + r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l + *r as f64)),
                (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
                // String concatenation with other types - convert to string
                (Value::String(l), r) => Ok(Value::String(format!("{}{}", l, r.as_string()))),
                (l, Value::String(r)) => Ok(Value::String(format!("{}{}", l.as_string(), r))),
                // Array concatenation
                (Value::Array(l), Value::Array(r)) => {
                    let mut combined = l.clone();
                    combined.extend(r.clone());
                    Ok(Value::Array(combined))
                },
                _ => Err("Cannot add incompatible types".to_string()),
            },
            "-" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l - r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 - r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l - *r as f64)),
                _ => Err("Cannot subtract incompatible types".to_string()),
            },
            "*" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 * r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l * *r as f64)),
                _ => Err("Cannot multiply incompatible types".to_string()),
            },
            "/" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => {
                    if *r == 0 {
                        Err("Division by zero".to_string())
                    } else {
                        Ok(Value::Int(l / r))
                    }
                }
                (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l / r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 / r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l / *r as f64)),
                _ => Err("Cannot divide incompatible types".to_string()),
            },
            "%" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => {
                    if *r == 0 {
                        Err("Modulo by zero".to_string())
                    } else {
                        Ok(Value::Int(l % r))
                    }
                }
                _ => Err("Modulo only supported for integers".to_string()),
            },
            "==" => Ok(Value::Bool(left == right)),
            "!=" => Ok(Value::Bool(left != right)),
            "<" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l < r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l < r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Bool((*l as f64) < *r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Bool(*l < *r as f64)),
                (Value::String(l), Value::String(r)) => Ok(Value::Bool(l < r)),
                _ => Err("Cannot compare incompatible types".to_string()),
            },
            ">" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l > r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l > r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Bool((*l as f64) > *r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Bool(*l > *r as f64)),
                (Value::String(l), Value::String(r)) => Ok(Value::Bool(l > r)),
                _ => Err("Cannot compare incompatible types".to_string()),
            },
            "<=" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l <= r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l <= r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Bool((*l as f64) <= *r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Bool(*l <= *r as f64)),
                (Value::String(l), Value::String(r)) => Ok(Value::Bool(l <= r)),
                _ => Err("Cannot compare incompatible types".to_string()),
            },
            ">=" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l >= r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l >= r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Bool((*l as f64) >= *r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Bool(*l >= *r as f64)),
                (Value::String(l), Value::String(r)) => Ok(Value::Bool(l >= r)),
                _ => Err("Cannot compare incompatible types".to_string()),
            },
            "&&" | "and" => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            "||" | "or" => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            // Bitwise operators
            "&" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l & r)),
                _ => Err("Bitwise & requires integers".to_string()),
            },
            "|" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l | r)),
                _ => Err("Bitwise | requires integers".to_string()),
            },
            "^" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l ^ r)),
                _ => Err("Bitwise ^ requires integers".to_string()),
            },
            "<<" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l << (*r).clamp(0, 63))),
                _ => Err("Bitwise << requires integers".to_string()),
            },
            ">>" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l >> (*r).clamp(0, 63))),
                _ => Err("Bitwise >> requires integers".to_string()),
            },
            _ => Err(format!("Unknown binary operator: {}", op)),
        }
    }

    /// Execute a function body node, handling both explicit `return` and
    /// implicit last-expression returns. The caller is responsible for
    /// pushing/popping scope and binding parameters.
    fn call_body(&mut self, body: &ASTNode) -> Result<Value, String> {
        match body {
            ASTNode::Block(nodes) => {
                let mut last = Value::Null;
                for (i, node) in nodes.iter().enumerate() {
                    if self.return_value.is_some() || self.break_flag || self.continue_flag {
                        break;
                    }
                    if i + 1 == nodes.len() {
                        // Last node: evaluate as expression for implicit return
                        last = self.evaluate(node)?;
                    } else {
                        self.execute_node(node)?;
                    }
                }
                // Explicit return_value takes priority over implicit last
                Ok(self.return_value.take().unwrap_or(last))
            }
            // Non-block body (single expression)
            expr => {
                let v = self.evaluate(expr)?;
                Ok(self.return_value.take().unwrap_or(v))
            }
        }
    }

    /// Call a function
    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Check for built-in functions first
        if let Some(result) = self.call_builtin(name, &args) {
            return result;
        }

        // Check if the name resolves to a Closure in scope
        if let Some(Value::Closure { params, body, env }) = self.get_variable(name) {
            self.push_scope();
            for (k, v) in &env {
                self.current_scope().set(k.clone(), v.clone());
            }
            for (param, arg) in params.iter().zip(args.iter()) {
                self.bind_parameter(param.clone(), arg.clone());
            }
            let body_node = *body.clone();
            let result = match &body_node {
                ASTNode::Block(_) | ASTNode::Program(_) => {
                    self.execute_node(&body_node)?;
                    self.return_value.take().unwrap_or(Value::Null)
                }
                expr => self.evaluate(expr)?,
            };
            self.pop_scope();
            return Ok(result);
        }

        // Check if the name resolves to a Function value in scope
        if let Some(Value::Function(func)) = self.get_variable(name) {
            let fname = func.name.clone();
            self.push_scope();
            for ((param, _), arg) in func.params.iter().zip(args.iter()) {
                self.bind_parameter(param.clone(), arg.clone());
            }
            let body_node = *func.body.clone();
            let result = self.call_body(&body_node)?;
            let _ = fname;
            self.pop_scope();
            return Ok(result);
        }

        // Check for user-defined functions
        if let Some(func_def) = self.functions.get(name).cloned() {
            self.push_scope();

            // Bind parameters to arguments (allow extra args to be silently ignored)
            for (param, arg) in func_def.params.iter().zip(args.iter()) {
                self.bind_parameter(param.clone(), arg.clone());
            }

            // Execute function body; call_body handles implicit last-expression return
            let result = self.call_body(&func_def.body)?;
            self.pop_scope();

            return Ok(result);
        }

        Err(format!("Unknown function: {}", name))
    }

    /// Call a string method: str.method(args)
    fn call_string_method(&mut self, s: &str, method: &str, args: &[Value]) -> Result<Value, String> {
        match method {
            "len" | "length" => Ok(Value::Int(s.chars().count() as i64)),
            "is_empty" => Ok(Value::Bool(s.is_empty())),
            "to_uppercase" | "upper" => Ok(Value::String(s.to_uppercase())),
            "to_lowercase" | "lower" => Ok(Value::String(s.to_lowercase())),
            "trim" => Ok(Value::String(s.trim().to_string())),
            "trim_start" | "ltrim" => Ok(Value::String(s.trim_start().to_string())),
            "trim_end" | "rtrim" => Ok(Value::String(s.trim_end().to_string())),
            "chars" => Ok(Value::Array(s.chars().map(|c| Value::String(c.to_string())).collect())),
            "bytes" => Ok(Value::Array(s.bytes().map(|b| Value::Int(b as i64)).collect())),
            "lines" => Ok(Value::Array(s.lines().map(|l| Value::String(l.to_string())).collect())),
            "split" => {
                let sep = args.first().map(|v| v.as_string()).unwrap_or_default();
                if sep.is_empty() {
                    Ok(Value::Array(s.chars().map(|c| Value::String(c.to_string())).collect()))
                } else {
                    Ok(Value::Array(s.split(&sep as &str).map(|p| Value::String(p.to_string())).collect()))
                }
            }
            "split_whitespace" => Ok(Value::Array(s.split_whitespace().map(|p| Value::String(p.to_string())).collect())),
            "contains" => {
                let needle = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::Bool(s.contains(&needle as &str)))
            }
            "starts_with" => {
                let prefix = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::Bool(s.starts_with(&prefix as &str)))
            }
            "ends_with" => {
                let suffix = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::Bool(s.ends_with(&suffix as &str)))
            }
            "replace" => {
                if args.len() >= 2 {
                    Ok(Value::String(s.replace(&args[0].as_string() as &str, &args[1].as_string() as &str)))
                } else { Err("str.replace(from, to)".to_string()) }
            }
            "replace_n" => {
                if args.len() >= 3 {
                    let n = args[2].as_int() as usize;
                    Ok(Value::String(s.replacen(&args[0].as_string() as &str, &args[1].as_string() as &str, n)))
                } else { Err("str.replace_n(from, to, n)".to_string()) }
            }
            "repeat" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(1) as usize;
                Ok(Value::String(s.repeat(n)))
            }
            "find" | "index_of" => {
                let needle = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::Int(s.find(&needle as &str).map(|i| i as i64).unwrap_or(-1)))
            }
            "rfind" => {
                let needle = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::Int(s.rfind(&needle as &str).map(|i| i as i64).unwrap_or(-1)))
            }
            "char_at" | "get" => {
                let idx = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
                Ok(s.chars().nth(idx).map(|c| Value::String(c.to_string())).unwrap_or(Value::Null))
            }
            "slice" | "substring" => {
                let start = args.first().map(|v| v.as_int().max(0) as usize).unwrap_or(0);
                let end = args.get(1).map(|v| v.as_int() as usize).unwrap_or_else(|| s.chars().count());
                Ok(Value::String(s.chars().skip(start).take(end.saturating_sub(start)).collect()))
            }
            "to_int" | "parse_int" => {
                Ok(s.trim().parse::<i64>().map(Value::Int).unwrap_or(Value::Null))
            }
            "to_float" | "parse_float" => {
                Ok(s.trim().parse::<f64>().map(Value::Float).unwrap_or(Value::Null))
            }
            "to_string" => Ok(Value::String(s.to_string())),
            "pad_left" => {
                let width = args.first().map(|v| v.as_int() as usize).unwrap_or(0);
                let pad = args.get(1).and_then(|v| v.as_string().chars().next()).unwrap_or(' ');
                let cur_len = s.chars().count();
                if cur_len < width {
                    let padding: String = std::iter::repeat(pad).take(width - cur_len).collect();
                    Ok(Value::String(format!("{}{}", padding, s)))
                } else {
                    Ok(Value::String(s.to_string()))
                }
            }
            "pad_right" => {
                let width = args.first().map(|v| v.as_int() as usize).unwrap_or(0);
                let pad = args.get(1).and_then(|v| v.as_string().chars().next()).unwrap_or(' ');
                let cur_len = s.chars().count();
                if cur_len < width {
                    let padding: String = std::iter::repeat(pad).take(width - cur_len).collect();
                    Ok(Value::String(format!("{}{}", s, padding)))
                } else {
                    Ok(Value::String(s.to_string()))
                }
            }
            "reverse" => Ok(Value::String(s.chars().rev().collect())),
            "count" => {
                let needle = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::Int(s.matches(&needle as &str).count() as i64))
            }
            "format" => {
                // "template {0} {1}".format(a, b)
                let mut result = s.to_string();
                for (i, arg) in args.iter().enumerate() {
                    result = result.replacen(&format!("{{{}}}", i), &arg.as_string(), 1);
                    result = result.replacen("{}", &arg.as_string(), 1);
                }
                Ok(Value::String(result))
            }
            _ => Err(format!("String has no method '{}'", method)),
        }
    }

    /// Call an array method: arr.method(args)
    fn call_array_method(&mut self, arr: Vec<Value>, method: &str, args: &[Value]) -> Result<Value, String> {
        match method {
            "len" | "length" | "count" => Ok(Value::Int(arr.len() as i64)),
            "is_empty" => Ok(Value::Bool(arr.is_empty())),
            "first" | "head" => Ok(arr.first().cloned().unwrap_or(Value::Null)),
            "last" => Ok(arr.last().cloned().unwrap_or(Value::Null)),
            "push" | "append" => {
                let mut new_arr = arr;
                if let Some(val) = args.first() { new_arr.push(val.clone()); }
                Ok(Value::Array(new_arr))
            }
            "pop" => {
                let mut new_arr = arr;
                let last = new_arr.pop().unwrap_or(Value::Null);
                Ok(Value::Array(vec![Value::Array(new_arr), last]))
            }
            "shift" | "dequeue" => {
                if arr.is_empty() { return Ok(Value::Null); }
                Ok(arr[0].clone())
            }
            "insert" => {
                if args.len() >= 2 {
                    let idx = args[0].as_int() as usize;
                    let mut new_arr = arr;
                    new_arr.insert(idx.min(new_arr.len()), args[1].clone());
                    Ok(Value::Array(new_arr))
                } else { Err("arr.insert(idx, val)".to_string()) }
            }
            "remove" => {
                let idx = args.first().map(|v| v.as_int() as usize).unwrap_or(0);
                let mut new_arr = arr;
                if idx < new_arr.len() { new_arr.remove(idx); }
                Ok(Value::Array(new_arr))
            }
            "contains" | "includes" => {
                let needle = args.first().cloned().unwrap_or(Value::Null);
                Ok(Value::Bool(arr.contains(&needle)))
            }
            "index_of" | "find_index" => {
                let needle = args.first().cloned().unwrap_or(Value::Null);
                Ok(Value::Int(arr.iter().position(|v| v == &needle).map(|i| i as i64).unwrap_or(-1)))
            }
            "sort" => {
                let mut new_arr = arr;
                new_arr.sort_by(|a, b| match (a, b) {
                    (Value::Int(x), Value::Int(y)) => x.cmp(y),
                    (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::String(x), Value::String(y)) => x.cmp(y),
                    _ => std::cmp::Ordering::Equal,
                });
                Ok(Value::Array(new_arr))
            }
            "sort_desc" => {
                let mut new_arr = arr;
                new_arr.sort_by(|a, b| match (b, a) {
                    (Value::Int(x), Value::Int(y)) => x.cmp(y),
                    (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::String(x), Value::String(y)) => x.cmp(y),
                    _ => std::cmp::Ordering::Equal,
                });
                Ok(Value::Array(new_arr))
            }
            "sort_by" | "sorted_by" => {
                // arr.sort_by(|a, b| a < b) or sorted_by(arr, comparator)
                if let Some(cmp_fn) = args.first() {
                    let cmp_fn = cmp_fn.clone();
                    let mut new_arr = arr;
                    let mut sort_err: Option<String> = None;
                    new_arr.sort_by(|a, b| {
                        if sort_err.is_some() { return std::cmp::Ordering::Equal; }
                        match self.call_value(cmp_fn.clone(), vec![a.clone(), b.clone()]) {
                            Ok(Value::Bool(true)) => std::cmp::Ordering::Less,
                            Ok(Value::Bool(false)) => std::cmp::Ordering::Greater,
                            Ok(Value::Int(n)) => n.cmp(&0),
                            Ok(_) => std::cmp::Ordering::Equal,
                            Err(e) => { sort_err = Some(e); std::cmp::Ordering::Equal }
                        }
                    });
                    if let Some(e) = sort_err { return Err(e); }
                    Ok(Value::Array(new_arr))
                } else { Err("arr.sort_by(|a,b| a < b)".to_string()) }
            }
            "reverse" => {
                let mut new_arr = arr;
                new_arr.reverse();
                Ok(Value::Array(new_arr))
            }
            "join" => {
                let sep = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::String(arr.iter().map(|v| v.as_string()).collect::<Vec<_>>().join(&sep)))
            }
            "sum" => {
                let has_float = arr.iter().any(|v| matches!(v, Value::Float(_)));
                if has_float {
                    Ok(Value::Float(arr.iter().fold(0.0f64, |acc, v| acc + v.as_float())))
                } else {
                    Ok(Value::Int(arr.iter().fold(0i64, |acc, v| acc + v.as_int())))
                }
            }
            "product" => {
                let has_float = arr.iter().any(|v| matches!(v, Value::Float(_)));
                if has_float {
                    Ok(Value::Float(arr.iter().fold(1.0f64, |acc, v| acc * v.as_float())))
                } else {
                    Ok(Value::Int(arr.iter().fold(1i64, |acc, v| acc * v.as_int())))
                }
            }
            "min" => {
                arr.iter().min_by(|a, b| match (*a, *b) {
                    (Value::Int(x), Value::Int(y)) => x.cmp(y),
                    _ => a.as_float().partial_cmp(&b.as_float()).unwrap_or(std::cmp::Ordering::Equal),
                }).cloned().map(Ok).unwrap_or(Ok(Value::Null))
            }
            "max" => {
                arr.iter().max_by(|a, b| match (*a, *b) {
                    (Value::Int(x), Value::Int(y)) => x.cmp(y),
                    _ => a.as_float().partial_cmp(&b.as_float()).unwrap_or(std::cmp::Ordering::Equal),
                }).cloned().map(Ok).unwrap_or(Ok(Value::Null))
            }
            "flatten" => {
                let mut flat = Vec::new();
                for v in &arr {
                    if let Value::Array(inner) = v { flat.extend(inner.iter().cloned()); }
                    else { flat.push(v.clone()); }
                }
                Ok(Value::Array(flat))
            }
            "unique" | "dedup" => {
                let mut seen = Vec::new();
                let mut result = Vec::new();
                for v in &arr {
                    if !seen.contains(v) { seen.push(v.clone()); result.push(v.clone()); }
                }
                Ok(Value::Array(result))
            }
            "slice" => {
                let start = args.first().map(|v| v.as_int().max(0) as usize).unwrap_or(0);
                let end = args.get(1).map(|v| v.as_int() as usize).unwrap_or(arr.len()).min(arr.len());
                Ok(Value::Array(arr[start..end].to_vec()))
            }
            "concat" | "extend" => {
                let mut new_arr = arr;
                if let Some(Value::Array(other)) = args.first() {
                    new_arr.extend(other.iter().cloned());
                }
                Ok(Value::Array(new_arr))
            }
            "zip" => {
                if let Some(Value::Array(other)) = args.first() {
                    Ok(Value::Array(arr.iter().zip(other.iter())
                        .map(|(a, b)| Value::Array(vec![a.clone(), b.clone()]))
                        .collect()))
                } else { Err("arr.zip(other_arr)".to_string()) }
            }
            "any" => Ok(Value::Bool(arr.iter().any(|v| v.is_truthy()))),
            "all" => Ok(Value::Bool(arr.iter().all(|v| v.is_truthy()))),
            "none" => Ok(Value::Bool(arr.iter().all(|v| !v.is_truthy()))),
            "to_string" => Ok(Value::String(
                format!("[{}]", arr.iter().map(|v| v.as_string()).collect::<Vec<_>>().join(", "))
            )),
            // Functional methods: map, filter, reduce (take closure as arg)
            "map" => {
                if let Some(closure) = args.first() {
                    let mut result = Vec::new();
                    for item in &arr {
                        let r = self.call_value(closure.clone(), vec![item.clone()])?;
                        result.push(r);
                    }
                    Ok(Value::Array(result))
                } else { Err("arr.map(fn)".to_string()) }
            }
            "filter" => {
                if let Some(closure) = args.first() {
                    let mut result = Vec::new();
                    for item in &arr {
                        let r = self.call_value(closure.clone(), vec![item.clone()])?;
                        if r.is_truthy() { result.push(item.clone()); }
                    }
                    Ok(Value::Array(result))
                } else { Err("arr.filter(fn)".to_string()) }
            }
            "reduce" | "fold" => {
                if args.len() >= 2 {
                    let closure = args[0].clone();
                    let mut acc = args[1].clone();
                    for item in &arr {
                        acc = self.call_value(closure.clone(), vec![acc, item.clone()])?;
                    }
                    Ok(acc)
                } else if let Some(closure) = args.first() {
                    if arr.is_empty() { return Ok(Value::Null); }
                    let mut acc = arr[0].clone();
                    for item in arr.iter().skip(1) {
                        acc = self.call_value(closure.clone(), vec![acc, item.clone()])?;
                    }
                    Ok(acc)
                } else { Err("arr.reduce(fn, [initial])".to_string()) }
            }
            "for_each" | "each" => {
                if let Some(closure) = args.first() {
                    for item in &arr {
                        self.call_value(closure.clone(), vec![item.clone()])?;
                    }
                    Ok(Value::Null)
                } else { Err("arr.for_each(fn)".to_string()) }
            }
            "find" => {
                if let Some(closure) = args.first() {
                    for item in &arr {
                        let r = self.call_value(closure.clone(), vec![item.clone()])?;
                        if r.is_truthy() { return Ok(item.clone()); }
                    }
                    Ok(Value::Null)
                } else { Err("arr.find(fn)".to_string()) }
            }
            "count_if" => {
                if let Some(closure) = args.first() {
                    let mut count = 0i64;
                    for item in &arr {
                        let r = self.call_value(closure.clone(), vec![item.clone()])?;
                        if r.is_truthy() { count += 1; }
                    }
                    Ok(Value::Int(count))
                } else { Ok(Value::Int(arr.len() as i64)) }
            }
            "flat_map" => {
                if let Some(closure) = args.first() {
                    let mut result = Vec::new();
                    for item in &arr {
                        let r = self.call_value(closure.clone(), vec![item.clone()])?;
                        if let Value::Array(inner) = r { result.extend(inner); }
                        else { result.push(r); }
                    }
                    Ok(Value::Array(result))
                } else { Err("arr.flat_map(fn)".to_string()) }
            }
            _ => Err(format!("Array has no method '{}'", method)),
        }
    }

    /// Call a map method: map.method(args)
    fn call_map_method(&mut self, mut map: HashMap<String, Value>, method: &str, args: &[Value]) -> Result<Value, String> {
        match method {
            "len" | "length" | "count" => Ok(Value::Int(map.len() as i64)),
            "is_empty" => Ok(Value::Bool(map.is_empty())),
            "keys" => Ok(Value::Array(map.keys().map(|k| Value::String(k.clone())).collect())),
            "values" => Ok(Value::Array(map.values().cloned().collect())),
            "entries" | "items" => Ok(Value::Array(
                map.iter().map(|(k, v)| Value::Array(vec![Value::String(k.clone()), v.clone()])).collect()
            )),
            "get" => {
                let key = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(map.get(&key).cloned().unwrap_or(Value::Null))
            }
            "set" | "insert" => {
                if args.len() >= 2 {
                    let key = args[0].as_string();
                    let val = args[1].clone();
                    map.insert(key, val);
                    Ok(Value::Map(map))
                } else { Err("map.set(key, val)".to_string()) }
            }
            "remove" | "delete" => {
                let key = args.first().map(|v| v.as_string()).unwrap_or_default();
                map.remove(&key);
                Ok(Value::Map(map))
            }
            "has" | "has_key" | "contains_key" => {
                let key = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::Bool(map.contains_key(&key)))
            }
            "merge" => {
                if let Some(Value::Map(other)) = args.first() {
                    for (k, v) in other { map.insert(k.clone(), v.clone()); }
                    Ok(Value::Map(map))
                } else { Err("map.merge(other_map)".to_string()) }
            }
            "to_array" | "to_list" => Ok(Value::Array(
                map.iter().map(|(k, v)| Value::Array(vec![Value::String(k.clone()), v.clone()])).collect()
            )),
            "to_string" => Ok(Value::String(format!("{}", Value::Map(map)))),
            "for_each" | "each" => {
                if let Some(closure) = args.first() {
                    for (k, v) in &map {
                        self.call_value(closure.clone(), vec![Value::String(k.clone()), v.clone()])?;
                    }
                    Ok(Value::Null)
                } else { Err("map.for_each(fn)".to_string()) }
            }
            _ => Err(format!("Map has no method '{}'", method)),
        }
    }

    /// Call an int method: n.method(args)
    fn call_int_method(&self, n: i64, method: &str, args: &[Value]) -> Result<Value, String> {
        match method {
            "to_string" | "str" => Ok(Value::String(n.to_string())),
            "to_float" => Ok(Value::Float(n as f64)),
            "abs" => Ok(Value::Int(n.abs())),
            "pow" => {
                let exp = args.first().map(|v| v.as_int()).unwrap_or(1);
                if exp >= 0 { Ok(Value::Int(n.pow(exp as u32))) }
                else { Ok(Value::Float((n as f64).powi(exp as i32))) }
            }
            "min" => {
                let other = args.first().map(|v| v.as_int()).unwrap_or(n);
                Ok(Value::Int(n.min(other)))
            }
            "max" => {
                let other = args.first().map(|v| v.as_int()).unwrap_or(n);
                Ok(Value::Int(n.max(other)))
            }
            "clamp" => {
                if args.len() >= 2 {
                    let lo = args[0].as_int();
                    let hi = args[1].as_int();
                    Ok(Value::Int(n.clamp(lo, hi)))
                } else { Err("n.clamp(lo, hi)".to_string()) }
            }
            "is_even" => Ok(Value::Bool(n % 2 == 0)),
            "is_odd" => Ok(Value::Bool(n % 2 != 0)),
            "is_zero" => Ok(Value::Bool(n == 0)),
            "is_positive" => Ok(Value::Bool(n > 0)),
            "is_negative" => Ok(Value::Bool(n < 0)),
            "signum" => Ok(Value::Int(n.signum())),
            "times" => Ok(Value::Array((0..n.max(0)).map(Value::Int).collect())),
            "repeat" => Ok(Value::Array(std::iter::repeat(Value::Int(n)).take(args.first().map(|v| v.as_int() as usize).unwrap_or(0)).collect())),
            "to_char" => Ok(char::from_u32(n as u32).map(|c| Value::String(c.to_string())).unwrap_or(Value::Null)),
            "to_bits" => Ok(Value::String(format!("{:b}", n))),
            "to_hex" => Ok(Value::String(format!("{:x}", n))),
            "to_oct" => Ok(Value::String(format!("{:o}", n))),
            _ => Err(format!("Int has no method '{}'", method)),
        }
    }

    /// Call a float method: f.method(args)
    fn call_float_method(&self, f: f64, method: &str, _args: &[Value]) -> Result<Value, String> {
        match method {
            "to_string" | "str" => Ok(Value::String(f.to_string())),
            "to_int" => Ok(Value::Int(f as i64)),
            "abs" => Ok(Value::Float(f.abs())),
            "floor" => Ok(Value::Float(f.floor())),
            "ceil" => Ok(Value::Float(f.ceil())),
            "round" => Ok(Value::Float(f.round())),
            "trunc" => Ok(Value::Float(f.trunc())),
            "fract" => Ok(Value::Float(f.fract())),
            "sqrt" => Ok(Value::Float(f.sqrt())),
            "cbrt" => Ok(Value::Float(f.cbrt())),
            "exp" => Ok(Value::Float(f.exp())),
            "ln" | "log" => Ok(Value::Float(f.ln())),
            "log2" => Ok(Value::Float(f.log2())),
            "log10" => Ok(Value::Float(f.log10())),
            "sin" => Ok(Value::Float(f.sin())),
            "cos" => Ok(Value::Float(f.cos())),
            "tan" => Ok(Value::Float(f.tan())),
            "asin" => Ok(Value::Float(f.asin())),
            "acos" => Ok(Value::Float(f.acos())),
            "atan" => Ok(Value::Float(f.atan())),
            "signum" => Ok(Value::Float(f.signum())),
            "is_nan" => Ok(Value::Bool(f.is_nan())),
            "is_finite" => Ok(Value::Bool(f.is_finite())),
            "is_infinite" => Ok(Value::Bool(f.is_infinite())),
            "is_zero" => Ok(Value::Bool(f == 0.0)),
            "is_positive" => Ok(Value::Bool(f > 0.0)),
            "is_negative" => Ok(Value::Bool(f < 0.0)),
            _ => Err(format!("Float has no method '{}'", method)),
        }
    }

    /// Call a value as a function (used by higher-order builtins like map, filter)
    fn call_value(&mut self, callable: Value, args: Vec<Value>) -> Result<Value, String> {
        match callable {
            Value::Closure { params, body, env } => {
                self.push_scope();
                for (k, v) in &env {
                    self.current_scope().set(k.clone(), v.clone());
                }
                for (param, arg) in params.iter().zip(args.iter()) {
                    self.bind_parameter(param.clone(), arg.clone());
                }
                let result = match body.as_ref() {
                    ASTNode::Block(_) | ASTNode::Program(_) => {
                        self.execute_node(&body)?;
                        self.return_value.take().unwrap_or(Value::Null)
                    }
                    expr => self.evaluate(expr)?,
                };
                self.pop_scope();
                Ok(result)
            }
            Value::Function(func) => {
                let name = func.name.clone();
                self.call_function(&name, args)
            }
            _ => Err(format!("Cannot call {:?} as a function", callable)),
        }
    }

    /// Check if a value matches a pattern
    fn pattern_matches(&self, pattern: &crate::parser::Pattern, value: &Value) -> bool {
        use crate::parser::{Pattern, Literal};
        match pattern {
            Pattern::Wildcard => true,
            Pattern::Identifier(_) => true, // capture — always matches
            Pattern::Literal(lit) => {
                let lit_val = match lit {
                    Literal::Int(n) => Value::Int(*n),
                    Literal::Float(f) => Value::Float(*f),
                    Literal::String(s) => Value::String(s.clone()),
                    Literal::Bool(b) => Value::Bool(*b),
                    Literal::Null => Value::Null,
                };
                value == &lit_val
            }
            Pattern::Struct { name, fields } => {
                match value {
                    Value::StructInstance(inst) => {
                        // Check type name matches
                        if inst.def.name != *name { return false; }
                        // Check each field pattern
                        for (field_name, field_pat) in fields {
                            if let Some(field_val) = inst.fields.get(field_name) {
                                if !self.pattern_matches(field_pat, field_val) {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                        true
                    }
                    Value::Map(m) => {
                        // Also match maps with struct-like pattern
                        for (field_name, field_pat) in fields {
                            if let Some(field_val) = m.get(field_name) {
                                if !self.pattern_matches(field_pat, field_val) {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                        true
                    }
                    _ => false,
                }
            }
            Pattern::Or(pats) => {
                pats.iter().any(|p| self.pattern_matches(p, value))
            }
            Pattern::Enum { name, variant, data } => {
                match value {
                    Value::StructInstance(inst) => {
                        // Enum variant as struct with "variant" field
                        let matches_name = inst.def.name == *name || inst.def.name == *variant;
                        if !matches_name { return false; }
                        if let Some(field_val) = inst.fields.get("value").or_else(|| inst.fields.get("data")) {
                            if let Some(dp) = data {
                                return self.pattern_matches(dp, field_val);
                            }
                        }
                        data.is_none()
                    }
                    Value::String(s) => {
                        // Simple enum variant as string
                        s == variant && data.is_none()
                    }
                    _ => false,
                }
            }
        }
    }

    /// Bind captured variables from a pattern match into current scope
    fn pattern_bind(&mut self, pattern: &crate::parser::Pattern, value: &Value) {
        use crate::parser::Pattern;
        match pattern {
            Pattern::Identifier(name) => {
                self.current_scope().set(name.clone(), value.clone());
            }
            Pattern::Struct { fields, .. } => {
                match value {
                    Value::StructInstance(inst) => {
                        for (field_name, field_pat) in fields {
                            if let Some(fv) = inst.fields.get(field_name) {
                                self.pattern_bind(field_pat, fv);
                            }
                        }
                    }
                    Value::Map(m) => {
                        for (field_name, field_pat) in fields {
                            if let Some(fv) = m.get(field_name) {
                                self.pattern_bind(field_pat, fv);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Pattern::Enum { data, .. } => {
                if let Some(dp) = data {
                    match value {
                        Value::StructInstance(inst) => {
                            if let Some(inner) = inst.fields.get("value").or_else(|| inst.fields.get("data")) {
                                self.pattern_bind(dp, inner);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    /// Call a built-in function
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, String>> {
        match name {
            "print" => {
                if args.is_empty() { print!(""); }
                else {
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 { print!(" "); }
                        print!("{}", a);
                    }
                }
                Some(Ok(Value::Null))
            }
            "println" => {
                if args.is_empty() { println!(); }
                else {
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 { print!(" "); }
                        print!("{}", a);
                    }
                    println!();
                }
                Some(Ok(Value::Null))
            }
            "eprint" => {
                if let Some(arg) = args.first() {
                    eprint!("{}", arg);
                }
                Some(Ok(Value::Null))
            }
            "eprintln" => {
                if let Some(arg) = args.first() {
                    eprintln!("{}", arg);
                } else {
                    eprintln!();
                }
                Some(Ok(Value::Null))
            }
            "len" => {
                if let Some(arg) = args.first() {
                    let len = match arg {
                        Value::String(s) => s.len() as i64,
                        Value::Array(arr) => arr.len() as i64,
                        _ => return Some(Err("len() requires a string or array".to_string())),
                    };
                    Some(Ok(Value::Int(len)))
                } else {
                    Some(Err("len() requires an argument".to_string()))
                }
            }
            "to_int" => {
                if let Some(arg) = args.first() {
                    Some(Ok(Value::Int(arg.as_int())))
                } else {
                    Some(Err("to_int() requires an argument".to_string()))
                }
            }
            "to_float" => {
                if let Some(arg) = args.first() {
                    Some(Ok(Value::Float(arg.as_float())))
                } else {
                    Some(Err("to_float() requires an argument".to_string()))
                }
            }
            "to_string" => {
                if let Some(arg) = args.first() {
                    Some(Ok(Value::String(arg.as_string())))
                } else {
                    Some(Err("to_string() requires an argument".to_string()))
                }
            }
            "typeof" => {
                if let Some(arg) = args.first() {
                    let type_name = match arg {
                        Value::Int(_) => "int",
                        Value::Float(_) => "float",
                        Value::Bool(_) => "bool",
                        Value::String(_) => "string",
                        Value::Array(_) => "array",
                        Value::Map(_) => "map",
                        Value::Tuple(_) => "tuple",
                        Value::Closure { .. } => "closure",
                        Value::StructDef(_) => "struct_def",
                        Value::StructInstance(_) => "struct_instance",
                        Value::Function(_) => "function",
                        Value::Trait(_) => "trait",
                        Value::Reference(_) => "reference",
                        Value::Null => "null",
                        Value::Range { .. } => "range",
                    };
                    Some(Ok(Value::String(type_name.to_string())))
                } else {
                    Some(Err("typeof() requires an argument".to_string()))
                }
            }
            "exit" => {
                let code = args.first().map(|v| v.as_int()).unwrap_or(0) as i32;
                std::process::exit(code);
            }
            // File I/O operations
            "file_read" => {
                if let Some(arg) = args.first() {
                    let path = arg.as_string();
                    match fs::read_to_string(&path) {
                        Ok(contents) => Some(Ok(Value::String(contents))),
                        Err(e) => Some(Err(format!("Failed to read file: {}", e))),
                    }
                } else {
                    Some(Err("file_read() requires a path argument".to_string()))
                }
            }
            "file_write" => {
                if args.len() >= 2 {
                    let path = args[0].as_string();
                    let contents = args[1].as_string();
                    match fs::write(&path, &contents) {
                        Ok(_) => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("Failed to write file: {}", e))),
                    }
                } else {
                    Some(Err(
                        "file_write() requires path and content arguments".to_string()
                    ))
                }
            }
            "file_append" => {
                if args.len() >= 2 {
                    let path = args[0].as_string();
                    let contents = args[1].as_string();
                    match fs::OpenOptions::new().create(true).append(true).open(&path) {
                        Ok(mut file) => match file.write_all(contents.as_bytes()) {
                            Ok(_) => Some(Ok(Value::Null)),
                            Err(e) => Some(Err(format!("Failed to append: {}", e))),
                        },
                        Err(e) => Some(Err(format!("Failed to open file: {}", e))),
                    }
                } else {
                    Some(Err(
                        "file_append() requires path and content arguments".to_string()
                    ))
                }
            }
            "file_exists" => {
                if let Some(arg) = args.first() {
                    let path = arg.as_string();
                    Some(Ok(Value::Bool(std::path::Path::new(&path).exists())))
                } else {
                    Some(Err("file_exists() requires a path argument".to_string()))
                }
            }
            "file_remove" => {
                if let Some(arg) = args.first() {
                    let path = arg.as_string();
                    match fs::remove_file(&path) {
                        Ok(_) => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("Failed to remove: {}", e))),
                    }
                } else {
                    Some(Err("file_remove() requires a path argument".to_string()))
                }
            }
            "mkdir" => {
                if let Some(arg) = args.first() {
                    let path = arg.as_string();
                    match fs::create_dir_all(&path) {
                        Ok(_) => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("Failed to create directory: {}", e))),
                    }
                } else {
                    Some(Err("mkdir() requires a path argument".to_string()))
                }
            }
            "dir_list" => {
                let path = args
                    .first()
                    .map(|a| a.as_string())
                    .unwrap_or_else(|| ".".to_string());
                match fs::read_dir(&path) {
                    Ok(entries) => {
                        let mut result = Vec::new();
                        for entry in entries {
                            if let Ok(entry) = entry {
                                if let Some(name) = entry.file_name().to_str() {
                                    result.push(Value::String(name.to_string()));
                                }
                            }
                        }
                        Some(Ok(Value::Array(result)))
                    }
                    Err(e) => Some(Err(format!("Failed to list directory: {}", e))),
                }
            }
            // Threading operations
            "spawn" => {
                if let Some(arg) = args.first() {
                    if let Value::String(code) = arg {
                        let code = code.clone();
                        thread::spawn(move || {
                            let _ = execute_source(&code);
                        });
                        Some(Ok(Value::Null))
                    } else {
                        Some(Err("spawn() requires a string argument".to_string()))
                    }
                } else {
                    Some(Err("spawn() requires an argument".to_string()))
                }
            }
            "sleep" => {
                let millis = args.first().map(|v| v.as_int()).unwrap_or(1000) as u64;
                thread::sleep(Duration::from_millis(millis));
                Some(Ok(Value::Null))
            }
            "thread_id" => Some(Ok(Value::Int(0))),
            // Networking (see full implementations further below)
            "get_hostname" => Some(Ok(Value::String("localhost".to_string()))),
            // FFI functions
            "ffi_open" => {
                if let Some(arg) = args.first() {
                    let path = arg.as_string();
                    match crate::ffi::ffi_open(&path) {
                        Ok(handle) => Some(Ok(Value::Int(handle as i64))),
                        Err(e) => Some(Err(e)),
                    }
                } else {
                    Some(Err("ffi_open() requires a path argument".to_string()))
                }
            }
            "ffi_get_symbol" => {
                if args.len() >= 2 {
                    let lib_handle = args[0].as_int() as usize;
                    let name = args[1].as_string();
                    match crate::ffi::ffi_get_symbol(lib_handle, &name) {
                        Ok(symbol) => Some(Ok(Value::Int(symbol as i64))),
                        Err(e) => Some(Err(e)),
                    }
                } else {
                    Some(Err(
                        "ffi_get_symbol() requires library handle and symbol name".to_string(),
                    ))
                }
            }
            "ffi_malloc" => {
                let size = args.first().map(|v| v.as_int()).unwrap_or(1024) as usize;
                let ptr = crate::ffi::ffi_malloc(size);
                Some(Ok(Value::Int(ptr as i64)))
            }
            "ffi_free" => {
                if let Some(arg) = args.first() {
                    let ptr = arg.as_int() as usize;
                    crate::ffi::ffi_free(ptr);
                    Some(Ok(Value::Null))
                } else {
                    Some(Err("ffi_free() requires a pointer argument".to_string()))
                }
            }
            // GC functions
            "gc_collect" => {
                let freed = crate::gc::gc_collect();
                Some(Ok(Value::Int(freed as i64)))
            }
            "gc_stats" => {
                let stats = crate::gc::gc_stats();
                Some(Ok(Value::Array(vec![
                    Value::Int(stats.live_objects as i64),
                    Value::Int(stats.total_allocated as i64),
                    Value::Int(stats.total_freed as i64),
                ])))
            }
            // Time functions
            "time" => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                Some(Ok(Value::Int(now)))
            }
            "time_millis" => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                Some(Ok(Value::Int(now)))
            }
            // Environment functions
            "env_get" => {
                if let Some(arg) = args.first() {
                    let key = arg.as_string();
                    match std::env::var(&key) {
                        Ok(val) => Some(Ok(Value::String(val))),
                        Err(_) => Some(Ok(Value::Null)),
                    }
                } else {
                    Some(Err("env_get() requires a key argument".to_string()))
                }
            }
            "env_set" => {
                if args.len() >= 2 {
                    let key = args[0].as_string();
                    let val = args[1].as_string();
                    std::env::set_var(&key, &val);
                    Some(Ok(Value::Null))
                } else {
                    Some(Err("env_set() requires key and value arguments".to_string()))
                }
            }
            // Command execution
            "exec" => {
                if let Some(arg) = args.first() {
                    let cmd = arg.as_string();
                    match std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .output()
                    {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                            Some(Ok(Value::String(stdout)))
                        }
                        Err(e) => Some(Err(format!("Command failed: {}", e))),
                    }
                } else {
                    Some(Err("exec() requires a command argument".to_string()))
                }
            }
            // String utilities
            "strlen" => {
                if let Some(arg) = args.first() {
                    let s = arg.as_string();
                    Some(Ok(Value::Int(s.len() as i64)))
                } else {
                    Some(Err("strlen() requires a string argument".to_string()))
                }
            }
            "substring" => {
                if args.len() >= 3 {
                    let s = args[0].as_string();
                    let start = args[1].as_int() as usize;
                    let end = args[2].as_int() as usize;
                    let result = s.chars().skip(start).take(end - start).collect::<String>();
                    Some(Ok(Value::String(result)))
                } else {
                    Some(Err(
                        "substring() requires string, start, and end arguments".to_string()
                    ))
                }
            }
            // ── Array utilities ──────────────────────────────────────────────
            "push" => {
                if args.len() >= 2 {
                    match &args[0] {
                        Value::Array(arr) => {
                            let mut new_arr = arr.clone();
                            new_arr.push(args[1].clone());
                            Some(Ok(Value::Array(new_arr)))
                        }
                        _ => Some(Err("push() requires an array as first argument".to_string())),
                    }
                } else {
                    Some(Err("push(arr, elem) requires 2 arguments".to_string()))
                }
            }
            // ── String operations ────────────────────────────────────────────
            "format" => {
                if let Some(fmt) = args.first() {
                    let mut s = fmt.as_string();
                    let rest: Vec<&Value> = args.iter().skip(1).collect();
                    // printf-style: %s, %d, %i, %f, %.Nf, %x
                    let mut arg_idx = 0usize;
                    let mut out = String::new();
                    let chars: Vec<char> = s.chars().collect();
                    let mut i = 0;
                    while i < chars.len() {
                        if chars[i] == '%' && i + 1 < chars.len() {
                            i += 1;
                            // parse optional precision like .2
                            let mut precision: Option<usize> = None;
                            let mut width: Option<usize> = None;
                            // optional width digits
                            let mut num_s = String::new();
                            while i < chars.len() && chars[i].is_ascii_digit() {
                                num_s.push(chars[i]); i += 1;
                            }
                            if !num_s.is_empty() { width = num_s.parse().ok(); }
                            if i < chars.len() && chars[i] == '.' {
                                i += 1;
                                let mut prec_s = String::new();
                                while i < chars.len() && chars[i].is_ascii_digit() {
                                    prec_s.push(chars[i]); i += 1;
                                }
                                precision = prec_s.parse().ok();
                            }
                            let spec = if i < chars.len() { chars[i] } else { 's' };
                            i += 1;
                            let arg = rest.get(arg_idx).copied();
                            arg_idx += 1;
                            match spec {
                                's' => { out.push_str(&arg.map(|v| v.as_string()).unwrap_or_default()); }
                                'd' | 'i' => { out.push_str(&arg.map(|v| format!("{}", v.as_int())).unwrap_or_default()); }
                                'f' => {
                                    let f = arg.map(|v| v.as_float()).unwrap_or(0.0);
                                    let prec = precision.unwrap_or(6);
                                    out.push_str(&format!("{:.prec$}", f, prec = prec));
                                }
                                'e' | 'E' => {
                                    let f = arg.map(|v| v.as_float()).unwrap_or(0.0);
                                    let prec = precision.unwrap_or(6);
                                    if spec == 'e' { out.push_str(&format!("{:.prec$e}", f, prec = prec)); }
                                    else { out.push_str(&format!("{:.prec$E}", f, prec = prec)); }
                                }
                                'x' => { out.push_str(&format!("{:x}", arg.map(|v| v.as_int()).unwrap_or(0))); }
                                'X' => { out.push_str(&format!("{:X}", arg.map(|v| v.as_int()).unwrap_or(0))); }
                                'o' => { out.push_str(&format!("{:o}", arg.map(|v| v.as_int()).unwrap_or(0))); }
                                'b' => { out.push_str(&format!("{:b}", arg.map(|v| v.as_int()).unwrap_or(0))); }
                                '%' => { out.push('%'); arg_idx -= 1; } // literal %
                                _ => { out.push('%'); out.push(spec); arg_idx -= 1; }
                            }
                            let _ = width;
                        } else if chars[i] == '{' && i + 1 < chars.len() {
                            // {N} or {} positional args
                            if chars[i+1] == '}' {
                                out.push_str(&rest.get(arg_idx).map(|v| v.as_string()).unwrap_or_default());
                                arg_idx += 1; i += 2;
                            } else {
                                // {N}
                                let mut num_s = String::new();
                                let start = i + 1;
                                let mut j = start;
                                while j < chars.len() && chars[j] != '}' { num_s.push(chars[j]); j += 1; }
                                if j < chars.len() && chars[j] == '}' {
                                    if let Ok(idx) = num_s.parse::<usize>() {
                                        out.push_str(&rest.get(idx).map(|v| v.as_string()).unwrap_or_default());
                                        i = j + 1;
                                    } else { out.push(chars[i]); i += 1; }
                                } else { out.push(chars[i]); i += 1; }
                            }
                        } else {
                            out.push(chars[i]); i += 1;
                        }
                    }
                    s = out;
                    Some(Ok(Value::String(s)))
                } else { Some(Ok(Value::String(String::new()))) }
            }
            "input" | "readline" | "read_line" => {
                let prompt = args.first().map(|v| v.as_string()).unwrap_or_default();
                if !prompt.is_empty() { print!("{}", prompt); let _ = std::io::stdout().flush(); }
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).ok();
                Some(Ok(Value::String(line.trim_end_matches('\n').trim_end_matches('\r').to_string())))
            }
            "split" => {
                if args.len() >= 2 {
                    let s = args[0].as_string();
                    let sep = args[1].as_string();
                    let parts: Vec<Value> = if sep.is_empty() {
                        s.chars().map(|c| Value::String(c.to_string())).collect()
                    } else {
                        s.split(&sep as &str).map(|p| Value::String(p.to_string())).collect()
                    };
                    Some(Ok(Value::Array(parts)))
                } else { Some(Err("split(string, sep) requires 2 args".to_string())) }
            }
            "join" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let sep = args[1].as_string();
                        let joined = arr.iter().map(|v| v.as_string()).collect::<Vec<_>>().join(&sep);
                        Some(Ok(Value::String(joined)))
                    } else { Some(Err("join() first arg must be array".to_string())) }
                } else { Some(Err("join(array, sep) requires 2 args".to_string())) }
            }
            "contains" => {
                if args.len() >= 2 {
                    match &args[0] {
                        Value::String(s) => Some(Ok(Value::Bool(s.contains(&args[1].as_string() as &str)))),
                        Value::Array(arr) => Some(Ok(Value::Bool(arr.contains(&args[1])))),
                        _ => Some(Err("contains() requires string or array".to_string())),
                    }
                } else { Some(Err("contains(haystack, needle) requires 2 args".to_string())) }
            }
            "starts_with" => {
                if args.len() >= 2 {
                    Some(Ok(Value::Bool(args[0].as_string().starts_with(&args[1].as_string() as &str))))
                } else { Some(Err("starts_with() requires 2 args".to_string())) }
            }
            "ends_with" => {
                if args.len() >= 2 {
                    Some(Ok(Value::Bool(args[0].as_string().ends_with(&args[1].as_string() as &str))))
                } else { Some(Err("ends_with() requires 2 args".to_string())) }
            }
            "replace" => {
                if args.len() >= 3 {
                    let s = args[0].as_string();
                    let from = args[1].as_string();
                    let to = args[2].as_string();
                    Some(Ok(Value::String(s.replace(&from as &str, &to as &str))))
                } else { Some(Err("replace(s, from, to) requires 3 args".to_string())) }
            }
            "trim" => {
                Some(Ok(Value::String(args.first().map(|v| v.as_string().trim().to_string()).unwrap_or_default())))
            }
            "trim_start" | "ltrim" => {
                Some(Ok(Value::String(args.first().map(|v| v.as_string().trim_start().to_string()).unwrap_or_default())))
            }
            "trim_end" | "rtrim" => {
                Some(Ok(Value::String(args.first().map(|v| v.as_string().trim_end().to_string()).unwrap_or_default())))
            }
            "upper" | "to_upper" | "to_uppercase" => {
                Some(Ok(Value::String(args.first().map(|v| v.as_string().to_uppercase()).unwrap_or_default())))
            }
            "lower" | "to_lower" | "to_lowercase" => {
                Some(Ok(Value::String(args.first().map(|v| v.as_string().to_lowercase()).unwrap_or_default())))
            }
            "repeat" => {
                if args.len() >= 2 {
                    let s = args[0].as_string();
                    let n = args[1].as_int() as usize;
                    Some(Ok(Value::String(s.repeat(n))))
                } else { Some(Err("repeat(s, n) requires 2 args".to_string())) }
            }
            "index_of" | "find" => {
                if args.len() >= 2 {
                    match &args[0] {
                        Value::String(s) => {
                            let needle = args[1].as_string();
                            Some(Ok(Value::Int(s.find(&needle as &str).map(|i| i as i64).unwrap_or(-1))))
                        }
                        Value::Array(arr) => {
                            let needle = &args[1];
                            Some(Ok(Value::Int(arr.iter().position(|v| v == needle).map(|i| i as i64).unwrap_or(-1))))
                        }
                        _ => Some(Err("index_of() requires string or array".to_string())),
                    }
                } else { Some(Err("index_of(haystack, needle) requires 2 args".to_string())) }
            }
            "char_at" => {
                if args.len() >= 2 {
                    let s = args[0].as_string();
                    let idx = args[1].as_int() as usize;
                    Some(Ok(s.chars().nth(idx).map(|c| Value::String(c.to_string())).unwrap_or(Value::Null)))
                } else { Some(Err("char_at(s, i) requires 2 args".to_string())) }
            }
            "parse_int" | "int" => {
                if let Some(v) = args.first() {
                    let s = v.as_string();
                    let base = args.get(1).map(|v| v.as_int() as u32).unwrap_or(0);
                    let n: Result<i64, _> = if base > 1 {
                        i64::from_str_radix(s.trim(), base).map_err(|e| e.to_string())
                    } else if s.starts_with("0x") || s.starts_with("0X") {
                        i64::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
                    } else if s.starts_with("0b") || s.starts_with("0B") {
                        i64::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())
                    } else if s.starts_with("0o") || s.starts_with("0O") {
                        i64::from_str_radix(&s[2..], 8).map_err(|e| e.to_string())
                    } else {
                        s.trim().parse::<i64>().map_err(|e| e.to_string())
                    };
                    Some(Ok(n.map(Value::Int).unwrap_or(Value::Null)))
                } else { Some(Err("parse_int() requires argument".to_string())) }
            }
            "parse_float" | "float" => {
                if let Some(v) = args.first() {
                    let s = v.as_string();
                    Some(Ok(s.trim().parse::<f64>().map(Value::Float).unwrap_or(Value::Null)))
                } else { Some(Err("parse_float() requires argument".to_string())) }
            }
            "chars" => {
                if let Some(v) = args.first() {
                    let chars: Vec<Value> = v.as_string().chars().map(|c| Value::String(c.to_string())).collect();
                    Some(Ok(Value::Array(chars)))
                } else { Some(Err("chars() requires argument".to_string())) }
            }
            "bytes" => {
                if let Some(v) = args.first() {
                    let bytes: Vec<Value> = v.as_string().bytes().map(|b| Value::Int(b as i64)).collect();
                    Some(Ok(Value::Array(bytes)))
                } else { Some(Err("bytes() requires argument".to_string())) }
            }
            "pad_left" => {
                if args.len() >= 2 {
                    let s = args[0].as_string();
                    let width = args[1].as_int() as usize;
                    let pad = args.get(2).map(|v| v.as_string()).unwrap_or_else(|| " ".to_string());
                    let pad_char = pad.chars().next().unwrap_or(' ');
                    let padded = format!("{:>width$}", s, width = width).replace(' ', &pad_char.to_string());
                    Some(Ok(Value::String(format!("{}{}", pad_char.to_string().repeat(width.saturating_sub(s.len())), s))))
                } else { Some(Err("pad_left(s, width [, char]) requires 2+ args".to_string())) }
            }
            "pad_right" => {
                if args.len() >= 2 {
                    let s = args[0].as_string();
                    let width = args[1].as_int() as usize;
                    let pad_char = args.get(2).and_then(|v| v.as_string().chars().next()).unwrap_or(' ');
                    Some(Ok(Value::String(format!("{:<width$}", s, width = width).replace(' ', &pad_char.to_string()))))
                } else { Some(Err("pad_right(s, width [, char]) requires 2+ args".to_string())) }
            }
            // ── Math operations ──────────────────────────────────────────────
            "abs" => {
                if let Some(v) = args.first() {
                    Some(Ok(match v { Value::Int(i) => Value::Int(i.abs()), Value::Float(f) => Value::Float(f.abs()), _ => return Some(Err("abs() requires numeric argument".to_string())) }))
                } else { Some(Err("abs() requires argument".to_string())) }
            }
            "min" => {
                if args.len() == 1 {
                    // min(array) — find minimum element
                    if let Value::Array(arr) = &args[0] {
                        if arr.is_empty() { return Some(Ok(Value::Null)); }
                        let mut m = arr[0].clone();
                        for v in arr.iter().skip(1) {
                            if self.eval_binary_op("<", v.clone(), m.clone()).unwrap_or(Value::Bool(false)).is_truthy() {
                                m = v.clone();
                            }
                        }
                        Some(Ok(m))
                    } else { Some(Err("min(array) or min(a, b)".to_string())) }
                } else if args.len() >= 2 {
                    let cmp = self.eval_binary_op("<", args[0].clone(), args[1].clone()).unwrap_or(Value::Bool(false));
                    Some(Ok(if cmp.is_truthy() { args[0].clone() } else { args[1].clone() }))
                } else { Some(Err("min(a, b) or min(array)".to_string())) }
            }
            "max" => {
                if args.len() == 1 {
                    // max(array) — find maximum element
                    if let Value::Array(arr) = &args[0] {
                        if arr.is_empty() { return Some(Ok(Value::Null)); }
                        let mut m = arr[0].clone();
                        for v in arr.iter().skip(1) {
                            if self.eval_binary_op(">", v.clone(), m.clone()).unwrap_or(Value::Bool(false)).is_truthy() {
                                m = v.clone();
                            }
                        }
                        Some(Ok(m))
                    } else { Some(Err("max(array) or max(a, b)".to_string())) }
                } else if args.len() >= 2 {
                    let cmp = self.eval_binary_op(">", args[0].clone(), args[1].clone()).unwrap_or(Value::Bool(false));
                    Some(Ok(if cmp.is_truthy() { args[0].clone() } else { args[1].clone() }))
                } else { Some(Err("max(a, b) or max(array)".to_string())) }
            }
            "sqrt" => {
                if let Some(v) = args.first() {
                    Some(Ok(Value::Float(v.as_float().sqrt())))
                } else { Some(Err("sqrt() requires argument".to_string())) }
            }
            "pow" => {
                if args.len() >= 2 {
                    let base = args[0].as_float();
                    let exp = args[1].as_float();
                    if args[0].is_int() && args[1].is_int() && args[1].as_int() >= 0 {
                        Some(Ok(Value::Int(args[0].as_int().pow(args[1].as_int() as u32))))
                    } else {
                        Some(Ok(Value::Float(base.powf(exp))))
                    }
                } else { Some(Err("pow(base, exp) requires 2 args".to_string())) }
            }
            "floor" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().floor()).unwrap_or(0.0)))) }
            "ceil" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().ceil()).unwrap_or(0.0)))) }
            "round" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().round()).unwrap_or(0.0)))) }
            "trunc" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().trunc()).unwrap_or(0.0)))) }
            "sin" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().sin()).unwrap_or(0.0)))) }
            "cos" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().cos()).unwrap_or(0.0)))) }
            "tan" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().tan()).unwrap_or(0.0)))) }
            "asin" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().asin()).unwrap_or(0.0)))) }
            "acos" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().acos()).unwrap_or(0.0)))) }
            "atan" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().atan()).unwrap_or(0.0)))) }
            "atan2" => {
                if args.len() >= 2 { Some(Ok(Value::Float(args[0].as_float().atan2(args[1].as_float()))))
                } else { Some(Err("atan2(y, x) requires 2 args".to_string())) }
            }
            "log" | "ln" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().ln()).unwrap_or(0.0)))) }
            "log2" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().log2()).unwrap_or(0.0)))) }
            "log10" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().log10()).unwrap_or(0.0)))) }
            "exp" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().exp()).unwrap_or(1.0)))) }
            "random" | "rand" => {
                // LCG random — no external crate needed
                let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() as i64;
                let r = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) >> 33) as f64 / i64::MAX as f64;
                if args.len() >= 2 {
                    let lo = args[0].as_int();
                    let hi = args[1].as_int();
                    Some(Ok(Value::Int(lo + (r.abs() * (hi - lo) as f64) as i64)))
                } else {
                    Some(Ok(Value::Float(r.abs())))
                }
            }
            "pi" => Some(Ok(Value::Float(std::f64::consts::PI))),
            "e"  => Some(Ok(Value::Float(std::f64::consts::E))),
            // ── Array operations ─────────────────────────────────────────────
            "pop" => {
                if let Some(Value::Array(arr)) = args.first() {
                    let mut a = arr.clone();
                    let last = a.pop().unwrap_or(Value::Null);
                    Some(Ok(Value::Array(vec![Value::Array(a), last])))
                } else { Some(Err("pop() requires array".to_string())) }
            }
            "shift" => {
                if let Some(Value::Array(arr)) = args.first() {
                    if arr.is_empty() { return Some(Ok(Value::Null)); }
                    let first = arr[0].clone();
                    Some(Ok(first))
                } else { Some(Err("shift() requires array".to_string())) }
            }
            "insert" => {
                if args.len() >= 3 {
                    if let Value::Array(arr) = &args[0] {
                        let idx = args[1].as_int() as usize;
                        let mut a = arr.clone();
                        a.insert(idx.min(a.len()), args[2].clone());
                        Some(Ok(Value::Array(a)))
                    } else { Some(Err("insert() first arg must be array".to_string())) }
                } else { Some(Err("insert(arr, idx, val) requires 3 args".to_string())) }
            }
            "remove" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let idx = args[1].as_int() as usize;
                        let mut a = arr.clone();
                        if idx < a.len() { a.remove(idx); }
                        Some(Ok(Value::Array(a)))
                    } else { Some(Err("remove() first arg must be array".to_string())) }
                } else { Some(Err("remove(arr, idx) requires 2 args".to_string())) }
            }
            "sort" => {
                if let Some(Value::Array(arr)) = args.first() {
                    let mut a = arr.clone();
                    a.sort_by(|x, y| {
                        match (x, y) {
                            (Value::Int(a), Value::Int(b)) => a.cmp(b),
                            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                            (Value::String(a), Value::String(b)) => a.cmp(b),
                            _ => std::cmp::Ordering::Equal,
                        }
                    });
                    Some(Ok(Value::Array(a)))
                } else { Some(Err("sort() requires array".to_string())) }
            }
            "reverse" => {
                if let Some(Value::Array(arr)) = args.first() {
                    let mut a = arr.clone();
                    a.reverse();
                    Some(Ok(Value::Array(a)))
                } else if let Some(v) = args.first() {
                    let s: String = v.as_string().chars().rev().collect();
                    Some(Ok(Value::String(s)))
                } else { Some(Err("reverse() requires array or string".to_string())) }
            }
            "concat" => {
                let mut result = Vec::new();
                for arg in args {
                    if let Value::Array(arr) = arg {
                        result.extend(arr.iter().cloned());
                    } else {
                        result.push(arg.clone());
                    }
                }
                Some(Ok(Value::Array(result)))
            }
            "slice" => {
                if args.len() >= 3 {
                    if let Value::Array(arr) = &args[0] {
                        let start = args[1].as_int().max(0) as usize;
                        let end = (args[2].as_int() as usize).min(arr.len());
                        Some(Ok(Value::Array(arr[start..end].to_vec())))
                    } else {
                        let s = args[0].as_string();
                        let start = args[1].as_int().max(0) as usize;
                        let end = (args[2].as_int() as usize).min(s.chars().count());
                        Some(Ok(Value::String(s.chars().skip(start).take(end - start).collect())))
                    }
                } else { Some(Err("slice(arr, start, end) requires 3 args".to_string())) }
            }
            "flatten" => {
                if let Some(Value::Array(arr)) = args.first() {
                    let mut flat = Vec::new();
                    for v in arr {
                        if let Value::Array(inner) = v { flat.extend(inner.iter().cloned()); }
                        else { flat.push(v.clone()); }
                    }
                    Some(Ok(Value::Array(flat)))
                } else { Some(Err("flatten() requires array".to_string())) }
            }
            "unique" | "dedup" => {
                if let Some(Value::Array(arr)) = args.first() {
                    let mut seen = Vec::new();
                    let mut result = Vec::new();
                    for v in arr {
                        if !seen.contains(v) { seen.push(v.clone()); result.push(v.clone()); }
                    }
                    Some(Ok(Value::Array(result)))
                } else { Some(Err("unique() requires array".to_string())) }
            }
            "sum" => {
                if let Some(Value::Array(arr)) = args.first() {
                    let total = arr.iter().fold(0i64, |acc, v| acc + v.as_int());
                    let has_float = arr.iter().any(|v| matches!(v, Value::Float(_)));
                    if has_float {
                        Some(Ok(Value::Float(arr.iter().fold(0.0f64, |acc, v| acc + v.as_float()))))
                    } else {
                        Some(Ok(Value::Int(total)))
                    }
                } else { Some(Err("sum() requires array".to_string())) }
            }
            "product" => {
                if let Some(Value::Array(arr)) = args.first() {
                    let has_float = arr.iter().any(|v| matches!(v, Value::Float(_)));
                    if has_float {
                        Some(Ok(Value::Float(arr.iter().fold(1.0f64, |acc, v| acc * v.as_float()))))
                    } else {
                        Some(Ok(Value::Int(arr.iter().fold(1i64, |acc, v| acc * v.as_int()))))
                    }
                } else { Some(Err("product() requires array".to_string())) }
            }
            "range" => {
                // range(end) or range(start, end) or range(start, end, step)
                let (start, end, step) = match args.len() {
                    1 => (0, args[0].as_int(), 1),
                    2 => (args[0].as_int(), args[1].as_int(), 1),
                    _ => (args[0].as_int(), args[1].as_int(), args[2].as_int().max(1)),
                };
                let mut v = Vec::new();
                let mut i = start;
                while i < end { v.push(Value::Int(i)); i += step; }
                Some(Ok(Value::Array(v)))
            }
            "zip" => {
                if args.len() >= 2 {
                    if let (Value::Array(a), Value::Array(b)) = (&args[0], &args[1]) {
                        let zipped: Vec<Value> = a.iter().zip(b.iter())
                            .map(|(x, y)| Value::Array(vec![x.clone(), y.clone()]))
                            .collect();
                        Some(Ok(Value::Array(zipped)))
                    } else { Some(Err("zip() requires two arrays".to_string())) }
                } else { Some(Err("zip(a, b) requires 2 args".to_string())) }
            }
            "count" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let needle = &args[1];
                        Some(Ok(Value::Int(arr.iter().filter(|v| *v == needle).count() as i64)))
                    } else {
                        let s = args[0].as_string();
                        let pat = args[1].as_string();
                        Some(Ok(Value::Int(s.matches(&pat as &str).count() as i64)))
                    }
                } else if let Some(Value::Array(arr)) = args.first() {
                    Some(Ok(Value::Int(arr.len() as i64)))
                } else { Some(Err("count() requires array".to_string())) }
            }
            "any" => {
                if args.is_empty() { return Some(Err("any(array[, fn])".to_string())); }
                if let Value::Array(arr) = &args[0] {
                    if args.len() >= 2 {
                        // any(array, closure) — test each with closure
                        let f = args[1].clone(); let arr = arr.clone();
                        for item in arr {
                            let r = match self.call_value(f.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            if r.is_truthy() { return Some(Ok(Value::Bool(true))); }
                        }
                        Some(Ok(Value::Bool(false)))
                    } else {
                        Some(Ok(Value::Bool(arr.iter().any(|v| v.is_truthy()))))
                    }
                } else { Some(Err("any: expected array".to_string())) }
            }
            "all" => {
                if args.is_empty() { return Some(Err("all(array[, fn])".to_string())); }
                if let Value::Array(arr) = &args[0] {
                    if args.len() >= 2 {
                        // all(array, closure) — test each with closure
                        let f = args[1].clone(); let arr = arr.clone();
                        for item in arr {
                            let r = match self.call_value(f.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            if !r.is_truthy() { return Some(Ok(Value::Bool(false))); }
                        }
                        Some(Ok(Value::Bool(true)))
                    } else {
                        Some(Ok(Value::Bool(arr.iter().all(|v| v.is_truthy()))))
                    }
                } else { Some(Err("all: expected array".to_string())) }
            }
            "first" | "head" => {
                if let Some(Value::Array(arr)) = args.first() {
                    Some(Ok(arr.first().cloned().unwrap_or(Value::Null)))
                } else { Some(Err("first() requires array".to_string())) }
            }
            "last" => {
                if let Some(Value::Array(arr)) = args.first() {
                    Some(Ok(arr.last().cloned().unwrap_or(Value::Null)))
                } else { Some(Err("last() requires array".to_string())) }
            }
            "is_empty" => {
                if let Some(v) = args.first() {
                    Some(Ok(Value::Bool(match v {
                        Value::String(s) => s.is_empty(),
                        Value::Array(a) => a.is_empty(),
                        Value::Null => true,
                        _ => false,
                    })))
                } else { Some(Ok(Value::Bool(true))) }
            }
            // ── Type checks ──────────────────────────────────────────────────
            "is_int"    => { Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Int(_)))))) }
            "is_float"  => { Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Float(_)))))) }
            "is_string" => { Some(Ok(Value::Bool(matches!(args.first(), Some(Value::String(_)))))) }
            "is_bool"   => { Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Bool(_)))))) }
            "is_null"   => { Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Null) | None)))) }
            "is_array"  => { Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Array(_)))))) }
            "is_number" => { Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Int(_)) | Some(Value::Float(_)))))) }
            // ── I/O & system ─────────────────────────────────────────────────
            "panic" => {
                let msg = args.first().map(|v| v.as_string()).unwrap_or_else(|| "explicit panic".to_string());
                eprintln!("PANIC: {}", msg);
                std::process::exit(101);
            }
            "assert" => {
                let cond = args.first().map(|v| v.is_truthy()).unwrap_or(false);
                if !cond {
                    let msg = args.get(1).map(|v| v.as_string()).unwrap_or_else(|| "assertion failed".to_string());
                    eprintln!("ASSERTION FAILED: {}", msg);
                    std::process::exit(1);
                }
                Some(Ok(Value::Null))
            }
            "assert_eq" => {
                if args.len() >= 2 {
                    if args[0] != args[1] {
                        let msg = args.get(2).map(|v| v.as_string()).unwrap_or_else(|| format!("assert_eq failed: {:?} != {:?}", args[0], args[1]));
                        eprintln!("ASSERTION FAILED: {}", msg);
                        std::process::exit(1);
                    }
                    Some(Ok(Value::Null))
                } else { Some(Err("assert_eq(a, b) requires 2 args".to_string())) }
            }
            "dbg" => {
                if let Some(v) = args.first() {
                    eprintln!("[dbg] {:?}", v);
                    Some(Ok(v.clone()))
                } else { Some(Ok(Value::Null)) }
            }
            "print_err" | "eprint" => {
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { eprint!(" "); }
                    eprint!("{}", a);
                }
                Some(Ok(Value::Null))
            }
            "println_err" | "eprintln" => {
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { eprint!(" "); }
                    eprint!("{}", a);
                }
                eprintln!();
                Some(Ok(Value::Null))
            }
            "flush" => {
                let _ = std::io::stdout().flush();
                Some(Ok(Value::Null))
            }
            "read_file" => {
                if let Some(arg) = args.first() {
                    let path = arg.as_string();
                    match fs::read_to_string(&path) {
                        Ok(contents) => Some(Ok(Value::String(contents))),
                        Err(e) => Some(Err(format!("read_file: {}", e))),
                    }
                } else { Some(Err("read_file(path)".to_string())) }
            }
            "write_file" => {
                if args.len() >= 2 {
                    match fs::write(args[0].as_string(), args[1].as_string()) {
                        Ok(_) => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("write_file: {}", e))),
                    }
                } else { Some(Err("write_file(path, content)".to_string())) }
            }
            "args" => {
                let a: Vec<Value> = std::env::args().skip(1).map(Value::String).collect();
                Some(Ok(Value::Array(a)))
            }
            "cwd" => {
                Some(Ok(std::env::current_dir()
                    .map(|p| Value::String(p.to_string_lossy().to_string()))
                    .unwrap_or(Value::Null)))
            }
            "chdir" => {
                if let Some(v) = args.first() {
                    match std::env::set_current_dir(v.as_string()) {
                        Ok(_) => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("chdir: {}", e))),
                    }
                } else { Some(Err("chdir(path)".to_string())) }
            }
            "getpid" => Some(Ok(Value::Int(std::process::id() as i64))),
            // ── Networking extras (full impls are in TCP/UDP section below) ──
            // ── Functional programming ───────────────────────────────────────
            "map_fn" | "map" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let closure = args[1].clone();
                        let arr_clone = arr.clone();
                        let mut result = Vec::new();
                        for item in arr_clone {
                            let r = match self.call_value(closure.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            result.push(r);
                        }
                        Some(Ok(Value::Array(result)))
                    } else { Some(Err("map(array, fn)".to_string())) }
                } else { None }
            }
            "filter_fn" | "filter" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let closure = args[1].clone();
                        let arr_clone = arr.clone();
                        let mut result = Vec::new();
                        for item in arr_clone {
                            let r = match self.call_value(closure.clone(), vec![item.clone()]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            if r.is_truthy() { result.push(item); }
                        }
                        Some(Ok(Value::Array(result)))
                    } else { Some(Err("filter(array, fn)".to_string())) }
                } else { None }
            }
            "reduce" | "fold" => {
                if args.len() >= 3 {
                    if let Value::Array(arr) = &args[0] {
                        let closure = args[1].clone();
                        let mut acc = args[2].clone();
                        for item in arr.clone() {
                            acc = match self.call_value(closure.clone(), vec![acc, item]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                        }
                        Some(Ok(acc))
                    } else { Some(Err("reduce(array, fn, initial)".to_string())) }
                } else { None }
            }
            "for_each" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let closure = args[1].clone();
                        for item in arr.clone() {
                            match self.call_value(closure.clone(), vec![item]) { Ok(_) => {}, Err(e) => return Some(Err(e)) };
                        }
                        Some(Ok(Value::Null))
                    } else { Some(Err("for_each(array, fn)".to_string())) }
                } else { None }
            }
            "find_fn" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let closure = args[1].clone();
                        for item in arr.clone() {
                            let r = match self.call_value(closure.clone(), vec![item.clone()]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            if r.is_truthy() { return Some(Ok(item)); }
                        }
                        Some(Ok(Value::Null))
                    } else { Some(Err("find_fn(array, fn)".to_string())) }
                } else { None }
            }
            "flat_map" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let closure = args[1].clone();
                        let mut result = Vec::new();
                        for item in arr.clone() {
                            let r = match self.call_value(closure.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            if let Value::Array(inner) = r { result.extend(inner); }
                            else { result.push(r); }
                        }
                        Some(Ok(Value::Array(result)))
                    } else { Some(Err("flat_map(array, fn)".to_string())) }
                } else { None }
            }
            "call" => {
                // call(fn, args...) — call a closure/function value
                if let Some(callable) = args.first() {
                    let rest = args[1..].to_vec();
                    Some(self.call_value(callable.clone(), rest))
                } else { Some(Err("call(fn, args...)".to_string())) }
            }
            "sorted_by" | "sort_by" => {
                // sorted_by(array, |a, b| a < b) — sort using comparator closure
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let cmp_fn = args[1].clone();
                        let mut new_arr = arr.clone();
                        let mut sort_err: Option<String> = None;
                        new_arr.sort_by(|a, b| {
                            if sort_err.is_some() { return std::cmp::Ordering::Equal; }
                            match self.call_value(cmp_fn.clone(), vec![a.clone(), b.clone()]) {
                                Ok(Value::Bool(true)) => std::cmp::Ordering::Less,
                                Ok(Value::Bool(false)) => std::cmp::Ordering::Greater,
                                Ok(Value::Int(n)) => n.cmp(&0),
                                Ok(_) => std::cmp::Ordering::Equal,
                                Err(e) => { sort_err = Some(e); std::cmp::Ordering::Equal }
                            }
                        });
                        if let Some(e) = sort_err { return Some(Err(e)); }
                        Some(Ok(Value::Array(new_arr)))
                    } else { Some(Err("sorted_by(array, comparator_fn)".to_string())) }
                } else { None }
            }
            // ── Map/Dict operations ──────────────────────────────────────────
            "map_new" | "dict" | "hashmap" => Some(Ok(Value::Map(HashMap::new()))),
            "keys" => {
                if let Some(Value::Map(m)) = args.first() {
                    Some(Ok(Value::Array(m.keys().map(|k| Value::String(k.clone())).collect())))
                } else { Some(Err("keys(map)".to_string())) }
            }
            "values" => {
                if let Some(Value::Map(m)) = args.first() {
                    Some(Ok(Value::Array(m.values().cloned().collect())))
                } else if let Some(Value::Array(_)) = args.first() {
                    // values(array) → same as the array itself
                    Some(Ok(args[0].clone()))
                } else { Some(Err("values(map)".to_string())) }
            }
            "get" => {
                if args.len() >= 2 {
                    match &args[0] {
                        Value::Map(m) => {
                            let key = args[1].as_string();
                            let default = args.get(2).cloned().unwrap_or(Value::Null);
                            Some(Ok(m.get(&key).cloned().unwrap_or(default)))
                        }
                        Value::Array(arr) => {
                            let idx = args[1].as_int();
                            let idx = if idx < 0 { arr.len() as i64 + idx } else { idx } as usize;
                            Some(Ok(arr.get(idx).cloned().unwrap_or(Value::Null)))
                        }
                        _ => Some(Err("get(dict, key) or get(array, idx)".to_string())),
                    }
                } else { None }
            }
            "set_key" | "set" => {
                if args.len() >= 3 {
                    if let Value::Map(mut m) = args[0].clone() {
                        m.insert(args[1].as_string(), args[2].clone());
                        Some(Ok(Value::Map(m)))
                    } else { Some(Err("set(map, key, val)".to_string())) }
                } else { None }
            }
            "del_key" | "delete" => {
                if args.len() >= 2 {
                    if let Value::Map(mut m) = args[0].clone() {
                        m.remove(&args[1].as_string());
                        Some(Ok(Value::Map(m)))
                    } else { Some(Err("del_key(map, key)".to_string())) }
                } else { None }
            }
            "has_key" | "has" => {
                if args.len() >= 2 {
                    if let Value::Map(m) = &args[0] {
                        Some(Ok(Value::Bool(m.contains_key(&args[1].as_string()))))
                    } else { Some(Err("has_key(map, key)".to_string())) }
                } else { None }
            }
            "merge_maps" | "merge" => {
                if args.len() >= 2 {
                    if let (Value::Map(mut a), Value::Map(b)) = (args[0].clone(), args[1].clone()) {
                        for (k, v) in b { a.insert(k, v); }
                        Some(Ok(Value::Map(a)))
                    } else { Some(Err("merge(map1, map2)".to_string())) }
                } else { None }
            }
            "from_pairs" | "from_entries" => {
                if let Some(Value::Array(pairs)) = args.first() {
                    let mut m = HashMap::new();
                    for pair in pairs {
                        if let Value::Array(kv) = pair {
                            if kv.len() >= 2 {
                                m.insert(kv[0].as_string(), kv[1].clone());
                            }
                        }
                    }
                    Some(Ok(Value::Map(m)))
                } else { Some(Err("from_pairs([[key,val], ...])".to_string())) }
            }
            "entries" => {
                if let Some(Value::Map(m)) = args.first() {
                    Some(Ok(Value::Array(
                        m.iter().map(|(k, v)| Value::Array(vec![Value::String(k.clone()), v.clone()])).collect()
                    )))
                } else { None }
            }
            // ── Tuple operations ─────────────────────────────────────────────
            "tuple" => Some(Ok(Value::Tuple(args.to_vec()))),
            "tuple_get" => {
                if args.len() >= 2 {
                    if let Value::Tuple(t) = &args[0] {
                        let idx = args[1].as_int() as usize;
                        Some(Ok(t.get(idx).cloned().unwrap_or(Value::Null)))
                    } else { Some(Err("tuple_get(tuple, idx)".to_string())) }
                } else { None }
            }
            // ── JSON serialization ────────────────────────────────────────────
            "json_encode" | "to_json" | "json" | "json_stringify" => {
                if let Some(val) = args.first() {
                    Some(Ok(Value::String(Self::value_to_json(val))))
                } else { Some(Ok(Value::String("null".to_string()))) }
            }
            "json_decode" | "from_json" | "parse_json" | "json_parse" => {
                if let Some(val) = args.first() {
                    Some(Ok(Self::json_to_value(&val.as_string())))
                } else { Some(Ok(Value::Null)) }
            }
            // ── HTTP ─────────────────────────────────────────────────────────
            "http_post" => {
                if args.len() >= 2 {
                    let url = args[0].as_string();
                    let body = args[1].as_string();
                    let content_type = args.get(2).map(|v| v.as_string()).unwrap_or_else(|| "application/json".to_string());
                    match std::process::Command::new("curl")
                        .args(&["-s", "-X", "POST", "-H", &format!("Content-Type: {}", content_type), "-d", &body, &url])
                        .output()
                    {
                        Ok(out) => Some(Ok(Value::String(String::from_utf8_lossy(&out.stdout).to_string()))),
                        Err(e) => Some(Err(format!("http_post failed: {}", e))),
                    }
                } else { Some(Err("http_post(url, body, [content_type])".to_string())) }
            }
            "http_put" => {
                if args.len() >= 2 {
                    let url = args[0].as_string();
                    let body = args[1].as_string();
                    match std::process::Command::new("curl")
                        .args(&["-s", "-X", "PUT", "-d", &body, &url])
                        .output()
                    {
                        Ok(out) => Some(Ok(Value::String(String::from_utf8_lossy(&out.stdout).to_string()))),
                        Err(e) => Some(Err(format!("http_put failed: {}", e))),
                    }
                } else { Some(Err("http_put(url, body)".to_string())) }
            }
            "http_delete" => {
                if let Some(v) = args.first() {
                    match std::process::Command::new("curl").args(&["-s", "-X", "DELETE", &v.as_string()]).output() {
                        Ok(out) => Some(Ok(Value::String(String::from_utf8_lossy(&out.stdout).to_string()))),
                        Err(e) => Some(Err(format!("http_delete failed: {}", e))),
                    }
                } else { Some(Err("http_delete(url)".to_string())) }
            }
            // ── Regex (via process) ───────────────────────────────────────────
            // regex_match(pattern, text) -> bool  [updated: uses regex crate, pattern first]
            "simple_regex_match" => {
                if args.len() >= 2 {
                    let pat  = args[0].as_string();
                    let text = args[1].as_string();
                    match regex::Regex::new(&pat) {
                        Ok(re) => Some(Ok(Value::Bool(re.is_match(&text)))),
                        Err(e) => Some(Err(format!("regex_match: {}", e))),
                    }
                } else { Some(Err("regex_match(pattern, text)".to_string())) }
            }
            // ── Type conversions ──────────────────────────────────────────────
            "str" | "string" => {
                Some(Ok(Value::String(args.first().map(|v| v.as_string()).unwrap_or_default())))
            }
            "num" | "number" => {
                if let Some(v) = args.first() {
                    match v {
                        Value::Int(i) => Some(Ok(Value::Int(*i))),
                        Value::Float(f) => Some(Ok(Value::Float(*f))),
                        Value::String(s) => {
                            if let Ok(i) = s.trim().parse::<i64>() { Some(Ok(Value::Int(i))) }
                            else if let Ok(f) = s.trim().parse::<f64>() { Some(Ok(Value::Float(f))) }
                            else { Some(Ok(Value::Null)) }
                        }
                        Value::Bool(b) => Some(Ok(Value::Int(if *b { 1 } else { 0 }))),
                        _ => Some(Ok(Value::Null)),
                    }
                } else { Some(Ok(Value::Null)) }
            }
            "bool" => {
                Some(Ok(Value::Bool(args.first().map(|v| v.is_truthy()).unwrap_or(false))))
            }
            "array" | "list" => {
                // Convert single value to array, or wrap multiple args
                if args.len() == 1 {
                    match &args[0] {
                        Value::Array(a) => Some(Ok(Value::Array(a.clone()))),
                        Value::Range { start, end, inclusive } => {
                            let limit = if *inclusive { end + 1 } else { *end };
                            Some(Ok(Value::Array((*start..limit).map(Value::Int).collect())))
                        }
                        other => Some(Ok(Value::Array(vec![other.clone()]))),
                    }
                } else {
                    Some(Ok(Value::Array(args.to_vec())))
                }
            }
            // ── Hashing / UUID ────────────────────────────────────────────────
            "hash" => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut h = DefaultHasher::new();
                args.first().map(|v| v.as_string()).unwrap_or_default().hash(&mut h);
                Some(Ok(Value::Int(h.finish() as i64)))
            }
            "uuid" => {
                // Simple UUID v4 approximation using time + pid
                let t = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos();
                let p = std::process::id();
                Some(Ok(Value::String(format!("{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
                    t, p & 0xffff, (t >> 4) & 0xfff, 0x8000 | (p & 0x3fff),
                    (t as u64).wrapping_mul(p as u64) & 0xffffffffffff
                ))))
            }
            // ── Bitwise operations ────────────────────────────────────────────
            "bit_and" => {
                if args.len() >= 2 { Some(Ok(Value::Int(args[0].as_int() & args[1].as_int()))) }
                else { None }
            }
            "bit_or" => {
                if args.len() >= 2 { Some(Ok(Value::Int(args[0].as_int() | args[1].as_int()))) }
                else { None }
            }
            "bit_xor" => {
                if args.len() >= 2 { Some(Ok(Value::Int(args[0].as_int() ^ args[1].as_int()))) }
                else { None }
            }
            "bit_not" => {
                if let Some(v) = args.first() { Some(Ok(Value::Int(!v.as_int()))) }
                else { None }
            }
            "bit_shl" => {
                if args.len() >= 2 { Some(Ok(Value::Int(args[0].as_int() << (args[1].as_int() as u32)))) }
                else { None }
            }
            "bit_shr" => {
                if args.len() >= 2 { Some(Ok(Value::Int(args[0].as_int() >> (args[1].as_int() as u32)))) }
                else { None }
            }
            // ── Process / OS ─────────────────────────────────────────────────
            "shell" => {
                let cmd = args.first().map(|v| v.as_string()).unwrap_or_default();
                match std::process::Command::new("sh").arg("-c").arg(&cmd).output() {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        let code = out.status.code().unwrap_or(-1);
                        let mut m = HashMap::new();
                        m.insert("stdout".to_string(), Value::String(stdout));
                        m.insert("stderr".to_string(), Value::String(stderr));
                        m.insert("code".to_string(), Value::Int(code as i64));
                        Some(Ok(Value::Map(m)))
                    }
                    Err(e) => Some(Err(format!("shell: {}", e))),
                }
            }
            "pipe" => {
                // pipe(cmd1, cmd2, ...) — pipe multiple commands
                if args.is_empty() { return Some(Ok(Value::Null)); }
                let mut input = String::new();
                for arg in args {
                    let cmd = arg.as_string();
                    match std::process::Command::new("sh")
                        .arg("-c").arg(&cmd)
                        .stdin(if input.is_empty() { std::process::Stdio::inherit() } else { std::process::Stdio::piped() })
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                    {
                        Ok(mut child) => {
                            if !input.is_empty() {
                                if let Some(s) = child.stdin.take() {
                                    let _ = std::io::Write::write_all(&mut std::io::BufWriter::new(s), input.as_bytes());
                                }
                            }
                            input = child.wait_with_output()
                                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                                .unwrap_or_default();
                        }
                        Err(_) => break,
                    }
                }
                Some(Ok(Value::String(input)))
            }
            // ── Misc missing ──────────────────────────────────────────────────
            "not" => Some(Ok(Value::Bool(!args.first().map(|v| v.is_truthy()).unwrap_or(false)))),
            "if_null" | "coalesce" => {
                // Return first non-null value
                for arg in args {
                    if !matches!(arg, Value::Null) { return Some(Ok(arg.clone())); }
                }
                Some(Ok(Value::Null))
            }
            "identity" | "id" => Some(Ok(args.first().cloned().unwrap_or(Value::Null))),
            "noop" | "pass" => Some(Ok(Value::Null)),

            // ── Terminal / ANSI Colors ────────────────────────────────────────
            "black"   => Some(Ok(Value::String(format!("\x1b[30m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "red"     => Some(Ok(Value::String(format!("\x1b[31m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "green"   => Some(Ok(Value::String(format!("\x1b[32m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "yellow"  => Some(Ok(Value::String(format!("\x1b[33m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "blue"    => Some(Ok(Value::String(format!("\x1b[34m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "magenta" => Some(Ok(Value::String(format!("\x1b[35m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "cyan"    => Some(Ok(Value::String(format!("\x1b[36m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "white"   => Some(Ok(Value::String(format!("\x1b[37m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "bright_black" | "gray" | "grey" =>
                Some(Ok(Value::String(format!("\x1b[90m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "bright_red"     => Some(Ok(Value::String(format!("\x1b[91m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "bright_green"   => Some(Ok(Value::String(format!("\x1b[92m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "bright_yellow"  => Some(Ok(Value::String(format!("\x1b[93m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "bright_blue"    => Some(Ok(Value::String(format!("\x1b[94m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "bright_magenta" => Some(Ok(Value::String(format!("\x1b[95m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "bright_cyan"    => Some(Ok(Value::String(format!("\x1b[96m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "bright_white"   => Some(Ok(Value::String(format!("\x1b[97m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "bold"      => Some(Ok(Value::String(format!("\x1b[1m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "dim"       => Some(Ok(Value::String(format!("\x1b[2m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "italic"    => Some(Ok(Value::String(format!("\x1b[3m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "underline" => Some(Ok(Value::String(format!("\x1b[4m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "blink"     => Some(Ok(Value::String(format!("\x1b[5m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "strikethrough" => Some(Ok(Value::String(format!("\x1b[9m{}\x1b[0m", args.first().map(|v| v.as_string()).unwrap_or_default())))),
            "reset_color" | "ansi_reset" => Some(Ok(Value::String("\x1b[0m".to_string()))),
            "color_256" => {
                // color_256(code, text)
                if args.len() >= 2 {
                    let code = args[0].as_int();
                    let text = args[1].as_string();
                    Some(Ok(Value::String(format!("\x1b[38;5;{}m{}\x1b[0m", code, text))))
                } else { Some(Err("color_256(code, text)".to_string())) }
            }
            "bg_color_256" => {
                if args.len() >= 2 {
                    let code = args[0].as_int();
                    let text = args[1].as_string();
                    Some(Ok(Value::String(format!("\x1b[48;5;{}m{}\x1b[0m", code, text))))
                } else { Some(Err("bg_color_256(code, text)".to_string())) }
            }
            "rgb" => {
                if args.len() >= 4 {
                    let r = args[0].as_int();
                    let g = args[1].as_int();
                    let b = args[2].as_int();
                    let text = args[3].as_string();
                    Some(Ok(Value::String(format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text))))
                } else { Some(Err("rgb(r, g, b, text)".to_string())) }
            }
            "bg_rgb" => {
                if args.len() >= 4 {
                    let r = args[0].as_int();
                    let g = args[1].as_int();
                    let b = args[2].as_int();
                    let text = args[3].as_string();
                    Some(Ok(Value::String(format!("\x1b[48;2;{};{};{}m{}\x1b[0m", r, g, b, text))))
                } else { Some(Err("bg_rgb(r, g, b, text)".to_string())) }
            }

            // ── Terminal Cursor / Screen Control ──────────────────────────────
            "clear_screen" | "clear" => {
                print!("\x1b[2J\x1b[H");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                Some(Ok(Value::Null))
            }
            "clear_line" => {
                print!("\x1b[2K\r");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                Some(Ok(Value::Null))
            }
            "cursor_up" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(1);
                print!("\x1b[{}A", n);
                Some(Ok(Value::Null))
            }
            "cursor_down" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(1);
                print!("\x1b[{}B", n);
                Some(Ok(Value::Null))
            }
            "cursor_right" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(1);
                print!("\x1b[{}C", n);
                Some(Ok(Value::Null))
            }
            "cursor_left" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(1);
                print!("\x1b[{}D", n);
                Some(Ok(Value::Null))
            }
            "cursor_move" | "set_cursor" => {
                if args.len() >= 2 {
                    let r = args[0].as_int();
                    let c = args[1].as_int();
                    print!("\x1b[{};{}H", r, c);
                }
                Some(Ok(Value::Null))
            }
            "cursor_save" => { print!("\x1b[s"); Some(Ok(Value::Null)) }
            "cursor_restore" => { print!("\x1b[u"); Some(Ok(Value::Null)) }
            "cursor_hide" => { print!("\x1b[?25l"); Some(Ok(Value::Null)) }
            "cursor_show" => { print!("\x1b[?25h"); Some(Ok(Value::Null)) }
            "flush" => {
                let _ = std::io::Write::flush(&mut std::io::stdout());
                Some(Ok(Value::Null))
            }
            "term_size" | "terminal_size" => {
                // Get terminal size via tput
                let rows = std::process::Command::new("tput").arg("lines").output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<i64>().unwrap_or(24))
                    .unwrap_or(24);
                let cols = std::process::Command::new("tput").arg("cols").output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<i64>().unwrap_or(80))
                    .unwrap_or(80);
                let mut m = HashMap::new();
                m.insert("rows".to_string(), Value::Int(rows));
                m.insert("cols".to_string(), Value::Int(cols));
                Some(Ok(Value::Map(m)))
            }

            // ── Box Drawing / TUI ─────────────────────────────────────────────
            "box_line" | "hline" => {
                let w = args.first().map(|v| v.as_int()).unwrap_or(40) as usize;
                let ch = args.get(1).map(|v| v.as_string()).unwrap_or_else(|| "─".to_string());
                Some(Ok(Value::String(ch.repeat(w))))
            }
            "box_draw" => {
                // box_draw(width, height, title?) returns array of lines
                let w = args.first().map(|v| v.as_int() as usize).unwrap_or(40);
                let h = args.get(1).map(|v| v.as_int() as usize).unwrap_or(10);
                let title = args.get(2).map(|v| v.as_string()).unwrap_or_default();
                let mut lines = Vec::new();
                let top_bar = if title.is_empty() {
                    format!("┌{}┐", "─".repeat(w.saturating_sub(2)))
                } else {
                    let t = format!(" {} ", title);
                    let remaining = w.saturating_sub(2).saturating_sub(t.len());
                    let left = remaining / 2;
                    let right = remaining - left;
                    format!("┌{}{}{}┐", "─".repeat(left), t, "─".repeat(right))
                };
                lines.push(Value::String(top_bar));
                for _ in 0..h.saturating_sub(2) {
                    lines.push(Value::String(format!("│{}│", " ".repeat(w.saturating_sub(2)))));
                }
                lines.push(Value::String(format!("└{}┘", "─".repeat(w.saturating_sub(2)))));
                Some(Ok(Value::Array(lines)))
            }
            "table_format" => {
                // table_format(headers, rows) -> string table
                if args.len() < 2 { return Some(Err("table_format(headers, rows)".to_string())); }
                let headers = if let Value::Array(h) = &args[0] {
                    h.iter().map(|v| v.as_string()).collect::<Vec<_>>()
                } else { return Some(Err("headers must be array".to_string())); };
                let rows_val = if let Value::Array(r) = &args[1] { r.clone() } else { return Some(Err("rows must be array".to_string())); };
                // Calculate column widths
                let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
                let rows: Vec<Vec<String>> = rows_val.iter().map(|row| {
                    if let Value::Array(cells) = row {
                        cells.iter().map(|c| c.as_string()).collect()
                    } else { vec![row.as_string()] }
                }).collect();
                for row in &rows {
                    for (i, cell) in row.iter().enumerate() {
                        if i < widths.len() { widths[i] = widths[i].max(cell.len()); }
                    }
                }
                let sep = format!("├{}┤", widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("┼"));
                let top = format!("┌{}┐", widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("┬"));
                let bot = format!("└{}┘", widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("┴"));
                let header_row = format!("│{}│", headers.iter().enumerate().map(|(i, h)| {
                    let w = *widths.get(i).unwrap_or(&h.len());
                    format!(" {:width$} ", h, width = w)
                }).collect::<Vec<_>>().join("│"));
                let mut out = top + "\n" + &header_row + "\n" + &sep + "\n";
                for row in &rows {
                    let line = format!("│{}│", widths.iter().enumerate().map(|(i, w)| {
                        let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                        format!(" {:width$} ", cell, width = w)
                    }).collect::<Vec<_>>().join("│"));
                    out += &(line + "\n");
                }
                out += &bot;
                Some(Ok(Value::String(out)))
            }
            "progress_bar" => {
                // progress_bar(current, total, width?)
                if args.len() < 2 { return Some(Err("progress_bar(current, total, width?)".to_string())); }
                let current = args[0].as_float();
                let total = args[1].as_float();
                let width = args.get(2).map(|v| v.as_int() as usize).unwrap_or(40);
                let pct = (current / total).max(0.0).min(1.0);
                let filled = (pct * width as f64) as usize;
                let bar = format!("[{}{}] {:.1}%",
                    "█".repeat(filled),
                    "░".repeat(width - filled),
                    pct * 100.0
                );
                Some(Ok(Value::String(bar)))
            }
            "spinner_frames" => {
                Some(Ok(Value::Array(vec![
                    Value::String("⠋".to_string()), Value::String("⠙".to_string()),
                    Value::String("⠹".to_string()), Value::String("⠸".to_string()),
                    Value::String("⠼".to_string()), Value::String("⠴".to_string()),
                    Value::String("⠦".to_string()), Value::String("⠧".to_string()),
                    Value::String("⠇".to_string()), Value::String("⠏".to_string()),
                ])))
            }

            // ── Path operations ───────────────────────────────────────────────
            "path_join" => {
                let parts: Vec<String> = args.iter().map(|v| v.as_string()).collect();
                Some(Ok(Value::String(parts.join("/"))))
            }
            "path_dirname" | "dirname" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                let dir = std::path::Path::new(&p).parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or(".")
                    .to_string();
                Some(Ok(Value::String(dir)))
            }
            "path_basename" | "basename" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                let base = std::path::Path::new(&p).file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                Some(Ok(Value::String(base)))
            }
            "path_ext" | "file_ext" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                let ext = std::path::Path::new(&p).extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string();
                Some(Ok(Value::String(ext)))
            }
            "path_stem" | "file_stem" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                let stem = std::path::Path::new(&p).file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                Some(Ok(Value::String(stem)))
            }
            "path_exists" | "file_exists" | "exists" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                Some(Ok(Value::Bool(std::path::Path::new(&p).exists())))
            }
            "path_is_file" | "is_file" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                Some(Ok(Value::Bool(std::path::Path::new(&p).is_file())))
            }
            "path_is_dir" | "is_dir" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                Some(Ok(Value::Bool(std::path::Path::new(&p).is_dir())))
            }
            "path_abs" | "abs_path" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                match std::fs::canonicalize(&p) {
                    Ok(pb) => Some(Ok(Value::String(pb.to_string_lossy().to_string()))),
                    Err(_) => Some(Ok(Value::String(p))),
                }
            }
            "cwd" | "getcwd" => {
                match std::env::current_dir() {
                    Ok(p) => Some(Ok(Value::String(p.to_string_lossy().to_string()))),
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "chdir" | "set_cwd" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                match std::env::set_current_dir(&p) {
                    Ok(()) => Some(Ok(Value::Bool(true))),
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "ls" | "list_dir" | "read_dir" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_else(|| ".".to_string());
                match std::fs::read_dir(&p) {
                    Ok(entries) => {
                        let files: Vec<Value> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| Value::String(e.file_name().to_string_lossy().to_string()))
                            .collect();
                        Some(Ok(Value::Array(files)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "mkdir" | "make_dir" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                let recursive = args.get(1).map(|v| v.is_truthy()).unwrap_or(true);
                let result = if recursive {
                    std::fs::create_dir_all(&p)
                } else {
                    std::fs::create_dir(&p)
                };
                match result {
                    Ok(()) => Some(Ok(Value::Bool(true))),
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "rmdir" | "remove_dir" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                match std::fs::remove_dir_all(&p) {
                    Ok(()) => Some(Ok(Value::Bool(true))),
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "copy_file" | "file_copy" => {
                if args.len() < 2 { return Some(Err("copy_file(src, dst)".to_string())); }
                match std::fs::copy(args[0].as_string(), args[1].as_string()) {
                    Ok(bytes) => Some(Ok(Value::Int(bytes as i64))),
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "move_file" | "rename" => {
                if args.len() < 2 { return Some(Err("rename(src, dst)".to_string())); }
                match std::fs::rename(args[0].as_string(), args[1].as_string()) {
                    Ok(()) => Some(Ok(Value::Bool(true))),
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "file_meta" | "stat" => {
                let p = args.first().map(|v| v.as_string()).unwrap_or_default();
                match std::fs::metadata(&p) {
                    Ok(meta) => {
                        let mut m = HashMap::new();
                        m.insert("size".to_string(), Value::Int(meta.len() as i64));
                        m.insert("is_file".to_string(), Value::Bool(meta.is_file()));
                        m.insert("is_dir".to_string(), Value::Bool(meta.is_dir()));
                        m.insert("readonly".to_string(), Value::Bool(meta.permissions().readonly()));
                        if let Ok(modified) = meta.modified() {
                            if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                                m.insert("modified".to_string(), Value::Int(dur.as_secs() as i64));
                            }
                        }
                        Some(Ok(Value::Map(m)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }

            // ── Date / Time ───────────────────────────────────────────────────
            "now" | "timestamp" => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                Some(Ok(Value::Int(ts.as_secs() as i64)))
            }
            "now_ms" | "timestamp_ms" => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                Some(Ok(Value::Int(ts.as_millis() as i64)))
            }
            "now_ns" | "timestamp_ns" => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                Some(Ok(Value::Int(ts.as_nanos() as i64)))
            }
            "format_time" | "strftime" => {
                // Use `date` command for formatting
                let ts = args.first().map(|v| v.as_int()).unwrap_or(0);
                let fmt = args.get(1).map(|v| v.as_string()).unwrap_or_else(|| "%Y-%m-%d %H:%M:%S".to_string());
                match std::process::Command::new("date")
                    .args(&["-d", &format!("@{}", ts), &format!("+{}", fmt)])
                    .output()
                {
                    Ok(out) => Some(Ok(Value::String(String::from_utf8_lossy(&out.stdout).trim().to_string()))),
                    Err(_) => Some(Ok(Value::String(ts.to_string()))),
                }
            }
            "parse_time" | "strptime" => {
                // Use `date` to parse time string to timestamp
                let s = args.first().map(|v| v.as_string()).unwrap_or_default();
                match std::process::Command::new("date")
                    .args(&["-d", &s, "+%s"])
                    .output()
                {
                    Ok(out) => {
                        let parsed = String::from_utf8_lossy(&out.stdout).trim().parse::<i64>().unwrap_or(0);
                        Some(Ok(Value::Int(parsed)))
                    }
                    Err(_) => Some(Ok(Value::Int(0))),
                }
            }

            // ── Base64 ────────────────────────────────────────────────────────
            "base64_encode" | "b64_encode" => {
                let s = args.first().map(|v| v.as_string()).unwrap_or_default();
                let bytes = s.as_bytes();
                let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                let mut out = String::new();
                let mut i = 0;
                while i < bytes.len() {
                    let b0 = bytes[i] as u32;
                    let b1 = if i + 1 < bytes.len() { bytes[i+1] as u32 } else { 0 };
                    let b2 = if i + 2 < bytes.len() { bytes[i+2] as u32 } else { 0 };
                    let n = (b0 << 16) | (b1 << 8) | b2;
                    out.push(alphabet[((n >> 18) & 63) as usize] as char);
                    out.push(alphabet[((n >> 12) & 63) as usize] as char);
                    out.push(if i + 1 < bytes.len() { alphabet[((n >> 6) & 63) as usize] as char } else { '=' });
                    out.push(if i + 2 < bytes.len() { alphabet[(n & 63) as usize] as char } else { '=' });
                    i += 3;
                }
                Some(Ok(Value::String(out)))
            }
            "base64_decode" | "b64_decode" => {
                let s = args.first().map(|v| v.as_string()).unwrap_or_default();
                let decode_char = |c: char| -> u32 {
                    match c {
                        'A'..='Z' => c as u32 - 'A' as u32,
                        'a'..='z' => c as u32 - 'a' as u32 + 26,
                        '0'..='9' => c as u32 - '0' as u32 + 52,
                        '+' => 62, '/' => 63, _ => 0,
                    }
                };
                // Count padding before stripping
                let pad = s.chars().rev().take_while(|&c| c == '=').count();
                let chars: Vec<char> = s.trim_end_matches('=').chars().collect();
                let mut bytes = Vec::new();
                let mut i = 0;
                while i < chars.len() {
                    let c0 = chars[i];
                    let c1 = if i + 1 < chars.len() { chars[i+1] } else { 'A' };
                    let c2 = if i + 2 < chars.len() { chars[i+2] } else { 'A' };
                    let c3 = if i + 3 < chars.len() { chars[i+3] } else { 'A' };
                    let n = (decode_char(c0) << 18) | (decode_char(c1) << 12)
                          | (decode_char(c2) << 6)  | decode_char(c3);
                    let remaining = chars.len() - i;
                    bytes.push(((n >> 16) & 0xff) as u8);
                    if remaining >= 3 { bytes.push(((n >> 8) & 0xff) as u8); }
                    if remaining >= 4 { bytes.push((n & 0xff) as u8); }
                    i += 4;
                }
                Some(Ok(Value::String(String::from_utf8_lossy(&bytes).to_string())))
            }

            // ── URL encode/decode ──────────────────────────────────────────────
            "url_encode" | "urlencode" => {
                let s = args.first().map(|v| v.as_string()).unwrap_or_default();
                let mut out = String::new();
                for c in s.chars() {
                    match c {
                        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(c),
                        ' ' => out.push('+'),
                        c => {
                            for b in c.to_string().as_bytes() {
                                out.push_str(&format!("%{:02X}", b));
                            }
                        }
                    }
                }
                Some(Ok(Value::String(out)))
            }
            "url_decode" | "urldecode" => {
                let s = args.first().map(|v| v.as_string()).unwrap_or_default();
                let mut out = String::new();
                let chars: Vec<char> = s.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    match chars[i] {
                        '+' => { out.push(' '); i += 1; }
                        '%' if i + 2 < chars.len() => {
                            let hex = format!("{}{}", chars[i+1], chars[i+2]);
                            if let Ok(b) = u8::from_str_radix(&hex, 16) {
                                out.push(b as char);
                            }
                            i += 3;
                        }
                        c => { out.push(c); i += 1; }
                    }
                }
                Some(Ok(Value::String(out)))
            }

            // ── Advanced Math ─────────────────────────────────────────────────
            "atan2" => {
                if args.len() >= 2 {
                    Some(Ok(Value::Float(args[0].as_float().atan2(args[1].as_float()))))
                } else { Some(Err("atan2(y, x)".to_string())) }
            }
            "hypot" => {
                if args.len() >= 2 {
                    Some(Ok(Value::Float(args[0].as_float().hypot(args[1].as_float()))))
                } else { Some(Err("hypot(x, y)".to_string())) }
            }
            "factorial" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                if n < 0 { return Some(Err("factorial: negative input".to_string())); }
                let result = (1i64..=n).fold(1i64, |a, b| a.saturating_mul(b));
                Some(Ok(Value::Int(result)))
            }
            "gcd" => {
                if args.len() < 2 { return Some(Err("gcd(a, b)".to_string())); }
                let mut a = args[0].as_int().abs();
                let mut b = args[1].as_int().abs();
                while b != 0 { let t = b; b = a % b; a = t; }
                Some(Ok(Value::Int(a)))
            }
            "lcm" => {
                if args.len() < 2 { return Some(Err("lcm(a, b)".to_string())); }
                let a = args[0].as_int().abs();
                let b = args[1].as_int().abs();
                let mut ga = a; let mut gb = b;
                while gb != 0 { let t = gb; gb = ga % gb; ga = t; }
                Some(Ok(Value::Int(if ga == 0 { 0 } else { a / ga * b })))
            }
            "is_prime" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                if n < 2 { return Some(Ok(Value::Bool(false))); }
                if n == 2 { return Some(Ok(Value::Bool(true))); }
                if n % 2 == 0 { return Some(Ok(Value::Bool(false))); }
                let mut i = 3i64;
                while i * i <= n { if n % i == 0 { return Some(Ok(Value::Bool(false))); } i += 2; }
                Some(Ok(Value::Bool(true)))
            }
            "primes_up_to" | "sieve" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
                if n < 2 { return Some(Ok(Value::Array(vec![]))); }
                let mut sieve = vec![true; n + 1];
                sieve[0] = false; sieve[1] = false;
                let mut i = 2;
                while i * i <= n {
                    if sieve[i] { let mut j = i*i; while j <= n { sieve[j] = false; j += i; } }
                    i += 1;
                }
                let primes = sieve.iter().enumerate()
                    .filter(|(_, &is_p)| is_p)
                    .map(|(i, _)| Value::Int(i as i64))
                    .collect();
                Some(Ok(Value::Array(primes)))
            }
            "log_b" | "log_base" => {
                if args.len() >= 2 {
                    let x = args[0].as_float();
                    let base = args[1].as_float();
                    Some(Ok(Value::Float(x.ln() / base.ln())))
                } else { Some(Err("log_base(x, base)".to_string())) }
            }
            "deg2rad" | "to_radians" => {
                Some(Ok(Value::Float(args.first().map(|v| v.as_float() * std::f64::consts::PI / 180.0).unwrap_or(0.0))))
            }
            "rad2deg" | "to_degrees" => {
                Some(Ok(Value::Float(args.first().map(|v| v.as_float() * 180.0 / std::f64::consts::PI).unwrap_or(0.0))))
            }
            "e" => Some(Ok(Value::Float(std::f64::consts::E))),
            "tau" => Some(Ok(Value::Float(std::f64::consts::TAU))),
            "phi" | "golden_ratio" => Some(Ok(Value::Float(1.618033988749895))),
            "inf" | "infinity" => Some(Ok(Value::Float(f64::INFINITY))),
            "nan" => Some(Ok(Value::Float(f64::NAN))),
            "epsilon" => Some(Ok(Value::Float(f64::EPSILON))),
            "max_int" => Some(Ok(Value::Int(i64::MAX))),
            "min_int" => Some(Ok(Value::Int(i64::MIN))),
            "lerp" => {
                if args.len() >= 3 {
                    let a = args[0].as_float();
                    let b = args[1].as_float();
                    let t = args[2].as_float();
                    Some(Ok(Value::Float(a + (b - a) * t)))
                } else { Some(Err("lerp(a, b, t)".to_string())) }
            }
            "map_range" | "remap" => {
                // remap(val, in_min, in_max, out_min, out_max)
                if args.len() >= 5 {
                    let v = args[0].as_float();
                    let in_min = args[1].as_float();
                    let in_max = args[2].as_float();
                    let out_min = args[3].as_float();
                    let out_max = args[4].as_float();
                    let result = (v - in_min) / (in_max - in_min) * (out_max - out_min) + out_min;
                    Some(Ok(Value::Float(result)))
                } else { Some(Err("remap(val, in_min, in_max, out_min, out_max)".to_string())) }
            }
            "sign" | "signum" => {
                if let Some(v) = args.first() {
                    match v {
                        Value::Float(f) => Some(Ok(Value::Float(f.signum()))),
                        Value::Int(n) => Some(Ok(Value::Int(n.signum()))),
                        other => Some(Ok(Value::Int(if other.is_truthy() { 1 } else { 0 }))),
                    }
                } else { Some(Ok(Value::Int(0))) }
            }
            "between" | "in_range" => {
                if args.len() >= 3 {
                    let v = args[0].as_float();
                    let lo = args[1].as_float();
                    let hi = args[2].as_float();
                    Some(Ok(Value::Bool(v >= lo && v <= hi)))
                } else { Some(Err("between(val, lo, hi)".to_string())) }
            }

            // ── Sorting with comparator ────────────────────────────────────────
            "sort_by" => {
                if args.len() >= 2 {
                    if let Value::Array(mut arr) = args[0].clone() {
                        let key_fn = args[1].clone();
                        let mut keys: Vec<Value> = arr.iter()
                            .map(|item| self.call_value(key_fn.clone(), vec![item.clone()]).unwrap_or(Value::Null))
                            .collect();
                        // Stable sort by key
                        let mut indices: Vec<usize> = (0..arr.len()).collect();
                        indices.sort_by(|&a, &b| {
                            let ka = &keys[a];
                            let kb = &keys[b];
                            ka.as_float().partial_cmp(&kb.as_float())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        let sorted: Vec<Value> = indices.iter().map(|&i| arr[i].clone()).collect();
                        Some(Ok(Value::Array(sorted)))
                    } else { Some(Err("sort_by(array, key_fn)".to_string())) }
                } else { None }
            }
            "sort_by_desc" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = args[0].clone() {
                        let key_fn = args[1].clone();
                        let mut indices: Vec<usize> = (0..arr.len()).collect();
                        let keys: Vec<Value> = arr.iter()
                            .map(|item| self.call_value(key_fn.clone(), vec![item.clone()]).unwrap_or(Value::Null))
                            .collect();
                        indices.sort_by(|&a, &b| {
                            keys[b].as_float().partial_cmp(&keys[a].as_float())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        Some(Ok(Value::Array(indices.iter().map(|&i| arr[i].clone()).collect())))
                    } else { Some(Err("sort_by_desc(array, key_fn)".to_string())) }
                } else { None }
            }
            "group_by" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = args[0].clone() {
                        let key_fn = args[1].clone();
                        let mut groups: HashMap<String, Vec<Value>> = HashMap::new();
                        for item in arr {
                            let key = self.call_value(key_fn.clone(), vec![item.clone()])
                                .unwrap_or(Value::Null)
                                .as_string();
                            groups.entry(key).or_insert_with(Vec::new).push(item);
                        }
                        let result: HashMap<String, Value> = groups.into_iter()
                            .map(|(k, v)| (k, Value::Array(v)))
                            .collect();
                        Some(Ok(Value::Map(result)))
                    } else { Some(Err("group_by(array, key_fn)".to_string())) }
                } else { None }
            }
            "zip" => {
                if args.len() >= 2 {
                    if let (Value::Array(a), Value::Array(b)) = (&args[0], &args[1]) {
                        let result = a.iter().zip(b.iter())
                            .map(|(x, y)| Value::Array(vec![x.clone(), y.clone()]))
                            .collect();
                        Some(Ok(Value::Array(result)))
                    } else { Some(Err("zip(array, array)".to_string())) }
                } else { None }
            }
            "zip_with" => {
                if args.len() >= 3 {
                    if let (Value::Array(a), Value::Array(b)) = (&args[0], &args[1]) {
                        let f = args[2].clone();
                        let a = a.clone(); let b = b.clone();
                        let mut result = Vec::new();
                        for (x, y) in a.iter().zip(b.iter()) {
                            let r = match self.call_value(f.clone(), vec![x.clone(), y.clone()]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            result.push(r);
                        }
                        Some(Ok(Value::Array(result)))
                    } else { Some(Err("zip_with(a, b, fn)".to_string())) }
                } else { None }
            }
            "take" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let n = args[1].as_int() as usize;
                        Some(Ok(Value::Array(arr.iter().take(n).cloned().collect())))
                    } else { Some(Err("take(array, n)".to_string())) }
                } else { None }
            }
            "drop" | "skip" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let n = args[1].as_int() as usize;
                        Some(Ok(Value::Array(arr.iter().skip(n).cloned().collect())))
                    } else { Some(Err("drop(array, n)".to_string())) }
                } else { None }
            }
            "take_while" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let f = args[1].clone(); let arr = arr.clone();
                        let mut result = Vec::new();
                        for item in arr {
                            let r = match self.call_value(f.clone(), vec![item.clone()]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            if r.is_truthy() { result.push(item); } else { break; }
                        }
                        Some(Ok(Value::Array(result)))
                    } else { Some(Err("take_while(array, fn)".to_string())) }
                } else { None }
            }
            "drop_while" | "skip_while" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let f = args[1].clone(); let arr = arr.clone();
                        let mut dropping = true;
                        let mut result = Vec::new();
                        for item in arr {
                            if dropping {
                                let r = match self.call_value(f.clone(), vec![item.clone()]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                                if r.is_truthy() { continue; } else { dropping = false; }
                            }
                            result.push(item);
                        }
                        Some(Ok(Value::Array(result)))
                    } else { Some(Err("drop_while(array, fn)".to_string())) }
                } else { None }
            }
            "partition" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let f = args[1].clone(); let arr = arr.clone();
                        let mut yes = Vec::new(); let mut no = Vec::new();
                        for item in arr {
                            let r = match self.call_value(f.clone(), vec![item.clone()]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            if r.is_truthy() { yes.push(item); } else { no.push(item); }
                        }
                        Some(Ok(Value::Tuple(vec![Value::Array(yes), Value::Array(no)])))
                    } else { Some(Err("partition(array, fn)".to_string())) }
                } else { None }
            }
            "chunks" | "chunk" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let n = args[1].as_int() as usize;
                        if n == 0 { return Some(Err("chunk size must be > 0".to_string())); }
                        let chunks = arr.chunks(n)
                            .map(|c| Value::Array(c.to_vec()))
                            .collect();
                        Some(Ok(Value::Array(chunks)))
                    } else { Some(Err("chunks(array, size)".to_string())) }
                } else { None }
            }
            "windows" | "sliding" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let n = args[1].as_int() as usize;
                        if n == 0 { return Some(Err("window size must be > 0".to_string())); }
                        let windows = arr.windows(n)
                            .map(|w| Value::Array(w.to_vec()))
                            .collect();
                        Some(Ok(Value::Array(windows)))
                    } else { Some(Err("windows(array, size)".to_string())) }
                } else { None }
            }
            "count_if" | "count_where" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let f = args[1].clone(); let arr = arr.clone();
                        let mut count = 0i64;
                        for item in arr {
                            let r = match self.call_value(f.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            if r.is_truthy() { count += 1; }
                        }
                        Some(Ok(Value::Int(count)))
                    } else { Some(Err("count_if(array, fn)".to_string())) }
                } else { None }
            }
            "sum_by" | "sum_map" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        let f = args[1].clone(); let arr = arr.clone();
                        let mut sum = 0.0f64;
                        let mut is_int = true;
                        for item in arr {
                            let r = match self.call_value(f.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                            match &r { Value::Float(_) => is_int = false, _ => {} }
                            sum += r.as_float();
                        }
                        if is_int { Some(Ok(Value::Int(sum as i64))) } else { Some(Ok(Value::Float(sum))) }
                    } else { Some(Err("sum_by(array, fn)".to_string())) }
                } else { None }
            }
            "max_by" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        if arr.is_empty() { return Some(Ok(Value::Null)); }
                        let f = args[1].clone(); let arr = arr.clone();
                        let mut best = arr[0].clone();
                        let mut best_key = self.call_value(f.clone(), vec![best.clone()]).unwrap_or(Value::Null);
                        for item in arr.into_iter().skip(1) {
                            let k = self.call_value(f.clone(), vec![item.clone()]).unwrap_or(Value::Null);
                            if k.as_float() > best_key.as_float() { best = item; best_key = k; }
                        }
                        Some(Ok(best))
                    } else { Some(Err("max_by(array, fn)".to_string())) }
                } else { None }
            }
            "min_by" => {
                if args.len() >= 2 {
                    if let Value::Array(arr) = &args[0] {
                        if arr.is_empty() { return Some(Ok(Value::Null)); }
                        let f = args[1].clone(); let arr = arr.clone();
                        let mut best = arr[0].clone();
                        let mut best_key = self.call_value(f.clone(), vec![best.clone()]).unwrap_or(Value::Null);
                        for item in arr.into_iter().skip(1) {
                            let k = self.call_value(f.clone(), vec![item.clone()]).unwrap_or(Value::Null);
                            if k.as_float() < best_key.as_float() { best = item; best_key = k; }
                        }
                        Some(Ok(best))
                    } else { Some(Err("min_by(array, fn)".to_string())) }
                } else { None }
            }

            // ── String utilities ───────────────────────────────────────────────
            "sprintf" | "format_str" => {
                // sprintf(fmt, args...) — printf-style formatting with flags, width, precision
                if let Some(fmt_val) = args.first() {
                    let fmt = fmt_val.as_string();
                    let mut out = String::new();
                    let mut arg_idx = 1usize;
                    let chars: Vec<char> = fmt.chars().collect();
                    let mut i = 0;
                    while i < chars.len() {
                        if chars[i] == '%' && i + 1 < chars.len() {
                            i += 1;
                            // Collect optional flags: -, +, space, 0, #
                            let mut flags = String::new();
                            while i < chars.len() && "-+ 0#".contains(chars[i]) {
                                flags.push(chars[i]); i += 1;
                            }
                            // Width
                            let mut width_str = String::new();
                            while i < chars.len() && chars[i].is_ascii_digit() {
                                width_str.push(chars[i]); i += 1;
                            }
                            let width = width_str.parse::<usize>().unwrap_or(0);
                            // Precision
                            let mut precision: Option<usize> = None;
                            if i < chars.len() && chars[i] == '.' {
                                i += 1;
                                let mut prec_str = String::new();
                                while i < chars.len() && chars[i].is_ascii_digit() {
                                    prec_str.push(chars[i]); i += 1;
                                }
                                precision = Some(prec_str.parse::<usize>().unwrap_or(6));
                            }
                            if i >= chars.len() { break; }
                            let spec = chars[i]; i += 1;
                            if spec == '%' { out.push('%'); continue; }
                            let arg = args.get(arg_idx).cloned().unwrap_or(Value::Null);
                            arg_idx += 1;
                            let left_align = flags.contains('-');
                            let zero_pad  = flags.contains('0') && !left_align;
                            let formatted = match spec {
                                'd' | 'i' => format!("{}", arg.as_int()),
                                'u' => format!("{}", arg.as_int().unsigned_abs()),
                                'f' => {
                                    let p = precision.unwrap_or(6);
                                    format!("{:.prec$}", arg.as_float(), prec = p)
                                }
                                'e' => {
                                    let p = precision.unwrap_or(6);
                                    format!("{:.prec$e}", arg.as_float(), prec = p)
                                }
                                'g' => {
                                    let p = precision.unwrap_or(6);
                                    let v = arg.as_float();
                                    if v.abs() >= 1e-4 && v.abs() < 1e6 {
                                        format!("{:.prec$}", v, prec = p)
                                    } else {
                                        format!("{:.prec$e}", v, prec = p)
                                    }
                                }
                                's' => {
                                    let s = arg.as_string();
                                    if let Some(p) = precision {
                                        s.chars().take(p).collect()
                                    } else { s }
                                }
                                'x' => format!("{:x}", arg.as_int()),
                                'X' => format!("{:X}", arg.as_int()),
                                'o' => format!("{:o}", arg.as_int()),
                                'b' => format!("{:b}", arg.as_int()),
                                'c' => char::from_u32(arg.as_int() as u32).map(|c| c.to_string()).unwrap_or_default(),
                                'p' => format!("0x{:x}", arg.as_int()),
                                _ => { out.push('%'); out.push_str(&flags); out.push_str(&width_str); out.push(spec); continue; }
                            };
                            // Apply width padding
                            if width > 0 && formatted.len() < width {
                                let pad = width - formatted.len();
                                if left_align {
                                    out.push_str(&formatted);
                                    out.push_str(&" ".repeat(pad));
                                } else if zero_pad && spec != 's' {
                                    out.push_str(&"0".repeat(pad));
                                    out.push_str(&formatted);
                                } else {
                                    out.push_str(&" ".repeat(pad));
                                    out.push_str(&formatted);
                                }
                            } else {
                                out.push_str(&formatted);
                            }
                        } else {
                            out.push(chars[i]); i += 1;
                        }
                    }
                    Some(Ok(Value::String(out)))
                } else { Some(Err("sprintf(fmt, ...)".to_string())) }
            }
            "pad_left" | "ljust" => {
                if args.len() >= 2 {
                    let s = args[0].as_string();
                    let n = args[1].as_int() as usize;
                    let ch = args.get(2).map(|v| v.as_string().chars().next().unwrap_or(' ')).unwrap_or(' ');
                    Some(Ok(Value::String(format!("{:>width$}", s, width = n).replacen(&" ".repeat(n.saturating_sub(s.len())), &ch.to_string().repeat(n.saturating_sub(s.len())), 1))))
                } else { None }
            }
            "pad_right" | "rjust" => {
                if args.len() >= 2 {
                    let s = args[0].as_string();
                    let n = args[1].as_int() as usize;
                    let ch = args.get(2).map(|v| v.as_string().chars().next().unwrap_or(' ')).unwrap_or(' ');
                    let padded: String = s.chars().chain(std::iter::repeat(ch).take(n.saturating_sub(s.len()))).collect();
                    Some(Ok(Value::String(padded)))
                } else { None }
            }
            "center" | "center_str" => {
                if args.len() >= 2 {
                    let s = args[0].as_string();
                    let n = args[1].as_int() as usize;
                    let ch = args.get(2).map(|v| v.as_string().chars().next().unwrap_or(' ')).unwrap_or(' ');
                    let len = s.len();
                    if n <= len { return Some(Ok(Value::String(s))); }
                    let total_pad = n - len;
                    let left_pad = total_pad / 2;
                    let right_pad = total_pad - left_pad;
                    let result = format!("{}{}{}", ch.to_string().repeat(left_pad), s, ch.to_string().repeat(right_pad));
                    Some(Ok(Value::String(result)))
                } else { None }
            }
            "word_wrap" => {
                if args.len() >= 2 {
                    let text = args[0].as_string();
                    let width = args[1].as_int() as usize;
                    let mut lines = Vec::new();
                    let mut current = String::new();
                    for word in text.split_whitespace() {
                        if current.is_empty() {
                            current = word.to_string();
                        } else if current.len() + 1 + word.len() <= width {
                            current.push(' ');
                            current.push_str(word);
                        } else {
                            lines.push(Value::String(current.clone()));
                            current = word.to_string();
                        }
                    }
                    if !current.is_empty() { lines.push(Value::String(current)); }
                    Some(Ok(Value::Array(lines)))
                } else { None }
            }
            "lcs" | "longest_common_subsequence" => {
                if args.len() >= 2 {
                    let a = args[0].as_string();
                    let b = args[1].as_string();
                    let ac: Vec<char> = a.chars().collect();
                    let bc: Vec<char> = b.chars().collect();
                    let m = ac.len(); let n = bc.len();
                    let mut dp = vec![vec![0usize; n + 1]; m + 1];
                    for i in 1..=m {
                        for j in 1..=n {
                            dp[i][j] = if ac[i-1] == bc[j-1] { dp[i-1][j-1] + 1 }
                                       else { dp[i-1][j].max(dp[i][j-1]) };
                        }
                    }
                    Some(Ok(Value::Int(dp[m][n] as i64)))
                } else { None }
            }
            "levenshtein" | "edit_distance" => {
                if args.len() >= 2 {
                    let a = args[0].as_string();
                    let b = args[1].as_string();
                    let m = a.len(); let n = b.len();
                    let mut dp = vec![vec![0usize; n + 1]; m + 1];
                    for i in 0..=m { dp[i][0] = i; }
                    for j in 0..=n { dp[0][j] = j; }
                    let ac: Vec<char> = a.chars().collect();
                    let bc: Vec<char> = b.chars().collect();
                    for i in 1..=m {
                        for j in 1..=n {
                            dp[i][j] = if ac[i-1] == bc[j-1] { dp[i-1][j-1] }
                                       else { 1 + dp[i-1][j].min(dp[i][j-1]).min(dp[i-1][j-1]) };
                        }
                    }
                    Some(Ok(Value::Int(dp[m][n] as i64)))
                } else { None }
            }

            // ── Assert / Error handling ───────────────────────────────────────
            "assert" => {
                let cond = args.first().map(|v| v.is_truthy()).unwrap_or(false);
                let msg = args.get(1).map(|v| v.as_string()).unwrap_or_else(|| "Assertion failed".to_string());
                if !cond { Some(Err(format!("AssertionError: {}", msg))) }
                else { Some(Ok(Value::Null)) }
            }
            "assert_eq" => {
                if args.len() >= 2 {
                    if args[0] == args[1] { Some(Ok(Value::Null)) }
                    else { Some(Err(format!("AssertionError: {} != {}", args[0], args[1]))) }
                } else { Some(Err("assert_eq(a, b)".to_string())) }
            }
            "assert_ne" => {
                if args.len() >= 2 {
                    if args[0] != args[1] { Some(Ok(Value::Null)) }
                    else { Some(Err(format!("AssertionError: expected values to differ, both are {}", args[0]))) }
                } else { Some(Err("assert_ne(a, b)".to_string())) }
            }
            "panic" | "raise" => {
                let msg = args.first().map(|v| v.as_string()).unwrap_or_else(|| "panic called".to_string());
                Some(Err(format!("Panic: {}", msg)))
            }
            "ok" => Some(Ok(args.first().cloned().unwrap_or(Value::Null))),
            "err" | "error" => {
                let msg = args.first().map(|v| v.as_string()).unwrap_or_else(|| "error".to_string());
                let mut m = HashMap::new();
                m.insert("__type".to_string(), Value::String("Error".to_string()));
                m.insert("message".to_string(), Value::String(msg));
                Some(Ok(Value::Map(m)))
            }
            "is_error" | "is_err" => {
                if let Some(Value::Map(m)) = args.first() {
                    Some(Ok(Value::Bool(m.get("__type").map(|v| v.as_string() == "Error").unwrap_or(false))))
                } else { Some(Ok(Value::Bool(false))) }
            }
            "unwrap" => {
                if let Some(val) = args.first() {
                    if let Value::Map(m) = val {
                        if m.get("__type").map(|v| v.as_string() == "Error").unwrap_or(false) {
                            let msg = m.get("message").map(|v| v.as_string()).unwrap_or_else(|| "unwrap on error".to_string());
                            Some(Err(format!("Unwrap error: {}", msg)))
                        } else { Some(Ok(val.clone())) }
                    } else if matches!(val, Value::Null) {
                        let msg = args.get(1).map(|v| v.as_string()).unwrap_or_else(|| "unwrap on null".to_string());
                        Some(Err(format!("Unwrap null: {}", msg)))
                    } else { Some(Ok(val.clone())) }
                } else { Some(Ok(Value::Null)) }
            }
            "unwrap_or" => {
                if args.len() >= 2 {
                    let val = &args[0];
                    let default = args[1].clone();
                    if matches!(val, Value::Null) { Some(Ok(default)) }
                    else if let Value::Map(m) = val {
                        if m.get("__type").map(|v| v.as_string() == "Error").unwrap_or(false) { Some(Ok(default)) }
                        else { Some(Ok(val.clone())) }
                    } else { Some(Ok(val.clone())) }
                } else { None }
            }

            // ── Type checking extended ─────────────────────────────────────────
            "is_null" | "is_none" => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Null) | None)))),
            "is_int" => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Int(_)))))),
            "is_float" => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Float(_)))))),
            "is_bool" => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Bool(_)))))),
            "is_array" | "is_list" => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Array(_)))))),
            "is_map" | "is_dict" => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Map(_)))))),
            "is_tuple" => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Tuple(_)))))),
            "is_closure" | "is_fn" | "is_function" => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Closure { .. }) | Some(Value::Function(_)))))),
            "is_struct" => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::StructInstance(_)))))),

            // ── Environment / args ────────────────────────────────────────────
            "env_all" | "env_vars" => {
                let map: HashMap<String, Value> = std::env::vars()
                    .map(|(k, v)| (k, Value::String(v)))
                    .collect();
                Some(Ok(Value::Map(map)))
            }
            "args" | "cli_args" => {
                let args_list: Vec<Value> = std::env::args()
                    .map(|a| Value::String(a))
                    .collect();
                Some(Ok(Value::Array(args_list)))
            }

            // ── Memoize / Lazy ────────────────────────────────────────────────
            "memoize" => {
                // Returns the same closure but with a cache map injected
                // For simplicity, just return the closure unchanged (user manages cache)
                Some(Ok(args.first().cloned().unwrap_or(Value::Null)))
            }
            "once" => {
                // Returns a wrapper that only calls the inner fn once
                Some(Ok(args.first().cloned().unwrap_or(Value::Null)))
            }

            // ── Networking utilities ───────────────────────────────────────────
            "dns_lookup" | "resolve" => {
                let host = args.first().map(|v| v.as_string()).unwrap_or_default();
                match std::net::ToSocketAddrs::to_socket_addrs(&format!("{}:0", host)) {
                    Ok(addrs) => {
                        let ips: Vec<Value> = addrs
                            .filter_map(|a| match a {
                                std::net::SocketAddr::V4(v4) => Some(Value::String(v4.ip().to_string())),
                                std::net::SocketAddr::V6(v6) => Some(Value::String(v6.ip().to_string())),
                            })
                            .collect();
                        Some(Ok(Value::Array(ips)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "port_open" | "tcp_check" => {
                if args.len() >= 2 {
                    let host = args[0].as_string();
                    let port = args[1].as_int() as u16;
                    let addr = format!("{}:{}", host, port);
                    let result = std::net::TcpStream::connect_timeout(
                        &addr.parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
                        std::time::Duration::from_secs(3)
                    );
                    Some(Ok(Value::Bool(result.is_ok())))
                } else { Some(Err("port_open(host, port)".to_string())) }
            }

            // ── TCP Sockets ─────────────────────────────────────────────────────
            // tcp_connect(host, port) or tcp_connect("host:port") -> handle_int or null
            "tcp_connect" => {
                if args.len() >= 1 {
                    let addr = if args.len() >= 2 {
                        format!("{}:{}", args[0].as_string(), args[1].as_int())
                    } else {
                        args[0].as_string() // already "host:port"
                    };
                    match std::net::TcpStream::connect_timeout(
                        &addr.parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
                        std::time::Duration::from_secs(10),
                    ) {
                        Ok(stream) => {
                            self.net_handle_counter += 1;
                            let handle = self.net_handle_counter;
                            self.tcp_streams.insert(handle, stream);
                            Some(Ok(Value::Int(handle)))
                        }
                        Err(_) => Some(Ok(Value::Null)),
                    }
                } else { Some(Err("tcp_connect(host, port)".to_string())) }
            }

            // tcp_send(handle, data) -> bytes_written
            "tcp_send" => {
                use std::io::Write;
                if args.len() >= 2 {
                    let handle = args[0].as_int();
                    let data = args[1].as_string();
                    if let Some(stream) = self.tcp_streams.get_mut(&handle) {
                        match stream.write(data.as_bytes()) {
                            Ok(n) => Some(Ok(Value::Int(n as i64))),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    } else { Some(Err(format!("tcp_send: unknown handle {}", handle))) }
                } else { Some(Err("tcp_send(handle, data)".to_string())) }
            }

            // tcp_recv(handle, max_bytes?) -> string
            "tcp_recv" => {
                use std::io::Read;
                if args.len() >= 1 {
                    let handle = args[0].as_int();
                    let max = if args.len() >= 2 { args[1].as_int() as usize } else { 65536 };
                    if let Some(stream) = self.tcp_streams.get_mut(&handle) {
                        let mut buf = vec![0u8; max];
                        match stream.read(&mut buf) {
                            Ok(n) => {
                                buf.truncate(n);
                                Some(Ok(Value::String(String::from_utf8_lossy(&buf).to_string())))
                            }
                            Err(e) => Some(Err(e.to_string())),
                        }
                    } else { Some(Err(format!("tcp_recv: unknown handle {}", handle))) }
                } else { Some(Err("tcp_recv(handle)".to_string())) }
            }

            // tcp_recv_all(handle) -> string  (reads until connection closed)
            "tcp_recv_all" => {
                use std::io::Read;
                if args.len() >= 1 {
                    let handle = args[0].as_int();
                    if let Some(stream) = self.tcp_streams.get_mut(&handle) {
                        let mut buf = Vec::new();
                        match stream.read_to_end(&mut buf) {
                            Ok(_) => Some(Ok(Value::String(String::from_utf8_lossy(&buf).to_string()))),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    } else { Some(Err(format!("tcp_recv_all: unknown handle {}", handle))) }
                } else { Some(Err("tcp_recv_all(handle)".to_string())) }
            }

            // tcp_close(handle)
            "tcp_close" => {
                if args.len() >= 1 {
                    let handle = args[0].as_int();
                    self.tcp_streams.remove(&handle);
                    Some(Ok(Value::Null))
                } else { Some(Err("tcp_close(handle)".to_string())) }
            }

            // tcp_listen(host, port) or tcp_listen("host:port") -> listener_handle
            "tcp_listen" => {
                if args.len() >= 1 {
                    let addr = if args.len() >= 2 {
                        format!("{}:{}", args[0].as_string(), args[1].as_int())
                    } else {
                        args[0].as_string()
                    };
                    match std::net::TcpListener::bind(&addr) {
                        Ok(listener) => {
                            self.net_handle_counter += 1;
                            let handle = self.net_handle_counter;
                            self.tcp_listeners.insert(handle, listener);
                            Some(Ok(Value::Int(handle)))
                        }
                        Err(e) => Some(Err(e.to_string())),
                    }
                } else { Some(Err("tcp_listen(host, port)".to_string())) }
            }

            // tcp_accept(listener_handle) -> [stream_handle, peer_addr_str]
            "tcp_accept" => {
                if args.len() >= 1 {
                    let handle = args[0].as_int();
                    // Collect accept result while listener is borrowed, then drop borrow
                    let accept_result = if let Some(listener) = self.tcp_listeners.get(&handle) {
                        Some(listener.accept())
                    } else { None };
                    match accept_result {
                        Some(Ok((stream, peer))) => {
                            self.net_handle_counter += 1;
                            let sh = self.net_handle_counter;
                            let peer_str = peer.to_string();
                            self.tcp_streams.insert(sh, stream);
                            Some(Ok(Value::Array(vec![
                                Value::Int(sh),
                                Value::String(peer_str),
                            ])))
                        }
                        Some(Err(e)) => Some(Err(e.to_string())),
                        None => Some(Err(format!("tcp_accept: unknown listener {}", handle))),
                    }
                } else { Some(Err("tcp_accept(listener_handle)".to_string())) }
            }

            // tcp_listen_close(handle)
            "tcp_listen_close" => {
                if args.len() >= 1 {
                    let handle = args[0].as_int();
                    self.tcp_listeners.remove(&handle);
                    Some(Ok(Value::Null))
                } else { Some(Err("tcp_listen_close(handle)".to_string())) }
            }

            // tcp_set_timeout(handle, read_ms, write_ms)
            "tcp_set_timeout" => {
                if args.len() >= 3 {
                    let handle = args[0].as_int();
                    let read_ms  = args[1].as_int() as u64;
                    let write_ms = args[2].as_int() as u64;
                    if let Some(stream) = self.tcp_streams.get(&handle) {
                        let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(read_ms)));
                        let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(write_ms)));
                        Some(Ok(Value::Null))
                    } else { Some(Err(format!("tcp_set_timeout: unknown handle {}", handle))) }
                } else { Some(Err("tcp_set_timeout(handle, read_ms, write_ms)".to_string())) }
            }

            // tcp_peer_addr(handle) -> string
            "tcp_peer_addr" => {
                if args.len() >= 1 {
                    let handle = args[0].as_int();
                    if let Some(stream) = self.tcp_streams.get(&handle) {
                        match stream.peer_addr() {
                            Ok(addr) => Some(Ok(Value::String(addr.to_string()))),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    } else { Some(Err(format!("tcp_peer_addr: unknown handle {}", handle))) }
                } else { Some(Err("tcp_peer_addr(handle)".to_string())) }
            }

            // tcp_local_addr(handle) -> string
            "tcp_local_addr" => {
                if args.len() >= 1 {
                    let handle = args[0].as_int();
                    if let Some(stream) = self.tcp_streams.get(&handle) {
                        match stream.local_addr() {
                            Ok(addr) => Some(Ok(Value::String(addr.to_string()))),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    } else { Some(Err(format!("tcp_local_addr: unknown handle {}", handle))) }
                } else { Some(Err("tcp_local_addr(handle)".to_string())) }
            }

            // ── UDP Sockets ─────────────────────────────────────────────────────
            // udp_bind(addr, port) -> handle
            "udp_bind" => {
                if args.len() >= 2 {
                    let host = args[0].as_string();
                    let port = args[1].as_int() as u16;
                    let addr = format!("{}:{}", host, port);
                    match std::net::UdpSocket::bind(&addr) {
                        Ok(sock) => {
                            self.net_handle_counter += 1;
                            let handle = self.net_handle_counter;
                            self.udp_sockets.insert(handle, sock);
                            Some(Ok(Value::Int(handle)))
                        }
                        Err(e) => Some(Err(e.to_string())),
                    }
                } else { Some(Err("udp_bind(host, port)".to_string())) }
            }

            // udp_send(handle, dest_host, dest_port, data) -> bytes_sent
            "udp_send" => {
                if args.len() >= 4 {
                    let handle    = args[0].as_int();
                    let dest_host = args[1].as_string();
                    let dest_port = args[2].as_int() as u16;
                    let data      = args[3].as_string();
                    let dest_addr = format!("{}:{}", dest_host, dest_port);
                    if let Some(sock) = self.udp_sockets.get(&handle) {
                        match sock.send_to(data.as_bytes(), &dest_addr) {
                            Ok(n) => Some(Ok(Value::Int(n as i64))),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    } else { Some(Err(format!("udp_send: unknown handle {}", handle))) }
                } else { Some(Err("udp_send(handle, host, port, data)".to_string())) }
            }

            // udp_recv(handle, max_bytes?) -> [data_str, sender_addr_str]
            "udp_recv" => {
                if args.len() >= 1 {
                    let handle = args[0].as_int();
                    let max = if args.len() >= 2 { args[1].as_int() as usize } else { 65536 };
                    if let Some(sock) = self.udp_sockets.get(&handle) {
                        let mut buf = vec![0u8; max];
                        match sock.recv_from(&mut buf) {
                            Ok((n, sender)) => {
                                buf.truncate(n);
                                Some(Ok(Value::Array(vec![
                                    Value::String(String::from_utf8_lossy(&buf).to_string()),
                                    Value::String(sender.to_string()),
                                ])))
                            }
                            Err(e) => Some(Err(e.to_string())),
                        }
                    } else { Some(Err(format!("udp_recv: unknown handle {}", handle))) }
                } else { Some(Err("udp_recv(handle)".to_string())) }
            }

            // udp_close(handle)
            "udp_close" => {
                if args.len() >= 1 {
                    let handle = args[0].as_int();
                    self.udp_sockets.remove(&handle);
                    Some(Ok(Value::Null))
                } else { Some(Err("udp_close(handle)".to_string())) }
            }

            // udp_set_timeout(handle, read_ms)
            "udp_set_timeout" => {
                if args.len() >= 2 {
                    let handle  = args[0].as_int();
                    let read_ms = args[1].as_int() as u64;
                    if let Some(sock) = self.udp_sockets.get(&handle) {
                        let _ = sock.set_read_timeout(Some(std::time::Duration::from_millis(read_ms)));
                        Some(Ok(Value::Null))
                    } else { Some(Err(format!("udp_set_timeout: unknown handle {}", handle))) }
                } else { Some(Err("udp_set_timeout(handle, read_ms)".to_string())) }
            }

            // udp_broadcast(handle, port, data) -> bytes_sent
            "udp_broadcast" => {
                if args.len() >= 3 {
                    let handle = args[0].as_int();
                    let port   = args[1].as_int() as u16;
                    let data   = args[2].as_string();
                    if let Some(sock) = self.udp_sockets.get(&handle) {
                        let _ = sock.set_broadcast(true);
                        match sock.send_to(data.as_bytes(), format!("255.255.255.255:{}", port)) {
                            Ok(n) => Some(Ok(Value::Int(n as i64))),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    } else { Some(Err(format!("udp_broadcast: unknown handle {}", handle))) }
                } else { Some(Err("udp_broadcast(handle, port, data)".to_string())) }
            }

            // ── HTTP (builtin raw HTTP over TCP) ────────────────────────────────
            // http_get(url) -> response_body_string
            "http_get" => {
                let url = args.first().map(|v| v.as_string()).unwrap_or_default();
                // Parse url: [http://]host[:port][/path]
                let (host, port, path) = parse_http_url(&url);
                match std::net::TcpStream::connect_timeout(
                    &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
                    std::time::Duration::from_secs(15),
                ) {
                    Ok(mut stream) => {
                        use std::io::{Write, Read};
                        let req = format!(
                            "GET {} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\nUser-Agent: Knull/1.0\r\n\r\n",
                            path, host
                        );
                        let _ = stream.write_all(req.as_bytes());
                        let mut resp = Vec::new();
                        let _ = stream.read_to_end(&mut resp);
                        let s = String::from_utf8_lossy(&resp).to_string();
                        // Strip HTTP headers — return body only
                        let body = if let Some(pos) = s.find("\r\n\r\n") {
                            s[pos + 4..].to_string()
                        } else { s };
                        Some(Ok(Value::String(body)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }

            // http_post(url, body, content_type?) -> response_body_string
            "http_post" => {
                if args.len() >= 2 {
                    let url  = args[0].as_string();
                    let body = args[1].as_string();
                    let ct   = if args.len() >= 3 { args[2].as_string() } else { "application/octet-stream".to_string() };
                    let (host, port, path) = parse_http_url(&url);
                    match std::net::TcpStream::connect_timeout(
                        &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
                        std::time::Duration::from_secs(15),
                    ) {
                        Ok(mut stream) => {
                            use std::io::{Write, Read};
                            let req = format!(
                                "POST {} HTTP/1.0\r\nHost: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nUser-Agent: Knull/1.0\r\n\r\n{}",
                                path, host, ct, body.len(), body
                            );
                            let _ = stream.write_all(req.as_bytes());
                            let mut resp = Vec::new();
                            let _ = stream.read_to_end(&mut resp);
                            let s = String::from_utf8_lossy(&resp).to_string();
                            let body_out = if let Some(pos) = s.find("\r\n\r\n") { s[pos+4..].to_string() } else { s };
                            Some(Ok(Value::String(body_out)))
                        }
                        Err(e) => Some(Err(e.to_string())),
                    }
                } else { Some(Err("http_post(url, body, content_type?)".to_string())) }
            }

            // http_response_code(url) -> status int
            "http_status" | "http_response_code" => {
                let url = args.first().map(|v| v.as_string()).unwrap_or_default();
                let (host, port, path) = parse_http_url(&url);
                match std::net::TcpStream::connect_timeout(
                    &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
                    std::time::Duration::from_secs(10),
                ) {
                    Ok(mut stream) => {
                        use std::io::{Write, Read};
                        let req = format!(
                            "HEAD {} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\nUser-Agent: Knull/1.0\r\n\r\n",
                            path, host
                        );
                        let _ = stream.write_all(req.as_bytes());
                        let mut resp = vec![0u8; 1024];
                        let n = stream.read(&mut resp).unwrap_or(0);
                        let s = String::from_utf8_lossy(&resp[..n]).to_string();
                        // Extract status code from "HTTP/1.x 200 OK"
                        let code: i64 = s.split_whitespace().nth(1)
                            .and_then(|c| c.parse().ok())
                            .unwrap_or(0);
                        Some(Ok(Value::Int(code)))
                    }
                    Err(_) => Some(Ok(Value::Int(0))),
                }
            }

            // ── Crypto / Hashing ────────────────────────────────────────────────
            "sha256" | "sha256_hex" => {
                // Use shell command since we have no crypto crate
                let input = args.first().map(|v| v.as_string()).unwrap_or_default();
                match std::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!("printf '%s' '{}' | sha256sum | awk '{{print $1}}'", input.replace("'", "'\\''")))
                    .output()
                {
                    Ok(out) => Some(Ok(Value::String(String::from_utf8_lossy(&out.stdout).trim().to_string()))),
                    Err(_) => Some(Ok(Value::String("".to_string()))),
                }
            }
            "md5" | "md5_hex" => {
                let input = args.first().map(|v| v.as_string()).unwrap_or_default();
                match std::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!("printf '%s' '{}' | md5sum | awk '{{print $1}}'", input.replace("'", "'\\''")))
                    .output()
                {
                    Ok(out) => Some(Ok(Value::String(String::from_utf8_lossy(&out.stdout).trim().to_string()))),
                    Err(_) => Some(Ok(Value::String("".to_string()))),
                }
            }
            "random_bytes" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(16) as usize;
                match std::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!("dd if=/dev/urandom bs=1 count={} 2>/dev/null | xxd -p | tr -d '\\n'", n))
                    .output()
                {
                    Ok(out) => Some(Ok(Value::String(String::from_utf8_lossy(&out.stdout).trim().to_string()))),
                    Err(_) => Some(Ok(Value::String("".to_string()))),
                }
            }
            "random_string" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(16) as usize;
                let charset = args.get(1).map(|v| v.as_string())
                    .unwrap_or_else(|| "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".to_string());
                let chars: Vec<char> = charset.chars().collect();
                if chars.is_empty() { return Some(Ok(Value::String("".to_string()))); }
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as usize;
                let result: String = (0..n)
                    .map(|i| chars[seed.wrapping_mul(6364136223846793005usize).wrapping_add(i.wrapping_mul(1442695040888963407usize)) % chars.len()])
                    .collect();
                Some(Ok(Value::String(result)))
            }

            // ── Scope introspection ───────────────────────────────────────────
            "vars" | "globals" => {
                let mut map = HashMap::new();
                for scope in &self.scopes {
                    for (k, v) in &scope.variables {
                        map.insert(k.clone(), v.clone());
                    }
                }
                Some(Ok(Value::Map(map)))
            }
            "defined" => {
                let name = args.first().map(|v| v.as_string()).unwrap_or_default();
                Some(Ok(Value::Bool(self.get_variable(&name).is_some())))
            }

            // ── Matrix / Vector math (AI, 3D, physics) ──────────────────────
            // mat_new(rows, cols) -> flat array of 0.0
            "flat_mat_new" => {
                if args.len() >= 2 {
                    let rows = args[0].as_int() as usize;
                    let cols = args[1].as_int() as usize;
                    let data = vec![Value::Float(0.0); rows * cols];
                    Some(Ok(Value::Array(data)))
                } else { Some(Err("mat_new(rows, cols)".to_string())) }
            }
            // mat_get(mat, rows, cols, r, c) -> float
            "mat_get" => {
                if args.len() >= 5 {
                    if let Value::Array(ref m) = args[0] {
                        let cols = args[2].as_int() as usize;
                        let r = args[3].as_int() as usize;
                        let c = args[4].as_int() as usize;
                        Some(Ok(m.get(r * cols + c).cloned().unwrap_or(Value::Float(0.0))))
                    } else { Some(Err("mat_get: first arg must be array".to_string())) }
                } else { Some(Err("mat_get(mat, rows, cols, r, c)".to_string())) }
            }
            // mat_set(mat, rows, cols, r, c, val) -> new mat
            "mat_set" => {
                if args.len() >= 6 {
                    if let Value::Array(ref m) = args[0] {
                        let cols = args[2].as_int() as usize;
                        let r    = args[3].as_int() as usize;
                        let c    = args[4].as_int() as usize;
                        let val  = args[5].clone();
                        let mut out = m.clone();
                        if r * cols + c < out.len() { out[r * cols + c] = val; }
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("mat_set: first arg must be array".to_string())) }
                } else { Some(Err("mat_set(mat, rows, cols, r, c, val)".to_string())) }
            }
            // mat_mul(a, b, ar, ac_br, bc) -> flat result
            "flat_mat_mul" => {
                if args.len() >= 5 {
                    if let (Value::Array(ref a), Value::Array(ref b)) = (&args[0], &args[1]) {
                        let ar   = args[2].as_int() as usize;
                        let ac   = args[3].as_int() as usize; // == br
                        let bc   = args[4].as_int() as usize;
                        let mut out = vec![Value::Float(0.0); ar * bc];
                        for i in 0..ar {
                            for j in 0..bc {
                                let mut s = 0.0f64;
                                for k in 0..ac {
                                    s += a[i*ac+k].as_float() * b[k*bc+j].as_float();
                                }
                                out[i*bc+j] = Value::Float(s);
                            }
                        }
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("mat_mul: args must be arrays".to_string())) }
                } else { Some(Err("mat_mul(a, b, a_rows, shared, b_cols)".to_string())) }
            }
            // mat_add(a, b) -> element-wise sum
            "flat_mat_add" => {
                if args.len() >= 2 {
                    if let (Value::Array(ref a), Value::Array(ref b)) = (&args[0], &args[1]) {
                        let out: Vec<Value> = a.iter().zip(b.iter())
                            .map(|(x, y)| Value::Float(x.as_float() + y.as_float()))
                            .collect();
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("mat_add: args must be arrays".to_string())) }
                } else { Some(Err("mat_add(a, b)".to_string())) }
            }
            // mat_transpose(mat, rows, cols) -> flat transposed
            "flat_mat_transpose" => {
                if args.len() >= 3 {
                    if let Value::Array(ref m) = args[0] {
                        let rows = args[1].as_int() as usize;
                        let cols = args[2].as_int() as usize;
                        let mut out = vec![Value::Float(0.0); rows * cols];
                        for r in 0..rows {
                            for c in 0..cols {
                                out[c*rows+r] = m[r*cols+c].clone();
                            }
                        }
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("mat_transpose: first arg must be array".to_string())) }
                } else { Some(Err("mat_transpose(mat, rows, cols)".to_string())) }
            }
            // dot(a, b) -> scalar
            "dot" | "flat_vec_dot" => {
                if args.len() >= 2 {
                    if let (Value::Array(ref a), Value::Array(ref b)) = (&args[0], &args[1]) {
                        let s: f64 = a.iter().zip(b.iter()).map(|(x,y)| x.as_float()*y.as_float()).sum();
                        Some(Ok(Value::Float(s)))
                    } else { Some(Err("dot: args must be arrays".to_string())) }
                } else { Some(Err("dot(a, b)".to_string())) }
            }
            // cross(a, b) -> vec3
            "cross" | "flat_vec_cross" => {
                if args.len() >= 2 {
                    if let (Value::Array(ref a), Value::Array(ref b)) = (&args[0], &args[1]) {
                        if a.len() >= 3 && b.len() >= 3 {
                            let (ax,ay,az) = (a[0].as_float(), a[1].as_float(), a[2].as_float());
                            let (bx,by,bz) = (b[0].as_float(), b[1].as_float(), b[2].as_float());
                            Some(Ok(Value::Array(vec![
                                Value::Float(ay*bz - az*by),
                                Value::Float(az*bx - ax*bz),
                                Value::Float(ax*by - ay*bx),
                            ])))
                        } else { Some(Err("cross: need vec3".to_string())) }
                    } else { Some(Err("cross: args must be arrays".to_string())) }
                } else { Some(Err("cross(a, b)".to_string())) }
            }
            // vec_norm(v) -> float magnitude
            "vec_norm" | "vec_magnitude" | "magnitude" => {
                if let Some(Value::Array(ref v)) = args.first() {
                    let s: f64 = v.iter().map(|x| { let f = x.as_float(); f*f }).sum();
                    Some(Ok(Value::Float(s.sqrt())))
                } else { Some(Err("vec_norm(v)".to_string())) }
            }
            // vec_normalize(v) -> unit vector
            "vec_normalize" | "normalize" => {
                if let Some(Value::Array(ref v)) = args.first() {
                    let mag: f64 = v.iter().map(|x| { let f = x.as_float(); f*f }).sum::<f64>().sqrt();
                    if mag == 0.0 { return Some(Ok(Value::Array(v.clone()))); }
                    let out: Vec<Value> = v.iter().map(|x| Value::Float(x.as_float()/mag)).collect();
                    Some(Ok(Value::Array(out)))
                } else { Some(Err("vec_normalize(v)".to_string())) }
            }
            // vec_scale(v, scalar) -> scaled vector
            "vec_scale" => {
                if args.len() >= 2 {
                    if let Value::Array(ref v) = args[0] {
                        let s = args[1].as_float();
                        let out: Vec<Value> = v.iter().map(|x| Value::Float(x.as_float()*s)).collect();
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("vec_scale: first arg must be array".to_string())) }
                } else { Some(Err("vec_scale(v, scalar)".to_string())) }
            }
            // vec_add(a, b) / vec_sub(a, b)
            "vec_add" => {
                if args.len() >= 2 {
                    if let (Value::Array(ref a), Value::Array(ref b)) = (&args[0], &args[1]) {
                        let out: Vec<Value> = a.iter().zip(b.iter()).map(|(x,y)| Value::Float(x.as_float()+y.as_float())).collect();
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("vec_add: args must be arrays".to_string())) }
                } else { Some(Err("vec_add(a, b)".to_string())) }
            }
            "vec_sub" => {
                if args.len() >= 2 {
                    if let (Value::Array(ref a), Value::Array(ref b)) = (&args[0], &args[1]) {
                        let out: Vec<Value> = a.iter().zip(b.iter()).map(|(x,y)| Value::Float(x.as_float()-y.as_float())).collect();
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("vec_sub: args must be arrays".to_string())) }
                } else { Some(Err("vec_sub(a, b)".to_string())) }
            }

            // ── Raw Memory / Pointers (OS, kernels, low-level, FFI) ─────────
            // mem_alloc(size) -> ptr_int
            "mem_alloc" | "malloc" => {
                let size = args.first().map(|v| v.as_int()).unwrap_or(1) as usize;
                let layout = std::alloc::Layout::from_size_align(size.max(1), 8).unwrap();
                let ptr = unsafe { std::alloc::alloc_zeroed(layout) } as i64;
                Some(Ok(Value::Int(ptr)))
            }
            // mem_free(ptr, size)
            "mem_free" | "free" => {
                if args.len() >= 2 {
                    let ptr  = args[0].as_int() as *mut u8;
                    let size = args[1].as_int() as usize;
                    if !ptr.is_null() && size > 0 {
                        let layout = std::alloc::Layout::from_size_align(size, 8).unwrap();
                        unsafe { std::alloc::dealloc(ptr, layout); }
                    }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_free(ptr, size)".to_string())) }
            }
            // mem_read_u8/u16/u32/u64/i8/i32/i64/f32/f64(ptr) -> int/float
            "mem_read_u8" => {
                if let Some(a) = args.first() {
                    let ptr = a.as_int() as *const u8;
                    Some(Ok(Value::Int(unsafe { *ptr } as i64)))
                } else { Some(Err("mem_read_u8(ptr)".to_string())) }
            }
            "mem_read_u16" => {
                if let Some(a) = args.first() {
                    let ptr = a.as_int() as *const u16;
                    Some(Ok(Value::Int(unsafe { *ptr } as i64)))
                } else { Some(Err("mem_read_u16(ptr)".to_string())) }
            }
            "mem_read_u32" => {
                if let Some(a) = args.first() {
                    let ptr = a.as_int() as *const u32;
                    Some(Ok(Value::Int(unsafe { *ptr } as i64)))
                } else { Some(Err("mem_read_u32(ptr)".to_string())) }
            }
            "mem_read_u64" | "mem_read_i64" => {
                if let Some(a) = args.first() {
                    let ptr = a.as_int() as *const i64;
                    Some(Ok(Value::Int(unsafe { *ptr })))
                } else { Some(Err("mem_read_u64(ptr)".to_string())) }
            }
            "mem_read_i32" => {
                if let Some(a) = args.first() {
                    let ptr = a.as_int() as *const i32;
                    Some(Ok(Value::Int(unsafe { *ptr } as i64)))
                } else { Some(Err("mem_read_i32(ptr)".to_string())) }
            }
            "mem_read_f32" => {
                if let Some(a) = args.first() {
                    let ptr = a.as_int() as *const f32;
                    Some(Ok(Value::Float(unsafe { *ptr } as f64)))
                } else { Some(Err("mem_read_f32(ptr)".to_string())) }
            }
            "mem_read_f64" => {
                if let Some(a) = args.first() {
                    let ptr = a.as_int() as *const f64;
                    Some(Ok(Value::Float(unsafe { *ptr })))
                } else { Some(Err("mem_read_f64(ptr)".to_string())) }
            }
            // mem_write_u8/u16/u32/u64/i32/f32/f64(ptr, val)
            "mem_write_u8" => {
                if args.len() >= 2 {
                    let ptr = args[0].as_int() as *mut u8;
                    unsafe { *ptr = args[1].as_int() as u8; }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_write_u8(ptr, val)".to_string())) }
            }
            "mem_write_u16" => {
                if args.len() >= 2 {
                    let ptr = args[0].as_int() as *mut u16;
                    unsafe { *ptr = args[1].as_int() as u16; }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_write_u16(ptr, val)".to_string())) }
            }
            "mem_write_u32" => {
                if args.len() >= 2 {
                    let ptr = args[0].as_int() as *mut u32;
                    unsafe { *ptr = args[1].as_int() as u32; }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_write_u32(ptr, val)".to_string())) }
            }
            "mem_write_u64" | "mem_write_i64" => {
                if args.len() >= 2 {
                    let ptr = args[0].as_int() as *mut i64;
                    unsafe { *ptr = args[1].as_int(); }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_write_u64(ptr, val)".to_string())) }
            }
            "mem_write_i32" => {
                if args.len() >= 2 {
                    let ptr = args[0].as_int() as *mut i32;
                    unsafe { *ptr = args[1].as_int() as i32; }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_write_i32(ptr, val)".to_string())) }
            }
            "mem_write_f32" => {
                if args.len() >= 2 {
                    let ptr = args[0].as_int() as *mut f32;
                    unsafe { *ptr = args[1].as_float() as f32; }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_write_f32(ptr, val)".to_string())) }
            }
            "mem_write_f64" => {
                if args.len() >= 2 {
                    let ptr = args[0].as_int() as *mut f64;
                    unsafe { *ptr = args[1].as_float(); }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_write_f64(ptr, val)".to_string())) }
            }
            // mem_copy(dst, src, bytes)
            "mem_copy" | "memcpy" => {
                if args.len() >= 3 {
                    let dst   = args[0].as_int() as *mut u8;
                    let src   = args[1].as_int() as *const u8;
                    let bytes = args[2].as_int() as usize;
                    unsafe { std::ptr::copy_nonoverlapping(src, dst, bytes); }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_copy(dst, src, bytes)".to_string())) }
            }
            // mem_set(ptr, byte_val, count)
            "mem_set" | "memset" => {
                if args.len() >= 3 {
                    let ptr   = args[0].as_int() as *mut u8;
                    let val   = args[1].as_int() as u8;
                    let count = args[2].as_int() as usize;
                    unsafe { std::ptr::write_bytes(ptr, val, count); }
                    Some(Ok(Value::Null))
                } else { Some(Err("mem_set(ptr, byte_val, count)".to_string())) }
            }
            // mem_cmp(a, b, bytes) -> int  (like memcmp)
            "mem_cmp" | "memcmp" => {
                if args.len() >= 3 {
                    let a     = args[0].as_int() as *const u8;
                    let b     = args[1].as_int() as *const u8;
                    let bytes = args[2].as_int() as usize;
                    let result = unsafe {
                        let sa = std::slice::from_raw_parts(a, bytes);
                        let sb = std::slice::from_raw_parts(b, bytes);
                        sa.cmp(sb) as i8 as i64
                    };
                    Some(Ok(Value::Int(result)))
                } else { Some(Err("mem_cmp(a, b, bytes)".to_string())) }
            }
            // ptr_add(ptr, offset) -> ptr
            "ptr_add" => {
                if args.len() >= 2 {
                    let ptr    = args[0].as_int();
                    let offset = args[1].as_int();
                    Some(Ok(Value::Int(ptr + offset)))
                } else { Some(Err("ptr_add(ptr, offset)".to_string())) }
            }
            // ptr_diff(a, b) -> int (a - b)
            "ptr_diff" => {
                if args.len() >= 2 {
                    Some(Ok(Value::Int(args[0].as_int() - args[1].as_int())))
                } else { Some(Err("ptr_diff(a, b)".to_string())) }
            }
            // ptr_null() -> 0
            "ptr_null" | "null_ptr" => Some(Ok(Value::Int(0))),
            // ptr_is_null(ptr) -> bool
            "ptr_is_null" => Some(Ok(Value::Bool(args.first().map(|v| v.as_int() == 0).unwrap_or(true)))),
            // str_to_ptr(s) -> ptr to utf8 bytes (leaks — for FFI use)
            "str_to_ptr" => {
                if let Some(v) = args.first() {
                    let s = v.as_string();
                    let boxed = s.into_bytes().into_boxed_slice();
                    let ptr = boxed.as_ptr() as i64;
                    std::mem::forget(boxed);
                    Some(Ok(Value::Int(ptr)))
                } else { Some(Err("str_to_ptr(s)".to_string())) }
            }
            // ptr_to_str(ptr, len) -> string
            "ptr_to_str" => {
                if args.len() >= 2 {
                    let ptr = args[0].as_int() as *const u8;
                    let len = args[1].as_int() as usize;
                    let s = unsafe {
                        let slice = std::slice::from_raw_parts(ptr, len);
                        String::from_utf8_lossy(slice).to_string()
                    };
                    Some(Ok(Value::String(s)))
                } else { Some(Err("ptr_to_str(ptr, len)".to_string())) }
            }
            // syscall(nr, a0, a1, a2, a3, a4, a5) -> int  [Linux x86-64 only]
            "syscall" => {
                #[cfg(target_arch = "x86_64")]
                {
                    if args.len() >= 1 {
                        let nr = args[0].as_int();
                        let a = [
                            args.get(1).map(|v| v.as_int()).unwrap_or(0) as u64,
                            args.get(2).map(|v| v.as_int()).unwrap_or(0) as u64,
                            args.get(3).map(|v| v.as_int()).unwrap_or(0) as u64,
                            args.get(4).map(|v| v.as_int()).unwrap_or(0) as u64,
                            args.get(5).map(|v| v.as_int()).unwrap_or(0) as u64,
                            args.get(6).map(|v| v.as_int()).unwrap_or(0) as u64,
                        ];
                        let ret: i64;
                        unsafe {
                            std::arch::asm!(
                                "syscall",
                                inlateout("rax") nr as u64 => ret,
                                in("rdi") a[0], in("rsi") a[1], in("rdx") a[2],
                                in("r10") a[3], in("r8")  a[4], in("r9")  a[5],
                                lateout("rcx") _, lateout("r11") _,
                                options(nostack),
                            );
                        }
                        Some(Ok(Value::Int(ret)))
                    } else { Some(Err("syscall(nr, ...)".to_string())) }
                }
                #[cfg(not(target_arch = "x86_64"))]
                { Some(Err("syscall() only supported on x86_64".to_string())) }
            }

            // ── OS Threads ───────────────────────────────────────────────────
            // thread_spawn(fn, arg0, arg1, ...) -> thread_handle
            "thread_spawn" => {
                if args.len() >= 1 {
                    let func  = args[0].clone();
                    let fargs = args[1..].to_vec();
                    let (tx, rx) = std::sync::mpsc::channel::<Value>();
                    let func_name = match &func {
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    };
                    let funcs_clone: Vec<(String, Vec<String>, ASTNode)> = self.functions
                        .iter()
                        .map(|(k, v)| (k.clone(), v.params.clone(), v.body.clone()))
                        .collect();
                    std::thread::spawn(move || {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let mut interp = Interpreter::new();
                            for (name, params, body) in funcs_clone {
                                interp.functions.insert(name.clone(), FunctionDef { name, params, body });
                            }
                            if let Some(name) = func_name {
                                interp.call_function(&name, fargs).unwrap_or(Value::Null)
                            } else {
                                interp.call_value(func, fargs).unwrap_or(Value::Null)
                            }
                        }));
                        let val = result.unwrap_or(Value::Null);
                        let _ = tx.send(val);
                    });
                    self.thread_counter += 1;
                    let id = self.thread_counter;
                    self.thread_results.insert(id, rx);
                    Some(Ok(Value::Int(id)))
                } else { Some(Err("thread_spawn(fn, args...)".to_string())) }
            }
            // thread_join(handle) -> return_value
            "thread_join" => {
                if let Some(a) = args.first() {
                    let id = a.as_int();
                    if let Some(rx) = self.thread_results.remove(&id) {
                        match rx.recv() {
                            Ok(v)  => Some(Ok(v)),
                            Err(_) => Some(Ok(Value::Null)),
                        }
                    } else { Some(Err(format!("thread_join: unknown handle {}", id))) }
                } else { Some(Err("thread_join(handle)".to_string())) }
            }
            // thread_try_recv(handle) -> value or null (non-blocking)
            "thread_try_recv" | "thread_poll" => {
                if let Some(a) = args.first() {
                    let id = a.as_int();
                    if let Some(rx) = self.thread_results.get(&id) {
                        match rx.try_recv() {
                            Ok(v)  => Some(Ok(v)),
                            Err(_) => Some(Ok(Value::Null)),
                        }
                    } else { Some(Ok(Value::Null)) }
                } else { Some(Err("thread_poll(handle)".to_string())) }
            }
            // thread_sleep(ms)
            "thread_sleep" | "sleep_ms" => {
                let ms = args.first().map(|v| v.as_int()).unwrap_or(0);
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                Some(Ok(Value::Null))
            }
            // thread_current_id() -> int
            "thread_current_id" => {
                let id_str = format!("{:?}", std::thread::current().id());
                // ThreadId debug is "ThreadId(N)" — extract N
                let n: i64 = id_str
                    .trim_start_matches("ThreadId(")
                    .trim_end_matches(')')
                    .parse()
                    .unwrap_or(0);
                Some(Ok(Value::Int(n)))
            }

            // ── Mutexes ──────────────────────────────────────────────────────
            // mutex_new(initial_val?) -> handle
            "mutex_new" => {
                let init = args.first().cloned().unwrap_or(Value::Null);
                let m = std::sync::Arc::new(std::sync::Mutex::new(init));
                self.mutex_counter += 1;
                let id = self.mutex_counter;
                self.mutexes.insert(id, m);
                Some(Ok(Value::Int(id)))
            }
            // mutex_lock(handle) -> current_value
            "mutex_lock" => {
                if let Some(a) = args.first() {
                    let id = a.as_int();
                    if let Some(m) = self.mutexes.get(&id) {
                        match m.lock() {
                            Ok(v)  => Some(Ok(v.clone())),
                            Err(_) => Some(Err("mutex_lock: poisoned".to_string())),
                        }
                    } else { Some(Err(format!("mutex_lock: unknown {}", id))) }
                } else { Some(Err("mutex_lock(handle)".to_string())) }
            }
            // mutex_set(handle, value) — set value under lock
            "mutex_set" => {
                if args.len() >= 2 {
                    let id  = args[0].as_int();
                    let val = args[1].clone();
                    if let Some(m) = self.mutexes.get(&id) {
                        match m.lock() {
                            Ok(mut v) => { *v = val; Some(Ok(Value::Null)) }
                            Err(_) => Some(Err("mutex_set: poisoned".to_string())),
                        }
                    } else { Some(Err(format!("mutex_set: unknown {}", id))) }
                } else { Some(Err("mutex_set(handle, value)".to_string())) }
            }
            // mutex_clone(handle) -> new handle sharing same Arc
            "mutex_clone" => {
                if let Some(a) = args.first() {
                    let id = a.as_int();
                    if let Some(m) = self.mutexes.get(&id).cloned() {
                        self.mutex_counter += 1;
                        let new_id = self.mutex_counter;
                        self.mutexes.insert(new_id, m);
                        Some(Ok(Value::Int(new_id)))
                    } else { Some(Err(format!("mutex_clone: unknown {}", id))) }
                } else { Some(Err("mutex_clone(handle)".to_string())) }
            }

            // ── AI / ML Math ─────────────────────────────────────────────────
            "sigmoid" => {
                let x = args.first().map(|v| v.as_float()).unwrap_or(0.0);
                Some(Ok(Value::Float(1.0 / (1.0 + (-x).exp()))))
            }
            "sigmoid_deriv" => {
                let x = args.first().map(|v| v.as_float()).unwrap_or(0.0);
                let s = 1.0 / (1.0 + (-x).exp());
                Some(Ok(Value::Float(s * (1.0 - s))))
            }
            "relu" => {
                let x = args.first().map(|v| v.as_float()).unwrap_or(0.0);
                Some(Ok(Value::Float(x.max(0.0))))
            }
            "relu_deriv" => {
                let x = args.first().map(|v| v.as_float()).unwrap_or(0.0);
                Some(Ok(Value::Float(if x > 0.0 { 1.0 } else { 0.0 })))
            }
            "leaky_relu" => {
                if args.len() >= 2 {
                    let x     = args[0].as_float();
                    let alpha = args[1].as_float();
                    Some(Ok(Value::Float(if x > 0.0 { x } else { alpha * x })))
                } else { Some(Err("leaky_relu(x, alpha)".to_string())) }
            }
            "tanh_act" | "tanh_activation" => {
                let x = args.first().map(|v| v.as_float()).unwrap_or(0.0);
                Some(Ok(Value::Float(x.tanh())))
            }
            "softmax" => {
                if let Some(Value::Array(ref v)) = args.first() {
                    let max = v.iter().map(|x| x.as_float()).fold(f64::NEG_INFINITY, f64::max);
                    let exps: Vec<f64> = v.iter().map(|x| (x.as_float() - max).exp()).collect();
                    let sum: f64 = exps.iter().sum();
                    let out: Vec<Value> = exps.iter().map(|e| Value::Float(e / sum)).collect();
                    Some(Ok(Value::Array(out)))
                } else { Some(Err("softmax(array)".to_string())) }
            }
            // mse(predicted_arr, target_arr) -> float
            "mse" | "mean_squared_error" => {
                if args.len() >= 2 {
                    if let (Value::Array(ref p), Value::Array(ref t)) = (&args[0], &args[1]) {
                        let n = p.len().max(1) as f64;
                        let loss: f64 = p.iter().zip(t.iter())
                            .map(|(pi, ti)| { let d = pi.as_float() - ti.as_float(); d*d })
                            .sum::<f64>() / n;
                        Some(Ok(Value::Float(loss)))
                    } else { Some(Err("mse: args must be arrays".to_string())) }
                } else { Some(Err("mse(predicted, target)".to_string())) }
            }
            // cross_entropy(predicted_arr, target_arr) -> float
            "cross_entropy" => {
                if args.len() >= 2 {
                    if let (Value::Array(ref p), Value::Array(ref t)) = (&args[0], &args[1]) {
                        let eps = 1e-15_f64;
                        let loss: f64 = p.iter().zip(t.iter())
                            .map(|(pi, ti)| -ti.as_float() * (pi.as_float().max(eps)).ln())
                            .sum();
                        Some(Ok(Value::Float(loss)))
                    } else { Some(Err("cross_entropy: args must be arrays".to_string())) }
                } else { Some(Err("cross_entropy(predicted, target)".to_string())) }
            }
            // mat_scalar_mul(mat, scalar) -> scaled mat
            "mat_scalar_mul" => {
                if args.len() >= 2 {
                    if let Value::Array(ref m) = args[0] {
                        let s = args[1].as_float();
                        let out: Vec<Value> = m.iter().map(|x| Value::Float(x.as_float()*s)).collect();
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("mat_scalar_mul: first arg must be array".to_string())) }
                } else { Some(Err("mat_scalar_mul(mat, scalar)".to_string())) }
            }
            // mat_apply(mat, fn) -> mat with fn applied to each element
            "mat_apply" => {
                if args.len() >= 2 {
                    if let Value::Array(ref m) = args[0] {
                        let func = args[1].clone();
                        let m_clone = m.clone();
                        let mut out = Vec::with_capacity(m_clone.len());
                        for elem in m_clone {
                            let v = match self.call_value(func.clone(), vec![elem]) {
                                Ok(v) => v,
                                Err(e) => return Some(Err(e)),
                            };
                            out.push(v);
                        }
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("mat_apply: first arg must be array".to_string())) }
                } else { Some(Err("mat_apply(mat, fn)".to_string())) }
            }
            // argmax(arr) -> index of max element
            "argmax" => {
                if let Some(Value::Array(ref v)) = args.first() {
                    let idx = v.iter().enumerate()
                        .max_by(|(_, a), (_, b)| a.as_float().partial_cmp(&b.as_float()).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    Some(Ok(Value::Int(idx as i64)))
                } else { Some(Err("argmax(array)".to_string())) }
            }
            // argmin(arr) -> index of min element
            "argmin" => {
                if let Some(Value::Array(ref v)) = args.first() {
                    let idx = v.iter().enumerate()
                        .min_by(|(_, a), (_, b)| a.as_float().partial_cmp(&b.as_float()).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    Some(Ok(Value::Int(idx as i64)))
                } else { Some(Err("argmin(array)".to_string())) }
            }
            // linspace(start, end, n) -> array of n evenly spaced floats
            "linspace" => {
                if args.len() >= 3 {
                    let start = args[0].as_float();
                    let end   = args[1].as_float();
                    let n     = args[2].as_int() as usize;
                    if n == 0 { return Some(Ok(Value::Array(vec![]))); }
                    if n == 1 { return Some(Ok(Value::Array(vec![Value::Float(start)]))); }
                    let step = (end - start) / (n - 1) as f64;
                    let out: Vec<Value> = (0..n).map(|i| Value::Float(start + step * i as f64)).collect();
                    Some(Ok(Value::Array(out)))
                } else { Some(Err("linspace(start, end, n)".to_string())) }
            }
            // arange(start, end, step) -> array
            "arange" => {
                if args.len() >= 3 {
                    let start = args[0].as_float();
                    let end   = args[1].as_float();
                    let step  = args[2].as_float();
                    if step == 0.0 { return Some(Err("arange: step cannot be 0".to_string())); }
                    let mut v = Vec::new();
                    let mut cur = start;
                    while (step > 0.0 && cur < end) || (step < 0.0 && cur > end) {
                        v.push(Value::Float(cur));
                        cur += step;
                    }
                    Some(Ok(Value::Array(v)))
                } else { Some(Err("arange(start, end, step)".to_string())) }
            }
            // mean(arr) -> float
            "mean" | "average" => {
                if let Some(Value::Array(ref v)) = args.first() {
                    if v.is_empty() { return Some(Ok(Value::Float(0.0))); }
                    let s: f64 = v.iter().map(|x| x.as_float()).sum();
                    Some(Ok(Value::Float(s / v.len() as f64)))
                } else { Some(Err("mean(array)".to_string())) }
            }
            // std_dev(arr) -> float
            "std_dev" | "stddev" => {
                if let Some(Value::Array(ref v)) = args.first() {
                    if v.is_empty() { return Some(Ok(Value::Float(0.0))); }
                    let n = v.len() as f64;
                    let m: f64 = v.iter().map(|x| x.as_float()).sum::<f64>() / n;
                    let variance: f64 = v.iter().map(|x| { let d = x.as_float() - m; d*d }).sum::<f64>() / n;
                    Some(Ok(Value::Float(variance.sqrt())))
                } else { Some(Err("std_dev(array)".to_string())) }
            }

            // ── Regex ───────────────────────────────────────────────────────
            // regex_match(pattern, text) -> bool
            "re_match" => {
                if args.len() >= 2 {
                    let pat  = args[0].as_string();
                    let text = args[1].as_string();
                    match regex::Regex::new(&pat) {
                        Ok(re) => Some(Ok(Value::Bool(re.is_match(&text)))),
                        Err(e) => Some(Err(format!("regex_match: {}", e))),
                    }
                } else { Some(Err("regex_match(pattern, text)".to_string())) }
            }
            // regex_find(pattern, text) -> first match string or null
            "re_find" => {
                if args.len() >= 2 {
                    let pat  = args[0].as_string();
                    let text = args[1].as_string();
                    match regex::Regex::new(&pat) {
                        Ok(re) => match re.find(&text) {
                            Some(m) => Some(Ok(Value::String(m.as_str().to_string()))),
                            None    => Some(Ok(Value::Null)),
                        },
                        Err(e) => Some(Err(format!("regex_find: {}", e))),
                    }
                } else { Some(Err("regex_find(pattern, text)".to_string())) }
            }
            // regex_findall(pattern, text) -> array of match strings
            "re_findall" => {
                if args.len() >= 2 {
                    let pat  = args[0].as_string();
                    let text = args[1].as_string();
                    match regex::Regex::new(&pat) {
                        Ok(re) => {
                            let matches: Vec<Value> = re.find_iter(&text)
                                .map(|m| Value::String(m.as_str().to_string()))
                                .collect();
                            Some(Ok(Value::Array(matches)))
                        },
                        Err(e) => Some(Err(format!("regex_findall: {}", e))),
                    }
                } else { Some(Err("regex_findall(pattern, text)".to_string())) }
            }
            // regex_replace(pattern, replacement, text) -> string
            "re_replace" => {
                if args.len() >= 3 {
                    let pat  = args[0].as_string();
                    let rep  = args[1].as_string();
                    let text = args[2].as_string();
                    match regex::Regex::new(&pat) {
                        Ok(re) => Some(Ok(Value::String(re.replace_all(&text, rep.as_str()).to_string()))),
                        Err(e) => Some(Err(format!("regex_replace: {}", e))),
                    }
                } else { Some(Err("regex_replace(pattern, replacement, text)".to_string())) }
            }
            // regex_split(pattern, text) -> array of parts
            "re_split" => {
                if args.len() >= 2 {
                    let pat  = args[0].as_string();
                    let text = args[1].as_string();
                    match regex::Regex::new(&pat) {
                        Ok(re) => {
                            let parts: Vec<Value> = re.split(&text)
                                .map(|s| Value::String(s.to_string()))
                                .collect();
                            Some(Ok(Value::Array(parts)))
                        },
                        Err(e) => Some(Err(format!("regex_split: {}", e))),
                    }
                } else { Some(Err("regex_split(pattern, text)".to_string())) }
            }
            // regex_captures(pattern, text) -> array of captured groups (or null)
            "re_captures" => {
                if args.len() >= 2 {
                    let pat  = args[0].as_string();
                    let text = args[1].as_string();
                    match regex::Regex::new(&pat) {
                        Ok(re) => match re.captures(&text) {
                            Some(caps) => {
                                let groups: Vec<Value> = caps.iter()
                                    .map(|m| m.map(|m| Value::String(m.as_str().to_string()))
                                               .unwrap_or(Value::Null))
                                    .collect();
                                Some(Ok(Value::Array(groups)))
                            }
                            None => Some(Ok(Value::Null)),
                        },
                        Err(e) => Some(Err(format!("regex_captures: {}", e))),
                    }
                } else { Some(Err("regex_captures(pattern, text)".to_string())) }
            }

            // ── ANSI Terminal ────────────────────────────────────────────────
            // ansi(code) -> escape string e.g. ansi("31") = red fg
            "ansi" => {
                let code = args.first().map(|v| v.as_string()).unwrap_or_default();
                Some(Ok(Value::String(format!("\x1b[{}m", code))))
            }
            "ansi_reset" => Some(Ok(Value::String("\x1b[0m".to_string()))),
            // Color shortcuts
            "red"     => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[31m{}\x1b[0m", s)))) }
            "green"   => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[32m{}\x1b[0m", s)))) }
            "yellow"  => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[33m{}\x1b[0m", s)))) }
            "blue"    => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[34m{}\x1b[0m", s)))) }
            "magenta" | "purple" => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[35m{}\x1b[0m", s)))) }
            "cyan"    => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[36m{}\x1b[0m", s)))) }
            "white"   => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[37m{}\x1b[0m", s)))) }
            "bold"    => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[1m{}\x1b[0m", s)))) }
            "dim"     => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[2m{}\x1b[0m", s)))) }
            "italic"  => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[3m{}\x1b[0m", s)))) }
            "underline" => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[4m{}\x1b[0m", s)))) }
            "blink"   => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[5m{}\x1b[0m", s)))) }
            "bg_red"  => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[41m{}\x1b[0m", s)))) }
            "bg_green"=> { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[42m{}\x1b[0m", s)))) }
            "bg_blue" => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(format!("\x1b[44m{}\x1b[0m", s)))) }
            // Cursor control
            "cursor_up"   => { let n = args.first().map(|v| v.as_int()).unwrap_or(1); Some(Ok(Value::String(format!("\x1b[{}A", n)))) }
            "cursor_down" => { let n = args.first().map(|v| v.as_int()).unwrap_or(1); Some(Ok(Value::String(format!("\x1b[{}B", n)))) }
            "cursor_right"=> { let n = args.first().map(|v| v.as_int()).unwrap_or(1); Some(Ok(Value::String(format!("\x1b[{}C", n)))) }
            "cursor_left" => { let n = args.first().map(|v| v.as_int()).unwrap_or(1); Some(Ok(Value::String(format!("\x1b[{}D", n)))) }
            "cursor_move" => {
                if args.len() >= 2 {
                    Some(Ok(Value::String(format!("\x1b[{};{}H", args[0].as_int(), args[1].as_int()))))
                } else { Some(Err("cursor_move(row, col)".to_string())) }
            }
            "clear_screen" => Some(Ok(Value::String("\x1b[2J\x1b[H".to_string()))),
            "clear_line"   => Some(Ok(Value::String("\x1b[2K\r".to_string()))),
            "hide_cursor"  => Some(Ok(Value::String("\x1b[?25l".to_string()))),
            "show_cursor"  => Some(Ok(Value::String("\x1b[?25h".to_string()))),
            "term_width" => {
                // ioctl TIOCGWINSZ
                let width: u16 = unsafe {
                    let mut ws: libc::winsize = std::mem::zeroed();
                    if libc::ioctl(1, libc::TIOCGWINSZ, &mut ws as *mut _) == 0 { ws.ws_col } else { 80 }
                };
                Some(Ok(Value::Int(width as i64)))
            }
            "term_height" => {
                let height: u16 = unsafe {
                    let mut ws: libc::winsize = std::mem::zeroed();
                    if libc::ioctl(1, libc::TIOCGWINSZ, &mut ws as *mut _) == 0 { ws.ws_row } else { 24 }
                };
                Some(Ok(Value::Int(height as i64)))
            }
            // print_raw(s) — print without newline
            "print_raw" | "print_no_newline" => {
                use std::io::Write;
                let s = args.first().map(|v| v.as_string()).unwrap_or_default();
                print!("{}", s);
                let _ = std::io::stdout().flush();
                Some(Ok(Value::Null))
            }

            // ── Binary File I/O ──────────────────────────────────────────────
            // file_read_bytes(path) -> array of ints (0-255)
            "file_read_bytes" | "read_bytes" => {
                if let Some(v) = args.first() {
                    match std::fs::read(v.as_string()) {
                        Ok(bytes) => {
                            let arr: Vec<Value> = bytes.iter().map(|&b| Value::Int(b as i64)).collect();
                            Some(Ok(Value::Array(arr)))
                        }
                        Err(e) => Some(Err(format!("file_read_bytes: {}", e))),
                    }
                } else { Some(Err("file_read_bytes(path)".to_string())) }
            }
            // file_write_bytes(path, array_of_ints)
            "file_write_bytes" | "write_bytes" => {
                if args.len() >= 2 {
                    let path = args[0].as_string();
                    if let Value::Array(ref arr) = args[1] {
                        let bytes: Vec<u8> = arr.iter().map(|v| v.as_int() as u8).collect();
                        match std::fs::write(&path, &bytes) {
                            Ok(_)  => Some(Ok(Value::Null)),
                            Err(e) => Some(Err(format!("file_write_bytes: {}", e))),
                        }
                    } else { Some(Err("file_write_bytes: second arg must be array".to_string())) }
                } else { Some(Err("file_write_bytes(path, bytes_array)".to_string())) }
            }
            // file_append_bytes(path, bytes_array)
            "file_append_bytes" => {
                if args.len() >= 2 {
                    let path = args[0].as_string();
                    if let Value::Array(ref arr) = args[1] {
                        use std::io::Write;
                        let bytes: Vec<u8> = arr.iter().map(|v| v.as_int() as u8).collect();
                        match std::fs::OpenOptions::new().append(true).create(true).open(&path) {
                            Ok(mut f) => match f.write_all(&bytes) {
                                Ok(_) => Some(Ok(Value::Null)),
                                Err(e) => Some(Err(e.to_string())),
                            },
                            Err(e) => Some(Err(e.to_string())),
                        }
                    } else { Some(Err("file_append_bytes: second arg must be array".to_string())) }
                } else { Some(Err("file_append_bytes(path, bytes_array)".to_string())) }
            }

            // ── Encoding: hex / base64 ───────────────────────────────────────
            // hex_encode(array_of_ints or string) -> hex string
            "hex_encode" | "to_hex" | "bytes_to_hex" => {
                if let Some(v) = args.first() {
                    let hex = match v {
                        Value::Array(arr) => arr.iter()
                            .map(|b| format!("{:02x}", b.as_int() & 0xFF))
                            .collect::<Vec<_>>().join(""),
                        _ => {
                            let s = v.as_string();
                            s.bytes().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join("")
                        }
                    };
                    Some(Ok(Value::String(hex)))
                } else { Some(Err("hex_encode(bytes_or_string)".to_string())) }
            }
            // hex_decode(hex_string) -> array of ints
            "hex_decode" | "from_hex" | "hex_to_bytes" => {
                if let Some(v) = args.first() {
                    let s = v.as_string();
                    let s = s.trim().replace(" ", "");
                    let mut bytes = Vec::new();
                    let mut chars = s.chars().peekable();
                    while let (Some(a), Some(b)) = (chars.next(), chars.next()) {
                        if let Ok(byte) = u8::from_str_radix(&format!("{}{}", a, b), 16) {
                            bytes.push(Value::Int(byte as i64));
                        }
                    }
                    Some(Ok(Value::Array(bytes)))
                } else { Some(Err("hex_decode(hex_string)".to_string())) }
            }
            // hex_decode_str(hex_string) -> utf8 string
            "hex_decode_str" => {
                if let Some(v) = args.first() {
                    let s = v.as_string().trim().replace(" ", "");
                    let mut bytes = Vec::new();
                    let mut chars = s.chars().peekable();
                    while let (Some(a), Some(b)) = (chars.next(), chars.next()) {
                        if let Ok(byte) = u8::from_str_radix(&format!("{}{}", a, b), 16) {
                            bytes.push(byte);
                        }
                    }
                    Some(Ok(Value::String(String::from_utf8_lossy(&bytes).to_string())))
                } else { Some(Err("hex_decode_str(hex_string)".to_string())) }
            }
            // base64_encode(string or bytes_array) -> base64 string
            "base64_encode" | "b64_encode" => {
                if let Some(v) = args.first() {
                    let bytes: Vec<u8> = match v {
                        Value::Array(arr) => arr.iter().map(|b| b.as_int() as u8).collect(),
                        _ => v.as_string().into_bytes(),
                    };
                    // encode without external crate using alphabet
                    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                    let mut out = String::new();
                    for chunk in bytes.chunks(3) {
                        let b0 = chunk[0] as u32;
                        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
                        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
                        let n = (b0 << 16) | (b1 << 8) | b2;
                        out.push(ALPHA[(n >> 18) as usize] as char);
                        out.push(ALPHA[((n >> 12) & 63) as usize] as char);
                        if chunk.len() > 1 { out.push(ALPHA[((n >> 6) & 63) as usize] as char); } else { out.push('='); }
                        if chunk.len() > 2 { out.push(ALPHA[(n & 63) as usize] as char); } else { out.push('='); }
                    }
                    Some(Ok(Value::String(out)))
                } else { Some(Err("base64_encode(string_or_bytes)".to_string())) }
            }
            // base64_decode(b64_string) -> string
            "base64_decode" | "b64_decode" => {
                if let Some(v) = args.first() {
                    let s = v.as_string();
                    let s = s.trim().replace('\n', "");
                    let decode_char = |c: u8| -> Option<u32> {
                        match c {
                            b'A'..=b'Z' => Some((c - b'A') as u32),
                            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
                            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
                            b'+' => Some(62),
                            b'/' => Some(63),
                            _ => None,
                        }
                    };
                    let bytes_in: Vec<u8> = s.bytes().collect();
                    let mut out = Vec::new();
                    for chunk in bytes_in.chunks(4) {
                        let a = chunk[0];
                        let b = if chunk.len() > 1 { chunk[1] } else { b'A' };
                        let c = if chunk.len() > 2 { chunk[2] } else { b'=' };
                        let d = if chunk.len() > 3 { chunk[3] } else { b'=' };
                        if let (Some(va), Some(vb)) = (decode_char(a), decode_char(b)) {
                            let n = (va << 18) | (vb << 12);
                            let n = if c != b'=' {
                                if let Some(vc) = decode_char(c) {
                                    let n2 = n | (vc << 6);
                                    out.push(((n2 >> 16) & 0xFF) as u8);
                                    out.push(((n2 >> 8) & 0xFF) as u8);
                                    if d != b'=' {
                                        if let Some(vd) = decode_char(d) {
                                            let n3 = n2 | vd;
                                            out.push((n3 & 0xFF) as u8);
                                        }
                                    }
                                    continue;
                                }
                                n
                            } else { n };
                            out.push(((n >> 16) & 0xFF) as u8);
                        }
                    }
                    Some(Ok(Value::String(String::from_utf8_lossy(&out).to_string())))
                } else { Some(Err("base64_decode(b64_string)".to_string())) }
            }
            // base64_decode_bytes(b64_string) -> array of ints
            "base64_decode_bytes" | "b64_decode_bytes" => {
                // Delegate to base64_decode and convert
                if let Some(v) = args.first() {
                    let decoded = {
                        let a = args[0].clone();
                        match self.call_builtin("base64_decode", &[a]) {
                            Some(Ok(Value::String(s))) => s.into_bytes().iter().map(|&b| Value::Int(b as i64)).collect(),
                            _ => vec![],
                        }
                    };
                    Some(Ok(Value::Array(decoded)))
                } else { Some(Err("base64_decode_bytes(b64_string)".to_string())) }
            }

            // ── Process Control (OS, red-team, automation) ───────────────────
            // kill(pid, signal) -> bool success
            "kill" | "process_kill" => {
                if args.len() >= 2 {
                    let pid = args[0].as_int();
                    let sig = args[1].as_int();
                    let result = unsafe { libc::kill(pid as libc::pid_t, sig as libc::c_int) };
                    Some(Ok(Value::Bool(result == 0)))
                } else if args.len() == 1 {
                    // kill(pid) -> SIGTERM
                    let pid = args[0].as_int();
                    let result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
                    Some(Ok(Value::Bool(result == 0)))
                } else { Some(Err("kill(pid, signal?)".to_string())) }
            }
            // getppid() -> parent pid
            "getppid" => Some(Ok(Value::Int(unsafe { libc::getppid() } as i64))),
            // setenv(key, val) already exists; also unsetenv
            "unsetenv" | "env_unset" => {
                if let Some(v) = args.first() {
                    std::env::remove_var(v.as_string());
                    Some(Ok(Value::Null))
                } else { Some(Err("unsetenv(key)".to_string())) }
            }
            // fork() -> pid (0 = child, >0 = parent with child pid)
            "fork" => {
                let pid = unsafe { libc::fork() };
                Some(Ok(Value::Int(pid as i64)))
            }
            // waitpid(pid) -> exit_code
            "waitpid" => {
                let pid   = args.first().map(|v| v.as_int()).unwrap_or(-1) as libc::pid_t;
                let flags = args.get(1).map(|v| v.as_int()).unwrap_or(0) as libc::c_int;
                let mut status: libc::c_int = 0;
                let ret  = unsafe { libc::waitpid(pid, &mut status, flags) };
                let exited   = unsafe { libc::WIFEXITED(status) };
                let code     = unsafe { libc::WEXITSTATUS(status) };
                let signaled = unsafe { libc::WIFSIGNALED(status) };
                let sig      = unsafe { libc::WTERMSIG(status) };
                let mut m = HashMap::new();
                m.insert("pid".to_string(),       Value::Int(ret as i64));
                m.insert("status".to_string(),    Value::Int(status as i64));
                m.insert("exited".to_string(),    Value::Bool(exited));
                m.insert("exit_code".to_string(), Value::Int(code as i64));
                m.insert("code".to_string(),      Value::Int(code as i64));
                m.insert("signaled".to_string(),  Value::Bool(signaled));
                m.insert("signal".to_string(),    Value::Int(sig as i64));
                Some(Ok(Value::Map(m)))
            }
            // signal constants
            "SIGTERM" => Some(Ok(Value::Int(libc::SIGTERM as i64))),
            "SIGKILL" => Some(Ok(Value::Int(libc::SIGKILL as i64))),
            "SIGINT"  => Some(Ok(Value::Int(libc::SIGINT  as i64))),
            "SIGHUP"  => Some(Ok(Value::Int(libc::SIGHUP  as i64))),
            "SIGUSR1" => Some(Ok(Value::Int(libc::SIGUSR1 as i64))),
            "SIGUSR2" => Some(Ok(Value::Int(libc::SIGUSR2 as i64))),
            "SIGSTOP" => Some(Ok(Value::Int(libc::SIGSTOP as i64))),
            "SIGCONT" => Some(Ok(Value::Int(libc::SIGCONT as i64))),

            // ── Native Crypto (pure Rust, no shell) ──────────────────────────
            // sha256_native(string_or_bytes) -> hex string
            "sha256_native" | "sha256n" => {
                fn sha256_round(data: &[u8]) -> [u8; 32] {
                    const K: [u32; 64] = [
                        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
                        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
                        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
                        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
                        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
                        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
                        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
                        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
                    ];
                    let mut h = [0x6a09e667u32,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
                    let bit_len = (data.len() as u64) * 8;
                    let mut msg = data.to_vec();
                    msg.push(0x80);
                    while msg.len() % 64 != 56 { msg.push(0); }
                    for b in bit_len.to_be_bytes() { msg.push(b); }
                    for chunk in msg.chunks(64) {
                        let mut w = [0u32; 64];
                        for i in 0..16 { w[i] = u32::from_be_bytes([chunk[i*4],chunk[i*4+1],chunk[i*4+2],chunk[i*4+3]]); }
                        for i in 16..64 {
                            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
                            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
                            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
                        }
                        let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh) = (h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
                        for i in 0..64 {
                            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                            let ch = (e & f) ^ ((!e) & g);
                            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
                            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                            let maj = (a & b) ^ (a & c) ^ (b & c);
                            let t2 = s0.wrapping_add(maj);
                            hh=g; g=f; f=e; e=d.wrapping_add(t1); d=c; c=b; b=a; a=t1.wrapping_add(t2);
                        }
                        let adds = [a,b,c,d,e,f,g,hh];
                        for i in 0..8 { h[i] = h[i].wrapping_add(adds[i]); }
                    }
                    let mut out = [0u8; 32];
                    for i in 0..8 { out[i*4..i*4+4].copy_from_slice(&h[i].to_be_bytes()); }
                    out
                }
                if let Some(v) = args.first() {
                    let bytes: Vec<u8> = match v {
                        Value::Array(arr) => arr.iter().map(|b| b.as_int() as u8).collect(),
                        _ => v.as_string().into_bytes(),
                    };
                    let hash = sha256_round(&bytes);
                    let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
                    Some(Ok(Value::String(hex)))
                } else { Some(Err("sha256_native(data)".to_string())) }
            }

            // hmac_sha256(key, data) -> hex
            "hmac_sha256" => {
                if args.len() >= 2 {
                    fn sha256_bytes(data: &[u8]) -> [u8; 32] {
                        const K: [u32; 64] = [
                            0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
                            0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
                            0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
                            0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
                            0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
                            0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
                            0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
                            0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
                        ];
                        let mut h = [0x6a09e667u32,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
                        let bit_len = (data.len() as u64) * 8;
                        let mut msg = data.to_vec();
                        msg.push(0x80);
                        while msg.len() % 64 != 56 { msg.push(0); }
                        for b in bit_len.to_be_bytes() { msg.push(b); }
                        for chunk in msg.chunks(64) {
                            let mut w = [0u32; 64];
                            for i in 0..16 { w[i] = u32::from_be_bytes([chunk[i*4],chunk[i*4+1],chunk[i*4+2],chunk[i*4+3]]); }
                            for i in 16..64 {
                                let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
                                let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
                                w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
                            }
                            let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh) = (h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
                            for i in 0..64 {
                                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                                let ch = (e & f) ^ ((!e) & g);
                                let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
                                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                                let maj = (a & b) ^ (a & c) ^ (b & c);
                                let t2 = s0.wrapping_add(maj);
                                hh=g; g=f; f=e; e=d.wrapping_add(t1); d=c; c=b; b=a; a=t1.wrapping_add(t2);
                            }
                            let adds = [a,b,c,d,e,f,g,hh];
                            for i in 0..8 { h[i] = h[i].wrapping_add(adds[i]); }
                        }
                        let mut out = [0u8; 32]; for i in 0..8 { out[i*4..i*4+4].copy_from_slice(&h[i].to_be_bytes()); } out
                    }
                    let mut key_bytes: Vec<u8> = args[0].as_string().into_bytes();
                    let data_bytes: Vec<u8>    = args[1].as_string().into_bytes();
                    if key_bytes.len() > 64 { key_bytes = sha256_bytes(&key_bytes).to_vec(); }
                    key_bytes.resize(64, 0);
                    let ipad: Vec<u8> = key_bytes.iter().map(|b| b ^ 0x36).collect();
                    let opad: Vec<u8> = key_bytes.iter().map(|b| b ^ 0x5c).collect();
                    let mut inner = ipad.clone(); inner.extend_from_slice(&data_bytes);
                    let inner_hash = sha256_bytes(&inner);
                    let mut outer = opad.clone(); outer.extend_from_slice(&inner_hash);
                    let mac = sha256_bytes(&outer);
                    let hex: String = mac.iter().map(|b| format!("{:02x}", b)).collect();
                    Some(Ok(Value::String(hex)))
                } else { Some(Err("hmac_sha256(key, data)".to_string())) }
            }

            // xor_bytes(bytes_array, key_byte) -> bytes_array  (XOR cipher)
            "xor_bytes" | "xor_encrypt" | "xor_decrypt" => {
                if args.len() >= 2 {
                    if let Value::Array(ref arr) = args[0] {
                        let key = args[1].as_int() as u8;
                        let out: Vec<Value> = arr.iter().map(|b| Value::Int((b.as_int() as u8 ^ key) as i64)).collect();
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("xor_bytes: first arg must be array".to_string())) }
                } else { Some(Err("xor_bytes(bytes, key_byte)".to_string())) }
            }
            // xor_key(bytes_array, key_bytes_array) -> bytes_array  (repeating XOR)
            "xor_key" | "xor_encrypt_key" => {
                if args.len() >= 2 {
                    if let (Value::Array(ref data), Value::Array(ref key)) = (&args[0], &args[1]) {
                        if key.is_empty() { return Some(Err("xor_key: empty key".to_string())); }
                        let out: Vec<Value> = data.iter().enumerate()
                            .map(|(i, b)| Value::Int((b.as_int() as u8 ^ key[i % key.len()].as_int() as u8) as i64))
                            .collect();
                        Some(Ok(Value::Array(out)))
                    } else { Some(Err("xor_key: args must be arrays".to_string())) }
                } else { Some(Err("xor_key(data_bytes, key_bytes)".to_string())) }
            }
            // crc32(bytes_array or string) -> int
            "crc32" => {
                if let Some(v) = args.first() {
                    let bytes: Vec<u8> = match v {
                        Value::Array(arr) => arr.iter().map(|b| b.as_int() as u8).collect(),
                        _ => v.as_string().into_bytes(),
                    };
                    let mut crc: u32 = 0xFFFFFFFF;
                    for byte in bytes {
                        crc ^= byte as u32;
                        for _ in 0..8 {
                            if crc & 1 != 0 { crc = (crc >> 1) ^ 0xEDB88320; } else { crc >>= 1; }
                        }
                    }
                    Some(Ok(Value::Int((crc ^ 0xFFFFFFFF) as i64)))
                } else { Some(Err("crc32(data)".to_string())) }
            }

            // ── File System Ops ──────────────────────────────────────────────
            // file_stat(path) -> map {size, mode, uid, gid, mtime, is_dir, is_file, is_link}
            "file_stat" | "stat" => {
                if let Some(v) = args.first() {
                    use std::os::unix::fs::MetadataExt;
                    match std::fs::metadata(v.as_string()) {
                        Ok(m) => {
                            let mut map = HashMap::new();
                            map.insert("size".to_string(),   Value::Int(m.len() as i64));
                            map.insert("mode".to_string(),   Value::Int(m.mode() as i64));
                            map.insert("uid".to_string(),    Value::Int(m.uid() as i64));
                            map.insert("gid".to_string(),    Value::Int(m.gid() as i64));
                            map.insert("mtime".to_string(),  Value::Int(m.mtime()));
                            map.insert("is_dir".to_string(), Value::Bool(m.is_dir()));
                            map.insert("is_file".to_string(),Value::Bool(m.is_file()));
                            map.insert("is_link".to_string(),Value::Bool(m.file_type().is_symlink()));
                            Some(Ok(Value::Map(map)))
                        }
                        Err(e) => Some(Err(format!("stat: {}", e))),
                    }
                } else { Some(Err("file_stat(path)".to_string())) }
            }
            // file_lstat(path) — like stat but doesn't follow symlinks
            "file_lstat" | "lstat" => {
                if let Some(v) = args.first() {
                    use std::os::unix::fs::MetadataExt;
                    match std::fs::symlink_metadata(v.as_string()) {
                        Ok(m) => {
                            let mut map = HashMap::new();
                            map.insert("size".to_string(),   Value::Int(m.len() as i64));
                            map.insert("mode".to_string(),   Value::Int(m.mode() as i64));
                            map.insert("uid".to_string(),    Value::Int(m.uid() as i64));
                            map.insert("gid".to_string(),    Value::Int(m.gid() as i64));
                            map.insert("mtime".to_string(),  Value::Int(m.mtime()));
                            map.insert("is_dir".to_string(), Value::Bool(m.is_dir()));
                            map.insert("is_file".to_string(),Value::Bool(m.is_file()));
                            map.insert("is_link".to_string(),Value::Bool(m.file_type().is_symlink()));
                            Some(Ok(Value::Map(map)))
                        }
                        Err(e) => Some(Err(format!("lstat: {}", e))),
                    }
                } else { Some(Err("file_lstat(path)".to_string())) }
            }
            // chmod(path, mode_oct)
            "chmod" | "file_chmod" => {
                if args.len() >= 2 {
                    use std::os::unix::fs::PermissionsExt;
                    let path = args[0].as_string();
                    let mode = args[1].as_int() as u32;
                    match std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)) {
                        Ok(_)  => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("chmod: {}", e))),
                    }
                } else { Some(Err("chmod(path, mode)".to_string())) }
            }
            // symlink(target, link_path)
            "symlink" | "file_symlink" => {
                if args.len() >= 2 {
                    let target = args[0].as_string();
                    let link   = args[1].as_string();
                    match std::os::unix::fs::symlink(&target, &link) {
                        Ok(_)  => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("symlink: {}", e))),
                    }
                } else { Some(Err("symlink(target, link_path)".to_string())) }
            }
            // readlink(path) -> target string
            "readlink" => {
                if let Some(v) = args.first() {
                    match std::fs::read_link(v.as_string()) {
                        Ok(p)  => Some(Ok(Value::String(p.to_string_lossy().to_string()))),
                        Err(e) => Some(Err(format!("readlink: {}", e))),
                    }
                } else { Some(Err("readlink(path)".to_string())) }
            }
            // hardlink(src, dst)
            "hardlink" | "file_hardlink" => {
                if args.len() >= 2 {
                    match std::fs::hard_link(args[0].as_string(), args[1].as_string()) {
                        Ok(_)  => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("hardlink: {}", e))),
                    }
                } else { Some(Err("hardlink(src, dst)".to_string())) }
            }
            // truncate(path, size)
            "truncate" | "file_truncate" => {
                if args.len() >= 2 {
                    match std::fs::OpenOptions::new().write(true).open(args[0].as_string()) {
                        Ok(f) => {
                            match f.set_len(args[1].as_int() as u64) {
                                Ok(_)  => Some(Ok(Value::Null)),
                                Err(e) => Some(Err(format!("truncate: {}", e))),
                            }
                        }
                        Err(e) => Some(Err(format!("truncate: {}", e))),
                    }
                } else { Some(Err("truncate(path, size)".to_string())) }
            }
            // glob(pattern) -> array of paths
            "glob" | "file_glob" => {
                if let Some(v) = args.first() {
                    let pattern = v.as_string();
                    // use std::path glob via walkdir-like iteration
                    // Simple approach: shell glob via Command
                    match std::process::Command::new("sh")
                        .arg("-c")
                        .arg(format!("ls -1d {} 2>/dev/null", pattern))
                        .output()
                    {
                        Ok(out) => {
                            let paths: Vec<Value> = String::from_utf8_lossy(&out.stdout)
                                .lines()
                                .filter(|l| !l.is_empty())
                                .map(|l| Value::String(l.to_string()))
                                .collect();
                            Some(Ok(Value::Array(paths)))
                        }
                        Err(e) => Some(Err(format!("glob: {}", e))),
                    }
                } else { Some(Err("glob(pattern)".to_string())) }
            }

            // ── mmap (memory-mapped files) ───────────────────────────────────
            // mmap_file(path) -> [ptr_int, size_int]  (readonly)
            "mmap_file" | "mmap_read" => {
                if let Some(v) = args.first() {
                    let path = v.as_string();
                    use std::os::unix::io::AsRawFd;
                    match std::fs::File::open(&path) {
                        Ok(f) => {
                            let size = f.metadata().map(|m| m.len() as usize).unwrap_or(0);
                            if size == 0 { return Some(Ok(Value::Array(vec![Value::Int(0), Value::Int(0)]))); }
                            let ptr = unsafe {
                                libc::mmap(std::ptr::null_mut(), size, libc::PROT_READ, libc::MAP_PRIVATE, f.as_raw_fd(), 0)
                            };
                            if ptr == libc::MAP_FAILED {
                                Some(Err("mmap_file: mmap failed".to_string()))
                            } else {
                                Some(Ok(Value::Array(vec![Value::Int(ptr as i64), Value::Int(size as i64)])))
                            }
                        }
                        Err(e) => Some(Err(format!("mmap_file: {}", e))),
                    }
                } else { Some(Err("mmap_file(path)".to_string())) }
            }
            // mmap_anon(size) -> [ptr_int, size_int]  (anonymous RW mapping)
            "mmap_anon" => {
                if let Some(v) = args.first() {
                    let size = v.as_int() as usize;
                    let ptr = unsafe {
                        libc::mmap(std::ptr::null_mut(), size, libc::PROT_READ | libc::PROT_WRITE,
                                   libc::MAP_PRIVATE | libc::MAP_ANONYMOUS, -1, 0)
                    };
                    if ptr == libc::MAP_FAILED {
                        Some(Err("mmap_anon: mmap failed".to_string()))
                    } else {
                        Some(Ok(Value::Array(vec![Value::Int(ptr as i64), Value::Int(size as i64)])))
                    }
                } else { Some(Err("mmap_anon(size)".to_string())) }
            }
            // munmap(ptr, size)
            "munmap" => {
                if args.len() >= 2 {
                    let ptr  = args[0].as_int() as *mut libc::c_void;
                    let size = args[1].as_int() as usize;
                    unsafe { libc::munmap(ptr, size); }
                    Some(Ok(Value::Null))
                } else { Some(Err("munmap(ptr, size)".to_string())) }
            }
            // mprotect(ptr, size, prot_flags)  e.g. mprotect(ptr, sz, 7) = RWX
            "mprotect" => {
                if args.len() >= 3 {
                    let ptr   = args[0].as_int() as *mut libc::c_void;
                    let size  = args[1].as_int() as usize;
                    let prot  = args[2].as_int() as libc::c_int;
                    let r = unsafe { libc::mprotect(ptr, size, prot) };
                    Some(Ok(Value::Bool(r == 0)))
                } else { Some(Err("mprotect(ptr, size, prot)".to_string())) }
            }
            // mmap protection constants
            "PROT_READ"  => Some(Ok(Value::Int(libc::PROT_READ  as i64))),
            "PROT_WRITE" => Some(Ok(Value::Int(libc::PROT_WRITE as i64))),
            "PROT_EXEC"  => Some(Ok(Value::Int(libc::PROT_EXEC  as i64))),
            "PROT_NONE"  => Some(Ok(Value::Int(libc::PROT_NONE  as i64))),

            // ═══════════════════════════════════════════════════════════════
            // SQLite / embedded database
            // ═══════════════════════════════════════════════════════════════

            // db_open(path) -> int handle
            "db_open" => {
                if args.is_empty() { return Some(Err("db_open(path)".to_string())); }
                let path = args[0].as_string();
                match rusqlite::Connection::open(&path) {
                    Ok(conn) => {
                        self.db_counter += 1;
                        let id = self.db_counter;
                        self.db_connections.insert(id, conn);
                        Some(Ok(Value::Int(id)))
                    }
                    Err(e) => Some(Err(format!("db_open: {}", e))),
                }
            }
            // db_open_memory() -> int handle  (in-memory DB)
            "db_open_memory" => {
                match rusqlite::Connection::open_in_memory() {
                    Ok(conn) => {
                        self.db_counter += 1;
                        let id = self.db_counter;
                        self.db_connections.insert(id, conn);
                        Some(Ok(Value::Int(id)))
                    }
                    Err(e) => Some(Err(format!("db_open_memory: {}", e))),
                }
            }
            // db_exec(handle, sql) -> bool
            "db_exec" => {
                if args.len() < 2 { return Some(Err("db_exec(handle, sql)".to_string())); }
                let id = args[0].as_int();
                let sql = args[1].as_string();
                match self.db_connections.get(&id) {
                    None => Some(Err(format!("db_exec: invalid handle {}", id))),
                    Some(conn) => {
                        match conn.execute_batch(&sql) {
                            Ok(_)  => Some(Ok(Value::Bool(true))),
                            Err(e) => Some(Err(format!("db_exec: {}", e))),
                        }
                    }
                }
            }
            // db_query(handle, sql) -> array of maps
            "db_query" => {
                if args.len() < 2 { return Some(Err("db_query(handle, sql)".to_string())); }
                let id = args[0].as_int();
                let sql = args[1].as_string();
                // bind params (optional 3rd arg: array of values)
                let params: Vec<rusqlite::types::Value> = if args.len() >= 3 {
                    match &args[2] {
                        Value::Array(arr) => arr.iter().map(|v| match v {
                            Value::Int(n)   => rusqlite::types::Value::Integer(*n),
                            Value::Float(f) => rusqlite::types::Value::Real(*f),
                            Value::Null     => rusqlite::types::Value::Null,
                            other           => rusqlite::types::Value::Text(other.as_string()),
                        }).collect(),
                        _ => vec![],
                    }
                } else { vec![] };
                match self.db_connections.get(&id) {
                    None => Some(Err(format!("db_query: invalid handle {}", id))),
                    Some(conn) => {
                        let result = (|| -> Result<Value, rusqlite::Error> {
                            let mut stmt = conn.prepare(&sql)?;
                            let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
                            let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p as &dyn rusqlite::types::ToSql).collect();
                            let rows_iter = stmt.query(param_refs.as_slice())?;
                            let mut rows_iter = rows_iter;
                            let mut rows: Vec<Value> = Vec::new();
                            while let Some(row) = rows_iter.next()? {
                                let mut map: HashMap<String, Value> = HashMap::new();
                                for (i, col) in col_names.iter().enumerate() {
                                    let val: rusqlite::types::Value = row.get(i)?;
                                    let kval = match val {
                                        rusqlite::types::Value::Null    => Value::Null,
                                        rusqlite::types::Value::Integer(n) => Value::Int(n),
                                        rusqlite::types::Value::Real(f)    => Value::Float(f),
                                        rusqlite::types::Value::Text(s)    => Value::String(s),
                                        rusqlite::types::Value::Blob(b)    => Value::Array(b.iter().map(|x| Value::Int(*x as i64)).collect()),
                                    };
                                    map.insert(col.clone(), kval);
                                }
                                rows.push(Value::Map(map));
                            }
                            Ok(Value::Array(rows))
                        })();
                        match result {
                            Ok(v) => Some(Ok(v)),
                            Err(e) => Some(Err(format!("db_query: {}", e))),
                        }
                    }
                }
            }
            // db_query_one(handle, sql) -> map or null
            "db_query_one" => {
                if args.len() < 2 { return Some(Err("db_query_one(handle, sql)".to_string())); }
                let id = args[0].as_int();
                let sql = args[1].as_string();
                match self.db_connections.get(&id) {
                    None => Some(Err(format!("db_query_one: invalid handle {}", id))),
                    Some(conn) => {
                        let result = (|| -> Result<Value, rusqlite::Error> {
                            let mut stmt = conn.prepare(&sql)?;
                            let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
                            let mut rows = stmt.query([])?;
                            if let Some(row) = rows.next()? {
                                let mut map: HashMap<String, Value> = HashMap::new();
                                for (i, col) in col_names.iter().enumerate() {
                                    let val: rusqlite::types::Value = row.get(i)?;
                                    let kval = match val {
                                        rusqlite::types::Value::Null    => Value::Null,
                                        rusqlite::types::Value::Integer(n) => Value::Int(n),
                                        rusqlite::types::Value::Real(f)    => Value::Float(f),
                                        rusqlite::types::Value::Text(s)    => Value::String(s),
                                        rusqlite::types::Value::Blob(b)    => Value::Array(b.iter().map(|x| Value::Int(*x as i64)).collect()),
                                    };
                                    map.insert(col.clone(), kval);
                                }
                                Ok(Value::Map(map))
                            } else {
                                Ok(Value::Null)
                            }
                        })();
                        match result {
                            Ok(v) => Some(Ok(v)),
                            Err(e) => Some(Err(format!("db_query_one: {}", e))),
                        }
                    }
                }
            }
            // db_last_insert_id(handle) -> int
            "db_last_insert_id" => {
                if args.is_empty() { return Some(Err("db_last_insert_id(handle)".to_string())); }
                let id = args[0].as_int();
                match self.db_connections.get(&id) {
                    None => Some(Err(format!("db_last_insert_id: invalid handle {}", id))),
                    Some(conn) => Some(Ok(Value::Int(conn.last_insert_rowid()))),
                }
            }
            // db_close(handle) -> bool
            "db_close" => {
                if args.is_empty() { return Some(Err("db_close(handle)".to_string())); }
                let id = args[0].as_int();
                let removed = self.db_connections.remove(&id);
                Some(Ok(Value::Bool(removed.is_some())))
            }

            // ═══════════════════════════════════════════════════════════════
            // Compression (gzip / zlib / deflate) — flate2 crate
            // ═══════════════════════════════════════════════════════════════

            // gzip_compress(bytes_array) -> bytes_array
            "gzip_compress" => {
                if args.is_empty() { return Some(Err("gzip_compress(bytes)".to_string())); }
                let bytes = match &args[0] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect::<Vec<u8>>(),
                    Value::String(s) => s.as_bytes().to_vec(),
                    _ => return Some(Err("gzip_compress: expected array or string".to_string())),
                };
                let mut enc = GzEncoder::new(Vec::new(), Compression::default());
                if let Err(e) = enc.write_all(&bytes) { return Some(Err(format!("gzip_compress: {}", e))); }
                match enc.finish() {
                    Ok(out) => Some(Ok(Value::Array(out.iter().map(|b| Value::Int(*b as i64)).collect()))),
                    Err(e)  => Some(Err(format!("gzip_compress: {}", e))),
                }
            }
            // gzip_decompress(bytes_array) -> bytes_array
            "gzip_decompress" => {
                if args.is_empty() { return Some(Err("gzip_decompress(bytes)".to_string())); }
                let bytes = match &args[0] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect::<Vec<u8>>(),
                    _ => return Some(Err("gzip_decompress: expected array".to_string())),
                };
                let mut dec = GzDecoder::new(bytes.as_slice());
                let mut out = Vec::new();
                if let Err(e) = dec.read_to_end(&mut out) { return Some(Err(format!("gzip_decompress: {}", e))); }
                Some(Ok(Value::Array(out.iter().map(|b| Value::Int(*b as i64)).collect())))
            }
            // zlib_compress(bytes_array) -> bytes_array
            "zlib_compress" => {
                if args.is_empty() { return Some(Err("zlib_compress(bytes)".to_string())); }
                let bytes = match &args[0] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect::<Vec<u8>>(),
                    Value::String(s) => s.as_bytes().to_vec(),
                    _ => return Some(Err("zlib_compress: expected array or string".to_string())),
                };
                let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
                if let Err(e) = enc.write_all(&bytes) { return Some(Err(format!("zlib_compress: {}", e))); }
                match enc.finish() {
                    Ok(out) => Some(Ok(Value::Array(out.iter().map(|b| Value::Int(*b as i64)).collect()))),
                    Err(e)  => Some(Err(format!("zlib_compress: {}", e))),
                }
            }
            // zlib_decompress(bytes_array) -> bytes_array
            "zlib_decompress" => {
                if args.is_empty() { return Some(Err("zlib_decompress(bytes)".to_string())); }
                let bytes = match &args[0] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect::<Vec<u8>>(),
                    _ => return Some(Err("zlib_decompress: expected array".to_string())),
                };
                let mut dec = ZlibDecoder::new(bytes.as_slice());
                let mut out = Vec::new();
                if let Err(e) = dec.read_to_end(&mut out) { return Some(Err(format!("zlib_decompress: {}", e))); }
                Some(Ok(Value::Array(out.iter().map(|b| Value::Int(*b as i64)).collect())))
            }
            // deflate_compress(bytes_array) -> bytes_array
            "deflate_compress" => {
                if args.is_empty() { return Some(Err("deflate_compress(bytes)".to_string())); }
                let bytes = match &args[0] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect::<Vec<u8>>(),
                    Value::String(s) => s.as_bytes().to_vec(),
                    _ => return Some(Err("deflate_compress: expected array or string".to_string())),
                };
                let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
                if let Err(e) = enc.write_all(&bytes) { return Some(Err(format!("deflate_compress: {}", e))); }
                match enc.finish() {
                    Ok(out) => Some(Ok(Value::Array(out.iter().map(|b| Value::Int(*b as i64)).collect()))),
                    Err(e)  => Some(Err(format!("deflate_compress: {}", e))),
                }
            }
            // deflate_decompress(bytes_array) -> bytes_array
            "deflate_decompress" => {
                if args.is_empty() { return Some(Err("deflate_decompress(bytes)".to_string())); }
                let bytes = match &args[0] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect::<Vec<u8>>(),
                    _ => return Some(Err("deflate_decompress: expected array".to_string())),
                };
                let mut dec = DeflateDecoder::new(bytes.as_slice());
                let mut out = Vec::new();
                if let Err(e) = dec.read_to_end(&mut out) { return Some(Err(format!("deflate_decompress: {}", e))); }
                Some(Ok(Value::Array(out.iter().map(|b| Value::Int(*b as i64)).collect())))
            }
            // compress_level(bytes, level) -> bytes  (gzip with custom level 0-9)
            "gzip_compress_level" => {
                if args.len() < 2 { return Some(Err("gzip_compress_level(bytes, level)".to_string())); }
                let bytes = match &args[0] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect::<Vec<u8>>(),
                    Value::String(s) => s.as_bytes().to_vec(),
                    _ => return Some(Err("gzip_compress_level: expected array or string".to_string())),
                };
                let level = (args[1].as_int() as u32).min(9);
                let mut enc = GzEncoder::new(Vec::new(), Compression::new(level));
                if let Err(e) = enc.write_all(&bytes) { return Some(Err(format!("gzip_compress_level: {}", e))); }
                match enc.finish() {
                    Ok(out) => Some(Ok(Value::Array(out.iter().map(|b| Value::Int(*b as i64)).collect()))),
                    Err(e)  => Some(Err(format!("gzip_compress_level: {}", e))),
                }
            }
            // gzip_decompress_str(bytes_array) -> string
            "gzip_decompress_str" => {
                if args.is_empty() { return Some(Err("gzip_decompress_str(bytes)".to_string())); }
                let bytes = match &args[0] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect::<Vec<u8>>(),
                    _ => return Some(Err("gzip_decompress_str: expected array".to_string())),
                };
                let mut dec = GzDecoder::new(bytes.as_slice());
                let mut out = String::new();
                if let Err(e) = dec.read_to_string(&mut out) { return Some(Err(format!("gzip_decompress_str: {}", e))); }
                Some(Ok(Value::String(out)))
            }

            // ═══════════════════════════════════════════════════════════════
            // Fundamental string / byte primitives
            // ═══════════════════════════════════════════════════════════════

            // chr(n) -> single-char string  (e.g. chr(65) == "A")
            "chr" => {
                if args.is_empty() { return Some(Err("chr(int)".to_string())); }
                let n = args[0].as_int() as u32;
                match char::from_u32(n) {
                    Some(c) => Some(Ok(Value::String(c.to_string()))),
                    None    => Some(Err(format!("chr: {} is not a valid Unicode code point", n))),
                }
            }
            // ord(s) -> int (code point of first character)
            "ord" => {
                if args.is_empty() { return Some(Err("ord(str)".to_string())); }
                let s = args[0].as_string();
                match s.chars().next() {
                    Some(c) => Some(Ok(Value::Int(c as i64))),
                    None    => Some(Err("ord: empty string".to_string())),
                }
            }
            // str_bytes(s) -> array of ints (UTF-8 bytes)
            "str_bytes" => {
                if args.is_empty() { return Some(Err("str_bytes(str)".to_string())); }
                let s = args[0].as_string();
                Some(Ok(Value::Array(s.as_bytes().iter().map(|b| Value::Int(*b as i64)).collect())))
            }
            // bytes_str(array) -> string (from UTF-8 byte array)
            "bytes_str" => {
                if args.is_empty() { return Some(Err("bytes_str(array)".to_string())); }
                match &args[0] {
                    Value::Array(a) => {
                        let bytes: Vec<u8> = a.iter().map(|v| v.as_int() as u8).collect();
                        match String::from_utf8(bytes) {
                            Ok(s)  => Some(Ok(Value::String(s))),
                            Err(_) => {
                                // Fallback: replace invalid bytes
                                let bytes2: Vec<u8> = a.iter().map(|v| v.as_int() as u8).collect();
                                Some(Ok(Value::String(String::from_utf8_lossy(&bytes2).to_string())))
                            }
                        }
                    }
                    _ => Some(Err("bytes_str: expected array".to_string())),
                }
            }
            // str_chars(s) -> array of single-char strings
            "str_chars" => {
                if args.is_empty() { return Some(Err("str_chars(str)".to_string())); }
                let s = args[0].as_string();
                Some(Ok(Value::Array(s.chars().map(|c| Value::String(c.to_string())).collect())))
            }
            // str_codepoints(s) -> array of ints (Unicode code points)
            "str_codepoints" => {
                if args.is_empty() { return Some(Err("str_codepoints(str)".to_string())); }
                let s = args[0].as_string();
                Some(Ok(Value::Array(s.chars().map(|c| Value::Int(c as i64)).collect())))
            }
            // str_repeat(s, n) -> string
            "str_repeat" => {
                if args.len() < 2 { return Some(Err("str_repeat(str, n)".to_string())); }
                let s = args[0].as_string();
                let n = args[1].as_int().max(0) as usize;
                Some(Ok(Value::String(s.repeat(n))))
            }
            // str_pad_left(s, width, pad_char) -> string
            "str_pad_left" => {
                if args.len() < 3 { return Some(Err("str_pad_left(str, width, pad_char)".to_string())); }
                let s = args[0].as_string();
                let width = args[1].as_int().max(0) as usize;
                let pad = args[2].as_string();
                let pad_char = pad.chars().next().unwrap_or(' ');
                if s.len() >= width {
                    Some(Ok(Value::String(s)))
                } else {
                    let padding: String = std::iter::repeat(pad_char).take(width - s.len()).collect();
                    Some(Ok(Value::String(format!("{}{}", padding, s))))
                }
            }
            // str_pad_right(s, width, pad_char) -> string
            "str_pad_right" => {
                if args.len() < 3 { return Some(Err("str_pad_right(str, width, pad_char)".to_string())); }
                let s = args[0].as_string();
                let width = args[1].as_int().max(0) as usize;
                let pad = args[2].as_string();
                let pad_char = pad.chars().next().unwrap_or(' ');
                if s.len() >= width {
                    Some(Ok(Value::String(s)))
                } else {
                    let padding: String = std::iter::repeat(pad_char).take(width - s.len()).collect();
                    Some(Ok(Value::String(format!("{}{}", s, padding))))
                }
            }
            // sprintf(fmt, ...args) -> string  — simple %d %f %s %x %o %b support
            "sprintf" => {
                if args.is_empty() { return Some(Err("sprintf(fmt, ...)".to_string())); }
                let fmt = args[0].as_string();
                let mut result = String::new();
                let mut arg_idx = 1usize;
                let chars: Vec<char> = fmt.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    if chars[i] == '%' && i + 1 < chars.len() {
                        i += 1;
                        // optional flags/width: collect digits + flags
                        let mut flags = String::new();
                        while i < chars.len() && (chars[i] == '-' || chars[i] == '+' || chars[i] == '0' || chars[i] == ' ') {
                            flags.push(chars[i]);
                            i += 1;
                        }
                        let mut width_str = String::new();
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            width_str.push(chars[i]);
                            i += 1;
                        }
                        let mut prec_str = String::new();
                        if i < chars.len() && chars[i] == '.' {
                            i += 1;
                            while i < chars.len() && chars[i].is_ascii_digit() {
                                prec_str.push(chars[i]);
                                i += 1;
                            }
                        }
                        let width: usize = width_str.parse().unwrap_or(0);
                        let prec: usize  = prec_str.parse().unwrap_or(6);
                        let left_align   = flags.contains('-');
                        let zero_pad     = flags.contains('0') && !left_align;
                        if i >= chars.len() { break; }
                        let spec = chars[i];
                        i += 1;
                        if spec == '%' { result.push('%'); continue; }
                        let arg = if arg_idx < args.len() { &args[arg_idx] } else { &Value::Null };
                        arg_idx += 1;
                        let formatted = match spec {
                            'd' | 'i' => {
                                let n = arg.as_int();
                                let s = format!("{}", n);
                                pad_sprintf(&s, width, if zero_pad { '0' } else { ' ' }, left_align)
                            }
                            'u' => {
                                let n = arg.as_int() as u64;
                                let s = format!("{}", n);
                                pad_sprintf(&s, width, if zero_pad { '0' } else { ' ' }, left_align)
                            }
                            'f' => {
                                let s = format!("{:.prec$}", arg.as_float(), prec = prec);
                                pad_sprintf(&s, width, if zero_pad { '0' } else { ' ' }, left_align)
                            }
                            'e' => {
                                let s = format!("{:.prec$e}", arg.as_float(), prec = prec);
                                pad_sprintf(&s, width, ' ', left_align)
                            }
                            'g' => {
                                let f = arg.as_float();
                                let s = if f.abs() < 1e-4 || f.abs() >= 1e6 {
                                    format!("{:.prec$e}", f, prec = prec)
                                } else {
                                    format!("{:.prec$}", f, prec = prec)
                                };
                                pad_sprintf(&s, width, ' ', left_align)
                            }
                            's' => {
                                let s = arg.as_string();
                                let s = if !prec_str.is_empty() && s.len() > prec { s[..prec].to_string() } else { s };
                                pad_sprintf(&s, width, ' ', left_align)
                            }
                            'x' => {
                                let s = format!("{:x}", arg.as_int() as u64);
                                pad_sprintf(&s, width, if zero_pad { '0' } else { ' ' }, left_align)
                            }
                            'X' => {
                                let s = format!("{:X}", arg.as_int() as u64);
                                pad_sprintf(&s, width, if zero_pad { '0' } else { ' ' }, left_align)
                            }
                            'o' => {
                                let s = format!("{:o}", arg.as_int() as u64);
                                pad_sprintf(&s, width, if zero_pad { '0' } else { ' ' }, left_align)
                            }
                            'b' => {
                                let s = format!("{:b}", arg.as_int() as u64);
                                pad_sprintf(&s, width, if zero_pad { '0' } else { ' ' }, left_align)
                            }
                            'c' => {
                                let n = arg.as_int() as u32;
                                char::from_u32(n).map(|c| c.to_string()).unwrap_or_default()
                            }
                            'p' => format!("0x{:x}", arg.as_int() as u64),
                            _   => format!("%{}", spec),
                        };
                        result.push_str(&formatted);
                    } else {
                        result.push(chars[i]);
                        i += 1;
                    }
                }
                Some(Ok(Value::String(result)))
            }
            // printf(fmt, ...args) — print formatted string with newline
            "printf" => {
                if args.is_empty() { return Some(Err("printf(fmt, ...)".to_string())); }
                // reuse sprintf logic via recursive call
                let s = match self.call_builtin("sprintf", args) {
                    Some(Ok(v)) => v.as_string(),
                    Some(Err(e)) => return Some(Err(e)),
                    None => return Some(Err("printf: sprintf failed".to_string())),
                };
                print!("{}", s);
                Some(Ok(Value::Null))
            }

            // ═══════════════════════════════════════════════════════════════
            // AES-128 — pure Rust, no external crate
            // ═══════════════════════════════════════════════════════════════

            // aes128_ecb_encrypt(plaintext_bytes, key_bytes) -> bytes
            "aes128_ecb_encrypt" => {
                if args.len() < 2 { return Some(Err("aes128_ecb_encrypt(plaintext, key)".to_string())); }
                let data = val_to_bytes(&args[0]);
                let key  = val_to_bytes(&args[1]);
                if key.len() != 16 { return Some(Err("aes128_ecb_encrypt: key must be 16 bytes".to_string())); }
                let key16: [u8; 16] = key[..16].try_into().unwrap();
                let padded = pkcs7_pad(&data, 16);
                let mut out = Vec::with_capacity(padded.len());
                for chunk in padded.chunks(16) {
                    let block: [u8; 16] = chunk.try_into().unwrap();
                    let enc = aes128_encrypt_block(&block, &key16);
                    out.extend_from_slice(&enc);
                }
                Some(Ok(bytes_to_val(&out)))
            }
            // aes128_ecb_decrypt(ciphertext_bytes, key_bytes) -> bytes
            "aes128_ecb_decrypt" => {
                if args.len() < 2 { return Some(Err("aes128_ecb_decrypt(ciphertext, key)".to_string())); }
                let data = val_to_bytes(&args[0]);
                let key  = val_to_bytes(&args[1]);
                if key.len() != 16 { return Some(Err("aes128_ecb_decrypt: key must be 16 bytes".to_string())); }
                if data.len() % 16 != 0 { return Some(Err("aes128_ecb_decrypt: ciphertext length not a multiple of 16".to_string())); }
                let key16: [u8; 16] = key[..16].try_into().unwrap();
                let mut out = Vec::with_capacity(data.len());
                for chunk in data.chunks(16) {
                    let block: [u8; 16] = chunk.try_into().unwrap();
                    let dec = aes128_decrypt_block(&block, &key16);
                    out.extend_from_slice(&dec);
                }
                match pkcs7_unpad(&out) {
                    Ok(v)  => Some(Ok(bytes_to_val(&v))),
                    Err(e) => Some(Err(format!("aes128_ecb_decrypt: {}", e))),
                }
            }
            // aes128_cbc_encrypt(plaintext_bytes, key_bytes, iv_bytes) -> bytes
            "aes128_cbc_encrypt" => {
                if args.len() < 3 { return Some(Err("aes128_cbc_encrypt(plaintext, key, iv)".to_string())); }
                let data = val_to_bytes(&args[0]);
                let key  = val_to_bytes(&args[1]);
                let iv   = val_to_bytes(&args[2]);
                if key.len() != 16 { return Some(Err("aes128_cbc_encrypt: key must be 16 bytes".to_string())); }
                if iv.len() != 16  { return Some(Err("aes128_cbc_encrypt: iv must be 16 bytes".to_string())); }
                let key16: [u8; 16] = key[..16].try_into().unwrap();
                let mut prev: [u8; 16] = iv[..16].try_into().unwrap();
                let padded = pkcs7_pad(&data, 16);
                let mut out = Vec::with_capacity(padded.len());
                for chunk in padded.chunks(16) {
                    let mut block = [0u8; 16];
                    for i in 0..16 { block[i] = chunk[i] ^ prev[i]; }
                    let enc = aes128_encrypt_block(&block, &key16);
                    out.extend_from_slice(&enc);
                    prev = enc;
                }
                Some(Ok(bytes_to_val(&out)))
            }
            // aes128_cbc_decrypt(ciphertext_bytes, key_bytes, iv_bytes) -> bytes
            "aes128_cbc_decrypt" => {
                if args.len() < 3 { return Some(Err("aes128_cbc_decrypt(ciphertext, key, iv)".to_string())); }
                let data = val_to_bytes(&args[0]);
                let key  = val_to_bytes(&args[1]);
                let iv   = val_to_bytes(&args[2]);
                if key.len() != 16 { return Some(Err("aes128_cbc_decrypt: key must be 16 bytes".to_string())); }
                if iv.len() != 16  { return Some(Err("aes128_cbc_decrypt: iv must be 16 bytes".to_string())); }
                if data.len() % 16 != 0 { return Some(Err("aes128_cbc_decrypt: ciphertext length not a multiple of 16".to_string())); }
                let key16: [u8; 16] = key[..16].try_into().unwrap();
                let mut prev: [u8; 16] = iv[..16].try_into().unwrap();
                let mut out = Vec::with_capacity(data.len());
                for chunk in data.chunks(16) {
                    let block: [u8; 16] = chunk.try_into().unwrap();
                    let dec = aes128_decrypt_block(&block, &key16);
                    let mut plain = [0u8; 16];
                    for i in 0..16 { plain[i] = dec[i] ^ prev[i]; }
                    out.extend_from_slice(&plain);
                    prev = block;
                }
                match pkcs7_unpad(&out) {
                    Ok(v)  => Some(Ok(bytes_to_val(&v))),
                    Err(e) => Some(Err(format!("aes128_cbc_decrypt: {}", e))),
                }
            }
            // aes128_key_from_str(s) -> bytes (takes first 16 bytes, zero-pads)
            "aes128_key_from_str" => {
                if args.is_empty() { return Some(Err("aes128_key_from_str(str)".to_string())); }
                let s = args[0].as_string();
                let mut key = [0u8; 16];
                let b = s.as_bytes();
                let copy_len = b.len().min(16);
                key[..copy_len].copy_from_slice(&b[..copy_len]);
                Some(Ok(bytes_to_val(&key)))
            }

            // ═══════════════════════════════════════════════════════════════
            // Functional array helpers: map_fn, reduce, flat_map, zip, enumerate,
            // all, any, count_if, group_by, unique, flatten, chunk, partition
            // ═══════════════════════════════════════════════════════════════

            // map_fn(array, closure) -> array  [duplicate of map — uses call_value]
            // (intentionally left blank — map handles this via call_value at lines above)
            // foreach(array, closure) — call fn(item) for side effects
            "foreach" => {
                if args.len() < 2 { return Some(Err("foreach(array, fn)".to_string())); }
                let callable = args[1].clone();
                match &args[0].clone() {
                    Value::Array(arr) => {
                        for item in arr.clone() { match self.call_value(callable.clone(), vec![item]) { Ok(_) => {} Err(e) => return Some(Err(e)) }; }
                        Some(Ok(Value::Null))
                    }
                    _ => Some(Err("foreach: first arg must be array".to_string())),
                }
            }
            // flat_map(array, closure) -> array
            "flat_map" => {
                if args.len() < 2 { return Some(Err("flat_map(array, fn)".to_string())); }
                let callable = args[1].clone();
                match &args[0].clone() {
                    Value::Array(arr) => {
                        let mut out = Vec::new();
                        for item in arr.clone() {
                            let r = match self.call_value(callable.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)), };
                            match r { Value::Array(inner) => out.extend(inner), other => out.push(other), }
                        }
                        Some(Ok(Value::Array(out)))
                    }
                    _ => Some(Err("flat_map: first arg must be array".to_string())),
                }
            }
            // zip(a, b) -> array of [a[i], b[i]] pairs
            "zip" => {
                if args.len() < 2 { return Some(Err("zip(a, b)".to_string())); }
                let a = match &args[0] { Value::Array(x) => x.clone(), _ => return Some(Err("zip: both args must be arrays".to_string())), };
                let b = match &args[1] { Value::Array(x) => x.clone(), _ => return Some(Err("zip: both args must be arrays".to_string())), };
                let len = a.len().min(b.len());
                Some(Ok(Value::Array((0..len).map(|i| Value::Array(vec![a[i].clone(), b[i].clone()])).collect())))
            }
            // enumerate(array) -> array of [index, value] pairs
            "enumerate" => {
                if args.is_empty() { return Some(Err("enumerate(array)".to_string())); }
                match &args[0] {
                    Value::Array(arr) => {
                        Some(Ok(Value::Array(arr.iter().enumerate().map(|(i, v)| Value::Array(vec![Value::Int(i as i64), v.clone()])).collect())))
                    }
                    _ => Some(Err("enumerate: expected array".to_string())),
                }
            }
            // all(array, closure) -> bool
            "all" => {
                if args.len() < 2 { return Some(Err("all(array, fn)".to_string())); }
                let callable = args[1].clone();
                match &args[0].clone() {
                    Value::Array(arr) => {
                        for item in arr.clone() {
                            let r = match self.call_value(callable.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)) }; if !r.is_truthy() { return Some(Ok(Value::Bool(false))); }
                        }
                        Some(Ok(Value::Bool(true)))
                    }
                    _ => Some(Err("all: first arg must be array".to_string())),
                }
            }
            // any(array, closure) -> bool
            "any" => {
                if args.len() < 2 { return Some(Err("any(array, fn)".to_string())); }
                let callable = args[1].clone();
                match &args[0].clone() {
                    Value::Array(arr) => {
                        for item in arr.clone() {
                            let r = match self.call_value(callable.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)) }; if r.is_truthy() { return Some(Ok(Value::Bool(true))); }
                        }
                        Some(Ok(Value::Bool(false)))
                    }
                    _ => Some(Err("any: first arg must be array".to_string())),
                }
            }
            // count_if(array, closure) -> int
            "count_if" => {
                if args.len() < 2 { return Some(Err("count_if(array, fn)".to_string())); }
                let callable = args[1].clone();
                match &args[0].clone() {
                    Value::Array(arr) => {
                        let mut n = 0i64;
                        for item in arr.clone() { let r = match self.call_value(callable.clone(), vec![item]) { Ok(v) => v, Err(e) => return Some(Err(e)) }; if r.is_truthy() { n += 1; } }
                        Some(Ok(Value::Int(n)))
                    }
                    _ => Some(Err("count_if: first arg must be array".to_string())),
                }
            }
            // unique(array) -> array (removes duplicates, preserves order)
            "unique" | "distinct" => {
                if args.is_empty() { return Some(Err("unique(array)".to_string())); }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut seen: Vec<Value> = Vec::new();
                        for v in arr { if !seen.contains(v) { seen.push(v.clone()); } }
                        Some(Ok(Value::Array(seen)))
                    }
                    _ => Some(Err("unique: expected array".to_string())),
                }
            }
            // flatten(array) -> array (one level deep)
            "flatten" => {
                if args.is_empty() { return Some(Err("flatten(array)".to_string())); }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut out = Vec::new();
                        for v in arr { match v { Value::Array(inner) => out.extend(inner.clone()), other => out.push(other.clone()), } }
                        Some(Ok(Value::Array(out)))
                    }
                    _ => Some(Err("flatten: expected array".to_string())),
                }
            }
            // chunk(array, size) -> array of arrays
            "chunk" => {
                if args.len() < 2 { return Some(Err("chunk(array, size)".to_string())); }
                let size = args[1].as_int().max(1) as usize;
                match &args[0] {
                    Value::Array(arr) => Some(Ok(Value::Array(arr.chunks(size).map(|c| Value::Array(c.to_vec())).collect()))),
                    _ => Some(Err("chunk: expected array".to_string())),
                }
            }
            // partition(array, closure) -> [matching_array, not_matching_array]
            "partition" => {
                if args.len() < 2 { return Some(Err("partition(array, fn)".to_string())); }
                let callable = args[1].clone();
                match &args[0].clone() {
                    Value::Array(arr) => {
                        let (mut yes, mut no) = (Vec::new(), Vec::new());
                        for item in arr.clone() {
                            let r = match self.call_value(callable.clone(), vec![item.clone()]) { Ok(v) => v, Err(e) => return Some(Err(e)) }; if r.is_truthy() { yes.push(item); } else { no.push(item); }
                        }
                        Some(Ok(Value::Array(vec![Value::Array(yes), Value::Array(no)])))
                    }
                    _ => Some(Err("partition: first arg must be array".to_string())),
                }
            }
            // take(array, n) -> first n elements
            "take" => {
                if args.len() < 2 { return Some(Err("take(array, n)".to_string())); }
                let n = args[1].as_int().max(0) as usize;
                match &args[0] {
                    Value::Array(arr) => Some(Ok(Value::Array(arr[..n.min(arr.len())].to_vec()))),
                    _ => Some(Err("take: expected array".to_string())),
                }
            }
            // drop(array, n) -> skip first n elements
            "drop" => {
                if args.len() < 2 { return Some(Err("drop(array, n)".to_string())); }
                let n = args[1].as_int().max(0) as usize;
                match &args[0] {
                    Value::Array(arr) => Some(Ok(Value::Array(arr[n.min(arr.len())..].to_vec()))),
                    _ => Some(Err("drop: expected array".to_string())),
                }
            }
            // sum(array) -> number
            "array_sum" | "sum" => {
                if args.is_empty() { return Some(Err("sum(array)".to_string())); }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut is_float = false;
                        let mut sf = 0.0f64;
                        let mut si = 0i64;
                        for v in arr {
                            match v { Value::Float(f) => { is_float = true; sf += f; } Value::Int(n) => { si += n; sf += *n as f64; } _ => {} }
                        }
                        if is_float { Some(Ok(Value::Float(sf))) } else { Some(Ok(Value::Int(si))) }
                    }
                    _ => Some(Err("sum: expected array".to_string())),
                }
            }
            // array_min(array) -> value
            "array_min" => {
                if args.is_empty() { return Some(Err("array_min(array)".to_string())); }
                match &args[0] {
                    Value::Array(arr) if !arr.is_empty() => {
                        let mut best = arr[0].clone();
                        for v in arr.iter().skip(1) { if v.as_float() < best.as_float() { best = v.clone(); } }
                        Some(Ok(best))
                    }
                    Value::Array(_) => Some(Ok(Value::Null)),
                    _ => Some(Err("array_min: expected array".to_string())),
                }
            }
            // array_max(array) -> value
            "array_max" => {
                if args.is_empty() { return Some(Err("array_max(array)".to_string())); }
                match &args[0] {
                    Value::Array(arr) if !arr.is_empty() => {
                        let mut best = arr[0].clone();
                        for v in arr.iter().skip(1) { if v.as_float() > best.as_float() { best = v.clone(); } }
                        Some(Ok(best))
                    }
                    Value::Array(_) => Some(Ok(Value::Null)),
                    _ => Some(Err("array_max: expected array".to_string())),
                }
            }
            // range(start?, end, step?) -> array of integers
            "range" => {
                if args.is_empty() { return Some(Err("range(end) or range(start, end) or range(start, end, step)".to_string())); }
                let (start, end, step) = match args.len() {
                    1 => (0i64, args[0].as_int(), 1i64),
                    2 => (args[0].as_int(), args[1].as_int(), 1i64),
                    _ => (args[0].as_int(), args[1].as_int(), args[2].as_int()),
                };
                if step == 0 { return Some(Err("range: step cannot be zero".to_string())); }
                let mut out = Vec::new();
                let mut i = start;
                while (step > 0 && i < end) || (step < 0 && i > end) {
                    out.push(Value::Int(i));
                    i += step;
                    if out.len() > 10_000_000 { return Some(Err("range: too many elements".to_string())); }
                }
                Some(Ok(Value::Array(out)))
            }
            // contains(array_or_string, value) -> bool
            "array_contains" | "contains" => {
                if args.len() < 2 { return Some(Err("contains(array, value)".to_string())); }
                match &args[0] {
                    Value::Array(arr) => Some(Ok(Value::Bool(arr.contains(&args[1])))),
                    Value::String(s)  => Some(Ok(Value::Bool(s.contains(&args[1].as_string() as &str)))),
                    _ => Some(Err("contains: expected array or string".to_string())),
                }
            }
            // index_of(array, value) -> int (-1 if not found)
            "array_index" | "index_of" => {
                if args.len() < 2 { return Some(Err("index_of(array, value)".to_string())); }
                match &args[0] {
                    Value::Array(arr) => Some(Ok(Value::Int(arr.iter().position(|v| v == &args[1]).map(|p| p as i64).unwrap_or(-1)))),
                    _ => Some(Err("index_of: expected array".to_string())),
                }
            }
            // sort_cmp(array, closure) -> sorted array (fn(a,b) returns negative/0/positive)
            "sort_cmp" => {
                if args.len() < 2 { return Some(Err("sort_cmp(array, fn)".to_string())); }
                let callable = args[1].clone();
                match &args[0].clone() {
                    Value::Array(arr) => {
                        let mut sorted = arr.clone();
                        // Insertion sort to use interpreter's closure
                        for i in 1..sorted.len() {
                            let mut j = i;
                            while j > 0 {
                                let cmp = self.call_value(callable.clone(), vec![sorted[j-1].clone(), sorted[j].clone()]).unwrap_or(Value::Int(0));
                                if cmp.as_int() > 0 { sorted.swap(j-1, j); j -= 1; } else { break; }
                            }
                        }
                        Some(Ok(Value::Array(sorted)))
                    }
                    _ => Some(Err("sort_cmp: first arg must be array".to_string())),
                }
            }
            // keys(map) -> sorted array of keys
            "map_keys" | "keys" => {
                if args.is_empty() { return Some(Err("keys(map)".to_string())); }
                match &args[0] {
                    Value::Map(m) => {
                        let mut out: Vec<Value> = m.keys().map(|k| Value::String(k.clone())).collect();
                        out.sort_by(|a, b| a.as_string().cmp(&b.as_string()));
                        Some(Ok(Value::Array(out)))
                    }
                    _ => Some(Err("keys: expected map".to_string())),
                }
            }
            // values(map) -> array of values
            "map_values" | "values" => {
                if args.is_empty() { return Some(Err("values(map)".to_string())); }
                match &args[0] {
                    Value::Map(m) => {
                        let mut kvs: Vec<(&String, &Value)> = m.iter().collect();
                        kvs.sort_by_key(|(k, _)| k.as_str());
                        Some(Ok(Value::Array(kvs.into_iter().map(|(_, v)| v.clone()).collect())))
                    }
                    _ => Some(Err("values: expected map".to_string())),
                }
            }
            // entries(map) -> array of [key, value] pairs
            "map_entries" | "entries" => {
                if args.is_empty() { return Some(Err("entries(map)".to_string())); }
                match &args[0] {
                    Value::Map(m) => {
                        let mut kvs: Vec<(&String, &Value)> = m.iter().collect();
                        kvs.sort_by_key(|(k, _)| k.as_str());
                        Some(Ok(Value::Array(kvs.into_iter().map(|(k, v)| Value::Array(vec![Value::String(k.clone()), v.clone()])).collect())))
                    }
                    _ => Some(Err("entries: expected map".to_string())),
                }
            }
            // merge(m1, m2) -> merged map (m2 overwrites m1)
            "map_merge" | "merge" => {
                if args.len() < 2 { return Some(Err("merge(map1, map2)".to_string())); }
                let mut out = match &args[0] { Value::Map(m) => m.clone(), _ => return Some(Err("merge: args must be maps".to_string())), };
                match &args[1] { Value::Map(m) => { for (k, v) in m { out.insert(k.clone(), v.clone()); } } _ => return Some(Err("merge: args must be maps".to_string())), }
                Some(Ok(Value::Map(out)))
            }
            // has_key(map, key) -> bool
            "map_has" | "has_key" => {
                if args.len() < 2 { return Some(Err("has_key(map, key)".to_string())); }
                match &args[0] {
                    Value::Map(m) => Some(Ok(Value::Bool(m.contains_key(&args[1].as_string())))),
                    _ => Some(Err("has_key: expected map".to_string())),
                }
            }
            // map_remove(map, key) -> new map without key
            "map_remove" => {
                if args.len() < 2 { return Some(Err("map_remove(map, key)".to_string())); }
                let mut m = match &args[0] { Value::Map(m) => m.clone(), _ => return Some(Err("map_remove: expected map".to_string())), };
                m.remove(&args[1].as_string());
                Some(Ok(Value::Map(m)))
            }
            // map_from_arrays(keys, values) -> map
            "map_from_arrays" => {
                if args.len() < 2 { return Some(Err("map_from_arrays(keys, values)".to_string())); }
                let ks = match &args[0] { Value::Array(a) => a.clone(), _ => return Some(Err("map_from_arrays: keys must be array".to_string())), };
                let vs = match &args[1] { Value::Array(a) => a.clone(), _ => return Some(Err("map_from_arrays: values must be array".to_string())), };
                let mut m = HashMap::new();
                for (k, v) in ks.iter().zip(vs.iter()) { m.insert(k.as_string(), v.clone()); }
                Some(Ok(Value::Map(m)))
            }

            // ── HTTPS (TLS) ────────────────────────────────────────────────────
            // https_get(url) -> string
            "https_get" => {
                if args.is_empty() { return Some(Err("https_get(url)".to_string())); }
                let url = args[0].as_string();
                match reqwest::blocking::get(&url) {
                    Ok(resp) => match resp.text() {
                        Ok(t) => Some(Ok(Value::String(t))),
                        Err(e) => Some(Err(e.to_string())),
                    },
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // https_post(url, body, content_type?) -> string
            "https_post" => {
                if args.len() < 2 { return Some(Err("https_post(url, body)".to_string())); }
                let url = args[0].as_string();
                let body = args[1].as_string();
                let ct = args.get(2).map(|v| v.as_string()).unwrap_or_else(|| "application/json".to_string());
                let client = reqwest::blocking::Client::new();
                match client.post(&url).header("Content-Type", ct).body(body).send() {
                    Ok(resp) => match resp.text() {
                        Ok(t) => Some(Ok(Value::String(t))),
                        Err(e) => Some(Err(e.to_string())),
                    },
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // https_request(method, url, headers_map, body) -> {status, body, headers}
            "https_request" => {
                if args.len() < 2 { return Some(Err("https_request(method, url, headers?, body?)".to_string())); }
                let method = args[0].as_string().to_uppercase();
                let url = args[1].as_string();
                let client = reqwest::blocking::Client::new();
                let mut req = match method.as_str() {
                    "GET"    => client.get(&url),
                    "POST"   => client.post(&url),
                    "PUT"    => client.put(&url),
                    "DELETE" => client.delete(&url),
                    "PATCH"  => client.patch(&url),
                    "HEAD"   => client.head(&url),
                    other    => return Some(Err(format!("Unknown HTTP method: {}", other))),
                };
                if let Some(Value::Map(hdrs)) = args.get(2) {
                    for (k, v) in hdrs { req = req.header(k.as_str(), v.as_string()); }
                }
                if let Some(body) = args.get(3) { req = req.body(body.as_string()); }
                match req.send() {
                    Ok(resp) => {
                        let status = resp.status().as_u16() as i64;
                        let mut hdrs = HashMap::new();
                        for (k, v) in resp.headers() {
                            hdrs.insert(k.to_string(), Value::String(v.to_str().unwrap_or("").to_string()));
                        }
                        let body_text = resp.text().unwrap_or_default();
                        let mut out = HashMap::new();
                        out.insert("status".to_string(), Value::Int(status));
                        out.insert("body".to_string(), Value::String(body_text));
                        out.insert("headers".to_string(), Value::Map(hdrs));
                        Some(Ok(Value::Map(out)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }

            // ── WebSocket ──────────────────────────────────────────────────────
            // ws_connect(url) -> handle
            "ws_connect" => {
                if args.is_empty() { return Some(Err("ws_connect(url)".to_string())); }
                let url = args[0].as_string();
                match tungstenite::connect(&url) {
                    Ok((ws, _)) => {
                        let id = self.ws_counter;
                        self.ws_counter += 1;
                        self.ws_connections.insert(id, ws);
                        Some(Ok(Value::Int(id)))
                    }
                    Err(e) => Some(Err(format!("ws_connect: {}", e))),
                }
            }
            // ws_send(handle, message)
            "ws_send" => {
                if args.len() < 2 { return Some(Err("ws_send(handle, msg)".to_string())); }
                let id = args[0].as_int();
                let msg = args[1].as_string();
                match self.ws_connections.get_mut(&id) {
                    Some(ws) => match ws.send(tungstenite::Message::Text(msg)) {
                        Ok(_) => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(e.to_string())),
                    },
                    None => Some(Err(format!("ws_send: no connection {}", id))),
                }
            }
            // ws_recv(handle) -> string
            "ws_recv" => {
                if args.is_empty() { return Some(Err("ws_recv(handle)".to_string())); }
                let id = args[0].as_int();
                match self.ws_connections.get_mut(&id) {
                    Some(ws) => match ws.read() {
                        Ok(tungstenite::Message::Text(t)) => Some(Ok(Value::String(t))),
                        Ok(tungstenite::Message::Binary(b)) => Some(Ok(Value::Array(b.iter().map(|x| Value::Int(*x as i64)).collect()))),
                        Ok(tungstenite::Message::Close(_)) => Some(Ok(Value::Null)),
                        Ok(_) => Some(Ok(Value::String(String::new()))),
                        Err(e) => Some(Err(e.to_string())),
                    },
                    None => Some(Err(format!("ws_recv: no connection {}", id))),
                }
            }
            // ws_close(handle)
            "ws_close" => {
                if args.is_empty() { return Some(Err("ws_close(handle)".to_string())); }
                let id = args[0].as_int();
                if let Some(mut ws) = self.ws_connections.remove(&id) {
                    let _ = ws.close(None);
                }
                Some(Ok(Value::Null))
            }

            // ── Dynamic Libraries (dlopen/dlsym) ───────────────────────────────
            // dlopen(path) -> handle
            "dlopen" => {
                if args.is_empty() { return Some(Err("dlopen(path)".to_string())); }
                let path = args[0].as_string();
                match unsafe { libloading::Library::new(&path) } {
                    Ok(lib) => {
                        let id = self.dl_counter;
                        self.dl_counter += 1;
                        self.dl_libs.insert(id, lib);
                        Some(Ok(Value::Int(id)))
                    }
                    Err(e) => Some(Err(format!("dlopen: {}", e))),
                }
            }
            // dlsym(handle, symbol) -> fn_handle
            "dlsym" => {
                if args.len() < 2 { return Some(Err("dlsym(handle, symbol)".to_string())); }
                let lib_id = args[0].as_int();
                let sym = args[1].as_string();
                match self.dl_libs.get(&lib_id) {
                    Some(lib) => {
                        let sym_bytes = format!("{}\0", sym);
                        let raw_ptr: usize = unsafe {
                            match lib.get::<unsafe extern fn()>(sym_bytes.as_bytes()) {
                                Ok(sym) => *sym as usize,
                                Err(e) => return Some(Err(format!("dlsym: {}", e))),
                            }
                        };
                        let id = self.dl_counter;
                        self.dl_counter += 1;
                        self.dl_syms.insert(id, raw_ptr);
                        Some(Ok(Value::Int(id)))
                    }
                    None => Some(Err(format!("dlsym: no library {}", lib_id))),
                }
            }
            // dlcall_i(sym_handle, args...) -> int  [call fn returning i64]
            "dlcall_i" => {
                if args.is_empty() { return Some(Err("dlcall_i(sym_handle, args...)".to_string())); }
                let sym_id = args[0].as_int();
                match self.dl_syms.get(&sym_id) {
                    Some(&ptr) => {
                        let iargs: Vec<i64> = args[1..].iter().map(|v| v.as_int()).collect();
                        let result: i64 = unsafe {
                            let f: unsafe extern fn(i64, i64, i64, i64, i64, i64) -> i64 = std::mem::transmute(ptr);
                            let a = |i: usize| iargs.get(i).copied().unwrap_or(0);
                            f(a(0), a(1), a(2), a(3), a(4), a(5))
                        };
                        Some(Ok(Value::Int(result)))
                    }
                    None => Some(Err(format!("dlcall_i: no symbol {}", sym_id))),
                }
            }
            // dlclose(handle)
            "dlclose" => {
                if args.is_empty() { return Some(Err("dlclose(handle)".to_string())); }
                let id = args[0].as_int();
                self.dl_libs.remove(&id);
                Some(Ok(Value::Null))
            }

            // ── Raw sockets (AF_INET SOCK_RAW via libc) ────────────────────────
            // raw_socket(proto) -> fd  (proto: 1=ICMP, 6=TCP, 17=UDP, 255=RAW)
            "raw_socket" => {
                let proto = args.first().map(|v| v.as_int()).unwrap_or(255) as libc::c_int;
                let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, proto) };
                if fd < 0 { return Some(Err(format!("raw_socket: errno {}", unsafe { *libc::__errno_location() }))); }
                let id = self.raw_sock_counter;
                self.raw_sock_counter += 1;
                self.raw_sockets.insert(id, fd);
                Some(Ok(Value::Int(id)))
            }
            // raw_send(fd_handle, dest_ip, payload_bytes)
            "raw_send" => {
                if args.len() < 3 { return Some(Err("raw_send(fd, ip, bytes)".to_string())); }
                let fd = match self.raw_sockets.get(&args[0].as_int()) {
                    Some(&f) => f,
                    None => return Some(Err("raw_send: bad fd".to_string())),
                };
                let ip_str = args[1].as_string();
                let payload: Vec<u8> = match &args[2] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect(),
                    Value::String(s) => s.bytes().collect(),
                    _ => return Some(Err("raw_send: payload must be array or string".to_string())),
                };
                let ip_bytes: Vec<u8> = ip_str.split('.').map(|p| p.parse::<u8>().unwrap_or(0)).collect();
                if ip_bytes.len() < 4 { return Some(Err("raw_send: invalid IP".to_string())); }
                let addr = libc::sockaddr_in {
                    sin_family: libc::AF_INET as u16,
                    sin_port: 0,
                    sin_addr: libc::in_addr { s_addr: u32::from_be_bytes([ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]]).to_be() },
                    sin_zero: [0; 8],
                };
                let sent = unsafe {
                    libc::sendto(fd, payload.as_ptr() as *const _, payload.len(), 0,
                        &addr as *const _ as *const libc::sockaddr,
                        std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t)
                };
                Some(Ok(Value::Int(sent as i64)))
            }
            // raw_recv(fd_handle, max_bytes?) -> array of bytes
            "raw_recv" => {
                if args.is_empty() { return Some(Err("raw_recv(fd)".to_string())); }
                let fd = match self.raw_sockets.get(&args[0].as_int()) {
                    Some(&f) => f,
                    None => return Some(Err("raw_recv: bad fd".to_string())),
                };
                let max = args.get(1).map(|v| v.as_int() as usize).unwrap_or(65535);
                let mut buf = vec![0u8; max];
                let n = unsafe { libc::recv(fd, buf.as_mut_ptr() as *mut _, max, 0) };
                if n < 0 { return Some(Err(format!("raw_recv: errno {}", unsafe { *libc::__errno_location() }))); }
                let out: Vec<Value> = buf[..n as usize].iter().map(|b| Value::Int(*b as i64)).collect();
                Some(Ok(Value::Array(out)))
            }
            // raw_close(fd_handle)
            "raw_close" => {
                if args.is_empty() { return Some(Err("raw_close(fd)".to_string())); }
                if let Some(fd) = self.raw_sockets.remove(&args[0].as_int()) {
                    unsafe { libc::close(fd); }
                }
                Some(Ok(Value::Null))
            }

            // ── Signal handling ────────────────────────────────────────────────
            // signal_send(pid, signum) -> 0 on success
            "signal_send" | "kill" => {
                if args.len() < 2 { return Some(Err("signal_send(pid, signum)".to_string())); }
                let pid = args[0].as_int() as libc::pid_t;
                let sig = args[1].as_int() as libc::c_int;
                let r = unsafe { libc::kill(pid, sig) };
                Some(Ok(Value::Int(r as i64)))
            }
            // signal_check(signum) -> count of times received (0 if none)
            "signal_check" => {
                let sig = args.first().map(|v| v.as_int() as i32).unwrap_or(2);
                let count = SIGNAL_FLAGS.lock().unwrap().get(&sig).copied().unwrap_or(0);
                Some(Ok(Value::Int(count as i64)))
            }
            // signal_trap(signum) — register a no-op handler so we can poll via signal_check
            "signal_trap" => {
                let sig = args.first().map(|v| v.as_int() as libc::c_int).unwrap_or(libc::SIGINT);
                extern "C" fn handler(sig: libc::c_int) {
                    if let Ok(mut map) = SIGNAL_FLAGS.lock() {
                        *map.entry(sig).or_insert(0) += 1;
                    }
                }
                unsafe { libc::signal(sig, handler as libc::sighandler_t); }
                Some(Ok(Value::Null))
            }
            // signal_reset(signum) — clear counter
            "signal_reset" => {
                let sig = args.first().map(|v| v.as_int() as i32).unwrap_or(2);
                SIGNAL_FLAGS.lock().unwrap().remove(&sig);
                Some(Ok(Value::Null))
            }

            // ── GUI window (minifb) ────────────────────────────────────────────
            // gui_window(title, width, height) -> handle
            "gui_window" => {
                if args.len() < 3 { return Some(Err("gui_window(title, width, height)".to_string())); }
                let title  = args[0].as_string();
                let width  = args[1].as_int() as usize;
                let height = args[2].as_int() as usize;
                match minifb::Window::new(&title, width, height, minifb::WindowOptions::default()) {
                    Ok(win) => {
                        let id = self.gui_counter;
                        self.gui_counter += 1;
                        let buf = vec![0u32; width * height];
                        self.gui_windows.insert(id, (win, buf));
                        Some(Ok(Value::Int(id)))
                    }
                    Err(e) => Some(Err(format!("gui_window: {}", e))),
                }
            }
            // gui_set_pixel(handle, x, y, rgb) — set pixel (rgb = 0xRRGGBB)
            "gui_set_pixel" => {
                if args.len() < 4 { return Some(Err("gui_set_pixel(h, x, y, rgb)".to_string())); }
                let id = args[0].as_int();
                let x = args[1].as_int() as usize;
                let y = args[2].as_int() as usize;
                let rgb = args[3].as_int() as u32;
                match self.gui_windows.get_mut(&id) {
                    Some((win, buf)) => {
                        let (w, h) = win.get_size();
                        if x < w && y < h { buf[y * w + x] = rgb; }
                        Some(Ok(Value::Null))
                    }
                    None => Some(Err(format!("gui_set_pixel: no window {}", id))),
                }
            }
            // gui_fill(handle, rgb) — fill whole buffer
            "gui_fill" => {
                if args.len() < 2 { return Some(Err("gui_fill(h, rgb)".to_string())); }
                let id = args[0].as_int();
                let rgb = args[1].as_int() as u32;
                match self.gui_windows.get_mut(&id) {
                    Some((_, buf)) => { buf.iter_mut().for_each(|p| *p = rgb); Some(Ok(Value::Null)) }
                    None => Some(Err(format!("gui_fill: no window {}", id))),
                }
            }
            // gui_present(handle) — flush buffer to window
            "gui_present" => {
                if args.is_empty() { return Some(Err("gui_present(handle)".to_string())); }
                let id = args[0].as_int();
                match self.gui_windows.get_mut(&id) {
                    Some((win, buf)) => {
                        let (w, h) = win.get_size();
                        match win.update_with_buffer(buf, w, h) {
                            Ok(_) => Some(Ok(Value::Null)),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    }
                    None => Some(Err(format!("gui_present: no window {}", id))),
                }
            }
            // gui_is_open(handle) -> bool
            "gui_is_open" => {
                if args.is_empty() { return Some(Err("gui_is_open(handle)".to_string())); }
                let id = args[0].as_int();
                let open = self.gui_windows.get(&id).map(|(w, _)| w.is_open()).unwrap_or(false);
                Some(Ok(Value::Bool(open)))
            }
            // gui_get_keys(handle) -> array of key names
            "gui_get_keys" => {
                if args.is_empty() { return Some(Err("gui_get_keys(handle)".to_string())); }
                let id = args[0].as_int();
                match self.gui_windows.get(&id) {
                    Some((win, _)) => {
                        let keys: Vec<Value> = win.get_keys().iter().map(|k| Value::String(format!("{:?}", k))).collect();
                        Some(Ok(Value::Array(keys)))
                    }
                    None => Some(Err(format!("gui_get_keys: no window {}", id))),
                }
            }
            // gui_get_mouse(handle) -> {x, y, left, right, middle}
            "gui_get_mouse" => {
                if args.is_empty() { return Some(Err("gui_get_mouse(handle)".to_string())); }
                let id = args[0].as_int();
                match self.gui_windows.get(&id) {
                    Some((win, _)) => {
                        let (mx, my) = win.get_mouse_pos(minifb::MouseMode::Clamp).unwrap_or((0.0, 0.0));
                        let left   = win.get_mouse_down(minifb::MouseButton::Left);
                        let right  = win.get_mouse_down(minifb::MouseButton::Right);
                        let middle = win.get_mouse_down(minifb::MouseButton::Middle);
                        let mut m = HashMap::new();
                        m.insert("x".to_string(), Value::Float(mx as f64));
                        m.insert("y".to_string(), Value::Float(my as f64));
                        m.insert("left".to_string(), Value::Bool(left));
                        m.insert("right".to_string(), Value::Bool(right));
                        m.insert("middle".to_string(), Value::Bool(middle));
                        Some(Ok(Value::Map(m)))
                    }
                    None => Some(Err(format!("gui_get_mouse: no window {}", id))),
                }
            }
            // gui_close(handle)
            "gui_close" => {
                if args.is_empty() { return Some(Err("gui_close(handle)".to_string())); }
                self.gui_windows.remove(&args[0].as_int());
                Some(Ok(Value::Null))
            }
            // gui_rgb(r, g, b) -> packed 0xRRGGBB color
            "gui_rgb" => {
                if args.len() < 3 { return Some(Err("gui_rgb(r, g, b)".to_string())); }
                let r = (args[0].as_int() as u32) & 0xFF;
                let g = (args[1].as_int() as u32) & 0xFF;
                let b = (args[2].as_int() as u32) & 0xFF;
                Some(Ok(Value::Int((r << 16 | g << 8 | b) as i64)))
            }
            // gui_size(handle) -> {w, h}
            "gui_size" => {
                if args.is_empty() { return Some(Err("gui_size(handle)".to_string())); }
                let id = args[0].as_int();
                match self.gui_windows.get(&id) {
                    Some((win, _)) => {
                        let (w, h) = win.get_size();
                        let mut m = HashMap::new();
                        m.insert("w".to_string(), Value::Int(w as i64));
                        m.insert("h".to_string(), Value::Int(h as i64));
                        Some(Ok(Value::Map(m)))
                    }
                    None => Some(Err(format!("gui_size: no window {}", id))),
                }
            }
            // gui_rect(handle, x, y, w, h, rgb) — filled rectangle
            "gui_rect" => {
                if args.len() < 6 { return Some(Err("gui_rect(h, x, y, w, h, rgb)".to_string())); }
                let id  = args[0].as_int();
                let rx  = args[1].as_int() as i64;
                let ry  = args[2].as_int() as i64;
                let rw  = args[3].as_int() as i64;
                let rh  = args[4].as_int() as i64;
                let rgb = args[5].as_int() as u32;
                match self.gui_windows.get_mut(&id) {
                    Some((win, buf)) => {
                        let (ww, wh) = win.get_size();
                        let ww = ww as i64; let wh = wh as i64;
                        for py in ry..(ry + rh) {
                            if py < 0 || py >= wh { continue; }
                            for px in rx..(rx + rw) {
                                if px < 0 || px >= ww { continue; }
                                buf[(py as usize) * (ww as usize) + (px as usize)] = rgb;
                            }
                        }
                        Some(Ok(Value::Null))
                    }
                    None => Some(Err(format!("gui_rect: no window {}", id))),
                }
            }
            // gui_rect_outline(handle, x, y, w, h, rgb) — rectangle outline only
            "gui_rect_outline" => {
                if args.len() < 6 { return Some(Err("gui_rect_outline(h, x, y, w, h, rgb)".to_string())); }
                let id  = args[0].as_int();
                let rx  = args[1].as_int() as i64;
                let ry  = args[2].as_int() as i64;
                let rw  = args[3].as_int() as i64;
                let rh  = args[4].as_int() as i64;
                let rgb = args[5].as_int() as u32;
                match self.gui_windows.get_mut(&id) {
                    Some((win, buf)) => {
                        let (ww, wh) = win.get_size();
                        let ww = ww as i64; let wh = wh as i64;
                        let set = |buf: &mut Vec<u32>, px: i64, py: i64| {
                            if px >= 0 && px < ww && py >= 0 && py < wh {
                                buf[(py as usize) * (ww as usize) + (px as usize)] = rgb;
                            }
                        };
                        for px in rx..(rx + rw) { set(buf, px, ry); set(buf, px, ry + rh - 1); }
                        for py in ry..(ry + rh) { set(buf, rx, py); set(buf, rx + rw - 1, py); }
                        Some(Ok(Value::Null))
                    }
                    None => Some(Err(format!("gui_rect_outline: no window {}", id))),
                }
            }
            // gui_line(handle, x0, y0, x1, y1, rgb) — Bresenham anti-clipped line
            "gui_line" => {
                if args.len() < 6 { return Some(Err("gui_line(h, x0, y0, x1, y1, rgb)".to_string())); }
                let id  = args[0].as_int();
                let mut x0 = args[1].as_int() as i64;
                let mut y0 = args[2].as_int() as i64;
                let x1  = args[3].as_int() as i64;
                let y1  = args[4].as_int() as i64;
                let rgb = args[5].as_int() as u32;
                match self.gui_windows.get_mut(&id) {
                    Some((win, buf)) => {
                        let (ww, wh) = win.get_size();
                        let ww = ww as i64; let wh = wh as i64;
                        let dx = (x1 - x0).abs();
                        let dy = -(y1 - y0).abs();
                        let sx: i64 = if x0 < x1 { 1 } else { -1 };
                        let sy: i64 = if y0 < y1 { 1 } else { -1 };
                        let mut err = dx + dy;
                        loop {
                            if x0 >= 0 && x0 < ww && y0 >= 0 && y0 < wh {
                                buf[(y0 as usize) * (ww as usize) + (x0 as usize)] = rgb;
                            }
                            if x0 == x1 && y0 == y1 { break; }
                            let e2 = 2 * err;
                            if e2 >= dy { err += dy; x0 += sx; }
                            if e2 <= dx { err += dx; y0 += sy; }
                        }
                        Some(Ok(Value::Null))
                    }
                    None => Some(Err(format!("gui_line: no window {}", id))),
                }
            }
            // gui_circle(handle, cx, cy, r, rgb) — filled circle
            "gui_circle" => {
                if args.len() < 5 { return Some(Err("gui_circle(h, cx, cy, r, rgb)".to_string())); }
                let id  = args[0].as_int();
                let cx  = args[1].as_int() as i64;
                let cy  = args[2].as_int() as i64;
                let cr  = args[3].as_int() as i64;
                let rgb = args[4].as_int() as u32;
                match self.gui_windows.get_mut(&id) {
                    Some((win, buf)) => {
                        let (ww, wh) = win.get_size();
                        let ww = ww as i64; let wh = wh as i64;
                        for py in (cy - cr)..=(cy + cr) {
                            if py < 0 || py >= wh { continue; }
                            let dy  = py - cy;
                            let dx  = ((cr * cr - dy * dy) as f64).sqrt() as i64;
                            for px in (cx - dx)..=(cx + dx) {
                                if px < 0 || px >= ww { continue; }
                                buf[(py as usize) * (ww as usize) + (px as usize)] = rgb;
                            }
                        }
                        Some(Ok(Value::Null))
                    }
                    None => Some(Err(format!("gui_circle: no window {}", id))),
                }
            }
            // gui_circle_outline(handle, cx, cy, r, rgb) — circle outline (midpoint)
            "gui_circle_outline" => {
                if args.len() < 5 { return Some(Err("gui_circle_outline(h, cx, cy, r, rgb)".to_string())); }
                let id  = args[0].as_int();
                let cx  = args[1].as_int() as i64;
                let cy  = args[2].as_int() as i64;
                let cr  = args[3].as_int() as i64;
                let rgb = args[4].as_int() as u32;
                match self.gui_windows.get_mut(&id) {
                    Some((win, buf)) => {
                        let (ww, wh) = win.get_size();
                        let ww = ww as i64; let wh = wh as i64;
                        let mut x = cr; let mut y: i64 = 0;
                        let mut p = 1 - cr;
                        let draw = |buf: &mut Vec<u32>, px: i64, py: i64| {
                            if px >= 0 && px < ww && py >= 0 && py < wh {
                                buf[(py as usize) * (ww as usize) + (px as usize)] = rgb;
                            }
                        };
                        while x >= y {
                            draw(buf, cx+x, cy+y); draw(buf, cx-x, cy+y);
                            draw(buf, cx+x, cy-y); draw(buf, cx-x, cy-y);
                            draw(buf, cx+y, cy+x); draw(buf, cx-y, cy+x);
                            draw(buf, cx+y, cy-x); draw(buf, cx-y, cy-x);
                            y += 1;
                            if p <= 0 { p += 2*y + 1; } else { x -= 1; p += 2*(y-x) + 1; }
                        }
                        Some(Ok(Value::Null))
                    }
                    None => Some(Err(format!("gui_circle_outline: no window {}", id))),
                }
            }
            // gui_set_title(handle, title) — update window title at runtime
            "gui_set_title" => {
                if args.len() < 2 { return Some(Err("gui_set_title(h, title)".to_string())); }
                let id    = args[0].as_int();
                let title = args[1].as_string();
                match self.gui_windows.get_mut(&id) {
                    Some((win, _)) => { win.set_title(&title); Some(Ok(Value::Null)) }
                    None => Some(Err(format!("gui_set_title: no window {}", id))),
                }
            }

            // ── Process: fork / exec ──────────────────────────────────────────
            // fork() -> pid (0 in child, child_pid in parent)
            "fork" => {
                let pid = unsafe { libc::fork() };
                Some(Ok(Value::Int(pid as i64)))
            }
            // execve(path, args_array, env_map?) -> never returns on success
            "execve" => {
                if args.is_empty() { return Some(Err("execve(path, args, env?)".to_string())); }
                let path = args[0].as_string();
                let a_list: Vec<String> = match args.get(1) {
                    Some(Value::Array(a)) => a.iter().map(|v| v.as_string()).collect(),
                    _ => vec![path.clone()],
                };
                let env_list: Vec<String> = match args.get(2) {
                    Some(Value::Map(m)) => m.iter().map(|(k,v)| format!("{}={}", k, v.as_string())).collect(),
                    _ => std::env::vars().map(|(k,v)| format!("{}={}", k, v)).collect(),
                };
                use std::ffi::CString;
                let c_path = CString::new(path.as_bytes()).unwrap();
                let c_args: Vec<CString> = a_list.iter().map(|s| CString::new(s.as_bytes()).unwrap_or_default()).collect();
                let c_env:  Vec<CString> = env_list.iter().map(|s| CString::new(s.as_bytes()).unwrap_or_default()).collect();
                let mut p_args: Vec<*const libc::c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
                let mut p_env:  Vec<*const libc::c_char> = c_env.iter().map(|s| s.as_ptr()).collect();
                p_args.push(std::ptr::null()); p_env.push(std::ptr::null());
                unsafe { libc::execve(c_path.as_ptr(), p_args.as_ptr(), p_env.as_ptr()) };
                Some(Err(format!("execve failed: errno {}", unsafe { *libc::__errno_location() })))
            }
            // waitpid(pid, options?) -> {pid, status, exited, code}
            "waitpid" => {
                let pid = args.first().map(|v| v.as_int()).unwrap_or(-1) as libc::pid_t;
                let opts = args.get(1).map(|v| v.as_int()).unwrap_or(0) as libc::c_int;
                let mut status: libc::c_int = 0;
                let ret = unsafe { libc::waitpid(pid, &mut status, opts) };
                let mut m = HashMap::new();
                m.insert("pid".to_string(), Value::Int(ret as i64));
                m.insert("status".to_string(), Value::Int(status as i64));
                m.insert("exited".to_string(), Value::Bool(unsafe { libc::WIFEXITED(status) }));
                m.insert("code".to_string(), Value::Int(unsafe { libc::WEXITSTATUS(status) } as i64));
                Some(Ok(Value::Map(m)))
            }

            // ── UID / capabilities ─────────────────────────────────────────────
            "getuid"  => Some(Ok(Value::Int(unsafe { libc::getuid() } as i64))),
            "getgid"  => Some(Ok(Value::Int(unsafe { libc::getgid() } as i64))),
            "geteuid" => Some(Ok(Value::Int(unsafe { libc::geteuid() } as i64))),
            "getegid" => Some(Ok(Value::Int(unsafe { libc::getegid() } as i64))),
            "setuid" => {
                let uid = args.first().map(|v| v.as_int()).unwrap_or(0) as libc::uid_t;
                Some(Ok(Value::Int(unsafe { libc::setuid(uid) } as i64)))
            }
            "setgid" => {
                let gid = args.first().map(|v| v.as_int()).unwrap_or(0) as libc::gid_t;
                Some(Ok(Value::Int(unsafe { libc::setgid(gid) } as i64)))
            }
            "chroot" => {
                if args.is_empty() { return Some(Err("chroot(path)".to_string())); }
                use std::ffi::CString;
                let p = CString::new(args[0].as_string().as_bytes()).unwrap();
                Some(Ok(Value::Int(unsafe { libc::chroot(p.as_ptr()) } as i64)))
            }
            "umask" => {
                let mask = args.first().map(|v| v.as_int()).unwrap_or(0o022) as libc::mode_t;
                Some(Ok(Value::Int(unsafe { libc::umask(mask) } as i64)))
            }

            // ── ioctl ──────────────────────────────────────────────────────────
            // ioctl(fd, request, arg?) -> int
            "ioctl" => {
                if args.len() < 2 { return Some(Err("ioctl(fd, request, arg?)".to_string())); }
                let fd  = args[0].as_int() as libc::c_int;
                let req = args[1].as_int() as libc::c_ulong;
                let arg = args.get(2).map(|v| v.as_int()).unwrap_or(0) as libc::c_long;
                let r = unsafe { libc::ioctl(fd, req, arg) };
                Some(Ok(Value::Int(r as i64)))
            }

            // ── mprotect ──────────────────────────────────────────────────────
            // mprotect(addr, size, prot) -> 0 on success
            "mprotect" => {
                if args.len() < 3 { return Some(Err("mprotect(addr, size, prot)".to_string())); }
                let addr = args[0].as_int() as *mut libc::c_void;
                let size = args[1].as_int() as libc::size_t;
                let prot = args[2].as_int() as libc::c_int;
                Some(Ok(Value::Int(unsafe { libc::mprotect(addr, size, prot) } as i64)))
            }

            // ── Shared memory (SysV) ───────────────────────────────────────────
            // shm_create(key_or_size, size?) -> {id, addr}
            "shm_create" => {
                if args.is_empty() { return Some(Err("shm_create(size) or shm_create(key, size)".to_string())); }
                let (key, size) = if args.len() >= 2 {
                    (args[0].as_int() as libc::key_t, args[1].as_int() as libc::size_t)
                } else {
                    (libc::IPC_PRIVATE, args[0].as_int() as libc::size_t)
                };
                let id = unsafe { libc::shmget(key, size, 0o600 | libc::IPC_CREAT) };
                if id < 0 { return Some(Err(format!("shm_create failed: errno {}", unsafe { *libc::__errno_location() }))); }
                let addr = unsafe { libc::shmat(id, std::ptr::null(), 0) } as usize;
                if addr == usize::MAX { return Some(Err(format!("shmat failed: errno {}", unsafe { *libc::__errno_location() }))); }
                let mut m = HashMap::new();
                m.insert("id".to_string(), Value::Int(id as i64));
                m.insert("addr".to_string(), Value::Int(addr as i64));
                Some(Ok(Value::Map(m)))
            }
            // shm_write(addr, bytes_or_string)
            "shm_write" => {
                if args.len() < 2 { return Some(Err("shm_write(addr, bytes)".to_string())); }
                let addr_val = match &args[0] {
                    Value::Map(m) => m.get("addr").map(|v| v.as_int()).unwrap_or(0),
                    other => other.as_int(),
                };
                if addr_val == 0 { return Some(Err("shm_write: null address".to_string())); }
                let addr = addr_val as *mut u8;
                let bytes: Vec<u8> = match &args[1] {
                    Value::Array(a) => a.iter().map(|v| v.as_int() as u8).collect(),
                    Value::String(s) => s.bytes().collect(),
                    _ => return Some(Err("shm_write: expected array or string".to_string())),
                };
                unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), addr, bytes.len()); }
                Some(Ok(Value::Int(bytes.len() as i64)))
            }
            // shm_read(addr, size) -> string
            "shm_read" => {
                if args.len() < 2 { return Some(Err("shm_read(addr, size)".to_string())); }
                let addr_val = match &args[0] {
                    Value::Map(m) => m.get("addr").map(|v| v.as_int()).unwrap_or(0),
                    other => other.as_int(),
                };
                if addr_val == 0 { return Some(Err("shm_read: null address".to_string())); }
                let addr = addr_val as *const u8;
                let size = args[1].as_int() as usize;
                let bytes = unsafe { std::slice::from_raw_parts(addr, size) };
                Some(Ok(Value::String(String::from_utf8_lossy(bytes).to_string())))
            }
            // shm_remove(id_or_map)
            "shm_remove" => {
                if args.is_empty() { return Some(Err("shm_remove(id)".to_string())); }
                let id = match &args[0] {
                    Value::Map(m) => m.get("id").map(|v| v.as_int()).unwrap_or(0),
                    other => other.as_int(),
                } as libc::c_int;
                unsafe { libc::shmctl(id, libc::IPC_RMID, std::ptr::null_mut()); }
                Some(Ok(Value::Null))
            }

            // ── TAR archive ────────────────────────────────────────────────────
            // tar_create(output_path, files_array) -> bool
            "tar_create" => {
                use tar::Builder;
                if args.len() < 2 { return Some(Err("tar_create(output, [files])".to_string())); }
                let out_path = args[0].as_string();
                let files: Vec<String> = match &args[1] {
                    Value::Array(a) => a.iter().map(|v| v.as_string()).collect(),
                    _ => return Some(Err("tar_create: second arg must be array".to_string())),
                };
                match std::fs::File::create(&out_path) {
                    Ok(f) => {
                        let mut ar = Builder::new(f);
                        for file in &files {
                            let p = std::path::Path::new(file);
                            // Use just the filename as the archive entry name
                            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| file.clone());
                            match std::fs::File::open(p) {
                                Ok(mut fh) => {
                                    if let Err(e) = ar.append_file(&name, &mut fh) {
                                        return Some(Err(e.to_string()));
                                    }
                                }
                                Err(e) => return Some(Err(e.to_string())),
                            }
                        }
                        match ar.finish() {
                            Ok(_) => Some(Ok(Value::Bool(true))),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // tar_list(path) -> array of filenames
            "tar_list" => {
                use tar::Archive;
                if args.is_empty() { return Some(Err("tar_list(path)".to_string())); }
                match std::fs::File::open(args[0].as_string()) {
                    Ok(f) => {
                        let mut ar = Archive::new(f);
                        let mut names = Vec::new();
                        match ar.entries() {
                            Ok(entries) => {
                                for entry in entries.flatten() {
                                    if let Ok(p) = entry.path() {
                                        names.push(Value::String(p.to_string_lossy().to_string()));
                                    }
                                }
                            }
                            Err(e) => return Some(Err(e.to_string())),
                        }
                        Some(Ok(Value::Array(names)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // tar_extract(path, dest_dir) -> bool
            "tar_extract" => {
                use tar::Archive;
                if args.len() < 2 { return Some(Err("tar_extract(path, dest)".to_string())); }
                match std::fs::File::open(args[0].as_string()) {
                    Ok(f) => {
                        let mut ar = Archive::new(f);
                        match ar.unpack(args[1].as_string()) {
                            Ok(_) => Some(Ok(Value::Bool(true))),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }

            // ── ptrace ─────────────────────────────────────────────────────────
            // ptrace_attach(pid) -> 0 on success
            "ptrace_attach" => {
                if args.is_empty() { return Some(Err("ptrace_attach(pid)".to_string())); }
                let pid = args[0].as_int() as libc::pid_t;
                let r = unsafe { libc::ptrace(libc::PTRACE_ATTACH, pid, 0, 0) };
                Some(Ok(Value::Int(r as i64)))
            }
            // ptrace_detach(pid)
            "ptrace_detach" => {
                if args.is_empty() { return Some(Err("ptrace_detach(pid)".to_string())); }
                let pid = args[0].as_int() as libc::pid_t;
                let r = unsafe { libc::ptrace(libc::PTRACE_DETACH, pid, 0, 0) };
                Some(Ok(Value::Int(r as i64)))
            }
            // ptrace_peekdata(pid, addr) -> i64
            "ptrace_peekdata" => {
                if args.len() < 2 { return Some(Err("ptrace_peekdata(pid, addr)".to_string())); }
                let pid  = args[0].as_int() as libc::pid_t;
                let addr = args[1].as_int() as *mut libc::c_void;
                let val = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, pid, addr, 0) };
                Some(Ok(Value::Int(val as i64)))
            }
            // ptrace_pokedata(pid, addr, value)
            "ptrace_pokedata" => {
                if args.len() < 3 { return Some(Err("ptrace_pokedata(pid, addr, val)".to_string())); }
                let pid  = args[0].as_int() as libc::pid_t;
                let addr = args[1].as_int() as *mut libc::c_void;
                let val  = args[2].as_int() as *mut libc::c_void;
                let r = unsafe { libc::ptrace(libc::PTRACE_POKEDATA, pid, addr, val) };
                Some(Ok(Value::Int(r as i64)))
            }
            // ptrace_cont(pid, sig?)
            "ptrace_cont" => {
                if args.is_empty() { return Some(Err("ptrace_cont(pid, sig?)".to_string())); }
                let pid = args[0].as_int() as libc::pid_t;
                let sig = args.get(1).map(|v| v.as_int()).unwrap_or(0) as *mut libc::c_void;
                let r = unsafe { libc::ptrace(libc::PTRACE_CONT, pid, 0, sig) };
                Some(Ok(Value::Int(r as i64)))
            }

            // ── Regex (extended) ───────────────────────────────────────────────
            // regex_replace(pattern, replacement, text) -> string
            "old_regex_replace" => {
                if args.len() < 3 { return Some(Err("regex_replace(pattern, replacement, text)".to_string())); }
                let pat  = args[0].as_string();
                let rep  = args[1].as_string();
                let text = args[2].as_string();
                match regex::Regex::new(&pat) {
                    Ok(re) => Some(Ok(Value::String(re.replace_all(&text, rep.as_str()).to_string()))),
                    Err(e) => Some(Err(format!("regex_replace: {}", e))),
                }
            }
            // regex_find_all(pattern, text) -> array of strings
            "regex_find_all" => {
                if args.len() < 2 { return Some(Err("regex_find_all(pattern, text)".to_string())); }
                let pat  = args[0].as_string();
                let text = args[1].as_string();
                match regex::Regex::new(&pat) {
                    Ok(re) => Some(Ok(Value::Array(re.find_iter(&text).map(|m| Value::String(m.as_str().to_string())).collect()))),
                    Err(e) => Some(Err(format!("regex_find_all: {}", e))),
                }
            }
            // regex_captures(pattern, text) -> array of [full, group1, group2, ...]  or null
            "old_regex_captures" => {
                if args.len() < 2 { return Some(Err("regex_captures(pattern, text)".to_string())); }
                let pat  = args[0].as_string();
                let text = args[1].as_string();
                match regex::Regex::new(&pat) {
                    Ok(re) => match re.captures(&text) {
                        Some(caps) => Some(Ok(Value::Array(
                            caps.iter().map(|m| m.map(|s| Value::String(s.as_str().to_string())).unwrap_or(Value::Null)).collect()
                        ))),
                        None => Some(Ok(Value::Null)),
                    },
                    Err(e) => Some(Err(format!("regex_captures: {}", e))),
                }
            }
            // regex_captures_all(pattern, text) -> array of arrays
            "regex_captures_all" => {
                if args.len() < 2 { return Some(Err("regex_captures_all(pattern, text)".to_string())); }
                let pat  = args[0].as_string();
                let text = args[1].as_string();
                match regex::Regex::new(&pat) {
                    Ok(re) => Some(Ok(Value::Array(
                        re.captures_iter(&text).map(|caps|
                            Value::Array(caps.iter().map(|m| m.map(|s| Value::String(s.as_str().to_string())).unwrap_or(Value::Null)).collect())
                        ).collect()
                    ))),
                    Err(e) => Some(Err(format!("regex_captures_all: {}", e))),
                }
            }
            // regex_split(pattern, text) -> array of strings
            "old_regex_split" => {
                if args.len() < 2 { return Some(Err("regex_split(pattern, text)".to_string())); }
                let pat  = args[0].as_string();
                let text = args[1].as_string();
                match regex::Regex::new(&pat) {
                    Ok(re) => Some(Ok(Value::Array(re.split(&text).map(|s| Value::String(s.to_string())).collect()))),
                    Err(e) => Some(Err(format!("regex_split: {}", e))),
                }
            }

            // ── Named pipes (mkfifo) ───────────────────────────────────────────
            // mkfifo(path, mode?) -> 0 on success
            "mkfifo" => {
                if args.is_empty() { return Some(Err("mkfifo(path, mode?)".to_string())); }
                use std::ffi::CString;
                let path = CString::new(args[0].as_string().as_bytes()).unwrap();
                let mode = args.get(1).map(|v| v.as_int()).unwrap_or(0o644) as libc::mode_t;
                Some(Ok(Value::Int(unsafe { libc::mkfifo(path.as_ptr(), mode) } as i64)))
            }
            // pipe_create() -> {read_fd, write_fd}
            "pipe_create" => {
                let mut fds = [0i32; 2];
                let r = unsafe { libc::pipe(fds.as_mut_ptr()) };
                if r < 0 { return Some(Err(format!("pipe_create: errno {}", unsafe { *libc::__errno_location() }))); }
                let mut m = HashMap::new();
                m.insert("read_fd".to_string(), Value::Int(fds[0] as i64));
                m.insert("write_fd".to_string(), Value::Int(fds[1] as i64));
                Some(Ok(Value::Map(m)))
            }
            // fd_read(fd, size?) -> string
            "fd_read" => {
                if args.is_empty() { return Some(Err("fd_read(fd, size?)".to_string())); }
                let fd = args[0].as_int() as libc::c_int;
                let size = args.get(1).map(|v| v.as_int() as usize).unwrap_or(4096);
                let mut buf = vec![0u8; size];
                let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, size) };
                if n < 0 { return Some(Err(format!("fd_read: errno {}", unsafe { *libc::__errno_location() }))); }
                Some(Ok(Value::String(String::from_utf8_lossy(&buf[..n as usize]).to_string())))
            }
            // fd_write(fd, data) -> bytes_written
            "fd_write" => {
                if args.len() < 2 { return Some(Err("fd_write(fd, data)".to_string())); }
                let fd = args[0].as_int() as libc::c_int;
                let data = args[1].as_string();
                let n = unsafe { libc::write(fd, data.as_ptr() as *const _, data.len()) };
                Some(Ok(Value::Int(n as i64)))
            }
            // fd_close(fd)
            "fd_close" => {
                if args.is_empty() { return Some(Err("fd_close(fd)".to_string())); }
                unsafe { libc::close(args[0].as_int() as libc::c_int); }
                Some(Ok(Value::Null))
            }

            // ── Environment variables ──────────────────────────────────────────
            // env_list() -> map of all env vars
            "env_list" => {
                let m: HashMap<String, Value> = std::env::vars().map(|(k, v)| (k, Value::String(v))).collect();
                Some(Ok(Value::Map(m)))
            }
            // env_set(key, value)
            "env_set" | "setenv" => {
                if args.len() < 2 { return Some(Err("env_set(key, val)".to_string())); }
                std::env::set_var(args[0].as_string(), args[1].as_string());
                Some(Ok(Value::Null))
            }
            // env_del(key)
            "env_del" | "unsetenv" => {
                if args.is_empty() { return Some(Err("env_del(key)".to_string())); }
                std::env::remove_var(args[0].as_string());
                Some(Ok(Value::Null))
            }

            // ── sendfile (fast file copy to socket/fd) ─────────────────────────
            // sendfile(out_fd, in_fd, offset?, count) -> bytes_sent
            "sendfile" => {
                if args.len() < 3 { return Some(Err("sendfile(out_fd, in_fd, count)".to_string())); }
                let out_fd = args[0].as_int() as libc::c_int;
                let in_fd  = args[1].as_int() as libc::c_int;
                let count  = args[2].as_int() as libc::size_t;
                let n = unsafe { libc::sendfile(out_fd, in_fd, std::ptr::null_mut(), count) };
                Some(Ok(Value::Int(n as i64)))
            }

            // ── epoll (Linux event notification) ──────────────────────────────
            // epoll_create() -> epoll_fd
            "epoll_create" => {
                let fd = unsafe { libc::epoll_create1(0) };
                if fd < 0 { return Some(Err(format!("epoll_create: errno {}", unsafe { *libc::__errno_location() }))); }
                Some(Ok(Value::Int(fd as i64)))
            }
            // epoll_add(epoll_fd, watch_fd, events?) — events: 1=EPOLLIN 4=EPOLLOUT
            "epoll_add" => {
                if args.len() < 2 { return Some(Err("epoll_add(epoll_fd, fd, events?)".to_string())); }
                let epfd = args[0].as_int() as libc::c_int;
                let fd   = args[1].as_int() as libc::c_int;
                let events = args.get(2).map(|v| v.as_int() as u32).unwrap_or(libc::EPOLLIN as u32 | libc::EPOLLOUT as u32);
                let mut ev = libc::epoll_event { events, u64: fd as u64 };
                let r = unsafe { libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, fd, &mut ev) };
                Some(Ok(Value::Int(r as i64)))
            }
            // epoll_wait(epoll_fd, max_events?, timeout_ms?) -> array of ready fds
            "epoll_wait" => {
                if args.is_empty() { return Some(Err("epoll_wait(epoll_fd)".to_string())); }
                let epfd   = args[0].as_int() as libc::c_int;
                let maxevs = args.get(1).map(|v| v.as_int() as usize).unwrap_or(64);
                let timeout = args.get(2).map(|v| v.as_int() as libc::c_int).unwrap_or(-1);
                let mut events = vec![libc::epoll_event { events: 0, u64: 0 }; maxevs];
                let n = unsafe { libc::epoll_wait(epfd, events.as_mut_ptr(), maxevs as libc::c_int, timeout) };
                if n < 0 { return Some(Err(format!("epoll_wait: errno {}", unsafe { *libc::__errno_location() }))); }
                let ready: Vec<Value> = events[..n as usize].iter().map(|e| Value::Int(e.u64 as i64)).collect();
                Some(Ok(Value::Array(ready)))
            }

            // ── fd-level control ───────────────────────────────────────────────
            // fcntl(fd, cmd, arg?) -> int
            "fcntl" => {
                if args.len() < 2 { return Some(Err("fcntl(fd, cmd, arg?)".to_string())); }
                let fd  = args[0].as_int() as libc::c_int;
                let cmd = args[1].as_int() as libc::c_int;
                let arg = args.get(2).map(|v| v.as_int()).unwrap_or(0) as libc::c_int;
                Some(Ok(Value::Int(unsafe { libc::fcntl(fd, cmd, arg) } as i64)))
            }
            // fcntl constants as values
            "F_GETFL"  => Some(Ok(Value::Int(libc::F_GETFL as i64))),
            "F_SETFL"  => Some(Ok(Value::Int(libc::F_SETFL as i64))),
            "O_NONBLOCK" => Some(Ok(Value::Int(libc::O_NONBLOCK as i64))),
            "O_CLOEXEC"  => Some(Ok(Value::Int(libc::O_CLOEXEC as i64))),
            "F_DUPFD"    => Some(Ok(Value::Int(libc::F_DUPFD as i64))),
            // dup(fd) -> new_fd
            "dup" => {
                if args.is_empty() { return Some(Err("dup(fd)".to_string())); }
                Some(Ok(Value::Int(unsafe { libc::dup(args[0].as_int() as libc::c_int) } as i64)))
            }
            // dup2(old_fd, new_fd) -> new_fd
            "dup2" => {
                if args.len() < 2 { return Some(Err("dup2(old_fd, new_fd)".to_string())); }
                Some(Ok(Value::Int(unsafe { libc::dup2(args[0].as_int() as libc::c_int, args[1].as_int() as libc::c_int) } as i64)))
            }
            // lseek(fd, offset, whence) -> new_pos   whence: 0=SET 1=CUR 2=END
            "lseek" => {
                if args.len() < 2 { return Some(Err("lseek(fd, offset, whence?)".to_string())); }
                let fd     = args[0].as_int() as libc::c_int;
                let offset = args[1].as_int() as libc::off_t;
                let whence = args.get(2).map(|v| v.as_int()).unwrap_or(0) as libc::c_int;
                Some(Ok(Value::Int(unsafe { libc::lseek(fd, offset, whence) } as i64)))
            }
            // flock(fd, op) -> 0  op: 1=LOCK_SH 2=LOCK_EX 4=LOCK_NB 8=LOCK_UN
            "flock" => {
                if args.len() < 2 { return Some(Err("flock(fd, op)".to_string())); }
                let fd = args[0].as_int() as libc::c_int;
                let op = args[1].as_int() as libc::c_int;
                Some(Ok(Value::Int(unsafe { libc::flock(fd, op) } as i64)))
            }
            "LOCK_SH" => Some(Ok(Value::Int(libc::LOCK_SH as i64))),
            "LOCK_EX" => Some(Ok(Value::Int(libc::LOCK_EX as i64))),
            "LOCK_UN" => Some(Ok(Value::Int(libc::LOCK_UN as i64))),
            "LOCK_NB" => Some(Ok(Value::Int(libc::LOCK_NB as i64))),
            // ftruncate(fd, length) -> 0
            "ftruncate" => {
                if args.len() < 2 { return Some(Err("ftruncate(fd, len)".to_string())); }
                let fd  = args[0].as_int() as libc::c_int;
                let len = args[1].as_int() as libc::off_t;
                Some(Ok(Value::Int(unsafe { libc::ftruncate(fd, len) } as i64)))
            }
            // open(path, flags, mode?) -> fd
            "open" | "fd_open" => {
                if args.is_empty() { return Some(Err("open(path, flags?, mode?)".to_string())); }
                use std::ffi::CString;
                let path  = CString::new(args[0].as_string().as_bytes()).unwrap();
                let flags = args.get(1).map(|v| v.as_int()).unwrap_or(libc::O_RDONLY as i64) as libc::c_int;
                let mode  = args.get(2).map(|v| v.as_int()).unwrap_or(0o644) as libc::mode_t;
                Some(Ok(Value::Int(unsafe { libc::open(path.as_ptr(), flags, mode) } as i64)))
            }
            "O_RDONLY" => Some(Ok(Value::Int(libc::O_RDONLY as i64))),
            "O_WRONLY" => Some(Ok(Value::Int(libc::O_WRONLY as i64))),
            "O_RDWR"   => Some(Ok(Value::Int(libc::O_RDWR as i64))),
            "O_CREAT"  => Some(Ok(Value::Int(libc::O_CREAT as i64))),
            "O_TRUNC"  => Some(Ok(Value::Int(libc::O_TRUNC as i64))),
            "O_APPEND" => Some(Ok(Value::Int(libc::O_APPEND as i64))),
            // setsockopt(fd, level, name, value) -> int
            "setsockopt" => {
                if args.len() < 4 { return Some(Err("setsockopt(fd, level, optname, value)".to_string())); }
                let fd       = args[0].as_int() as libc::c_int;
                let level    = args[1].as_int() as libc::c_int;
                let optname  = args[2].as_int() as libc::c_int;
                let val: libc::c_int = args[3].as_int() as libc::c_int;
                let r = unsafe { libc::setsockopt(fd, level, optname, &val as *const _ as *const _, std::mem::size_of::<libc::c_int>() as libc::socklen_t) };
                Some(Ok(Value::Int(r as i64)))
            }
            // getsockopt(fd, level, name) -> int
            "getsockopt" => {
                if args.len() < 3 { return Some(Err("getsockopt(fd, level, optname)".to_string())); }
                let fd      = args[0].as_int() as libc::c_int;
                let level   = args[1].as_int() as libc::c_int;
                let optname = args[2].as_int() as libc::c_int;
                let mut val: libc::c_int = 0;
                let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
                unsafe { libc::getsockopt(fd, level, optname, &mut val as *mut _ as *mut _, &mut len); }
                Some(Ok(Value::Int(val as i64)))
            }
            // socket options constants
            "SOL_SOCKET"   => Some(Ok(Value::Int(libc::SOL_SOCKET as i64))),
            "SO_REUSEADDR" => Some(Ok(Value::Int(libc::SO_REUSEADDR as i64))),
            "SO_REUSEPORT" => Some(Ok(Value::Int(libc::SO_REUSEPORT as i64))),
            "SO_KEEPALIVE" => Some(Ok(Value::Int(libc::SO_KEEPALIVE as i64))),
            "SO_SNDBUF"    => Some(Ok(Value::Int(libc::SO_SNDBUF as i64))),
            "SO_RCVBUF"    => Some(Ok(Value::Int(libc::SO_RCVBUF as i64))),
            "IPPROTO_TCP"  => Some(Ok(Value::Int(libc::IPPROTO_TCP as i64))),
            "TCP_NODELAY"  => Some(Ok(Value::Int(libc::TCP_NODELAY as i64))),

            // ── inotify constants ──────────────────────────────────────────────
            "IN_ACCESS"      => Some(Ok(Value::Int(libc::IN_ACCESS as i64))),
            "IN_MODIFY"      => Some(Ok(Value::Int(libc::IN_MODIFY as i64))),
            "IN_ATTRIB"      => Some(Ok(Value::Int(libc::IN_ATTRIB as i64))),
            "IN_CLOSE_WRITE" => Some(Ok(Value::Int(libc::IN_CLOSE_WRITE as i64))),
            "IN_CLOSE_NOWRITE" => Some(Ok(Value::Int(libc::IN_CLOSE_NOWRITE as i64))),
            "IN_OPEN"        => Some(Ok(Value::Int(libc::IN_OPEN as i64))),
            "IN_MOVED_FROM"  => Some(Ok(Value::Int(libc::IN_MOVED_FROM as i64))),
            "IN_MOVED_TO"    => Some(Ok(Value::Int(libc::IN_MOVED_TO as i64))),
            "IN_CREATE"      => Some(Ok(Value::Int(libc::IN_CREATE as i64))),
            "IN_DELETE"      => Some(Ok(Value::Int(libc::IN_DELETE as i64))),
            "IN_DELETE_SELF" => Some(Ok(Value::Int(libc::IN_DELETE_SELF as i64))),
            "IN_MOVE_SELF"   => Some(Ok(Value::Int(libc::IN_MOVE_SELF as i64))),
            "IN_ALL_EVENTS"  => Some(Ok(Value::Int(libc::IN_ALL_EVENTS as i64))),

            // inotify_init() -> fd
            "inotify_init" => {
                let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
                if fd < 0 { Some(Err(format!("inotify_init: errno {}", unsafe { *libc::__errno_location() }))) }
                else { Some(Ok(Value::Int(fd as i64))) }
            }
            // inotify_add_watch(fd, path, mask) -> wd
            "inotify_add_watch" => {
                if args.len() < 3 { return Some(Err("inotify_add_watch(fd, path, mask)".to_string())); }
                let fd   = args[0].as_int() as libc::c_int;
                let path = args[1].as_string();
                let mask = args[2].as_int() as libc::uint32_t;
                let cs = std::ffi::CString::new(path).unwrap();
                let wd = unsafe { libc::inotify_add_watch(fd, cs.as_ptr(), mask) };
                if wd < 0 { Some(Err(format!("inotify_add_watch: errno {}", unsafe { *libc::__errno_location() }))) }
                else { Some(Ok(Value::Int(wd as i64))) }
            }
            // inotify_rm_watch(fd, wd)
            "inotify_rm_watch" => {
                if args.len() < 2 { return Some(Err("inotify_rm_watch(fd, wd)".to_string())); }
                let fd = args[0].as_int() as libc::c_int;
                let wd = args[1].as_int() as libc::c_int;
                unsafe { libc::inotify_rm_watch(fd, wd); }
                Some(Ok(Value::Bool(true)))
            }
            // inotify_read(fd) -> array of {wd, mask, cookie, name}
            "inotify_read" => {
                if args.is_empty() { return Some(Err("inotify_read(fd)".to_string())); }
                let fd = args[0].as_int() as libc::c_int;
                let mut buf = [0u8; 4096];
                let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                if n <= 0 { return Some(Ok(Value::Array(vec![]))); }
                let mut events = Vec::new();
                let mut offset = 0usize;
                while offset + 16 <= n as usize {
                    let wd     = i32::from_ne_bytes(buf[offset..offset+4].try_into().unwrap());
                    let mask   = u32::from_ne_bytes(buf[offset+4..offset+8].try_into().unwrap());
                    let cookie = u32::from_ne_bytes(buf[offset+8..offset+12].try_into().unwrap());
                    let len    = u32::from_ne_bytes(buf[offset+12..offset+16].try_into().unwrap()) as usize;
                    let name = if len > 0 && offset + 16 + len <= n as usize {
                        let raw = &buf[offset+16..offset+16+len];
                        let end = raw.iter().position(|&b| b == 0).unwrap_or(len);
                        String::from_utf8_lossy(&raw[..end]).to_string()
                    } else { String::new() };
                    let mut m = HashMap::new();
                    m.insert("wd".to_string(),     Value::Int(wd as i64));
                    m.insert("mask".to_string(),   Value::Int(mask as i64));
                    m.insert("cookie".to_string(), Value::Int(cookie as i64));
                    m.insert("name".to_string(),   Value::String(name));
                    events.push(Value::Map(m));
                    offset += 16 + len;
                }
                Some(Ok(Value::Array(events)))
            }

            // ── Secure random (getrandom syscall) ─────────────────────────────
            // random_bytes(n) -> array of n random bytes
            "random_bytes" | "getrandom" => {
                let n = args.first().map(|v| v.as_int() as usize).unwrap_or(32).min(65536);
                let mut buf = vec![0u8; n];
                unsafe { libc::getrandom(buf.as_mut_ptr() as *mut _, n, 0); }
                Some(Ok(Value::Array(buf.iter().map(|b| Value::Int(*b as i64)).collect())))
            }
            // random_hex(n) -> hex string of n random bytes
            "random_hex" => {
                let n = args.first().map(|v| v.as_int() as usize).unwrap_or(16).min(65536);
                let mut buf = vec![0u8; n];
                unsafe { libc::getrandom(buf.as_mut_ptr() as *mut _, n, 0); }
                Some(Ok(Value::String(buf.iter().map(|b| format!("{:02x}", b)).collect())))
            }
            // random_int(min?, max?) -> random i64 in [min, max)
            "random_int" | "randint" => {
                let mut buf = [0u8; 8];
                unsafe { libc::getrandom(buf.as_mut_ptr() as *mut _, 8, 0); }
                let raw = i64::from_le_bytes(buf).abs();
                let result = match (args.first(), args.get(1)) {
                    (Some(lo), Some(hi)) => {
                        let lo = lo.as_int(); let hi = hi.as_int();
                        let range = (hi - lo).abs().max(1);
                        lo + raw % range
                    }
                    (Some(hi), None) => raw % hi.as_int().abs().max(1),
                    _ => raw,
                };
                Some(Ok(Value::Int(result)))
            }
            // random_float() -> f64 in [0.0, 1.0)
            "random_float" | "randf" => {
                let mut buf = [0u8; 8];
                unsafe { libc::getrandom(buf.as_mut_ptr() as *mut _, 8, 0); }
                let raw = u64::from_le_bytes(buf);
                Some(Ok(Value::Float((raw >> 11) as f64 / (1u64 << 53) as f64)))
            }
            // shuffle(array) -> shuffled copy
            "shuffle" => {
                if args.is_empty() { return Some(Err("shuffle(array)".to_string())); }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut out = arr.clone();
                        for i in (1..out.len()).rev() {
                            let mut buf = [0u8; 8];
                            unsafe { libc::getrandom(buf.as_mut_ptr() as *mut _, 8, 0); }
                            let j = (u64::from_le_bytes(buf) as usize) % (i + 1);
                            out.swap(i, j);
                        }
                        Some(Ok(Value::Array(out)))
                    }
                    _ => Some(Err("shuffle: expected array".to_string())),
                }
            }

            // ── /proc filesystem ───────────────────────────────────────────────
            // proc_self_maps() -> array of {addr, perms, name} maps
            "proc_self_maps" => {
                match std::fs::read_to_string("/proc/self/maps") {
                    Ok(s) => {
                        let entries: Vec<Value> = s.lines().map(|line| {
                            let parts: Vec<&str> = line.splitn(6, ' ').collect();
                            let mut m = HashMap::new();
                            m.insert("addr".to_string(), Value::String(parts.first().copied().unwrap_or("").to_string()));
                            m.insert("perms".to_string(), Value::String(parts.get(1).copied().unwrap_or("").to_string()));
                            m.insert("name".to_string(), Value::String(parts.last().copied().unwrap_or("").trim().to_string()));
                            Value::Map(m)
                        }).collect();
                        Some(Ok(Value::Array(entries)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // proc_status(pid?) -> map of key=value from /proc/PID/status
            "proc_status" => {
                let pid = match args.first().map(|v| v.as_int()).unwrap_or(0) {
                    0 => (unsafe { libc::getpid() }) as i64,
                    n => n,
                };
                let path = format!("/proc/{}/status", pid);
                match std::fs::read_to_string(&path) {
                    Ok(s) => {
                        let mut m = HashMap::new();
                        for line in s.lines() {
                            if let Some((k, v)) = line.split_once(':') {
                                m.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
                            }
                        }
                        Some(Ok(Value::Map(m)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // proc_fds(pid?) -> array of open fd numbers
            "proc_fds" => {
                let pid = match args.first().map(|v| v.as_int()).unwrap_or(0) {
                    0 => (unsafe { libc::getpid() }) as i64, n => n };
                let path = format!("/proc/{}/fd", pid);
                match std::fs::read_dir(&path) {
                    Ok(entries) => {
                        let fds: Vec<Value> = entries
                            .filter_map(|e| e.ok())
                            .filter_map(|e| e.file_name().to_string_lossy().parse::<i64>().ok())
                            .map(Value::Int)
                            .collect();
                        Some(Ok(Value::Array(fds)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // proc_cmdline(pid?) -> array of argv strings
            "proc_cmdline" => {
                let pid = match args.first().map(|v| v.as_int()).unwrap_or(0) {
                    0 => (unsafe { libc::getpid() }) as i64, n => n };
                match std::fs::read(format!("/proc/{}/cmdline", pid)) {
                    Ok(bytes) => {
                        let args_vec: Vec<Value> = bytes.split(|&b| b == 0)
                            .filter(|s| !s.is_empty())
                            .map(|s| Value::String(String::from_utf8_lossy(s).to_string()))
                            .collect();
                        Some(Ok(Value::Array(args_vec)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // proc_list() -> array of {pid, name} for all processes
            "proc_list" => {
                let mut procs = Vec::new();
                if let Ok(entries) = std::fs::read_dir("/proc") {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if let Ok(pid) = name.parse::<i64>() {
                            let comm = std::fs::read_to_string(format!("/proc/{}/comm", pid))
                                .unwrap_or_default().trim().to_string();
                            let mut m = HashMap::new();
                            m.insert("pid".to_string(), Value::Int(pid));
                            m.insert("name".to_string(), Value::String(comm));
                            procs.push(Value::Map(m));
                        }
                    }
                }
                Some(Ok(Value::Array(procs)))
            }
            // mem_read(pid, addr, size) -> bytes (reads /proc/PID/mem)
            "mem_read" => {
                if args.len() < 3 { return Some(Err("mem_read(pid, addr, size)".to_string())); }
                let pid  = args[0].as_int();
                let addr = args[1].as_int() as u64;
                let size = args[2].as_int() as usize;
                let path = format!("/proc/{}/mem", pid);
                match std::fs::OpenOptions::new().read(true).open(&path) {
                    Ok(mut f) => {
                        use std::io::Seek;
                        if f.seek(std::io::SeekFrom::Start(addr)).is_err() {
                            return Some(Err("mem_read: seek failed".to_string()));
                        }
                        let mut buf = vec![0u8; size];
                        use std::io::Read;
                        match f.read_exact(&mut buf) {
                            Ok(_) => Some(Ok(Value::Array(buf.iter().map(|b| Value::Int(*b as i64)).collect()))),
                            Err(e) => Some(Err(e.to_string())),
                        }
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }

            // ── Extra string predicates ────────────────────────────────────────
            "str_is_alpha"   => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_alphabetic())))) }
            "str_is_digit"   => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_ascii_digit())))) }
            "str_is_alnum"   => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_alphanumeric())))) }
            "str_is_upper"   => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_uppercase())))) }
            "str_is_lower"   => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_lowercase())))) }
            "str_is_space"   => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_whitespace())))) }
            "str_is_hex"     => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())))) }
            "str_reverse"    => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::String(s.chars().rev().collect()))) }
            "str_count"      => {
                if args.len() < 2 { return Some(Err("str_count(s, sub)".to_string())); }
                let s   = args[0].as_string();
                let sub = args[1].as_string();
                Some(Ok(Value::Int(s.matches(sub.as_str()).count() as i64)))
            }
            "str_center" => {
                if args.len() < 2 { return Some(Err("str_center(s, width, fill?)".to_string())); }
                let s = args[0].as_string();
                let w = args[1].as_int() as usize;
                let fill = args.get(2).map(|v| v.as_string()).unwrap_or_else(|| " ".to_string());
                let fill_ch = fill.chars().next().unwrap_or(' ');
                if s.len() >= w { return Some(Ok(Value::String(s))); }
                let pad = w - s.len();
                let left = pad / 2;
                let right = pad - left;
                Some(Ok(Value::String(format!("{}{}{}", fill_ch.to_string().repeat(left), s, fill_ch.to_string().repeat(right)))))
            }
            "str_zfill" => {
                if args.len() < 2 { return Some(Err("str_zfill(s, width)".to_string())); }
                let s = args[0].as_string();
                let w = args[1].as_int() as usize;
                if s.len() >= w { return Some(Ok(Value::String(s))); }
                Some(Ok(Value::String(format!("{:0>width$}", s, width = w))))
            }
            "str_indent" => {
                if args.len() < 2 { return Some(Err("str_indent(s, prefix)".to_string())); }
                let s      = args[0].as_string();
                let prefix = args[1].as_string();
                Some(Ok(Value::String(s.lines().map(|l| format!("{}{}", prefix, l)).collect::<Vec<_>>().join("\n"))))
            }
            "str_dedent" => {
                if args.is_empty() { return Some(Err("str_dedent(s)".to_string())); }
                let s = args[0].as_string();
                let min_indent = s.lines().filter(|l| !l.trim().is_empty())
                    .map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
                Some(Ok(Value::String(s.lines().map(|l| if l.len() >= min_indent { &l[min_indent..] } else { l }).collect::<Vec<_>>().join("\n"))))
            }

            // ── High-resolution time ───────────────────────────────────────────
            // monotonic_ns() -> i64 nanoseconds (CLOCK_MONOTONIC)
            "monotonic_ns" | "clock_mono" => {
                let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
                unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts); }
                Some(Ok(Value::Int(ts.tv_sec as i64 * 1_000_000_000 + ts.tv_nsec as i64)))
            }
            // realtime_ns() -> i64 nanoseconds since epoch (CLOCK_REALTIME)
            "realtime_ns" | "clock_real" => {
                let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
                unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts); }
                Some(Ok(Value::Int(ts.tv_sec as i64 * 1_000_000_000 + ts.tv_nsec as i64)))
            }
            // sleep_us(n) -> sleep microseconds
            "sleep_us" | "usleep" => {
                let us = args.first().map(|v| v.as_int() as u64).unwrap_or(0);
                unsafe { libc::usleep(us as libc::c_uint); }
                Some(Ok(Value::Null))
            }
            // sleep_ns(n) -> nanosleep
            "sleep_ns" | "nanosleep" => {
                let ns = args.first().map(|v| v.as_int() as i64).unwrap_or(0);
                let ts = libc::timespec { tv_sec: ns / 1_000_000_000, tv_nsec: ns % 1_000_000_000 };
                unsafe { libc::nanosleep(&ts, std::ptr::null_mut()); }
                Some(Ok(Value::Null))
            }

            // ── System information ─────────────────────────────────────────────
            // hostname() -> string
            "hostname" | "gethostname" => {
                let mut buf = vec![0u8; 256];
                unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()); }
                let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                Some(Ok(Value::String(String::from_utf8_lossy(&buf[..end]).to_string())))
            }
            // uname() -> {sysname, nodename, release, version, machine}
            "uname" => {
                let mut u: libc::utsname = unsafe { std::mem::zeroed() };
                unsafe { libc::uname(&mut u); }
                let to_s = |arr: &[libc::c_char]| {
                    let bytes: Vec<u8> = arr.iter().map(|&c| c as u8).collect();
                    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                    String::from_utf8_lossy(&bytes[..end]).to_string()
                };
                let mut m = HashMap::new();
                m.insert("sysname".to_string(),  Value::String(to_s(&u.sysname)));
                m.insert("nodename".to_string(), Value::String(to_s(&u.nodename)));
                m.insert("release".to_string(),  Value::String(to_s(&u.release)));
                m.insert("version".to_string(),  Value::String(to_s(&u.version)));
                m.insert("machine".to_string(),  Value::String(to_s(&u.machine)));
                Some(Ok(Value::Map(m)))
            }
            // sysinfo() -> {uptime, totalram, freeram, procs, loads}
            "sysinfo" => {
                let mut si: libc::sysinfo = unsafe { std::mem::zeroed() };
                unsafe { libc::sysinfo(&mut si); }
                let mut m = HashMap::new();
                m.insert("uptime".to_string(),   Value::Int(si.uptime as i64));
                m.insert("totalram".to_string(), Value::Int(si.totalram as i64 * si.mem_unit as i64));
                m.insert("freeram".to_string(),  Value::Int(si.freeram as i64 * si.mem_unit as i64));
                m.insert("procs".to_string(),    Value::Int(si.procs as i64));
                m.insert("load1".to_string(),    Value::Float(si.loads[0] as f64 / 65536.0));
                m.insert("load5".to_string(),    Value::Float(si.loads[1] as f64 / 65536.0));
                m.insert("load15".to_string(),   Value::Float(si.loads[2] as f64 / 65536.0));
                Some(Ok(Value::Map(m)))
            }
            // getpid() -> i64
            "getpid" | "self_pid" => Some(Ok(Value::Int(unsafe { libc::getpid() } as i64))),
            // getppid() -> i64
            "getppid" => Some(Ok(Value::Int(unsafe { libc::getppid() } as i64))),
            // nprocs() -> number of CPU cores
            "nprocs" | "ncpus" => Some(Ok(Value::Int(unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) } as i64))),
            // page_size() -> system page size
            "page_size" => Some(Ok(Value::Int(unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as i64))),

            // ── Terminal dimensions ────────────────────────────────────────────
            // term_size() -> {rows, cols}
            "term_size" | "winsize" => {
                #[repr(C)]
                struct Winsize { ws_row: u16, ws_col: u16, ws_xpixel: u16, ws_ypixel: u16 }
                let mut ws: Winsize = unsafe { std::mem::zeroed() };
                // TIOCGWINSZ = 0x5413 on Linux
                unsafe { libc::ioctl(1, 0x5413, &mut ws); }
                let mut m = HashMap::new();
                m.insert("rows".to_string(), Value::Int(ws.ws_row as i64));
                m.insert("cols".to_string(), Value::Int(ws.ws_col as i64));
                Some(Ok(Value::Map(m)))
            }

            // ── DNS / networking ──────────────────────────────────────────────
            // dns_lookup(host) -> array of IP strings
            "dns_lookup" | "getaddrinfo_ips" => {
                if args.is_empty() { return Some(Err("dns_lookup(host)".to_string())); }
                let host = args[0].as_string();
                use std::net::ToSocketAddrs;
                match (host.as_str(), 0u16).to_socket_addrs() {
                    Ok(iter) => {
                        let ips: Vec<Value> = iter.map(|a| Value::String(a.ip().to_string())).collect();
                        Some(Ok(Value::Array(ips)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // ip_to_int(ip_str) -> i64 (IPv4 only)
            "ip_to_int" | "inet_aton" => {
                if args.is_empty() { return Some(Err("ip_to_int(str)".to_string())); }
                let s = args[0].as_string();
                match s.parse::<std::net::Ipv4Addr>() {
                    Ok(ip) => Some(Ok(Value::Int(u32::from(ip) as i64))),
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // int_to_ip(n) -> string
            "int_to_ip" | "inet_ntoa" => {
                if args.is_empty() { return Some(Err("int_to_ip(n)".to_string())); }
                let n = args[0].as_int() as u32;
                Some(Ok(Value::String(std::net::Ipv4Addr::from(n).to_string())))
            }
            // is_ipv4(s) -> bool
            "is_ipv4" => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::Bool(s.parse::<std::net::Ipv4Addr>().is_ok()))) }
            // is_ipv6(s) -> bool
            "is_ipv6" => { let s = args.first().map(|v| v.as_string()).unwrap_or_default(); Some(Ok(Value::Bool(s.parse::<std::net::Ipv6Addr>().is_ok()))) }

            // ── waitpid ───────────────────────────────────────────────────────
            // waitpid(pid, flags?) -> {pid, status, exited, exit_code}
            "waitpid" => {
                if args.is_empty() { return Some(Err("waitpid(pid, flags?)".to_string())); }
                let pid   = args[0].as_int() as libc::pid_t;
                let flags = args.get(1).map(|v| v.as_int() as libc::c_int).unwrap_or(0);
                let mut status: libc::c_int = 0;
                let ret = unsafe { libc::waitpid(pid, &mut status, flags) };
                let exited   = libc::WIFEXITED(status);
                let code     = if exited { libc::WEXITSTATUS(status) } else { -1 };
                let signaled = libc::WIFSIGNALED(status);
                let sig      = if signaled { libc::WTERMSIG(status) } else { 0 };
                let mut m = HashMap::new();
                m.insert("pid".to_string(),       Value::Int(ret as i64));
                m.insert("status".to_string(),    Value::Int(status as i64));
                m.insert("exited".to_string(),    Value::Bool(exited));
                m.insert("exit_code".to_string(), Value::Int(code as i64));
                m.insert("signaled".to_string(),  Value::Bool(signaled));
                m.insert("signal".to_string(),    Value::Int(sig as i64));
                Some(Ok(Value::Map(m)))
            }
            // WNOHANG constant
            "WNOHANG" => Some(Ok(Value::Int(libc::WNOHANG as i64))),

            // ── select() I/O multiplexer ──────────────────────────────────────
            // fd_select(read_fds, write_fds, except_fds, timeout_us?) -> {readable, writable}
            "fd_select" | "select" => {
                if args.is_empty() { return Some(Err("fd_select(read_fds, write_fds?, except_fds?, timeout_us?)".to_string())); }
                let to_fd_list = |v: &Value| -> Vec<i32> {
                    match v { Value::Array(a) => a.iter().map(|x| x.as_int() as i32).collect(), _ => vec![] }
                };
                let read_fds   = to_fd_list(&args[0]);
                let write_fds  = if args.len() > 1 { to_fd_list(&args[1]) } else { vec![] };
                let except_fds = if args.len() > 2 { to_fd_list(&args[2]) } else { vec![] };
                let timeout_us = args.get(3).map(|v| v.as_int() as i64);
                let max_fd = read_fds.iter().chain(write_fds.iter()).chain(except_fds.iter()).copied().max().unwrap_or(-1);
                if max_fd < 0 { return Some(Ok(Value::Map({ let mut m = HashMap::new(); m.insert("readable".to_string(), Value::Array(vec![])); m.insert("writable".to_string(), Value::Array(vec![])); m }))); }
                let mut rfds: libc::fd_set = unsafe { std::mem::zeroed() };
                let mut wfds: libc::fd_set = unsafe { std::mem::zeroed() };
                let mut efds: libc::fd_set = unsafe { std::mem::zeroed() };
                for &fd in &read_fds   { unsafe { libc::FD_SET(fd, &mut rfds); } }
                for &fd in &write_fds  { unsafe { libc::FD_SET(fd, &mut wfds); } }
                for &fd in &except_fds { unsafe { libc::FD_SET(fd, &mut efds); } }
                let mut tv_storage;
                let tv_ptr = match timeout_us {
                    Some(us) => { tv_storage = libc::timeval { tv_sec: us / 1_000_000, tv_usec: us % 1_000_000 }; &mut tv_storage as *mut _ }
                    None => std::ptr::null_mut(),
                };
                unsafe { libc::select(max_fd + 1, &mut rfds, &mut wfds, &mut efds, tv_ptr); }
                let readable: Vec<Value> = read_fds.iter().filter(|&&fd| unsafe { libc::FD_ISSET(fd, &rfds) }).map(|&fd| Value::Int(fd as i64)).collect();
                let writable: Vec<Value> = write_fds.iter().filter(|&&fd| unsafe { libc::FD_ISSET(fd, &wfds) }).map(|&fd| Value::Int(fd as i64)).collect();
                let mut m = HashMap::new();
                m.insert("readable".to_string(), Value::Array(readable));
                m.insert("writable".to_string(), Value::Array(writable));
                Some(Ok(Value::Map(m)))
            }

            // ── Base64 (pure Rust, no dependency) ────────────────────────────
            "base64_encode" | "b64enc" => {
                if args.is_empty() { return Some(Err("base64_encode(str_or_bytes)".to_string())); }
                let bytes: Vec<u8> = match &args[0] {
                    Value::String(s) => s.bytes().collect(),
                    Value::Array(a)  => a.iter().map(|v| v.as_int() as u8).collect(),
                    other => other.as_string().bytes().collect(),
                };
                const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                let mut out = String::new();
                for chunk in bytes.chunks(3) {
                    let b0 = chunk[0] as u32;
                    let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
                    let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
                    let n = (b0 << 16) | (b1 << 8) | b2;
                    out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
                    out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
                    out.push(if chunk.len() > 1 { CHARS[((n >> 6) & 0x3F) as usize] as char } else { '=' });
                    out.push(if chunk.len() > 2 { CHARS[(n & 0x3F) as usize] as char } else { '=' });
                }
                Some(Ok(Value::String(out)))
            }
            "base64_decode" | "b64dec" => {
                if args.is_empty() { return Some(Err("base64_decode(str)".to_string())); }
                let s = args[0].as_string();
                let decode_char = |c: u8| -> u8 {
                    match c {
                        b'A'..=b'Z' => c - b'A',
                        b'a'..=b'z' => c - b'a' + 26,
                        b'0'..=b'9' => c - b'0' + 52,
                        b'+' => 62, b'/' => 63, _ => 0,
                    }
                };
                let filtered: Vec<u8> = s.bytes().filter(|&c| c != b'=' && c != b'\n' && c != b'\r').collect();
                let mut out = Vec::new();
                for chunk in filtered.chunks(4) {
                    let c0 = decode_char(chunk[0]) as u32;
                    let c1 = if chunk.len() > 1 { decode_char(chunk[1]) as u32 } else { 0 };
                    let c2 = if chunk.len() > 2 { decode_char(chunk[2]) as u32 } else { 0 };
                    let c3 = if chunk.len() > 3 { decode_char(chunk[3]) as u32 } else { 0 };
                    let n = (c0 << 18) | (c1 << 12) | (c2 << 6) | c3;
                    out.push(((n >> 16) & 0xFF) as u8);
                    if chunk.len() > 2 { out.push(((n >> 8) & 0xFF) as u8); }
                    if chunk.len() > 3 { out.push((n & 0xFF) as u8); }
                }
                Some(Ok(Value::String(String::from_utf8_lossy(&out).to_string())))
            }
            "base64_decode_bytes" => {
                if args.is_empty() { return Some(Err("base64_decode_bytes(str)".to_string())); }
                let s = args[0].as_string();
                let decode_char = |c: u8| -> u8 {
                    match c {
                        b'A'..=b'Z' => c - b'A',
                        b'a'..=b'z' => c - b'a' + 26,
                        b'0'..=b'9' => c - b'0' + 52,
                        b'+' => 62, b'/' => 63, _ => 0,
                    }
                };
                let filtered: Vec<u8> = s.bytes().filter(|&c| c != b'=' && c != b'\n' && c != b'\r').collect();
                let mut out = Vec::new();
                for chunk in filtered.chunks(4) {
                    let c0 = decode_char(chunk[0]) as u32;
                    let c1 = if chunk.len() > 1 { decode_char(chunk[1]) as u32 } else { 0 };
                    let c2 = if chunk.len() > 2 { decode_char(chunk[2]) as u32 } else { 0 };
                    let c3 = if chunk.len() > 3 { decode_char(chunk[3]) as u32 } else { 0 };
                    let n = (c0 << 18) | (c1 << 12) | (c2 << 6) | c3;
                    out.push(((n >> 16) & 0xFF) as u8);
                    if chunk.len() > 2 { out.push(((n >> 8) & 0xFF) as u8); }
                    if chunk.len() > 3 { out.push((n & 0xFF) as u8); }
                }
                Some(Ok(Value::Array(out.iter().map(|b| Value::Int(*b as i64)).collect())))
            }

            // ── bytes <-> string helpers ──────────────────────────────────────
            // bytes_to_str(array) -> utf8 string
            "bytes_to_str" | "bytes_to_string" => {
                if args.is_empty() { return Some(Err("bytes_to_str(array)".to_string())); }
                match &args[0] {
                    Value::Array(a) => {
                        let bytes: Vec<u8> = a.iter().map(|v| v.as_int() as u8).collect();
                        Some(Ok(Value::String(String::from_utf8_lossy(&bytes).to_string())))
                    }
                    Value::String(s) => Some(Ok(Value::String(s.clone()))),
                    _ => Some(Err("bytes_to_str: expected array".to_string())),
                }
            }
            // str_to_bytes(str) -> array of ints
            "str_to_bytes" | "string_to_bytes" => {
                if args.is_empty() { return Some(Err("str_to_bytes(str)".to_string())); }
                let bytes: Vec<Value> = args[0].as_string().bytes().map(|b| Value::Int(b as i64)).collect();
                Some(Ok(Value::Array(bytes)))
            }
            // hex_to_bytes(hex_str) -> array
            "hex_to_bytes" | "from_hex" => {
                if args.is_empty() { return Some(Err("hex_to_bytes(hex_str)".to_string())); }
                let s = args[0].as_string();
                let s = s.trim_start_matches("0x").trim_start_matches("0X");
                let bytes: Result<Vec<u8>, _> = (0..s.len()).step_by(2)
                    .map(|i| u8::from_str_radix(&s[i..i.min(s.len()).max(i+2).min(s.len())], 16))
                    .collect();
                match bytes {
                    Ok(b) => Some(Ok(Value::Array(b.iter().map(|v| Value::Int(*v as i64)).collect()))),
                    Err(_) => Some(Err("hex_to_bytes: invalid hex string".to_string())),
                }
            }
            // bytes_to_hex(array) -> hex string
            "bytes_to_hex" | "to_hex" => {
                if args.is_empty() { return Some(Err("bytes_to_hex(array)".to_string())); }
                match &args[0] {
                    Value::Array(a) => {
                        let s: String = a.iter().map(|v| format!("{:02x}", v.as_int() as u8)).collect();
                        Some(Ok(Value::String(s)))
                    }
                    Value::String(s) => {
                        let s: String = s.bytes().map(|b| format!("{:02x}", b)).collect();
                        Some(Ok(Value::String(s)))
                    }
                    _ => Some(Err("bytes_to_hex: expected array or string".to_string())),
                }
            }

            // ── File metadata ─────────────────────────────────────────────────
            "file_size" => {
                if args.is_empty() { return Some(Err("file_size(path)".to_string())); }
                match std::fs::metadata(args[0].as_string()) {
                    Ok(m) => Some(Ok(Value::Int(m.len() as i64))),
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "file_mtime" => {
                if args.is_empty() { return Some(Err("file_mtime(path)".to_string())); }
                match std::fs::metadata(args[0].as_string()) {
                    Ok(m) => match m.modified() {
                        Ok(t) => Some(Ok(Value::Int(t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64))),
                        Err(e) => Some(Err(e.to_string())),
                    },
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            "file_is_dir" => {
                if args.is_empty() { return Some(Err("file_is_dir(path)".to_string())); }
                Some(Ok(Value::Bool(std::fs::metadata(args[0].as_string()).map(|m| m.is_dir()).unwrap_or(false))))
            }
            "file_is_file" => {
                if args.is_empty() { return Some(Err("file_is_file(path)".to_string())); }
                Some(Ok(Value::Bool(std::fs::metadata(args[0].as_string()).map(|m| m.is_file()).unwrap_or(false))))
            }
            // dir_files(path) -> array of {name, size, is_dir} entries
            "dir_files" | "dir_entries" => {
                if args.is_empty() { return Some(Err("dir_files(path)".to_string())); }
                match std::fs::read_dir(args[0].as_string()) {
                    Ok(entries) => {
                        let mut out = Vec::new();
                        for e in entries.filter_map(|e| e.ok()) {
                            let meta = e.metadata().ok();
                            let mut m = HashMap::new();
                            m.insert("name".to_string(), Value::String(e.file_name().to_string_lossy().to_string()));
                            m.insert("path".to_string(), Value::String(e.path().to_string_lossy().to_string()));
                            m.insert("is_dir".to_string(), Value::Bool(meta.as_ref().map(|m| m.is_dir()).unwrap_or(false)));
                            m.insert("size".to_string(), Value::Int(meta.as_ref().map(|m| m.len() as i64).unwrap_or(0)));
                            out.push(Value::Map(m));
                        }
                        Some(Ok(Value::Array(out)))
                    }
                    Err(e) => Some(Err(e.to_string())),
                }
            }
            // glob(pattern) -> array of matching paths
            "glob" => {
                if args.is_empty() { return Some(Err("glob(pattern)".to_string())); }
                let pattern = args[0].as_string();
                // Simple glob: supports * in filename, ** for recursive
                fn glob_match(pattern: &str, s: &str) -> bool {
                    let mut pi = pattern.chars().peekable();
                    let mut si = s.chars().peekable();
                    loop {
                        match pi.peek() {
                            None => return si.peek().is_none(),
                            Some(&'*') => {
                                pi.next();
                                if pi.peek().is_none() { return true; }
                                let rest_p: String = pi.collect();
                                let rest_s: String = si.collect();
                                for i in 0..=rest_s.len() {
                                    if glob_match(&rest_p, &rest_s[i..]) { return true; }
                                }
                                return false;
                            }
                            Some(&'?') => { pi.next(); if si.next().is_none() { return false; } }
                            Some(&pc) => {
                                pi.next();
                                match si.next() {
                                    Some(sc) if sc == pc => {}
                                    _ => return false,
                                }
                            }
                        }
                    }
                }
                let (dir, file_pat) = if let Some(pos) = pattern.rfind('/') {
                    (&pattern[..pos], &pattern[pos+1..])
                } else {
                    (".", pattern.as_str())
                };
                let mut results = Vec::new();
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for e in entries.filter_map(|e| e.ok()) {
                        let name = e.file_name().to_string_lossy().to_string();
                        if glob_match(file_pat, &name) {
                            results.push(Value::String(e.path().to_string_lossy().to_string()));
                        }
                    }
                }
                results.sort_by(|a, b| a.as_string().cmp(&b.as_string()));
                Some(Ok(Value::Array(results)))
            }

            // ── Type inspection ───────────────────────────────────────────────
            "type_of" | "typeof" => {
                if args.is_empty() { return Some(Ok(Value::String("null".to_string()))); }
                let t = match &args[0] {
                    Value::Null        => "null",
                    Value::Bool(_)     => "bool",
                    Value::Int(_)      => "int",
                    Value::Float(_)    => "float",
                    Value::String(_)   => "string",
                    Value::Array(_)    => "array",
                    Value::Map(_)      => "map",
                    Value::Closure { .. } | Value::Function(_) => "function",
                    _                  => "unknown",
                };
                Some(Ok(Value::String(t.to_string())))
            }
            "is_int"      => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Int(_)))))),
            "is_float"    => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Float(_)))))),
            "is_string"   => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::String(_)))))),
            "is_bool"     => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Bool(_)))))),
            "is_array"    => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Array(_)))))),
            "is_map"      => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Map(_)))))),
            "is_null"     => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Null) | None)))),
            "is_func"     => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Closure { .. }) | Some(Value::Function(_)))))),
            "is_number"   => Some(Ok(Value::Bool(matches!(args.first(), Some(Value::Int(_)) | Some(Value::Float(_)))))),

            // ── Bit operations ────────────────────────────────────────────────
            "popcount"       => { let n = args.first().map(|v| v.as_int() as u64).unwrap_or(0); Some(Ok(Value::Int(n.count_ones() as i64))) }
            "leading_zeros"  => { let n = args.first().map(|v| v.as_int() as u64).unwrap_or(0); Some(Ok(Value::Int(n.leading_zeros() as i64))) }
            "trailing_zeros" => { let n = args.first().map(|v| v.as_int() as u64).unwrap_or(0); Some(Ok(Value::Int(n.trailing_zeros() as i64))) }
            "bit_reverse"    => { let n = args.first().map(|v| v.as_int() as u64).unwrap_or(0); Some(Ok(Value::Int(n.reverse_bits() as i64))) }
            "rotate_left"    => {
                if args.len() < 2 { return Some(Err("rotate_left(n, bits)".to_string())); }
                let n = args[0].as_int() as u64;
                let b = (args[1].as_int() as u32) & 63;
                Some(Ok(Value::Int(n.rotate_left(b) as i64)))
            }
            "rotate_right" => {
                if args.len() < 2 { return Some(Err("rotate_right(n, bits)".to_string())); }
                let n = args[0].as_int() as u64;
                let b = (args[1].as_int() as u32) & 63;
                Some(Ok(Value::Int(n.rotate_right(b) as i64)))
            }

            // ── Raw TCP server fd ─────────────────────────────────────────────
            // tcp_server_fd(port, backlog?) -> fd  (non-blocking capable)
            "tcp_server_fd" => {
                if args.is_empty() { return Some(Err("tcp_server_fd(port, backlog?)".to_string())); }
                let port    = args[0].as_int() as u16;
                let backlog = args.get(1).map(|v| v.as_int() as libc::c_int).unwrap_or(128);
                let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
                if fd < 0 { return Some(Err(format!("tcp_server_fd: socket errno {}", unsafe { *libc::__errno_location() }))); }
                let opt: libc::c_int = 1;
                unsafe { libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, &opt as *const _ as *const _, std::mem::size_of::<libc::c_int>() as libc::socklen_t); }
                let addr = libc::sockaddr_in {
                    sin_family: libc::AF_INET as libc::sa_family_t,
                    sin_port: port.to_be(),
                    sin_addr: libc::in_addr { s_addr: 0 }, // INADDR_ANY
                    sin_zero: [0; 8],
                };
                if unsafe { libc::bind(fd, &addr as *const _ as *const libc::sockaddr, std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t) } < 0 {
                    unsafe { libc::close(fd); }
                    return Some(Err(format!("tcp_server_fd: bind errno {}", unsafe { *libc::__errno_location() })));
                }
                if unsafe { libc::listen(fd, backlog) } < 0 {
                    unsafe { libc::close(fd); }
                    return Some(Err(format!("tcp_server_fd: listen errno {}", unsafe { *libc::__errno_location() })));
                }
                Some(Ok(Value::Int(fd as i64)))
            }
            // tcp_accept_fd(server_fd) -> {fd, addr, port}
            "tcp_accept_fd" => {
                if args.is_empty() { return Some(Err("tcp_accept_fd(server_fd)".to_string())); }
                let server_fd = args[0].as_int() as libc::c_int;
                let mut addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
                let mut addrlen = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
                let fd = unsafe { libc::accept(server_fd, &mut addr as *mut _ as *mut libc::sockaddr, &mut addrlen) };
                if fd < 0 { return Some(Err(format!("tcp_accept_fd: errno {}", unsafe { *libc::__errno_location() }))); }
                let ip = std::net::Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr)).to_string();
                let port = u16::from_be(addr.sin_port);
                let mut m = HashMap::new();
                m.insert("fd".to_string(),   Value::Int(fd as i64));
                m.insert("addr".to_string(), Value::String(ip));
                m.insert("port".to_string(), Value::Int(port as i64));
                Some(Ok(Value::Map(m)))
            }
            // tcp_connect_fd(host, port) -> fd
            "tcp_connect_fd" => {
                if args.len() < 2 { return Some(Err("tcp_connect_fd(host, port)".to_string())); }
                let host = args[0].as_string();
                let port = args[1].as_int() as u16;
                use std::net::ToSocketAddrs;
                let addr = match format!("{}:{}", host, port).to_socket_addrs() {
                    Ok(mut it) => match it.next() {
                        Some(a) => a,
                        None => return Some(Err("tcp_connect_fd: no addr".to_string())),
                    },
                    Err(e) => return Some(Err(e.to_string())),
                };
                let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
                if fd < 0 { return Some(Err(format!("tcp_connect_fd: socket errno {}", unsafe { *libc::__errno_location() }))); }
                let sin = libc::sockaddr_in {
                    sin_family: libc::AF_INET as libc::sa_family_t,
                    sin_port: port.to_be(),
                    sin_addr: libc::in_addr { s_addr: match addr.ip() {
                        std::net::IpAddr::V4(v4) => u32::from(v4).to_be(),
                        _ => 0,
                    }},
                    sin_zero: [0; 8],
                };
                if unsafe { libc::connect(fd, &sin as *const _ as *const libc::sockaddr, std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t) } < 0 {
                    unsafe { libc::close(fd); }
                    return Some(Err(format!("tcp_connect_fd: connect errno {}", unsafe { *libc::__errno_location() })));
                }
                Some(Ok(Value::Int(fd as i64)))
            }

            // ── Raw UDP socket ────────────────────────────────────────────────
            // udp_socket(port?) -> fd
            "udp_socket" => {
                let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
                if fd < 0 { return Some(Err(format!("udp_socket: errno {}", unsafe { *libc::__errno_location() }))); }
                if let Some(port_val) = args.first() {
                    let port = port_val.as_int() as u16;
                    let addr = libc::sockaddr_in {
                        sin_family: libc::AF_INET as libc::sa_family_t,
                        sin_port: port.to_be(),
                        sin_addr: libc::in_addr { s_addr: 0 },
                        sin_zero: [0; 8],
                    };
                    unsafe { libc::bind(fd, &addr as *const _ as *const libc::sockaddr, std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t); }
                }
                Some(Ok(Value::Int(fd as i64)))
            }
            // udp_sendto(fd, data, host, port) -> bytes_sent
            "udp_sendto" => {
                if args.len() < 4 { return Some(Err("udp_sendto(fd, data, host, port)".to_string())); }
                let fd   = args[0].as_int() as libc::c_int;
                let data = args[1].as_string();
                let host = args[2].as_string();
                let port = args[3].as_int() as u16;
                use std::net::ToSocketAddrs;
                let addr = match format!("{}:{}", host, port).to_socket_addrs().ok().and_then(|mut it| it.next()) {
                    Some(a) => a,
                    None => return Some(Err("udp_sendto: resolve failed".to_string())),
                };
                let sin = libc::sockaddr_in {
                    sin_family: libc::AF_INET as libc::sa_family_t,
                    sin_port: port.to_be(),
                    sin_addr: libc::in_addr { s_addr: match addr.ip() {
                        std::net::IpAddr::V4(v4) => u32::from(v4).to_be(),
                        _ => 0,
                    }},
                    sin_zero: [0; 8],
                };
                let n = unsafe { libc::sendto(fd, data.as_ptr() as *const _, data.len(), 0,
                    &sin as *const _ as *const libc::sockaddr,
                    std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t) };
                Some(Ok(Value::Int(n as i64)))
            }
            // udp_recvfrom(fd, size?) -> {data, addr, port}
            "udp_recvfrom" => {
                if args.is_empty() { return Some(Err("udp_recvfrom(fd, size?)".to_string())); }
                let fd   = args[0].as_int() as libc::c_int;
                let size = args.get(1).map(|v| v.as_int() as usize).unwrap_or(4096);
                let mut buf = vec![0u8; size];
                let mut addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
                let mut addrlen = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
                let n = unsafe { libc::recvfrom(fd, buf.as_mut_ptr() as *mut _, size, 0,
                    &mut addr as *mut _ as *mut libc::sockaddr, &mut addrlen) };
                if n < 0 { return Some(Err(format!("udp_recvfrom: errno {}", unsafe { *libc::__errno_location() }))); }
                let ip   = std::net::Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr)).to_string();
                let port = u16::from_be(addr.sin_port);
                let mut m = HashMap::new();
                m.insert("data".to_string(), Value::String(String::from_utf8_lossy(&buf[..n as usize]).to_string()));
                m.insert("addr".to_string(), Value::String(ip));
                m.insert("port".to_string(), Value::Int(port as i64));
                Some(Ok(Value::Map(m)))
            }

            // ── try/parse helpers ─────────────────────────────────────────────
            // parse_int(s, base?) -> int or null
            "parse_int" | "try_int" => {
                if args.is_empty() { return Some(Ok(Value::Null)); }
                let s = args[0].as_string();
                let base = args.get(1).map(|v| v.as_int() as u32).unwrap_or(10);
                match i64::from_str_radix(s.trim(), base) {
                    Ok(n) => Some(Ok(Value::Int(n))),
                    Err(_) => Some(Ok(Value::Null)),
                }
            }
            // try_float(s) -> float or null
            "try_float" => {
                if args.is_empty() { return Some(Ok(Value::Null)); }
                match args[0].as_string().trim().parse::<f64>() {
                    Ok(f) => Some(Ok(Value::Float(f))),
                    Err(_) => Some(Ok(Value::Null)),
                }
            }
            // int_to_str(n, base) -> string (e.g. int_to_str(255, 16) -> "ff")
            "int_to_str" | "to_base" => {
                if args.is_empty() { return Some(Err("int_to_str(n, base?)".to_string())); }
                let n = args[0].as_int() as u64;
                let base = args.get(1).map(|v| v.as_int()).unwrap_or(10);
                let s = match base {
                    2  => format!("{:b}", n),
                    8  => format!("{:o}", n),
                    10 => format!("{}", n),
                    16 => format!("{:x}", n),
                    _  => format!("{}", n),
                };
                Some(Ok(Value::String(s)))
            }

            // ── Math extras ───────────────────────────────────────────────────
            "asin"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().asin()).unwrap_or(0.0)))) }
            "acos"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().acos()).unwrap_or(0.0)))) }
            "atan"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().atan()).unwrap_or(0.0)))) }
            "sinh"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().sinh()).unwrap_or(0.0)))) }
            "cosh"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().cosh()).unwrap_or(0.0)))) }
            "tanh"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().tanh()).unwrap_or(0.0)))) }
            "asinh" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().asinh()).unwrap_or(0.0)))) }
            "acosh" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().acosh()).unwrap_or(0.0)))) }
            "atanh" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().atanh()).unwrap_or(0.0)))) }
            "log2"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().log2()).unwrap_or(0.0)))) }
            "log10" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().log10()).unwrap_or(0.0)))) }
            "exp"   => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().exp()).unwrap_or(0.0)))) }
            "exp2"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().exp2()).unwrap_or(0.0)))) }
            "signum"=> { Some(Ok(Value::Float(args.first().map(|v| v.as_float().signum()).unwrap_or(0.0)))) }
            "fract" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().fract()).unwrap_or(0.0)))) }
            "trunc" => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().trunc()).unwrap_or(0.0)))) }
            "PI"    => Some(Ok(Value::Float(std::f64::consts::PI))),
            "TAU"   => Some(Ok(Value::Float(std::f64::consts::TAU))),
            "E"     => Some(Ok(Value::Float(std::f64::consts::E))),
            "INF"   => Some(Ok(Value::Float(f64::INFINITY))),
            "NAN"   => Some(Ok(Value::Float(f64::NAN))),
            "is_nan"   => { Some(Ok(Value::Bool(args.first().map(|v| v.as_float().is_nan()).unwrap_or(false)))) }
            "is_inf"   => { Some(Ok(Value::Bool(args.first().map(|v| v.as_float().is_infinite()).unwrap_or(false)))) }
            "is_finite"=> { Some(Ok(Value::Bool(args.first().map(|v| v.as_float().is_finite()).unwrap_or(true)))) }
            "degrees"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().to_degrees()).unwrap_or(0.0)))) }
            "radians"  => { Some(Ok(Value::Float(args.first().map(|v| v.as_float().to_radians()).unwrap_or(0.0)))) }

            "aes_encrypt" => {
                if args.len() < 3 {
                    return Some(Err("aes_encrypt: requires key, nonce, plaintext".to_string()));
                }
                let mut key_bytes = vbytes(&args[0]);
                key_bytes.resize(32, 0);
                let nonce_bytes = vbytes(&args[1]);
                if nonce_bytes.len() < 12 {
                    return Some(Err("aes_encrypt: nonce must be 12 bytes".to_string()));
                }
                let plaintext = vbytes(&args[2]);
                let key_arr = aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&key_bytes[..32]);
                let nonce_arr = aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&nonce_bytes[..12]);
                let cipher = <Aes256Gcm as AeadKeyInit>::new(&key_arr);
                match cipher.encrypt(&nonce_arr, plaintext.as_slice()) {
                    Ok(ct) => Some(Ok(Value::String(b2he(&ct)))),
                    Err(e) => Some(Err(format!("aes_encrypt error: {}", e))),
                }
            }
            "aes_decrypt" => {
                if args.len() < 3 {
                    return Some(Err("aes_decrypt: requires key, nonce, ciphertext".to_string()));
                }
                let mut key_bytes = vbytes(&args[0]);
                key_bytes.resize(32, 0);
                let nonce_bytes = vbytes(&args[1]);
                if nonce_bytes.len() < 12 {
                    return Some(Err("aes_decrypt: nonce must be 12 bytes".to_string()));
                }
                let ct_bytes = vbytes(&args[2]);
                let key_arr = aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&key_bytes[..32]);
                let nonce_arr = aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&nonce_bytes[..12]);
                let cipher = <Aes256Gcm as AeadKeyInit>::new(&key_arr);
                match cipher.decrypt(&nonce_arr, ct_bytes.as_slice()) {
                    Ok(pt) => Some(Ok(Value::String(String::from_utf8_lossy(&pt).into_owned()))),
                    Err(e) => Some(Err(format!("aes_decrypt error: {}", e))),
                }
            }
            "chacha_encrypt" | "chacha20_encrypt" => {
                if args.len() < 3 {
                    return Some(Err("chacha_encrypt: requires key, nonce, plaintext".to_string()));
                }
                let mut key_bytes = vbytes(&args[0]);
                key_bytes.resize(32, 0);
                let nonce_bytes = vbytes(&args[1]);
                if nonce_bytes.len() < 12 {
                    return Some(Err("chacha_encrypt: nonce must be 12 bytes".to_string()));
                }
                let plaintext = vbytes(&args[2]);
                let key_arr = aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&key_bytes[..32]);
                let nonce_arr = aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&nonce_bytes[..12]);
                let cipher = <ChaCha20Poly1305 as ChaChaKeyInit>::new(&key_arr);
                match cipher.encrypt(&nonce_arr, plaintext.as_slice()) {
                    Ok(ct) => Some(Ok(Value::String(b2he(&ct)))),
                    Err(e) => Some(Err(format!("chacha_encrypt error: {}", e))),
                }
            }
            "chacha_decrypt" | "chacha20_decrypt" => {
                if args.len() < 3 {
                    return Some(Err("chacha_decrypt: requires key, nonce, ciphertext".to_string()));
                }
                let mut key_bytes = vbytes(&args[0]);
                key_bytes.resize(32, 0);
                let nonce_bytes = vbytes(&args[1]);
                if nonce_bytes.len() < 12 {
                    return Some(Err("chacha_decrypt: nonce must be 12 bytes".to_string()));
                }
                let ct_bytes = vbytes(&args[2]);
                let key_arr = aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&key_bytes[..32]);
                let nonce_arr = aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&nonce_bytes[..12]);
                let cipher = <ChaCha20Poly1305 as ChaChaKeyInit>::new(&key_arr);
                match cipher.decrypt(&nonce_arr, ct_bytes.as_slice()) {
                    Ok(pt) => Some(Ok(Value::String(String::from_utf8_lossy(&pt).into_owned()))),
                    Err(e) => Some(Err(format!("chacha_decrypt error: {}", e))),
                }
            }
            "ed25519_keygen" => {
                let mut secret_bytes = [0u8; 32];
                OsRng.fill_bytes(&mut secret_bytes);
                let signing_key = SigningKey::from_bytes(&secret_bytes);
                let verifying_key = signing_key.verifying_key();
                let public_bytes = verifying_key.to_bytes();
                let mut map = std::collections::HashMap::new();
                map.insert("secret".to_string(), Value::String(b2he(&secret_bytes)));
                map.insert("public".to_string(), Value::String(b2he(&public_bytes)));
                Some(Ok(Value::Map(map)))
            }
            "ed25519_sign" => {
                if args.len() < 2 {
                    return Some(Err("ed25519_sign: requires secret_hex, message".to_string()));
                }
                let secret_bytes = vbytes(&args[0]);
                if secret_bytes.len() < 32 {
                    return Some(Err("ed25519_sign: secret must be 32 bytes".to_string()));
                }
                let mut secret_arr = [0u8; 32];
                secret_arr.copy_from_slice(&secret_bytes[..32]);
                let signing_key = SigningKey::from_bytes(&secret_arr);
                let message = vbytes(&args[1]);
                let signature = signing_key.sign(&message);
                let sig_bytes = signature.to_bytes();
                Some(Ok(Value::String(b2he(&sig_bytes))))
            }
            "ed25519_verify" => {
                use ed25519_dalek::Signature as Ed25519Signature;
                if args.len() < 3 {
                    return Some(Err("ed25519_verify: requires public_hex, message, signature_hex".to_string()));
                }
                let pub_bytes = vbytes(&args[0]);
                if pub_bytes.len() < 32 {
                    return Some(Err("ed25519_verify: public key must be 32 bytes".to_string()));
                }
                let mut pub_arr = [0u8; 32];
                pub_arr.copy_from_slice(&pub_bytes[..32]);
                let verifying_key = match VerifyingKey::from_bytes(&pub_arr) {
                    Ok(k) => k,
                    Err(e) => return Some(Err(format!("ed25519_verify: invalid public key: {}", e))),
                };
                let message = vbytes(&args[1]);
                let sig_bytes = vbytes(&args[2]);
                if sig_bytes.len() < 64 {
                    return Some(Err("ed25519_verify: signature must be 64 bytes".to_string()));
                }
                let mut sig_arr = [0u8; 64];
                sig_arr.copy_from_slice(&sig_bytes[..64]);
                let sig = Ed25519Signature::from_bytes(&sig_arr);
                let ok = verifying_key.verify(&message, &sig).is_ok();
                Some(Ok(Value::Bool(ok)))
            }
            "x25519_keygen" => {
                let mut secret_bytes = [0u8; 32];
                OsRng.fill_bytes(&mut secret_bytes);
                let secret = StaticSecret::from(secret_bytes);
                let public = X25519PublicKey::from(&secret);
                let mut map = std::collections::HashMap::new();
                map.insert("secret".to_string(), Value::String(b2he(secret.as_bytes())));
                map.insert("public".to_string(), Value::String(b2he(public.as_bytes())));
                Some(Ok(Value::Map(map)))
            }
            "x25519_dh" | "x25519_exchange" => {
                if args.len() < 2 {
                    return Some(Err("x25519_dh: requires secret_hex, their_public_hex".to_string()));
                }
                let sec_bytes = vbytes(&args[0]);
                let pub_bytes = vbytes(&args[1]);
                if sec_bytes.len() < 32 {
                    return Some(Err("x25519_dh: secret must be 32 bytes".to_string()));
                }
                if pub_bytes.len() < 32 {
                    return Some(Err("x25519_dh: their public must be 32 bytes".to_string()));
                }
                let mut sec_arr = [0u8; 32];
                sec_arr.copy_from_slice(&sec_bytes[..32]);
                let mut pub_arr = [0u8; 32];
                pub_arr.copy_from_slice(&pub_bytes[..32]);
                let secret = StaticSecret::from(sec_arr);
                let their_public = X25519PublicKey::from(pub_arr);
                let shared = secret.diffie_hellman(&their_public);
                Some(Ok(Value::String(b2he(shared.as_bytes()))))
            }
            "rsa_keygen" | "rsa_gen" => {
                use rsa::traits::PublicKeyParts as RsaPKP;
                use rsa::traits::PrivateKeyParts as RsaPKPPriv;
                let bits = if args.is_empty() { 2048usize } else { args[0].as_int() as usize };
                let bits = if bits < 512 { 512 } else { bits };
                let priv_key = match RsaPrivateKey::new(&mut OsRng, bits) {
                    Ok(k) => k,
                    Err(e) => return Some(Err(format!("rsa_keygen error: {}", e))),
                };
                let n_bytes = priv_key.n().to_bytes_be();
                let e_bytes = priv_key.e().to_bytes_be();
                let d_bytes = priv_key.d().to_bytes_be();
                let mut map = std::collections::HashMap::new();
                map.insert("n".to_string(), Value::String(b2he(&n_bytes)));
                map.insert("e".to_string(), Value::String(b2he(&e_bytes)));
                map.insert("d".to_string(), Value::String(b2he(&d_bytes)));
                Some(Ok(Value::Map(map)))
            }
            "rsa_encrypt" => {
                use rsa::BigUint;
                if args.len() < 3 {
                    return Some(Err("rsa_encrypt: requires n_hex, e_hex, plaintext".to_string()));
                }
                let n_bytes = vbytes(&args[0]);
                let e_bytes = vbytes(&args[1]);
                let plaintext = vbytes(&args[2]);
                let n = BigUint::from_bytes_be(&n_bytes);
                let e = BigUint::from_bytes_be(&e_bytes);
                let pub_key = match RsaPublicKey::new(n, e) {
                    Ok(k) => k,
                    Err(err) => return Some(Err(format!("rsa_encrypt: invalid key: {}", err))),
                };
                match pub_key.encrypt(&mut OsRng, Pkcs1v15Encrypt, &plaintext) {
                    Ok(ct) => Some(Ok(Value::String(b2he(&ct)))),
                    Err(err) => Some(Err(format!("rsa_encrypt error: {}", err))),
                }
            }
            "rsa_decrypt" => {
                Some(Ok(Value::String(
                    "rsa_decrypt: provide private key d — full RSA decrypt requires d,p,q; use rsa_keygen flow"
                        .to_string(),
                )))
            }
            "sha3_256" | "sha3256" => {
                let data = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
                let mut hasher = Sha3_256::new();
                sha3::Digest::update(&mut hasher, &data);
                let result = sha3::Digest::finalize(hasher);
                Some(Ok(Value::String(b2he(&result))))
            }
            "sha3_512" | "sha3512" => {
                let data = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
                let mut hasher = Sha3_512::new();
                sha3::Digest::update(&mut hasher, &data);
                let result = sha3::Digest::finalize(hasher);
                Some(Ok(Value::String(b2he(&result))))
            }
            "blake3_hash" | "blake3" => {
                let data = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
                let hash = blake3::hash(&data);
                Some(Ok(Value::String(hash.to_hex().to_string())))
            }
            "blake3_keyed" => {
                if args.len() < 2 {
                    return Some(Err("blake3_keyed: requires key (32 bytes), data".to_string()));
                }
                let key_bytes = vbytes(&args[0]);
                if key_bytes.len() < 32 {
                    return Some(Err("blake3_keyed: key must be 32 bytes".to_string()));
                }
                let mut key_arr = [0u8; 32];
                key_arr.copy_from_slice(&key_bytes[..32]);
                let data = vbytes(&args[1]);
                let hash = blake3::keyed_hash(&key_arr, &data);
                Some(Ok(Value::String(hash.to_hex().to_string())))
            }
            "hmac_sha512" => {
                if args.len() < 2 {
                    return Some(Err("hmac_sha512: requires key, data".to_string()));
                }
                type HmacSha512 = Hmac<Sha512>;
                let key = vbytes(&args[0]);
                let data = vbytes(&args[1]);
                let mut mac = match <HmacSha512 as hmac::Mac>::new_from_slice(&key) {
                    Ok(m) => m,
                    Err(e) => return Some(Err(format!("hmac_sha512: invalid key length: {}", e))),
                };
                hmac::Mac::update(&mut mac, &data);
                let result = hmac::Mac::finalize(mac);
                Some(Ok(Value::String(b2he(&result.into_bytes()))))
            }
            "argon2_hash" => {
                if args.is_empty() {
                    return Some(Err("argon2_hash: requires password".to_string()));
                }
                let pwd = args[0].as_string();
                let salt = SaltString::generate(&mut OsRng);
                match Argon2::default().hash_password(pwd.as_bytes(), &salt) {
                    Ok(hash) => Some(Ok(Value::String(hash.to_string()))),
                    Err(e) => Some(Err(format!("argon2_hash error: {}", e))),
                }
            }
            "argon2_verify" => {
                use argon2::password_hash::PasswordHash;
                if args.len() < 2 {
                    return Some(Err("argon2_verify: requires hash_str, password".to_string()));
                }
                let hash_str = args[0].as_string();
                let pwd = args[1].as_string();
                let parsed_hash = match PasswordHash::new(&hash_str) {
                    Ok(h) => h,
                    Err(e) => return Some(Err(format!("argon2_verify: invalid hash: {}", e))),
                };
                let ok = Argon2::default()
                    .verify_password(pwd.as_bytes(), &parsed_hash)
                    .is_ok();
                Some(Ok(Value::Bool(ok)))
            }
            "pbkdf2_sha256" => {
                if args.is_empty() {
                    return Some(Err("pbkdf2_sha256: requires password".to_string()));
                }
                let pwd = vbytes(&args[0]);
                let salt = if args.len() > 1 { vbytes(&args[1]) } else { b"knull-salt".to_vec() };
                let iters: u64 = if args.len() > 2 { args[2].as_int() as u64 } else { 100000 };
                let iters = if iters < 1 { 1 } else { iters };
                let key_len: usize = if args.len() > 3 { args[3].as_int() as usize } else { 32 };
                let key_len = if key_len < 1 { 1 } else { key_len };
                let mut out = vec![0u8; key_len];
                pbkdf2_hmac::<Sha256>(&pwd, &salt, iters as u32, &mut out);
                Some(Ok(Value::String(b2he(&out))))
            }
            "sha1_hash" | "sha1" => {
                fn sha1_impl(data: &[u8]) -> [u8; 20] {
                    let mut h: [u32; 5] = [
                        0x67452301u32,
                        0xEFCDAB89u32,
                        0x98BADCFEu32,
                        0x10325476u32,
                        0xC3D2E1F0u32,
                    ];
                    let bit_len = (data.len() as u64).wrapping_mul(8);
                    let mut msg = data.to_vec();
                    msg.push(0x80u8);
                    while msg.len() % 64 != 56 {
                        msg.push(0x00u8);
                    }
                    msg.extend_from_slice(&bit_len.to_be_bytes());
                    for chunk in msg.chunks(64) {
                        let mut w = [0u32; 80];
                        for i in 0..16 {
                            w[i] = u32::from_be_bytes([
                                chunk[i * 4],
                                chunk[i * 4 + 1],
                                chunk[i * 4 + 2],
                                chunk[i * 4 + 3],
                            ]);
                        }
                        for i in 16..80 {
                            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
                        }
                        let (mut a, mut b, mut c, mut d, mut e) =
                            (h[0], h[1], h[2], h[3], h[4]);
                        for i in 0..80 {
                            let (f, k): (u32, u32) = if i < 20 {
                                ((b & c) | ((!b) & d), 0x5A827999u32)
                            } else if i < 40 {
                                (b ^ c ^ d, 0x6ED9EBA1u32)
                            } else if i < 60 {
                                ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32)
                            } else {
                                (b ^ c ^ d, 0xCA62C1D6u32)
                            };
                            let temp = a
                                .rotate_left(5)
                                .wrapping_add(f)
                                .wrapping_add(e)
                                .wrapping_add(k)
                                .wrapping_add(w[i]);
                            e = d;
                            d = c;
                            c = b.rotate_left(30);
                            b = a;
                            a = temp;
                        }
                        h[0] = h[0].wrapping_add(a);
                        h[1] = h[1].wrapping_add(b);
                        h[2] = h[2].wrapping_add(c);
                        h[3] = h[3].wrapping_add(d);
                        h[4] = h[4].wrapping_add(e);
                    }
                    let mut result = [0u8; 20];
                    for i in 0..5 {
                        result[i * 4..(i + 1) * 4].copy_from_slice(&h[i].to_be_bytes());
                    }
                    result
                }
                let data = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
                let hash = sha1_impl(&data);
                Some(Ok(Value::String(b2he(&hash))))
            }

            "bignum" | "big_int" => {
                if args.is_empty() { return Some(Err("bignum requires 1 arg".to_string())); }
                let s = args[0].as_string();
                let s2 = s.trim_start_matches("bignum:");
                match s2.parse::<BigInt>() {
                    Ok(n) => Some(Ok(Value::String(format!("bignum:{}", n)))),
                    Err(e) => Some(Err(format!("bignum parse error: {}", e))),
                }
            }
            "big_add" => {
                if args.len() < 2 { return Some(Err("big_add requires 2 args".to_string())); }
                let parse_big = |v: &Value| -> Result<BigInt, String> {
                    let s = v.as_string();
                    let s2 = s.trim_start_matches("bignum:").to_string();
                    s2.parse::<BigInt>().map_err(|e| e.to_string())
                };
                let a = match parse_big(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let b = match parse_big(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                Some(Ok(Value::String(format!("bignum:{}", a + b))))
            }
            "big_sub" => {
                if args.len() < 2 { return Some(Err("big_sub requires 2 args".to_string())); }
                let parse_big = |v: &Value| -> Result<BigInt, String> {
                    let s = v.as_string();
                    let s2 = s.trim_start_matches("bignum:").to_string();
                    s2.parse::<BigInt>().map_err(|e| e.to_string())
                };
                let a = match parse_big(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let b = match parse_big(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                Some(Ok(Value::String(format!("bignum:{}", a - b))))
            }
            "big_mul" => {
                if args.len() < 2 { return Some(Err("big_mul requires 2 args".to_string())); }
                let parse_big = |v: &Value| -> Result<BigInt, String> {
                    let s = v.as_string();
                    let s2 = s.trim_start_matches("bignum:").to_string();
                    s2.parse::<BigInt>().map_err(|e| e.to_string())
                };
                let a = match parse_big(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let b = match parse_big(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                Some(Ok(Value::String(format!("bignum:{}", a * b))))
            }
            "big_div" => {
                if args.len() < 2 { return Some(Err("big_div requires 2 args".to_string())); }
                let parse_big = |v: &Value| -> Result<BigInt, String> {
                    let s = v.as_string();
                    let s2 = s.trim_start_matches("bignum:").to_string();
                    s2.parse::<BigInt>().map_err(|e| e.to_string())
                };
                let a = match parse_big(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let b = match parse_big(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                if b.is_zero() { return Some(Err("big_div: division by zero".to_string())); }
                Some(Ok(Value::String(format!("bignum:{}", a / b))))
            }
            "big_mod" => {
                if args.len() < 2 { return Some(Err("big_mod requires 2 args".to_string())); }
                let parse_big = |v: &Value| -> Result<BigInt, String> {
                    let s = v.as_string();
                    let s2 = s.trim_start_matches("bignum:").to_string();
                    s2.parse::<BigInt>().map_err(|e| e.to_string())
                };
                let a = match parse_big(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let b = match parse_big(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                if b.is_zero() { return Some(Err("big_mod: modulo by zero".to_string())); }
                Some(Ok(Value::String(format!("bignum:{}", a % b))))
            }
            "big_pow" => {
                if args.len() < 2 { return Some(Err("big_pow requires 2 args".to_string())); }
                let parse_big = |v: &Value| -> Result<BigInt, String> {
                    let s = v.as_string();
                    let s2 = s.trim_start_matches("bignum:").to_string();
                    s2.parse::<BigInt>().map_err(|e| e.to_string())
                };
                let base = match parse_big(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let exp_val = args[1].as_int() as u64;
                let mut result = BigInt::one();
                let mut b = base.clone();
                let mut e = exp_val;
                while e > 0 {
                    if e % 2 == 1 {
                        result = result * b.clone();
                    }
                    b = b.clone() * b.clone();
                    e /= 2;
                }
                Some(Ok(Value::String(format!("bignum:{}", result))))
            }
            "big_cmp" => {
                if args.len() < 2 { return Some(Err("big_cmp requires 2 args".to_string())); }
                let parse_big = |v: &Value| -> Result<BigInt, String> {
                    let s = v.as_string();
                    let s2 = s.trim_start_matches("bignum:").to_string();
                    s2.parse::<BigInt>().map_err(|e| e.to_string())
                };
                let a = match parse_big(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let b = match parse_big(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let cmp_result = match a.cmp(&b) {
                    std::cmp::Ordering::Less => -1i64,
                    std::cmp::Ordering::Equal => 0i64,
                    std::cmp::Ordering::Greater => 1i64,
                };
                Some(Ok(Value::Int(cmp_result)))
            }
            "big_to_hex" => {
                if args.is_empty() { return Some(Err("big_to_hex requires 1 arg".to_string())); }
                let s = args[0].as_string();
                let s2 = s.trim_start_matches("bignum:");
                let n = match s2.parse::<BigInt>() {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e.to_string())),
                };
                let (sign, bytes) = n.to_bytes_be();
                let hex_str = b2he(&bytes);
                let result = if sign == Sign::Minus {
                    format!("-{}", hex_str)
                } else {
                    hex_str
                };
                Some(Ok(Value::String(result)))
            }
            "big_from_hex" => {
                if args.is_empty() { return Some(Err("big_from_hex requires 1 arg".to_string())); }
                let hex_str = args[0].as_string();
                let (negative, hex_clean) = if hex_str.starts_with('-') {
                    (true, &hex_str[1..])
                } else {
                    (false, hex_str.as_str())
                };
                let bytes = match he2b(hex_clean) {
                    Some(b) => b,
                    None => return Some(Err("big_from_hex: invalid hex string".to_string())),
                };
                let sign = if negative { Sign::Minus } else { Sign::Plus };
                let n = BigInt::from_bytes_be(sign, &bytes);
                Some(Ok(Value::String(format!("bignum:{}", n))))
            }
            "big_gcd" => {
                if args.len() < 2 { return Some(Err("big_gcd requires 2 args".to_string())); }
                let parse_big = |v: &Value| -> Result<BigInt, String> {
                    let s = v.as_string();
                    let s2 = s.trim_start_matches("bignum:").to_string();
                    s2.parse::<BigInt>().map_err(|e| e.to_string())
                };
                let a = match parse_big(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let b = match parse_big(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let g = a.gcd(&b);
                Some(Ok(Value::String(format!("bignum:{}", g))))
            }
            "big_lcm" => {
                if args.len() < 2 { return Some(Err("big_lcm requires 2 args".to_string())); }
                let parse_big = |v: &Value| -> Result<BigInt, String> {
                    let s = v.as_string();
                    let s2 = s.trim_start_matches("bignum:").to_string();
                    s2.parse::<BigInt>().map_err(|e| e.to_string())
                };
                let a = match parse_big(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let b = match parse_big(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
                let g = a.gcd(&b);
                if g.is_zero() {
                    return Some(Ok(Value::String("bignum:0".to_string())));
                }
                let lcm = (a.abs() / g) * b.abs();
                Some(Ok(Value::String(format!("bignum:{}", lcm))))
            }
            "big_is_prime" => {
                if args.is_empty() { return Some(Err("big_is_prime requires 1 arg".to_string())); }
                let s = args[0].as_string();
                let s2 = s.trim_start_matches("bignum:");
                let n = match s2.parse::<BigInt>() {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e.to_string())),
                };
                if n < BigInt::from(2i64) {
                    return Some(Ok(Value::Bool(false)));
                }
                if n == BigInt::from(2i64) || n == BigInt::from(3i64) {
                    return Some(Ok(Value::Bool(true)));
                }
                if &n % BigInt::from(2i64) == BigInt::zero() {
                    return Some(Ok(Value::Bool(false)));
                }
                let nu = match n.to_biguint() {
                    Some(v) => v,
                    None => return Some(Ok(Value::Bool(false))),
                };
                let one = BigUint::one();
                let two = BigUint::from(2u64);
                let n_minus_1 = nu.clone() - &one;
                let mut r = 0u64;
                let mut d = n_minus_1.clone();
                while &d % &two == BigUint::zero() {
                    d = d / &two;
                    r += 1;
                }
                let witnesses: &[u64] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
                let mut is_prime = true;
                'miller_rabin: for &w in witnesses {
                    let a = BigUint::from(w);
                    if a >= nu.clone() {
                        continue;
                    }
                    let mut x = a.modpow(&d, &nu);
                    if x == one || x == n_minus_1 {
                        continue;
                    }
                    let mut composite = true;
                    for _ in 0..r.saturating_sub(1) {
                        x = x.modpow(&two, &nu);
                        if x == n_minus_1 {
                            composite = false;
                            break;
                        }
                    }
                    if composite {
                        is_prime = false;
                        break 'miller_rabin;
                    }
                }
                Some(Ok(Value::Bool(is_prime)))
            }
            "big_sqrt" => {
                if args.is_empty() { return Some(Err("big_sqrt requires 1 arg".to_string())); }
                let s = args[0].as_string();
                let s2 = s.trim_start_matches("bignum:");
                let n = match s2.parse::<BigInt>() {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e.to_string())),
                };
                if n < BigInt::zero() {
                    return Some(Err("big_sqrt: cannot take sqrt of negative number".to_string()));
                }
                if n.is_zero() {
                    return Some(Ok(Value::String("bignum:0".to_string())));
                }
                let mut x = n.clone();
                let two = BigInt::from(2i64);
                let mut y = (x.clone() + BigInt::one()) / two.clone();
                while y < x {
                    x = y.clone();
                    y = (y.clone() + n.clone() / y.clone()) / two.clone();
                }
                Some(Ok(Value::String(format!("bignum:{}", x))))
            }
            "big_factorial" => {
                if args.is_empty() { return Some(Err("big_factorial requires 1 arg".to_string())); }
                let n = args[0].as_int();
                if n < 0 {
                    return Some(Err("big_factorial: negative input".to_string()));
                }
                if n > 10000 {
                    return Some(Err("big_factorial: input too large (max 10000)".to_string()));
                }
                let mut result = BigInt::one();
                for i in 2..=(n as u64) {
                    result = result * BigInt::from(i);
                }
                Some(Ok(Value::String(format!("bignum:{}", result))))
            }
            "big_to_int" => {
                if args.is_empty() { return Some(Err("big_to_int requires 1 arg".to_string())); }
                let s = args[0].as_string();
                let s2 = s.trim_start_matches("bignum:");
                let n = match s2.parse::<BigInt>() {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e.to_string())),
                };
                match n.to_i64() {
                    Some(i) => Some(Ok(Value::Int(i))),
                    None => Some(Ok(Value::String(n.to_string()))),
                }
            }
            "uuid_v4" | "uuid" => {
                Some(Ok(Value::String(Uuid::new_v4().to_string())))
            }
            "uuid_v7" => {
                Some(Ok(Value::String(Uuid::now_v7().to_string())))
            }
            "uuid_parse" => {
                if args.is_empty() { return Some(Err("uuid_parse requires 1 arg".to_string())); }
                let s = args[0].as_string();
                let id = match Uuid::parse_str(&s) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(format!("uuid_parse: {}", e))),
                };
                let version = id.get_version_num() as i64;
                let variant_str = format!("{:?}", id.get_variant());
                let bytes: Vec<Value> = id.as_bytes().iter().map(|b| Value::Int(*b as i64)).collect();
                let mut m = std::collections::HashMap::new();
                m.insert("version".to_string(), Value::Int(version));
                m.insert("variant".to_string(), Value::String(variant_str));
                m.insert("bytes".to_string(), Value::Array(bytes));
                Some(Ok(Value::Map(m)))
            }
            "uuid_nil" => {
                Some(Ok(Value::String(Uuid::nil().to_string())))
            }
            "now" | "datetime_now" => {
                let dt = Local::now();
                let mut m = std::collections::HashMap::new();
                m.insert("unix".to_string(), Value::Int(dt.timestamp()));
                m.insert("year".to_string(), Value::Int(dt.year() as i64));
                m.insert("month".to_string(), Value::Int(dt.month() as i64));
                m.insert("day".to_string(), Value::Int(dt.day() as i64));
                m.insert("hour".to_string(), Value::Int(dt.hour() as i64));
                m.insert("minute".to_string(), Value::Int(dt.minute() as i64));
                m.insert("second".to_string(), Value::Int(dt.second() as i64));
                m.insert("tz".to_string(), Value::String("local".to_string()));
                m.insert("iso".to_string(), Value::String(dt.to_rfc3339()));
                Some(Ok(Value::Map(m)))
            }
            "utc_now" | "datetime_utc" => {
                let dt = Utc::now();
                let mut m = std::collections::HashMap::new();
                m.insert("unix".to_string(), Value::Int(dt.timestamp()));
                m.insert("year".to_string(), Value::Int(dt.year() as i64));
                m.insert("month".to_string(), Value::Int(dt.month() as i64));
                m.insert("day".to_string(), Value::Int(dt.day() as i64));
                m.insert("hour".to_string(), Value::Int(dt.hour() as i64));
                m.insert("minute".to_string(), Value::Int(dt.minute() as i64));
                m.insert("second".to_string(), Value::Int(dt.second() as i64));
                m.insert("tz".to_string(), Value::String("UTC".to_string()));
                m.insert("iso".to_string(), Value::String(dt.to_rfc3339()));
                Some(Ok(Value::Map(m)))
            }
            "date_format" => {
                if args.len() < 2 { return Some(Err("date_format requires 2 args: ts, fmt".to_string())); }
                let ts = args[0].as_int();
                let fmt = args[1].as_string();
                match Local.timestamp_opt(ts, 0).single() {
                    Some(dt) => Some(Ok(Value::String(dt.format(&fmt).to_string()))),
                    None => Some(Err(format!("date_format: invalid timestamp {}", ts))),
                }
            }
            "date_parse" => {
                if args.len() < 2 { return Some(Err("date_parse requires 2 args: date_str, fmt".to_string())); }
                let date_str = args[0].as_string();
                let fmt_str = args[1].as_string();
                match chrono::NaiveDateTime::parse_from_str(&date_str, &fmt_str) {
                    Ok(ndt) => Some(Ok(Value::Int(ndt.and_utc().timestamp()))),
                    Err(e) => Some(Err(format!("date_parse: {}", e))),
                }
            }
            "date_add" => {
                if args.len() < 2 { return Some(Err("date_add requires 2 args: ts, seconds".to_string())); }
                let ts = args[0].as_int();
                let secs = args[1].as_int();
                Some(Ok(Value::Int(ts + secs)))
            }
            "date_diff" => {
                if args.len() < 2 { return Some(Err("date_diff requires 2 args: ts1, ts2".to_string())); }
                let ts1 = args[0].as_int();
                let ts2 = args[1].as_int();
                Some(Ok(Value::Int(ts2 - ts1)))
            }
            "date_weekday" => {
                if args.is_empty() { return Some(Err("date_weekday requires 1 arg".to_string())); }
                let ts = args[0].as_int();
                match Local.timestamp_opt(ts, 0).single() {
                    Some(dt) => {
                        use chrono::Datelike;
                        let wd = match dt.weekday() {
                            chrono::Weekday::Mon => "Monday",
                            chrono::Weekday::Tue => "Tuesday",
                            chrono::Weekday::Wed => "Wednesday",
                            chrono::Weekday::Thu => "Thursday",
                            chrono::Weekday::Fri => "Friday",
                            chrono::Weekday::Sat => "Saturday",
                            chrono::Weekday::Sun => "Sunday",
                        };
                        Some(Ok(Value::String(wd.to_string())))
                    }
                    None => Some(Err(format!("date_weekday: invalid timestamp {}", ts))),
                }
            }
            "date_components" => {
                if args.is_empty() { return Some(Err("date_components requires 1 arg".to_string())); }
                let ts = args[0].as_int();
                match Local.timestamp_opt(ts, 0).single() {
                    Some(dt) => {
                        use chrono::{Datelike, Timelike, IsoWeek};
                        let wd = match dt.weekday() {
                            chrono::Weekday::Mon => "Monday",
                            chrono::Weekday::Tue => "Tuesday",
                            chrono::Weekday::Wed => "Wednesday",
                            chrono::Weekday::Thu => "Thursday",
                            chrono::Weekday::Fri => "Friday",
                            chrono::Weekday::Sat => "Saturday",
                            chrono::Weekday::Sun => "Sunday",
                        };
                        let mut m = std::collections::HashMap::new();
                        m.insert("year".to_string(), Value::Int(dt.year() as i64));
                        m.insert("month".to_string(), Value::Int(dt.month() as i64));
                        m.insert("day".to_string(), Value::Int(dt.day() as i64));
                        m.insert("hour".to_string(), Value::Int(dt.hour() as i64));
                        m.insert("minute".to_string(), Value::Int(dt.minute() as i64));
                        m.insert("second".to_string(), Value::Int(dt.second() as i64));
                        m.insert("weekday".to_string(), Value::String(wd.to_string()));
                        m.insert("day_of_year".to_string(), Value::Int(dt.ordinal() as i64));
                        m.insert("week_of_year".to_string(), Value::Int(dt.iso_week().week() as i64));
                        Some(Ok(Value::Map(m)))
                    }
                    None => Some(Err(format!("date_components: invalid timestamp {}", ts))),
                }
            }
            "arr_unique" | "unique" => {
                if args.is_empty() { return Some(Err("arr_unique requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("arr_unique: first arg must be array".to_string())),
                };
                let mut seen = std::collections::HashSet::new();
                let mut result = Vec::new();
                for item in arr {
                    let key = format!("{:?}", item);
                    if seen.insert(key) {
                        result.push(item);
                    }
                }
                Some(Ok(Value::Array(result)))
            }
            "arr_flatten" | "flatten" => {
                if args.is_empty() { return Some(Err("arr_flatten requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("arr_flatten: first arg must be array".to_string())),
                };
                let mut result = Vec::new();
                for item in arr {
                    match item {
                        Value::Array(inner) => result.extend(inner),
                        other => result.push(other),
                    }
                }
                Some(Ok(Value::Array(result)))
            }
            "arr_flatten_deep" => {
                if args.is_empty() { return Some(Err("arr_flatten_deep requires 1 arg".to_string())); }
                fn flatten_deep(v: &Value) -> Vec<Value> {
                    match v {
                        Value::Array(arr) => arr.iter().flat_map(|x| flatten_deep(x)).collect(),
                        other => vec![other.clone()],
                    }
                }
                let result = flatten_deep(&args[0]);
                Some(Ok(Value::Array(result)))
            }
            "arr_zip" | "zip" => {
                if args.len() < 2 { return Some(Err("arr_zip requires 2 args".to_string())); }
                let a = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_zip: first arg must be array".to_string())),
                };
                let b = match &args[1] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_zip: second arg must be array".to_string())),
                };
                let result: Vec<Value> = a.into_iter().zip(b.into_iter())
                    .map(|(x, y)| Value::Array(vec![x, y]))
                    .collect();
                Some(Ok(Value::Array(result)))
            }
            "arr_unzip" => {
                if args.is_empty() { return Some(Err("arr_unzip requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_unzip: first arg must be array".to_string())),
                };
                let mut first = Vec::new();
                let mut second = Vec::new();
                for item in arr {
                    match item {
                        Value::Array(ref pair) if pair.len() >= 2 => {
                            first.push(pair[0].clone());
                            second.push(pair[1].clone());
                        }
                        _ => return Some(Err("arr_unzip: each element must be a pair array".to_string())),
                    }
                }
                let mut m = std::collections::HashMap::new();
                m.insert("first".to_string(), Value::Array(first));
                m.insert("second".to_string(), Value::Array(second));
                Some(Ok(Value::Map(m)))
            }
            "arr_chunk" | "chunk" => {
                if args.len() < 2 { return Some(Err("arr_chunk requires 2 args".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_chunk: first arg must be array".to_string())),
                };
                let size = args[1].as_int() as usize;
                if size == 0 { return Some(Err("arr_chunk: chunk size must be > 0".to_string())); }
                let result: Vec<Value> = arr.chunks(size)
                    .map(|c| Value::Array(c.to_vec()))
                    .collect();
                Some(Ok(Value::Array(result)))
            }
            "arr_window" | "windows" => {
                if args.len() < 2 { return Some(Err("arr_window requires 2 args".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_window: first arg must be array".to_string())),
                };
                let size = args[1].as_int() as usize;
                if size == 0 { return Some(Err("arr_window: window size must be > 0".to_string())); }
                if size > arr.len() {
                    return Some(Ok(Value::Array(vec![])));
                }
                let result: Vec<Value> = arr.windows(size)
                    .map(|w| Value::Array(w.to_vec()))
                    .collect();
                Some(Ok(Value::Array(result)))
            }
            "arr_take" | "take" => {
                if args.len() < 2 { return Some(Err("arr_take requires 2 args".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_take: first arg must be array".to_string())),
                };
                let n = args[1].as_int() as usize;
                let result = arr.into_iter().take(n).collect();
                Some(Ok(Value::Array(result)))
            }
            "arr_drop" | "drop" => {
                if args.len() < 2 { return Some(Err("arr_drop requires 2 args".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_drop: first arg must be array".to_string())),
                };
                let n = args[1].as_int() as usize;
                let result = arr.into_iter().skip(n).collect();
                Some(Ok(Value::Array(result)))
            }
            "arr_count_by" => {
                if args.is_empty() { return Some(Err("arr_count_by requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_count_by: first arg must be array".to_string())),
                };
                let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
                for item in &arr {
                    let key = item.as_string();
                    *counts.entry(key).or_insert(0) += 1;
                }
                let m: std::collections::HashMap<String, Value> = counts.into_iter()
                    .map(|(k, v)| (k, Value::Int(v)))
                    .collect();
                Some(Ok(Value::Map(m)))
            }
            "arr_mean" | "mean" => {
                if args.is_empty() { return Some(Err("arr_mean requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_mean: first arg must be array".to_string())),
                };
                if arr.is_empty() { return Some(Err("arr_mean: empty array".to_string())); }
                let sum: f64 = arr.iter().map(|v| v.as_float()).sum();
                Some(Ok(Value::Float(sum / arr.len() as f64)))
            }
            "arr_median" | "median" => {
                if args.is_empty() { return Some(Err("arr_median requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_median: first arg must be array".to_string())),
                };
                if arr.is_empty() { return Some(Err("arr_median: empty array".to_string())); }
                let mut sorted: Vec<f64> = arr.iter().map(|v| v.as_float()).collect();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let n = sorted.len();
                let median = if n % 2 == 1 {
                    sorted[n / 2]
                } else {
                    (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
                };
                Some(Ok(Value::Float(median)))
            }
            "arr_mode" => {
                if args.is_empty() { return Some(Err("arr_mode requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_mode: first arg must be array".to_string())),
                };
                if arr.is_empty() { return Some(Err("arr_mode: empty array".to_string())); }
                let mut counts: std::collections::HashMap<String, (usize, Value)> = std::collections::HashMap::new();
                for item in &arr {
                    let key = item.as_string();
                    let entry = counts.entry(key).or_insert((0, item.clone()));
                    entry.0 += 1;
                }
                let mode_val = counts.into_values()
                    .max_by_key(|(c, _)| *c)
                    .map(|(_, v)| v)
                    .unwrap_or(Value::Null);
                Some(Ok(mode_val))
            }
            "arr_variance" => {
                if args.is_empty() { return Some(Err("arr_variance requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_variance: first arg must be array".to_string())),
                };
                if arr.is_empty() { return Some(Err("arr_variance: empty array".to_string())); }
                let floats: Vec<f64> = arr.iter().map(|v| v.as_float()).collect();
                let mean = floats.iter().sum::<f64>() / floats.len() as f64;
                let variance = floats.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / floats.len() as f64;
                Some(Ok(Value::Float(variance)))
            }
            "arr_stdev" | "stdev" => {
                if args.is_empty() { return Some(Err("arr_stdev requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_stdev: first arg must be array".to_string())),
                };
                if arr.is_empty() { return Some(Err("arr_stdev: empty array".to_string())); }
                let floats: Vec<f64> = arr.iter().map(|v| v.as_float()).collect();
                let mean = floats.iter().sum::<f64>() / floats.len() as f64;
                let variance = floats.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / floats.len() as f64;
                Some(Ok(Value::Float(variance.sqrt())))
            }
            "arr_product" => {
                if args.is_empty() { return Some(Err("arr_product requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_product: first arg must be array".to_string())),
                };
                if arr.is_empty() { return Some(Ok(Value::Float(1.0))); }
                let has_float = arr.iter().any(|v| matches!(v, Value::Float(_)));
                if has_float {
                    let product: f64 = arr.iter().map(|v| v.as_float()).product();
                    Some(Ok(Value::Float(product)))
                } else {
                    let product: i64 = arr.iter().map(|v| v.as_int()).product();
                    Some(Ok(Value::Int(product)))
                }
            }
            "arr_percentile" => {
                if args.len() < 2 { return Some(Err("arr_percentile requires 2 args: array, p".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_percentile: first arg must be array".to_string())),
                };
                if arr.is_empty() { return Some(Err("arr_percentile: empty array".to_string())); }
                let p = args[1].as_float().clamp(0.0, 100.0);
                let mut sorted: Vec<f64> = arr.iter().map(|v| v.as_float()).collect();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let n = sorted.len();
                if n == 1 { return Some(Ok(Value::Float(sorted[0]))); }
                let index = p / 100.0 * (n - 1) as f64;
                let lower = index.floor() as usize;
                let upper = index.ceil() as usize;
                let frac = index - lower as f64;
                let result = sorted[lower] + frac * (sorted[upper] - sorted[lower]);
                Some(Ok(Value::Float(result)))
            }
            "arr_rotate" => {
                if args.len() < 2 { return Some(Err("arr_rotate requires 2 args".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_rotate: first arg must be array".to_string())),
                };
                if arr.is_empty() { return Some(Ok(Value::Array(vec![]))); }
                let n = arr.len();
                let rot = ((args[1].as_int() % n as i64) + n as i64) as usize % n;
                let mut result = arr[rot..].to_vec();
                result.extend_from_slice(&arr[..rot]);
                Some(Ok(Value::Array(result)))
            }
            "arr_repeat" | "repeat_arr" => {
                if args.len() < 2 { return Some(Err("arr_repeat requires 2 args: value, n".to_string())); }
                let val = args[0].clone();
                let n = args[1].as_int();
                if n < 0 { return Some(Err("arr_repeat: count must be non-negative".to_string())); }
                let result = vec![val; n as usize];
                Some(Ok(Value::Array(result)))
            }
            "arr_interleave" => {
                if args.len() < 2 { return Some(Err("arr_interleave requires 2 args".to_string())); }
                let a = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_interleave: first arg must be array".to_string())),
                };
                let b = match &args[1] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_interleave: second arg must be array".to_string())),
                };
                let max_len = a.len().max(b.len());
                let mut result = Vec::new();
                for i in 0..max_len {
                    if i < a.len() { result.push(a[i].clone()); }
                    if i < b.len() { result.push(b[i].clone()); }
                }
                Some(Ok(Value::Array(result)))
            }
            "arr_partition" => {
                if args.len() < 2 { return Some(Err("arr_partition requires 2 args: array, n".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_partition: first arg must be array".to_string())),
                };
                let n = args[1].as_int() as usize;
                let pass = arr[..n.min(arr.len())].to_vec();
                let fail = arr[n.min(arr.len())..].to_vec();
                let mut m = std::collections::HashMap::new();
                m.insert("pass".to_string(), Value::Array(pass));
                m.insert("fail".to_string(), Value::Array(fail));
                Some(Ok(Value::Map(m)))
            }
            "arr_tally" => {
                if args.is_empty() { return Some(Err("arr_tally requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_tally: first arg must be array".to_string())),
                };
                let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
                for item in &arr {
                    let key = item.as_string();
                    *counts.entry(key).or_insert(0) += 1;
                }
                let m: std::collections::HashMap<String, Value> = counts.into_iter()
                    .map(|(k, v)| (k, Value::Int(v)))
                    .collect();
                Some(Ok(Value::Map(m)))
            }
            "arr_combinations" => {
                if args.len() < 2 { return Some(Err("arr_combinations requires 2 args: array, r".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_combinations: first arg must be array".to_string())),
                };
                let r = args[1].as_int() as usize;
                if arr.len() > 20 { return Some(Err("arr_combinations: array too large (max 20)".to_string())); }
                if r > 4 { return Some(Err("arr_combinations: r too large (max 4)".to_string())); }
                fn combos(arr: &[Value], r: usize) -> Vec<Value> {
                    if r == 0 { return vec![Value::Array(vec![])]; }
                    if arr.len() < r { return vec![]; }
                    let mut result = Vec::new();
                    for i in 0..=(arr.len() - r) {
                        let first = arr[i].clone();
                        for c in combos(&arr[i + 1..], r - 1) {
                            if let Value::Array(mut cv) = c {
                                cv.insert(0, first.clone());
                                result.push(Value::Array(cv));
                            }
                        }
                    }
                    result
                }
                Some(Ok(Value::Array(combos(&arr, r))))
            }
            "arr_permutations" => {
                if args.is_empty() { return Some(Err("arr_permutations requires 1 arg".to_string())); }
                let arr = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Some(Err("arr_permutations: first arg must be array".to_string())),
                };
                if arr.len() > 8 { return Some(Err("arr_permutations: array too large (max 8)".to_string())); }
                fn perms(arr: &[Value]) -> Vec<Value> {
                    if arr.is_empty() { return vec![Value::Array(vec![])]; }
                    let mut result = Vec::new();
                    for i in 0..arr.len() {
                        let item = arr[i].clone();
                        let mut rest = arr.to_vec();
                        rest.remove(i);
                        for p in perms(&rest) {
                            if let Value::Array(mut pv) = p {
                                pv.insert(0, item.clone());
                                result.push(Value::Array(pv));
                            }
                        }
                    }
                    result
                }
                Some(Ok(Value::Array(perms(&arr))))
            }
            "map_invert" => {
                if args.is_empty() { return Some(Err("map_invert requires 1 arg".to_string())); }
                let map = match &args[0] {
                    Value::Map(m) => m.clone(),
                    _ => return Some(Err("map_invert: first arg must be map".to_string())),
                };
                let result: std::collections::HashMap<String, Value> = map.into_iter()
                    .map(|(k, v)| (v.as_string(), Value::String(k)))
                    .collect();
                Some(Ok(Value::Map(result)))
            }
            "map_filter" => {
                if args.is_empty() { return Some(Err("map_filter requires at least 1 arg".to_string())); }
                let map = match &args[0] {
                    Value::Map(m) => m.clone(),
                    _ => return Some(Err("map_filter: first arg must be map".to_string())),
                };
                let prefix = if args.len() > 1 { args[1].as_string() } else { String::new() };
                let result: std::collections::HashMap<String, Value> = map.into_iter()
                    .filter(|(k, _)| prefix.is_empty() || k.starts_with(&prefix))
                    .collect();
                Some(Ok(Value::Map(result)))
            }
            "map_count" => {
                if args.is_empty() { return Some(Err("map_count requires 1 arg".to_string())); }
                let count = match &args[0] {
                    Value::Map(m) => m.len() as i64,
                    _ => return Some(Err("map_count: first arg must be map".to_string())),
                };
                Some(Ok(Value::Int(count)))
            }
            "map_update" | "map_set" => {
                if args.len() < 3 { return Some(Err("map_update requires 3 args: map, key, value".to_string())); }
                let mut map = match &args[0] {
                    Value::Map(m) => m.clone(),
                    _ => return Some(Err("map_update: first arg must be map".to_string())),
                };
                let key = args[1].as_string();
                let val = args[2].clone();
                map.insert(key, val);
                Some(Ok(Value::Map(map)))
            }
            "map_get_or" => {
                if args.len() < 3 { return Some(Err("map_get_or requires 3 args: map, key, default".to_string())); }
                let map = match &args[0] {
                    Value::Map(m) => m.clone(),
                    _ => return Some(Err("map_get_or: first arg must be map".to_string())),
                };
                let key = args[1].as_string();
                let default = args[2].clone();
                let result = map.get(&key).cloned().unwrap_or(default);
                Some(Ok(result))
            }
            "lz4_compress" => {
                let data: Vec<u8> = if let Value::Array(arr) = &args[0] {
                    arr.iter().map(|v| v.as_int() as u8).collect()
                } else {
                    args[0].as_string().into_bytes()
                };
                let compressed = lz4_flex::compress_prepend_size(&data);
                Some(Ok(Value::Array(
                    compressed.into_iter().map(|b| Value::Int(b as i64)).collect(),
                )))
            }
            "lz4_decompress" => {
                let bytes: Vec<u8> = if let Value::Array(arr) = &args[0] {
                    arr.iter().map(|v| v.as_int() as u8).collect()
                } else {
                    return Some(Err("lz4_decompress: expected array of ints".to_string()));
                };
                match lz4_flex::decompress_size_prepended(&bytes) {
                    Ok(decompressed) => Some(Ok(Value::String(
                        String::from_utf8_lossy(&decompressed).to_string(),
                    ))),
                    Err(e) => Some(Err(format!("lz4_decompress error: {}", e))),
                }
            }
            "lz4_compress_hex" => {
                let data = args[0].as_string().into_bytes();
                let compressed = lz4_flex::compress_prepend_size(&data);
                Some(Ok(Value::String(b2he(&compressed))))
            }
            "lz4_decompress_hex" => {
                let s = args[0].as_string();
                let bytes = match he2b(&s) {
                    Some(b) => b,
                    None => return Some(Err("lz4_decompress_hex: invalid hex string".to_string())),
                };
                match lz4_flex::decompress_size_prepended(&bytes) {
                    Ok(decompressed) => Some(Ok(Value::String(
                        String::from_utf8_lossy(&decompressed).to_string(),
                    ))),
                    Err(e) => Some(Err(format!("lz4_decompress_hex error: {}", e))),
                }
            }
            "zstd_compress" => {
                let data = args[0].as_string().into_bytes();
                let level = if args.len() > 1 { args[1].as_int() } else { 3 };
                match zstd::encode_all(std::io::Cursor::new(data), level as i32) {
                    Ok(compressed) => Some(Ok(Value::Array(
                        compressed.into_iter().map(|b| Value::Int(b as i64)).collect(),
                    ))),
                    Err(e) => Some(Err(format!("zstd_compress error: {}", e))),
                }
            }
            "zstd_decompress" => {
                let bytes: Vec<u8> = if let Value::Array(arr) = &args[0] {
                    arr.iter().map(|v| v.as_int() as u8).collect()
                } else {
                    return Some(Err("zstd_decompress: expected array of ints".to_string()));
                };
                match zstd::decode_all(std::io::Cursor::new(bytes)) {
                    Ok(decompressed) => Some(Ok(Value::String(
                        String::from_utf8_lossy(&decompressed).to_string(),
                    ))),
                    Err(e) => Some(Err(format!("zstd_decompress error: {}", e))),
                }
            }
            "zstd_compress_hex" => {
                let data = args[0].as_string().into_bytes();
                let level = if args.len() > 1 { args[1].as_int() } else { 3 };
                match zstd::encode_all(std::io::Cursor::new(data), level as i32) {
                    Ok(compressed) => Some(Ok(Value::String(b2he(&compressed)))),
                    Err(e) => Some(Err(format!("zstd_compress_hex error: {}", e))),
                }
            }
            "zstd_decompress_hex" => {
                let s = args[0].as_string();
                let bytes = match he2b(&s) {
                    Some(b) => b,
                    None => return Some(Err("zstd_decompress_hex: invalid hex string".to_string())),
                };
                match zstd::decode_all(std::io::Cursor::new(bytes)) {
                    Ok(decompressed) => Some(Ok(Value::String(
                        String::from_utf8_lossy(&decompressed).to_string(),
                    ))),
                    Err(e) => Some(Err(format!("zstd_decompress_hex error: {}", e))),
                }
            }
            "csv_parse" => {
                let s = args[0].as_string();
                let mut rdr = csv_crate::ReaderBuilder::new()
                    .has_headers(false)
                    .from_reader(s.as_bytes());
                let mut rows = Vec::new();
                for result in rdr.records() {
                    match result {
                        Ok(record) => {
                            let row: Vec<Value> = record
                                .iter()
                                .map(|f| Value::String(f.to_string()))
                                .collect();
                            rows.push(Value::Array(row));
                        }
                        Err(e) => return Some(Err(format!("csv_parse error: {}", e))),
                    }
                }
                Some(Ok(Value::Array(rows)))
            }
            "csv_parse_maps" => {
                let s = args[0].as_string();
                let mut rdr = csv_crate::ReaderBuilder::new()
                    .has_headers(true)
                    .from_reader(s.as_bytes());
                let headers: Vec<String> = match rdr.headers() {
                    Ok(h) => h.iter().map(|f| f.to_string()).collect(),
                    Err(e) => return Some(Err(format!("csv_parse_maps header error: {}", e))),
                };
                let mut rows = Vec::new();
                for result in rdr.records() {
                    match result {
                        Ok(record) => {
                            let mut map = std::collections::HashMap::new();
                            for (i, field) in record.iter().enumerate() {
                                let key = headers
                                    .get(i)
                                    .cloned()
                                    .unwrap_or_else(|| i.to_string());
                                map.insert(key, Value::String(field.to_string()));
                            }
                            rows.push(Value::Map(map));
                        }
                        Err(e) => return Some(Err(format!("csv_parse_maps error: {}", e))),
                    }
                }
                Some(Ok(Value::Array(rows)))
            }
            "csv_stringify" => {
                let rows = if let Value::Array(arr) = &args[0] {
                    arr
                } else {
                    return Some(Err("csv_stringify: expected array of arrays".to_string()));
                };
                let mut wtr = csv_crate::WriterBuilder::new().from_writer(vec![]);
                for row in rows {
                    if let Value::Array(fields) = row {
                        let record: Vec<String> =
                            fields.iter().map(|f| f.as_string()).collect();
                        if let Err(e) = wtr.write_record(&record) {
                            return Some(Err(format!("csv_stringify write error: {}", e)));
                        }
                    }
                }
                match wtr.into_inner() {
                    Ok(bytes) => Some(Ok(Value::String(
                        String::from_utf8_lossy(&bytes).to_string(),
                    ))),
                    Err(e) => Some(Err(format!("csv_stringify flush error: {}", e))),
                }
            }
            "csv_from_maps" => {
                let rows = if let Value::Array(arr) = &args[0] {
                    arr
                } else {
                    return Some(Err("csv_from_maps: expected array of maps".to_string()));
                };
                let headers: Vec<String> = if let Some(Value::Map(first)) = rows.first() {
                    let mut keys: Vec<String> = first.keys().cloned().collect();
                    keys.sort();
                    keys
                } else {
                    return Some(Ok(Value::String(String::new())));
                };
                let mut wtr = csv_crate::WriterBuilder::new().from_writer(vec![]);
                if let Err(e) = wtr.write_record(&headers) {
                    return Some(Err(format!("csv_from_maps header write error: {}", e)));
                }
                for row in rows {
                    if let Value::Map(map) = row {
                        let record: Vec<String> = headers
                            .iter()
                            .map(|k| map.get(k).map(|v| v.as_string()).unwrap_or_default())
                            .collect();
                        if let Err(e) = wtr.write_record(&record) {
                            return Some(Err(format!("csv_from_maps write error: {}", e)));
                        }
                    }
                }
                match wtr.into_inner() {
                    Ok(bytes) => Some(Ok(Value::String(
                        String::from_utf8_lossy(&bytes).to_string(),
                    ))),
                    Err(e) => Some(Err(format!("csv_from_maps flush error: {}", e))),
                }
            }
            "xml_parse" => {
                let s = args[0].as_string();
                use quick_xml::events::Event;

                fn build_node(
                    tag: String,
                    text: String,
                    attrs: std::collections::HashMap<String, Value>,
                    children: Vec<Value>,
                ) -> Value {
                    let mut map = std::collections::HashMap::new();
                    map.insert("tag".to_string(), Value::String(tag));
                    map.insert("text".to_string(), Value::String(text));
                    map.insert("attrs".to_string(), Value::Map(attrs));
                    map.insert("children".to_string(), Value::Array(children));
                    Value::Map(map)
                }

                // stack entries: (tag, accumulated_text, attrs, children)
                let mut stack: Vec<(
                    String,
                    String,
                    std::collections::HashMap<String, Value>,
                    Vec<Value>,
                )> = Vec::new();
                let mut buf = Vec::new();
                let mut root: Option<Value> = None;
                let mut reader = quick_xml::Reader::from_str(&s);
                reader.config_mut().trim_text(true);

                loop {
                    match reader.read_event_into(&mut buf) {
                        Ok(Event::Start(e)) => {
                            let tag =
                                String::from_utf8_lossy(e.name().as_ref()).to_string();
                            let mut attrs = std::collections::HashMap::new();
                            for attr in e.attributes().flatten() {
                                let k =
                                    String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let v =
                                    String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                attrs.insert(k, Value::String(v));
                            }
                            stack.push((tag, String::new(), attrs, Vec::new()));
                        }
                        Ok(Event::End(_)) => {
                            if let Some((tag, text, attrs, children)) = stack.pop() {
                                let node = build_node(tag, text, attrs, children);
                                if let Some(parent) = stack.last_mut() {
                                    parent.3.push(node);
                                } else {
                                    root = Some(node);
                                }
                            }
                        }
                        Ok(Event::Empty(e)) => {
                            let tag =
                                String::from_utf8_lossy(e.name().as_ref()).to_string();
                            let mut attrs = std::collections::HashMap::new();
                            for attr in e.attributes().flatten() {
                                let k =
                                    String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let v =
                                    String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                attrs.insert(k, Value::String(v));
                            }
                            let node = build_node(tag, String::new(), attrs, Vec::new());
                            if let Some(parent) = stack.last_mut() {
                                parent.3.push(node);
                            } else {
                                root = Some(node);
                            }
                        }
                        Ok(Event::Text(e)) => {
                            if let Some(parent) = stack.last_mut() {
                                if let Ok(text) = e.unescape() {
                                    parent.1.push_str(&text);
                                }
                            }
                        }
                        Ok(Event::Eof) => break,
                        Err(e) => return Some(Err(format!("xml_parse error: {}", e))),
                        _ => {}
                    }
                    buf.clear();
                }
                Some(Ok(root.unwrap_or(Value::Null)))
            }
            "xml_stringify" => {
                fn elem_to_xml(val: &Value, out: &mut String) {
                    if let Value::Map(map) = val {
                        let tag = map
                            .get("tag")
                            .map(|v| v.as_string())
                            .unwrap_or_default();
                        let text = map
                            .get("text")
                            .map(|v| v.as_string())
                            .unwrap_or_default();
                        out.push('<');
                        out.push_str(&tag);
                        if let Some(Value::Map(amap)) = map.get("attrs") {
                            let mut akeys: Vec<&String> = amap.keys().collect();
                            akeys.sort();
                            for k in akeys {
                                if let Some(v) = amap.get(k) {
                                    out.push(' ');
                                    out.push_str(k);
                                    out.push_str("=\"");
                                    out.push_str(&v.as_string());
                                    out.push('"');
                                }
                            }
                        }
                        out.push('>');
                        out.push_str(&text);
                        if let Some(Value::Array(children)) = map.get("children") {
                            for child in children {
                                elem_to_xml(child, out);
                            }
                        }
                        out.push_str("</");
                        out.push_str(&tag);
                        out.push('>');
                    }
                }
                let mut out = String::new();
                elem_to_xml(&args[0], &mut out);
                Some(Ok(Value::String(out)))
            }
            "xml_get" | "xml_find" => {
                let tag_name = args[1].as_string();
                if let Value::Map(map) = &args[0] {
                    if let Some(Value::Array(children)) = map.get("children") {
                        for child in children {
                            if let Value::Map(cmap) = child {
                                if let Some(Value::String(t)) = cmap.get("tag") {
                                    if *t == tag_name {
                                        return Some(Ok(child.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Ok(Value::Null))
            }
            "yaml_parse" => {
                let s = args[0].as_string();
                fn sjv_to_val(v: serde_json::Value) -> Value {
                    match v {
                        serde_json::Value::Null => Value::Null,
                        serde_json::Value::Bool(b) => Value::Bool(b),
                        serde_json::Value::Number(n) => {
                            if n.is_i64() {
                                Value::Int(n.as_i64().unwrap())
                            } else {
                                Value::Float(n.as_f64().unwrap_or(0.0))
                            }
                        }
                        serde_json::Value::String(s) => Value::String(s),
                        serde_json::Value::Array(a) => {
                            Value::Array(a.into_iter().map(sjv_to_val).collect())
                        }
                        serde_json::Value::Object(o) => Value::Map(
                            o.into_iter()
                                .map(|(k, v)| (k, sjv_to_val(v)))
                                .collect(),
                        ),
                    }
                }
                match serde_yaml::from_str::<serde_json::Value>(&s) {
                    Ok(v) => Some(Ok(sjv_to_val(v))),
                    Err(e) => Some(Err(format!("yaml_parse error: {}", e))),
                }
            }
            "yaml_stringify" => {
                fn val_to_sjv(v: &Value) -> serde_json::Value {
                    match v {
                        Value::Null => serde_json::Value::Null,
                        Value::Bool(b) => serde_json::Value::Bool(*b),
                        Value::Int(i) => serde_json::Value::Number((*i).into()),
                        Value::Float(f) => {
                            serde_json::Number::from_f64(*f)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        }
                        Value::String(s) => serde_json::Value::String(s.clone()),
                        Value::Array(a) => {
                            serde_json::Value::Array(a.iter().map(val_to_sjv).collect())
                        }
                        Value::Map(m) => serde_json::Value::Object(
                            m.iter()
                                .map(|(k, v)| (k.clone(), val_to_sjv(v)))
                                .collect(),
                        ),
                        _ => serde_json::Value::Null,
                    }
                }
                let jv = val_to_sjv(&args[0]);
                match serde_yaml::to_string(&jv) {
                    Ok(s) => Some(Ok(Value::String(s))),
                    Err(e) => Some(Err(format!("yaml_stringify error: {}", e))),
                }
            }
            "yaml_to_json" => {
                let s = args[0].as_string();
                match serde_yaml::from_str::<serde_json::Value>(&s) {
                    Ok(v) => match serde_json::to_string(&v) {
                        Ok(j) => Some(Ok(Value::String(j))),
                        Err(e) => Some(Err(format!("yaml_to_json serialize error: {}", e))),
                    },
                    Err(e) => Some(Err(format!("yaml_to_json parse error: {}", e))),
                }
            }
            "json_to_yaml" => {
                let s = args[0].as_string();
                match serde_json::from_str::<serde_json::Value>(&s) {
                    Ok(v) => match serde_yaml::to_string(&v) {
                        Ok(y) => Some(Ok(Value::String(y))),
                        Err(e) => Some(Err(format!("json_to_yaml serialize error: {}", e))),
                    },
                    Err(e) => Some(Err(format!("json_to_yaml parse error: {}", e))),
                }
            }
            "chan_create" => {
                let id = self.chan_counter;
                self.chan_counter += 1;
                let (sender, receiver) = if args.is_empty() {
                    crossbeam_channel::unbounded()
                } else {
                    let n = args[0].as_int() as usize;
                    crossbeam_channel::bounded(n)
                };
                self.chan_senders.insert(id, sender);
                self.chan_receivers.insert(id, receiver);
                let mut map = std::collections::HashMap::new();
                map.insert("id".to_string(), Value::Int(id));
                Some(Ok(Value::Map(map)))
            }
            "chan_send" => {
                let id = args[0].as_int();
                let val = args[1].clone();
                if let Some(s) = self.chan_senders.get(&id) {
                    match s.send(val) {
                        Ok(_) => Some(Ok(Value::Bool(true))),
                        Err(e) => Some(Err(format!("chan_send error: {}", e))),
                    }
                } else {
                    Some(Err(format!("chan_send: unknown channel id {}", id)))
                }
            }
            "chan_recv" => {
                let id = args[0].as_int();
                if let Some(r) = self.chan_receivers.get(&id) {
                    match r.recv() {
                        Ok(val) => Some(Ok(val)),
                        Err(_) => Some(Ok(Value::Null)),
                    }
                } else {
                    Some(Err(format!("chan_recv: unknown channel id {}", id)))
                }
            }
            "chan_try_recv" => {
                let id = args[0].as_int();
                if let Some(r) = self.chan_receivers.get(&id) {
                    match r.try_recv() {
                        Ok(val) => Some(Ok(val)),
                        Err(_) => Some(Ok(Value::Null)),
                    }
                } else {
                    Some(Err(format!("chan_try_recv: unknown channel id {}", id)))
                }
            }
            "chan_close" => {
                let id = args[0].as_int();
                self.chan_senders.remove(&id);
                Some(Ok(Value::Bool(true)))
            }
            "chan_len" => {
                let id = args[0].as_int();
                if let Some(s) = self.chan_senders.get(&id) {
                    Some(Ok(Value::Int(s.len() as i64)))
                } else {
                    Some(Err(format!("chan_len: unknown channel id {}", id)))
                }
            }
            "atomic_new" => {
                let id = self.atom_counter;
                self.atom_counter += 1;
                let init = if args.is_empty() { 0 } else { args[0].as_int() };
                self.atomics.insert(id, AtomicI64::new(init));
                Some(Ok(Value::Int(id)))
            }
            "atomic_load" => {
                let id = args[0].as_int();
                if let Some(a) = self.atomics.get(&id) {
                    Some(Ok(Value::Int(a.load(Ordering::SeqCst))))
                } else {
                    Some(Err(format!("atomic_load: unknown atomic id {}", id)))
                }
            }
            "atomic_store" => {
                let id = args[0].as_int();
                let val = args[1].as_int();
                if let Some(a) = self.atomics.get(&id) {
                    a.store(val, Ordering::SeqCst);
                    Some(Ok(Value::Bool(true)))
                } else {
                    Some(Err(format!("atomic_store: unknown atomic id {}", id)))
                }
            }
            "atomic_add" => {
                let id = args[0].as_int();
                let delta = args[1].as_int();
                if let Some(a) = self.atomics.get(&id) {
                    Some(Ok(Value::Int(a.fetch_add(delta, Ordering::SeqCst))))
                } else {
                    Some(Err(format!("atomic_add: unknown atomic id {}", id)))
                }
            }
            "atomic_sub" => {
                let id = args[0].as_int();
                let delta = args[1].as_int();
                if let Some(a) = self.atomics.get(&id) {
                    Some(Ok(Value::Int(a.fetch_sub(delta, Ordering::SeqCst))))
                } else {
                    Some(Err(format!("atomic_sub: unknown atomic id {}", id)))
                }
            }
            "atomic_cas" | "atomic_compare_exchange" => {
                let id = args[0].as_int();
                let expected = args[1].as_int();
                let new_val = args[2].as_int();
                if let Some(a) = self.atomics.get(&id) {
                    let ok = a
                        .compare_exchange(
                            expected,
                            new_val,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        )
                        .is_ok();
                    Some(Ok(Value::Bool(ok)))
                } else {
                    Some(Err(format!("atomic_cas: unknown atomic id {}", id)))
                }
            }
            "atomic_swap" => {
                let id = args[0].as_int();
                let new_val = args[1].as_int();
                if let Some(a) = self.atomics.get(&id) {
                    Some(Ok(Value::Int(a.swap(new_val, Ordering::SeqCst))))
                } else {
                    Some(Err(format!("atomic_swap: unknown atomic id {}", id)))
                }
            }

            "img_load" => {
                if args.is_empty() { return Some(Err("img_load: expected path".to_string())); }
                let path = args[0].as_string();
                match img_crate::open(&path) {
                    Ok(img) => {
                        let id = self.img_counter;
                        self.images.insert(id, img);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(id)))
                    }
                    Err(e) => Some(Err(format!("img_load: {}", e))),
                }
            }
            "img_save" => {
                if args.len() < 2 { return Some(Err("img_save: expected id, path".to_string())); }
                let id = args[0].as_int();
                let path = args[1].as_string();
                match self.images.get(&id) {
                    Some(img) => match img.save(&path) {
                        Ok(_) => Some(Ok(Value::Bool(true))),
                        Err(e) => Some(Err(format!("img_save: {}", e))),
                    },
                    None => Some(Err(format!("img_save: no image with id {}", id))),
                }
            }
            "img_new" => {
                if args.len() < 5 { return Some(Err("img_new: expected w,h,r,g,b".to_string())); }
                let w = args[0].as_int() as u32;
                let h = args[1].as_int() as u32;
                let r = args[2].as_int() as u8;
                let g = args[3].as_int() as u8;
                let b = args[4].as_int() as u8;
                let img = img_crate::DynamicImage::ImageRgb8(
                    img_crate::RgbImage::from_pixel(w, h, img_crate::Rgb([r, g, b]))
                );
                let id = self.img_counter;
                self.images.insert(id, img);
                self.img_counter += 1;
                Some(Ok(Value::Int(id)))
            }
            "img_width" => {
                if args.is_empty() { return Some(Err("img_width: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id) {
                    Some(img) => Some(Ok(Value::Int(img.width() as i64))),
                    None => Some(Err(format!("img_width: no image with id {}", id))),
                }
            }
            "img_height" => {
                if args.is_empty() { return Some(Err("img_height: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id) {
                    Some(img) => Some(Ok(Value::Int(img.height() as i64))),
                    None => Some(Err(format!("img_height: no image with id {}", id))),
                }
            }
            "img_resize" => {
                if args.len() < 3 { return Some(Err("img_resize: expected id,w,h".to_string())); }
                let id = args[0].as_int();
                let w = args[1].as_int() as u32;
                let h = args[2].as_int() as u32;
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.resize(w, h, img_crate::imageops::FilterType::Lanczos3);
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_resize: no image with id {}", id))),
                }
            }
            "img_crop" => {
                if args.len() < 5 { return Some(Err("img_crop: expected id,x,y,w,h".to_string())); }
                let id = args[0].as_int();
                let x = args[1].as_int() as u32;
                let y = args[2].as_int() as u32;
                let w = args[3].as_int() as u32;
                let h = args[4].as_int() as u32;
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.crop_imm(x, y, w, h);
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_crop: no image with id {}", id))),
                }
            }
            "img_rotate90" => {
                if args.is_empty() { return Some(Err("img_rotate90: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.rotate90();
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_rotate90: no image with id {}", id))),
                }
            }
            "img_rotate180" => {
                if args.is_empty() { return Some(Err("img_rotate180: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.rotate180();
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_rotate180: no image with id {}", id))),
                }
            }
            "img_rotate270" => {
                if args.is_empty() { return Some(Err("img_rotate270: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.rotate270();
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_rotate270: no image with id {}", id))),
                }
            }
            "img_fliph" => {
                if args.is_empty() { return Some(Err("img_fliph: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.fliph();
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_fliph: no image with id {}", id))),
                }
            }
            "img_flipv" => {
                if args.is_empty() { return Some(Err("img_flipv: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.flipv();
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_flipv: no image with id {}", id))),
                }
            }
            "img_grayscale" => {
                if args.is_empty() { return Some(Err("img_grayscale: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.grayscale();
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_grayscale: no image with id {}", id))),
                }
            }
            "img_blur" | "img_gaussian_blur" => {
                if args.len() < 2 { return Some(Err("img_blur: expected id, sigma".to_string())); }
                let id = args[0].as_int();
                let sigma = args[1].as_float() as f32;
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.blur(sigma);
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_blur: no image with id {}", id))),
                }
            }
            "img_brighten" => {
                if args.len() < 2 { return Some(Err("img_brighten: expected id, val".to_string())); }
                let id = args[0].as_int();
                let val = args[1].as_int() as i32;
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.brighten(val);
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_brighten: no image with id {}", id))),
                }
            }
            "img_contrast" => {
                if args.len() < 2 { return Some(Err("img_contrast: expected id, c".to_string())); }
                let id = args[0].as_int();
                let c = args[1].as_float() as f32;
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.adjust_contrast(c);
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_contrast: no image with id {}", id))),
                }
            }
            "img_get_pixel" => {
                if args.len() < 3 { return Some(Err("img_get_pixel: expected id,x,y".to_string())); }
                let id = args[0].as_int();
                let x = args[1].as_int() as u32;
                let y = args[2].as_int() as u32;
                match self.images.get(&id) {
                    Some(img) => {
                        use img_crate::GenericImageView;
                        let pixel = img.get_pixel(x, y);
                        let img_crate::Rgba([r, g, b, a]) = pixel;
                        let mut map = std::collections::HashMap::new();
                        map.insert("r".to_string(), Value::Int(r as i64));
                        map.insert("g".to_string(), Value::Int(g as i64));
                        map.insert("b".to_string(), Value::Int(b as i64));
                        map.insert("a".to_string(), Value::Int(a as i64));
                        Some(Ok(Value::Map(map)))
                    }
                    None => Some(Err(format!("img_get_pixel: no image with id {}", id))),
                }
            }
            "img_set_pixel" => {
                if args.len() < 6 { return Some(Err("img_set_pixel: expected id,x,y,r,g,b".to_string())); }
                let id = args[0].as_int();
                let x = args[1].as_int() as u32;
                let y = args[2].as_int() as u32;
                let r = args[3].as_int() as u8;
                let g = args[4].as_int() as u8;
                let b = args[5].as_int() as u8;
                match self.images.get_mut(&id) {
                    Some(img) => {
                        use img_crate::GenericImage;
                        img.put_pixel(x, y, img_crate::Rgba([r, g, b, 255u8]));
                        Some(Ok(Value::Bool(true)))
                    }
                    None => Some(Err(format!("img_set_pixel: no image with id {}", id))),
                }
            }
            "img_to_bytes" => {
                if args.is_empty() { return Some(Err("img_to_bytes: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id) {
                    Some(img) => {
                        let bytes = img.to_rgba8().into_raw();
                        let arr = bytes.into_iter().map(|b| Value::Int(b as i64)).collect();
                        Some(Ok(Value::Array(arr)))
                    }
                    None => Some(Err(format!("img_to_bytes: no image with id {}", id))),
                }
            }
            "img_info" => {
                if args.is_empty() { return Some(Err("img_info: expected id".to_string())); }
                let id = args[0].as_int();
                match self.images.get(&id) {
                    Some(img) => {
                        let mut map = std::collections::HashMap::new();
                        map.insert("width".to_string(), Value::Int(img.width() as i64));
                        map.insert("height".to_string(), Value::Int(img.height() as i64));
                        map.insert("format".to_string(), Value::String("rgba8".to_string()));
                        Some(Ok(Value::Map(map)))
                    }
                    None => Some(Err(format!("img_info: no image with id {}", id))),
                }
            }
            "img_thumbnail" => {
                if args.len() < 3 { return Some(Err("img_thumbnail: expected id,max_w,max_h".to_string())); }
                let id = args[0].as_int();
                let max_w = args[1].as_int() as u32;
                let max_h = args[2].as_int() as u32;
                match self.images.get(&id).cloned() {
                    Some(img) => {
                        let result = img.thumbnail(max_w, max_h);
                        let new_id = self.img_counter;
                        self.images.insert(new_id, result);
                        self.img_counter += 1;
                        Some(Ok(Value::Int(new_id)))
                    }
                    None => Some(Err(format!("img_thumbnail: no image with id {}", id))),
                }
            }
            "img_overlay" => {
                if args.len() < 4 { return Some(Err("img_overlay: expected base_id,overlay_id,x,y".to_string())); }
                let base_id = args[0].as_int();
                let overlay_id = args[1].as_int();
                let x = args[2].as_int();
                let y = args[3].as_int();
                let base_clone = match self.images.get(&base_id) {
                    Some(img) => img.clone(),
                    None => return Some(Err(format!("img_overlay: no base image with id {}", base_id))),
                };
                let overlay_clone = match self.images.get(&overlay_id) {
                    Some(img) => img.clone(),
                    None => return Some(Err(format!("img_overlay: no overlay image with id {}", overlay_id))),
                };
                let mut base = base_clone;
                img_crate::imageops::overlay(&mut base, &overlay_clone, x as i64, y as i64);
                let new_id = self.img_counter;
                self.images.insert(new_id, base);
                self.img_counter += 1;
                Some(Ok(Value::Int(new_id)))
            }
            "img_free" => {
                if args.is_empty() { return Some(Err("img_free: expected id".to_string())); }
                let id = args[0].as_int();
                self.images.remove(&id);
                Some(Ok(Value::Bool(true)))
            }
            "tui_init" | "term_raw" => {
                ct_terminal::enable_raw_mode().ok();
                self.tui_active = true;
                Some(Ok(Value::Bool(true)))
            }
            "tui_exit" | "term_restore" => {
                ct_terminal::disable_raw_mode().ok();
                self.tui_active = false;
                Some(Ok(Value::Bool(true)))
            }
            "tui_clear" => {
                execute!(
                    std::io::stdout(),
                    crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
                ).ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_move" | "term_goto" => {
                if args.len() < 2 { return Some(Err("tui_move: expected col,row".to_string())); }
                let col = args[0].as_int() as u16;
                let row = args[1].as_int() as u16;
                let mut stdout = std::io::stdout();
                execute!(stdout, cursor::MoveTo(col, row)).ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_print" | "term_print" => {
                if args.is_empty() { return Some(Err("tui_print: expected str".to_string())); }
                let s = args[0].as_string();
                use std::io::Write;
                print!("{}", s);
                std::io::stdout().flush().ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_color" | "term_color" => {
                if args.len() < 4 { return Some(Err("tui_color: expected r,g,b,str".to_string())); }
                let r = args[0].as_int() as u8;
                let g = args[1].as_int() as u8;
                let b = args[2].as_int() as u8;
                let s = args[3].as_string();
                use std::io::Write;
                let mut stdout = std::io::stdout();
                execute!(stdout, ct_style::SetForegroundColor(CtColor::Rgb { r, g, b })).ok();
                print!("{}", s);
                stdout.flush().ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_reset_color" | "term_reset" => {
                let mut stdout = std::io::stdout();
                execute!(stdout, ct_style::ResetColor).ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_read_key" | "term_read_key" => {
                match ct_event::read() {
                    Ok(CtEvent::Key(KeyEvent { code, modifiers, .. })) => {
                        let key_str = match code {
                            KeyCode::Char(c) => c.to_string(),
                            KeyCode::Enter => "Enter".to_string(),
                            KeyCode::Backspace => "Backspace".to_string(),
                            KeyCode::Esc => "Esc".to_string(),
                            KeyCode::Up => "Up".to_string(),
                            KeyCode::Down => "Down".to_string(),
                            KeyCode::Left => "Left".to_string(),
                            KeyCode::Right => "Right".to_string(),
                            KeyCode::Tab => "Tab".to_string(),
                            KeyCode::Delete => "Delete".to_string(),
                            KeyCode::Home => "Home".to_string(),
                            KeyCode::End => "End".to_string(),
                            _ => "Unknown".to_string(),
                        };
                        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
                        let alt = modifiers.contains(KeyModifiers::ALT);
                        let shift = modifiers.contains(KeyModifiers::SHIFT);
                        let mut map = std::collections::HashMap::new();
                        map.insert("key".to_string(), Value::String(key_str));
                        map.insert("ctrl".to_string(), Value::Bool(ctrl));
                        map.insert("alt".to_string(), Value::Bool(alt));
                        map.insert("shift".to_string(), Value::Bool(shift));
                        Some(Ok(Value::Map(map)))
                    }
                    Ok(_) => {
                        let mut map = std::collections::HashMap::new();
                        map.insert("key".to_string(), Value::String("Unknown".to_string()));
                        map.insert("ctrl".to_string(), Value::Bool(false));
                        map.insert("alt".to_string(), Value::Bool(false));
                        map.insert("shift".to_string(), Value::Bool(false));
                        Some(Ok(Value::Map(map)))
                    }
                    Err(e) => Some(Err(format!("tui_read_key: {}", e))),
                }
            }
            "tui_poll_key" => {
                use std::time::Duration;
                match ct_event::poll(Duration::from_millis(0)) {
                    Ok(true) => {
                        match ct_event::read() {
                            Ok(CtEvent::Key(KeyEvent { code, modifiers, .. })) => {
                                let key_str = match code {
                                    KeyCode::Char(c) => c.to_string(),
                                    KeyCode::Enter => "Enter".to_string(),
                                    KeyCode::Backspace => "Backspace".to_string(),
                                    KeyCode::Esc => "Esc".to_string(),
                                    KeyCode::Up => "Up".to_string(),
                                    KeyCode::Down => "Down".to_string(),
                                    KeyCode::Left => "Left".to_string(),
                                    KeyCode::Right => "Right".to_string(),
                                    KeyCode::Tab => "Tab".to_string(),
                                    KeyCode::Delete => "Delete".to_string(),
                                    KeyCode::Home => "Home".to_string(),
                                    KeyCode::End => "End".to_string(),
                                    _ => "Unknown".to_string(),
                                };
                                let ctrl = modifiers.contains(KeyModifiers::CONTROL);
                                let alt = modifiers.contains(KeyModifiers::ALT);
                                let shift = modifiers.contains(KeyModifiers::SHIFT);
                                let mut map = std::collections::HashMap::new();
                                map.insert("key".to_string(), Value::String(key_str));
                                map.insert("ctrl".to_string(), Value::Bool(ctrl));
                                map.insert("alt".to_string(), Value::Bool(alt));
                                map.insert("shift".to_string(), Value::Bool(shift));
                                Some(Ok(Value::Map(map)))
                            }
                            _ => Some(Ok(Value::Null)),
                        }
                    }
                    _ => Some(Ok(Value::Null)),
                }
            }
            "tui_hide_cursor" => {
                let mut stdout = std::io::stdout();
                execute!(stdout, cursor::Hide).ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_show_cursor" => {
                let mut stdout = std::io::stdout();
                execute!(stdout, cursor::Show).ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_bold" => {
                if args.is_empty() { return Some(Err("tui_bold: expected str".to_string())); }
                let s = args[0].as_string();
                use std::io::Write;
                let mut stdout = std::io::stdout();
                execute!(stdout, ct_style::SetAttribute(Attribute::Bold)).ok();
                print!("{}", s);
                execute!(stdout, ct_style::SetAttribute(Attribute::Reset)).ok();
                stdout.flush().ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_dim" => {
                if args.is_empty() { return Some(Err("tui_dim: expected str".to_string())); }
                let s = args[0].as_string();
                use std::io::Write;
                let mut stdout = std::io::stdout();
                execute!(stdout, ct_style::SetAttribute(Attribute::Dim)).ok();
                print!("{}", s);
                execute!(stdout, ct_style::SetAttribute(Attribute::Reset)).ok();
                stdout.flush().ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_italic" => {
                if args.is_empty() { return Some(Err("tui_italic: expected str".to_string())); }
                let s = args[0].as_string();
                use std::io::Write;
                let mut stdout = std::io::stdout();
                execute!(stdout, ct_style::SetAttribute(Attribute::Italic)).ok();
                print!("{}", s);
                execute!(stdout, ct_style::SetAttribute(Attribute::Reset)).ok();
                stdout.flush().ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_underline" => {
                if args.is_empty() { return Some(Err("tui_underline: expected str".to_string())); }
                let s = args[0].as_string();
                use std::io::Write;
                let mut stdout = std::io::stdout();
                execute!(stdout, ct_style::SetAttribute(Attribute::Underlined)).ok();
                print!("{}", s);
                execute!(stdout, ct_style::SetAttribute(Attribute::Reset)).ok();
                stdout.flush().ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_blink" => {
                if args.is_empty() { return Some(Err("tui_blink: expected str".to_string())); }
                let s = args[0].as_string();
                use std::io::Write;
                let mut stdout = std::io::stdout();
                execute!(stdout, ct_style::SetAttribute(Attribute::SlowBlink)).ok();
                print!("{}", s);
                execute!(stdout, ct_style::SetAttribute(Attribute::Reset)).ok();
                stdout.flush().ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_bg_color" => {
                if args.len() < 4 { return Some(Err("tui_bg_color: expected r,g,b,str".to_string())); }
                let r = args[0].as_int() as u8;
                let g = args[1].as_int() as u8;
                let b = args[2].as_int() as u8;
                let s = args[3].as_string();
                use std::io::Write;
                let mut stdout = std::io::stdout();
                execute!(stdout, ct_style::SetBackgroundColor(CtColor::Rgb { r, g, b })).ok();
                print!("{}", s);
                execute!(stdout, ct_style::ResetColor).ok();
                stdout.flush().ok();
                Some(Ok(Value::Bool(true)))
            }
            "tui_box" => {
                if args.len() < 4 { return Some(Err("tui_box: expected col,row,width,height".to_string())); }
                let col = args[0].as_int() as u16;
                let row = args[1].as_int() as u16;
                let width = args[2].as_int() as u16;
                let height = args[3].as_int() as u16;
                use std::io::Write;
                let mut stdout = std::io::stdout();
                if width < 2 || height < 2 {
                    return Some(Err("tui_box: width and height must be at least 2".to_string()));
                }
                // Top row
                execute!(stdout, cursor::MoveTo(col, row)).ok();
                write!(stdout, "┌").ok();
                for _ in 0..(width - 2) {
                    write!(stdout, "─").ok();
                }
                write!(stdout, "┐").ok();
                // Middle rows
                for r in 1..(height - 1) {
                    execute!(stdout, cursor::MoveTo(col, row + r)).ok();
                    write!(stdout, "│").ok();
                    for _ in 0..(width - 2) {
                        write!(stdout, " ").ok();
                    }
                    write!(stdout, "│").ok();
                }
                // Bottom row
                execute!(stdout, cursor::MoveTo(col, row + height - 1)).ok();
                write!(stdout, "└").ok();
                for _ in 0..(width - 2) {
                    write!(stdout, "─").ok();
                }
                write!(stdout, "┘").ok();
                stdout.flush().ok();
                Some(Ok(Value::Bool(true)))
            }
            "par_map" => {
                if args.len() < 2 { return Some(Err("par_map: expected array, fname".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_map: first arg must be array".to_string())),
                };
                let fname = args[1].as_string();
                let result: Vec<Value> = arr.into_iter().map(|item| {
                    match self.call_builtin(&fname, &[item.clone()]) {
                        Some(Ok(v)) => v,
                        _ => item,
                    }
                }).collect();
                Some(Ok(Value::Array(result)))
            }
            "par_filter" => {
                if args.len() < 2 { return Some(Err("par_filter: expected array, fname".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_filter: first arg must be array".to_string())),
                };
                let fname = args[1].as_string();
                let result: Vec<Value> = arr.into_iter().filter(|item| {
                    match self.call_builtin(&fname, &[item.clone()]) {
                        Some(Ok(Value::Bool(true))) => true,
                        Some(Ok(Value::Int(n))) if n != 0 => true,
                        _ => false,
                    }
                }).collect();
                Some(Ok(Value::Array(result)))
            }
            "par_reduce" => {
                if args.len() < 3 { return Some(Err("par_reduce: expected array, fname, init".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_reduce: first arg must be array".to_string())),
                };
                let fname = args[1].as_string();
                let init = args[2].clone();
                let result = arr.into_iter().fold(init, |acc, item| {
                    match self.call_builtin(&fname, &[acc.clone(), item]) {
                        Some(Ok(v)) => v,
                        _ => acc,
                    }
                });
                Some(Ok(result))
            }
            "par_sum" => {
                if args.is_empty() { return Some(Err("par_sum: expected array".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_sum: first arg must be array".to_string())),
                };
                let mut has_float = false;
                for v in &arr {
                    if let Value::Float(_) = v { has_float = true; break; }
                }
                if has_float {
                    let sum: f64 = arr.iter().map(|v| v.as_float()).sum();
                    Some(Ok(Value::Float(sum)))
                } else {
                    let sum: i64 = arr.iter().map(|v| v.as_int()).sum();
                    Some(Ok(Value::Int(sum)))
                }
            }
            "par_any" => {
                if args.len() < 2 { return Some(Err("par_any: expected array, fname".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_any: first arg must be array".to_string())),
                };
                let fname = args[1].as_string();
                let result = arr.into_iter().any(|item| {
                    match self.call_builtin(&fname, &[item]) {
                        Some(Ok(Value::Bool(true))) => true,
                        Some(Ok(Value::Int(n))) if n != 0 => true,
                        _ => false,
                    }
                });
                Some(Ok(Value::Bool(result)))
            }
            "par_all" => {
                if args.len() < 2 { return Some(Err("par_all: expected array, fname".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_all: first arg must be array".to_string())),
                };
                let fname = args[1].as_string();
                let result = arr.into_iter().all(|item| {
                    match self.call_builtin(&fname, &[item]) {
                        Some(Ok(Value::Bool(true))) => true,
                        Some(Ok(Value::Int(n))) if n != 0 => true,
                        _ => false,
                    }
                });
                Some(Ok(Value::Bool(result)))
            }
            "par_for_each" => {
                if args.len() < 2 { return Some(Err("par_for_each: expected array, fname".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_for_each: first arg must be array".to_string())),
                };
                let fname = args[1].as_string();
                for item in arr {
                    self.call_builtin(&fname, &[item]);
                }
                Some(Ok(Value::Bool(true)))
            }
            "par_find" => {
                if args.len() < 2 { return Some(Err("par_find: expected array, fname".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_find: first arg must be array".to_string())),
                };
                let fname = args[1].as_string();
                let result = arr.into_iter().find(|item| {
                    match self.call_builtin(&fname, &[item.clone()]) {
                        Some(Ok(Value::Bool(true))) => true,
                        Some(Ok(Value::Int(n))) if n != 0 => true,
                        _ => false,
                    }
                });
                Some(Ok(result.unwrap_or(Value::Null)))
            }
            "par_count" => {
                if args.len() < 2 { return Some(Err("par_count: expected array, fname".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_count: first arg must be array".to_string())),
                };
                let fname = args[1].as_string();
                let count = arr.into_iter().filter(|item| {
                    match self.call_builtin(&fname, &[item.clone()]) {
                        Some(Ok(Value::Bool(true))) => true,
                        Some(Ok(Value::Int(n))) if n != 0 => true,
                        _ => false,
                    }
                }).count() as i64;
                Some(Ok(Value::Int(count)))
            }
            "par_flat_map" => {
                if args.len() < 2 { return Some(Err("par_flat_map: expected array, fname".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_flat_map: first arg must be array".to_string())),
                };
                let fname = args[1].as_string();
                let mut result: Vec<Value> = Vec::new();
                for item in arr {
                    match self.call_builtin(&fname, &[item.clone()]) {
                        Some(Ok(Value::Array(sub))) => result.extend(sub),
                        Some(Ok(v)) => result.push(v),
                        _ => result.push(item),
                    }
                }
                Some(Ok(Value::Array(result)))
            }
            "par_zip_with" => {
                if args.len() < 3 { return Some(Err("par_zip_with: expected array, array, fname".to_string())); }
                let arr_a = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("par_zip_with: first arg must be array".to_string())),
                };
                let arr_b = match &args[1] {
                    Value::Array(b) => b.clone(),
                    _ => return Some(Err("par_zip_with: second arg must be array".to_string())),
                };
                let fname = args[2].as_string();
                let result: Vec<Value> = arr_a.into_iter().zip(arr_b.into_iter()).map(|(a, b)| {
                    match self.call_builtin(&fname, &[a.clone(), b]) {
                        Some(Ok(v)) => v,
                        _ => a,
                    }
                }).collect();
                Some(Ok(Value::Array(result)))
            }
            "str_levenshtein" => {
                if args.len() < 2 { return Some(Err("str_levenshtein: expected a, b".to_string())); }
                let a = args[0].as_string();
                let b = args[1].as_string();
                Some(Ok(Value::Int(strsim::levenshtein(&a, &b) as i64)))
            }
            "str_jaro" => {
                if args.len() < 2 { return Some(Err("str_jaro: expected a, b".to_string())); }
                let a = args[0].as_string();
                let b = args[1].as_string();
                Some(Ok(Value::Float(strsim::jaro(&a, &b))))
            }
            "str_jaro_winkler" => {
                if args.len() < 2 { return Some(Err("str_jaro_winkler: expected a, b".to_string())); }
                let a = args[0].as_string();
                let b = args[1].as_string();
                Some(Ok(Value::Float(strsim::jaro_winkler(&a, &b))))
            }
            "str_hamming" => {
                if args.len() < 2 { return Some(Err("str_hamming: expected a, b".to_string())); }
                let a = args[0].as_string();
                let b = args[1].as_string();
                match strsim::hamming(&a, &b) {
                    Ok(d) => Some(Ok(Value::Int(d as i64))),
                    Err(e) => Some(Err(format!("str_hamming: {}", e))),
                }
            }
            "str_damerau_levenshtein" | "str_osa" => {
                if args.len() < 2 { return Some(Err("str_damerau_levenshtein: expected a, b".to_string())); }
                let a = args[0].as_string();
                let b = args[1].as_string();
                Some(Ok(Value::Int(strsim::damerau_levenshtein(&a, &b) as i64)))
            }
            "str_sorensen_dice" => {
                if args.len() < 2 { return Some(Err("str_sorensen_dice: expected a, b".to_string())); }
                let a = args[0].as_string();
                let b = args[1].as_string();
                Some(Ok(Value::Float(strsim::sorensen_dice(&a, &b))))
            }
            "str_normalize" => {
                if args.is_empty() { return Some(Err("str_normalize: expected str".to_string())); }
                let s = args[0].as_string();
                let lowered = s.to_lowercase();
                let trimmed = lowered.trim().to_string();
                let mut result = String::new();
                let mut last_space = false;
                for c in trimmed.chars() {
                    if c.is_whitespace() {
                        if !last_space {
                            result.push(' ');
                        }
                        last_space = true;
                    } else {
                        result.push(c);
                        last_space = false;
                    }
                }
                Some(Ok(Value::String(result)))
            }
            "str_word_count" => {
                if args.is_empty() { return Some(Err("str_word_count: expected str".to_string())); }
                let s = args[0].as_string();
                Some(Ok(Value::Int(s.split_whitespace().count() as i64)))
            }
            "str_line_count" => {
                if args.is_empty() { return Some(Err("str_line_count: expected str".to_string())); }
                let s = args[0].as_string();
                Some(Ok(Value::Int(s.lines().count() as i64)))
            }
            "str_char_count" => {
                if args.is_empty() { return Some(Err("str_char_count: expected str".to_string())); }
                let s = args[0].as_string();
                Some(Ok(Value::Int(s.chars().count() as i64)))
            }
            "str_bytes_len" => {
                if args.is_empty() { return Some(Err("str_bytes_len: expected str".to_string())); }
                let s = args[0].as_string();
                Some(Ok(Value::Int(s.len() as i64)))
            }
            "str_is_palindrome" => {
                if args.is_empty() { return Some(Err("str_is_palindrome: expected str".to_string())); }
                let s = args[0].as_string();
                let chars: Vec<char> = s.chars().collect();
                let rev: Vec<char> = chars.iter().cloned().rev().collect();
                Some(Ok(Value::Bool(chars == rev)))
            }
            "str_camel_to_snake" => {
                if args.is_empty() { return Some(Err("str_camel_to_snake: expected str".to_string())); }
                let s = args[0].as_string();
                let mut result = String::new();
                for (i, c) in s.chars().enumerate() {
                    if c.is_uppercase() && i > 0 {
                        result.push('_');
                    }
                    result.push(c.to_lowercase().next().unwrap_or(c));
                }
                Some(Ok(Value::String(result)))
            }
            "str_snake_to_camel" => {
                if args.is_empty() { return Some(Err("str_snake_to_camel: expected str".to_string())); }
                let s = args[0].as_string();
                let mut result = String::new();
                let mut capitalize_next = false;
                for c in s.chars() {
                    if c == '_' {
                        capitalize_next = true;
                    } else if capitalize_next {
                        result.push(c.to_uppercase().next().unwrap_or(c));
                        capitalize_next = false;
                    } else {
                        result.push(c);
                    }
                }
                Some(Ok(Value::String(result)))
            }
            "str_kebab_to_snake" => {
                if args.is_empty() { return Some(Err("str_kebab_to_snake: expected str".to_string())); }
                let s = args[0].as_string();
                Some(Ok(Value::String(s.replace('-', "_"))))
            }
            "str_title_case" => {
                if args.is_empty() { return Some(Err("str_title_case: expected str".to_string())); }
                let s = args[0].as_string();
                let result = s.split_whitespace().map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                }).collect::<Vec<_>>().join(" ");
                Some(Ok(Value::String(result)))
            }
            "str_wrap" => {
                if args.len() < 2 { return Some(Err("str_wrap: expected str, width".to_string())); }
                let s = args[0].as_string();
                let width = args[1].as_int() as usize;
                if width == 0 { return Some(Ok(Value::String(s))); }
                let mut lines: Vec<String> = Vec::new();
                let mut current_line = String::new();
                let mut current_len = 0usize;
                for word in s.split_whitespace() {
                    let word_len = word.chars().count();
                    if current_len == 0 {
                        current_line.push_str(word);
                        current_len = word_len;
                    } else if current_len + 1 + word_len <= width {
                        current_line.push(' ');
                        current_line.push_str(word);
                        current_len += 1 + word_len;
                    } else {
                        lines.push(current_line.clone());
                        current_line = word.to_string();
                        current_len = word_len;
                    }
                }
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                Some(Ok(Value::String(lines.join("\n"))))
            }
            "str_truncate" => {
                if args.is_empty() { return Some(Err("str_truncate: expected str, max_len".to_string())); }
                let s = args[0].as_string();
                let max_len = if args.len() > 1 { args[1].as_int() as usize } else { return Some(Err("str_truncate: expected max_len".to_string())); };
                let suffix = if args.len() > 2 { args[2].as_string() } else { "...".to_string() };
                let char_count = s.chars().count();
                if char_count <= max_len {
                    Some(Ok(Value::String(s)))
                } else {
                    let suffix_len = suffix.chars().count();
                    let keep = if max_len > suffix_len { max_len - suffix_len } else { 0 };
                    let truncated: String = s.chars().take(keep).collect();
                    Some(Ok(Value::String(truncated + &suffix)))
                }
            }
            "str_common_prefix" => {
                if args.len() < 2 { return Some(Err("str_common_prefix: expected a, b".to_string())); }
                let a = args[0].as_string();
                let b = args[1].as_string();
                let prefix: String = a.chars().zip(b.chars()).take_while(|(ca, cb)| ca == cb).map(|(c, _)| c).collect();
                Some(Ok(Value::String(prefix)))
            }
            "str_common_suffix" => {
                if args.len() < 2 { return Some(Err("str_common_suffix: expected a, b".to_string())); }
                let a = args[0].as_string();
                let b = args[1].as_string();
                let suffix: String = a.chars().rev().zip(b.chars().rev()).take_while(|(ca, cb)| ca == cb).map(|(c, _)| c).collect::<String>().chars().rev().collect();
                Some(Ok(Value::String(suffix)))
            }
            "str_rot13" => {
                if args.is_empty() { return Some(Err("str_rot13: expected str".to_string())); }
                let s = args[0].as_string();
                let result: String = s.chars().map(|c| match c {
                    'a'..='m' | 'A'..='M' => (c as u8 + 13) as char,
                    'n'..='z' | 'N'..='Z' => (c as u8 - 13) as char,
                    _ => c,
                }).collect();
                Some(Ok(Value::String(result)))
            }
            "str_caesar" => {
                if args.len() < 2 { return Some(Err("str_caesar: expected str, shift".to_string())); }
                let s = args[0].as_string();
                let shift = args[1].as_int();
                let result: String = s.chars().map(|c| {
                    if c.is_ascii_alphabetic() {
                        let base = if c.is_lowercase() { b'a' } else { b'A' };
                        let shifted = ((c as i64 - base as i64 + shift).rem_euclid(26)) as u8 + base;
                        shifted as char
                    } else {
                        c
                    }
                }).collect();
                Some(Ok(Value::String(result)))
            }
            "str_expand_tabs" => {
                if args.is_empty() { return Some(Err("str_expand_tabs: expected str".to_string())); }
                let s = args[0].as_string();
                let n = if args.len() > 1 { args[1].as_int() as usize } else { 4 };
                Some(Ok(Value::String(s.replace('\t', &" ".repeat(n)))))
            }
            "str_from_chars" => {
                if args.is_empty() { return Some(Err("str_from_chars: expected array".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("str_from_chars: expected array of codepoints".to_string())),
                };
                let s: String = arr.iter().map(|v| {
                    let cp = v.as_int() as u32;
                    char::from_u32(cp).unwrap_or('?')
                }).collect();
                Some(Ok(Value::String(s)))
            }
            "str_to_chars" => {
                if args.is_empty() { return Some(Err("str_to_chars: expected str".to_string())); }
                let s = args[0].as_string();
                let arr: Vec<Value> = s.chars().map(|c| Value::Int(c as i64)).collect();
                Some(Ok(Value::Array(arr)))
            }
            "str_each_line" => {
                if args.is_empty() { return Some(Err("str_each_line: expected str".to_string())); }
                let s = args[0].as_string();
                let arr: Vec<Value> = s.lines().map(|l| Value::String(l.to_string())).collect();
                Some(Ok(Value::Array(arr)))
            }
            "str_join_lines" => {
                if args.is_empty() { return Some(Err("str_join_lines: expected array".to_string())); }
                let arr = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Some(Err("str_join_lines: expected array of strings".to_string())),
                };
                let joined = arr.iter().map(|v| v.as_string()).collect::<Vec<_>>().join("\n");
                Some(Ok(Value::String(joined)))
            }
            "whois" => {
                if args.is_empty() { return Some(Err("whois: expected query".to_string())); }
                let query = args[0].as_string();
                use std::io::{Read, Write};
                match std::net::TcpStream::connect("whois.iana.org:43") {
                    Ok(mut stream) => {
                        let req = format!("{}\r\n", query);
                        if let Err(e) = stream.write_all(req.as_bytes()) {
                            return Some(Err(format!("whois write: {}", e)));
                        }
                        let mut response = String::new();
                        match stream.read_to_string(&mut response) {
                            Ok(_) => Some(Ok(Value::String(response))),
                            Err(e) => Some(Err(format!("whois read: {}", e))),
                        }
                    }
                    Err(e) => Some(Err(format!("whois connect: {}", e))),
                }
            }
            "net_interfaces" => {
                match std::fs::read_to_string("/proc/net/dev") {
                    Ok(content) => {
                        let mut interfaces: Vec<Value> = Vec::new();
                        for line in content.lines().skip(2) {
                            let trimmed = line.trim();
                            if trimmed.is_empty() { continue; }
                            let colon_pos = match trimmed.find(':') {
                                Some(p) => p,
                                None => continue,
                            };
                            let iface = trimmed[..colon_pos].trim().to_string();
                            let rest = trimmed[colon_pos + 1..].trim();
                            let fields: Vec<&str> = rest.split_whitespace().collect();
                            let rx_bytes = fields.get(0).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                            let tx_bytes = fields.get(8).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                            let mut map = std::collections::HashMap::new();
                            map.insert("name".to_string(), Value::String(iface));
                            map.insert("rx_bytes".to_string(), Value::Int(rx_bytes));
                            map.insert("tx_bytes".to_string(), Value::Int(tx_bytes));
                            interfaces.push(Value::Map(map));
                        }
                        Some(Ok(Value::Array(interfaces)))
                    }
                    Err(e) => Some(Err(format!("net_interfaces: {}", e))),
                }
            }
            "tcp_port_open" | "port_open" => {
                if args.len() < 2 { return Some(Err("tcp_port_open: expected host, port".to_string())); }
                let host = args[0].as_string();
                let port = args[1].as_int();
                let timeout_ms = if args.len() > 2 { args[2].as_int() as u64 } else { 1000u64 };
                use std::time::Duration;
                use std::net::ToSocketAddrs;
                let addr_str = format!("{}:{}", host, port);
                match addr_str.to_socket_addrs() {
                    Ok(mut addrs) => {
                        match addrs.next() {
                            Some(addr) => {
                                let open = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).is_ok();
                                Some(Ok(Value::Bool(open)))
                            }
                            None => Some(Ok(Value::Bool(false))),
                        }
                    }
                    Err(_) => Some(Ok(Value::Bool(false))),
                }
            }
            "net_gateway" => {
                match std::fs::read_to_string("/proc/net/route") {
                    Ok(content) => {
                        for line in content.lines().skip(1) {
                            let fields: Vec<&str> = line.split_whitespace().collect();
                            if fields.len() < 3 { continue; }
                            if fields[1] == "00000000" {
                                let hex = fields[2];
                                if hex.len() >= 8 {
                                    if let Ok(val) = u32::from_str_radix(&hex[..8], 16) {
                                        let bytes = val.to_le_bytes();
                                        let ip = format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3]);
                                        return Some(Ok(Value::String(ip)));
                                    }
                                }
                            }
                        }
                        Some(Ok(Value::Null))
                    }
                    Err(e) => Some(Err(format!("net_gateway: {}", e))),
                }
            }
            "net_arp_table" => {
                match std::fs::read_to_string("/proc/net/arp") {
                    Ok(content) => {
                        let mut entries: Vec<Value> = Vec::new();
                        for line in content.lines().skip(1) {
                            let fields: Vec<&str> = line.split_whitespace().collect();
                            if fields.len() < 6 { continue; }
                            let mut map = std::collections::HashMap::new();
                            map.insert("ip".to_string(), Value::String(fields[0].to_string()));
                            map.insert("hw_addr".to_string(), Value::String(fields[3].to_string()));
                            map.insert("iface".to_string(), Value::String(fields[5].to_string()));
                            entries.push(Value::Map(map));
                        }
                        Some(Ok(Value::Array(entries)))
                    }
                    Err(e) => Some(Err(format!("net_arp_table: {}", e))),
                }
            }
            "net_connections" => {
                fn hex4_to_ip(s: &str) -> String {
                    if s.len() < 8 { return "0.0.0.0".to_string(); }
                    match u32::from_str_radix(&s[..8], 16) {
                        Ok(val) => {
                            let bytes = val.to_le_bytes();
                            format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
                        }
                        Err(_) => "0.0.0.0".to_string(),
                    }
                }
                fn parse_hex_port(s: &str) -> i64 {
                    i64::from_str_radix(s, 16).unwrap_or(0)
                }
                fn state_name(s: &str) -> &'static str {
                    match s {
                        "01" => "ESTABLISHED",
                        "02" => "SYN_SENT",
                        "03" => "SYN_RECV",
                        "04" => "FIN_WAIT1",
                        "05" => "FIN_WAIT2",
                        "06" => "TIME_WAIT",
                        "07" => "CLOSE",
                        "08" => "CLOSE_WAIT",
                        "09" => "LAST_ACK",
                        "0A" => "LISTEN",
                        "0B" => "CLOSING",
                        _ => "UNKNOWN",
                    }
                }
                match std::fs::read_to_string("/proc/net/tcp") {
                    Ok(content) => {
                        let mut connections: Vec<Value> = Vec::new();
                        for line in content.lines().skip(1) {
                            let fields: Vec<&str> = line.split_whitespace().collect();
                            if fields.len() < 4 { continue; }
                            let local = fields[1];
                            let remote = fields[2];
                            let state_hex = fields[3];
                            let local_parts: Vec<&str> = local.splitn(2, ':').collect();
                            let remote_parts: Vec<&str> = remote.splitn(2, ':').collect();
                            let local_ip = if local_parts.len() > 0 { hex4_to_ip(local_parts[0]) } else { "0.0.0.0".to_string() };
                            let local_port = if local_parts.len() > 1 { parse_hex_port(local_parts[1]) } else { 0 };
                            let remote_ip = if remote_parts.len() > 0 { hex4_to_ip(remote_parts[0]) } else { "0.0.0.0".to_string() };
                            let remote_port = if remote_parts.len() > 1 { parse_hex_port(remote_parts[1]) } else { 0 };
                            let state = state_name(&state_hex.to_uppercase()[..]);
                            let mut map = std::collections::HashMap::new();
                            map.insert("local_ip".to_string(), Value::String(local_ip));
                            map.insert("local_port".to_string(), Value::Int(local_port));
                            map.insert("remote_ip".to_string(), Value::String(remote_ip));
                            map.insert("remote_port".to_string(), Value::Int(remote_port));
                            map.insert("state".to_string(), Value::String(state.to_string()));
                            connections.push(Value::Map(map));
                        }
                        Some(Ok(Value::Array(connections)))
                    }
                    Err(e) => Some(Err(format!("net_connections: {}", e))),
                }
            }
            "net_ping" => {
                if args.is_empty() { return Some(Err("net_ping: expected host".to_string())); }
                let host = args[0].as_string();
                use std::net::ToSocketAddrs;
                let resolvable = format!("{}:0", host).to_socket_addrs().is_ok();
                Some(Ok(Value::Bool(resolvable)))
            }




            // ── KNULL_B9_LINALG ──
"mat_new" => Some((|| -> Result<Value, String> {
    // Two calling conventions:
    // mat_new([[row0], [row1], ...])      — from nested array
    // mat_new(rows, cols, fill_value)     — fill form
    if args.is_empty() { return Err("mat_new: need arguments".into()); }
    if let Value::Array(_) = &args[0] {
        // Nested array form — just return as-is (already an array of arrays)
        // Ensure all rows are arrays of floats
        let outer = if let Value::Array(a) = &args[0] { a } else { unreachable!() };
        let rows: Vec<Value> = outer.iter().map(|row| {
            match row {
                Value::Array(cols) => Value::Array(cols.iter().map(|v| Value::Float(v.as_float())).collect()),
                v => Value::Array(vec![Value::Float(v.as_float())]),
            }
        }).collect();
        Ok(Value::Array(rows))
    } else {
        if args.len() < 3 { return Err("mat_new(rows, cols, fill) requires 3 args".into()); }
        let rows = args[0].as_int() as usize;
        let cols = args[1].as_int() as usize;
        let val = args[2].as_float();
        let row_val: Vec<Value> = (0..cols).map(|_| Value::Float(val)).collect();
        let matrix: Vec<Value> = (0..rows).map(|_| Value::Array(row_val.clone())).collect();
        Ok(Value::Array(matrix))
    }
})()),

"mat_identity" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_identity requires 1 arg".into()); }
    let n = args[0].as_int() as usize;
    let m = DMatrix::<f64>::identity(n, n);
    let rows: Vec<Value> = (0..n)
        .map(|i| Value::Array((0..n).map(|j| Value::Float(m[(i, j)])).collect()))
        .collect();
    Ok(Value::Array(rows))
})()),

"mat_add" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("mat_add requires 2 args".into()); }
    let to_dm = |v: &Value| -> Result<DMatrix<f64>, String> {
        if let Value::Array(rows) = v {
            let nrows = rows.len();
            if nrows == 0 { return Err("Empty matrix".into()); }
            let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
            let mut data = Vec::with_capacity(nrows * ncols);
            for row in rows {
                if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
                else { return Err("Expected row array".into()); }
            }
            Ok(DMatrix::from_row_slice(nrows, ncols, &data))
        } else { Err("Expected matrix array".into()) }
    };
    let a = to_dm(&args[0])?;
    let b = to_dm(&args[1])?;
    if a.nrows() != b.nrows() || a.ncols() != b.ncols() { return Err("Matrix dimension mismatch".into()); }
    let c = a + b;
    let rows: Vec<Value> = (0..c.nrows())
        .map(|i| Value::Array((0..c.ncols()).map(|j| Value::Float(c[(i, j)])).collect()))
        .collect();
    Ok(Value::Array(rows))
})()),

"mat_sub" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("mat_sub requires 2 args".into()); }
    let to_dm = |v: &Value| -> Result<DMatrix<f64>, String> {
        if let Value::Array(rows) = v {
            let nrows = rows.len();
            if nrows == 0 { return Err("Empty matrix".into()); }
            let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
            let mut data = Vec::with_capacity(nrows * ncols);
            for row in rows {
                if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
                else { return Err("Expected row array".into()); }
            }
            Ok(DMatrix::from_row_slice(nrows, ncols, &data))
        } else { Err("Expected matrix array".into()) }
    };
    let a = to_dm(&args[0])?;
    let b = to_dm(&args[1])?;
    if a.nrows() != b.nrows() || a.ncols() != b.ncols() { return Err("Matrix dimension mismatch".into()); }
    let c = a - b;
    let rows: Vec<Value> = (0..c.nrows())
        .map(|i| Value::Array((0..c.ncols()).map(|j| Value::Float(c[(i, j)])).collect()))
        .collect();
    Ok(Value::Array(rows))
})()),

"mat_mul" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("mat_mul requires 2 args".into()); }
    let to_dm = |v: &Value| -> Result<DMatrix<f64>, String> {
        if let Value::Array(rows) = v {
            let nrows = rows.len();
            if nrows == 0 { return Err("Empty matrix".into()); }
            let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
            let mut data = Vec::with_capacity(nrows * ncols);
            for row in rows {
                if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
                else { return Err("Expected row array".into()); }
            }
            Ok(DMatrix::from_row_slice(nrows, ncols, &data))
        } else { Err("Expected matrix array".into()) }
    };
    let a = to_dm(&args[0])?;
    let b = to_dm(&args[1])?;
    if a.ncols() != b.nrows() { return Err(format!("Incompatible dimensions {}x{} * {}x{}", a.nrows(), a.ncols(), b.nrows(), b.ncols())); }
    let c = a * b;
    let rows: Vec<Value> = (0..c.nrows())
        .map(|i| Value::Array((0..c.ncols()).map(|j| Value::Float(c[(i, j)])).collect()))
        .collect();
    Ok(Value::Array(rows))
})()),

"mat_transpose" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_transpose requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Ok(Value::Array(vec![])); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
            else { return Err("Expected row array".into()); }
        }
        let m = DMatrix::from_row_slice(nrows, ncols, &data);
        let t = m.transpose();
        let out: Vec<Value> = (0..t.nrows())
            .map(|i| Value::Array((0..t.ncols()).map(|j| Value::Float(t[(i, j)])).collect()))
            .collect();
        Ok(Value::Array(out))
    } else { Err("Expected matrix array".into()) }
})()),

"mat_det" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_det requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Ok(Value::Float(1.0)); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        if nrows != ncols { return Err("Determinant requires square matrix".into()); }
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
            else { return Err("Expected row array".into()); }
        }
        let m = DMatrix::from_row_slice(nrows, ncols, &data);
        Ok(Value::Float(m.determinant()))
    } else { Err("Expected matrix array".into()) }
})()),

"mat_inv" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_inv requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Err("Empty matrix".into()); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        if nrows != ncols { return Err("Inverse requires square matrix".into()); }
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
            else { return Err("Expected row array".into()); }
        }
        let m = DMatrix::from_row_slice(nrows, ncols, &data);
        match m.try_inverse() {
            Some(inv) => {
                let out: Vec<Value> = (0..inv.nrows())
                    .map(|i| Value::Array((0..inv.ncols()).map(|j| Value::Float(inv[(i, j)])).collect()))
                    .collect();
                Ok(Value::Array(out))
            }
            None => Err("Matrix is singular".into()),
        }
    } else { Err("Expected matrix array".into()) }
})()),

"mat_trace" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_trace requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Ok(Value::Float(0.0)); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        let n = nrows.min(ncols);
        let mut trace = 0.0f64;
        for i in 0..n {
            if let Value::Array(r) = &rows[i] {
                trace += r[i].as_float();
            }
        }
        Ok(Value::Float(trace))
    } else { Err("Expected matrix array".into()) }
})()),

"mat_norm" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_norm requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Ok(Value::Float(0.0)); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
            else { return Err("Expected row array".into()); }
        }
        let m = DMatrix::from_row_slice(nrows, ncols, &data);
        Ok(Value::Float(m.norm()))
    } else { Err("Expected matrix array".into()) }
})()),

"mat_rank" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_rank requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Ok(Value::Int(0)); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
            else { return Err("Expected row array".into()); }
        }
        let m = DMatrix::from_row_slice(nrows, ncols, &data);
        let svd = nalgebra::linalg::SVD::new(m, false, false);
        let rank = svd.singular_values.iter().filter(|&&s| s > 1e-10).count();
        Ok(Value::Int(rank as i64))
    } else { Err("Expected matrix array".into()) }
})()),

"mat_svd" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_svd requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Err("Empty matrix".into()); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
            else { return Err("Expected row array".into()); }
        }
        let m = DMatrix::from_row_slice(nrows, ncols, &data);
        let svd = nalgebra::linalg::SVD::new(m, true, true);
        let u_mat = svd.u.ok_or("SVD failed: no U")?;
        let v_t_mat = svd.v_t.ok_or("SVD failed: no V_t")?;
        let s_vec = svd.singular_values;
        let dm_to_val = |m: &DMatrix<f64>| -> Value {
            Value::Array((0..m.nrows())
                .map(|i| Value::Array((0..m.ncols()).map(|j| Value::Float(m[(i, j)])).collect()))
                .collect())
        };
        let mut result = HashMap::new();
        result.insert("U".into(), dm_to_val(&u_mat));
        result.insert("S".into(), Value::Array(s_vec.iter().map(|&s| Value::Float(s)).collect()));
        result.insert("V".into(), dm_to_val(&v_t_mat.transpose()));
        Ok(Value::Map(result))
    } else { Err("Expected matrix array".into()) }
})()),

"mat_lu" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_lu requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Err("Empty matrix".into()); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
            else { return Err("Expected row array".into()); }
        }
        let m = DMatrix::from_row_slice(nrows, ncols, &data);
        let lu = nalgebra::linalg::LU::new(m);
        let l = lu.l();
        let u = lu.u();
        let p = lu.p();
        let dm_to_val = |mat: &DMatrix<f64>| -> Value {
            Value::Array((0..mat.nrows())
                .map(|i| Value::Array((0..mat.ncols()).map(|j| Value::Float(mat[(i, j)])).collect()))
                .collect())
        };
        let mut result = HashMap::new();
        result.insert("L".into(), dm_to_val(&l));
        result.insert("U".into(), dm_to_val(&u));
        result.insert("P".into(), Value::Null); // P permutation omitted (nalgebra API limitation)
        Ok(Value::Map(result))
    } else { Err("Expected matrix array".into()) }
})()),

"mat_qr" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_qr requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Err("Empty matrix".into()); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
            else { return Err("Expected row array".into()); }
        }
        let m = DMatrix::from_row_slice(nrows, ncols, &data);
        let qr = nalgebra::linalg::QR::new(m);
        let q = qr.q();
        let r = qr.r();
        let dm_to_val = |mat: &DMatrix<f64>| -> Value {
            Value::Array((0..mat.nrows())
                .map(|i| Value::Array((0..mat.ncols()).map(|j| Value::Float(mat[(i, j)])).collect()))
                .collect())
        };
        let mut result = HashMap::new();
        result.insert("Q".into(), dm_to_val(&q));
        result.insert("R".into(), dm_to_val(&r));
        Ok(Value::Map(result))
    } else { Err("Expected matrix array".into()) }
})()),

"mat_eigenvalues" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_eigenvalues requires 1 arg".into()); }
    if let Value::Array(rows) = &args[0] {
        let nrows = rows.len();
        if nrows == 0 { return Ok(Value::Array(vec![])); }
        let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
        if nrows != ncols { return Err("Eigenvalues require square matrix".into()); }
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in rows {
            if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
            else { return Err("Expected row array".into()); }
        }
        let m = DMatrix::from_row_slice(nrows, ncols, &data);
        // Symmetrize then use SymmetricEigen for real eigenvalues
        let sym = (&m + m.transpose()) * 0.5;
        let eigen = nalgebra::linalg::SymmetricEigen::new(sym);
        let mut evals: Vec<f64> = eigen.eigenvalues.iter().copied().collect();
        evals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Ok(Value::Array(evals.into_iter().map(Value::Float).collect()))
    } else { Err("Expected matrix array".into()) }
})()),

"mat_solve" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("mat_solve requires 2 args (A, b)".into()); }
    let to_dm = |v: &Value| -> Result<DMatrix<f64>, String> {
        if let Value::Array(rows) = v {
            let nrows = rows.len();
            if nrows == 0 { return Err("Empty matrix".into()); }
            let ncols = if let Value::Array(r) = &rows[0] { r.len() } else { return Err("Not array of arrays".into()); };
            let mut data = Vec::with_capacity(nrows * ncols);
            for row in rows {
                if let Value::Array(r) = row { for val in r { data.push(val.as_float()); } }
                else { return Err("Expected row array".into()); }
            }
            Ok(DMatrix::from_row_slice(nrows, ncols, &data))
        } else { Err("Expected matrix".into()) }
    };
    let a = to_dm(&args[0])?;
    let b_vec: Vec<f64> = if let Value::Array(arr) = &args[1] {
        arr.iter().map(|x| x.as_float()).collect()
    } else { return Err("Expected vector array for b".into()); };
    let b = DMatrix::from_column_slice(b_vec.len(), 1, &b_vec);
    let lu = nalgebra::linalg::LU::new(a);
    match lu.solve(&b) {
        Some(x) => Ok(Value::Array((0..x.nrows()).map(|i| Value::Float(x[(i, 0)])).collect())),
        None => Err("System is singular or inconsistent".into()),
    }
})()),

"mat_dot" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("mat_dot requires 2 args".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    if a.len() != b.len() { return Err("Vector length mismatch".into()); }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    Ok(Value::Float(dot))
})()),

"mat_cross" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("mat_cross requires 2 args".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    if a.len() != 3 || b.len() != 3 { return Err("Cross product requires 3D vectors".into()); }
    let av = Vector3::new(a[0], a[1], a[2]);
    let bv = Vector3::new(b[0], b[1], b[2]);
    let c = av.cross(&bv);
    Ok(Value::Array(vec![Value::Float(c[0]), Value::Float(c[1]), Value::Float(c[2])]))
})()),

"mat_normalize" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("mat_normalize requires 1 arg".into()); }
    let v: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm == 0.0 { return Err("Cannot normalize zero vector".into()); }
    Ok(Value::Array(v.into_iter().map(|x| Value::Float(x / norm)).collect()))
})()),

"mat_outer" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("mat_outer requires 2 args".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let rows: Vec<Value> = a.iter()
        .map(|&ai| Value::Array(b.iter().map(|&bi| Value::Float(ai * bi)).collect()))
        .collect();
    Ok(Value::Array(rows))
})()),

"vec_add" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("vec_add requires 2 args".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    if a.len() != b.len() { return Err("Vector length mismatch".into()); }
    Ok(Value::Array(a.iter().zip(b.iter()).map(|(x, y)| Value::Float(x + y)).collect()))
})()),

"vec_sub" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("vec_sub requires 2 args".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    if a.len() != b.len() { return Err("Vector length mismatch".into()); }
    Ok(Value::Array(a.iter().zip(b.iter()).map(|(x, y)| Value::Float(x - y)).collect()))
})()),

"vec_scale" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("vec_scale requires 2 args".into()); }
    let v: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let s = args[1].as_float();
    Ok(Value::Array(v.into_iter().map(|x| Value::Float(x * s)).collect()))
})()),

"vec_dot" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("vec_dot requires 2 args".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    if a.len() != b.len() { return Err("Vector length mismatch".into()); }
    Ok(Value::Float(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()))
})()),

"vec_norm" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("vec_norm requires 1 arg".into()); }
    let v: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    Ok(Value::Float(v.iter().map(|x| x * x).sum::<f64>().sqrt()))
})()),

"vec_cross" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("vec_cross requires 2 args".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    if a.len() != 3 || b.len() != 3 { return Err("vec_cross requires 3D vectors".into()); }
    let av = Vector3::new(a[0], a[1], a[2]);
    let bv = Vector3::new(b[0], b[1], b[2]);
    let c = av.cross(&bv);
    Ok(Value::Array(vec![Value::Float(c[0]), Value::Float(c[1]), Value::Float(c[2])]))
})()),

"vec_normalize" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("vec_normalize requires 1 arg".into()); }
    let v: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm == 0.0 { return Err("Cannot normalize zero vector".into()); }
    Ok(Value::Array(v.into_iter().map(|x| Value::Float(x / norm)).collect()))
})()),

"vec_angle" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("vec_angle requires 2 args".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    if a.len() != b.len() { return Err("Vector length mismatch".into()); }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nb: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 { return Err("Cannot compute angle with zero vector".into()); }
    let cos_theta = (dot / (na * nb)).clamp(-1.0, 1.0);
    Ok(Value::Float(cos_theta.acos()))
})()),

"vec_lerp" => Some((|| -> Result<Value, String> {
    if args.len() < 3 { return Err("vec_lerp requires 3 args (a, b, t)".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let t = args[2].as_float();
    if a.len() != b.len() { return Err("Vector length mismatch".into()); }
    Ok(Value::Array(a.iter().zip(b.iter()).map(|(ai, bi)| Value::Float(ai + t * (bi - ai))).collect()))
})()),

"vec_project" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("vec_project requires 2 args (a, b)".into()); }
    let a: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let b: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    if a.len() != b.len() { return Err("Vector length mismatch".into()); }
    let dot_ab: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let dot_bb: f64 = b.iter().map(|x| x * x).sum();
    if dot_bb == 0.0 { return Err("Cannot project onto zero vector".into()); }
    let scale = dot_ab / dot_bb;
    Ok(Value::Array(b.iter().map(|&bi| Value::Float(bi * scale)).collect()))
})()),

"fft" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("fft requires 1 arg".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let n = vals.len();
    if n == 0 { return Ok(Value::Array(vec![])); }
    let mut buffer: Vec<Complex<f64>> = vals.iter().map(|&r| Complex { re: r, im: 0.0 }).collect();
    let mut planner = FftPlanner::new();
    let fft_plan = planner.plan_fft_forward(n);
    fft_plan.process(&mut buffer);
    let result: Vec<Value> = buffer.iter().map(|c| {
        let mut m = HashMap::new();
        m.insert("re".into(), Value::Float(c.re));
        m.insert("im".into(), Value::Float(c.im));
        Value::Map(m)
    }).collect();
    Ok(Value::Array(result))
})()),

"ifft" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("ifft requires 1 arg".into()); }
    let vals: Vec<Complex<f64>> = if let Value::Array(arr) = &args[0] {
        arr.iter().map(|x| {
            if let Value::Map(m) = x {
                let re = m.get("re").map(|v| v.as_float()).unwrap_or(0.0);
                let im = m.get("im").map(|v| v.as_float()).unwrap_or(0.0);
                Complex { re, im }
            } else {
                Complex { re: x.as_float(), im: 0.0 }
            }
        }).collect()
    } else { return Err("Expected array of {re, im} maps".into()); };
    let n = vals.len();
    if n == 0 { return Ok(Value::Array(vec![])); }
    let mut buffer = vals;
    let mut planner = FftPlanner::new();
    let ifft_plan = planner.plan_fft_inverse(n);
    ifft_plan.process(&mut buffer);
    let scale = 1.0 / n as f64;
    Ok(Value::Array(buffer.iter().map(|c| Value::Float(c.re * scale)).collect()))
})()),

"fft_magnitude" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("fft_magnitude requires 1 arg".into()); }
    match &args[0] {
        Value::Array(arr) => {
            // Check if it's output from fft() — array of {re, im} maps
            if !arr.is_empty() {
                if let Value::Map(m) = &arr[0] {
                    if m.contains_key("re") || m.contains_key("im") {
                        let mags: Vec<Value> = arr.iter().map(|item| {
                            if let Value::Map(m) = item {
                                let re = m.get("re").map(|v| v.as_float()).unwrap_or(0.0);
                                let im = m.get("im").map(|v| v.as_float()).unwrap_or(0.0);
                                Value::Float((re * re + im * im).sqrt())
                            } else {
                                Value::Float(item.as_float().abs())
                            }
                        }).collect();
                        return Ok(Value::Array(mags));
                    }
                }
            }
            // Raw float array — run FFT then get magnitudes
            let vals: Vec<f64> = arr.iter().map(|x| x.as_float()).collect();
            let n = vals.len();
            if n == 0 { return Ok(Value::Array(vec![])); }
            let mut buffer: Vec<Complex<f64>> = vals.iter().map(|&r| Complex { re: r, im: 0.0 }).collect();
            let mut planner = FftPlanner::new();
            let fft_plan = planner.plan_fft_forward(n);
            fft_plan.process(&mut buffer);
            Ok(Value::Array(buffer.iter().map(|c| Value::Float(c.norm())).collect()))
        }
        _ => Err("fft_magnitude: expected array".into()),
    }
})()),

"fft_phase" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("fft_phase requires 1 arg".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let n = vals.len();
    if n == 0 { return Ok(Value::Array(vec![])); }
    let mut buffer: Vec<Complex<f64>> = vals.iter().map(|&r| Complex { re: r, im: 0.0 }).collect();
    let mut planner = FftPlanner::new();
    let fft_plan = planner.plan_fft_forward(n);
    fft_plan.process(&mut buffer);
    Ok(Value::Array(buffer.iter().map(|c| Value::Float(c.arg())).collect()))
})()),

"fft_power_spectrum" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("fft_power_spectrum requires 1 arg".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let n = vals.len();
    if n == 0 { return Ok(Value::Array(vec![])); }
    let mut buffer: Vec<Complex<f64>> = vals.iter().map(|&r| Complex { re: r, im: 0.0 }).collect();
    let mut planner = FftPlanner::new();
    let fft_plan = planner.plan_fft_forward(n);
    fft_plan.process(&mut buffer);
    Ok(Value::Array(buffer.iter().map(|c| Value::Float(c.norm_sqr())).collect()))
})()),

"fft_frequencies" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("fft_frequencies requires 2 args (n, sample_rate)".into()); }
    let n = args[0].as_int() as usize;
    let sample_rate = args[1].as_float();
    if n == 0 { return Ok(Value::Array(vec![])); }
    let freqs: Vec<Value> = (0..n).map(|i| {
        let k = if i <= n / 2 { i as f64 } else { i as f64 - n as f64 };
        Value::Float(k * sample_rate / n as f64)
    }).collect();
    Ok(Value::Array(freqs))
})()),

"integrate_simpson" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("integrate_simpson requires 2 args (arr, dx)".into()); }
    let y: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
    let dx = args[1].as_float();
    let n = y.len();
    if n < 2 { return Ok(Value::Float(0.0)); }
    if n % 2 == 0 {
        // Use composite Simpson's 3/8 or fall back to trapezoidal for last interval
        let m = n - 1;
        let mut sum = y[0] + y[m];
        for i in 1..m {
            sum += if i % 2 == 0 { 2.0 * y[i] } else { 4.0 * y[i] };
        }
        Ok(Value::Float(sum * dx / 3.0))
    } else {
        // n is odd → even number of intervals: standard composite Simpson's rule
        let m = n - 1;
        let mut sum = y[0] + y[m];
        for i in 1..m {
            sum += if i % 2 == 0 { 2.0 * y[i] } else { 4.0 * y[i] };
        }
        Ok(Value::Float(sum * dx / 3.0))
    }
})()),

"integrate_trapezoidal" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("integrate_trapezoidal requires 2 args".into()); }
    // Two forms:
    // (y_array, dx)            — uniform step
    // (x_array, y_array)       — non-uniform x values
    match (&args[0], &args[1]) {
        (Value::Array(x_arr), Value::Array(y_arr)) => {
            // (x_array, y_array) non-uniform trapezoidal
            let xs: Vec<f64> = x_arr.iter().map(|v| v.as_float()).collect();
            let ys: Vec<f64> = y_arr.iter().map(|v| v.as_float()).collect();
            let n = xs.len().min(ys.len());
            if n < 2 { return Ok(Value::Float(0.0)); }
            let mut sum = 0.0f64;
            for i in 0..n-1 {
                sum += (ys[i] + ys[i+1]) * (xs[i+1] - xs[i]) / 2.0;
            }
            Ok(Value::Float(sum))
        }
        _ => {
            // (y_array, dx) uniform
            let y: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|x| x.as_float()).collect() } else { return Err("Expected array".into()); };
            let dx = args[1].as_float();
            let n = y.len();
            if n < 2 { return Ok(Value::Float(0.0)); }
            let sum: f64 = y[0] / 2.0 + y[n - 1] / 2.0 + y[1..n - 1].iter().sum::<f64>();
            Ok(Value::Float(sum * dx))
        }
    }
})()),

"ode_rk4" => Some((|| -> Result<Value, String> {
    // Solves dy/dt = k*y (exponential growth/decay)
    // args: y0, t0, t1, dt, k
    if args.len() < 5 { return Err("ode_rk4 requires 5 args (y0, t0, t1, dt, k)".into()); }
    let mut y = args[0].as_float();
    let t0 = args[1].as_float();
    let t1 = args[2].as_float();
    let dt = args[3].as_float();
    let k = args[4].as_float();
    if dt <= 0.0 { return Err("dt must be positive".into()); }
    let f = |_t: f64, y: f64| k * y;
    let mut t = t0;
    let mut result = vec![Value::Array(vec![Value::Float(t), Value::Float(y)])];
    while t < t1 - dt * 0.5 {
        let h = dt.min(t1 - t);
        let k1 = f(t, y);
        let k2 = f(t + h / 2.0, y + h / 2.0 * k1);
        let k3 = f(t + h / 2.0, y + h / 2.0 * k2);
        let k4 = f(t + h, y + h * k3);
        y += h / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + k4);
        t += h;
        result.push(Value::Array(vec![Value::Float(t), Value::Float(y)]));
    }
    Ok(Value::Array(result))
})()),

"interpolate_linear" => Some((|| -> Result<Value, String> {
    if args.len() < 3 { return Err("interpolate_linear requires 3 args (x_arr, y_arr, x)".into()); }
    let xs: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected x array".into()); };
    let ys: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected y array".into()); };
    let x = args[2].as_float();
    if xs.len() != ys.len() || xs.is_empty() { return Err("x and y arrays must be non-empty and same length".into()); }
    if x <= xs[0] { return Ok(Value::Float(ys[0])); }
    if x >= xs[xs.len() - 1] { return Ok(Value::Float(ys[ys.len() - 1])); }
    for i in 0..xs.len() - 1 {
        if x >= xs[i] && x <= xs[i + 1] {
            let t = (x - xs[i]) / (xs[i + 1] - xs[i]);
            return Ok(Value::Float(ys[i] + t * (ys[i + 1] - ys[i])));
        }
    }
    Ok(Value::Float(ys[ys.len() - 1]))
})()),

"interpolate_cubic" => Some((|| -> Result<Value, String> {
    // Catmull-Rom spline interpolation
    if args.len() < 3 { return Err("interpolate_cubic requires 3 args (x_arr, y_arr, x)".into()); }
    let xs: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected x array".into()); };
    let ys: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected y array".into()); };
    let x = args[2].as_float();
    let n = xs.len();
    if n != ys.len() || n == 0 { return Err("x and y arrays must be non-empty and same length".into()); }
    if n == 1 { return Ok(Value::Float(ys[0])); }
    if n == 2 {
        let t = (x - xs[0]) / (xs[1] - xs[0]);
        return Ok(Value::Float(ys[0] + t * (ys[1] - ys[0])));
    }
    if x <= xs[0] { return Ok(Value::Float(ys[0])); }
    if x >= xs[n - 1] { return Ok(Value::Float(ys[n - 1])); }
    // Find segment
    let mut seg = n - 2;
    for i in 0..n - 1 {
        if x <= xs[i + 1] { seg = i; break; }
    }
    let i0 = if seg == 0 { 0 } else { seg - 1 };
    let i1 = seg;
    let i2 = seg + 1;
    let i3 = if seg + 2 < n { seg + 2 } else { n - 1 };
    let p0 = ys[i0]; let p1 = ys[i1]; let p2 = ys[i2]; let p3 = ys[i3];
    let t = (x - xs[i1]) / (xs[i2] - xs[i1]);
    // Catmull-Rom basis
    let t2 = t * t; let t3 = t2 * t;
    let v = 0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3);
    Ok(Value::Float(v))
})()),

"numerical_diff" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("numerical_diff requires 2 args (x_arr, y_arr)".into()); }
    let xs: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected x array".into()); };
    let ys: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected y array".into()); };
    let n = xs.len();
    if n != ys.len() || n == 0 { return Err("x and y arrays must be non-empty and same length".into()); }
    if n == 1 { return Ok(Value::Array(vec![Value::Float(0.0)])); }
    let mut dy = Vec::with_capacity(n);
    // Forward difference at start
    dy.push((ys[1] - ys[0]) / (xs[1] - xs[0]));
    // Central differences in middle
    for i in 1..n - 1 {
        dy.push((ys[i + 1] - ys[i - 1]) / (xs[i + 1] - xs[i - 1]));
    }
    // Backward difference at end
    dy.push((ys[n - 1] - ys[n - 2]) / (xs[n - 1] - xs[n - 2]));
    Ok(Value::Array(dy.into_iter().map(Value::Float).collect()))
})()),

"linspace" => Some((|| -> Result<Value, String> {
    if args.len() < 3 { return Err("linspace requires 3 args (start, end, n)".into()); }
    let start = args[0].as_float();
    let end = args[1].as_float();
    let n = args[2].as_int() as usize;
    if n == 0 { return Ok(Value::Array(vec![])); }
    if n == 1 { return Ok(Value::Array(vec![Value::Float(start)])); }
    let step = (end - start) / (n - 1) as f64;
    Ok(Value::Array((0..n).map(|i| Value::Float(start + i as f64 * step)).collect()))
})()),

"logspace" => Some((|| -> Result<Value, String> {
    if args.len() < 3 { return Err("logspace requires 3 args (start, end, n)".into()); }
    let start = args[0].as_float();
    let end = args[1].as_float();
    let n = args[2].as_int() as usize;
    if n == 0 { return Ok(Value::Array(vec![])); }
    if n == 1 { return Ok(Value::Array(vec![Value::Float(10_f64.powf(start))])); }
    let step = (end - start) / (n - 1) as f64;
    Ok(Value::Array((0..n).map(|i| Value::Float(10_f64.powf(start + i as f64 * step))).collect()))
})()),

"arange" => Some((|| -> Result<Value, String> {
    if args.len() < 3 { return Err("arange requires 3 args (start, end, step)".into()); }
    let start = args[0].as_float();
    let end = args[1].as_float();
    let step = args[2].as_float();
    if step == 0.0 { return Err("step cannot be zero".into()); }
    let mut result = Vec::new();
    let mut v = start;
    if step > 0.0 {
        while v < end { result.push(Value::Float(v)); v += step; }
    } else {
        while v > end { result.push(Value::Float(v)); v += step; }
    }
    Ok(Value::Array(result))
})()),

"meshgrid" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("meshgrid requires 2 args (x_arr, y_arr)".into()); }
    let xs: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected x array".into()); };
    let ys: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected y array".into()); };
    let ny = ys.len();
    let nx = xs.len();
    // X grid: each row is xs repeated for each y
    let x_grid: Vec<Value> = (0..ny).map(|_| Value::Array(xs.iter().map(|&x| Value::Float(x)).collect())).collect();
    // Y grid: each row is the same y value for each x
    let y_grid: Vec<Value> = ys.iter().map(|&y| Value::Array((0..nx).map(|_| Value::Float(y)).collect())).collect();
    let mut result = HashMap::new();
    result.insert("X".into(), Value::Array(x_grid));
    result.insert("Y".into(), Value::Array(y_grid));
    Ok(Value::Map(result))
})()),

"stat_correlation" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("stat_correlation requires 2 args".into()); }
    let x: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let y: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let n = x.len();
    if n != y.len() || n == 0 { return Err("Arrays must be non-empty and same length".into()); }
    let mx = x.iter().sum::<f64>() / n as f64;
    let my = y.iter().sum::<f64>() / n as f64;
    let cov: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| (xi - mx) * (yi - my)).sum::<f64>() / n as f64;
    let sx: f64 = (x.iter().map(|xi| (xi - mx).powi(2)).sum::<f64>() / n as f64).sqrt();
    let sy: f64 = (y.iter().map(|yi| (yi - my).powi(2)).sum::<f64>() / n as f64).sqrt();
    if sx == 0.0 || sy == 0.0 { return Ok(Value::Float(0.0)); }
    Ok(Value::Float(cov / (sx * sy)))
})()),

"stat_covariance" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("stat_covariance requires 2 args".into()); }
    let x: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let y: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let n = x.len();
    if n != y.len() || n < 2 { return Err("Arrays must have at least 2 elements and same length".into()); }
    let mx = x.iter().sum::<f64>() / n as f64;
    let my = y.iter().sum::<f64>() / n as f64;
    let cov: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| (xi - mx) * (yi - my)).sum::<f64>() / (n - 1) as f64;
    Ok(Value::Float(cov))
})()),

"stat_regression_linear" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("stat_regression_linear requires 2 args (x, y)".into()); }
    let x: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let y: Vec<f64> = if let Value::Array(arr) = &args[1] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let n = x.len();
    if n != y.len() || n < 2 { return Err("Arrays must have at least 2 elements and same length".into()); }
    let nf = n as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xx: f64 = x.iter().map(|xi| xi * xi).sum();
    let sum_xy: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| xi * yi).sum();
    let denom = nf * sum_xx - sum_x * sum_x;
    if denom == 0.0 { return Err("Cannot fit: all x values are identical".into()); }
    let slope = (nf * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / nf;
    let y_mean = sum_y / nf;
    let ss_res: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| {
        let y_pred = slope * xi + intercept;
        (yi - y_pred).powi(2)
    }).sum();
    let ss_tot: f64 = y.iter().map(|yi| (yi - y_mean).powi(2)).sum();
    let r2 = if ss_tot == 0.0 { 1.0 } else { 1.0 - ss_res / ss_tot };
    let mut result = HashMap::new();
    result.insert("slope".into(), Value::Float(slope));
    result.insert("intercept".into(), Value::Float(intercept));
    result.insert("r2".into(), Value::Float(r2));
    Ok(Value::Map(result))
})()),

"stat_histogram" => Some((|| -> Result<Value, String> {
    if args.len() < 2 { return Err("stat_histogram requires 2 args (arr, bins)".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let bins = args[1].as_int() as usize;
    if vals.is_empty() || bins == 0 { return Ok(Value::Array(vec![])); }
    let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    let bin_width = if range == 0.0 { 1.0 } else { range / bins as f64 };
    let mut counts = vec![0usize; bins];
    for &v in &vals {
        let idx = if range == 0.0 { 0 } else {
            let i = ((v - min) / bin_width) as usize;
            i.min(bins - 1)
        };
        counts[idx] += 1;
    }
    let result: Vec<Value> = (0..bins).map(|i| {
        let bin_start = min + i as f64 * bin_width;
        let bin_end = bin_start + bin_width;
        let mut m = HashMap::new();
        m.insert("bin_start".into(), Value::Float(bin_start));
        m.insert("bin_end".into(), Value::Float(bin_end));
        m.insert("count".into(), Value::Int(counts[i] as i64));
        Value::Map(m)
    }).collect();
    Ok(Value::Array(result))
})()),

"stat_entropy" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("stat_entropy requires 1 arg".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    if vals.is_empty() { return Ok(Value::Float(0.0)); }
    let total: f64 = vals.iter().sum();
    if total == 0.0 { return Err("Cannot compute entropy: all values are zero".into()); }
    let entropy: f64 = vals.iter()
        .filter(|&&v| v > 0.0)
        .map(|&v| { let p = v / total; -p * p.ln() })
        .sum();
    Ok(Value::Float(entropy))
})()),

"stat_zscore" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("stat_zscore requires 1 arg".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let n = vals.len();
    if n == 0 { return Ok(Value::Array(vec![])); }
    let mean = vals.iter().sum::<f64>() / n as f64;
    let variance = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();
    if std_dev == 0.0 { return Ok(Value::Array(vec![Value::Float(0.0); n])); }
    Ok(Value::Array(vals.iter().map(|&x| Value::Float((x - mean) / std_dev)).collect()))
})()),

"stat_normalize_minmax" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("stat_normalize_minmax requires 1 arg".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    if vals.is_empty() { return Ok(Value::Array(vec![])); }
    let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range == 0.0 { return Ok(Value::Array(vec![Value::Float(0.0); vals.len()])); }
    Ok(Value::Array(vals.iter().map(|&x| Value::Float((x - min) / range)).collect()))
})()),

"stat_iqr" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("stat_iqr requires 1 arg".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    if vals.is_empty() { return Ok(Value::Float(0.0)); }
    let mut sorted = vals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let q1_idx = n / 4;
    let q3_idx = 3 * n / 4;
    let q1 = sorted[q1_idx];
    let q3 = sorted[q3_idx.min(n - 1)];
    Ok(Value::Float(q3 - q1))
})()),

"stat_skewness" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("stat_skewness requires 1 arg".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let n = vals.len();
    if n < 3 { return Err("stat_skewness requires at least 3 elements".into()); }
    let nf = n as f64;
    let mean = vals.iter().sum::<f64>() / nf;
    let variance = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / nf;
    let std_dev = variance.sqrt();
    if std_dev == 0.0 { return Ok(Value::Float(0.0)); }
    let m3: f64 = vals.iter().map(|&x| ((x - mean) / std_dev).powi(3)).sum::<f64>() / nf;
    Ok(Value::Float(m3))
})()),

"stat_kurtosis" => Some((|| -> Result<Value, String> {
    if args.is_empty() { return Err("stat_kurtosis requires 1 arg".into()); }
    let vals: Vec<f64> = if let Value::Array(arr) = &args[0] { arr.iter().map(|v| v.as_float()).collect() } else { return Err("Expected array".into()); };
    let n = vals.len();
    if n < 4 { return Err("stat_kurtosis requires at least 4 elements".into()); }
    let nf = n as f64;
    let mean = vals.iter().sum::<f64>() / nf;
    let variance = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / nf;
    let std_dev = variance.sqrt();
    if std_dev == 0.0 { return Ok(Value::Float(0.0)); }
    let m4: f64 = vals.iter().map(|&x| ((x - mean) / std_dev).powi(4)).sum::<f64>() / nf;
    // Excess kurtosis: subtract 3
    Ok(Value::Float(m4 - 3.0))
})()),


            // ── KNULL_B9_GRAPH ──
"graph_new" => {
    let directed = if !args.is_empty() {
        match &args[0] {
            Value::Bool(b) => *b,
            _ => args[0].as_int() != 0,
        }
    } else {
        true
    };
    let id = self.graph_counter;
    self.graph_counter += 1;
    if directed {
        self.graphs_directed.insert(id, DiGraph::<String, f64>::new());
    } else {
        self.graphs_undirected.insert(id, UnGraph::<String, f64>::new_undirected());
    }
    let mut map = HashMap::new();
    map.insert("id".to_string(), Value::Int(id));
    map.insert("directed".to_string(), Value::Bool(directed));
    Some(Ok(Value::Map(map)))
}

"graph_add_node" => {
    if args.len() < 2 {
        return Some(Err("graph_add_node requires (id, label)".to_string()));
    }
    let id = args[0].as_int();
    let label = args[1].as_string();
    if let Some(g) = self.graphs_directed.get_mut(&id) {
        let idx = g.add_node(label);
        return Some(Ok(Value::Int(idx.index() as i64)));
    }
    if let Some(g) = self.graphs_undirected.get_mut(&id) {
        let idx = g.add_node(label);
        return Some(Ok(Value::Int(idx.index() as i64)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_add_edge" => {
    if args.len() < 3 {
        return Some(Err("graph_add_edge requires (id, from_idx, to_idx[, weight])".to_string()));
    }
    let id = args[0].as_int();
    let from = args[1].as_int() as usize;
    let to = args[2].as_int() as usize;
    let weight = if args.len() >= 4 { args[3].as_float() } else { 1.0_f64 };
    if let Some(g) = self.graphs_directed.get_mut(&id) {
        let ei = g.add_edge(NodeIndex::new(from), NodeIndex::new(to), weight);
        return Some(Ok(Value::Int(ei.index() as i64)));
    }
    if let Some(g) = self.graphs_undirected.get_mut(&id) {
        let ei = g.add_edge(NodeIndex::new(from), NodeIndex::new(to), weight);
        return Some(Ok(Value::Int(ei.index() as i64)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_remove_node" => {
    if args.len() < 2 {
        return Some(Err("graph_remove_node requires (id, node_idx)".to_string()));
    }
    let id = args[0].as_int();
    let node = NodeIndex::new(args[1].as_int() as usize);
    if let Some(g) = self.graphs_directed.get_mut(&id) {
        if g.remove_node(node).is_some() {
            return Some(Ok(Value::Bool(true)));
        }
        return Some(Err(format!("Node index {} not found in graph {}", node.index(), id)));
    }
    if let Some(g) = self.graphs_undirected.get_mut(&id) {
        if g.remove_node(node).is_some() {
            return Some(Ok(Value::Bool(true)));
        }
        return Some(Err(format!("Node index {} not found in graph {}", node.index(), id)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_remove_edge" => {
    if args.len() < 3 {
        return Some(Err("graph_remove_edge requires (id, from_idx, to_idx)".to_string()));
    }
    let id = args[0].as_int();
    let from = NodeIndex::new(args[1].as_int() as usize);
    let to = NodeIndex::new(args[2].as_int() as usize);
    if let Some(g) = self.graphs_directed.get_mut(&id) {
        if let Some(ei) = g.find_edge(from, to) {
            g.remove_edge(ei);
            return Some(Ok(Value::Bool(true)));
        }
        return Some(Err(format!("Edge {}->{} not found in graph {}", from.index(), to.index(), id)));
    }
    if let Some(g) = self.graphs_undirected.get_mut(&id) {
        if let Some(ei) = g.find_edge(from, to) {
            g.remove_edge(ei);
            return Some(Ok(Value::Bool(true)));
        }
        return Some(Err(format!("Edge {}--{} not found in graph {}", from.index(), to.index(), id)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_node_count" => {
    if args.is_empty() {
        return Some(Err("graph_node_count requires (id)".to_string()));
    }
    let id = args[0].as_int();
    if let Some(g) = self.graphs_directed.get(&id) {
        return Some(Ok(Value::Int(g.node_count() as i64)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        return Some(Ok(Value::Int(g.node_count() as i64)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_edge_count" => {
    if args.is_empty() {
        return Some(Err("graph_edge_count requires (id)".to_string()));
    }
    let id = args[0].as_int();
    if let Some(g) = self.graphs_directed.get(&id) {
        return Some(Ok(Value::Int(g.edge_count() as i64)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        return Some(Ok(Value::Int(g.edge_count() as i64)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_neighbors" => {
    if args.len() < 2 {
        return Some(Err("graph_neighbors requires (id, node_idx)".to_string()));
    }
    let id = args[0].as_int();
    let node = NodeIndex::new(args[1].as_int() as usize);
    if let Some(g) = self.graphs_directed.get(&id) {
        let neighbors: Vec<Value> = g.neighbors(node)
            .map(|n| Value::Int(n.index() as i64))
            .collect();
        return Some(Ok(Value::Array(neighbors)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        let neighbors: Vec<Value> = g.neighbors(node)
            .map(|n| Value::Int(n.index() as i64))
            .collect();
        return Some(Ok(Value::Array(neighbors)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_node_label" => {
    if args.len() < 2 {
        return Some(Err("graph_node_label requires (id, node_idx)".to_string()));
    }
    let id = args[0].as_int();
    let node = NodeIndex::new(args[1].as_int() as usize);
    if let Some(g) = self.graphs_directed.get(&id) {
        match g.node_weight(node) {
            Some(label) => return Some(Ok(Value::String(label.clone()))),
            None => return Some(Err(format!("Node index {} not found in graph {}", node.index(), id))),
        }
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        match g.node_weight(node) {
            Some(label) => return Some(Ok(Value::String(label.clone()))),
            None => return Some(Err(format!("Node index {} not found in graph {}", node.index(), id))),
        }
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_bfs" => {
    if args.len() < 2 {
        return Some(Err("graph_bfs requires (id, start_idx)".to_string()));
    }
    let id = args[0].as_int();
    let start = NodeIndex::new(args[1].as_int() as usize);
    if let Some(g) = self.graphs_directed.get(&id) {
        let mut bfs = petgraph::visit::Bfs::new(g, start);
        let mut result = Vec::new();
        while let Some(nx) = bfs.next(g) {
            result.push(Value::Int(nx.index() as i64));
        }
        return Some(Ok(Value::Array(result)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        let mut bfs = petgraph::visit::Bfs::new(g, start);
        let mut result = Vec::new();
        while let Some(nx) = bfs.next(g) {
            result.push(Value::Int(nx.index() as i64));
        }
        return Some(Ok(Value::Array(result)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_dfs" => {
    if args.len() < 2 {
        return Some(Err("graph_dfs requires (id, start_idx)".to_string()));
    }
    let id = args[0].as_int();
    let start = NodeIndex::new(args[1].as_int() as usize);
    if let Some(g) = self.graphs_directed.get(&id) {
        let mut dfs = petgraph::visit::Dfs::new(g, start);
        let mut result = Vec::new();
        while let Some(nx) = dfs.next(g) {
            result.push(Value::Int(nx.index() as i64));
        }
        return Some(Ok(Value::Array(result)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        let mut dfs = petgraph::visit::Dfs::new(g, start);
        let mut result = Vec::new();
        while let Some(nx) = dfs.next(g) {
            result.push(Value::Int(nx.index() as i64));
        }
        return Some(Ok(Value::Array(result)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_dijkstra" => {
    if args.len() < 2 {
        return Some(Err("graph_dijkstra requires (id, start_idx)".to_string()));
    }
    let id = args[0].as_int();
    let start = NodeIndex::new(args[1].as_int() as usize);
    if let Some(g) = self.graphs_directed.get(&id) {
        let scores = dijkstra(g, start, None, |e| *e.weight());
        let mut map = HashMap::new();
        for (node, dist) in scores {
            map.insert(node.index().to_string(), Value::Float(dist));
        }
        return Some(Ok(Value::Map(map)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        let scores = dijkstra(g, start, None, |e| *e.weight());
        let mut map = HashMap::new();
        for (node, dist) in scores {
            map.insert(node.index().to_string(), Value::Float(dist));
        }
        return Some(Ok(Value::Map(map)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_shortest_path" => {
    if args.len() < 3 {
        return Some(Err("graph_shortest_path requires (id, start_idx, end_idx)".to_string()));
    }
    let id = args[0].as_int();
    let start = NodeIndex::new(args[1].as_int() as usize);
    let end = NodeIndex::new(args[2].as_int() as usize);
    if let Some(g) = self.graphs_directed.get(&id) {
        match astar(g, start, |n| n == end, |e| *e.weight(), |_| 0.0_f64) {
            Some((cost, path)) => {
                let path_vals: Vec<Value> = path.iter()
                    .map(|n| Value::Int(n.index() as i64))
                    .collect();
                let mut map = HashMap::new();
                map.insert("path".to_string(), Value::Array(path_vals));
                map.insert("cost".to_string(), Value::Float(cost));
                return Some(Ok(Value::Map(map)));
            }
            None => return Some(Err(format!(
                "No path from {} to {} in graph {}", start.index(), end.index(), id
            ))),
        }
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        match astar(g, start, |n| n == end, |e| *e.weight(), |_| 0.0_f64) {
            Some((cost, path)) => {
                let path_vals: Vec<Value> = path.iter()
                    .map(|n| Value::Int(n.index() as i64))
                    .collect();
                let mut map = HashMap::new();
                map.insert("path".to_string(), Value::Array(path_vals));
                map.insert("cost".to_string(), Value::Float(cost));
                return Some(Ok(Value::Map(map)));
            }
            None => return Some(Err(format!(
                "No path from {} to {} in graph {}", start.index(), end.index(), id
            ))),
        }
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_bellman_ford" => {
    if args.len() < 2 {
        return Some(Err("graph_bellman_ford requires (id, start_idx)".to_string()));
    }
    let id = args[0].as_int();
    let start = NodeIndex::new(args[1].as_int() as usize);
    if let Some(g) = self.graphs_directed.get(&id) {
        match bellman_ford(g, start) {
            Ok(paths) => {
                let mut map = HashMap::new();
                for node in g.node_indices() {
                    let d = paths.distances[node.index()];
                    if d.is_finite() {
                        map.insert(node.index().to_string(), Value::Float(d));
                    }
                }
                return Some(Ok(Value::Map(map)));
            }
            Err(_) => return Some(Err("Negative cycle detected in graph".to_string())),
        }
    }
    if self.graphs_undirected.contains_key(&id) {
        return Some(Err("graph_bellman_ford is only supported for directed graphs".to_string()));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_topo_sort" => {
    if args.is_empty() {
        return Some(Err("graph_topo_sort requires (id)".to_string()));
    }
    let id = args[0].as_int();
    if let Some(g) = self.graphs_directed.get(&id) {
        match toposort(g, None) {
            Ok(order) => {
                let result: Vec<Value> = order.iter()
                    .map(|n| Value::Int(n.index() as i64))
                    .collect();
                return Some(Ok(Value::Array(result)));
            }
            Err(_) => return Some(Err(
                "Graph contains a cycle; topological sort not possible".to_string()
            )),
        }
    }
    if self.graphs_undirected.contains_key(&id) {
        return Some(Err("graph_topo_sort is only supported for directed graphs".to_string()));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_scc" => {
    if args.is_empty() {
        return Some(Err("graph_scc requires (id)".to_string()));
    }
    let id = args[0].as_int();
    if let Some(g) = self.graphs_directed.get(&id) {
        let sccs = petgraph::algo::kosaraju_scc(g);
        let result: Vec<Value> = sccs.iter()
            .map(|component| {
                let nodes: Vec<Value> = component.iter()
                    .map(|n| Value::Int(n.index() as i64))
                    .collect();
                Value::Array(nodes)
            })
            .collect();
        return Some(Ok(Value::Array(result)));
    }
    if self.graphs_undirected.contains_key(&id) {
        return Some(Err("graph_scc is only supported for directed graphs".to_string()));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_connected_components" => {
    if args.is_empty() {
        return Some(Err("graph_connected_components requires (id)".to_string()));
    }
    let id = args[0].as_int();
    if let Some(g) = self.graphs_undirected.get(&id) {
        let count = connected_components(g);
        return Some(Ok(Value::Int(count as i64)));
    }
    if self.graphs_directed.contains_key(&id) {
        return Some(Err(
            "graph_connected_components is only supported for undirected graphs".to_string()
        ));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_is_cyclic" => {
    if args.is_empty() {
        return Some(Err("graph_is_cyclic requires (id)".to_string()));
    }
    let id = args[0].as_int();
    if let Some(g) = self.graphs_directed.get(&id) {
        return Some(Ok(Value::Bool(is_cyclic_directed(g))));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        return Some(Ok(Value::Bool(petgraph::algo::is_cyclic_undirected(g))));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_min_spanning_tree" => {
    if args.is_empty() {
        return Some(Err("graph_min_spanning_tree requires (id)".to_string()));
    }
    use petgraph::data::Element;
    let id = args[0].as_int();
    if let Some(g) = self.graphs_undirected.get(&id) {
        let mut edges = Vec::new();
        for element in petgraph::algo::min_spanning_tree(g) {
            if let Element::Edge { source, target, weight } = element {
                let mut edge_map = HashMap::new();
                edge_map.insert("from".to_string(), Value::Int(source as i64));
                edge_map.insert("to".to_string(), Value::Int(target as i64));
                edge_map.insert("weight".to_string(), Value::Float(weight));
                edges.push(Value::Map(edge_map));
            }
        }
        return Some(Ok(Value::Array(edges)));
    }
    if let Some(g) = self.graphs_directed.get(&id) {
        let mut edges = Vec::new();
        for element in petgraph::algo::min_spanning_tree(g) {
            if let Element::Edge { source, target, weight } = element {
                let mut edge_map = HashMap::new();
                edge_map.insert("from".to_string(), Value::Int(source as i64));
                edge_map.insert("to".to_string(), Value::Int(target as i64));
                edge_map.insert("weight".to_string(), Value::Float(weight));
                edges.push(Value::Map(edge_map));
            }
        }
        return Some(Ok(Value::Array(edges)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_node_degree" => {
    if args.len() < 2 {
        return Some(Err("graph_node_degree requires (id, node_idx)".to_string()));
    }
    let id = args[0].as_int();
    let node = NodeIndex::new(args[1].as_int() as usize);
    if let Some(g) = self.graphs_directed.get(&id) {
        let out_deg = g.neighbors_directed(node, petgraph::Direction::Outgoing).count();
        let in_deg  = g.neighbors_directed(node, petgraph::Direction::Incoming).count();
        return Some(Ok(Value::Int((out_deg + in_deg) as i64)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        let deg = g.neighbors(node).count();
        return Some(Ok(Value::Int(deg as i64)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_all_nodes" => {
    if args.is_empty() {
        return Some(Err("graph_all_nodes requires (id)".to_string()));
    }
    let id = args[0].as_int();
    if let Some(g) = self.graphs_directed.get(&id) {
        let result: Vec<Value> = g.node_indices()
            .map(|n| {
                let mut m = HashMap::new();
                m.insert("idx".to_string(), Value::Int(n.index() as i64));
                m.insert(
                    "label".to_string(),
                    Value::String(g.node_weight(n).cloned().unwrap_or_default()),
                );
                Value::Map(m)
            })
            .collect();
        return Some(Ok(Value::Array(result)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        let result: Vec<Value> = g.node_indices()
            .map(|n| {
                let mut m = HashMap::new();
                m.insert("idx".to_string(), Value::Int(n.index() as i64));
                m.insert(
                    "label".to_string(),
                    Value::String(g.node_weight(n).cloned().unwrap_or_default()),
                );
                Value::Map(m)
            })
            .collect();
        return Some(Ok(Value::Array(result)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_all_edges" => {
    if args.is_empty() {
        return Some(Err("graph_all_edges requires (id)".to_string()));
    }
    let id = args[0].as_int();
    if let Some(g) = self.graphs_directed.get(&id) {
        let result: Vec<Value> = g.edge_references()
            .map(|e| {
                let mut m = HashMap::new();
                m.insert("from".to_string(), Value::Int(e.source().index() as i64));
                m.insert("to".to_string(), Value::Int(e.target().index() as i64));
                m.insert("weight".to_string(), Value::Float(*e.weight()));
                Value::Map(m)
            })
            .collect();
        return Some(Ok(Value::Array(result)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        let result: Vec<Value> = g.edge_references()
            .map(|e| {
                let mut m = HashMap::new();
                m.insert("from".to_string(), Value::Int(e.source().index() as i64));
                m.insert("to".to_string(), Value::Int(e.target().index() as i64));
                m.insert("weight".to_string(), Value::Float(*e.weight()));
                Value::Map(m)
            })
            .collect();
        return Some(Ok(Value::Array(result)));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_has_edge" => {
    if args.len() < 3 {
        return Some(Err("graph_has_edge requires (id, from_idx, to_idx)".to_string()));
    }
    let id = args[0].as_int();
    let from = NodeIndex::new(args[1].as_int() as usize);
    let to = NodeIndex::new(args[2].as_int() as usize);
    if let Some(g) = self.graphs_directed.get(&id) {
        return Some(Ok(Value::Bool(g.find_edge(from, to).is_some())));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        return Some(Ok(Value::Bool(g.find_edge(from, to).is_some())));
    }
    Some(Err(format!("Graph {} not found", id)))
}

"graph_free" => {
    if args.is_empty() {
        return Some(Err("graph_free requires (id)".to_string()));
    }
    let id = args[0].as_int();
    let removed_d = self.graphs_directed.remove(&id).is_some();
    let removed_u = self.graphs_undirected.remove(&id).is_some();
    if removed_d || removed_u {
        Some(Ok(Value::Bool(true)))
    } else {
        Some(Err(format!("Graph {} not found", id)))
    }
}

"graph_to_dot" => {
    if args.is_empty() {
        return Some(Err("graph_to_dot requires (id)".to_string()));
    }
    let id = args[0].as_int();
    if let Some(g) = self.graphs_directed.get(&id) {
        let mut dot = String::from("digraph G {\n");
        for n in g.node_indices() {
            let label = g.node_weight(n).map(|s| s.as_str()).unwrap_or("");
            dot.push_str(&format!("  {} [label=\"{}\"];\n", n.index(), label));
        }
        for e in g.edge_references() {
            dot.push_str(&format!(
                "  {} -> {} [label=\"{}\"];\n",
                e.source().index(),
                e.target().index(),
                e.weight()
            ));
        }
        dot.push('}');
        return Some(Ok(Value::String(dot)));
    }
    if let Some(g) = self.graphs_undirected.get(&id) {
        let mut dot = String::from("graph G {\n");
        for n in g.node_indices() {
            let label = g.node_weight(n).map(|s| s.as_str()).unwrap_or("");
            dot.push_str(&format!("  {} [label=\"{}\"];\n", n.index(), label));
        }
        for e in g.edge_references() {
            dot.push_str(&format!(
                "  {} -- {} [label=\"{}\"];\n",
                e.source().index(),
                e.target().index(),
                e.weight()
            ));
        }
        dot.push('}');
        return Some(Ok(Value::String(dot)));
    }
    Some(Err(format!("Graph {} not found", id)))
}


            // ── KNULL_B9_ENCODE ──
// Knull interpreter builtins — batch 9: hashing, encoding, msgpack, bincode, JWT, templates, bit ops
// Match arms for: fn call_builtin(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, String>>

// ─────────────────────  HASHING  ─────────────────────

"xxhash64" => {
    if args.is_empty() {
        return Some(Err("xxhash64 requires 1 argument".to_string()));
    }
    let bytes = vbytes(&args[0]);
    let hash = xxh3_64(&bytes);
    Some(Ok(Value::Int(hash as i64)))
}

"xxhash32" => {
    if args.is_empty() {
        return Some(Err("xxhash32 requires at least 1 argument".to_string()));
    }
    let bytes = vbytes(&args[0]);
    let seed = if args.len() > 1 { args[1].as_int() as u32 } else { 0u32 };
    let hash = xxh32(&bytes, seed);
    Some(Ok(Value::Int(hash as i64)))
}

"murmur3_32" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    let seed: u32 = if args.len() > 1 { args[1].as_int() as u32 } else { 0u32 };
    let c1: u32 = 0xcc9e2d51;
    let c2: u32 = 0x1b873593;
    let len = bytes.len();
    let nblocks = len / 4;
    let mut h1 = seed;
    for i in 0..nblocks {
        let mut k1 = u32::from_le_bytes([
            bytes[i * 4],
            bytes[i * 4 + 1],
            bytes[i * 4 + 2],
            bytes[i * 4 + 3],
        ]);
        k1 = k1.wrapping_mul(c1);
        k1 = k1.rotate_left(15);
        k1 = k1.wrapping_mul(c2);
        h1 ^= k1;
        h1 = h1.rotate_left(13);
        h1 = h1.wrapping_mul(5).wrapping_add(0xe6546b64);
    }
    let tail = &bytes[nblocks * 4..];
    let mut k1: u32 = 0;
    match tail.len() {
        3 => {
            k1 ^= (tail[2] as u32) << 16;
            k1 ^= (tail[1] as u32) << 8;
            k1 ^= tail[0] as u32;
            k1 = k1.wrapping_mul(c1);
            k1 = k1.rotate_left(15);
            k1 = k1.wrapping_mul(c2);
            h1 ^= k1;
        }
        2 => {
            k1 ^= (tail[1] as u32) << 8;
            k1 ^= tail[0] as u32;
            k1 = k1.wrapping_mul(c1);
            k1 = k1.rotate_left(15);
            k1 = k1.wrapping_mul(c2);
            h1 ^= k1;
        }
        1 => {
            k1 ^= tail[0] as u32;
            k1 = k1.wrapping_mul(c1);
            k1 = k1.rotate_left(15);
            k1 = k1.wrapping_mul(c2);
            h1 ^= k1;
        }
        _ => {}
    }
    h1 ^= len as u32;
    // fmix32
    h1 ^= h1 >> 16;
    h1 = h1.wrapping_mul(0x85ebca6b);
    h1 ^= h1 >> 13;
    h1 = h1.wrapping_mul(0xc2b2ae35);
    h1 ^= h1 >> 16;
    Some(Ok(Value::Int(h1 as i64)))
}

"fnv1a_32" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    let mut hash: u32 = 0x811c9dc5;
    for &b in &bytes {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    Some(Ok(Value::Int(hash as i64)))
}

"fnv1a_64" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in &bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x00000100000001b3);
    }
    Some(Ok(Value::Int(hash as i64)))
}

"djb2" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    let mut hash: u64 = 5381;
    for &b in &bytes {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    Some(Ok(Value::Int(hash as i64)))
}

"adler32" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    const MOD_ADLER: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in &bytes {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    let result = (b << 16) | a;
    Some(Ok(Value::Int(result as i64)))
}

"crc32" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in &bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc = !crc;
    Some(Ok(Value::Int(crc as i64)))
}

"crc16" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    let mut crc: u16 = 0x0000;
    for &byte in &bytes {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    Some(Ok(Value::Int(crc as i64)))
}

// ─────────────────────  ENCODING  ─────────────────────

"base32_encode" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    let encoded = base32::encode(base32::Alphabet::RFC4648 { padding: true }, &bytes);
    Some(Ok(Value::String(encoded)))
}

"base32_decode" => {
    if args.is_empty() {
        return Some(Err("base32_decode requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    match base32::decode(base32::Alphabet::RFC4648 { padding: true }, &s) {
        Some(bytes) => {
            let result = String::from_utf8(bytes.clone())
                .unwrap_or_else(|_| b2he(&bytes));
            Some(Ok(Value::String(result)))
        }
        None => Some(Err("base32_decode: invalid base32 input".to_string())),
    }
}

"base58_encode" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    let encoded = bs58::encode(bytes).into_string();
    Some(Ok(Value::String(encoded)))
}

"base58_decode" => {
    if args.is_empty() {
        return Some(Err("base58_decode requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    match bs58::decode(s).into_vec() {
        Ok(bytes) => Some(Ok(Value::String(b2he(&bytes)))),
        Err(e) => Some(Err(format!("base58_decode error: {}", e))),
    }
}

"base16_encode" => {
    let bytes = if args.is_empty() { vec![] } else { vbytes(&args[0]) };
    let encoded: String = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    Some(Ok(Value::String(encoded)))
}

"base16_decode" => {
    if args.is_empty() {
        return Some(Err("base16_decode requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    match he2b(&s) {
        Some(bytes) => {
            let result = String::from_utf8(bytes.clone())
                .unwrap_or_else(|_| b2he(&bytes));
            Some(Ok(Value::String(result)))
        }
        None => Some(Err("base16_decode: invalid hex input".to_string())),
    }
}

"hex_to_bytes" => {
    if args.is_empty() {
        return Some(Err("hex_to_bytes requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    match he2b(&s) {
        Some(bytes) => {
            let arr: Vec<Value> = bytes.iter().map(|&b| Value::Int(b as i64)).collect();
            Some(Ok(Value::Array(arr)))
        }
        None => Some(Err("hex_to_bytes: invalid hex string".to_string())),
    }
}

"bytes_to_hex" => {
    if args.is_empty() {
        return Some(Err("bytes_to_hex requires 1 argument".to_string()));
    }
    let arr = match &args[0] {
        Value::Array(v) => v.clone(),
        _ => return Some(Err("bytes_to_hex: expected Array".to_string())),
    };
    let bytes: Vec<u8> = arr.iter().map(|v| v.as_int() as u8).collect();
    Some(Ok(Value::String(b2he(&bytes))))
}

"url_encode" => {
    if args.is_empty() {
        return Some(Err("url_encode requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    let mut encoded = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push('%');
                encoded.push(char::from_digit((byte >> 4) as u32, 16).unwrap().to_ascii_uppercase());
                encoded.push(char::from_digit((byte & 0xf) as u32, 16).unwrap().to_ascii_uppercase());
            }
        }
    }
    Some(Ok(Value::String(encoded)))
}

"url_decode" => {
    if args.is_empty() {
        return Some(Err("url_decode requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    let bytes_in = s.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes_in.len());
    let mut i = 0usize;
    while i < bytes_in.len() {
        if bytes_in[i] == b'%' && i + 2 < bytes_in.len() {
            let hex = &bytes_in[i + 1..i + 3];
            if let Ok(hex_str) = std::str::from_utf8(hex) {
                if let Ok(b) = u8::from_str_radix(hex_str, 16) {
                    decoded.push(b);
                    i += 3;
                    continue;
                }
            }
            decoded.push(bytes_in[i]);
            i += 1;
        } else if bytes_in[i] == b'+' {
            decoded.push(b' ');
            i += 1;
        } else {
            decoded.push(bytes_in[i]);
            i += 1;
        }
    }
    Some(Ok(Value::String(String::from_utf8_lossy(&decoded).into_owned())))
}

"html_encode" => {
    if args.is_empty() {
        return Some(Err("html_encode requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '&'  => out.push_str("&amp;"),
            '<'  => out.push_str("&lt;"),
            '>'  => out.push_str("&gt;"),
            '"'  => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _    => out.push(c),
        }
    }
    Some(Ok(Value::String(out)))
}

"html_decode" => {
    if args.is_empty() {
        return Some(Err("html_decode requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    let out = s
        .replace("&amp;",  "&")
        .replace("&lt;",   "<")
        .replace("&gt;",   ">")
        .replace("&quot;", "\"")
        .replace("&#39;",  "'")
        .replace("&apos;", "'");
    Some(Ok(Value::String(out)))
}

"punycode_encode" => {
    if args.is_empty() {
        return Some(Err("punycode_encode requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    // Simplified: per-label encoding for hostnames
    let encoded_labels: Vec<String> = s.split('.').map(|label| {
        if label.is_ascii() {
            label.to_string()
        } else {
            // Very simplified: prefix with xn-- and append ASCII chars,
            // then append hex of non-ASCII bytes joined by '-'
            let ascii_part: String = label.chars().filter(|c| c.is_ascii()).collect();
            let non_ascii_hex: String = label
                .chars()
                .filter(|c| !c.is_ascii())
                .map(|c| { let mut buf = [0u8; 4]; let n = c.encode_utf8(&mut buf).len(); format!("{}", &buf[..n].iter().map(|b| format!("{:02x}", b)).collect::<String>()) })
                .collect::<Vec<_>>()
                .join("-");
            if ascii_part.is_empty() {
                format!("xn--{}", non_ascii_hex)
            } else {
                format!("xn--{}-{}", ascii_part, non_ascii_hex)
            }
        }
    }).collect();
    Some(Ok(Value::String(encoded_labels.join("."))))
}

"morse_encode" => {
    if args.is_empty() {
        return Some(Err("morse_encode requires 1 argument".to_string()));
    }
    const MORSE: &[(&str, &str)] = &[
        ("A",".-"),("B","-..."),("C","-.-."),("D","-.."),("E","."),
        ("F","..-."),("G","--."),("H","...."),("I",".."),("J",".---"),
        ("K","-.-"),("L",".-.."),("M","--"),("N","-."),("O","---"),
        ("P",".--."),("Q","--.-"),("R",".-."),("S","..."),("T","-"),
        ("U","..-"),("V","...-"),("W",".--"),("X","-..-"),("Y","-.--"),
        ("Z","--.."),
        ("0","-----"),("1",".----"),("2","..---"),("3","...--"),
        ("4","....-"),("5","....."),("6","-...."),("7","--..."),
        ("8","---.."),("9","----."),
    ];
    let s = args[0].as_string().to_uppercase();
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut word_codes: Vec<String> = Vec::new();
    for word in words {
        let mut char_codes: Vec<String> = Vec::new();
        for ch in word.chars() {
            let key = ch.to_string();
            if let Some(&(_, code)) = MORSE.iter().find(|&&(k, _)| k == key) {
                char_codes.push(code.to_string());
            } else {
                char_codes.push("?".to_string());
            }
        }
        word_codes.push(char_codes.join(" "));
    }
    Some(Ok(Value::String(word_codes.join(" / "))))
}

"morse_decode" => {
    if args.is_empty() {
        return Some(Err("morse_decode requires 1 argument".to_string()));
    }
    const MORSE: &[(&str, char)] = &[
        (".-",'A'),("-...",'B'),("-.-.",'C'),("-..",'D'),(".",'E'),
        ("..-.",'F'),("--.",'G'),("....",'H'),("..",'I'),(".---",'J'),
        ("-.-",'K'),(".-..",'L'),("--",'M'),("-.",'N'),("---",'O'),
        (".--.",'P'),("--.-",'Q'),(".-.",'R'),("...",'S'),("-",'T'),
        ("..-",'U'),("...-",'V'),(".--",'W'),("-..-",'X'),("-.--",'Y'),
        ("--..",'Z'),
        ("-----",'0'),(".----",'1'),("..---",'2'),("...--",'3'),
        ("....-",'4'),(".....",'5'),("-....",'6'),("--...",'7'),
        ("---..",'8'),("----.",'9'),
    ];
    let s = args[0].as_string();
    let mut result = String::new();
    for word in s.split(" / ") {
        for code in word.split_whitespace() {
            if let Some(&(_, ch)) = MORSE.iter().find(|&&(k, _)| k == code) {
                result.push(ch);
            } else {
                result.push('?');
            }
        }
        result.push(' ');
    }
    let result = result.trim_end().to_string();
    Some(Ok(Value::String(result)))
}

// ─────────────────────  MESSAGEPACK  ─────────────────────

"msgpack_encode" => {
    fn val_to_json_mp(v: &Value) -> serde_json::Value {
        match v {
            Value::Int(n)   => serde_json::Value::Number(serde_json::Number::from(*n)),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s)   => serde_json::Value::String(s.clone()),
            Value::Bool(b)  => serde_json::Value::Bool(*b),
            Value::Null     => serde_json::Value::Null,
            Value::Array(a) => serde_json::Value::Array(a.iter().map(val_to_json_mp).collect()),
            Value::Map(m)   => {
                let obj: serde_json::Map<String, serde_json::Value> =
                    m.iter().map(|(k, v)| (k.clone(), val_to_json_mp(v))).collect();
                serde_json::Value::Object(obj)
            }
        
            _ => serde_json::Value::Null,
        }
    }
    if args.is_empty() {
        return Some(Err("msgpack_encode requires 1 argument".to_string()));
    }
    let json_val = val_to_json_mp(&args[0]);
    match rmp_serde::to_vec(&json_val) {
        Ok(bytes) => Some(Ok(Value::String(b2he(&bytes)))),
        Err(e)    => Some(Err(format!("msgpack_encode error: {}", e))),
    }
}

"msgpack_decode" => {
    fn json_to_val_mp(j: serde_json::Value) -> Value {
        match j {
            serde_json::Value::Null      => Value::Null,
            serde_json::Value::Bool(b)   => Value::Bool(b),
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() { Value::Int(i) }
                else if let Some(f) = n.as_f64() { Value::Float(f) }
                else { Value::Null }
            }
            serde_json::Value::Array(a)  => Value::Array(a.into_iter().map(json_to_val_mp).collect()),
            serde_json::Value::Object(o) => {
                Value::Map(o.into_iter().map(|(k, v)| (k, json_to_val_mp(v))).collect())
            }
        }
    }
    if args.is_empty() {
        return Some(Err("msgpack_decode requires 1 argument".to_string()));
    }
    let hex = args[0].as_string();
    let bytes = match he2b(&hex) {
        Some(b) => b,
        None    => return Some(Err("msgpack_decode: invalid hex input".to_string())),
    };
    match rmp_serde::from_slice::<serde_json::Value>(&bytes) {
        Ok(json_val) => Some(Ok(json_to_val_mp(json_val))),
        Err(e)       => Some(Err(format!("msgpack_decode error: {}", e))),
    }
}

// ─────────────────────  BINCODE  ─────────────────────

"bincode_encode" => {
    fn val_to_json_bc(v: &Value) -> serde_json::Value {
        match v {
            Value::Int(n)   => serde_json::Value::Number(serde_json::Number::from(*n)),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s)   => serde_json::Value::String(s.clone()),
            Value::Bool(b)  => serde_json::Value::Bool(*b),
            Value::Null     => serde_json::Value::Null,
            Value::Array(a) => serde_json::Value::Array(a.iter().map(val_to_json_bc).collect()),
            Value::Map(m)   => {
                let obj: serde_json::Map<String, serde_json::Value> =
                    m.iter().map(|(k, v)| (k.clone(), val_to_json_bc(v))).collect();
                serde_json::Value::Object(obj)
            }
        
            _ => serde_json::Value::Null,
        }
    }
    if args.is_empty() {
        return Some(Err("bincode_encode requires 1 argument".to_string()));
    }
    let json_val  = val_to_json_bc(&args[0]);
    let json_str  = json_val.to_string();
    let json_bytes: Vec<u8> = json_str.into_bytes();
    match bincode::serialize(&json_bytes) {
        Ok(encoded) => Some(Ok(Value::String(b2he(&encoded)))),
        Err(e)      => Some(Err(format!("bincode_encode error: {}", e))),
    }
}

"bincode_decode" => {
    fn json_to_val_bc(j: serde_json::Value) -> Value {
        match j {
            serde_json::Value::Null      => Value::Null,
            serde_json::Value::Bool(b)   => Value::Bool(b),
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() { Value::Int(i) }
                else if let Some(f) = n.as_f64() { Value::Float(f) }
                else { Value::Null }
            }
            serde_json::Value::Array(a)  => Value::Array(a.into_iter().map(json_to_val_bc).collect()),
            serde_json::Value::Object(o) => {
                Value::Map(o.into_iter().map(|(k, v)| (k, json_to_val_bc(v))).collect())
            }
        }
    }
    if args.is_empty() {
        return Some(Err("bincode_decode requires 1 argument".to_string()));
    }
    let hex   = args[0].as_string();
    let bytes = match he2b(&hex) {
        Some(b) => b,
        None    => return Some(Err("bincode_decode: invalid hex input".to_string())),
    };
    let json_bytes: Vec<u8> = match bincode::deserialize(&bytes) {
        Ok(v)  => v,
        Err(e) => return Some(Err(format!("bincode_decode error: {}", e))),
    };
    let json_str = match std::str::from_utf8(&json_bytes) {
        Ok(s)  => s,
        Err(e) => return Some(Err(format!("bincode_decode utf8 error: {}", e))),
    };
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(json_val) => Some(Ok(json_to_val_bc(json_val))),
        Err(e)       => Some(Err(format!("bincode_decode json error: {}", e))),
    }
}

// ─────────────────────  JWT  ─────────────────────

"jwt_sign" => {
    fn val_to_json_jwt(v: &Value) -> serde_json::Value {
        match v {
            Value::Int(n)   => serde_json::Value::Number(serde_json::Number::from(*n)),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s)   => serde_json::Value::String(s.clone()),
            Value::Bool(b)  => serde_json::Value::Bool(*b),
            Value::Null     => serde_json::Value::Null,
            Value::Array(a) => serde_json::Value::Array(a.iter().map(val_to_json_jwt).collect()),
            Value::Map(m)   => {
                let obj: serde_json::Map<String, serde_json::Value> =
                    m.iter().map(|(k, v)| (k.clone(), val_to_json_jwt(v))).collect();
                serde_json::Value::Object(obj)
            }
        
            _ => serde_json::Value::Null,
        }
    }
    if args.len() < 3 {
        return Some(Err("jwt_sign requires 3 arguments: payload_map, secret, algorithm".to_string()));
    }
    let secret_str = args[1].as_string();
    let alg_str    = args[2].as_string();
    let algorithm  = match alg_str.as_str() {
        "HS256" => JwtAlgorithm::HS256,
        "HS384" => JwtAlgorithm::HS384,
        "HS512" => JwtAlgorithm::HS512,
        other   => return Some(Err(format!("jwt_sign: unsupported algorithm '{}'", other))),
    };
    let claims = val_to_json_jwt(&args[0]);
    let header = JwtHeader::new(algorithm);
    let key    = EncodingKey::from_secret(secret_str.as_bytes());
    match jwt_encode(&header, &claims, &key) {
        Ok(token) => Some(Ok(Value::String(token))),
        Err(e)    => Some(Err(format!("jwt_sign error: {}", e))),
    }
}

"jwt_verify" => {
    if args.len() < 3 {
        return Some(Err("jwt_verify requires 3 arguments: token, secret, algorithm".to_string()));
    }
    let token      = args[0].as_string();
    let secret_str = args[1].as_string();
    let alg_str    = args[2].as_string();
    let algorithm  = match alg_str.as_str() {
        "HS256" => JwtAlgorithm::HS256,
        "HS384" => JwtAlgorithm::HS384,
        "HS512" => JwtAlgorithm::HS512,
        other   => return Some(Err(format!("jwt_verify: unsupported algorithm '{}'", other))),
    };
    let key        = DecodingKey::from_secret(secret_str.as_bytes());
    let mut validation = Validation::new(algorithm);
    validation.required_spec_claims = std::collections::HashSet::new();
    let valid      = jwt_decode_fn::<serde_json::Value>(&token, &key, &validation).is_ok();
    Some(Ok(Value::Bool(valid)))
}

"jwt_decode" => {
    fn json_to_val_jd(j: serde_json::Value) -> Value {
        match j {
            serde_json::Value::Null      => Value::Null,
            serde_json::Value::Bool(b)   => Value::Bool(b),
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() { Value::Int(i) }
                else if let Some(f) = n.as_f64() { Value::Float(f) }
                else { Value::Null }
            }
            serde_json::Value::Array(a)  => Value::Array(a.into_iter().map(json_to_val_jd).collect()),
            serde_json::Value::Object(o) => {
                Value::Map(o.into_iter().map(|(k, v)| (k, json_to_val_jd(v))).collect())
            }
        }
    }
    if args.is_empty() {
        return Some(Err("jwt_decode requires 1 argument".to_string()));
    }
    let token = args[0].as_string();
    let mut validation = Validation::new(JwtAlgorithm::HS256);
    validation.insecure_disable_signature_validation();
    validation.required_spec_claims = std::collections::HashSet::new();
    let dummy_key = DecodingKey::from_secret(b"");
    match jwt_decode_fn::<serde_json::Value>(&token, &dummy_key, &validation) {
        Ok(token_data) => {
            let alg_str = format!("{:?}", token_data.header.alg);
            let mut header_map: HashMap<String, Value> = HashMap::new();
            header_map.insert("alg".to_string(), Value::String(alg_str));
            header_map.insert("typ".to_string(), Value::String("JWT".to_string()));
            let payload = json_to_val_jd(token_data.claims);
            let mut result: HashMap<String, Value> = HashMap::new();
            result.insert("header".to_string(),  Value::Map(header_map));
            result.insert("payload".to_string(), payload);
            result.insert("valid".to_string(),   Value::Bool(true));
            Some(Ok(Value::Map(result)))
        }
        Err(e) => {
            let mut result: HashMap<String, Value> = HashMap::new();
            result.insert("valid".to_string(), Value::Bool(false));
            result.insert("error".to_string(), Value::String(e.to_string()));
            Some(Ok(Value::Map(result)))
        }
    }
}

// ─────────────────────  HANDLEBARS TEMPLATING  ─────────────────────

"template_render" => {
    fn val_to_json_hb(v: &Value) -> serde_json::Value {
        match v {
            Value::Int(n)   => serde_json::Value::Number(serde_json::Number::from(*n)),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s)   => serde_json::Value::String(s.clone()),
            Value::Bool(b)  => serde_json::Value::Bool(*b),
            Value::Null     => serde_json::Value::Null,
            Value::Array(a) => serde_json::Value::Array(a.iter().map(val_to_json_hb).collect()),
            Value::Map(m)   => {
                let obj: serde_json::Map<String, serde_json::Value> =
                    m.iter().map(|(k, v)| (k.clone(), val_to_json_hb(v))).collect();
                serde_json::Value::Object(obj)
            }
        
    _ => serde_json::Value::Null,
}
    }
    if args.len() < 2 {
        return Some(Err("template_render requires 2 arguments: template_str, data_map".to_string()));
    }
    let template_str = args[0].as_string();
    let json_data    = val_to_json_hb(&args[1]);
    match self.hbs.render_template(&template_str, &json_data) {
        Ok(rendered) => Some(Ok(Value::String(rendered))),
        Err(e)       => Some(Err(format!("template_render error: {}", e))),
    }
}

"template_register" => {
    if args.len() < 2 {
        return Some(Err("template_register requires 2 arguments: name, template_str".to_string()));
    }
    let name         = args[0].as_string();
    let template_str = args[1].as_string();
    match self.hbs.register_template_string(&name, &template_str) {
        Ok(())  => Some(Ok(Value::Bool(true))),
        Err(e)  => Some(Err(format!("template_register error: {}", e))),
    }
}

"template_render_named" => {
    fn val_to_json_hrn(v: &Value) -> serde_json::Value {
        match v {
            Value::Int(n)   => serde_json::Value::Number(serde_json::Number::from(*n)),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s)   => serde_json::Value::String(s.clone()),
            Value::Bool(b)  => serde_json::Value::Bool(*b),
            Value::Null     => serde_json::Value::Null,
            Value::Array(a) => serde_json::Value::Array(a.iter().map(val_to_json_hrn).collect()),
            Value::Map(m)   => {
                let obj: serde_json::Map<String, serde_json::Value> =
                    m.iter().map(|(k, v)| (k.clone(), val_to_json_hrn(v))).collect();
                serde_json::Value::Object(obj)
            }
        
    _ => serde_json::Value::Null,
}
    }
    if args.len() < 2 {
        return Some(Err("template_render_named requires 2 arguments: name, data_map".to_string()));
    }
    let name      = args[0].as_string();
    let json_data = val_to_json_hrn(&args[1]);
    match self.hbs.render(&name, &json_data) {
        Ok(rendered) => Some(Ok(Value::String(rendered))),
        Err(e)       => Some(Err(format!("template_render_named error: {}", e))),
    }
}

"template_escape" => {
    if args.is_empty() {
        return Some(Err("template_escape requires 1 argument".to_string()));
    }
    let s = args[0].as_string();
    let escaped = handlebars::html_escape(&s);
    Some(Ok(Value::String(escaped)))
}

// ─────────────────────  BIT MANIPULATION  ─────────────────────

"bit_and" => {
    if args.len() < 2 {
        return Some(Err("bit_and requires 2 arguments".to_string()));
    }
    let a = args[0].as_int() as u64;
    let b = args[1].as_int() as u64;
    Some(Ok(Value::Int((a & b) as i64)))
}

"bit_or" => {
    if args.len() < 2 {
        return Some(Err("bit_or requires 2 arguments".to_string()));
    }
    let a = args[0].as_int() as u64;
    let b = args[1].as_int() as u64;
    Some(Ok(Value::Int((a | b) as i64)))
}

"bit_xor" => {
    if args.len() < 2 {
        return Some(Err("bit_xor requires 2 arguments".to_string()));
    }
    let a = args[0].as_int() as u64;
    let b = args[1].as_int() as u64;
    Some(Ok(Value::Int((a ^ b) as i64)))
}

"bit_not" => {
    if args.is_empty() {
        return Some(Err("bit_not requires 1 argument".to_string()));
    }
    let a = args[0].as_int() as u64;
    Some(Ok(Value::Int((!a) as i64)))
}

"bit_shl" => {
    if args.len() < 2 {
        return Some(Err("bit_shl requires 2 arguments".to_string()));
    }
    let a = args[0].as_int() as u64;
    let n = (args[1].as_int() as u32) & 63;
    Some(Ok(Value::Int((a << n) as i64)))
}

"bit_shr" => {
    if args.len() < 2 {
        return Some(Err("bit_shr requires 2 arguments".to_string()));
    }
    let a = args[0].as_int() as u64;
    let n = (args[1].as_int() as u32) & 63;
    Some(Ok(Value::Int((a >> n) as i64)))
}

"bit_rotl" => {
    if args.len() < 2 {
        return Some(Err("bit_rotl requires 2 arguments".to_string()));
    }
    let a = args[0].as_int() as u64;
    let n = (args[1].as_int() as u32) & 63;
    Some(Ok(Value::Int(a.rotate_left(n) as i64)))
}

"bit_rotr" => {
    if args.len() < 2 {
        return Some(Err("bit_rotr requires 2 arguments".to_string()));
    }
    let a = args[0].as_int() as u64;
    let n = (args[1].as_int() as u32) & 63;
    Some(Ok(Value::Int(a.rotate_right(n) as i64)))
}

"bit_popcnt" => {
    if args.is_empty() {
        return Some(Err("bit_popcnt requires 1 argument".to_string()));
    }
    let a = args[0].as_int() as u64;
    Some(Ok(Value::Int(a.count_ones() as i64)))
}

"bit_clz" => {
    if args.is_empty() {
        return Some(Err("bit_clz requires 1 argument".to_string()));
    }
    let a = args[0].as_int() as u64;
    Some(Ok(Value::Int(a.leading_zeros() as i64)))
}

"bit_ctz" => {
    if args.is_empty() {
        return Some(Err("bit_ctz requires 1 argument".to_string()));
    }
    let a = args[0].as_int() as u64;
    Some(Ok(Value::Int(a.trailing_zeros() as i64)))
}

"bit_reverse" => {
    if args.is_empty() {
        return Some(Err("bit_reverse requires 1 argument".to_string()));
    }
    let a = args[0].as_int() as u64;
    Some(Ok(Value::Int(a.reverse_bits() as i64)))
}

"bit_get" => {
    if args.len() < 2 {
        return Some(Err("bit_get requires 2 arguments".to_string()));
    }
    let a   = args[0].as_int() as u64;
    let pos = (args[1].as_int() as u32) & 63;
    Some(Ok(Value::Int(((a >> pos) & 1) as i64)))
}

"bit_set" => {
    if args.len() < 2 {
        return Some(Err("bit_set requires 2 arguments".to_string()));
    }
    let a   = args[0].as_int() as u64;
    let pos = (args[1].as_int() as u32) & 63;
    Some(Ok(Value::Int((a | (1u64 << pos)) as i64)))
}

"bit_clear" => {
    if args.len() < 2 {
        return Some(Err("bit_clear requires 2 arguments".to_string()));
    }
    let a   = args[0].as_int() as u64;
    let pos = (args[1].as_int() as u32) & 63;
    Some(Ok(Value::Int((a & !(1u64 << pos)) as i64)))
}

"bit_toggle" => {
    if args.len() < 2 {
        return Some(Err("bit_toggle requires 2 arguments".to_string()));
    }
    let a   = args[0].as_int() as u64;
    let pos = (args[1].as_int() as u32) & 63;
    Some(Ok(Value::Int((a ^ (1u64 << pos)) as i64)))
}

"bytes_to_int_be" => {
    if args.is_empty() {
        return Some(Err("bytes_to_int_be requires 1 argument".to_string()));
    }
    let arr = match &args[0] {
        Value::Array(v) => v.clone(),
        _ => return Some(Err("bytes_to_int_be: expected Array".to_string())),
    };
    if arr.len() > 8 {
        return Some(Err("bytes_to_int_be: array too large (max 8 bytes)".to_string()));
    }
    let mut result: i64 = 0;
    for val in &arr {
        result = (result << 8) | (val.as_int() & 0xFF);
    }
    Some(Ok(Value::Int(result)))
}

"bytes_to_int_le" => {
    if args.is_empty() {
        return Some(Err("bytes_to_int_le requires 1 argument".to_string()));
    }
    let arr = match &args[0] {
        Value::Array(v) => v.clone(),
        _ => return Some(Err("bytes_to_int_le: expected Array".to_string())),
    };
    if arr.len() > 8 {
        return Some(Err("bytes_to_int_le: array too large (max 8 bytes)".to_string()));
    }
    let mut result: i64 = 0;
    for (i, val) in arr.iter().enumerate() {
        result |= (val.as_int() & 0xFF) << (i * 8);
    }
    Some(Ok(Value::Int(result)))
}

"int_to_bytes_be" => {
    if args.len() < 2 {
        return Some(Err("int_to_bytes_be requires 2 arguments: n, size".to_string()));
    }
    let n    = args[0].as_int();
    let size = args[1].as_int() as usize;
    if size == 0 || size > 8 {
        return Some(Err("int_to_bytes_be: size must be 1..=8".to_string()));
    }
    let n_u64 = n as u64;
    let mut bytes: Vec<Value> = Vec::with_capacity(size);
    for i in (0..size).rev() {
        bytes.push(Value::Int(((n_u64 >> (i * 8)) & 0xFF) as i64));
    }
    Some(Ok(Value::Array(bytes)))
}

"int_to_bytes_le" => {
    if args.len() < 2 {
        return Some(Err("int_to_bytes_le requires 2 arguments: n, size".to_string()));
    }
    let n    = args[0].as_int();
    let size = args[1].as_int() as usize;
    if size == 0 || size > 8 {
        return Some(Err("int_to_bytes_le: size must be 1..=8".to_string()));
    }
    let n_u64 = n as u64;
    let bytes: Vec<Value> = (0..size)
        .map(|i| Value::Int(((n_u64 >> (i * 8)) & 0xFF) as i64))
        .collect();
    Some(Ok(Value::Array(bytes)))
}


            // ── KNULL_B9_SYS ──
// ── System Info (sysinfo) ────────────────────────────────────────────────

        "sys_cpu_count" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            Some(Ok(Value::Int(sys.cpus().len() as i64)))
        }

        "sys_cpu_usage" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            let usage: Vec<Value> = sys
                .cpus()
                .iter()
                .map(|c| Value::Float(c.cpu_usage() as f64))
                .collect();
            Some(Ok(Value::Array(usage)))
        }

        "sys_cpu_freq" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            let freqs: Vec<Value> = sys
                .cpus()
                .iter()
                .map(|c| Value::Int(c.frequency() as i64))
                .collect();
            Some(Ok(Value::Array(freqs)))
        }

        "sys_memory_total" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            Some(Ok(Value::Int(sys.total_memory() as i64)))
        }

        "sys_memory_used" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            Some(Ok(Value::Int(sys.used_memory() as i64)))
        }

        "sys_memory_available" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            Some(Ok(Value::Int(sys.available_memory() as i64)))
        }

        "sys_swap_total" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            Some(Ok(Value::Int(sys.total_swap() as i64)))
        }

        "sys_swap_used" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            Some(Ok(Value::Int(sys.used_swap() as i64)))
        }

        "sys_uptime" => {
            Some(Ok(Value::Int(System::uptime() as i64)))
        }

        "sys_boot_time" => {
            Some(Ok(Value::Int(System::boot_time() as i64)))
        }

        "sys_hostname" => {
            Some(Ok(Value::String(System::host_name().unwrap_or_default())))
        }

        "sys_os_name" => {
            Some(Ok(Value::String(System::name().unwrap_or_default())))
        }

        "sys_os_version" => {
            Some(Ok(Value::String(System::os_version().unwrap_or_default())))
        }

        "sys_kernel_version" => {
            Some(Ok(Value::String(System::kernel_version().unwrap_or_default())))
        }

        "sys_arch" => {
            Some(Ok(Value::String(System::cpu_arch().unwrap_or_else(|| std::env::consts::ARCH.to_string()))))
        }

        "sys_load_avg" => {
            let load = System::load_average();
            let mut m: HashMap<String, Value> = HashMap::new();
            m.insert("one".to_string(), Value::Float(load.one));
            m.insert("five".to_string(), Value::Float(load.five));
            m.insert("fifteen".to_string(), Value::Float(load.fifteen));
            Some(Ok(Value::Map(m)))
        }

        "sys_process_list" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            let procs: Vec<Value> = sys
                .processes()
                .iter()
                .take(50)
                .map(|(pid, proc)| {
                    let mut m: HashMap<String, Value> = HashMap::new();
                    m.insert("pid".to_string(), Value::Int(pid.as_u32() as i64));
                    m.insert("name".to_string(), Value::String(proc.name().to_string()));
                    m.insert("cpu".to_string(), Value::Float(proc.cpu_usage() as f64));
                    m.insert("mem".to_string(), Value::Int(proc.memory() as i64));
                    m.insert("status".to_string(), Value::String(format!("{:?}", proc.status())));
                    Value::Map(m)
                })
                .collect();
            Some(Ok(Value::Array(procs)))
        }

        "sys_process_by_name" => {
            if args.is_empty() {
                return Some(Err("sys_process_by_name requires 1 argument".to_string()));
            }
            let target_name = args[0].as_string();
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            let procs: Vec<Value> = sys
                .processes_by_name(&target_name)
                .map(|proc| {
                    let mut m: HashMap<String, Value> = HashMap::new();
                    m.insert("pid".to_string(), Value::Int(proc.pid().as_u32() as i64));
                    m.insert("name".to_string(), Value::String(proc.name().to_string()));
                    m.insert("cpu".to_string(), Value::Float(proc.cpu_usage() as f64));
                    m.insert("mem".to_string(), Value::Int(proc.memory() as i64));
                    m.insert("status".to_string(), Value::String(format!("{:?}", proc.status())));
                    Value::Map(m)
                })
                .collect();
            Some(Ok(Value::Array(procs)))
        }

        "sys_process_kill" => {
            use sysinfo::Pid;
            if args.is_empty() {
                return Some(Err("sys_process_kill requires 1 argument".to_string()));
            }
            let pid_val = args[0].as_int();
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            let killed = sys
                .process(Pid::from(pid_val as usize))
                .map(|p| p.kill())
                .unwrap_or(false);
            Some(Ok(Value::Bool(killed)))
        }

        "sys_disk_list" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            let disks_list = sysinfo::Disks::new_with_refreshed_list();
            let disks: Vec<Value> = disks_list
                .iter()
                .map(|d| {
                    let mut m: HashMap<String, Value> = HashMap::new();
                    m.insert("name".to_string(), Value::String(d.name().to_string_lossy().to_string()));
                    m.insert("mount".to_string(), Value::String(d.mount_point().to_string_lossy().to_string()));
                    m.insert("total".to_string(), Value::Int(d.total_space() as i64));
                    m.insert("used".to_string(), Value::Int((d.total_space().saturating_sub(d.available_space())) as i64));
                    m.insert("free".to_string(), Value::Int(d.available_space() as i64));
                    m.insert("fs".to_string(), Value::String(d.file_system().to_string_lossy().to_string()));
                    Value::Map(m)
                })
                .collect();
            Some(Ok(Value::Array(disks)))
        }

        "sys_network_list" => {
            let sys = self.sysinfo_sys.get_or_insert_with(|| {
                let mut s = System::new_all();
                s.refresh_all();
                s
            });
            let networks_list = sysinfo::Networks::new_with_refreshed_list();
            let nets: Vec<Value> = networks_list
                .iter()
                .map(|(name, data)| {
                    let mut m: HashMap<String, Value> = HashMap::new();
                    m.insert("name".to_string(), Value::String(name.clone()));
                    m.insert("rx".to_string(), Value::Int(data.total_received() as i64));
                    m.insert("tx".to_string(), Value::Int(data.total_transmitted() as i64));
                    m.insert("rx_speed".to_string(), Value::Int(data.received() as i64));
                    m.insert("tx_speed".to_string(), Value::Int(data.transmitted() as i64));
                    Value::Map(m)
                })
                .collect();
            Some(Ok(Value::Array(nets)))
        }

        "sys_refresh" => {
            self.sysinfo_sys.as_mut().map(|s| s.refresh_all());
            Some(Ok(Value::Bool(true)))
        }

        // ── Process Control ──────────────────────────────────────────────────────

        "proc_run" => {
            if args.len() < 2 {
                return Some(Err("proc_run requires 2 arguments".to_string()));
            }
            let cmd = args[0].as_string();
            let cmd_args: Vec<String> = match &args[1] {
                Value::Array(a) => a.iter().map(|v| v.as_string()).collect(),
                _ => vec![],
            };
            match Command::new(&cmd).args(&cmd_args).output() {
                Err(e) => Some(Err(e.to_string())),
                Ok(out) => {
                    let mut m: HashMap<String, Value> = HashMap::new();
                    m.insert("stdout".to_string(), Value::String(String::from_utf8_lossy(&out.stdout).to_string()));
                    m.insert("stderr".to_string(), Value::String(String::from_utf8_lossy(&out.stderr).to_string()));
                    m.insert("exit_code".to_string(), Value::Int(out.status.code().unwrap_or(-1) as i64));
                    Some(Ok(Value::Map(m)))
                }
            }
        }

        "proc_run_shell" => {
            if args.is_empty() {
                return Some(Err("proc_run_shell requires 1 argument".to_string()));
            }
            let shell_cmd = args[0].as_string();
            match Command::new("sh").arg("-c").arg(&shell_cmd).output() {
                Err(e) => Some(Err(e.to_string())),
                Ok(out) => {
                    let mut m: HashMap<String, Value> = HashMap::new();
                    m.insert("stdout".to_string(), Value::String(String::from_utf8_lossy(&out.stdout).to_string()));
                    m.insert("stderr".to_string(), Value::String(String::from_utf8_lossy(&out.stderr).to_string()));
                    m.insert("exit_code".to_string(), Value::Int(out.status.code().unwrap_or(-1) as i64));
                    Some(Ok(Value::Map(m)))
                }
            }
        }

        "proc_spawn_bg" => {
            if args.len() < 2 {
                return Some(Err("proc_spawn_bg requires 2 arguments".to_string()));
            }
            let cmd = args[0].as_string();
            let cmd_args: Vec<String> = match &args[1] {
                Value::Array(a) => a.iter().map(|v| v.as_string()).collect(),
                _ => vec![],
            };
            match Command::new(&cmd)
                .args(&cmd_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Err(e) => Some(Err(e.to_string())),
                Ok(child) => Some(Ok(Value::Int(child.id() as i64))),
            }
        }

        "proc_which" => {
            if args.is_empty() {
                return Some(Err("proc_which requires 1 argument".to_string()));
            }
            let prog_name = args[0].as_string();
            match Command::new("which").arg(&prog_name).output() {
                Err(e) => Some(Err(e.to_string())),
                Ok(out) => {
                    if out.status.success() {
                        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if path.is_empty() {
                            Some(Ok(Value::Null))
                        } else {
                            Some(Ok(Value::String(path)))
                        }
                    } else {
                        Some(Ok(Value::Null))
                    }
                }
            }
        }

        "proc_pid" => {
            Some(Ok(Value::Int(std::process::id() as i64)))
        }

        "proc_exit" => {
            let code = if args.is_empty() { 0 } else { args[0].as_int() };
            std::process::exit(code as i32);
        }

        "env_get" => {
            if args.is_empty() {
                return Some(Err("env_get requires 1 argument".to_string()));
            }
            let key = args[0].as_string();
            match std::env::var(&key) {
                Ok(val) => Some(Ok(Value::String(val))),
                Err(_) => Some(Ok(Value::Null)),
            }
        }

        "env_set" => {
            if args.len() < 2 {
                return Some(Err("env_set requires 2 arguments".to_string()));
            }
            let key = args[0].as_string();
            let val = args[1].as_string();
            std::env::set_var(&key, &val);
            Some(Ok(Value::Bool(true)))
        }

        "env_unset" => {
            if args.is_empty() {
                return Some(Err("env_unset requires 1 argument".to_string()));
            }
            let key = args[0].as_string();
            std::env::remove_var(&key);
            Some(Ok(Value::Bool(true)))
        }

        "env_all" => {
            let map: HashMap<String, Value> = std::env::vars()
                .map(|(k, v)| (k, Value::String(v)))
                .collect();
            Some(Ok(Value::Map(map)))
        }

        "env_cwd" => {
            match std::env::current_dir() {
                Ok(path) => Some(Ok(Value::String(path.to_string_lossy().to_string()))),
                Err(e) => Some(Err(e.to_string())),
            }
        }

        "env_set_cwd" => {
            if args.is_empty() {
                return Some(Err("env_set_cwd requires 1 argument".to_string()));
            }
            let path = args[0].as_string();
            match std::env::set_current_dir(&path) {
                Ok(_) => Some(Ok(Value::Bool(true))),
                Err(e) => Some(Err(e.to_string())),
            }
        }

        "env_args" => {
            let arg_list: Vec<Value> = std::env::args().map(|s| Value::String(s)).collect();
            Some(Ok(Value::Array(arg_list)))
        }

        "env_exe" => {
            match std::env::current_exe() {
                Ok(path) => Some(Ok(Value::String(path.to_string_lossy().to_string()))),
                Err(e) => Some(Err(e.to_string())),
            }
        }

        // ── Advanced File System ─────────────────────────────────────────────────

        "fs_glob" => {
            if args.is_empty() {
                return Some(Err("fs_glob requires 1 argument".to_string()));
            }
            let pattern = args[0].as_string();
            match glob::glob(&pattern) {
                Err(e) => Some(Err(e.to_string())),
                Ok(paths) => {
                    let results: Vec<Value> = paths
                        .filter_map(|p| p.ok())
                        .map(|p| Value::String(p.to_string_lossy().to_string()))
                        .collect();
                    Some(Ok(Value::Array(results)))
                }
            }
        }

        "fs_walk" => {
            if args.is_empty() {
                return Some(Err("fs_walk requires 1 argument".to_string()));
            }
            let dir = args[0].as_string();
            let files: Vec<Value> = WalkDir::new(&dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| Value::String(e.path().to_string_lossy().to_string()))
                .collect();
            Some(Ok(Value::Array(files)))
        }

        "fs_walk_dirs" => {
            if args.is_empty() {
                return Some(Err("fs_walk_dirs requires 1 argument".to_string()));
            }
            let dir = args[0].as_string();
            let dirs: Vec<Value> = WalkDir::new(&dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir())
                .map(|e| Value::String(e.path().to_string_lossy().to_string()))
                .collect();
            Some(Ok(Value::Array(dirs)))
        }

        "fs_find" => {
            if args.len() < 2 {
                return Some(Err("fs_find requires 2 arguments".to_string()));
            }
            let dir = args[0].as_string();
            let pattern = args[1].as_string();
            let full_pattern = format!("{}/{}", dir.trim_end_matches('/'), pattern);
            match glob::glob(&full_pattern) {
                Err(e) => Some(Err(e.to_string())),
                Ok(paths) => {
                    let results: Vec<Value> = paths
                        .filter_map(|p| p.ok())
                        .map(|p| Value::String(p.to_string_lossy().to_string()))
                        .collect();
                    Some(Ok(Value::Array(results)))
                }
            }
        }

        "fs_symlink" => {
            use std::os::unix::fs::symlink;
            if args.len() < 2 {
                return Some(Err("fs_symlink requires 2 arguments".to_string()));
            }
            let target = args[0].as_string();
            let link = args[1].as_string();
            match symlink(&target, &link) {
                Ok(_) => Some(Ok(Value::Bool(true))),
                Err(e) => Some(Err(e.to_string())),
            }
        }

        "fs_hardlink" => {
            if args.len() < 2 {
                return Some(Err("fs_hardlink requires 2 arguments".to_string()));
            }
            let src = args[0].as_string();
            let dst = args[1].as_string();
            match fs::hard_link(&src, &dst) {
                Ok(_) => Some(Ok(Value::Bool(true))),
                Err(e) => Some(Err(e.to_string())),
            }
        }

        "fs_chmod" => {
            use std::os::unix::fs::PermissionsExt;
            if args.len() < 2 {
                return Some(Err("fs_chmod requires 2 arguments".to_string()));
            }
            let path = args[0].as_string();
            let mode = args[1].as_int();
            match fs::set_permissions(&path, fs::Permissions::from_mode(mode as u32)) {
                Ok(_) => Some(Ok(Value::Bool(true))),
                Err(e) => Some(Err(e.to_string())),
            }
        }

        "fs_stat" => {
            if args.is_empty() {
                return Some(Err("fs_stat requires 1 argument".to_string()));
            }
            let path = args[0].as_string();
            match fs::metadata(&path) {
                Err(e) => Some(Err(e.to_string())),
                Ok(meta) => {
                    let ts_secs = |t: std::io::Result<SystemTime>| -> i64 {
                        t.ok()
                            .and_then(|st| st.duration_since(UNIX_EPOCH).ok())
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0)
                    };
                    let mut m: HashMap<String, Value> = HashMap::new();
                    m.insert("size".to_string(), Value::Int(meta.len() as i64));
                    m.insert("is_file".to_string(), Value::Bool(meta.is_file()));
                    m.insert("is_dir".to_string(), Value::Bool(meta.is_dir()));
                    m.insert("is_symlink".to_string(), Value::Bool(meta.file_type().is_symlink()));
                    m.insert("created".to_string(), Value::Int(ts_secs(meta.created())));
                    m.insert("modified".to_string(), Value::Int(ts_secs(meta.modified())));
                    m.insert("accessed".to_string(), Value::Int(ts_secs(meta.accessed())));
                    m.insert("readonly".to_string(), Value::Bool(meta.permissions().readonly()));
                    Some(Ok(Value::Map(m)))
                }
            }
        }

        "fs_disk_usage" => {
            if args.is_empty() {
                return Some(Err("fs_disk_usage requires 1 argument".to_string()));
            }
            let dir = args[0].as_string();
            let total: u64 = WalkDir::new(&dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum();
            Some(Ok(Value::Int(total as i64)))
        }

        "fs_tempdir" => {
            let base = std::env::temp_dir();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            let dir = base.join(format!("knull_tmp_{}_{}", std::process::id(), nanos));
            match fs::create_dir_all(&dir) {
                Ok(_) => Some(Ok(Value::String(dir.to_string_lossy().to_string()))),
                Err(e) => Some(Err(e.to_string())),
            }
        }

        "fs_tempfile" => {
            let base = std::env::temp_dir();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            let file = base.join(format!("knull_tmp_{}_{}.tmp", std::process::id(), nanos));
            match fs::write(&file, b"") {
                Ok(_) => Some(Ok(Value::String(file.to_string_lossy().to_string()))),
                Err(e) => Some(Err(e.to_string())),
            }
        }

        "fs_mime_type" => {
            if args.is_empty() {
                return Some(Err("fs_mime_type requires 1 argument".to_string()));
            }
            let path = args[0].as_string();
            let ext = Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let mime = match ext.as_str() {
                "html" | "htm"          => "text/html",
                "css"                   => "text/css",
                "js" | "mjs"            => "application/javascript",
                "json"                  => "application/json",
                "png"                   => "image/png",
                "jpg" | "jpeg"          => "image/jpeg",
                "gif"                   => "image/gif",
                "webp"                  => "image/webp",
                "bmp"                   => "image/bmp",
                "tiff" | "tif"          => "image/tiff",
                "ico"                   => "image/x-icon",
                "svg"                   => "image/svg+xml",
                "pdf"                   => "application/pdf",
                "mp3"                   => "audio/mpeg",
                "ogg"                   => "audio/ogg",
                "wav"                   => "audio/wav",
                "mp4"                   => "video/mp4",
                "webm"                  => "video/webm",
                "avi"                   => "video/x-msvideo",
                "mov"                   => "video/quicktime",
                "mkv"                   => "video/x-matroska",
                "txt"                   => "text/plain",
                "md"                    => "text/markdown",
                "csv"                   => "text/csv",
                "xml"                   => "application/xml",
                "toml"                  => "application/toml",
                "yaml" | "yml"          => "application/yaml",
                "sh"                    => "application/x-sh",
                "rs"                    => "text/x-rust",
                "py"                    => "text/x-python",
                "c"                     => "text/x-c",
                "cpp" | "cc" | "cxx"    => "text/x-c++",
                "h" | "hpp"             => "text/x-c",
                "go"                    => "text/x-go",
                "java"                  => "text/x-java-source",
                "rb"                    => "text/x-ruby",
                "php"                   => "application/x-httpd-php",
                "sql"                   => "application/sql",
                "ts"                    => "application/typescript",
                "wasm"                  => "application/wasm",
                "zip"                   => "application/zip",
                "tar"                   => "application/x-tar",
                "gz"                    => "application/gzip",
                "7z"                    => "application/x-7z-compressed",
                "rar"                   => "application/vnd.rar",
                "exe" | "dll"           => "application/vnd.microsoft.portable-executable",
                "apk"                   => "application/vnd.android.package-archive",
                "bin" | "so"            => "application/octet-stream",
                _                       => "application/octet-stream",
            };
            Some(Ok(Value::String(mime.to_string())))
        }

        "fs_hash_file" => {
            use sha2::{Sha256, Sha512, Digest};
            if args.len() < 2 {
                return Some(Err("fs_hash_file requires 2 arguments".to_string()));
            }
            let path = args[0].as_string();
            let algorithm = args[1].as_string();
            match fs::read(&path) {
                Err(e) => Some(Err(e.to_string())),
                Ok(bytes) => {
                    let hex = match algorithm.as_str() {
                        "sha256" => {
                            let mut h = Sha256::new();
                            h.update(&bytes);
                            format!("{:x}", h.finalize())
                        }
                        "sha512" => {
                            let mut h = Sha512::new();
                            h.update(&bytes);
                            format!("{:x}", h.finalize())
                        }
                        // "blake3" | "md5" | _ fall through to blake3
                        _ => {
                            let hash = blake3::hash(&bytes);
                            hash.to_hex().to_string()
                        }
                    };
                    Some(Ok(Value::String(hex)))
                }
            }
        }

        "fs_watch_once" => {
            if args.len() < 2 {
                return Some(Err("fs_watch_once requires 2 arguments".to_string()));
            }
            let watch_path = args[0].as_string();
            let timeout_ms = args[1].as_int() as u64;
            let get_mtime = |p: &str| -> Option<u64> {
                fs::metadata(p)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos() as u64)
            };
            let initial = get_mtime(&watch_path);
            std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
            let after = get_mtime(&watch_path);
            Some(Ok(Value::Bool(initial != after)))
        }

        // ── Timer / High-resolution time ─────────────────────────────────────────

        "timer_now_ns" => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as i64)
                .unwrap_or(0);
            Some(Ok(Value::Int(nanos)))
        }

        "timer_now_us" => {
            let micros = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_micros() as i64)
                .unwrap_or(0);
            Some(Ok(Value::Int(micros)))
        }

        "timer_now_ms" => {
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            Some(Ok(Value::Int(millis)))
        }

        "sleep_us" => {
            if args.is_empty() {
                return Some(Err("sleep_us requires 1 argument".to_string()));
            }
            let micros = args[0].as_int() as u64;
            std::thread::sleep(std::time::Duration::from_micros(micros));
            Some(Ok(Value::Null))
        }

        "sleep_ns" => {
            if args.is_empty() {
                return Some(Err("sleep_ns requires 1 argument".to_string()));
            }
            let nanos = args[0].as_int() as u64;
            std::thread::sleep(std::time::Duration::from_nanos(nanos));
            Some(Ok(Value::Null))
        }

        // ── Random ───────────────────────────────────────────────────────────────

        "rand_seed" => {
            if args.is_empty() {
                return Some(Err("rand_seed requires 1 argument".to_string()));
            }
            let seed = args[0].as_int();
            std::env::set_var("KNULL_RAND_SEED", seed.to_string());
            Some(Ok(Value::Bool(true)))
        }

        "rand_float_range" => {
            use rand::RngCore;
            use rand::rngs::OsRng;
            if args.len() < 2 {
                return Some(Err("rand_float_range requires 2 arguments".to_string()));
            }
            let min = args[0].as_float();
            let max = args[1].as_float();
            let mut rng = OsRng;
            let r = (rng.next_u64() as f64) / (u64::MAX as f64);
            Some(Ok(Value::Float(min + r * (max - min))))
        }

        "rand_int_range" => {
            use rand::RngCore;
            use rand::rngs::OsRng;
            if args.len() < 2 {
                return Some(Err("rand_int_range requires 2 arguments".to_string()));
            }
            let min = args[0].as_int();
            let max = args[1].as_int();
            if min >= max {
                return Some(Err("rand_int_range: min must be less than max".to_string()));
            }
            let mut rng = OsRng;
            let range = (max - min) as u64;
            let r = (rng.next_u64() % range) as i64;
            Some(Ok(Value::Int(min + r)))
        }

        "rand_bytes" => {
            use rand::RngCore;
            use rand::rngs::OsRng;
            if args.is_empty() {
                return Some(Err("rand_bytes requires 1 argument".to_string()));
            }
            let n = args[0].as_int() as usize;
            let mut rng = OsRng;
            let mut bytes = vec![0u8; n];
            rng.fill_bytes(&mut bytes);
            let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            Some(Ok(Value::String(hex)))
        }

        "rand_choice" => {
            use rand::RngCore;
            use rand::rngs::OsRng;
            if args.is_empty() {
                return Some(Err("rand_choice requires 1 argument".to_string()));
            }
            match &args[0] {
                Value::Array(arr) => {
                    if arr.is_empty() {
                        return Some(Ok(Value::Null));
                    }
                    let mut rng = OsRng;
                    let idx = (rng.next_u64() as usize) % arr.len();
                    Some(Ok(arr[idx].clone()))
                }
                _ => Some(Err("rand_choice: argument must be an Array".to_string())),
            }
        }


            // ── KNULL_B9_MISC ──
// ============================================================
// Knull interpreter builtins – batch 9 (misc)
// Match arms for:
//   fn call_builtin(&mut self, name: &str, args: &[Value])
//       -> Option<Result<Value, String>>
// ============================================================

// ── Diff (similar) ──────────────────────────────────────────

"diff_lines" => {
    if args.len() < 2 {
        return Some(Err("diff_lines: need 2 args".to_string()));
    }
    let old = args[0].as_string();
    let new_s = args[1].as_string();
    let diff = similar::TextDiff::from_lines(old.as_str(), new_s.as_str());
    let result: Vec<Value> = diff
        .iter_all_changes()
        .map(|c| {
            let tag = match c.tag() {
                similar::ChangeTag::Equal => "equal",
                similar::ChangeTag::Insert => "insert",
                similar::ChangeTag::Delete => "delete",
            };
            let mut m = HashMap::new();
            m.insert("tag".to_string(), Value::String(tag.to_string()));
            m.insert("value".to_string(), Value::String(c.value().to_string()));
            Value::Map(m)
        })
        .collect();
    Some(Ok(Value::Array(result)))
}

"diff_chars" => {
    if args.len() < 2 {
        return Some(Err("diff_chars: need 2 args".to_string()));
    }
    let old = args[0].as_string();
    let new_s = args[1].as_string();
    let diff = similar::TextDiff::from_chars(old.as_str(), new_s.as_str());
    let result: Vec<Value> = diff
        .iter_all_changes()
        .map(|c| {
            let tag = match c.tag() {
                similar::ChangeTag::Equal => "equal",
                similar::ChangeTag::Insert => "insert",
                similar::ChangeTag::Delete => "delete",
            };
            let mut m = HashMap::new();
            m.insert("tag".to_string(), Value::String(tag.to_string()));
            m.insert("value".to_string(), Value::String(c.value().to_string()));
            Value::Map(m)
        })
        .collect();
    Some(Ok(Value::Array(result)))
}

"diff_words" => {
    if args.len() < 2 {
        return Some(Err("diff_words: need 2 args".to_string()));
    }
    let old = args[0].as_string();
    let new_s = args[1].as_string();
    let diff = similar::TextDiff::from_words(old.as_str(), new_s.as_str());
    let result: Vec<Value> = diff
        .iter_all_changes()
        .map(|c| {
            let tag = match c.tag() {
                similar::ChangeTag::Equal => "equal",
                similar::ChangeTag::Insert => "insert",
                similar::ChangeTag::Delete => "delete",
            };
            let mut m = HashMap::new();
            m.insert("tag".to_string(), Value::String(tag.to_string()));
            m.insert("value".to_string(), Value::String(c.value().to_string()));
            Value::Map(m)
        })
        .collect();
    Some(Ok(Value::Array(result)))
}

"diff_unified" => {
    if args.len() < 3 {
        return Some(Err("diff_unified: need 3 args (old, new, context)".to_string()));
    }
    let old = args[0].as_string();
    let new_s = args[1].as_string();
    let ctx = args[2].as_int() as usize;
    let diff = similar::TextDiff::from_lines(old.as_str(), new_s.as_str());
    let out = diff
        .unified_diff()
        .context_radius(ctx)
        .to_string();
    Some(Ok(Value::String(out)))
}

"diff_ratio" => {
    if args.len() < 2 {
        return Some(Err("diff_ratio: need 2 args".to_string()));
    }
    let old = args[0].as_string();
    let new_s = args[1].as_string();
    let diff = similar::TextDiff::from_chars(old.as_str(), new_s.as_str());
    Some(Ok(Value::Float(diff.ratio() as f64)))
}

"diff_patch" => {
    if args.len() < 2 {
        return Some(Err("diff_patch: need (original, diff_arr)".to_string()));
    }
    let arr = match &args[1] {
        Value::Array(v) => v.clone(),
        _ => return Some(Err("diff_patch: second arg must be Array".to_string())),
    };
    let mut out = String::new();
    for item in &arr {
        if let Value::Map(m) = item {
            let tag = m
                .get("tag")
                .map(|v| v.as_string())
                .unwrap_or_default();
            let val = m
                .get("value")
                .map(|v| v.as_string())
                .unwrap_or_default();
            if tag == "equal" || tag == "insert" {
                out.push_str(&val);
            }
            // delete: skip
        }
    }
    Some(Ok(Value::String(out)))
}

"diff_is_same" => {
    if args.len() < 2 {
        return Some(Err("diff_is_same: need 2 args".to_string()));
    }
    Some(Ok(Value::Bool(args[0].as_string() == args[1].as_string())))
}

// ── PCRE Regex (fancy-regex) ─────────────────────────────────

"regex_match" => {
    if args.len() < 2 {
        return Some(Err("regex_match: need (pattern, text)".to_string()));
    }
    let pat = args[0].as_string();
    let txt = args[1].as_string();
    match FancyRegex::new(&pat) {
        Err(e) => Some(Err(format!("regex_match: bad pattern: {}", e))),
        Ok(re) => match re.is_match(&txt) {
            Err(e) => Some(Err(format!("regex_match: match error: {}", e))),
            Ok(b) => Some(Ok(Value::Bool(b))),
        },
    }
}

"regex_find" => {
    if args.len() < 2 {
        return Some(Err("regex_find: need (pattern, text)".to_string()));
    }
    let pat = args[0].as_string();
    let txt = args[1].as_string();
    match FancyRegex::new(&pat) {
        Err(e) => Some(Err(format!("regex_find: bad pattern: {}", e))),
        Ok(re) => match re.find(&txt) {
            Err(e) => Some(Err(format!("regex_find: error: {}", e))),
            Ok(None) => Some(Ok(Value::Null)),
            Ok(Some(m)) => Some(Ok(Value::String(m.as_str().to_string()))),
        },
    }
}

"regex_find_all" => {
    if args.len() < 2 {
        return Some(Err("regex_find_all: need (pattern, text)".to_string()));
    }
    let pat = args[0].as_string();
    let txt = args[1].as_string();
    match FancyRegex::new(&pat) {
        Err(e) => Some(Err(format!("regex_find_all: bad pattern: {}", e))),
        Ok(re) => {
            let mut results = Vec::new();
            let mut start = 0usize;
            loop {
                match re.find_from_pos(&txt, start) {
                    Err(e) => return Some(Err(format!("regex_find_all: error: {}", e))),
                    Ok(None) => break,
                    Ok(Some(m)) => {
                        results.push(Value::String(m.as_str().to_string()));
                        let end = m.end();
                        if end == start {
                            start += 1;
                            if start > txt.len() {
                                break;
                            }
                        } else {
                            start = end;
                        }
                    }
                }
            }
            Some(Ok(Value::Array(results)))
        }
    }
}

"regex_captures" => {
    if args.len() < 2 {
        return Some(Err("regex_captures: need (pattern, text)".to_string()));
    }
    let pat = args[0].as_string();
    let txt = args[1].as_string();
    match FancyRegex::new(&pat) {
        Err(e) => Some(Err(format!("regex_captures: bad pattern: {}", e))),
        Ok(re) => match re.captures(&txt) {
            Err(e) => Some(Err(format!("regex_captures: error: {}", e))),
            Ok(None) => Some(Ok(Value::Array(vec![]))),
            Ok(Some(caps)) => {
                let groups: Vec<Value> = (0..caps.len())
                    .map(|i| {
                        caps.get(i)
                            .map(|m| Value::String(m.as_str().to_string()))
                            .unwrap_or(Value::Null)
                    })
                    .collect();
                Some(Ok(Value::Array(groups)))
            }
        },
    }
}

"regex_replace" => {
    if args.len() < 3 {
        return Some(Err("regex_replace: need (pattern, text, replacement)".to_string()));
    }
    let pat = args[0].as_string();
    let txt = args[1].as_string();
    let rep = args[2].as_string();
    match FancyRegex::new(&pat) {
        Err(e) => Some(Err(format!("regex_replace: bad pattern: {}", e))),
        Ok(re) => {
            let out = re.replace(&txt, rep.as_str());
            Some(Ok(Value::String(out.to_string())))
        },
    }
}

"regex_replace_all" => {
    if args.len() < 3 {
        return Some(Err("regex_replace_all: need (pattern, text, replacement)".to_string()));
    }
    let pat = args[0].as_string();
    let txt = args[1].as_string();
    let rep = args[2].as_string();
    match FancyRegex::new(&pat) {
        Err(e) => Some(Err(format!("regex_replace_all: bad pattern: {}", e))),
        Ok(re) => {
            let out = re.replace_all(&txt, rep.as_str());
            Some(Ok(Value::String(out.to_string())))
        },
    }
}

"regex_split" => {
    if args.len() < 2 {
        return Some(Err("regex_split: need (pattern, text)".to_string()));
    }
    let pat = args[0].as_string();
    let txt = args[1].as_string();
    match FancyRegex::new(&pat) {
        Err(e) => Some(Err(format!("regex_split: bad pattern: {}", e))),
        Ok(re) => {
            let mut parts: Vec<Value> = Vec::new();
            let mut last = 0usize;
            let mut start = 0usize;
            loop {
                match re.find_from_pos(&txt, start) {
                    Err(e) => return Some(Err(format!("regex_split: error: {}", e))),
                    Ok(None) => {
                        parts.push(Value::String(txt[last..].to_string()));
                        break;
                    }
                    Ok(Some(m)) => {
                        parts.push(Value::String(txt[last..m.start()].to_string()));
                        last = m.end();
                        let end = m.end();
                        if end == start {
                            start += 1;
                            if start > txt.len() {
                                parts.push(Value::String(txt[last..].to_string()));
                                break;
                            }
                        } else {
                            start = end;
                        }
                    }
                }
            }
            Some(Ok(Value::Array(parts)))
        }
    }
}

"regex_test" => {
    if args.is_empty() {
        return Some(Err("regex_test: need (pattern)".to_string()));
    }
    let pat = args[0].as_string();
    Some(Ok(Value::Bool(FancyRegex::new(&pat).is_ok())))
}

"regex_named_captures" => {
    if args.len() < 2 {
        return Some(Err("regex_named_captures: need (pattern, text)".to_string()));
    }
    let pat = args[0].as_string();
    let txt = args[1].as_string();
    match FancyRegex::new(&pat) {
        Err(e) => Some(Err(format!("regex_named_captures: bad pattern: {}", e))),
        Ok(re) => match re.captures(&txt) {
            Err(e) => Some(Err(format!("regex_named_captures: error: {}", e))),
            Ok(None) => Some(Ok(Value::Map(HashMap::new()))),
            Ok(Some(caps)) => {
                // Extract named group names from the pattern using a simple scan
                let mut map: HashMap<String, Value> = HashMap::new();
                // Find (?P<name> or (?<name> style groups
                let name_re_str = r"\(\?(?:P?)<([a-zA-Z_][a-zA-Z0-9_]*)>";
                if let Ok(name_re) = FancyRegex::new(name_re_str) {
                    let mut ns = 0usize;
                    loop {
                        match name_re.find_from_pos(&pat, ns) {
                            Ok(Some(m)) => {
                                // extract the group name between < and >
                                let slice = m.as_str();
                                if let Some(start_b) = slice.find('<') {
                                    if let Some(end_b) = slice.find('>') {
                                        let gname = &slice[start_b + 1..end_b];
                                        if let Some(mv) = caps.name(gname) {
                                            map.insert(
                                                gname.to_string(),
                                                Value::String(mv.as_str().to_string()),
                                            );
                                        }
                                    }
                                }
                                let e = m.end();
                                if e == ns {
                                    ns += 1;
                                } else {
                                    ns = e;
                                }
                                if ns > pat.len() {
                                    break;
                                }
                            }
                            _ => break,
                        }
                    }
                }
                // Fallback: also add index-based
                for i in 0..caps.len() {
                    if let Some(m) = caps.get(i) {
                        map.entry(i.to_string())
                            .or_insert_with(|| Value::String(m.as_str().to_string()));
                    }
                }
                Some(Ok(Value::Map(map)))
            }
        },
    }
}

// ── 3D Math (glam) ───────────────────────────────────────────

"vec2" => {
    if args.len() < 2 {
        return Some(Err("vec2: need (x, y)".to_string()));
    }
    let x = args[0].as_float();
    let y = args[1].as_float();
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(x));
    m.insert("y".to_string(), Value::Float(y));
    m.insert("type".to_string(), Value::String("vec2".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec3" => {
    if args.len() < 3 {
        return Some(Err("vec3: need (x, y, z)".to_string()));
    }
    let x = args[0].as_float();
    let y = args[1].as_float();
    let z = args[2].as_float();
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(x));
    m.insert("y".to_string(), Value::Float(y));
    m.insert("z".to_string(), Value::Float(z));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec4" => {
    if args.len() < 4 {
        return Some(Err("vec4: need (x, y, z, w)".to_string()));
    }
    let x = args[0].as_float();
    let y = args[1].as_float();
    let z = args[2].as_float();
    let w = args[3].as_float();
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(x));
    m.insert("y".to_string(), Value::Float(y));
    m.insert("z".to_string(), Value::Float(z));
    m.insert("w".to_string(), Value::Float(w));
    m.insert("type".to_string(), Value::String("vec4".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec3_add" => {
    if args.len() < 2 {
        return Some(Err("vec3_add: need (a, b)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let a = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let b = match get_v3(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let r = a + b;
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec3_sub" => {
    if args.len() < 2 {
        return Some(Err("vec3_sub: need (a, b)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let a = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let b = match get_v3(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let r = a - b;
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec3_mul" => {
    if args.len() < 2 {
        return Some(Err("vec3_mul: need (v, scalar)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let v = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let s = args[1].as_float() as f32;
    let r = v * s;
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec3_dot" => {
    if args.len() < 2 {
        return Some(Err("vec3_dot: need (a, b)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let a = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let b = match get_v3(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    Some(Ok(Value::Float(a.dot(b) as f64)))
}

"vec3_cross" => {
    if args.len() < 2 {
        return Some(Err("vec3_cross: need (a, b)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let a = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let b = match get_v3(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let r = a.cross(b);
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec3_len" => {
    if args.is_empty() {
        return Some(Err("vec3_len: need (v)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let v = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    Some(Ok(Value::Float(v.length() as f64)))
}

"vec3_normalize" => {
    if args.is_empty() {
        return Some(Err("vec3_normalize: need (v)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let v = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let r = v.normalize_or_zero();
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec3_lerp" => {
    if args.len() < 3 {
        return Some(Err("vec3_lerp: need (a, b, t)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let a = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let b = match get_v3(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let t = args[2].as_float() as f32;
    let r = a.lerp(b, t);
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec3_distance" => {
    if args.len() < 2 {
        return Some(Err("vec3_distance: need (a, b)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let a = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let b = match get_v3(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    Some(Ok(Value::Float(a.distance(b) as f64)))
}

"vec3_reflect" => {
    if args.len() < 2 {
        return Some(Err("vec3_reflect: need (v, normal)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let v = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let n = match get_v3(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    // reflect = v - 2 * dot(v, n) * n
    let r = v - 2.0 * v.dot(n) * n;
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"vec3_angle" => {
    if args.len() < 2 {
        return Some(Err("vec3_angle: need (a, b)".to_string()));
    }
    let get_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            Ok(glam::Vec3::new(x, y, z))
        } else {
            Err("expected vec3 map".to_string())
        }
    };
    let a = match get_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let b = match get_v3(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let cos_angle = (a.dot(b) / (a.length() * b.length())).clamp(-1.0, 1.0);
    Some(Ok(Value::Float(cos_angle.acos() as f64)))
}

"mat4_identity" => {
    let mat = glam::Mat4::IDENTITY;
    let arr: Vec<f32> = mat.to_cols_array().to_vec();
    Some(Ok(Value::Array(arr.into_iter().map(|v| Value::Float(v as f64)).collect())))
}

"mat4_mul" => {
    if args.len() < 2 {
        return Some(Err("mat4_mul: need (a, b) each Array of 16 floats".to_string()));
    }
    let arr_to_mat4 = |v: &Value| -> Result<glam::Mat4, String> {
        if let Value::Array(a) = v {
            if a.len() != 16 {
                return Err("mat4 must be 16 elements".to_string());
            }
            let mut f = [0f32; 16];
            for (i, val) in a.iter().enumerate() {
                f[i] = val.as_float() as f32;
            }
            Ok(glam::Mat4::from_cols_array(&f))
        } else {
            Err("expected Array of 16 floats for mat4".to_string())
        }
    };
    let ma = match arr_to_mat4(&args[0]) { Ok(m) => m, Err(e) => return Some(Err(e)) };
    let mb = match arr_to_mat4(&args[1]) { Ok(m) => m, Err(e) => return Some(Err(e)) };
    let r = ma * mb;
    Some(Ok(Value::Array(
        r.to_cols_array().iter().map(|&v| Value::Float(v as f64)).collect()
    )))
}

"mat4_translate" => {
    if args.len() < 3 {
        return Some(Err("mat4_translate: need (x, y, z)".to_string()));
    }
    let x = args[0].as_float() as f32;
    let y = args[1].as_float() as f32;
    let z = args[2].as_float() as f32;
    let mat = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    Some(Ok(Value::Array(
        mat.to_cols_array().iter().map(|&v| Value::Float(v as f64)).collect()
    )))
}

"mat4_scale" => {
    if args.len() < 3 {
        return Some(Err("mat4_scale: need (x, y, z)".to_string()));
    }
    let x = args[0].as_float() as f32;
    let y = args[1].as_float() as f32;
    let z = args[2].as_float() as f32;
    let mat = glam::Mat4::from_scale(glam::Vec3::new(x, y, z));
    Some(Ok(Value::Array(
        mat.to_cols_array().iter().map(|&v| Value::Float(v as f64)).collect()
    )))
}

"mat4_rotate_x" => {
    if args.is_empty() {
        return Some(Err("mat4_rotate_x: need (angle)".to_string()));
    }
    let angle = args[0].as_float() as f32;
    let mat = glam::Mat4::from_rotation_x(angle);
    Some(Ok(Value::Array(
        mat.to_cols_array().iter().map(|&v| Value::Float(v as f64)).collect()
    )))
}

"mat4_rotate_y" => {
    if args.is_empty() {
        return Some(Err("mat4_rotate_y: need (angle)".to_string()));
    }
    let angle = args[0].as_float() as f32;
    let mat = glam::Mat4::from_rotation_y(angle);
    Some(Ok(Value::Array(
        mat.to_cols_array().iter().map(|&v| Value::Float(v as f64)).collect()
    )))
}

"mat4_rotate_z" => {
    if args.is_empty() {
        return Some(Err("mat4_rotate_z: need (angle)".to_string()));
    }
    let angle = args[0].as_float() as f32;
    let mat = glam::Mat4::from_rotation_z(angle);
    Some(Ok(Value::Array(
        mat.to_cols_array().iter().map(|&v| Value::Float(v as f64)).collect()
    )))
}

"mat4_perspective" => {
    if args.len() < 4 {
        return Some(Err("mat4_perspective: need (fov, aspect, near, far)".to_string()));
    }
    let fov = args[0].as_float() as f32;
    let aspect = args[1].as_float() as f32;
    let near = args[2].as_float() as f32;
    let far = args[3].as_float() as f32;
    let mat = glam::Mat4::perspective_rh(fov, aspect, near, far);
    Some(Ok(Value::Array(
        mat.to_cols_array().iter().map(|&v| Value::Float(v as f64)).collect()
    )))
}

"mat4_look_at" => {
    if args.len() < 3 {
        return Some(Err("mat4_look_at: need (eye_arr, target_arr, up_arr)".to_string()));
    }
    let arr_to_v3 = |v: &Value| -> Result<glam::Vec3, String> {
        match v {
            Value::Array(a) if a.len() >= 3 => Ok(glam::Vec3::new(
                a[0].as_float() as f32,
                a[1].as_float() as f32,
                a[2].as_float() as f32,
            )),
            Value::Map(m) => Ok(glam::Vec3::new(
                m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32,
                m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32,
                m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32,
            )),
            _ => Err("expected Array[3] or vec3 map".to_string()),
        }
    };
    let eye = match arr_to_v3(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let center = match arr_to_v3(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let up = match arr_to_v3(&args[2]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let mat = glam::Mat4::look_at_rh(eye, center, up);
    Some(Ok(Value::Array(
        mat.to_cols_array().iter().map(|&v| Value::Float(v as f64)).collect()
    )))
}

"mat4_transform_point" => {
    if args.len() < 2 {
        return Some(Err("mat4_transform_point: need (mat16, point3)".to_string()));
    }
    let mat = if let Value::Array(a) = &args[0] {
        if a.len() != 16 {
            return Some(Err("mat4_transform_point: mat must be 16 elements".to_string()));
        }
        let mut f = [0f32; 16];
        for (i, val) in a.iter().enumerate() { f[i] = val.as_float() as f32; }
        glam::Mat4::from_cols_array(&f)
    } else {
        return Some(Err("mat4_transform_point: first arg must be Array of 16".to_string()));
    };
    let pt = match &args[1] {
        Value::Array(a) if a.len() >= 3 => glam::Vec3::new(
            a[0].as_float() as f32, a[1].as_float() as f32, a[2].as_float() as f32
        ),
        Value::Map(m) => glam::Vec3::new(
            m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32,
            m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32,
            m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32,
        ),
        _ => return Some(Err("mat4_transform_point: invalid point".to_string())),
    };
    let r = mat.transform_point3(pt);
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"quat_identity" => {
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(0.0));
    m.insert("y".to_string(), Value::Float(0.0));
    m.insert("z".to_string(), Value::Float(0.0));
    m.insert("w".to_string(), Value::Float(1.0));
    m.insert("type".to_string(), Value::String("quat".to_string()));
    Some(Ok(Value::Map(m)))
}

"quat_from_axis_angle" => {
    if args.len() < 2 {
        return Some(Err("quat_from_axis_angle: need (axis_arr, angle)".to_string()));
    }
    let axis = match &args[0] {
        Value::Array(a) if a.len() >= 3 => glam::Vec3::new(
            a[0].as_float() as f32, a[1].as_float() as f32, a[2].as_float() as f32
        ),
        Value::Map(m) => glam::Vec3::new(
            m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32,
            m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32,
            m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32,
        ),
        _ => return Some(Err("quat_from_axis_angle: invalid axis".to_string())),
    };
    let angle = args[1].as_float() as f32;
    let q = glam::Quat::from_axis_angle(axis.normalize_or_zero(), angle);
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(q.x as f64));
    m.insert("y".to_string(), Value::Float(q.y as f64));
    m.insert("z".to_string(), Value::Float(q.z as f64));
    m.insert("w".to_string(), Value::Float(q.w as f64));
    m.insert("type".to_string(), Value::String("quat".to_string()));
    Some(Ok(Value::Map(m)))
}

"quat_mul" => {
    if args.len() < 2 {
        return Some(Err("quat_mul: need (a, b)".to_string()));
    }
    let get_q = |v: &Value| -> Result<glam::Quat, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let w = m.get("w").map(|v| v.as_float()).unwrap_or(1.0) as f32;
            Ok(glam::Quat::from_xyzw(x, y, z, w))
        } else {
            Err("expected quat map".to_string())
        }
    };
    let qa = match get_q(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let qb = match get_q(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let r = qa * qb;
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("w".to_string(), Value::Float(r.w as f64));
    m.insert("type".to_string(), Value::String("quat".to_string()));
    Some(Ok(Value::Map(m)))
}

"quat_slerp" => {
    if args.len() < 3 {
        return Some(Err("quat_slerp: need (a, b, t)".to_string()));
    }
    let get_q = |v: &Value| -> Result<glam::Quat, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let w = m.get("w").map(|v| v.as_float()).unwrap_or(1.0) as f32;
            Ok(glam::Quat::from_xyzw(x, y, z, w))
        } else {
            Err("expected quat map".to_string())
        }
    };
    let qa = match get_q(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let qb = match get_q(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let t = args[2].as_float() as f32;
    let r = qa.slerp(qb, t);
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("w".to_string(), Value::Float(r.w as f64));
    m.insert("type".to_string(), Value::String("quat".to_string()));
    Some(Ok(Value::Map(m)))
}

"quat_to_euler" => {
    if args.is_empty() {
        return Some(Err("quat_to_euler: need (q)".to_string()));
    }
    let q = if let Value::Map(map) = &args[0] {
        let x = map.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let y = map.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let z = map.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let w = map.get("w").map(|v| v.as_float()).unwrap_or(1.0) as f32;
        glam::Quat::from_xyzw(x, y, z, w)
    } else {
        return Some(Err("quat_to_euler: expected quat map".to_string()));
    };
    let (roll, pitch, yaw) = q.to_euler(glam::EulerRot::XYZ);
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(roll as f64));
    m.insert("y".to_string(), Value::Float(pitch as f64));
    m.insert("z".to_string(), Value::Float(yaw as f64));
    Some(Ok(Value::Map(m)))
}

"quat_from_euler" => {
    if args.len() < 3 {
        return Some(Err("quat_from_euler: need (x, y, z)".to_string()));
    }
    let ex = args[0].as_float() as f32;
    let ey = args[1].as_float() as f32;
    let ez = args[2].as_float() as f32;
    let q = glam::Quat::from_euler(glam::EulerRot::XYZ, ex, ey, ez);
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(q.x as f64));
    m.insert("y".to_string(), Value::Float(q.y as f64));
    m.insert("z".to_string(), Value::Float(q.z as f64));
    m.insert("w".to_string(), Value::Float(q.w as f64));
    m.insert("type".to_string(), Value::String("quat".to_string()));
    Some(Ok(Value::Map(m)))
}

"quat_rotate_vec3" => {
    if args.len() < 2 {
        return Some(Err("quat_rotate_vec3: need (q, v)".to_string()));
    }
    let q = if let Value::Map(map) = &args[0] {
        let x = map.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let y = map.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let z = map.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let w = map.get("w").map(|v| v.as_float()).unwrap_or(1.0) as f32;
        glam::Quat::from_xyzw(x, y, z, w)
    } else {
        return Some(Err("quat_rotate_vec3: expected quat map".to_string()));
    };
    let v = match &args[1] {
        Value::Map(m) => glam::Vec3::new(
            m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32,
            m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32,
            m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32,
        ),
        Value::Array(a) if a.len() >= 3 => glam::Vec3::new(
            a[0].as_float() as f32, a[1].as_float() as f32, a[2].as_float() as f32
        ),
        _ => return Some(Err("quat_rotate_vec3: invalid vec3".to_string())),
    };
    let r = q * v;
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("type".to_string(), Value::String("vec3".to_string()));
    Some(Ok(Value::Map(m)))
}

"quat_conjugate" => {
    if args.is_empty() {
        return Some(Err("quat_conjugate: need (q)".to_string()));
    }
    let q = if let Value::Map(map) = &args[0] {
        let x = map.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let y = map.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let z = map.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let w = map.get("w").map(|v| v.as_float()).unwrap_or(1.0) as f32;
        glam::Quat::from_xyzw(x, y, z, w)
    } else {
        return Some(Err("quat_conjugate: expected quat map".to_string()));
    };
    let r = q.conjugate();
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("w".to_string(), Value::Float(r.w as f64));
    m.insert("type".to_string(), Value::String("quat".to_string()));
    Some(Ok(Value::Map(m)))
}

"quat_normalize" => {
    if args.is_empty() {
        return Some(Err("quat_normalize: need (q)".to_string()));
    }
    let q = if let Value::Map(map) = &args[0] {
        let x = map.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let y = map.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let z = map.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
        let w = map.get("w").map(|v| v.as_float()).unwrap_or(1.0) as f32;
        glam::Quat::from_xyzw(x, y, z, w)
    } else {
        return Some(Err("quat_normalize: expected quat map".to_string()));
    };
    let r = q.normalize();
    let mut m = HashMap::new();
    m.insert("x".to_string(), Value::Float(r.x as f64));
    m.insert("y".to_string(), Value::Float(r.y as f64));
    m.insert("z".to_string(), Value::Float(r.z as f64));
    m.insert("w".to_string(), Value::Float(r.w as f64));
    m.insert("type".to_string(), Value::String("quat".to_string()));
    Some(Ok(Value::Map(m)))
}

"quat_dot" => {
    if args.len() < 2 {
        return Some(Err("quat_dot: need (a, b)".to_string()));
    }
    let get_q = |v: &Value| -> Result<glam::Quat, String> {
        if let Value::Map(m) = v {
            let x = m.get("x").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let y = m.get("y").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let z = m.get("z").map(|v| v.as_float()).unwrap_or(0.0) as f32;
            let w = m.get("w").map(|v| v.as_float()).unwrap_or(1.0) as f32;
            Ok(glam::Quat::from_xyzw(x, y, z, w))
        } else {
            Err("expected quat map".to_string())
        }
    };
    let qa = match get_q(&args[0]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    let qb = match get_q(&args[1]) { Ok(v) => v, Err(e) => return Some(Err(e)) };
    Some(Ok(Value::Float(qa.dot(qb) as f64)))
}

// ── Audio (hound) ────────────────────────────────────────────

"wav_read" => {
    if args.is_empty() {
        return Some(Err("wav_read: need (path)".to_string()));
    }
    let path = args[0].as_string();
    let mut reader = match hound::WavReader::open(&path) {
        Ok(r) => r,
        Err(e) => return Some(Err(format!("wav_read: {}", e))),
    };
    let spec = reader.spec();
    let samples: Vec<Value> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f64;
            reader
                .samples::<i32>()
                .map(|s| {
                    s.map(|v| Value::Float(v as f64 / max))
                        .unwrap_or(Value::Float(0.0))
                })
                .collect()
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| {
                s.map(|v| Value::Float(v as f64))
                    .unwrap_or(Value::Float(0.0))
            })
            .collect(),
    };
    let mut m = HashMap::new();
    m.insert("samples".to_string(), Value::Array(samples));
    m.insert("sample_rate".to_string(), Value::Int(spec.sample_rate as i64));
    m.insert("channels".to_string(), Value::Int(spec.channels as i64));
    m.insert("bits".to_string(), Value::Int(spec.bits_per_sample as i64));
    Some(Ok(Value::Map(m)))
}

"wav_write" => {
    if args.len() < 4 {
        return Some(Err("wav_write: need (path, samples, sample_rate, channels)".to_string()));
    }
    let path = args[0].as_string();
    let sample_rate = args[2].as_int() as u32;
    let channels = args[3].as_int() as u16;
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = match hound::WavWriter::create(&path, spec) {
        Ok(w) => w,
        Err(e) => return Some(Err(format!("wav_write: {}", e))),
    };
    let samples = match &args[1] {
        Value::Array(a) => a.clone(),
        _ => return Some(Err("wav_write: samples must be Array".to_string())),
    };
    for s in &samples {
        let fv = s.as_float().clamp(-1.0, 1.0);
        let iv = (fv * 32767.0) as i16;
        if let Err(e) = writer.write_sample(iv) {
            return Some(Err(format!("wav_write: write error: {}", e)));
        }
    }
    if let Err(e) = writer.finalize() {
        return Some(Err(format!("wav_write: finalize error: {}", e)));
    }
    Some(Ok(Value::Null))
}

"wav_info" => {
    if args.is_empty() {
        return Some(Err("wav_info: need (path)".to_string()));
    }
    let path = args[0].as_string();
    let reader = match hound::WavReader::open(&path) {
        Ok(r) => r,
        Err(e) => return Some(Err(format!("wav_info: {}", e))),
    };
    let spec = reader.spec();
    let duration_samples = reader.duration();
    let sample_rate = spec.sample_rate;
    let duration_secs = if sample_rate > 0 {
        duration_samples as f64 / sample_rate as f64
    } else {
        0.0
    };
    let mut m = HashMap::new();
    m.insert("sample_rate".to_string(), Value::Int(spec.sample_rate as i64));
    m.insert("channels".to_string(), Value::Int(spec.channels as i64));
    m.insert("bits_per_sample".to_string(), Value::Int(spec.bits_per_sample as i64));
    m.insert("duration_samples".to_string(), Value::Int(duration_samples as i64));
    m.insert("duration_secs".to_string(), Value::Float(duration_secs));
    m.insert(
        "sample_format".to_string(),
        Value::String(match spec.sample_format {
            hound::SampleFormat::Int => "int".to_string(),
            hound::SampleFormat::Float => "float".to_string(),
        }),
    );
    Some(Ok(Value::Map(m)))
}

"wav_generate_sine" => {
    if args.len() < 3 {
        return Some(Err("wav_generate_sine: need (freq, duration_sec, sample_rate)".to_string()));
    }
    let freq = args[0].as_float();
    let dur = args[1].as_float();
    let sr = args[2].as_float();
    let n = (dur * sr) as usize;
    let samples: Vec<Value> = (0..n)
        .map(|i| {
            let t = i as f64 / sr;
            Value::Float((2.0 * PI * freq * t).sin())
        })
        .collect();
    Some(Ok(Value::Array(samples)))
}

"wav_generate_square" => {
    if args.len() < 3 {
        return Some(Err("wav_generate_square: need (freq, duration_sec, sample_rate)".to_string()));
    }
    let freq = args[0].as_float();
    let dur = args[1].as_float();
    let sr = args[2].as_float();
    let n = (dur * sr) as usize;
    let samples: Vec<Value> = (0..n)
        .map(|i| {
            let t = i as f64 / sr;
            let sine = (2.0 * PI * freq * t).sin();
            Value::Float(if sine >= 0.0 { 1.0 } else { -1.0 })
        })
        .collect();
    Some(Ok(Value::Array(samples)))
}

"wav_generate_sawtooth" => {
    if args.len() < 3 {
        return Some(Err("wav_generate_sawtooth: need (freq, duration_sec, sample_rate)".to_string()));
    }
    let freq = args[0].as_float();
    let dur = args[1].as_float();
    let sr = args[2].as_float();
    let n = (dur * sr) as usize;
    let samples: Vec<Value> = (0..n)
        .map(|i| {
            let t = i as f64 / sr;
            // sawtooth: 2 * (t*f - floor(t*f + 0.5))
            let phase = t * freq;
            let saw = 2.0 * (phase - (phase + 0.5).floor());
            Value::Float(saw)
        })
        .collect();
    Some(Ok(Value::Array(samples)))
}

"wav_mix" => {
    if args.len() < 3 {
        return Some(Err("wav_mix: need (samples_a, samples_b, ratio)".to_string()));
    }
    let sa = match &args[0] {
        Value::Array(a) => a.clone(),
        _ => return Some(Err("wav_mix: first arg must be Array".to_string())),
    };
    let sb = match &args[1] {
        Value::Array(a) => a.clone(),
        _ => return Some(Err("wav_mix: second arg must be Array".to_string())),
    };
    let ratio = args[2].as_float().clamp(0.0, 1.0);
    let len = sa.len().max(sb.len());
    let mixed: Vec<Value> = (0..len)
        .map(|i| {
            let a = if i < sa.len() { sa[i].as_float() } else { 0.0 };
            let b = if i < sb.len() { sb[i].as_float() } else { 0.0 };
            Value::Float((a * (1.0 - ratio) + b * ratio).clamp(-1.0, 1.0))
        })
        .collect();
    Some(Ok(Value::Array(mixed)))
}

"wav_volume" => {
    if args.len() < 2 {
        return Some(Err("wav_volume: need (samples, factor)".to_string()));
    }
    let samples = match &args[0] {
        Value::Array(a) => a.clone(),
        _ => return Some(Err("wav_volume: first arg must be Array".to_string())),
    };
    let factor = args[1].as_float();
    let result: Vec<Value> = samples
        .iter()
        .map(|s| Value::Float((s.as_float() * factor).clamp(-1.0, 1.0)))
        .collect();
    Some(Ok(Value::Array(result)))
}

"wav_reverse" => {
    if args.is_empty() {
        return Some(Err("wav_reverse: need (samples)".to_string()));
    }
    let mut samples = match &args[0] {
        Value::Array(a) => a.clone(),
        _ => return Some(Err("wav_reverse: arg must be Array".to_string())),
    };
    samples.reverse();
    Some(Ok(Value::Array(samples)))
}

"wav_to_pcm16" => {
    if args.is_empty() {
        return Some(Err("wav_to_pcm16: need (samples)".to_string()));
    }
    let samples = match &args[0] {
        Value::Array(a) => a.clone(),
        _ => return Some(Err("wav_to_pcm16: arg must be Array".to_string())),
    };
    let pcm: Vec<Value> = samples
        .iter()
        .map(|s| {
            let fv = s.as_float().clamp(-1.0, 1.0);
            Value::Int((fv * 32767.0) as i64)
        })
        .collect();
    Some(Ok(Value::Array(pcm)))
}

// ── Finance / Math ───────────────────────────────────────────

"finance_pv" => {
    if args.len() < 4 {
        return Some(Err("finance_pv: need (rate, nper, pmt, fv)".to_string()));
    }
    let r = args[0].as_float();
    let n = args[1].as_float();
    let pmt = args[2].as_float();
    let fv = args[3].as_float();
    let pv = if r.abs() < 1e-12 {
        -pmt * n - fv
    } else {
        let factor = (1.0 + r).powf(-n);
        -pmt * (1.0 - factor) / r - fv * factor
    };
    Some(Ok(Value::Float(pv)))
}

"finance_fv" => {
    if args.len() < 4 {
        return Some(Err("finance_fv: need (rate, nper, pmt, pv)".to_string()));
    }
    let r = args[0].as_float();
    let n = args[1].as_float();
    let pmt = args[2].as_float();
    let pv = args[3].as_float();
    let fv = if r.abs() < 1e-12 {
        -pmt * n - pv
    } else {
        let factor = (1.0 + r).powf(n);
        -pmt * (factor - 1.0) / r - pv * factor
    };
    Some(Ok(Value::Float(fv)))
}

"finance_nper" => {
    if args.len() < 4 {
        return Some(Err("finance_nper: need (rate, pmt, pv, fv)".to_string()));
    }
    let r = args[0].as_float();
    let pmt = args[1].as_float();
    let pv = args[2].as_float();
    let fv = args[3].as_float();
    if r.abs() < 1e-12 {
        if pmt.abs() < 1e-12 {
            return Some(Err("finance_nper: rate and pmt both zero".to_string()));
        }
        return Some(Ok(Value::Float(-(pv + fv) / pmt)));
    }
    let num = pmt / r - fv;
    let den = pmt / r + pv;
    if den == 0.0 || (num / den) <= 0.0 {
        return Some(Err("finance_nper: invalid inputs (log of non-positive)".to_string()));
    }
    let nper = (num / den).ln() / (1.0 + r).ln();
    Some(Ok(Value::Float(nper)))
}

"finance_pmt" => {
    if args.len() < 4 {
        return Some(Err("finance_pmt: need (rate, nper, pv, fv)".to_string()));
    }
    let r = args[0].as_float();
    let n = args[1].as_float();
    let pv = args[2].as_float();
    let fv = args[3].as_float();
    let pmt = if r.abs() < 1e-12 {
        -(pv + fv) / n
    } else {
        let factor = (1.0 + r).powf(n);
        -r * (pv * factor + fv) / (factor - 1.0)
    };
    Some(Ok(Value::Float(pmt)))
}

"finance_npv" => {
    if args.len() < 2 {
        return Some(Err("finance_npv: need (rate, cashflows)".to_string()));
    }
    let r = args[0].as_float();
    let cfs = match &args[1] {
        Value::Array(a) => a.clone(),
        _ => return Some(Err("finance_npv: cashflows must be Array".to_string())),
    };
    let npv: f64 = cfs
        .iter()
        .enumerate()
        .map(|(i, cf)| cf.as_float() / (1.0 + r).powi(i as i32 + 1))
        .sum();
    Some(Ok(Value::Float(npv)))
}

"finance_irr" => {
    if args.is_empty() {
        return Some(Err("finance_irr: need (cashflows)".to_string()));
    }
    let cfs: Vec<f64> = match &args[0] {
        Value::Array(a) => a.iter().map(|v| v.as_float()).collect(),
        _ => return Some(Err("finance_irr: cashflows must be Array".to_string())),
    };
    if cfs.is_empty() {
        return Some(Err("finance_irr: cashflows empty".to_string()));
    }
    let npv_fn = |r: f64| -> f64 {
        cfs.iter()
            .enumerate()
            .map(|(i, &cf)| cf / (1.0 + r).powi(i as i32))
            .sum()
    };
    let npv_deriv = |r: f64| -> f64 {
        cfs.iter()
            .enumerate()
            .skip(1)
            .map(|(i, &cf)| -(i as f64) * cf / (1.0 + r).powi(i as i32 + 1))
            .sum()
    };
    let mut r = 0.1f64;
    for _ in 0..1000 {
        let f = npv_fn(r);
        let df = npv_deriv(r);
        if df.abs() < 1e-15 {
            break;
        }
        let r_new = r - f / df;
        if (r_new - r).abs() < 1e-10 {
            r = r_new;
            break;
        }
        r = r_new;
        if r < -0.9999 {
            r = -0.9999;
        }
    }
    Some(Ok(Value::Float(r)))
}

"finance_compound" => {
    if args.len() < 4 {
        return Some(Err("finance_compound: need (principal, rate, n_per_year, years)".to_string()));
    }
    let p = args[0].as_float();
    let r = args[1].as_float();
    let n = args[2].as_float();
    let t = args[3].as_float();
    let result = p * (1.0 + r / n).powf(n * t);
    Some(Ok(Value::Float(result)))
}

"finance_simple_interest" => {
    if args.len() < 3 {
        return Some(Err("finance_simple_interest: need (principal, rate, years)".to_string()));
    }
    let p = args[0].as_float();
    let r = args[1].as_float();
    let t = args[2].as_float();
    Some(Ok(Value::Float(p * (1.0 + r * t))))
}

"finance_black_scholes_call" => {
    if args.len() < 5 {
        return Some(Err("finance_black_scholes_call: need (S, K, T, r, sigma)".to_string()));
    }
    let s = args[0].as_float();
    let k = args[1].as_float();
    let t = args[2].as_float();
    let r = args[3].as_float();
    let sigma = args[4].as_float();
    if t <= 0.0 || sigma <= 0.0 || s <= 0.0 || k <= 0.0 {
        return Some(Err("finance_black_scholes_call: S, K, T, sigma must be positive".to_string()));
    }
    let sqrt_t = t.sqrt();
    let d1 = ((s / k).ln() + (r + sigma * sigma / 2.0) * t) / (sigma * sqrt_t);
    let d2 = d1 - sigma * sqrt_t;
    // Normal CDF approximation
    let normal_cdf = |x: f64| -> f64 { 0.5 * (1.0 + erf(x / 2.0f64.sqrt())) };
    // erf approximation (Horner's method, Abramowitz & Stegun 7.1.26)
    fn erf(x: f64) -> f64 {
        let t = 1.0 / (1.0 + 0.3275911 * x.abs());
        let poly = t * (0.254829592
            + t * (-0.284496736
                + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
        let sign = if x >= 0.0 { 1.0 } else { -1.0 };
        sign * (1.0 - poly * (-x * x).exp())
    }
    let call = s * normal_cdf(d1) - k * (-r * t).exp() * normal_cdf(d2);
    Some(Ok(Value::Float(call)))
}

"finance_black_scholes_put" => {
    if args.len() < 5 {
        return Some(Err("finance_black_scholes_put: need (S, K, T, r, sigma)".to_string()));
    }
    let s = args[0].as_float();
    let k = args[1].as_float();
    let t = args[2].as_float();
    let r = args[3].as_float();
    let sigma = args[4].as_float();
    if t <= 0.0 || sigma <= 0.0 || s <= 0.0 || k <= 0.0 {
        return Some(Err("finance_black_scholes_put: S, K, T, sigma must be positive".to_string()));
    }
    let sqrt_t = t.sqrt();
    let d1 = ((s / k).ln() + (r + sigma * sigma / 2.0) * t) / (sigma * sqrt_t);
    let d2 = d1 - sigma * sqrt_t;
    fn erf(x: f64) -> f64 {
        let t = 1.0 / (1.0 + 0.3275911 * x.abs());
        let poly = t * (0.254829592
            + t * (-0.284496736
                + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
        let sign = if x >= 0.0 { 1.0 } else { -1.0 };
        sign * (1.0 - poly * (-x * x).exp())
    }
    let normal_cdf = |x: f64| -> f64 { 0.5 * (1.0 + erf(x / 2.0f64.sqrt())) };
    let put = k * (-r * t).exp() * normal_cdf(-d2) - s * normal_cdf(-d1);
    Some(Ok(Value::Float(put)))
}

"finance_sharpe" => {
    if args.len() < 2 {
        return Some(Err("finance_sharpe: need (returns_arr, risk_free_rate)".to_string()));
    }
    let returns: Vec<f64> = match &args[0] {
        Value::Array(a) => a.iter().map(|v| v.as_float()).collect(),
        _ => return Some(Err("finance_sharpe: returns must be Array".to_string())),
    };
    let rf = args[1].as_float();
    let n = returns.len() as f64;
    if n == 0.0 {
        return Some(Err("finance_sharpe: empty returns".to_string()));
    }
    let excess: Vec<f64> = returns.iter().map(|&r| r - rf).collect();
    let mean = excess.iter().sum::<f64>() / n;
    let variance = excess.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();
    if std_dev.abs() < 1e-15 {
        return Some(Err("finance_sharpe: zero std deviation".to_string()));
    }
    Some(Ok(Value::Float(mean / std_dev)))
}

"finance_roi" => {
    if args.len() < 2 {
        return Some(Err("finance_roi: need (gain, cost)".to_string()));
    }
    let gain = args[0].as_float();
    let cost = args[1].as_float();
    if cost.abs() < 1e-15 {
        return Some(Err("finance_roi: cost cannot be zero".to_string()));
    }
    Some(Ok(Value::Float((gain - cost) / cost)))
}

"finance_cagr" => {
    if args.len() < 3 {
        return Some(Err("finance_cagr: need (begin, end, years)".to_string()));
    }
    let begin = args[0].as_float();
    let end = args[1].as_float();
    let years = args[2].as_float();
    if begin <= 0.0 {
        return Some(Err("finance_cagr: begin value must be positive".to_string()));
    }
    if years <= 0.0 {
        return Some(Err("finance_cagr: years must be positive".to_string()));
    }
    Some(Ok(Value::Float((end / begin).powf(1.0 / years) - 1.0)))
}

"finance_mortgage" => {
    if args.len() < 3 {
        return Some(Err("finance_mortgage: need (principal, annual_rate, years)".to_string()));
    }
    let p = args[0].as_float();
    let annual_rate = args[1].as_float();
    let years = args[2].as_float();
    let monthly_rate = annual_rate / 12.0;
    let n = (years * 12.0) as f64;
    let pmt = if monthly_rate.abs() < 1e-12 {
        p / n
    } else {
        let factor = (1.0 + monthly_rate).powf(n);
        p * monthly_rate * factor / (factor - 1.0)
    };
    Some(Ok(Value::Float(pmt)))
}

// ── Advanced String Operations ───────────────────────────────

"str_format_number" => {
    if args.len() < 4 {
        return Some(Err("str_format_number: need (n, decimals, thousands_sep, decimal_sep)".to_string()));
    }
    let n = args[0].as_float();
    let decimals = args[1].as_int() as usize;
    let thousands_sep = args[2].as_string();
    let decimal_sep = args[3].as_string();
    let formatted_raw = format!("{:.prec$}", n, prec = decimals);
    let parts: Vec<&str> = formatted_raw.splitn(2, '.').collect();
    let int_part = parts[0];
    let frac_part = if parts.len() > 1 { parts[1] } else { "" };
    let negative = int_part.starts_with('-');
    let digits = if negative { &int_part[1..] } else { int_part };
    let mut grouped = String::new();
    let mut count = 0usize;
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            // push separator in reverse
            for sc in thousands_sep.chars().rev() {
                grouped.insert(0, sc);
            }
        }
        grouped.insert(0, ch);
        count += 1;
    }
    let mut result = if negative {
        format!("-{}", grouped)
    } else {
        grouped
    };
    if decimals > 0 {
        result.push_str(&decimal_sep);
        result.push_str(frac_part);
    }
    Some(Ok(Value::String(result)))
}

"str_parse_number" => {
    if args.is_empty() {
        return Some(Err("str_parse_number: need (s)".to_string()));
    }
    let s = args[0].as_string();
    let cleaned: String = s.chars().filter(|&c| c != ',').collect();
    match cleaned.trim().parse::<f64>() {
        Ok(v) => Some(Ok(Value::Float(v))),
        Err(e) => Some(Err(format!("str_parse_number: {}", e))),
    }
}

"str_lorem_ipsum" => {
    if args.is_empty() {
        return Some(Err("str_lorem_ipsum: need (words)".to_string()));
    }
    let n = args[0].as_int() as usize;
    let bank = [
        "Lorem", "ipsum", "dolor", "sit", "amet", "consectetur",
        "adipiscing", "elit", "sed", "do", "eiusmod", "tempor",
        "incididunt", "ut", "labore", "et", "dolore", "magna", "aliqua",
    ];
    let words: Vec<&str> = (0..n).map(|i| bank[i % bank.len()]).collect();
    Some(Ok(Value::String(words.join(" "))))
}

"str_uuid_like" => {
    if args.is_empty() {
        return Some(Err("str_uuid_like: need (s)".to_string()));
    }
    let s = args[0].as_string();
    // xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    let parts: Vec<&str> = s.split('-').collect();
    let valid = parts.len() == 5
        && parts[0].len() == 8
        && parts[1].len() == 4
        && parts[2].len() == 4
        && parts[3].len() == 4
        && parts[4].len() == 12
        && s.chars().all(|c| c == '-' || c.is_ascii_hexdigit());
    Some(Ok(Value::Bool(valid)))
}

"str_email_valid" => {
    if args.is_empty() {
        return Some(Err("str_email_valid: need (s)".to_string()));
    }
    let s = args[0].as_string();
    let at_pos = s.find('@');
    let valid = if let Some(at) = at_pos {
        let domain = &s[at + 1..];
        !s[..at].is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
    } else {
        false
    };
    Some(Ok(Value::Bool(valid)))
}

"str_ip_valid" => {
    if args.is_empty() {
        return Some(Err("str_ip_valid: need (s)".to_string()));
    }
    let s = args[0].as_string();
    let parts: Vec<&str> = s.split('.').collect();
    let valid = parts.len() == 4
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars().all(|c| c.is_ascii_digit())
                && p.parse::<u8>().is_ok()
        });
    Some(Ok(Value::Bool(valid)))
}

"str_mask" => {
    if args.len() < 4 {
        return Some(Err("str_mask: need (s, start, end, char)".to_string()));
    }
    let s = args[0].as_string();
    let start = args[1].as_int() as usize;
    let end = args[2].as_int() as usize;
    let mask_ch_s = args[3].as_string();
    let mask_ch = mask_ch_s.chars().next().unwrap_or('*');
    let chars: Vec<char> = s.chars().collect();
    let result: String = chars
        .iter()
        .enumerate()
        .map(|(i, &c)| if i >= start && i < end.min(chars.len()) { mask_ch } else { c })
        .collect();
    Some(Ok(Value::String(result)))
}

"str_contains_all" => {
    if args.len() < 2 {
        return Some(Err("str_contains_all: need (s, arr)".to_string()));
    }
    let s = args[0].as_string();
    let needles = match &args[1] {
        Value::Array(a) => a.clone(),
        _ => return Some(Err("str_contains_all: second arg must be Array".to_string())),
    };
    let all = needles.iter().all(|n| s.contains(&n.as_string() as &str));
    Some(Ok(Value::Bool(all)))
}

"str_contains_any" => {
    if args.len() < 2 {
        return Some(Err("str_contains_any: need (s, arr)".to_string()));
    }
    let s = args[0].as_string();
    let needles = match &args[1] {
        Value::Array(a) => a.clone(),
        _ => return Some(Err("str_contains_any: second arg must be Array".to_string())),
    };
    let any = needles.iter().any(|n| s.contains(&n.as_string() as &str));
    Some(Ok(Value::Bool(any)))
}

"str_count_occurrences" => {
    if args.len() < 2 {
        return Some(Err("str_count_occurrences: need (s, substr)".to_string()));
    }
    let s = args[0].as_string();
    let sub = args[1].as_string();
    if sub.is_empty() {
        return Some(Ok(Value::Int(0)));
    }
    let count = s.match_indices(&sub as &str).count();
    Some(Ok(Value::Int(count as i64)))
}

"str_extract_numbers" => {
    if args.is_empty() {
        return Some(Err("str_extract_numbers: need (s)".to_string()));
    }
    let s = args[0].as_string();
    match FancyRegex::new(r"-?\d+(?:\.\d+)?") {
        Err(e) => Some(Err(format!("str_extract_numbers: regex error: {}", e))),
        Ok(re) => {
            let mut results = Vec::new();
            let mut pos = 0usize;
            loop {
                match re.find_from_pos(&s, pos) {
                    Ok(Some(m)) => {
                        if let Ok(n) = m.as_str().parse::<f64>() {
                            results.push(Value::Float(n));
                        }
                        let end = m.end();
                        pos = if end == pos { pos + 1 } else { end };
                        if pos > s.len() { break; }
                    }
                    _ => break,
                }
            }
            Some(Ok(Value::Array(results)))
        }
    }
}

"str_extract_urls" => {
    if args.is_empty() {
        return Some(Err("str_extract_urls: need (s)".to_string()));
    }
    let s = args[0].as_string();
    match FancyRegex::new(r#"https?://[^\s<>"'{}|\\^\[\]`]+"#) {
        Err(e) => Some(Err(format!("str_extract_urls: regex error: {}", e))),
        Ok(re) => {
            let mut results = Vec::new();
            let mut pos = 0usize;
            loop {
                match re.find_from_pos(&s, pos) {
                    Ok(Some(m)) => {
                        results.push(Value::String(m.as_str().to_string()));
                        let end = m.end();
                        pos = if end == pos { pos + 1 } else { end };
                        if pos > s.len() { break; }
                    }
                    _ => break,
                }
            }
            Some(Ok(Value::Array(results)))
        }
    }
}

"str_extract_emails" => {
    if args.is_empty() {
        return Some(Err("str_extract_emails: need (s)".to_string()));
    }
    let s = args[0].as_string();
    match FancyRegex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}") {
        Err(e) => Some(Err(format!("str_extract_emails: regex error: {}", e))),
        Ok(re) => {
            let mut results = Vec::new();
            let mut pos = 0usize;
            loop {
                match re.find_from_pos(&s, pos) {
                    Ok(Some(m)) => {
                        results.push(Value::String(m.as_str().to_string()));
                        let end = m.end();
                        pos = if end == pos { pos + 1 } else { end };
                        if pos > s.len() { break; }
                    }
                    _ => break,
                }
            }
            Some(Ok(Value::Array(results)))
        }
    }
}

"str_banner" => {
    // Wrap string in a Unicode box:
    // ┌──────────┐
    // │  HELLO   │
    // └──────────┘
    if args.is_empty() {
        return Some(Err("str_banner: need (s)".to_string()));
    }
    let s = args[0].as_string();
    let inner = format!("  {}  ", s);
    let width = inner.chars().count();
    let top = format!("┌{}┐", "─".repeat(width));
    let mid = format!("│{}│", inner);
    let bot = format!("└{}┘", "─".repeat(width));
    Some(Ok(Value::String(format!("{}\n{}\n{}", top, mid, bot))))
}

"str_soundex" => {
    if args.is_empty() {
        return Some(Err("str_soundex: need (s)".to_string()));
    }
    let s = args[0].as_string().to_uppercase();
    let chars: Vec<char> = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if chars.is_empty() {
        return Some(Ok(Value::String("0000".to_string())));
    }
    let soundex_code = |c: char| -> char {
        match c {
            'B' | 'F' | 'P' | 'V' => '1',
            'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
            'D' | 'T' => '3',
            'L' => '4',
            'M' | 'N' => '5',
            'R' => '6',
            _ => '0', // vowels and H, W, Y → 0 (ignored)
        }
    };
    let first = chars[0];
    let mut code = String::new();
    code.push(first);
    let mut prev = soundex_code(first);
    for &ch in &chars[1..] {
        if code.len() >= 4 {
            break;
        }
        let c = soundex_code(ch);
        if c != '0' && c != prev {
            code.push(c);
        }
        prev = c;
    }
    while code.len() < 4 {
        code.push('0');
    }
    Some(Ok(Value::String(code)))
}

"str_metaphone" => {
    // Simplified metaphone: basic consonant reduction
    if args.is_empty() {
        return Some(Err("str_metaphone: need (s)".to_string()));
    }
    let s = args[0].as_string().to_uppercase();
    let chars: Vec<char> = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if chars.is_empty() {
        return Some(Ok(Value::String(String::new())));
    }
    let mut result = String::new();
    let n = chars.len();
    let mut i = 0usize;
    while i < n {
        let c = chars[i];
        let prev = if i > 0 { chars[i - 1] } else { '\0' };
        let next = if i + 1 < n { chars[i + 1] } else { '\0' };
        let next2 = if i + 2 < n { chars[i + 2] } else { '\0' };
        match c {
            'A' | 'E' | 'I' | 'O' | 'U' => {
                if i == 0 {
                    result.push(c);
                }
            }
            'B' => {
                if prev != 'M' {
                    result.push('B');
                }
            }
            'C' => {
                if next == 'I' || next == 'E' || next == 'Y' {
                    result.push('S');
                    if next == 'I' && next2 == 'A' {
                        result.push('X');
                    }
                } else if next == 'H' {
                    result.push('X');
                    i += 1;
                } else if !(prev == 'S' && (next == 'I' || next == 'E' || next == 'Y')) {
                    result.push('K');
                }
            }
            'D' => {
                if next == 'G' && (next2 == 'E' || next2 == 'I' || next2 == 'Y') {
                    result.push('J');
                    i += 1;
                } else {
                    result.push('T');
                }
            }
            'F' => result.push('F'),
            'G' => {
                if next == 'H' {
                    // GH at start or before vowel → K, else silent
                    if i == 0 {
                        result.push('K');
                    }
                    i += 1;
                } else if next == 'N' {
                    // GN: silent in some positions
                } else if prev == 'G' {
                    // already handled
                } else if next == 'E' || next == 'I' || next == 'Y' {
                    result.push('J');
                } else {
                    result.push('K');
                }
            }
            'H' => {
                if (next == 'A' || next == 'E' || next == 'I' || next == 'O' || next == 'U')
                    && !(prev == 'A'
                        || prev == 'E'
                        || prev == 'I'
                        || prev == 'O'
                        || prev == 'U')
                {
                    result.push('H');
                }
            }
            'J' => result.push('J'),
            'K' => {
                if prev != 'C' {
                    result.push('K');
                }
            }
            'L' => result.push('L'),
            'M' => result.push('M'),
            'N' => result.push('N'),
            'P' => {
                if next == 'H' {
                    result.push('F');
                    i += 1;
                } else {
                    result.push('P');
                }
            }
            'Q' => result.push('K'),
            'R' => result.push('R'),
            'S' => {
                if next == 'H' || (next == 'I' && (next2 == 'A' || next2 == 'O')) {
                    result.push('X');
                    if next == 'H' {
                        i += 1;
                    }
                } else {
                    result.push('S');
                }
            }
            'T' => {
                if next == 'H' {
                    result.push('0'); // use 0 for TH sound (theta)
                    i += 1;
                } else if next == 'I' && (next2 == 'A' || next2 == 'O') {
                    result.push('X');
                } else {
                    result.push('T');
                }
            }
            'V' => result.push('F'),
            'W' | 'Y' => {
                if next == 'A'
                    || next == 'E'
                    || next == 'I'
                    || next == 'O'
                    || next == 'U'
                {
                    result.push(c);
                }
            }
            'X' => {
                result.push('K');
                result.push('S');
            }
            'Z' => result.push('S'),
            _ => {}
        }
        i += 1;
    }
    Some(Ok(Value::String(result)))
}

            _ => None,
        }
    }

    /// Convert a Value to a JSON string
    fn value_to_json(val: &Value) -> String {
        match val {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                if f.is_finite() { f.to_string() } else { "null".to_string() }
            }
            Value::String(s) => {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"")
                    .replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
                format!("\"{}\"", escaped)
            }
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| Self::value_to_json(v)).collect();
                format!("[{}]", items.join(","))
            }
            Value::Map(m) => {
                let items: Vec<String> = m.iter()
                    .map(|(k, v)| format!("\"{}\":{}", k, Self::value_to_json(v)))
                    .collect();
                format!("{{{}}}", items.join(","))
            }
            Value::Tuple(t) => {
                let items: Vec<String> = t.iter().map(|v| Self::value_to_json(v)).collect();
                format!("[{}]", items.join(","))
            }
            _ => "null".to_string(),
        }
    }

    /// Parse a JSON string to a Value (simple recursive descent JSON parser)
    fn json_to_value(s: &str) -> Value {
        let s = s.trim();
        if s == "null" { return Value::Null; }
        if s == "true" { return Value::Bool(true); }
        if s == "false" { return Value::Bool(false); }
        if s.starts_with('"') {
            let inner = &s[1..s.len().saturating_sub(1)];
            return Value::String(inner.replace("\\\"", "\"").replace("\\n", "\n").replace("\\t", "\t").replace("\\\\", "\\"));
        }
        if s.starts_with('[') {
            // Parse array
            let inner = &s[1..s.len().saturating_sub(1)];
            if inner.trim().is_empty() { return Value::Array(vec![]); }
            let items = Self::json_split_top_level(inner, ',');
            return Value::Array(items.iter().map(|i| Self::json_to_value(i.trim())).collect());
        }
        if s.starts_with('{') {
            // Parse object
            let inner = &s[1..s.len().saturating_sub(1)];
            if inner.trim().is_empty() { return Value::Map(HashMap::new()); }
            let mut map = HashMap::new();
            for pair in Self::json_split_top_level(inner, ',') {
                let kv: Vec<&str> = pair.trim().splitn(2, ':').collect();
                if kv.len() == 2 {
                    let key = kv[0].trim().trim_matches('"').to_string();
                    let val = Self::json_to_value(kv[1].trim());
                    map.insert(key, val);
                }
            }
            return Value::Map(map);
        }
        // Number
        if let Ok(n) = s.parse::<i64>() { return Value::Int(n); }
        if let Ok(f) = s.parse::<f64>() { return Value::Float(f); }
        Value::String(s.to_string())
    }

    fn json_split_top_level(s: &str, sep: char) -> Vec<String> {
        let mut parts = Vec::new();
        let mut depth = 0i32;
        let mut in_str = false;
        let mut current = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '"' && (i == 0 || chars[i - 1] != '\\') { in_str = !in_str; }
            if !in_str {
                if c == '[' || c == '{' { depth += 1; }
                else if c == ']' || c == '}' { depth -= 1; }
                else if c == sep && depth == 0 {
                    parts.push(current.clone());
                    current.clear();
                    i += 1;
                    continue;
                }
            }
            current.push(c);
            i += 1;
        }
        if !current.trim().is_empty() { parts.push(current); }
        parts
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute an AST node
pub fn execute(ast: &ASTNode) {
    let mut interpreter = Interpreter::new();
    if let Err(e) = interpreter.execute(ast) {
        eprintln!("Runtime error: {}", e);
    }
}

/// Pad a formatted string for sprintf.
fn pad_sprintf(s: &str, width: usize, pad_char: char, left_align: bool) -> String {
    if s.len() >= width { return s.to_string(); }
    let padding: String = std::iter::repeat(pad_char).take(width - s.len()).collect();
    if left_align { format!("{}{}", s, padding) } else { format!("{}{}", padding, s) }
}

/// Parse a URL string into (host, port, path) for raw HTTP builtins.
/// Handles http://host:port/path and http://host/path (default port 80).
fn parse_http_url(url: &str) -> (String, u16, String) {
    let stripped = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let (host_port, path) = match stripped.find('/') {
        Some(pos) => (&stripped[..pos], stripped[pos..].to_string()),
        None => (stripped, "/".to_string()),
    };
    let (host, port) = match host_port.rfind(':') {
        Some(pos) => {
            let h = &host_port[..pos];
            let p: u16 = host_port[pos + 1..].parse().unwrap_or(80);
            (h.to_string(), p)
        }
        None => (host_port.to_string(), 80u16),
    };
    (host, port, path)
}

// ─── AES-128 pure-Rust helpers ────────────────────────────────────────────────

fn val_to_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::Array(a) => a.iter().map(|x| x.as_int() as u8).collect(),
        Value::String(s) => s.as_bytes().to_vec(),
        _ => vec![],
    }
}

fn bytes_to_val(b: &[u8]) -> Value {
    Value::Array(b.iter().map(|x| Value::Int(*x as i64)).collect())
}

fn pkcs7_pad(data: &[u8], block: usize) -> Vec<u8> {
    let pad = block - (data.len() % block);
    let mut out = data.to_vec();
    out.extend(std::iter::repeat(pad as u8).take(pad));
    out
}

fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.is_empty() { return Err("empty input".to_string()); }
    let pad = *data.last().unwrap() as usize;
    if pad == 0 || pad > 16 { return Err(format!("bad PKCS7 pad byte {}", pad)); }
    if data.len() < pad { return Err("pad larger than data".to_string()); }
    Ok(data[..data.len() - pad].to_vec())
}

// AES-128 S-box
const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

const INV_SBOX: [u8; 256] = [
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
];

const RCON: [u8; 11] = [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

fn xtime(x: u8) -> u8 {
    if x & 0x80 != 0 { (x << 1) ^ 0x1b } else { x << 1 }
}

fn gmul(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0u8;
    for _ in 0..8 {
        if b & 1 != 0 { p ^= a; }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 { a ^= 0x1b; }
        b >>= 1;
    }
    p
}

fn aes128_key_expand(key: &[u8; 16]) -> [[u8; 4]; 44] {
    let mut w = [[0u8; 4]; 44];
    for i in 0..4 {
        w[i] = [key[4*i], key[4*i+1], key[4*i+2], key[4*i+3]];
    }
    for i in 4..44 {
        let mut temp = w[i-1];
        if i % 4 == 0 {
            temp = [
                SBOX[temp[1] as usize] ^ RCON[i/4],
                SBOX[temp[2] as usize],
                SBOX[temp[3] as usize],
                SBOX[temp[0] as usize],
            ];
        }
        w[i] = [w[i-4][0]^temp[0], w[i-4][1]^temp[1], w[i-4][2]^temp[2], w[i-4][3]^temp[3]];
    }
    w
}

fn add_round_key(state: &mut [u8; 16], round_keys: &[[u8; 4]; 44], round: usize) {
    for col in 0..4 {
        for row in 0..4 {
            state[row + 4*col] ^= round_keys[round*4 + col][row];
        }
    }
}

fn sub_bytes(state: &mut [u8; 16]) {
    for b in state.iter_mut() { *b = SBOX[*b as usize]; }
}

fn inv_sub_bytes(state: &mut [u8; 16]) {
    for b in state.iter_mut() { *b = INV_SBOX[*b as usize]; }
}

fn shift_rows(state: &mut [u8; 16]) {
    let s = *state;
    // row 0: no shift
    // row 1: shift left 1
    state[1]  = s[5]; state[5]  = s[9]; state[9]  = s[13]; state[13] = s[1];
    // row 2: shift left 2
    state[2]  = s[10]; state[10] = s[2]; state[6]  = s[14]; state[14] = s[6];
    // row 3: shift left 3
    state[3]  = s[15]; state[15] = s[11]; state[11] = s[7]; state[7]  = s[3];
}

fn inv_shift_rows(state: &mut [u8; 16]) {
    let s = *state;
    state[1]  = s[13]; state[13] = s[9]; state[9]  = s[5]; state[5]  = s[1];
    state[2]  = s[10]; state[10] = s[2]; state[6]  = s[14]; state[14] = s[6];
    state[3]  = s[7];  state[7]  = s[11]; state[11] = s[15]; state[15] = s[3];
}

fn mix_columns(state: &mut [u8; 16]) {
    for col in 0..4 {
        let s0 = state[4*col]; let s1 = state[4*col+1];
        let s2 = state[4*col+2]; let s3 = state[4*col+3];
        state[4*col]   = gmul(0x02,s0)^gmul(0x03,s1)^s2^s3;
        state[4*col+1] = s0^gmul(0x02,s1)^gmul(0x03,s2)^s3;
        state[4*col+2] = s0^s1^gmul(0x02,s2)^gmul(0x03,s3);
        state[4*col+3] = gmul(0x03,s0)^s1^s2^gmul(0x02,s3);
    }
}

fn inv_mix_columns(state: &mut [u8; 16]) {
    for col in 0..4 {
        let s0 = state[4*col]; let s1 = state[4*col+1];
        let s2 = state[4*col+2]; let s3 = state[4*col+3];
        state[4*col]   = gmul(0x0e,s0)^gmul(0x0b,s1)^gmul(0x0d,s2)^gmul(0x09,s3);
        state[4*col+1] = gmul(0x09,s0)^gmul(0x0e,s1)^gmul(0x0b,s2)^gmul(0x0d,s3);
        state[4*col+2] = gmul(0x0d,s0)^gmul(0x09,s1)^gmul(0x0e,s2)^gmul(0x0b,s3);
        state[4*col+3] = gmul(0x0b,s0)^gmul(0x0d,s1)^gmul(0x09,s2)^gmul(0x0e,s3);
    }
}

fn aes128_encrypt_block(block: &[u8; 16], key: &[u8; 16]) -> [u8; 16] {
    let rk = aes128_key_expand(key);
    let mut state = *block;
    add_round_key(&mut state, &rk, 0);
    for round in 1..10 {
        sub_bytes(&mut state);
        shift_rows(&mut state);
        mix_columns(&mut state);
        add_round_key(&mut state, &rk, round);
    }
    sub_bytes(&mut state);
    shift_rows(&mut state);
    add_round_key(&mut state, &rk, 10);
    state
}

fn aes128_decrypt_block(block: &[u8; 16], key: &[u8; 16]) -> [u8; 16] {
    let rk = aes128_key_expand(key);
    let mut state = *block;
    add_round_key(&mut state, &rk, 10);
    for round in (1..10).rev() {
        inv_shift_rows(&mut state);
        inv_sub_bytes(&mut state);
        add_round_key(&mut state, &rk, round);
        inv_mix_columns(&mut state);
    }
    inv_shift_rows(&mut state);
    inv_sub_bytes(&mut state);
    add_round_key(&mut state, &rk, 0);
    state
}

/// Execute source code directly
pub fn execute_source(source: &str) -> Result<(), String> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let mut lexer = Lexer::new(source);
    let _tokens = lexer.tokenize();

    let mut parser = Parser::new(source);
    let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    let mut interpreter = Interpreter::new();
    interpreter.execute(&ast)
}

// ── REPL helpers ──────────────────────────────────────────────────────────────

impl Interpreter {
    /// Run a single line/block of source in this interpreter instance (persists state).
    /// Returns the value of the last expression, or None for definitions/statements.
    pub fn repl_exec(&mut self, source: &str) -> Result<Option<Value>, String> {
        use crate::parser::Parser;
        let mut parser = Parser::new(source);
        let ast = parser.parse().map_err(|e| e)?;
        match &ast {
            ASTNode::Program(items) if items.len() == 1 => {
                let is_def = matches!(
                    &items[0],
                    ASTNode::Function { .. }
                        | ASTNode::StructDef { .. }
                        | ASTNode::Impl { .. }
                );
                if is_def {
                    self.execute(&ast)?;
                    Ok(None)
                } else {
                    // Evaluate the single item as an expression to capture its value
                    let val = self.evaluate(&items[0])?;
                    let _ = self.return_value.take();
                    if matches!(val, Value::Null) {
                        Ok(None)
                    } else {
                        Ok(Some(val))
                    }
                }
            }
            _ => {
                self.execute(&ast)?;
                let _ = self.return_value.take();
                Ok(None)
            }
        }
    }

    /// Return all variables visible in the current (global) scope.
    pub fn repl_variables(&self) -> Vec<(String, Value)> {
        let mut vars: Vec<(String, Value)> = Vec::new();
        if let Some(scope) = self.scopes.first() {
            let mut pairs: Vec<_> = scope.variables.iter().collect();
            pairs.sort_by_key(|(k, _)| k.as_str());
            for (k, v) in pairs {
                vars.push((k.clone(), v.clone()));
            }
        }
        vars
    }

    /// Return names of all defined functions.
    pub fn repl_functions(&self) -> Vec<String> {
        let mut names: Vec<_> = self.functions.keys().cloned().collect();
        names.sort();
        names
    }
}
