//! Knull Interpreter
//!
//! Tree-walking interpreter for executing Knull code directly.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use crate::parser::{ASTNode, Literal};

/// Runtime value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Null,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
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
            Value::Null => false,
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
}

#[derive(Debug, Clone)]
struct FunctionDef {
    name: String,
    params: Vec<String>,
    body: ASTNode,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            scopes: vec![Scope::new()],
            functions: HashMap::new(),
            return_value: None,
            break_flag: false,
            continue_flag: false,
        };

        interpreter
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

    fn bind_parameter(&mut self, name: String, value: Value) {
        // Always create in current scope (for function parameters)
        self.current_scope().set(name, value);
    }

    /// Execute a program
    pub fn execute(&mut self, ast: &ASTNode) -> Result<(), String> {
        match ast {
            ASTNode::Program(items) => {
                // First pass: collect function definitions
                for item in items {
                    if let ASTNode::Function { name, params, body } = item {
                        self.functions.insert(
                            name.clone(),
                            FunctionDef {
                                name: name.clone(),
                                params: params.clone(),
                                body: *body.clone(),
                            },
                        );
                    }
                }

                // Second pass: execute non-function statements
                for item in items {
                    if !matches!(item, ASTNode::Function { .. }) {
                        self.execute_node(item)?;
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
            ASTNode::Let { name, value } => {
                let val = self.evaluate(value)?;
                self.set_variable(name.clone(), val);
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
                if let Value::Array(arr) = iter_val {
                    for val in arr {
                        self.set_variable(var.clone(), val);
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
                self.call_function(func, arg_values?)?;
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
    fn evaluate(&mut self, node: &ASTNode) -> Result<Value, String> {
        match node {
            ASTNode::Literal(lit) => Ok(self.literal_to_value(lit)),
            ASTNode::Identifier(name) => self
                .get_variable(name)
                .ok_or_else(|| format!("Undefined variable: {}", name)),
            ASTNode::Binary { op, left, right } => {
                let left_val = self.evaluate(left)?;
                let right_val = self.evaluate(right)?;
                self.eval_binary_op(op, left_val, right_val)
            }
            ASTNode::Unary { op, operand } => {
                let val = self.evaluate(operand)?;
                match op.as_str() {
                    "!" => Ok(Value::Bool(!val.is_truthy())),
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
                self.call_function(func, arg_values?)
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
                    (Value::String(s), Value::Int(i)) => {
                        let idx = if i < 0 { s.len() as i64 + i } else { i } as usize;
                        s.chars()
                            .nth(idx)
                            .map(|c| Value::String(c.to_string()))
                            .ok_or_else(|| "Index out of bounds".to_string())
                    }
                    _ => Err("Cannot index non-array/string value".to_string()),
                }
            }
            ASTNode::Array(elements) => {
                let values: Result<Vec<Value>, String> =
                    elements.iter().map(|e| self.evaluate(e)).collect();
                Ok(Value::Array(values?))
            }
            _ => Err(format!("Cannot evaluate: {:?}", node)),
        }
    }

    /// Convert a literal to a value
    fn literal_to_value(&self, lit: &Literal) -> Value {
        match lit {
            Literal::Int(i) => Value::Int(*i),
            Literal::Float(f) => Value::Float(*f),
            Literal::String(s) => Value::String(s.clone()),
            Literal::Bool(b) => Value::Bool(*b),
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
                _ => Err("Cannot compare incompatible types".to_string()),
            },
            ">" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l > r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l > r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Bool((*l as f64) > *r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Bool(*l > *r as f64)),
                _ => Err("Cannot compare incompatible types".to_string()),
            },
            "<=" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l <= r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l <= r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Bool((*l as f64) <= *r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Bool(*l <= *r as f64)),
                _ => Err("Cannot compare incompatible types".to_string()),
            },
            ">=" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l >= r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l >= r)),
                (Value::Int(l), Value::Float(r)) => Ok(Value::Bool((*l as f64) >= *r)),
                (Value::Float(l), Value::Int(r)) => Ok(Value::Bool(*l >= *r as f64)),
                _ => Err("Cannot compare incompatible types".to_string()),
            },
            "&&" => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            "||" => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            _ => Err(format!("Unknown binary operator: {}", op)),
        }
    }

    /// Call a function
    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Check for built-in functions first
        if let Some(result) = self.call_builtin(name, &args) {
            return result;
        }

        // Check for user-defined functions
        if let Some(func_def) = self.functions.get(name).cloned() {
            // Check argument count
            if args.len() != func_def.params.len() {
                return Err(format!(
                    "Function {} expects {} arguments, got {}",
                    name,
                    func_def.params.len(),
                    args.len()
                ));
            }

            self.push_scope();

            // Bind parameters to arguments
            for (param, arg) in func_def.params.iter().zip(args.iter()) {
                self.bind_parameter(param.clone(), arg.clone());
            }

            // Execute function body
            self.execute_node(&func_def.body)?;

            let result = self.return_value.take().unwrap_or(Value::Null);
            self.pop_scope();

            return Ok(result);
        }

        Err(format!("Unknown function: {}", name))
    }

    /// Call a built-in function
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, String>> {
        match name {
            "print" => {
                if let Some(arg) = args.first() {
                    print!("{}", arg);
                }
                Some(Ok(Value::Null))
            }
            "println" => {
                if let Some(arg) = args.first() {
                    println!("{}", arg);
                } else {
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
                        Value::Null => "null",
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
            _ => None,
        }
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
