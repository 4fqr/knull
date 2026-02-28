//! Knull LLVM Code Generation Backend
//!
//! This module compiles Knull AST directly to native machine code using LLVM.
//! It supports all three modes: Novice (with GC), Expert (with ownership),
//! and God (with unsafe blocks and inline assembly).

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::{
    BasicType, BasicTypeEnum, FloatType, FunctionType, IntType, StructType, VoidType,
};
use inkwell::values::{
    BasicValue, BasicValueEnum, FloatValue, FunctionValue, IntValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;

use crate::ast::{ASTNode, BinaryOp, Literal, Type as KnullType, UnaryOp};
use crate::lexer::TokenKind;
use std::collections::HashMap;
use std::path::Path;

/// Represents the compilation mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompileMode {
    /// Novice mode: Dynamic typing with garbage collection
    Novice,
    /// Expert mode: Static typing with ownership system
    Expert,
    /// God mode: Unsafe blocks, direct memory access, inline assembly
    God,
}

/// LLVM Code Generator
pub struct LLVMCodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    target_machine: TargetMachine,

    // Symbol tables
    variables: HashMap<String, PointerValue<'ctx>>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    structs: HashMap<String, StructType<'ctx>>,

    // Current function context
    current_function: Option<FunctionValue<'ctx>>,

    // Compile mode
    mode: CompileMode,

    // For ownership tracking in Expert/God mode
    ownership_stack: Vec<OwnershipFrame>,
}

/// Tracks ownership information for variables
#[derive(Debug, Clone)]
struct OwnershipFrame {
    owned_vars: HashMap<String, OwnershipStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum OwnershipStatus {
    Owned,
    Borrowed,
    MutBorrowed,
    Moved,
}

/// Compilation result
#[derive(Debug)]
pub struct CompilationResult {
    pub output_path: String,
    pub object_path: Option<String>,
    pub executable_path: Option<String>,
}

impl<'ctx> LLVMCodeGen<'ctx> {
    /// Initialize LLVM and create a new code generator
    pub fn new(
        context: &'ctx Context,
        module_name: &str,
        mode: CompileMode,
    ) -> Result<Self, String> {
        // Initialize LLVM targets
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| format!("Failed to initialize LLVM target: {}", e))?;

        let module = context.create_module(module_name);
        let builder = context.create_builder();

        // Get target machine for native compilation
        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple)
            .map_err(|e| format!("Failed to get target from triple: {}", e))?;

        let target_machine = target
            .create_target_machine(
                &target_triple,
                TargetMachine::get_host_cpu_name()
                    .to_str()
                    .unwrap_or("generic"),
                TargetMachine::get_host_cpu_features()
                    .to_str()
                    .unwrap_or(""),
                OptimizationLevel::Default,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or("Failed to create target machine")?;

        Ok(LLVMCodeGen {
            context,
            module,
            builder,
            target_machine,
            variables: HashMap::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            current_function: None,
            mode,
            ownership_stack: vec![OwnershipFrame {
                owned_vars: HashMap::new(),
            }],
        })
    }

    /// Compile an AST node to LLVM IR
    pub fn compile(&mut self, ast: &ASTNode) -> Result<(), String> {
        match ast {
            ASTNode::Program(statements) => {
                // First pass: declare all functions and types
                for stmt in statements {
                    self.declare_top_level(stmt)?;
                }

                // Second pass: compile function bodies
                for stmt in statements {
                    self.compile_statement(stmt)?;
                }

                // Verify the module
                if let Err(e) = self.module.verify() {
                    return Err(format!("Module verification failed: {}", e.to_string()));
                }

                Ok(())
            }
            _ => self.compile_statement(ast),
        }
    }

