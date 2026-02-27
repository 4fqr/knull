# KNULL COMPILER ARCHITECTURE
## The Nihil Compiler

---

# PART I: COMPILER OVERVIEW

The Knull compiler, codenamed **Nihil**, is a retargetable, multi-tiered compiler that transforms Knull source code into optimized machine code. The compiler is designed with three core principles:

1. **Zero-Cost Abstractions**: High-level code compiles to no more code than hand-written low-level code
2. **Progressive Compilation**: Fast development builds via direct codegen, optimized releases via LLVM
3. **Layered Architecture**: Clear separation between frontend, midend, and backend

---

# PART II: COMPILER PIPELINE

## 2.1 Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         SOURCE CODE (.knull)                        │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         FRONTEND                                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐   │
│  │   LEXER     │→ │   PARSER    │→ │   TYPE INFERENCE        │   │
│  │  (Tokens)   │  │   (AST)     │  │   (Typed AST)           │   │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         MIDEND (KIR)                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐   │
│  │ LOWERING    │→ │ OPTIMIZATION│→ │   CODE GENERATION       │   │
│  │ (KIR)       │  │ (Passes)    │  │   (LLVM IR / ASM)      │   │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         BACKEND                                    │
│  ┌──────────────────┐    ┌──────────────────┐                    │
│  │  LLVM BACKEND    │    │  DIRECT BACKEND  │                    │
│  │  (Production)    │    │  (Development)   │                    │
│  └──────────────────┘    └──────────────────┘                    │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    MACHINE CODE / EXECUTABLE                        │
└─────────────────────────────────────────────────────────────────────┘
```

## 2.2 Frontend

### 2.2.1 Lexer

The lexer performs lexical analysis, converting source text into a stream of tokens.

**Token Types:**
- `KEYWORD`: Reserved words (`fn`, `let`, `struct`, etc.)
- `IDENTIFIER`: Variable/function names
- `LITERAL`: Numbers, strings, characters, booleans
- `OPERATOR`: Arithmetic, comparison, logical operators
- `DELIMITER`: Parentheses, brackets, braces, commas
- `COMMENT`: Single-line (`//`) and multi-line (`/* */`)
- `WHITESPACE`: Spaces, tabs, newlines (ignored)

**Lexer Output:**
```rust
struct Token {
    kind: TokenKind,
    span: Span,
    value: TokenValue,
}
```

### 2.2.2 Parser

The parser builds an Abstract Syntax Tree (AST) from the token stream according to the Knull grammar.

**AST Node Types:**
```rust
enum Expr {
    Lit(Literal),
    Ident(String),
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Unary { op: UnOp, expr: Box<Expr> },
    Call { func: Box<Expr>, args: Vec<Expr> },
    Index { expr: Box<Expr>, index: Box<Expr> },
    Field { expr: Box<Expr>, field: String },
    Lambda { params: Vec<Param>, body: Box<Expr> },
    If { condition: Box<Expr>, then: Box<Expr>, else_: Box<Expr> },
    Match { expr: Box<Expr>, arms: Vec<MatchArm> },
    Block { stmts: Vec<Stmt> },
    Loop { body: Box<Expr> },
    Break { label: Option<String>, value: Box<Expr> },
    Return { value: Box<Expr> },
}

enum Stmt {
    Expr(Expr),
    Let { pattern: Pattern, ty: Type, init: Expr },
    Assign { target: Expr, value: Expr },
    While { condition: Expr, body: Stmt },
    For { pattern: Pattern, iter: Expr, body: Stmt },
    Match { expr: Expr, arms: Vec<MatchArm> },
    Function { name: String, params: Vec<Param>, ret: Type, body: Stmt },
    Struct { name: String, fields: Vec<Field> },
    Enum { name: String, variants: Vec<Variant> },
}
```

### 2.2.3 Type Inference

The type inference engine performs bidirectional type checking with constraint solving.

**Inference Algorithm:**
1. Collect type constraints from expressions
2. Unify constraints using unification algorithm
3. Solve for type variables
4. Propagate concrete types

