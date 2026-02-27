//! Knull JIT Execution Engine
//!
//! This module provides JIT compilation and execution of Knull code.

use anyhow::{bail, Result};
use std::collections::HashMap;

use crate::compiler::CompiledModule;

/// JIT Executor
pub struct JitExecutor {
    #[cfg(feature = "llvm")]
    context: Option<inkwell::context::Context>,
    #[cfg(feature = "llvm")]
    execution_engine:
        Option<inkwell::execution_engine::ExecutionEngine<inkwell::execution_engine::Legacy贾科莫>>,
}

impl JitExecutor {
    pub fn new() -> Self {
        #[cfg(feature = "llvm")]
        {
            let context = inkwell::context::Context::create();
            JitExecutor {
                context: Some(context),
                execution_engine: None,
            }
        }
        #[cfg(not(feature = "llvm"))]
        {
            JitExecutor {}
        }
    }

    /// Execute a compiled module
    #[cfg(feature = "llvm")]
    pub fn execute(&mut self, module: &CompiledModule, args: Vec<String>) -> Result<()> {
        use inkwell::execution_engine::ExecutionEngine;
        use inkwell::module::Module;
        use inkwell::targets::{InitializationConfig, Target};

        // Initialize LLVM targets
        Target::initialize_native(&InitializationConfig::default())
            .expect("Failed to initialize native target");

        // Get the context
        let context = self.context.take().context("No LLVM context available")?;

        // Parse the LLVM IR
        let llvm_module = context
            .create_module_from_ir(
                inkwell::module::Module::parse_bitcode_from_memory(&module.llvm_ir(), &context)
                    .map_err(|e| anyhow::anyhow!("Failed to parse IR: {:?}", e))?,
            )
            .map_err(|e| anyhow::anyhow!("Failed to create module: {:?}", e))?;

        // Create execution engine
        let execution_engine = llvm_module
            .create_jit_execution_engine(inkwell::optimization_level::OptimizationLevel::Aggressive)
            .map_err(|e| anyhow::anyhow!("Failed to create JIT: {:?}", e))?;

        // Look up the main function
        let main_func = execution_engine
            .get_function::<unsafe extern "C" fn(i32, *const *const i8) -> i32>("main")
            .map_err(|e| anyhow::anyhow!("Failed to find main function: {:?}", e))?;

        // Prepare arguments
        let argc = args.len() as i32;

        // Create argument pointers
        let mut arg_ptrs: Vec<*const i8> = Vec::new();
        for arg in &args {
            let c_string = std::ffi::CString::new(arg.as_str())?;
            arg_ptrs.push(c_string.into_raw());
        }
        let argv = arg_ptrs.as_ptr();

        // Execute main
        unsafe {
            let result = main_func.call(argc, argv);
            if result != 0 {
                println!("Program exited with code: {}", result);
            }
        }

        // Cleanup
        for ptr in arg_ptrs {
            let _ = std::ffi::CString::from_raw(ptr as *mut i8);
        }

        // Put context back
        self.context = Some(context);

        Ok(())
    }

    #[cfg(not(feature = "llvm"))]
    pub fn execute(&mut self, module: &CompiledModule, args: Vec<String>) -> Result<()> {
        // Fallback: just print IR
        println!("=== LLVM IR ===");
        println!("{}", module.llvm_ir());
        println!("================");

        if args.is_empty() {
            println!("[JIT not available - install with llvm feature]");
        }

        Ok(())
    }

    /// Execute a function by name
    #[cfg(feature = "llvm")]
    pub fn execute_function(
        &self,
        name: &str,
        args: &[inkwell::values::BasicValueEnum],
    ) -> Result<inkwell::values::BasicValueEnum> {
        let ee = self
            .execution_engine
            .as_ref()
            .context("No execution engine")?;

        let function = ee
            .get_function(name)
            .map_err(|e| anyhow::anyhow!("Function not found: {}", e))?;

        let result = unsafe { function.call(args) };

        Ok(result)
    }