    /// Declare top-level items (functions, structs, etc.)
    fn declare_top_level(&mut self, node: &ASTNode) -> Result<(), String> {
        match node {
            ASTNode::Function {
                name,
                params,
                return_type,
                ..
            } => {
                self.declare_function(name, params, return_type)?;
            }
            ASTNode::Struct { name, fields } => {
                self.declare_struct(name, fields)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Declare a function
    fn declare_function(
        &mut self,
        name: &str,
        params: &[(String, KnullType)],
        return_type: &KnullType,
    ) -> Result<FunctionValue<'ctx>, String> {
        // Convert Knull types to LLVM types
        let param_types: Vec<BasicTypeEnum<'ctx>> = params
            .iter()
            .map(|(_, ty)| self.knull_type_to_llvm(ty))
            .collect::<Result<Vec<_>, _>>()?;

        let fn_type = if *return_type == KnullType::Void {
            self.context.void_type().fn_type(&param_types, false)
        } else {
            let ret_ty = self.knull_type_to_llvm(return_type)?;
            ret_ty.fn_type(&param_types, false)
        };

        let function = self.module.add_function(name, fn_type, None);
        self.functions.insert(name.to_string(), function);

        Ok(function)
    }

    /// Declare a struct type
    fn declare_struct(
        &mut self,
        name: &str,
        fields: &[(String, KnullType)],
    ) -> Result<StructType<'ctx>, String> {
        let field_types: Vec<BasicTypeEnum<'ctx>> = fields
            .iter()
            .map(|(_, ty)| self.knull_type_to_llvm(ty))
            .collect::<Result<Vec<_>, _>>()?;

        let struct_type = self.context.opaque_struct_type(name);
        struct_type.set_body(&field_types, false);

        self.structs.insert(name.to_string(), struct_type);

        Ok(struct_type)
    }

    /// Convert Knull types to LLVM types
    fn knull_type_to_llvm(&self, ty: &KnullType) -> Result<BasicTypeEnum<'ctx>, String> {
        match ty {
            KnullType::Int => Ok(self.context.i64_type().into()),
            KnullType::Float => Ok(self.context.f64_type().into()),
            KnullType::Bool => Ok(self.context.bool_type().into()),
            KnullType::String => {
                // String as {i8*, i64} (ptr, len)
                let ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let len_type = self.context.i64_type();
                let str_struct = self
                    .context
                    .struct_type(&[ptr_type.into(), len_type.into()], false);
                Ok(str_struct.into())
            }
            KnullType::Custom(name) => {
                if let Some(&struct_ty) = self.structs.get(name) {
                    Ok(struct_ty.into())
                } else {
                    Err(format!("Unknown type: {}", name))
                }
            }
            KnullType::Pointer(inner) => {
                let inner_ty = self.knull_type_to_llvm(inner)?;
                Ok(inner_ty.ptr_type(AddressSpace::default()).into())
            }
            KnullType::Array(inner, size) => {
                let inner_ty = self.knull_type_to_llvm(inner)?;
                Ok(inner_ty.array_type(*size as u32).into())
            }
            _ => Err(format!("Unsupported type: {:?}", ty)),
        }
    }