**Inference Rules:**
```
let x = 5           →  x: i32 (literal inference)
let f = fn(x) x + 1 →  f: fn(i32) -> i32 (lambda inference)
fn id<T>(x: T) T    →  id: fn<T>(T) -> T (generic)
```

---

# PART III: KNULL INTERMEDIATE REPRESENTATION (KIR)

## 3.1 KIR Design

KIR is a three-address code (TAC) representation that serves as the primary IR for optimization and code generation.

## 3.2 KIR Instruction Set

### 3.2.1 Control Flow

```
LABEL      @label_name
JUMP       @label_name
JUMP_IF    @true_label, @false_label, condition
SWITCH     value, [(value, label), ...], default_label
CALL       result, function, [args...]
RET        [value]
RET_VOID
UNREACHABLE
```

### 3.2.2 Memory Operations

```
ALLOCA     result, type, align
LOAD       result, address, align
STORE      value, address, align
GEP        result, base, [index, type], offset
MEMSET     address, value, size, align
MEMCPY     dest, src, size, align
```

### 3.2.3 Arithmetic

```
ADD        result, a, b
SUB        result, a, b
MUL        result, a, b
DIV        result, a, b
REM        result, a, b
NEG        result, value
```

### 3.2.4 Bitwise Operations

```
AND        result, a, b
OR         result, a, b
XOR        result, a, b
SHL        result, value, amount
SHR        result, value, amount
NOT        result, value
```

### 3.2.5 Comparison

```
ICMP       result, cond, a, b   // eq, ne, ugt, uge, ult, ile, sgt, sge, slt, sle
FCMP       result, cond, a, b   // oeq, one, ogt, oge, olt, ole, ord, uno
SELECT     result, cond, true_val, false_val
```

### 3.2.6 Cast Operations

```
ZEXT       result, value, target_type
SEXT       result, value, target_type
TRUNC      result, value, target_type
FPEXT      result, value, target_type
FPTRUNC    result, value, target_type
BITCAST    result, value, target_type
INT2PTR    result, value, target_type
PTR2INT    result, value, target_type
```

### 3.2.7 Atomic Operations

```
ATOMIC_LOAD        result, address, ordering
ATOMIC_STORE       value, address, ordering
ATOMIC_RMW         result, address, op, value, ordering
ATOMIC_CMP_XCHG    result, address, compare, new, success_ordering, fail_ordering
```

### 3.2.8 Intrinsics

```
INTRINSIC      result, intrinsic_id, [args...]
```

---

# PART IV: OPTIMIZATION PASSES

## 4.1 KIR-to-KIR Passes

### 4.1.1 Constant Folding

```kir
// Before
%a = ADD %x, 0
%b = MUL %y, 1
%c = DIV %z, %z

// After
%a = %x
%b = %y
%c = 1
```

### 4.1.2 Dead Code Elimination

```kir
// Before
%a = ADD %x, %y
%unused = MUL %a, %b
%result = LOAD %ptr

// After
%result = LOAD %ptr
```

### 4.1.3 Loop Unrolling

```kir
// Before (iterations known)
loop:
  %i = ADD %i, 1
  %sum = ADD %sum, %i
  JUMP_IF %i < 10, @loop

// After (unrolled 4x)
%sum = ADD %sum, 1
%sum = ADD %sum, 2
%sum = ADD %sum, 3
%sum = ADD %sum, 4
// ...
```

### 4.1.4 Function Inlining

```kir
// Before
fn add_one(x) x + 1
let y = add_one(5)

// After
let y = 5 + 1
```

### 4.1.5 Copy Propagation

```kir
// Before
%a = LOAD %x
%b = %a
%c = ADD %b, 1

// After
%c = ADD %x, 1
```

### 4.1.6 Common Subexpression Elimination

```kir
// Before
%a = ADD %x, %y
%b = MUL %x, %y
%c = ADD %x, %y
%d = MUL %x, %y

// After
%a = ADD %x, %y
%b = MUL %x, %y
%c = %a
%d = %b
```

