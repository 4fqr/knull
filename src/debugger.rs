//! Knull Debug Adapter Protocol Server
//! Implements JSON-RPC based debugging for the Knull language

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAPRequest {
    pub id: Option<i32>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAPResponse {
    pub id: Option<i32>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<DAPError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAPError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

impl DAPError {
    pub fn new(code: i32, message: &str) -> Self {
        DAPError {
            code,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn with_data(code: i32, message: &str, data: serde_json::Value) -> Self {
        DAPError {
            code,
            message: message.to_string(),
            data: Some(data),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAPEvent {
    pub event: String,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugState {
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "exited")]
    Exited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceBreakpoint {
    pub line: u32,
    #[serde(default)]
    pub column: Option<u32>,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: Option<i32>,
    pub verified: bool,
    pub line: u32,
    #[serde(default)]
    pub column: Option<u32>,
    #[serde(default)]
    pub source: Option<Source>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub name: Option<String>,
    pub path: Option<String>,
    #[serde(default)]
    pub source_reference: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: i32,
    pub name: String,
    pub source: Option<Source>,
    pub line: u32,
    pub column: u32,
    #[serde(default)]
    pub end_line: Option<u32>,
    #[serde(default)]
    pub end_column: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub name: String,
    pub variables_reference: i32,
    #[serde(default)]
    pub expensive: bool,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub column: Option<u32>,
    #[serde(default)]
    pub end_line: Option<u32>,
    #[serde(default)]
    pub end_column: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub type_: Option<String>,
    pub variables_reference: Option<i32>,
    #[serde(default)]
    pub indexed_variables: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoppedEventBody {
    pub reason: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub thread_id: Option<i32>,
    #[serde(default)]
    pub all_threads_stopped: Option<bool>,
    #[serde(default)]
    pub hit_breakpoint_ids: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuedEventBody {
    pub thread_id: i32,
    #[serde(default)]
    pub all_threads_continued: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitedEventBody {
    pub exit_code: i32,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminatedEventBody {
    #[serde(default)]
    pub restart: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputEventBody {
    pub output: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub output_type: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub variables_reference: Option<i32>,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointEventBody {
    pub reason: String,
    pub breakpoint: Breakpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_name: Option<String>,
    #[serde(default)]
    pub adapter_id: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub lines_start_at_1: Option<bool>,
    #[serde(default)]
    pub columns_start_at_1: Option<bool>,
    #[serde(default)]
    pub path_format: Option<String>,
    #[serde(default)]
    pub supports_variable_type: Option<bool>,
    #[serde(default)]
    pub supports_variable_paging: Option<bool>,
    #[serde(default)]
    pub supports_run_in_terminal_request: Option<bool>,
    #[serde(default)]
    pub supports_memory_references: Option<bool>,
    #[serde(default)]
    pub supports_progress_reporting: Option<bool>,
    #[serde(default)]
    pub supports_invalidated_event: Option<bool>,
    #[serde(default)]
    pub supports_args_can_be_interpreted_by_shell: Option<bool>,
    #[serde(default)]
    pub supports_start_debugging_request: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    pub adapter_id: String,
    pub locale: String,
    pub lines_start_at_1: bool,
    pub columns_start_at_1: bool,
    pub path_format: String,
    pub supports_variable_type: bool,
    pub supports_variable_paging: bool,
    pub supports_run_in_terminal_request: bool,
    pub supports_memory_references: bool,
    pub supports_progress_reporting: bool,
    pub supports_invalidated_event: bool,
    pub supports_args_can_be_interpreted_by_shell: bool,
    pub supports_start_debugging_request: bool,
    #[serde(default)]
    pub supports_stepping: Option<bool>,
    pub supports_clipboard_context: bool,
    pub supports_modules_request: bool,
    pub supports_execute_command: bool,
    pub supports_configuration_done_request: bool,
    pub supports_function_breakpoints: bool,
    pub supports_conditional_breakpoints: bool,
    pub supports_hit_conditional_breakpoints: bool,
    pub supports_set_variable: bool,
    pub supports_restart_frame: bool,
    pub supports_goto_targets_request: bool,
    pub supports_step_in_targets_request: bool,
    pub supports_completions_request: bool,
    pub supports_delayed_stack_trace_loading: bool,
    pub supports_loaded_sources_request: bool,
    pub supports_log_points: bool,
    pub supports_terminate_threads_request: bool,
    pub supports_terminate_debuggee: bool,
    pub supports_data_breakpoints: bool,
    pub supports_read_memory_request: bool,
    pub supports_write_memory_request: bool,
    pub supports_disassemble_request: bool,
    #[serde(default)]
    pub additional_columns: Option<Vec<String>>,
    #[serde(default)]
    pub dependency_graphs: Option<Vec<serde_json::Value>>,
}

impl Default for InitializeResponse {
    fn default() -> Self {
        InitializeResponse {
            adapter_id: "knull".to_string(),
            locale: "en".to_string(),
            lines_start_at_1: true,
            columns_start_at_1: true,
            path_format: "path".to_string(),
            supports_variable_type: true,
            supports_variable_paging: false,
            supports_run_in_terminal_request: false,
            supports_memory_references: false,
            supports_progress_reporting: false,
            supports_invalidated_event: false,
            supports_args_can_be_interpreted_by_shell: false,
            supports_start_debugging_request: false,
            supports_stepping: Some(true),
            supports_clipboard_context: false,
            supports_modules_request: false,
            supports_execute_command: false,
            supports_configuration_done_request: true,
            supports_function_breakpoints: false,
            supports_conditional_breakpoints: true,
            supports_hit_conditional_breakpoints: false,
            supports_set_variable: false,
            supports_restart_frame: false,
            supports_goto_targets_request: false,
            supports_step_in_targets_request: false,
            supports_completions_request: false,
            supports_delayed_stack_trace_loading: false,
            supports_loaded_sources_request: false,
            supports_log_points: false,
            supports_terminate_threads_request: false,
            supports_terminate_debuggee: false,
            supports_data_breakpoints: false,
            supports_read_memory_request: false,
            supports_write_memory_request: false,
            supports_disassemble_request: false,
            additional_columns: None,
            dependency_graphs: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBreakpointsRequest {
    pub source: Source,
    #[serde(default)]
    pub breakpoints: Option<Vec<SourceBreakpoint>>,
    #[serde(default)]
    pub source_modified: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBreakpointsResponse {
    pub breakpoints: Vec<Breakpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetExceptionBreakpointsRequest {
    #[serde(default)]
    pub filters: Option<Vec<String>>,
    #[serde(default)]
    pub filter_options: Option<Vec<ExceptionBreakpointFilter>>,
    #[serde(default)]
    pub exception_options: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionBreakpointFilter {
    pub filter: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: Option<bool>,
    #[serde(default)]
    pub supports_conditions: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetExceptionBreakpointsResponse {
    #[serde(default)]
    pub breakpoints: Option<Vec<Breakpoint>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchRequest {
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub debuggee: Option<serde_json::Value>,
    #[serde(default)]
    pub internal_console_options: Option<serde_json::Value>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub runtime_version: Option<String>,
    #[serde(default)]
    pub console_name: Option<String>,
    #[serde(default)]
    pub suppress_refresh: Option<bool>,
    #[serde(default)]
    pub enable_automatic_environment_configuration: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationDoneRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadsResponse {
    pub threads: Vec<Thread>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackTraceRequest {
    pub thread_id: i32,
    #[serde(default)]
    pub start_frame: Option<i32>,
    #[serde(default)]
    pub levels: Option<i32>,
    #[serde(default)]
    pub format: Option<StackTraceFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackTraceFormat {
    #[serde(default)]
    pub thread_id: Option<bool>,
    #[serde(default)]
    pub module: Option<bool>,
    #[serde(default)]
    pub group: Option<bool>,
    #[serde(default)]
    pub inline_frames: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackTraceResponse {
    pub stack_frames: Vec<StackFrame>,
    #[serde(default)]
    pub total_frames: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopesRequest {
    pub frame_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopesResponse {
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariablesRequest {
    pub variables_reference: i32,
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default)]
    pub start: Option<i32>,
    #[serde(default)]
    pub count: Option<i32>,
    #[serde(default)]
    pub format: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariablesResponse {
    pub variables: Vec<Variable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueRequest {
    pub thread_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueResponse {
    pub all_threads_continued: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextRequest {
    pub thread_id: i32,
    #[serde(default)]
    pub granularity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInRequest {
    pub thread_id: i32,
    #[serde(default)]
    pub target_id: Option<i32>,
    #[serde(default)]
    pub granularity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutRequest {
    pub thread_id: i32,
    #[serde(default)]
    pub granularity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseRequest {
    pub thread_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRequest {
    pub source: Source,
    #[serde(default)]
    pub source_reference: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedSourcesRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateRequest {
    pub expression: String,
    #[serde(default)]
    pub frame_id: Option<i32>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub format: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateResponse {
    pub result: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub type_: Option<String>,
    pub variables_reference: Option<i32>,
    #[serde(default)]
    pub indexed_variables: Option<i32>,
    #[serde(default)]
    pub memory_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVariableRequest {
    pub name: String,
    pub value: String,
    pub variables_reference: i32,
    #[serde(default)]
    pub format: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVariableResponse {
    pub value: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub type_: Option<String>,
    pub variables_reference: Option<i32>,
    #[serde(default)]
    pub indexed_variables: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisconnectRequest {
    #[serde(default)]
    pub terminate_debuggee: Option<bool>,
    #[serde(default)]
    pub restart: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartRequest {
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminateRequest {
    #[serde(default)]
    pub restart: Option<bool>,
}

pub struct DebuggerState {
    pub breakpoints: HashMap<String, Vec<SourceBreakpoint>>,
    pub exception_breakpoints: Vec<String>,
    pub state: DebugState,
    pub current_thread_id: i32,
    pub current_frame_id: i32,
    pub program: Option<String>,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: HashMap<String, String>,
    pub line: u32,
    pub column: u32,
    pub call_stack: Vec<StackFrame>,
    pub variables: HashMap<i32, Vec<Variable>>,
    pub next_var_ref: i32,
    pub stopped_reason: Option<String>,
    pub exit_code: Option<i32>,
    pub initialized: bool,
    pub configuration_done: bool,
    pub line_breaks: Vec<u32>,
    pub source_name: String,
}

impl Default for DebuggerState {
    fn default() -> Self {
        DebuggerState {
            breakpoints: HashMap::new(),
            exception_breakpoints: Vec::new(),
            state: DebugState::Stopped,
            current_thread_id: 1,
            current_frame_id: 0,
            program: None,
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            line: 1,
            column: 1,
            call_stack: Vec::new(),
            variables: HashMap::new(),
            next_var_ref: 1,
            stopped_reason: None,
            exit_code: None,
            initialized: false,
            configuration_done: false,
            line_breaks: Vec::new(),
            source_name: String::from("<unknown>"),
        }
    }
}

pub type DAPHandler = Arc<Mutex<DebuggerState>>;

pub fn create_debugger_state() -> DAPHandler {
    Arc::new(Mutex::new(DebuggerState::default()))
}

pub fn send_response<W: Write>(writer: &mut W, id: Option<i32>, result: serde_json::Value) {
    let response = DAPResponse {
        id,
        result: Some(result),
        error: None,
    };
    send_message(writer, response);
}

pub fn send_error_response<W: Write>(writer: &mut W, id: Option<i32>, code: i32, message: &str) {
    let response = DAPResponse {
        id,
        result: None,
        error: Some(DAPError::new(code, message)),
    };
    send_message(writer, response);
}

pub fn send_event<W: Write>(writer: &mut W, event: &str, body: Option<serde_json::Value>) {
    let dap_event = DAPEvent {
        event: event.to_string(),
        body,
    };
    let json = serde_json::to_string(&dap_event).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", json.len());
    let _ = writer.write_all(header.as_bytes());
    let _ = writer.write_all(json.as_bytes());
    let _ = writer.flush();
}

fn send_message<W: Write, T: Serialize>(writer: &mut W, message: T) {
    let json = serde_json::to_string(&message).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", json.len());
    let _ = writer.write_all(header.as_bytes());
    let _ = writer.write_all(json.as_bytes());
    let _ = writer.flush();
}

pub fn read_message<R: Read>(reader: &mut R) -> Option<DAPRequest> {
    let mut reader = BufReader::new(reader);
    let mut header = String::new();

    loop {
        header.clear();
        match reader.read_line(&mut header) {
            Ok(0) => return None,
            Ok(_) => {
                let trimmed = header.trim();
                if trimmed.is_empty() {
                    break;
                }
            }
            Err(_) => return None,
        }
    }

    let mut content_length: usize = 0;
    for line in header.lines() {
        if line.to_lowercase().starts_with("content-length:") {
            if let Some(len) = line.split(':').nth(1) {
                content_length = len.trim().parse().unwrap_or(0);
            }
        }
    }

    if content_length == 0 {
        return None;
    }

    let mut body = vec![0u8; content_length];
    match reader.read_exact(&mut body) {
        Ok(_) => {}
        Err(_) => return None,
    }

    let json_str = String::from_utf8_lossy(&body);
    match serde_json::from_str(&json_str) {
        Ok(request) => Some(request),
        Err(_) => None,
    }
}

pub fn handle_initialize(
    state: &mut DebuggerState,
    _params: Option<serde_json::Value>,
) -> serde_json::Value {
    state.initialized = true;
    let response = InitializeResponse::default();
    json!({
        "capabilities": response
    })
}

pub fn handle_launch(
    state: &mut DebuggerState,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if let Some(params) = params {
        if let Ok(launch) = serde_json::from_value::<LaunchRequest>(params.clone()) {
            state.program = launch.program;
            state.args = launch.args.unwrap_or_default();
            state.cwd = launch.cwd;
            state.env = launch.env.unwrap_or_default();
            state.state = DebugState::Stopped;

            if let Some(name) = launch.name {
                state.source_name = name;
            }

            return Ok(serde_json::Value::Null);
        }
    }
    Err("Invalid launch parameters".to_string())
}

pub fn handle_configuration_done(state: &mut DebuggerState) -> serde_json::Value {
    state.configuration_done = true;
    state.state = DebugState::Running;
    serde_json::Value::Null
}

pub fn handle_set_breakpoints(
    state: &mut DebuggerState,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if let Some(params) = params {
        if let Ok(set_bp) = serde_json::from_value::<SetBreakpointsRequest>(params) {
            let source_path = set_bp
                .source
                .path
                .clone()
                .unwrap_or_else(|| set_bp.source.name.clone().unwrap_or_default());

            let breakpoints = set_bp.breakpoints.unwrap_or_default();
            let mut bp_results: Vec<Breakpoint> = Vec::new();
            let mut line_breaks: Vec<u32> = Vec::new();

            for bp in &breakpoints {
                line_breaks.push(bp.line);
                bp_results.push(Breakpoint {
                    id: Some(bp.line as i32),
                    verified: true,
                    line: bp.line,
                    column: bp.column,
                    source: Some(set_bp.source.clone()),
                });
            }

            state.breakpoints.insert(source_path.clone(), breakpoints);
            state.line_breaks = line_breaks;

            return Ok(json!({
                "breakpoints": bp_results
            }));
        }
    }
    Err("Invalid setBreakpoints parameters".to_string())
}

pub fn handle_set_exception_breakpoints(
    state: &mut DebuggerState,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if let Some(params) = params {
        if let Ok(set_ex) = serde_json::from_value::<SetExceptionBreakpointsRequest>(params) {
            state.exception_breakpoints = set_ex.filters.unwrap_or_default();
            return Ok(serde_json::Value::Null);
        }
    }
    Err("Invalid setExceptionBreakpoints parameters".to_string())
}

pub fn handle_threads(_state: &mut DebuggerState) -> serde_json::Value {
    json!({
        "threads": [
            {
                "id": 1,
                "name": "main"
            }
        ]
    })
}

pub fn handle_stack_trace(
    state: &mut DebuggerState,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let start_frame = params
        .as_ref()
        .and_then(|p| p.get("startFrame"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as usize;

    let levels = params
        .as_ref()
        .and_then(|p| p.get("levels"))
        .and_then(|v| v.as_i64())
        .unwrap_or(100) as usize;

    let frames: Vec<StackFrame> = state
        .call_stack
        .iter()
        .skip(start_frame)
        .take(levels)
        .cloned()
        .collect();

    let total = state.call_stack.len() as i32;

    Ok(json!({
        "stackFrames": frames,
        "totalFrames": total
    }))
}

pub fn handle_scopes(
    state: &mut DebuggerState,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let frame_id = params
        .as_ref()
        .and_then(|p| p.get("frameId"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let scopes = vec![
        Scope {
            name: "Local".to_string(),
            variables_reference: frame_id,
            expensive: false,
            source: None,
            line: Some(state.line),
            column: Some(state.column),
            end_line: None,
            end_column: None,
        },
        Scope {
            name: "Global".to_string(),
            variables_reference: -1,
            expensive: false,
            source: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
    ];

    Ok(json!({
        "scopes": scopes
    }))
}

pub fn handle_variables(
    state: &mut DebuggerState,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let variables_ref = params
        .as_ref()
        .and_then(|p| p.get("variablesReference"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let vars = state
        .variables
        .get(&variables_ref)
        .cloned()
        .unwrap_or_else(|| {
            if variables_ref == -1 {
                vec![Variable {
                    name: "env".to_string(),
                    value: "{...}".to_string(),
                    type_: Some("Object".to_string()),
                    variables_reference: Some(-2),
                    indexed_variables: None,
                }]
            } else {
                vec![]
            }
        });

    Ok(json!({
        "variables": vars
    }))
}

pub fn handle_continue(
    state: &mut DebuggerState,
    _params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    state.state = DebugState::Running;
    state.stopped_reason = None;
    Ok(json!({
        "allThreadsContinued": true
    }))
}

pub fn handle_pause(
    _state: &mut DebuggerState,
    _params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Null)
}

pub fn handle_next(
    state: &mut DebuggerState,
    _params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    state.state = DebugState::Running;
    state.stopped_reason = None;
    Ok(serde_json::Value::Null)
}

pub fn handle_step_in(
    state: &mut DebuggerState,
    _params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    state.state = DebugState::Running;
    state.stopped_reason = None;
    Ok(serde_json::Value::Null)
}

pub fn handle_step_out(
    state: &mut DebuggerState,
    _params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    state.state = DebugState::Running;
    state.stopped_reason = None;
    Ok(serde_json::Value::Null)
}

pub fn handle_evaluate(
    state: &mut DebuggerState,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let expression = params
        .as_ref()
        .and_then(|p| p.get("expression"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let result = evaluate_expression(state, expression);

    Ok(json!({
        "result": result.0,
        "type": result.1,
        "variablesReference": result.2.unwrap_or(0)
    }))
}

fn evaluate_expression(
    state: &DebuggerState,
    expression: &str,
) -> (String, Option<String>, Option<i32>) {
    let expr = expression.trim();

    if expr.starts_with('$') {
        let var_name = &expr[1..];
        for vars in state.variables.values() {
            for var in vars {
                if var.name == var_name {
                    return (
                        var.value.clone(),
                        var.type_.clone(),
                        var.variables_reference,
                    );
                }
            }
        }
    }

    match expr {
        "threads" | "thread" => (
            format!("Thread {}", state.current_thread_id),
            Some("Thread".to_string()),
            None,
        ),
        "state" => (
            format!("{:?}", state.state),
            Some("String".to_string()),
            None,
        ),
        "line" => (state.line.to_string(), Some("Int".to_string()), None),
        "column" => (state.column.to_string(), Some("Int".to_string()), None),
        "program" => (
            state.program.clone().unwrap_or_default(),
            Some("String".to_string()),
            None,
        ),
        "args" => (format!("{:?}", state.args), Some("Array".to_string()), None),
        "breakpoints" => (
            format!(
                "{} breakpoints set",
                state.breakpoints.values().map(|v| v.len()).sum::<usize>()
            ),
            Some("Int".to_string()),
            None,
        ),
        "exceptionBreakpoints" => (
            format!("{:?}", state.exception_breakpoints),
            Some("Array".to_string()),
            None,
        ),
        _ => (
            format!("Unknown: {}", expr),
            Some("Unknown".to_string()),
            None,
        ),
    }
}

pub fn handle_disconnect(
    state: &mut DebuggerState,
    _params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    state.state = DebugState::Exited;
    state.breakpoints.clear();
    state.call_stack.clear();
    state.variables.clear();
    Ok(serde_json::Value::Null)
}

pub fn handle_terminate(_state: &mut DebuggerState) -> serde_json::Value {
    serde_json::Value::Null
}

pub fn handle_restart(
    _state: &mut DebuggerState,
    _params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Null)
}

pub fn set_breakpoint_hit(state: &mut DebuggerState, line: u32, column: u32) {
    state.line = line;
    state.column = column;
    state.state = DebugState::Paused;
    state.stopped_reason = Some("breakpoint".to_string());
}

pub fn set_paused(state: &mut DebuggerState, line: u32, column: u32, reason: &str) {
    state.line = line;
    state.column = column;
    state.state = DebugState::Paused;
    state.stopped_reason = Some(reason.to_string());
}

pub fn set_exited(state: &mut DebuggerState, exit_code: i32) {
    state.state = DebugState::Exited;
    state.exit_code = Some(exit_code);
}

pub fn add_stack_frame(state: &mut DebuggerState, name: &str, line: u32, column: u32) {
    let frame_id = state.call_stack.len() as i32;
    state.call_stack.push(StackFrame {
        id: frame_id,
        name: name.to_string(),
        source: Some(Source {
            name: Some(state.source_name.clone()),
            path: None,
            source_reference: None,
        }),
        line,
        column,
        end_line: None,
        end_column: None,
    });
}

pub fn add_variable(
    state: &mut DebuggerState,
    reference: i32,
    name: &str,
    value: &str,
    type_: &str,
) {
    let vars = state.variables.entry(reference).or_insert_with(Vec::new);
    vars.push(Variable {
        name: name.to_string(),
        value: value.to_string(),
        type_: Some(type_.to_string()),
        variables_reference: None,
        indexed_variables: None,
    });
}

pub fn clear_variables(state: &mut DebuggerState) {
    state.variables.clear();
    state.next_var_ref = 1;
}

pub fn create_variable_reference(state: &mut DebuggerState) -> i32 {
    let ref_id = state.next_var_ref;
    state.next_var_ref += 1;
    ref_id
}

pub fn run_debugger_session<R: Read, W: Write>(reader: &mut R, writer: &mut W, state: DAPHandler) {
    loop {
        let request = match read_message(reader) {
            Some(req) => req,
            None => break,
        };

        let method = request.method.clone();
        let id = request.id.clone();
        let params = request.params.clone();

        let mut state_guard = state.lock().unwrap();

        let result = match method.as_str() {
            "initialize" => Ok(handle_initialize(&mut state_guard, params)),
            "launch" => handle_launch(&mut state_guard, params),
            "configurationDone" => Ok(handle_configuration_done(&mut state_guard)),
            "setBreakpoints" => handle_set_breakpoints(&mut state_guard, params),
            "setExceptionBreakpoints" => handle_set_exception_breakpoints(&mut state_guard, params),
            "threads" => Ok(handle_threads(&mut state_guard)),
            "stackTrace" => handle_stack_trace(&mut state_guard, params),
            "scopes" => handle_scopes(&mut state_guard, params),
            "variables" => handle_variables(&mut state_guard, params),
            "continue" => handle_continue(&mut state_guard, params),
            "pause" => handle_pause(&mut state_guard, params),
            "next" => handle_next(&mut state_guard, params),
            "stepIn" => handle_step_in(&mut state_guard, params),
            "stepOut" => handle_step_out(&mut state_guard, params),
            "evaluate" => handle_evaluate(&mut state_guard, params),
            "disconnect" => handle_disconnect(&mut state_guard, params),
            "terminate" => Ok(handle_terminate(&mut state_guard)),
            "restart" => handle_restart(&mut state_guard, params),
            "loadedSources" | "modules" | "source" => Ok(serde_json::Value::Null),
            _ => Err(format!("Unknown method: {}", method)),
        };

        match result {
            Ok(response) => {
                send_response(writer, id, response);
            }
            Err(e) => {
                send_error_response(writer, id, -32601, &e);
            }
        }
    }
}

pub fn start_debug_server() -> DAPHandler {
    let state = create_debugger_state();
    let state_clone = Arc::clone(&state);

    thread::spawn(move || {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();

        run_debugger_session(&mut stdin.lock(), &mut stdout.lock(), state_clone);
    });

    state
}

pub fn send_stopped_event<W: Write>(writer: &mut W, reason: &str, thread_id: i32) {
    let body = StoppedEventBody {
        reason: reason.to_string(),
        description: None,
        thread_id: Some(thread_id),
        all_threads_stopped: Some(true),
        hit_breakpoint_ids: None,
    };

    send_event(writer, "stopped", Some(serde_json::to_value(body).unwrap()));
}

pub fn send_continued_event<W: Write>(writer: &mut W, thread_id: i32) {
    let body = ContinuedEventBody {
        thread_id,
        all_threads_continued: Some(true),
    };

    send_event(
        writer,
        "continued",
        Some(serde_json::to_value(body).unwrap()),
    );
}

pub fn send_exited_event<W: Write>(writer: &mut W, exit_code: i32) {
    let body = ExitedEventBody {
        exit_code,
        description: None,
    };

    send_event(writer, "exited", Some(serde_json::to_value(body).unwrap()));
}

pub fn send_terminated_event<W: Write>(writer: &mut W) {
    let body = TerminatedEventBody { restart: None };

    send_event(
        writer,
        "terminated",
        Some(serde_json::to_value(body).unwrap()),
    );
}

pub fn send_output_event<W: Write>(writer: &mut W, output: &str, output_type: &str) {
    let body = OutputEventBody {
        output: output.to_string(),
        output_type: Some(output_type.to_string()),
        group: None,
        variables_reference: None,
        source: None,
        line: None,
        column: None,
    };

    send_event(writer, "output", Some(serde_json::to_value(body).unwrap()));
}

pub fn send_breakpoint_event<W: Write>(writer: &mut W, reason: &str, breakpoint: Breakpoint) {
    let body = BreakpointEventBody {
        reason: reason.to_string(),
        breakpoint,
    };

    send_event(
        writer,
        "breakpoint",
        Some(serde_json::to_value(body).unwrap()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_state_default() {
        let state = DebuggerState::default();
        assert_eq!(state.state, DebugState::Stopped);
        assert_eq!(state.current_thread_id, 1);
        assert!(!state.initialized);
    }

    #[test]
    fn test_breakpoint_hit() {
        let mut state = DebuggerState::default();
        set_breakpoint_hit(&mut state, 10, 5);
        assert_eq!(state.line, 10);
        assert_eq!(state.column, 5);
        assert_eq!(state.state, DebugState::Paused);
    }

    #[test]
    fn test_add_stack_frame() {
        let mut state = DebuggerState::default();
        add_variable(&mut state, 0, "x", "42", "Int");
        add_variable(&mut state, 0, "y", "hello", "String");

        let vars = state.variables.get(&0).unwrap();
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn test_evaluate_expression() {
        let state = DebuggerState::default();

        let (result, type_, _) = evaluate_expression(&state, "line");
        assert_eq!(result, "1");
        assert_eq!(type_, Some("Int".to_string()));

        let (result, _, _) = evaluate_expression(&state, "state");
        assert_eq!(result, "Stopped");
    }

    #[test]
    fn test_initialize_response() {
        let response = InitializeResponse::default();
        assert!(response.supports_conditional_breakpoints);
        assert!(response.supports_configuration_done_request);
    }
}