    /// Compile a statement
    fn compile_statement(&mut self, node: &ASTNode) -> Result<(), String> {
        match node {
            ASTNode::Function {
                name, params, body, ..
            } => {
                self.compile_function_body(name, params, body)?;
            }
            ASTNode::Let { name, value, ty } => {
                self.compile_let(name, value, ty)?;
            }
            ASTNode::Assignment { name, value } => {
                self.compile_assignment(name, value)?;
            }
            ASTNode::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compile_if(condition, then_branch, else_branch.as_deref())?;
            }
            ASTNode::While { condition, body } => {
                self.compile_while(condition, body)?;
            }
            ASTNode::For {
                var,
                iterable,
                body,
            } => {
                self.compile_for(var, iterable, body)?;
            }
            ASTNode::Return { value } => {
                self.compile_return(value.as_deref())?;
            }
            ASTNode::Call { name, args } => {
                self.compile_call(name, args)?;
            }
            ASTNode::UnsafeBlock { body } => {
                if self.mode == CompileMode::God {
                    self.compile_unsafe_block(body)?;
                } else {
                    return Err("Unsafe blocks are only allowed in God mode".to_string());
                }
            }
            ASTNode::InlineAssembly {
                assembly,
                inputs,
                outputs,
            } => {
                if self.mode == CompileMode::God {
                    self.compile_inline_asm(assembly, inputs, outputs)?;
                } else {
                    return Err("Inline assembly is only allowed in God mode".to_string());
                }
            }
            ASTNode::Syscall { number, args } => {
                self.compile_syscall(number, args)?;
            }
            ASTNode::Block { statements } => {
                for stmt in statements {
                    self.compile_statement(stmt)?;
                }
            }
            _ => {
                // Expression statement - evaluate and discard
                let _ = self.compile_expression(node)?;
            }
        }
        Ok(())
    }

    /// Compile a function body
    fn compile_function_body(
        &mut self,
        name: &str,
        params: &[(String, KnullType)],
        body: &ASTNode,
    ) -> Result<(), String> {
        let function = *self
            .functions
            .get(name)
            .ok_or_else(|| format!("Function {} not declared", name))?;

        // Create entry basic block
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        // Save and set current function
        let old_function = self.current_function;
        self.current_function = Some(function);

        // Clear local variables
        let old_vars = self.variables.clone();
        self.variables.clear();

        // Create new ownership frame
        self.ownership_stack.push(OwnershipFrame {
            owned_vars: HashMap::new(),
        });

        // Allocate parameters
        for (i, (param_name, param_ty)) in params.iter().enumerate() {
            let param = function
                .get_nth_param(i as u32)
                .ok_or_else(|| format!("Failed to get parameter {}", i))?;

            let alloca = self
                .builder
                .build_alloca(self.knull_type_to_llvm(param_ty)?, param_name)
                .map_err(|e| e.to_string())?;

            self.builder
                .build_store(alloca, param)
                .map_err(|e| e.to_string())?;
            self.variables.insert(param_name.clone(), alloca);

            // Track ownership
            if self.mode != CompileMode::Novice {
                if let Some(frame) = self.ownership_stack.last_mut() {
                    frame
                        .owned_vars
                        .insert(param_name.clone(), OwnershipStatus::Owned);
                }
            }
        }

        // Compile function body
        self.compile_statement(body)?;

        // Ensure terminator (add implicit return void if needed)
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            if name == "main" {
                // Main returns 0 by default
                let zero = self.context.i64_type().const_int(0, false);
                self.builder
                    .build_return(Some(&zero))
                    .map_err(|e| e.to_string())?;
            } else {
                self.builder.build_return(None).map_err(|e| e.to_string())?;
            }
        }

        // Pop ownership frame
        self.ownership_stack.pop();

        // Restore context
        self.variables = old_vars;
        self.current_function = old_function;

        Ok(())
    }

    /// Compile variable declaration
    fn compile_let(
        &mut self,
        name: &str,
        value: &ASTNode,
        ty: &Option<KnullType>,
    ) -> Result<(), String> {
        let llvm_ty = if let Some(ref t) = ty {
            self.knull_type_to_llvm(t)?
        } else {
            // Infer type from value
            self.infer_type(value)?
        };

        let alloca = self
            .builder
            .build_alloca(llvm_ty, name)
            .map_err(|e| e.to_string())?;

        let val = self.compile_expression(value)?;
        self.builder
            .build_store(alloca, val)
            .map_err(|e| e.to_string())?;

        self.variables.insert(name.to_string(), alloca);

        // Track ownership
        if self.mode != CompileMode::Novice {
            if let Some(frame) = self.ownership_stack.last_mut() {
                frame
                    .owned_vars
                    .insert(name.to_string(), OwnershipStatus::Owned);
            }
        }

        Ok(())
    }

    /// Compile assignment
    fn compile_assignment(&mut self, name: &str, value: &ASTNode) -> Result<(), String> {
        let ptr = self
            .variables
            .get(name)
            .ok_or_else(|| format!("Variable {} not found", name))?;

        // Check ownership in Expert/God mode
        if self.mode != CompileMode::Novice {
            self.check_mutation_allowed(name)?;
        }

        let val = self.compile_expression(value)?;
        self.builder
            .build_store(*ptr, val)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Compile if statement
    fn compile_if(
        &mut self,
        condition: &ASTNode,
        then_branch: &ASTNode,
        else_branch: Option<&ASTNode>,
    ) -> Result<(), String> {
        let cond_val = self.compile_expression(condition)?;

        let function = self
            .current_function
            .ok_or("If statement outside of function")?;

        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let merge_block = self.context.append_basic_block(function, "ifmerge");

        // Convert condition to bool
        let cond_bool = self.to_bool(cond_val)?;

        self.builder
            .build_conditional_branch(cond_bool, then_block, else_block)
            .map_err(|e| e.to_string())?;

        // Then block
        self.builder.position_at_end(then_block);
        self.compile_statement(then_branch)?;
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder
                .build_unconditional_branch(merge_block)
                .map_err(|e| e.to_string())?;
        }

        // Else block
        self.builder.position_at_end(else_block);
        if let Some(else_stmt) = else_branch {
            self.compile_statement(else_stmt)?;
        }
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder
                .build_unconditional_branch(merge_block)
                .map_err(|e| e.to_string())?;
        }

        // Merge block
        self.builder.position_at_end(merge_block);

        Ok(())
    }

    /// Compile while loop
    fn compile_while(&mut self, condition: &ASTNode, body: &ASTNode) -> Result<(), String> {
        let function = self
            .current_function
            .ok_or("While loop outside of function")?;

        let cond_block = self.context.append_basic_block(function, "whilecond");
        let body_block = self.context.append_basic_block(function, "whilebody");
        let end_block = self.context.append_basic_block(function, "whileend");

        // Branch to condition
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| e.to_string())?;

        // Condition block
        self.builder.position_at_end(cond_block);
        let cond_val = self.compile_expression(condition)?;
        let cond_bool = self.to_bool(cond_val)?;
        self.builder
            .build_conditional_branch(cond_bool, body_block, end_block)
            .map_err(|e| e.to_string())?;

        // Body block
        self.builder.position_at_end(body_block);
        self.compile_statement(body)?;
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder
                .build_unconditional_branch(cond_block)
                .map_err(|e| e.to_string())?;
        }

        // End block
        self.builder.position_at_end(end_block);

        Ok(())
    }

    /// Compile for loop
    fn compile_for(&mut self, var: &str, iterable: &ASTNode, body: &ASTNode) -> Result<(), String> {
        // For now, implement as while loop for ranges
        // Full iterator support would be more complex

        let function = self
            .current_function
            .ok_or("For loop outside of function")?;

        // Initialize loop variable
        let i64_type = self.context.i64_type();
        let loop_var = self
            .builder
            .build_alloca(i64_type, var)
            .map_err(|e| e.to_string())?;

        let start_val = i64_type.const_int(0, false);
        self.builder
            .build_store(loop_var, start_val)
            .map_err(|e| e.to_string())?;

        self.variables.insert(var.to_string(), loop_var);

        // Get end value (simplified - assumes range)
        let end_val = match iterable {
            ASTNode::Range { end, .. } => self.compile_expression(end)?,
            _ => return Err("For loop currently only supports ranges".to_string()),
        };

        let cond_block = self.context.append_basic_block(function, "forcond");
        let body_block = self.context.append_basic_block(function, "forbody");
        let end_block = self.context.append_basic_block(function, "forend");

        // Branch to condition
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| e.to_string())?;

        // Condition block
        self.builder.position_at_end(cond_block);
        let current = self
            .builder
            .build_load(i64_type, loop_var, var)
            .map_err(|e| e.to_string())?;
        let cond = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::SLT,
                current.into_int_value(),
                end_val.into_int_value(),
                "forcond",
            )
            .map_err(|e| e.to_string())?;
        self.builder
            .build_conditional_branch(cond, body_block, end_block)
            .map_err(|e| e.to_string())?;

        // Body block
        self.builder.position_at_end(body_block);
        self.compile_statement(body)?;

        // Increment
        let current = self
            .builder
            .build_load(i64_type, loop_var, var)
            .map_err(|e| e.to_string())?;
        let one = i64_type.const_int(1, false);
        let next = self
            .builder
            .build_int_add(current.into_int_value(), one, "forinc")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(loop_var, next)
            .map_err(|e| e.to_string())?;

        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder
                .build_unconditional_branch(cond_block)
                .map_err(|e| e.to_string())?;
        }

        // End block
        self.builder.position_at_end(end_block);

        Ok(())
    }

    /// Compile return statement
    fn compile_return(&mut self, value: Option<&ASTNode>) -> Result<(), String> {
        if let Some(val) = value {
            let llvm_val = self.compile_expression(val)?;
            self.builder
                .build_return(Some(&llvm_val))
                .map_err(|e| e.to_string())?;
        } else {
            self.builder.build_return(None).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Compile function call
    fn compile_call(
        &mut self,
        name: &str,
        args: &[ASTNode],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match name {
            "println" => self.compile_println(args),
            "print" => self.compile_print(args),
            "alloc" => self.compile_alloc(args),
            "free" => self.compile_free(args),
            _ => {
                // Regular function call
                let function = *self
                    .functions
                    .get(name)
                    .ok_or_else(|| format!("Function {} not found", name))?;

                let mut llvm_args = Vec::new();
                for arg in args {
                    let val = self.compile_expression(arg)?;
                    llvm_args.push(val);
                }

                let args_refs: Vec<&dyn BasicValue<'ctx>> = llvm_args
                    .iter()
                    .map(|v| v as &dyn BasicValue<'ctx>)
                    .collect();

                let call_site = self
                    .builder
                    .build_call(function, &args_refs, name)
                    .map_err(|e| e.to_string())?;

                call_site
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| format!("Function {} returned void", name))
            }
        }
    }

    /// Compile println builtin
    fn compile_println(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.is_empty() {
            // Just print newline
            return self.compile_print(&[]);
        }

        let val = self.compile_expression(&args[0])?;

        // Get printf function
        let printf_type = self.context.i32_type().fn_type(
            &[self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .into()],
            true,
        );
        let printf = self.module.add_function("printf", printf_type, None);

        // Format and print based on type
        let format_str = match val {
            BasicValueEnum::IntValue(_) => "%lld\n\0",
            BasicValueEnum::FloatValue(_) => "%f\n\0",
            _ => "%s\n\0",
        };

        let fmt_ptr = self
            .builder
            .build_global_string_ptr(format_str, "fmt")
            .map_err(|e| e.to_string())?;

        self.builder
            .build_call(printf, &[fmt_ptr.as_pointer_value().into(), val], "println")
            .map_err(|e| e.to_string())?;

        Ok(self.context.i64_type().const_int(0, false).into())
    }

    /// Compile print builtin
    fn compile_print(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        // Similar to println but without newline
        // For now, just return 0
        Ok(self.context.i64_type().const_int(0, false).into())
    }

    /// Compile alloc builtin (memory allocation)
    fn compile_alloc(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() < 1 {
            return Err("alloc requires at least 1 argument (size)".to_string());
        }

        let size = self.compile_expression(&args[0])?;

        // Get malloc function
        let malloc_type = self
            .context
            .i8_type()
            .ptr_type(AddressSpace::default())
            .fn_type(&[self.context.i64_type().into()], false);
        let malloc = self.module.add_function("malloc", malloc_type, None);

        let ptr = self
            .builder
            .build_call(malloc, &[size], "alloc")
            .map_err(|e| e.to_string())?;

        ptr.try_as_basic_value()
            .left()
            .ok_or_else(|| "malloc returned void".to_string())
    }

    /// Compile free builtin (memory deallocation)
    fn compile_free(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() < 1 {
            return Err("free requires 1 argument (pointer)".to_string());
        }

        let ptr = self.compile_expression(&args[0])?;

        // Get free function
        let free_type = self.context.void_type().fn_type(
            &[self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .into()],
            false,
        );
        let free = self.module.add_function("free", free_type, None);

        self.builder
            .build_call(free, &[ptr], "free")
            .map_err(|e| e.to_string())?;

        Ok(self.context.i64_type().const_int(0, false).into())
    }

    /// Compile unsafe block (God mode only)
    fn compile_unsafe_block(&mut self, body: &ASTNode) -> Result<(), String> {
        // In unsafe blocks, we disable certain safety checks
        // For now, just compile the body normally
        self.compile_statement(body)
    }

    /// Compile inline assembly (God mode only)
    fn compile_inline_asm(
        &mut self,
        assembly: &str,
        inputs: &[(String, ASTNode)],
        outputs: &[(String, KnullType)],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // LLVM inline assembly support
        // This is a simplified version - full support would parse the asm string

        let asm_type = self.context.i64_type();
        let constraints = "=r,r";

        let asm = self.context.create_inline_asm(
            asm_type,
            assembly,
            constraints.to_string(),
            true,  // has side effects
            false, // is align stack
            None,  // dialect
            false, // can unwind
        );

        // Compile input values
        let mut input_vals = Vec::new();
        for (_, expr) in inputs {
            input_vals.push(self.compile_expression(expr)?);
        }

        let args_refs: Vec<&dyn BasicValue<'ctx>> = input_vals
            .iter()
            .map(|v| v as &dyn BasicValue<'ctx>)
            .collect();

        let result = self
            .builder
            .build_indirect_call(
                asm_type.fn_type(&[], false),
                asm.as_global_value().as_pointer_value(),
                &args_refs,
                "asm_result",
            )
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "Inline assembly returned void".to_string())
    }

    /// Compile syscall (God mode)
    fn compile_syscall(
        &mut self,
        number: &ASTNode,
        args: &[ASTNode],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let num_val = self.compile_expression(number)?;

        // On x86_64 Linux, syscalls use the `syscall` instruction
        // We'll use inline assembly for this

        let syscall_asm = "syscall";

        let mut asm_inputs = Vec::new();
        asm_inputs.push(("rax".to_string(), ASTNode::Literal(Literal::Int(0)))); // Placeholder

        for (i, arg) in args.iter().enumerate() {
            let reg = match i {
                0 => "rdi",
                1 => "rsi",
                2 => "rdx",
                3 => "r10",
                4 => "r8",
                5 => "r9",
                _ => return Err("Too many syscall arguments".to_string()),
            };
            asm_inputs.push((reg.to_string(), arg.clone()));
        }

        self.compile_inline_asm(
            syscall_asm,
            &asm_inputs,
            &[("rax".to_string(), KnullType::Int)],
        )
    }

    /// Compile an expression
    fn compile_expression(&mut self, node: &ASTNode) -> Result<BasicValueEnum<'ctx>, String> {
        match node {
            ASTNode::Literal(lit) => self.compile_literal(lit),
            ASTNode::Identifier(name) => self.compile_identifier(name),
            ASTNode::Binary { op, left, right } => self.compile_binary(op, left, right),
            ASTNode::Unary { op, operand } => self.compile_unary(op, operand),
            ASTNode::Call { name, args } => self.compile_call(name, args),
            ASTNode::FieldAccess { object, field } => self.compile_field_access(object, field),
            ASTNode::Index { array, index } => self.compile_index(array, index),
            ASTNode::Cast { value, to_type } => self.compile_cast(value, to_type),
            _ => Err(format!("Unsupported expression: {:?}", node)),
        }
    }

    /// Compile a literal
    fn compile_literal(&self, lit: &Literal) -> Result<BasicValueEnum<'ctx>, String> {
        match lit {
            Literal::Int(n) => Ok(self.context.i64_type().const_int(*n as u64, true).into()),
            Literal::Float(f) => Ok(self.context.f64_type().const_float(*f).into()),
            Literal::String(s) => {
                // Create a global string constant
                let str_ptr = self
                    .builder
                    .build_global_string_ptr(s, "str_lit")
                    .map_err(|e| e.to_string())?;
                Ok(str_ptr.as_pointer_value().into())
            }
            Literal::Bool(b) => Ok(self.context.bool_type().const_int(*b as u64, false).into()),
            Literal::Null => Ok(self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .const_null()
                .into()),
        }
    }

    /// Compile identifier (variable reference)
    fn compile_identifier(&self, name: &str) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self
            .variables
            .get(name)
            .ok_or_else(|| format!("Variable {} not found", name))?;

        // Determine type by loading and checking
        // For now, assume i64
        let val = self
            .builder
            .build_load(self.context.i64_type(), *ptr, name)
            .map_err(|e| e.to_string())?;

        Ok(val)
    }

    /// Compile binary operation
    fn compile_binary(
        &mut self,
        op: &BinaryOp,
        left: &ASTNode,
        right: &ASTNode,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let l = self.compile_expression(left)?;
        let r = self.compile_expression(right)?;

        match op {
            BinaryOp::Add => self.compile_add(l, r),
            BinaryOp::Sub => self.compile_sub(l, r),
            BinaryOp::Mul => self.compile_mul(l, r),
            BinaryOp::Div => self.compile_div(l, r),
            BinaryOp::Mod => self.compile_mod(l, r),
            BinaryOp::Eq => self.compile_cmp(inkwell::IntPredicate::EQ, l, r),
            BinaryOp::Neq => self.compile_cmp(inkwell::IntPredicate::NE, l, r),
            BinaryOp::Lt => self.compile_cmp(inkwell::IntPredicate::SLT, l, r),
            BinaryOp::Gt => self.compile_cmp(inkwell::IntPredicate::SGT, l, r),
            BinaryOp::Lte => self.compile_cmp(inkwell::IntPredicate::SLE, l, r),
            BinaryOp::Gte => self.compile_cmp(inkwell::IntPredicate::SGE, l, r),
            BinaryOp::And => self.compile_and(l, r),
            BinaryOp::Or => self.compile_or(l, r),
            BinaryOp::BitAnd => self.compile_bitand(l, r),
            BinaryOp::BitOr => self.compile_bitor(l, r),
            BinaryOp::BitXor => self.compile_bitxor(l, r),
            BinaryOp::Shl => self.compile_shl(l, r),
            BinaryOp::Shr => self.compile_shr(l, r),
        }
    }

    /// Compile addition
    fn compile_add(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_int_add(l, r, "add")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_add(l, r, "fadd")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Cannot add these types".to_string()),
        }
    }

    /// Compile subtraction
    fn compile_sub(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_int_sub(l, r, "sub")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_sub(l, r, "fsub")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Cannot subtract these types".to_string()),
        }
    }

    /// Compile multiplication
    fn compile_mul(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_int_mul(l, r, "mul")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_mul(l, r, "fmul")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Cannot multiply these types".to_string()),
        }
    }

    /// Compile division
    fn compile_div(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_int_signed_div(l, r, "div")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_div(l, r, "fdiv")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Cannot divide these types".to_string()),
        }
    }

    /// Compile modulo
    fn compile_mod(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_int_signed_rem(l, r, "mod")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Modulo only supported for integers".to_string()),
        }
    }

    /// Compile comparison
    fn compile_cmp(
        &mut self,
        pred: inkwell::IntPredicate,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_int_compare(pred, l, r, "cmp")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Comparison not supported for these types".to_string()),
        }
    }

    /// Compile logical AND
    fn compile_and(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_and(l, r, "and")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Logical AND only supported for integers".to_string()),
        }
    }

    /// Compile logical OR
    fn compile_or(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_or(l, r, "or")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Logical OR only supported for integers".to_string()),
        }
    }

    /// Compile bitwise AND
    fn compile_bitand(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.compile_and(left, right) // Same as logical for integers
    }

    /// Compile bitwise OR
    fn compile_bitor(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.compile_or(left, right) // Same as logical for integers
    }

    /// Compile bitwise XOR
    fn compile_bitxor(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_xor(l, r, "xor")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Bitwise XOR only supported for integers".to_string()),
        }
    }

    /// Compile shift left
    fn compile_shl(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_left_shift(l, r, "shl")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Shift only supported for integers".to_string()),
        }
    }

    /// Compile shift right
    fn compile_shr(
        &mut self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (left, right) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_right_shift(l, r, true, "shr")
                .map_err(|e| e.to_string())
                .map(|v| v.into()),
            _ => Err("Shift only supported for integers".to_string()),
        }
    }

    /// Compile unary operation
    fn compile_unary(
        &mut self,
        op: &UnaryOp,
        operand: &ASTNode,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let val = self.compile_expression(operand)?;

        match op {
            UnaryOp::Neg => match val {
                BasicValueEnum::IntValue(v) => {
                    let zero = self.context.i64_type().const_int(0, false);
                    self.builder
                        .build_int_sub(zero, v, "neg")
                        .map_err(|e| e.to_string())
                        .map(|v| v.into())
                }
                BasicValueEnum::FloatValue(v) => {
                    let zero = self.context.f64_type().const_float(0.0);
                    self.builder
                        .build_float_sub(zero, v, "fneg")
                        .map_err(|e| e.to_string())
                        .map(|v| v.into())
                }
                _ => Err("Negation only supported for numbers".to_string()),
            },
            UnaryOp::Not => match val {
                BasicValueEnum::IntValue(v) => {
                    let one = self.context.bool_type().const_int(1, false);
                    self.builder
                        .build_xor(v, one, "not")
                        .map_err(|e| e.to_string())
                        .map(|v| v.into())
                }
                _ => Err("Logical NOT only supported for booleans".to_string()),
            },
            UnaryOp::Deref => {
                // Load from pointer
                match val {
                    BasicValueEnum::PointerValue(ptr) => self
                        .builder
                        .build_load(self.context.i64_type(), ptr, "deref")
                        .map_err(|e| e.to_string()),
                    _ => Err("Cannot dereference non-pointer".to_string()),
                }
            }
            UnaryOp::Ref => {
                // Get address of variable
                if let ASTNode::Identifier(name) = operand {
                    let ptr = self
                        .variables
                        .get(name)
                        .ok_or_else(|| format!("Variable {} not found", name))?;
                    Ok((*ptr).into())
                } else {
                    Err("Can only take reference of variables".to_string())
                }
            }
        }
    }

    /// Compile field access
    fn compile_field_access(
        &mut self,
        object: &ASTNode,
        field: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_val = self.compile_expression(object)?;

        // For now, simplified field access
        // Full implementation would use GEP on structs
        Err("Field access not fully implemented".to_string())
    }

    /// Compile array index
    fn compile_index(
        &mut self,
        array: &ASTNode,
        index: &ASTNode,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expression(array)?;
        let idx_val = self.compile_expression(index)?;

        match (arr_val, idx_val) {
            (BasicValueEnum::PointerValue(arr), BasicValueEnum::IntValue(idx)) => {
                // Calculate element address
                let elem_ptr = unsafe {
                    self.builder
                        .build_in_bounds_gep(self.context.i64_type(), arr, &[idx], "elem_ptr")
                        .map_err(|e| e.to_string())?
                };

                // Load element
                self.builder
                    .build_load(self.context.i64_type(), elem_ptr, "elem")
                    .map_err(|e| e.to_string())
            }
            _ => Err("Invalid array access".to_string()),
        }
    }

    /// Compile type cast
    fn compile_cast(
        &mut self,
        value: &ASTNode,
        to_type: &KnullType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let val = self.compile_expression(value)?;
        let target_ty = self.knull_type_to_llvm(to_type)?;

        // For now, simplified casting
        // Full implementation would handle int<->float, sign extension, etc.
        Ok(val)
    }

    /// Convert value to boolean
    fn to_bool(&mut self, val: BasicValueEnum<'ctx>) -> Result<IntValue<'ctx>, String> {
        match val {
            BasicValueEnum::IntValue(v) => {
                let zero = self.context.i64_type().const_int(0, false);
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, v, zero, "tobool")
                    .map_err(|e| e.to_string())
            }
            BasicValueEnum::PointerValue(v) => {
                let null = v.get_type().const_null();
                self.builder
                    .build_ptr_compare(inkwell::IntPredicate::NE, v, null, "tobool")
                    .map_err(|e| e.to_string())
            }
            _ => Err("Cannot convert to bool".to_string()),
        }
    }

    /// Infer type from AST node
    fn infer_type(&self, node: &ASTNode) -> Result<BasicTypeEnum<'ctx>, String> {
        match node {
            ASTNode::Literal(lit) => match lit {
                Literal::Int(_) => Ok(self.context.i64_type().into()),
                Literal::Float(_) => Ok(self.context.f64_type().into()),
                Literal::Bool(_) => Ok(self.context.bool_type().into()),
                Literal::String(_) => Ok(self
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::default())
                    .into()),
                Literal::Null => Ok(self
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::default())
                    .into()),
            },
            ASTNode::Binary { op, left, .. } => match op {
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                    self.infer_type(left)
                }
                BinaryOp::Eq
                | BinaryOp::Neq
                | BinaryOp::Lt
                | BinaryOp::Gt
                | BinaryOp::Lte
                | BinaryOp::Gte
                | BinaryOp::And
                | BinaryOp::Or => Ok(self.context.bool_type().into()),
                _ => self.infer_type(left),
            },
            _ => Ok(self.context.i64_type().into()), // Default to i64
        }
    }

    /// Check if mutation is allowed (ownership system)
    fn check_mutation_allowed(&self, name: &str) -> Result<(), String> {
        if let Some(frame) = self.ownership_stack.last() {
            if let Some(status) = frame.owned_vars.get(name) {
                match status {
                    OwnershipStatus::Owned | OwnershipStatus::MutBorrowed => Ok(()),
                    OwnershipStatus::Borrowed => {
                        Err(format!("Cannot mutate {}: currently borrowed", name))
                    }
                    OwnershipStatus::Moved => {
                        Err(format!("Cannot mutate {}: value has been moved", name))
                    }
                }
            } else {
                Ok(()) // Not tracked, assume OK
            }
        } else {
            Ok(())
        }
    }

    /// Write LLVM IR to file
    pub fn write_ir(&self, path: &Path) -> Result<(), String> {
        self.module
            .print_to_file(path)
            .map_err(|e| format!("Failed to write IR: {}", e.to_string()))
    }

    /// Compile to object file
    pub fn compile_to_object(&self, path: &Path) -> Result<(), String> {
        self.target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| format!("Failed to compile to object: {}", e.to_string()))
    }

    /// Optimize the module
    pub fn optimize(&self, level: OptimizationLevel) {
        let pass_manager = PassManager::create(()); // Module-level pass manager

        match level {
            OptimizationLevel::None => {}
            OptimizationLevel::Less => {
                pass_manager.add_instruction_combining_pass();
                pass_manager.add_reassociate_pass();
            }
            OptimizationLevel::Default => {
                pass_manager.add_instruction_combining_pass();
                pass_manager.add_reassociate_pass();
                pass_manager.add_gvn_pass();
                pass_manager.add_cfg_simplification_pass();
                pass_manager.add_basic_alias_analysis_pass();
                pass_manager.add_promote_memory_to_register_pass();
                pass_manager.add_instruction_combining_pass();
                pass_manager.add_reassociate_pass();
            }
            OptimizationLevel::Aggressive => {
                pass_manager.add_instruction_combining_pass();
                pass_manager.add_reassociate_pass();
                pass_manager.add_gvn_pass();
                pass_manager.add_cfg_simplification_pass();
                pass_manager.add_basic_alias_analysis_pass();
                pass_manager.add_promote_memory_to_register_pass();
                pass_manager.add_instruction_combining_pass();
                pass_manager.add_reassociate_pass();
                pass_manager.add_gvn_pass();
                pass_manager.add_cfg_simplification_pass();
                pass_manager.add_loop_unroll_pass();
                pass_manager.add_loop_vectorize_pass();
            }
        }

        pass_manager.run_on(&self.module);
    }

    /// Get the LLVM module (for external use)
    pub fn get_module(&self) -> &Module<'ctx> {
        &self.module
    }
}