## 4.2 Memory-to-Register Conversion

Converts stack-based memory operations to SSA register form:

```kir
// Before
%ptr = ALLOCA i32
STORE %ptr, 5
%a = LOAD %ptr
STORE %ptr, 10
%b = LOAD %ptr

// After
%a = 5
%b = 10
```

## 4.3 SSA Construction

### 4.3.1 Phi Nodes

```kir
%result = PHI [%a, @block1], [%b, @block2]
```

### 4.3.2 SSA Form

```kir
entry:
  %i = 0
  JUMP @loop

loop:
  %i.next = PHI [%i, @entry], [%i.2, @loop]
  %sum.next = PHI [%sum, @entry], [%sum.2, @loop]
  %cond = ICMP slt %i.next, 10
  JUMP_IF %cond, @body, @exit

body:
  %sum.2 = ADD %sum.next, %i.next
  %i.2 = ADD %i.next, 1
  JUMP @loop

exit:
  %result = %sum.next
```

---

# PART V: CODE GENERATION

## 5.1 Target Selection

The compiler supports multiple target architectures:

| Architecture | Triple | Notes |
|--------------|--------|-------|
| x86_64 | `x86_64-unknown-linux-gnu` | Primary |
| AArch64 | `aarch64-unknown-linux-gnu` | ARM64 |
| ARM | `arm-unknown-linux-gnueabihf` | 32-bit ARM |
| RISC-V | `riscv64-unknown-linux-gnu` | RV64GC |
| WebAssembly | `wasm32-unknown-unknown` | WASI |

## 5.2 Register Allocation

### 5.2.1 Linear Scan

The compiler uses linear scan register allocation:

```rust
// Allocation order by live range
1. Calculate liveness intervals
2. Sort by start position
3. Allocate from available registers
4. Spill if necessary
```

### 5.2.2 Calling Conventions

**System V AMD64 ABI:**
- Integer arguments: RDI, RSI, RDX, RCX, R8, R9
- Float arguments: XMM0-XMM7
- Return: RAX, RDX (integer), XMM0, XMM1 (float)
- Stack alignment: 16-byte

## 5.3 Stack Frame Layout

```
+------------------+  <- High address
|   Return addr    |
+------------------+
|   Saved regs     |
+------------------+
|   Local vars     |
+------------------+
|   Outgoing args  |
+------------------+  <- Stack pointer
```

---

# PART VI: LLVM BACKEND

## 6.1 Integration

For production builds, KIR is lowered to LLVM IR and optimized using LLVM's pass infrastructure.

### 6.1.1 KIR to LLVM IR Lowering

```kir
// KIR
%a = ADD %x, %y
%b = MUL %a, 2

// LLVM IR
%a = add i32 %x, %y
%b = mul i32 %a, 2
```

### 6.1.2 LLVM Optimization Pipeline

```
O0:    -inline
O1:    -inline, -mem2reg, -dse, -dce, -gvn, -simplifycfg
O2:    O1 + -loop-unroll, -loop-vectorize, -reassociate
O3:    O2 + -inline-threshold, -unswitch
Oz:    O3 + -optimize-for-size
```

## 6.2 Direct Backend

For fast development builds, the compiler includes a direct backend that generates assembly without LLVM.

### 6.2.1 Code Generation

```rust
// Direct generation to x86_64 assembly
fn emit_add(ir: &Instr, ctx: &mut CodeGen) {
    let src1 = ctx.get_operand(ir.src1);
    let src2 = ctx.get_operand(ir.src2);
    let dest = ctx.get_dest(ir.dest);
    
    // Choose optimal instruction
    if let Some(reg) = ctx.try_alloc_register() {
        ctx.emit(format!("mov {}, {}", reg, src1));
        ctx.emit(format!("add {}, {}", reg, src2));
        ctx.move_to_dest(dest, reg);
    } else {
        // Spill to stack
        // ...
    }
}
```