    /// Add a global function callback
    #[cfg(feature = "llvm")]
    pub fn add_global(&self, name: &str, func: inkwell::module::Module::Function) -> Result<()> {
        let ee = self
            .execution_engine
            .as_ref()
            .context("No execution engine")?;

        ee.add_global_mapping(&func, name.as_ptr() as usize);

        Ok(())
    }
}

/// Simple interpreter fallback
pub struct Interpreter {
    functions: HashMap<String, InterpretedFunction>,
    stack: Vec<StackFrame>,
}

#[derive(Clone)]
struct InterpretedFunction {
    params: Vec<String>,
    body: Vec<Instruction>,
}

#[derive(Clone)]
enum Instruction {
    Constant(i64),
    Add,
    Sub,
    Mul,
    Div,
    Print,
    Return,
    Call(String),
    Jump(String),
    Branch(String, String),
}

struct StackFrame {
    locals: HashMap<String, i64>,
    pc: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            functions: HashMap::new(),
            stack: Vec::new(),
        }
    }

    pub fn execute(&mut self, module: &CompiledModule) -> Result<()> {
        // Simple interpretation of IR
        // This is a fallback when JIT is not available

        // Find main function
        for func in &module.functions {
            if func.name == "main" {
                self.execute_function(&func)?;
                return Ok(());
            }
        }

        bail!("No main function found");
    }

    fn execute_function(&mut self, func: &crate::compiler::IrFunction) -> Result<()> {
        // Execute function body
        for block in &func.blocks {
            for inst in &block.instructions {
                self.execute_instruction(inst)?;
            }
        }
        Ok(())
    }

    fn execute_instruction(&mut self, inst: &crate::compiler::IrInstruction) -> Result<()> {
        match inst {
            crate::compiler::IrInstruction::Constant { dest, value } => {
                let val = match value {
                    crate::compiler::IrValue::Int(i) => *i,
                    crate::compiler::IrValue::Float(f) => *f as i64,
                    crate::compiler::IrValue::String(s) => {
                        println!("{}", s);
                        0
                    }
                };

                if let Some(frame) = self.stack.last_mut() {
                    frame.locals.insert(dest.clone(), val);
                }
            }
            crate::compiler::IrInstruction::Binary {
                dest,
                op,
                left,
                right,
            } => {
                let l = self.get_local(left);
                let r = self.get_local(right);

                let result = match op {
                    crate::compiler::IrBinaryOp::Add => l + r,
                    crate::compiler::IrBinaryOp::Sub => l - r,
                    crate::compiler::IrBinaryOp::Mul => l * r,
                    crate::compiler::IrBinaryOp::Div => l / r,
                    _ => l,
                };

                if let Some(frame) = self.stack.last_mut() {
                    frame.locals.insert(dest.clone(), result);
                }
            }
            crate::compiler::IrInstruction::Ret { .. } => {
                // Return
            }
            crate::compiler::IrInstruction::RetVoid => {
                // Void return
            }
            crate::compiler::IrInstruction::Call { dest, func, args } => {
                println!("Calling function: {}", func);
            }
            _ => {}
        }

        Ok(())
    }

    fn get_local(&self, name: &str) -> i64 {
        if let Some(frame) = self.stack.last() {
            *frame.locals.get(name).unwrap_or(&0)
        } else {
            0
        }
    }
}

/// FFI helper for calling Knull from C
#[repr(C)]
pub struct KnullRuntime {
    pub execute: extern "C" fn(*const u8) -> i32,
    pub get_global: extern "C" fn(*const u8) -> *const u8,
    pub set_global: extern "C" fn(*const u8, *const u8),
}

impl KnullRuntime {
    pub fn new() -> Self {
        KnullRuntime {
            execute: Self::execute_placeholder,
            get_global: Self::get_global_placeholder,
            set_global: Self::set_global_placeholder,
        }
    }

    extern "C" fn execute_placeholder(_: *const u8) -> i32 {
        -1
    }

    extern "C" fn get_global_placeholder(_: *const u8) -> *const u8 {
        std::ptr::null()
    }

    extern "C" fn set_global_placeholder(_: *const u8, _: *const u8) {}
}