/// Compile a Knull AST to native code
pub fn compile_to_native(
    ast: &ASTNode,
    output_path: &Path,
    mode: CompileMode,
) -> Result<CompilationResult, String> {
    let context = Context::create();
    let mut codegen = LLVMCodeGen::new(&context, "knull_module", mode)?;

    // Compile AST to LLVM IR
    codegen.compile(ast)?;

    // Optimize
    codegen.optimize(OptimizationLevel::Default);

    // Write object file
    let obj_path = output_path.with_extension("o");
    codegen.compile_to_object(&obj_path)?;

    // Link to create executable
    let exe_path = output_path.to_path_buf();
    link_object(&obj_path, &exe_path)?;

    Ok(CompilationResult {
        output_path: output_path.to_string_lossy().to_string(),
        object_path: Some(obj_path.to_string_lossy().to_string()),
        executable_path: Some(exe_path.to_string_lossy().to_string()),
    })
}

/// Link object file to create executable
fn link_object(obj_path: &Path, exe_path: &Path) -> Result<(), String> {
    use std::process::Command;

    let status = Command::new("cc")
        .arg("-o")
        .arg(exe_path)
        .arg(obj_path)
        .arg("-lm") // Link math library
        .status()
        .map_err(|e| format!("Failed to link: {}", e))?;

    if !status.success() {
        return Err("Linking failed".to_string());
    }

    Ok(())
}