---

# PART VII: RUNTIME SUPPORT

## 7.1 Runtime Functions

The compiler emits calls to runtime functions for operations that cannot be lowered directly:

```c
// Memory management
void* knull_alloc(size_t size);
void knull_free(void* ptr);
void* knull_realloc(void* ptr, size_t new_size);

// String operations
void* knull_string_concat(void* s1, void* s2);
int   knull_string_compare(void* s1, void* s2);

// Array operations
void* knull_array_clone(void* arr, size_t elem_size);

// Panic handling
void knull_panic(const char* msg, const char* file, int line);

// Thread-local storage
void* knull_tls_get(int key);
void knull_tls_set(int key, void* value);
```

## 7.2 Startup Code

```c
// crt0.s equivalent
void _start() {
    // Initialize runtime
    knull_runtime_init();
    
    // Call main
    int result = main(0, null);
    
    // Exit
    knull_exit(result);
}
```

---

# PART VIII: COMPILATION MODES

## 8.1 Immediate Mode (`knull run`)

For rapid development, the compiler can interpret KIR directly:

```bash
knull run script.knull    # Interpret
knull run --jit script.knull  # JIT compile
```

## 8.2 Development Build (`knull build`)

```bash
knull build --target x86_64-unknown-linux-gnu
```

Uses direct backend for fast compilation:
- No LLVM linking
- Minimal optimization
- Debug info included

## 8.3 Release Build (`knull build --release`)

```bash
knull build --release --target x86_64-unknown-linux-gnu
```

Uses LLVM backend for maximum performance:
- Full optimization
- Link-time optimization (LTO)
- Size optimization available (`--optimize-for-size`)

---

# PART IX: COMPILER PLUGINS

## 9.1 Lint Plugins

```rust
trait LintPass {
    fn check_fn(&mut self, ctx: &LintContext, fn_: &Function);
    fn check_expr(&mut self, ctx: &LintContext, expr: &Expr);
}
```

## 9.2 Transform Plugins

```rust
trait TransformPass {
    fn transform_module(&mut self, module: &mut Module);
}
```

## 9.3 Code Generation Plugins

```rust
trait CodeGenPlugin {
    fn generate(&self, ir: &IR) -> Vec<u8>;
}
```

---

# APPENDIX A: KIR REFERENCE

## A.1 Instruction Reference

| Instruction | Operands | Description |
|-------------|----------|-------------|
| `ADD` | `result, a, b` | Integer addition |
| `SUB` | `result, a, b` | Integer subtraction |
| `MUL` | `result, a, b` | Integer multiplication |
| `DIV` | `result, a, b` | Integer division |
| `REM` | `result, a, b` | Integer remainder |
| `AND` | `result, a, b` | Bitwise AND |
| `OR` | `result, a, b` | Bitwise OR |
| `XOR` | `result, a, b` | Bitwise XOR |
| `SHL` | `result, value, amount` | Left shift |
| `SHR` | `result, value, amount` | Right shift |
| `LOAD` | `result, address, align` | Load from memory |
| `STORE` | `value, address, align` | Store to memory |
| `ALLOCA` | `result, type, align` | Allocate on stack |
| `CALL` | `result, func, args` | Function call |
| `RET` | `[value]` | Return value |
| `JUMP` | `label` | Unconditional jump |
| `JUMP_IF` | `true_label, false_label, cond` | Conditional jump |
| `PHI` | `result, [(value, label), ...]` | PHI node |

## A.2 Comparison Conditions

| Condition | Description |
|-----------|-------------|
| `eq` | Equal |
| `ne` | Not equal |
| `sgt` | Signed greater than |
| `sge` | Signed greater or equal |
| `slt` | Signed less than |
| `sle` | Signed less or equal |
| `ugt` | Unsigned greater than |
| `uge` | Unsigned greater or equal |
| `ult` | Unsigned less than |
| `ule` | Unsigned less or equal |

---

*Document Version: 1.0*
*Last Updated: 2026*
