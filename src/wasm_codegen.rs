//! Knull WebAssembly Code Generator
//!
//! Compiles Knull AST to WebAssembly binary format.

use std::collections::HashMap;
use std::fs;
use std::io::Write;

use crate::parser::{ASTNode, Literal};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmType {
    I32,
    I64,
    F32,
    F64,
    V128,
}

impl WasmType {
    fn to_byte(&self) -> u8 {
        match self {
            WasmType::I32 => 0x7F,
            WasmType::I64 => 0x7E,
            WasmType::F32 => 0x7D,
            WasmType::F64 => 0x7C,
            WasmType::V128 => 0x7B,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WasmFunc {
    pub type_idx: u32,
    pub locals: Vec<WasmType>,
    pub body: Vec<WasmInstr>,
}

#[derive(Debug, Clone)]
pub enum WasmInstr {
    Unreachable,
    Nop,
    Block(Option<WasmType>),
    Loop(Option<WasmType>),
    If(Option<WasmType>),
    Else,
    End,
    Br(u32),
    BrIf(u32),
    Return,
    Call(u32),
    I32Load(u32, u32),
    I32Store(u32, u32),
    I64Load(u32, u32),
    I64Store(u32, u32),
    F32Load(u32, u32),
    F32Store(u32, u32),
    F64Load(u32, u32),
    F64Store(u32, u32),
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    I32Eqz,
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32LeS,
    I32LeU,
    I32GtS,
    I32GtU,
    I32GeS,
    I32GeU,
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64LeS,
    I64LeU,
    I64GtS,
    I64GtU,
    I64GeS,
    I64GeU,
    F32Eq,
    F32Ne,
    F32Lt,
    F32Le,
    F32Gt,
    F32Ge,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Le,
    F64Gt,
    F64Ge,
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),
    Drop,
    Select,
}

pub struct WasmModule {
    pub types: Vec<Vec<WasmType>>,
    pub funcs: Vec<WasmFunc>,
    pub memories: Vec<WasmMemory>,
    pub globals: Vec<WasmGlobal>,
    pub exports: HashMap<String, WasmExport>,
    pub data: Vec<u8>,
    pub start_func: Option<u32>,
}

impl Clone for WasmModule {
    fn clone(&self) -> Self {
        WasmModule {
            types: self.types.clone(),
            funcs: self.funcs.clone(),
            memories: self.memories.clone(),
            globals: self.globals.clone(),
            exports: self.exports.clone(),
            data: self.data.clone(),
            start_func: self.start_func,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WasmMemory {
    pub min: u32,
    pub max: Option<u32>,
    pub is_shared: bool,
}

#[derive(Debug, Clone)]
pub struct WasmGlobal {
    pub ty: WasmType,
    pub mutability: bool,
    pub init: Vec<WasmInstr>,
}

#[derive(Debug, Clone)]
pub struct WasmExport {
    pub kind: u8,
    pub index: u32,
}

impl Default for WasmModule {
    fn default() -> Self {
        let mut module = WasmModule {
            types: Vec::new(),
            funcs: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            exports: HashMap::new(),
            data: Vec::new(),
            start_func: None,
        };
        module.types.push(vec![]);
        module
    }
}

impl WasmModule {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_type(&mut self, params: Vec<WasmType>, results: Vec<WasmType>) -> u32 {
        let mut full_type = params;
        full_type.extend(results);
        self.types.push(full_type);
        (self.types.len() - 1) as u32
    }

    pub fn add_func(&mut self, func: WasmFunc) -> u32 {
        self.funcs.push(func);
        (self.funcs.len() - 1) as u32
    }

    pub fn add_memory(&mut self, min: u32, max: Option<u32>) -> u32 {
        self.memories.push(WasmMemory {
            min,
            max,
            is_shared: false,
        });
        (self.memories.len() - 1) as u32
    }

    pub fn add_global(&mut self, ty: WasmType, mutability: bool, init: Vec<WasmInstr>) -> u32 {
        self.globals.push(WasmGlobal {
            ty,
            mutability,
            init,
        });
        (self.globals.len() - 1) as u32
    }

    pub fn export(&mut self, name: &str, kind: u8, index: u32) {
        self.exports
            .insert(name.to_string(), WasmExport { kind, index });
    }

    pub fn to_binary(&self) -> Vec<u8> {
        let mut binary = Vec::new();
        binary.extend_from_slice(b"\0asm");
        binary.extend_from_slice(&1u16.to_le_bytes());
        binary.extend_from_slice(&0u16.to_le_bytes());
        let mut section_buf = Vec::new();
        self.write_type_section(&mut section_buf);
        if !section_buf.is_empty() {
            self.write_section(1, section_buf, &mut binary);
        }
        let mut func_section = Vec::new();
        for func in &self.funcs {
            func_section.push(func.type_idx as u8);
        }
        if !func_section.is_empty() {
            self.write_section(3, func_section, &mut binary);
        }
        if !self.memories.is_empty() {
            let mut mem_section = Vec::new();
            mem_section.push(self.memories.len() as u8);
            for mem in &self.memories {
                let limits = encode_limits(mem.min, mem.max);
                mem_section.extend_from_slice(&limits);
            }
            self.write_section(5, mem_section, &mut binary);
        }
        if !self.globals.is_empty() {
            let mut global_section = Vec::new();
            for global in &self.globals {
                global_section.push(global.ty.to_byte());
                global_section.push(if global.mutability { 1 } else { 0 });
                for instr in &global.init {
                    self.write_instr(instr, &mut global_section);
                }
                global_section.push(0x0B);
            }
            self.write_section(6, global_section, &mut binary);
        }
        if !self.exports.is_empty() {
            let mut export_section = Vec::new();
            export_section.push(self.exports.len() as u8);
            for (name, export) in &self.exports {
                export_section.push(name.len() as u8);
                export_section.extend_from_slice(name.as_bytes());
                export_section.push(export.kind);
                export_section.extend_from_slice(&export.index.to_le_bytes());
            }
            self.write_section(7, export_section, &mut binary);
        }
        if let Some(start) = self.start_func {
            let mut start_section = Vec::new();
            start_section.extend_from_slice(&start.to_le_bytes());
            self.write_section(8, start_section, &mut binary);
        }
        if !self.data.is_empty() {
            let mut data_section = Vec::new();
            data_section.push(1u8);
            data_section.push(0u8);
            data_section.extend_from_slice(&encode_varuint(self.data.len() as u32));
            data_section.extend_from_slice(&self.data);
            self.write_section(11, data_section, &mut binary);
        }
        binary
    }

    fn write_type_section(&self, buf: &mut Vec<u8>) {
        if self.types.is_empty() {
            return;
        }
        buf.push(self.types.len() as u8);
        for t in &self.types {
            let params = &t[..t.len().saturating_sub(if t.is_empty() { 0 } else { 1 })];
            let results = if t.len() > params.len() {
                &t[params.len()..]
            } else {
                &[]
            };
            buf.push(params.len() as u8);
            for p in params {
                buf.push(p.to_byte());
            }
            buf.push(results.len() as u8);
            for r in results {
                buf.push(r.to_byte());
            }
        }
    }

    fn write_section(&self, id: u8, contents: Vec<u8>, binary: &mut Vec<u8>) {
        binary.push(id);
        binary.extend_from_slice(&encode_varuint(contents.len() as u32));
        binary.extend_from_slice(&contents);
    }

    fn write_instr(&self, instr: &WasmInstr, buf: &mut Vec<u8>) {
        match instr {
            WasmInstr::Unreachable => buf.push(0x00),
            WasmInstr::Nop => buf.push(0x01),
            WasmInstr::Block(ty) => {
                buf.push(0x02);
                buf.push(ty.as_ref().map_or(0x40, |t| t.to_byte()));
            }
            WasmInstr::Loop(ty) => {
                buf.push(0x03);
                buf.push(ty.as_ref().map_or(0x40, |t| t.to_byte()));
            }
            WasmInstr::If(ty) => {
                buf.push(0x04);
                buf.push(ty.as_ref().map_or(0x40, |t| t.to_byte()));
            }
            WasmInstr::Else => buf.push(0x05),
            WasmInstr::End => buf.push(0x0B),
            WasmInstr::Br(label) => {
                buf.push(0x0C);
                buf.extend_from_slice(&encode_varuint(*label));
            }
            WasmInstr::BrIf(label) => {
                buf.push(0x0D);
                buf.extend_from_slice(&encode_varuint(*label));
            }
            WasmInstr::Return => buf.push(0x0F),
            WasmInstr::Call(idx) => {
                buf.push(0x10);
                buf.extend_from_slice(&encode_varuint(*idx));
            }
            WasmInstr::I32Load(align, offset) => {
                buf.push(0x28);
                buf.extend_from_slice(&encode_varuint(*align));
                buf.extend_from_slice(&encode_varuint(*offset));
            }
            WasmInstr::I32Store(align, offset) => {
                buf.push(0x36);
                buf.extend_from_slice(&encode_varuint(*align));
                buf.extend_from_slice(&encode_varuint(*offset));
            }
            WasmInstr::I64Load(align, offset) => {
                buf.push(0x29);
                buf.extend_from_slice(&encode_varuint(*align));
                buf.extend_from_slice(&encode_varuint(*offset));
            }
            WasmInstr::I64Store(align, offset) => {
                buf.push(0x37);
                buf.extend_from_slice(&encode_varuint(*align));
                buf.extend_from_slice(&encode_varuint(*offset));
            }
            WasmInstr::F32Load(align, offset) => {
                buf.push(0x2A);
                buf.extend_from_slice(&encode_varuint(*align));
                buf.extend_from_slice(&encode_varuint(*offset));
            }
            WasmInstr::F32Store(align, offset) => {
                buf.push(0x38);
                buf.extend_from_slice(&encode_varuint(*align));
                buf.extend_from_slice(&encode_varuint(*offset));
            }
            WasmInstr::F64Load(align, offset) => {
                buf.push(0x2B);
                buf.extend_from_slice(&encode_varuint(*align));
                buf.extend_from_slice(&encode_varuint(*offset));
            }
            WasmInstr::F64Store(align, offset) => {
                buf.push(0x39);
                buf.extend_from_slice(&encode_varuint(*align));
                buf.extend_from_slice(&encode_varuint(*offset));
            }
            WasmInstr::I32Add => buf.push(0x6A),
            WasmInstr::I32Sub => buf.push(0x6B),
            WasmInstr::I32Mul => buf.push(0x6C),
            WasmInstr::I32DivS => buf.push(0x6D),
            WasmInstr::I32DivU => buf.push(0x6E),
            WasmInstr::I32RemS => buf.push(0x6F),
            WasmInstr::I32RemU => buf.push(0x70),
            WasmInstr::I64Add => buf.push(0x7C),
            WasmInstr::I64Sub => buf.push(0x7D),
            WasmInstr::I64Mul => buf.push(0x7E),
            WasmInstr::I64DivS => buf.push(0x7F),
            WasmInstr::I64DivU => buf.push(0x80),
            WasmInstr::I64RemS => buf.push(0x81),
            WasmInstr::I64RemU => buf.push(0x82),
            WasmInstr::F32Add => buf.push(0x92),
            WasmInstr::F32Sub => buf.push(0x93),
            WasmInstr::F32Mul => buf.push(0x94),
            WasmInstr::F32Div => buf.push(0x95),
            WasmInstr::F64Add => buf.push(0xA0),
            WasmInstr::F64Sub => buf.push(0xA1),
            WasmInstr::F64Mul => buf.push(0xA2),
            WasmInstr::F64Div => buf.push(0xA3),
            WasmInstr::I32Eqz => buf.push(0x45),
            WasmInstr::I32Eq => buf.push(0x46),
            WasmInstr::I32Ne => buf.push(0x47),
            WasmInstr::I32LtS => buf.push(0x48),
            WasmInstr::I32LtU => buf.push(0x49),
            WasmInstr::I32LeS => buf.push(0x4C),
            WasmInstr::I32LeU => buf.push(0x4D),
            WasmInstr::I32GtS => buf.push(0x4A),
            WasmInstr::I32GtU => buf.push(0x4B),
            WasmInstr::I32GeS => buf.push(0x4E),
            WasmInstr::I32GeU => buf.push(0x4F),
            WasmInstr::I64Eqz => buf.push(0x50),
            WasmInstr::I64Eq => buf.push(0x51),
            WasmInstr::I64Ne => buf.push(0x52),
            WasmInstr::I64LtS => buf.push(0x53),
            WasmInstr::I64LtU => buf.push(0x54),
            WasmInstr::I64LeS => buf.push(0x57),
            WasmInstr::I64LeU => buf.push(0x58),
            WasmInstr::I64GtS => buf.push(0x55),
            WasmInstr::I64GtU => buf.push(0x56),
            WasmInstr::I64GeS => buf.push(0x59),
            WasmInstr::I64GeU => buf.push(0x5A),
            WasmInstr::F32Eq => buf.push(0x5B),
            WasmInstr::F32Ne => buf.push(0x5C),
            WasmInstr::F32Lt => buf.push(0x5D),
            WasmInstr::F32Le => buf.push(0x5E),
            WasmInstr::F32Gt => buf.push(0x5F),
            WasmInstr::F32Ge => buf.push(0x60),
            WasmInstr::F64Eq => buf.push(0x61),
            WasmInstr::F64Ne => buf.push(0x62),
            WasmInstr::F64Lt => buf.push(0x63),
            WasmInstr::F64Le => buf.push(0x64),
            WasmInstr::F64Gt => buf.push(0x65),
            WasmInstr::F64Ge => buf.push(0x66),
            WasmInstr::I32Const(val) => {
                buf.push(0x41);
                buf.extend_from_slice(&encode_varint32(*val));
            }
            WasmInstr::I64Const(val) => {
                buf.push(0x42);
                buf.extend_from_slice(&encode_varint64(*val));
            }
            WasmInstr::F32Const(val) => {
                buf.push(0x43);
                buf.extend_from_slice(&val.to_le_bytes());
            }
            WasmInstr::F64Const(val) => {
                buf.push(0x44);
                buf.extend_from_slice(&val.to_le_bytes());
            }
            WasmInstr::LocalGet(idx) => {
                buf.push(0x20);
                buf.extend_from_slice(&encode_varuint(*idx));
            }
            WasmInstr::LocalSet(idx) => {
                buf.push(0x21);
                buf.extend_from_slice(&encode_varuint(*idx));
            }
            WasmInstr::LocalTee(idx) => {
                buf.push(0x22);
                buf.extend_from_slice(&encode_varuint(*idx));
            }
            WasmInstr::GlobalGet(idx) => {
                buf.push(0x23);
                buf.extend_from_slice(&encode_varuint(*idx));
            }
            WasmInstr::GlobalSet(idx) => {
                buf.push(0x24);
                buf.extend_from_slice(&encode_varuint(*idx));
            }
            WasmInstr::Drop => buf.push(0x1A),
            WasmInstr::Select => buf.push(0x1B),
        }
    }
}

fn encode_varuint(mut n: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if n == 0 {
            break;
        }
    }
    buf
}

fn encode_varint(mut n: i32) -> Vec<u8> {
    let mut buf = Vec::new();
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if (n == 0 && (byte & 0x40) == 0) || (n == -1 && (byte & 0x40) != 0) {
            buf.push(byte);
            break;
        } else {
            byte |= 0x80;
            buf.push(byte);
        }
    }
    buf
}

fn encode_varint32(n: i32) -> Vec<u8> {
    encode_varint(n)
}

fn encode_varint64(mut n: i64) -> Vec<u8> {
    let mut buf = Vec::new();
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if (n == 0 && (byte & 0x40) == 0) || (n == -1 && (byte & 0x40) != 0) {
            buf.push(byte);
            break;
        } else {
            byte |= 0x80;
            buf.push(byte);
        }
    }
    buf
}

fn encode_limits(min: u32, max: Option<u32>) -> Vec<u8> {
    let mut buf = Vec::new();
    match max {
        Some(m) => {
            buf.push(0x01);
            buf.extend_from_slice(&encode_varuint(min));
            buf.extend_from_slice(&encode_varuint(m));
        }
        None => {
            buf.push(0x00);
            buf.extend_from_slice(&encode_varuint(min));
        }
    }
    buf
}

pub struct WasmCodeGen {
    module: WasmModule,
    func_indices: HashMap<String, u32>,
    local_types: Vec<WasmType>,
    temp_count: u32,
    string_data: Vec<u8>,
}

impl WasmCodeGen {
    pub fn new() -> Self {
        WasmCodeGen {
            module: WasmModule::new(),
            func_indices: HashMap::new(),
            local_types: Vec::new(),
            temp_count: 0,
            string_data: Vec::new(),
        }
    }

    pub fn compile(&mut self, ast: &ASTNode) -> Result<WasmModule, String> {
        self.collect_functions(ast)?;
        self.emit_module(ast)?;
        self.module.data = self.string_data.clone();
        Ok(self.module.clone())
    }

    fn collect_functions(&mut self, node: &ASTNode) -> Result<(), String> {
        if let ASTNode::Program(items) = node {
            for item in items {
                if let ASTNode::Function { name, params, .. } = item {
                    let type_idx = self.module.add_type(
                        params.iter().map(|_| WasmType::I32).collect(),
                        vec![WasmType::I32],
                    );
                    self.func_indices.insert(name.clone(), type_idx);
                }
            }
        }
        Ok(())
    }

    fn emit_module(&mut self, node: &ASTNode) -> Result<(), String> {
        if let ASTNode::Program(items) = node {
            for item in items {
                if let ASTNode::Function {
                    name, params, body, ..
                } = item
                {
                    self.emit_function(name, params, body)?;
                }
            }
        }
        Ok(())
    }

    fn emit_function(
        &mut self,
        name: &str,
        params: &[crate::parser::Param],
        body: &ASTNode,
    ) -> Result<u32, String> {
        self.local_types = params.iter().map(|_| WasmType::I32).collect();
        let mut body_instrs = Vec::new();
        self.compile_node(body, &mut body_instrs)?;
        body_instrs.push(WasmInstr::I32Const(0));
        body_instrs.push(WasmInstr::Return);
        let type_idx = self.func_indices.get(name).copied().unwrap_or(0);
        let func = WasmFunc {
            type_idx,
            locals: self.local_types.clone(),
            body: body_instrs,
        };
        let func_idx = self.module.add_func(func);
        self.module.export(name, 0, func_idx);
        if name == "_start" || name == "main" {
            self.module.start_func = Some(func_idx);
        }
        Ok(func_idx)
    }

    fn compile_node(&mut self, node: &ASTNode, instrs: &mut Vec<WasmInstr>) -> Result<(), String> {
        match node {
            ASTNode::Program(items) => {
                for item in items {
                    self.compile_node(item, instrs)?;
                }
            }
            ASTNode::Function {
                name, params, body, ..
            } => {
                let _ = self.emit_function(name, params, body);
            }
            ASTNode::Block(nodes) => {
                for node in nodes {
                    self.compile_node(node, instrs)?;
                }
            }
            ASTNode::Let { name, value, .. } => {
                self.compile_node(value, instrs)?;
                let local_idx = self.local_types.len() as u32;
                self.local_types.push(WasmType::I32);
                instrs.push(WasmInstr::LocalSet(local_idx));
            }
            ASTNode::Return(expr) => {
                self.compile_node(expr, instrs)?;
                instrs.push(WasmInstr::Return);
            }
            ASTNode::If {
                cond,
                then_body,
                else_body,
            } => {
                self.compile_node(cond, instrs)?;
                instrs.push(WasmInstr::If(None));
                self.compile_node(then_body, instrs)?;
                if let Some(else_node) = else_body {
                    instrs.push(WasmInstr::Else);
                    self.compile_node(else_node, instrs)?;
                }
                instrs.push(WasmInstr::End);
            }
            ASTNode::While { cond, body } => {
                instrs.push(WasmInstr::Loop(None));
                let loop_start = instrs.len() as u32;
                self.compile_node(cond, instrs)?;
                instrs.push(WasmInstr::BrIf(loop_start + 1));
                self.compile_node(body, instrs)?;
                instrs.push(WasmInstr::Br(loop_start));
                instrs.push(WasmInstr::End);
            }
            ASTNode::Binary { op, left, right } => {
                self.compile_node(left, instrs)?;
                self.compile_node(right, instrs)?;
                let wasm_op = match op.as_str() {
                    "+" => Some(WasmInstr::I32Add),
                    "-" => Some(WasmInstr::I32Sub),
                    "*" => Some(WasmInstr::I32Mul),
                    "/" => Some(WasmInstr::I32DivS),
                    "%" => Some(WasmInstr::I32RemS),
                    "==" => Some(WasmInstr::I32Eq),
                    "!=" => Some(WasmInstr::I32Ne),
                    "<" => Some(WasmInstr::I32LtS),
                    ">" => Some(WasmInstr::I32GtS),
                    "<=" => Some(WasmInstr::I32LeS),
                    ">=" => Some(WasmInstr::I32GeS),
                    _ => None,
                };
                if let Some(op) = wasm_op {
                    instrs.push(op);
                }
            }
            ASTNode::Unary { op, operand } => {
                self.compile_node(operand, instrs)?;
                match op.as_str() {
                    "-" => {
                        instrs.push(WasmInstr::I32Const(0));
                        instrs.push(WasmInstr::I32Sub);
                    }
                    "!" => {
                        instrs.push(WasmInstr::I32Eqz);
                    }
                    _ => {}
                }
            }
            ASTNode::Call { func, args } => {
                for arg in args {
                    self.compile_node(arg, instrs)?;
                }
                let func_name = match func.as_ref() {
                    ASTNode::Identifier(name) => name.clone(),
                    _ => return Err("Complex function calls not supported".to_string()),
                };
                if let Some(&idx) = self.func_indices.get(&func_name) {
                    instrs.push(WasmInstr::Call(idx));
                } else {
                    let builtin = self.get_builtin_idx(&func_name);
                    instrs.push(WasmInstr::Call(builtin));
                }
            }
            ASTNode::Literal(lit) => match lit {
                Literal::Int(n) => {
                    instrs.push(WasmInstr::I32Const(*n as i32));
                }
                Literal::Float(f) => {
                    instrs.push(WasmInstr::F32Const(*f as f32));
                }
                Literal::String(s) => {
                    let offset = self.string_data.len() as u32;
                    self.string_data.extend_from_slice(s.as_bytes());
                    self.string_data.push(0);
                    instrs.push(WasmInstr::I32Const(offset as i32));
                }
                Literal::Bool(b) => {
                    instrs.push(WasmInstr::I32Const(if *b { 1 } else { 0 }));
                }
                Literal::Null => {
                    instrs.push(WasmInstr::I32Const(0));
                }
            },
            ASTNode::Identifier(name) => {
                if let Some(&idx) = self.func_indices.get(name) {
                    instrs.push(WasmInstr::GlobalGet(idx));
                } else if let Some(local_idx) = self.find_local(name) {
                    instrs.push(WasmInstr::LocalGet(local_idx));
                } else {
                    instrs.push(WasmInstr::I32Const(0));
                }
            }
            ASTNode::Array(elements) => {
                let temp = self.temp_count;
                self.temp_count += 1;
                for elem in elements {
                    self.compile_node(elem, instrs)?;
                }
                instrs.push(WasmInstr::I32Const(temp as i32));
            }
            _ => {
                instrs.push(WasmInstr::Nop);
            }
        }
        Ok(())
    }

    fn find_local(&self, name: &str) -> Option<u32> {
        None
    }

    fn get_builtin_idx(&self, name: &str) -> u32 {
        match name {
            "wasi_proc_exit" => 0,
            "wasi_fd_write" => 1,
            "wasi_fd_read" => 2,
            _ => 0,
        }
    }

    fn next_temp(&mut self) -> u32 {
        let temp = self.temp_count;
        self.temp_count += 1;
        temp
    }
}

pub fn compile_to_wasm(source: &str, output_path: &str) -> Result<(), String> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let mut lexer = Lexer::new(source);
    let _tokens = lexer.tokenize();

    let mut parser = Parser::new(source);
    let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    let mut codegen = WasmCodeGen::new();
    let module = codegen.compile(&ast)?;

    let binary = module.to_binary();

    let mut file = fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    file.write_all(&binary)
        .map_err(|e| format!("Failed to write WASM file: {}", e))?;

    Ok(())
}

pub fn compile_to_wat(source: &str, output_path: &str) -> Result<(), String> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let mut lexer = Lexer::new(source);
    let _tokens = lexer.tokenize();

    let mut parser = Parser::new(source);
    let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    let mut codegen = WasmCodeGen::new();
    let _module = codegen.compile(&ast)?;

    let wat = "(module)\n";
    fs::write(output_path, wat).map_err(|e| format!("Failed to write WAT file: {}", e))?;

    Ok(())
}
