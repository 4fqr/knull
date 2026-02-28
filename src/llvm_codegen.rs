//! Knull LLVM Code Generation Backend
//!
//! This module compiles Knull AST directly to native machine code using LLVM.

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::{
    BasicType, BasicTypeEnum, FloatType, FunctionType, IntType, PointerType, StructType, VoidType,
};
use inkwell::values::{
    BasicValue, BasicValueEnum, FloatValue, FunctionValue, IntValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::FloatPredicate;
use inkwell::OptimizationLevel;

use crate::parser::{ASTNode, Literal, Param, Type as KnullType};
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
    variable_types: HashMap<String, KnullType>, // Track variable types for field access
    functions: HashMap<String, FunctionValue<'ctx>>,
    structs: HashMap<String, (StructType<'ctx>, Vec<(String, KnullType)>)>,

    // Current function context
    current_function: Option<FunctionValue<'ctx>>,

    // Compile mode
    mode: CompileMode,
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
            variable_types: HashMap::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            current_function: None,
            mode,
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
                ret_type,
                ..
            } => {
                self.declare_function(name, params, ret_type)?;
            }
            ASTNode::StructDef { name, fields } => {
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
        params: &[Param],
        return_type: &Option<KnullType>,
    ) -> Result<FunctionValue<'ctx>, String> {
        // Convert Knull types to LLVM types
        let param_types: Vec<BasicTypeEnum<'ctx>> = params
            .iter()
            .map(|p| self.knull_type_to_llvm(p.ty.as_ref().unwrap_or(&KnullType::I64)))
            .collect::<Result<Vec<_>, _>>()?;

        let fn_type = if return_type.is_none() || return_type.as_ref() == Some(&KnullType::Void) {
            self.context.void_type().fn_type(&param_types, false)
        } else {
            let ret_ty = self.knull_type_to_llvm(return_type.as_ref().unwrap())?;
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

        self.structs
            .insert(name.to_string(), (struct_type, fields.to_vec()));

        Ok(struct_type)
    }

    /// Convert Knull types to LLVM types
    fn knull_type_to_llvm(&self, ty: &KnullType) -> Result<BasicTypeEnum<'ctx>, String> {
        match ty {
            KnullType::I8 => Ok(self.context.i8_type().into()),
            KnullType::I16 => Ok(self.context.i16_type().into()),
            KnullType::I32 => Ok(self.context.i32_type().into()),
            KnullType::I64 => Ok(self.context.i64_type().into()),
            KnullType::I128 => Ok(self.context.i128_type().into()),
            KnullType::U8 => Ok(self.context.i8_type().into()),
            KnullType::U16 => Ok(self.context.i16_type().into()),
            KnullType::U32 => Ok(self.context.i32_type().into()),
            KnullType::U64 => Ok(self.context.i64_type().into()),
            KnullType::U128 => Ok(self.context.i128_type().into()),
            KnullType::F32 => Ok(self.context.f32_type().into()),
            KnullType::F64 => Ok(self.context.f64_type().into()),
            KnullType::Bool => Ok(self.context.bool_type().into()),
            KnullType::Char => Ok(self.context.i8_type().into()),
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
                if let Some((struct_ty, _)) = self.structs.get(name) {
                    Ok(struct_ty.as_basic_type_enum())
                } else {
                    Err(format!("Unknown type: {}", name))
                }
            }
            KnullType::Ref(inner) | KnullType::MutRef(inner) => {
                let inner_ty = self.knull_type_to_llvm(inner)?;
                Ok(inner_ty.ptr_type(AddressSpace::default()).into())
            }
            KnullType::RawPtr(inner) | KnullType::MutRawPtr(inner) => {
                let _inner_ty = self.knull_type_to_llvm(inner)?;
                Ok(self
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::default())
                    .into())
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
            ASTNode::If {
                cond,
                then_body,
                else_body,
            } => {
                self.compile_if(cond, then_body, else_body.as_deref())?;
            }
            ASTNode::While { cond, body } => {
                self.compile_while(cond, body)?;
            }
            ASTNode::For { var, iter, body } => {
                self.compile_for(var, iter, body)?;
            }
            ASTNode::Loop(body) => {
                self.compile_loop(body)?;
            }
            ASTNode::Return(value) => {
                self.compile_return(value.as_deref())?;
            }
            ASTNode::Call { func, args } => {
                if let ASTNode::Identifier(name) = func.as_ref() {
                    self.compile_call(name, args)?;
                } else {
                    let _ = self.compile_expression(node)?;
                }
            }
            ASTNode::Unsafe(body) => {
                if self.mode == CompileMode::God {
                    self.compile_unsafe_block(body)?;
                } else {
                    return Err("Unsafe blocks are only allowed in God mode".to_string());
                }
            }
            ASTNode::Asm(assembly) => {
                if self.mode == CompileMode::God {
                    self.compile_inline_asm(assembly)?;
                } else {
                    return Err("Inline assembly is only allowed in God mode".to_string());
                }
            }
            ASTNode::Syscall(args) => {
                self.compile_syscall(args)?;
            }
            ASTNode::Block(stmts) => {
                for stmt in stmts {
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
        params: &[Param],
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

        // Clear local variables and types
        let old_vars = self.variables.clone();
        let old_types = self.variable_types.clone();
        self.variables.clear();
        self.variable_types.clear();

        // Allocate parameters
        for (i, param) in params.iter().enumerate() {
            let llvm_ty = self.knull_type_to_llvm(param.ty.as_ref().unwrap_or(&KnullType::I64))?;
            let param_val = function
                .get_nth_param(i as u32)
                .ok_or_else(|| format!("Failed to get parameter {}", i))?;

            let alloca = self
                .builder
                .build_alloca(llvm_ty, &param.name)
                .map_err(|e| e.to_string())?;

            self.builder
                .build_store(alloca, param_val)
                .map_err(|e| e.to_string())?;
            self.variables.insert(param.name.clone(), alloca);
            if let Some(ref ty) = param.ty {
                self.variable_types.insert(param.name.clone(), ty.clone());
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

        // Restore context
        self.variables = old_vars;
        self.variable_types = old_types;
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

        // Store variable type for field access
        let var_type = if let Some(ref t) = ty {
            t.clone()
        } else {
            // Try to infer from value
            match value {
                ASTNode::StructLiteral { name, .. } => KnullType::Custom(name.clone()),
                _ => KnullType::I64, // Default
            }
        };
        self.variable_types.insert(name.to_string(), var_type);

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

    /// Compile loop (infinite)
    fn compile_loop(&mut self, body: &ASTNode) -> Result<(), String> {
        let function = self.current_function.ok_or("Loop outside of function")?;

        let body_block = self.context.append_basic_block(function, "loopbody");
        let end_block = self.context.append_basic_block(function, "loopend");

        // Branch to body
        self.builder
            .build_unconditional_branch(body_block)
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
                .build_unconditional_branch(body_block)
                .map_err(|e| e.to_string())?;
        }

        // End block (unreachable unless break is implemented)
        self.builder.position_at_end(end_block);

        Ok(())
    }

    /// Compile for loop
    fn compile_for(&mut self, var: &str, iterable: &ASTNode, body: &ASTNode) -> Result<(), String> {
        let function = self
            .current_function
            .ok_or("For loop outside of function")?;

        // Initialize loop variable
        let i64_type = self.context.i64_type();
        let loop_var = self
            .builder
            .build_alloca(i64_type, var)
            .map_err(|e| e.to_string())?;

        // Get start and end values from range
        let (start_val, end_val, inclusive) = match iterable {
            ASTNode::Range {
                start,
                end,
                inclusive,
            } => {
                let s = self.compile_expression(start)?;
                let e = self.compile_expression(end)?;
                (s, e, *inclusive)
            }
            _ => return Err("For loop currently only supports ranges".to_string()),
        };

        // Store initial value
        self.builder
            .build_store(loop_var, start_val)
            .map_err(|e| e.to_string())?;
        self.variables.insert(var.to_string(), loop_var);

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

        let cond = if inclusive {
            self.builder.build_int_compare(
                inkwell::IntPredicate::SLE,
                current.into_int_value(),
                end_val.into_int_value(),
                "forcond",
            )
        } else {
            self.builder.build_int_compare(
                inkwell::IntPredicate::SLT,
                current.into_int_value(),
                end_val.into_int_value(),
                "forcond",
            )
        }
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
            "strlen" => self.compile_strlen(args),
            "strcmp" => self.compile_strcmp(args),
            "strcat" => self.compile_strcat(args),
            "strcpy" => self.compile_strcpy(args),
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
        let (format_str, arg_val): (&str, BasicValueEnum<'ctx>) = match val {
            BasicValueEnum::IntValue(v) => {
                // Check if it's a boolean (i1) or regular integer
                if v.get_type().get_bit_width() == 1 {
                    // Boolean - print as true/false
                    ("%s\n\0", val)
                } else {
                    ("%lld\n\0", val)
                }
            }
            BasicValueEnum::FloatValue(_) => ("%f\n\0", val),
            BasicValueEnum::PointerValue(_) => ("%s\n\0", val),
            _ => ("%p\n\0", val),
        };

        let fmt_ptr = self
            .builder
            .build_global_string_ptr(format_str, "fmt")
            .map_err(|e| e.to_string())?;

        self.builder
            .build_call(
                printf,
                &[fmt_ptr.as_pointer_value().into(), arg_val],
                "println",
            )
            .map_err(|e| e.to_string())?;

        Ok(self.context.i64_type().const_int(0, false).into())
    }

    /// Compile print builtin
    fn compile_print(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.is_empty() {
            // Just print newline
            let printf_type = self.context.i32_type().fn_type(
                &[self
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::default())
                    .into()],
                true,
            );
            let printf = self.module.add_function("printf", printf_type, None);
            let fmt_ptr = self
                .builder
                .build_global_string_ptr("\n\0", "fmt")
                .map_err(|e| e.to_string())?;
            self.builder
                .build_call(printf, &[fmt_ptr.as_pointer_value().into()], "print")
                .map_err(|e| e.to_string())?;
            return Ok(self.context.i64_type().const_int(0, false).into());
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
        let (format_str, arg_val): (&str, BasicValueEnum<'ctx>) = match val {
            BasicValueEnum::IntValue(_) => ("%lld\0", val),
            BasicValueEnum::FloatValue(_) => ("%f\0", val),
            BasicValueEnum::PointerValue(_) => ("%s\0", val),
            _ => ("%p\0", val),
        };

        let fmt_ptr = self
            .builder
            .build_global_string_ptr(format_str, "fmt")
            .map_err(|e| e.to_string())?;

        self.builder
            .build_call(
                printf,
                &[fmt_ptr.as_pointer_value().into(), arg_val],
                "print",
            )
            .map_err(|e| e.to_string())?;

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

    /// Compile strlen builtin
    fn compile_strlen(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() < 1 {
            return Err("strlen requires 1 argument (string)".to_string());
        }

        let ptr = self.compile_expression(&args[0])?;

        // Get strlen function
        let strlen_type = self.context.i64_type().fn_type(
            &[self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .into()],
            false,
        );
        let strlen = self.module.add_function("strlen", strlen_type, None);

        let result = self
            .builder
            .build_call(strlen, &[ptr], "strlen")
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "strlen returned void".to_string())
    }

    /// Compile strcmp builtin
    fn compile_strcmp(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() < 2 {
            return Err("strcmp requires 2 arguments (strings)".to_string());
        }

        let ptr1 = self.compile_expression(&args[0])?;
        let ptr2 = self.compile_expression(&args[1])?;

        // Get strcmp function
        let strcmp_type = self.context.i32_type().fn_type(
            &[
                self.context
                    .i8_type()
                    .ptr_type(AddressSpace::default())
                    .into(),
                self.context
                    .i8_type()
                    .ptr_type(AddressSpace::default())
                    .into(),
            ],
            false,
        );
        let strcmp = self.module.add_function("strcmp", strcmp_type, None);

        let result = self
            .builder
            .build_call(strcmp, &[ptr1, ptr2], "strcmp")
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "strcmp returned void".to_string())
    }

    /// Compile strcat builtin (string concatenation)
    fn compile_strcat(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() < 2 {
            return Err("strcat requires 2 arguments (strings)".to_string());
        }

        let ptr1 = self.compile_expression(&args[0])?;
        let ptr2 = self.compile_expression(&args[1])?;

        // Get strcat function
        let strcat_type = self
            .context
            .i8_type()
            .ptr_type(AddressSpace::default())
            .fn_type(
                &[
                    self.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .into(),
                    self.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .into(),
                ],
                false,
            );
        let strcat = self.module.add_function("strcat", strcat_type, None);

        let result = self
            .builder
            .build_call(strcat, &[ptr1, ptr2], "strcat")
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "strcat returned void".to_string())
    }

    /// Compile strcpy builtin
    fn compile_strcpy(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() < 2 {
            return Err("strcpy requires 2 arguments (dest, src)".to_string());
        }

        let dest = self.compile_expression(&args[0])?;
        let src = self.compile_expression(&args[1])?;

        // Get strcpy function
        let strcpy_type = self
            .context
            .i8_type()
            .ptr_type(AddressSpace::default())
            .fn_type(
                &[
                    self.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .into(),
                    self.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .into(),
                ],
                false,
            );
        let strcpy = self.module.add_function("strcpy", strcpy_type, None);

        let result = self
            .builder
            .build_call(strcpy, &[dest, src], "strcpy")
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "strcpy returned void".to_string())
    }

    /// Compile unsafe block (God mode only)
    fn compile_unsafe_block(&mut self, body: &ASTNode) -> Result<(), String> {
        // In unsafe blocks, we disable certain safety checks
        // For now, just compile the body normally
        self.compile_statement(body)
    }

    /// Compile inline assembly (God mode only)
    fn compile_inline_asm(&mut self, assembly: &str) -> Result<BasicValueEnum<'ctx>, String> {
        // LLVM inline assembly support - simplified version
        let asm_type = self.context.i64_type();
        let constraints = "=r";

        let asm = self.context.create_inline_asm(
            asm_type,
            assembly,
            constraints.to_string(),
            true,  // has side effects
            false, // is align stack
            None,  // dialect
            false, // can unwind
        );

        let result = self
            .builder
            .build_indirect_call(
                asm_type.fn_type(&[], false),
                asm.as_global_value().as_pointer_value(),
                &[] as &[&dyn BasicValue<'ctx>],
                "asm_result",
            )
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "Inline assembly returned void".to_string())
    }

    /// Compile syscall (God mode)
    fn compile_syscall(&mut self, args: &[ASTNode]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.is_empty() {
            return Err("syscall requires at least 1 argument (syscall number)".to_string());
        }

        // Compile syscall number
        let num_val = self.compile_expression(&args[0])?;

        // For x86_64 Linux, we'll use inline assembly
        // This is a simplified implementation
        let syscall_asm = "syscall";

        // For a proper implementation, we would generate inline assembly
        // with the correct register assignments for each argument
        // rax = syscall number
        // rdi, rsi, rdx, r10, r8, r9 = arguments

        // For now, return the syscall number as placeholder
        Ok(num_val)
    }

    /// Compile an expression
    fn compile_expression(&mut self, node: &ASTNode) -> Result<BasicValueEnum<'ctx>, String> {
        match node {
            ASTNode::Literal(lit) => self.compile_literal(lit),
            ASTNode::Identifier(name) => self.compile_identifier(name),
            ASTNode::Binary { op, left, right } => self.compile_binary(op, left, right),
            ASTNode::Unary { op, operand } => self.compile_unary(op, operand),
            ASTNode::Call { func, args } => {
                if let ASTNode::Identifier(name) = func.as_ref() {
                    self.compile_call(name, args)
                } else {
                    Err("Indirect calls not yet supported".to_string())
                }
            }
            ASTNode::FieldAccess { obj, field } => self.compile_field_access(obj, field),
            ASTNode::Index { obj, index } => self.compile_index(obj, index),
            ASTNode::StructLiteral { name, fields } => self.compile_struct_literal(name, fields),
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
        // For now, try i64 first, if that fails we need type tracking
        let val = self
            .builder
            .build_load(self.context.i64_type(), *ptr, name)
            .map_err(|e| e.to_string())?;

        Ok(val)
    }

    /// Compile binary operation
    fn compile_binary(
        &mut self,
        op: &str,
        left: &ASTNode,
        right: &ASTNode,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let l = self.compile_expression(left)?;
        let r = self.compile_expression(right)?;

        match op {
            "+" => self.compile_add(l, r),
            "-" => self.compile_sub(l, r),
            "*" => self.compile_mul(l, r),
            "/" => self.compile_div(l, r),
            "%" => self.compile_mod(l, r),
            "==" => self.compile_cmp(inkwell::IntPredicate::EQ, l, r),
            "!=" => self.compile_cmp(inkwell::IntPredicate::NE, l, r),
            "<" => self.compile_cmp(inkwell::IntPredicate::SLT, l, r),
            ">" => self.compile_cmp(inkwell::IntPredicate::SGT, l, r),
            "<=" => self.compile_cmp(inkwell::IntPredicate::SLE, l, r),
            ">=" => self.compile_cmp(inkwell::IntPredicate::SGE, l, r),
            "&&" => self.compile_and(l, r),
            "||" => self.compile_or(l, r),
            "&" => self.compile_bitand(l, r),
            "|" => self.compile_bitor(l, r),
            "^" => self.compile_bitxor(l, r),
            "<<" => self.compile_shl(l, r),
            ">>" => self.compile_shr(l, r),
            _ => Err(format!("Unknown binary operator: {}", op)),
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
            // String concatenation - pointer arithmetic or strcat
            (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
                // For string concatenation, we'd need to allocate new memory
                // and copy both strings. For now, use strcat behavior.
                self.compile_strcat_from_ptrs(l, r)
            }
            _ => Err("Cannot add these types".to_string()),
        }
    }

    /// Helper for string concatenation
    fn compile_strcat_from_ptrs(
        &mut self,
        left: PointerValue<'ctx>,
        right: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Simplified: just return left pointer
        // Full implementation would allocate, strcpy left, then strcat right
        Ok(left.into())
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
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                // Convert IntPredicate to FloatPredicate
                let float_pred = match pred {
                    inkwell::IntPredicate::EQ => FloatPredicate::OEQ,
                    inkwell::IntPredicate::NE => FloatPredicate::ONE,
                    inkwell::IntPredicate::SLT => FloatPredicate::OLT,
                    inkwell::IntPredicate::SGT => FloatPredicate::OGT,
                    inkwell::IntPredicate::SLE => FloatPredicate::OLE,
                    inkwell::IntPredicate::SGE => FloatPredicate::OGE,
                    _ => FloatPredicate::OEQ,
                };
                self.builder
                    .build_float_compare(float_pred, l, r, "fcmp")
                    .map_err(|e| e.to_string())
                    .map(|v| v.into())
            }
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
        op: &str,
        operand: &ASTNode,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let val = self.compile_expression(operand)?;

        match op {
            "-" => match val {
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
            "!" => match val {
                BasicValueEnum::IntValue(v) => {
                    let one = self.context.bool_type().const_int(1, false);
                    self.builder
                        .build_xor(v, one, "not")
                        .map_err(|e| e.to_string())
                        .map(|v| v.into())
                }
                _ => Err("Logical NOT only supported for booleans".to_string()),
            },
            "*" => {
                // Dereference - load from pointer
                match val {
                    BasicValueEnum::PointerValue(ptr) => self
                        .builder
                        .build_load(self.context.i64_type(), ptr, "deref")
                        .map_err(|e| e.to_string()),
                    _ => Err("Cannot dereference non-pointer".to_string()),
                }
            }
            "&" => {
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
            _ => Err(format!("Unknown unary operator: {}", op)),
        }
    }

    /// Compile field access using GEP instructions
    fn compile_field_access(
        &mut self,
        obj: &ASTNode,
        field: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Get the struct type information
        let (struct_name, obj_ptr) = match obj {
            ASTNode::Identifier(name) => {
                // Look up the variable's struct type
                let var_type = self
                    .variable_types
                    .get(name)
                    .ok_or_else(|| format!("Unknown variable: {}", name))?;

                let struct_name = match var_type {
                    KnullType::Custom(s) => s.clone(),
                    _ => return Err(format!("Variable {} is not a struct type", name)),
                };

                let obj_ptr = self
                    .variables
                    .get(name)
                    .ok_or_else(|| format!("Variable {} not found", name))?;

                (struct_name, *obj_ptr)
            }
            ASTNode::FieldAccess {
                obj: inner_obj,
                field: inner_field,
            } => {
                // Nested field access - compile inner first
                let inner_val = self.compile_field_access(inner_obj, inner_field)?;
                // For now, nested field access returning a struct is complex
                return Err("Nested field access not yet implemented".to_string());
            }
            _ => return Err("Field access requires struct variable or expression".to_string()),
        };

        // Get struct type info
        let (struct_ty, field_defs) = self
            .structs
            .get(&struct_name)
            .ok_or_else(|| format!("Unknown struct type: {}", struct_name))?
            .clone();

        // Find field index
        let field_idx = field_defs
            .iter()
            .position(|(n, _)| n == field)
            .ok_or_else(|| format!("Field {} not found in struct {}", field, struct_name))?;

        // Get field pointer using GEP
        let field_ptr = self
            .builder
            .build_struct_gep(
                struct_ty,
                obj_ptr,
                field_idx as u32,
                &format!("field_{}", field),
            )
            .map_err(|e| e.to_string())?;

        // Load and return field value
        let field_type = &field_defs[field_idx].1;
        let llvm_field_type = self.knull_type_to_llvm(field_type)?;

        self.builder
            .build_load(llvm_field_type, field_ptr, &format!("load_{}", field))
            .map_err(|e| e.to_string())
    }

    /// Compile array index
    fn compile_index(
        &mut self,
        obj: &ASTNode,
        index: &ASTNode,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_val = self.compile_expression(obj)?;
        let idx_val = self.compile_expression(index)?;

        match (obj_val, idx_val) {
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

    /// Compile struct literal
    fn compile_struct_literal(
        &mut self,
        name: &str,
        fields: &[(String, ASTNode)],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Look up the struct type
        let (struct_ty, field_defs) = self
            .structs
            .get(name)
            .ok_or_else(|| format!("Unknown struct type: {}", name))?
            .clone();

        // Allocate the struct on the stack
        let alloca = self
            .builder
            .build_alloca(struct_ty, &format!("{}_lit", name))
            .map_err(|e| e.to_string())?;

        // Compile and store each field
        for (field_name, field_expr) in fields {
            // Find the field index
            let field_idx = field_defs
                .iter()
                .position(|(n, _)| n == field_name)
                .ok_or_else(|| format!("Unknown field: {}", field_name))?;

            // Compile the field value
            let field_val = self.compile_expression(field_expr)?;

            // Get pointer to field using GEP
            let field_ptr = self
                .builder
                .build_struct_gep(
                    struct_ty,
                    alloca,
                    field_idx as u32,
                    &format!("field_{}", field_name),
                )
                .map_err(|e| e.to_string())?;

            // Store the value
            self.builder
                .build_store(field_ptr, field_val)
                .map_err(|e| e.to_string())?;
        }

        // Return the struct value (load it)
        self.builder
            .build_load(struct_ty, alloca, &format!("{}_val", name))
            .map_err(|e| e.to_string())
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
            ASTNode::Binary { op, left, .. } => match op.as_str() {
                "+" | "-" | "*" | "/" | "%" => self.infer_type(left),
                "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||" => {
                    Ok(self.context.bool_type().into())
                }
                _ => self.infer_type(left),
            },
            _ => Ok(self.context.i64_type().into()), // Default to i64
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
        let pass_manager = PassManager::create(());

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
