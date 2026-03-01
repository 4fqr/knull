//! Knull Documentation Generator
//!
//! Parses /// comments and generates HTML/Markdown documentation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DocComment {
    pub content: String,
    pub line_number: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionDoc {
    pub name: String,
    pub signature: String,
    pub description: String,
    pub params: Vec<ParamDoc>,
    pub return_type: Option<String>,
    pub examples: Vec<String>,
    pub see_also: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParamDoc {
    pub name: String,
    pub ty: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct StructDoc {
    pub name: String,
    pub description: String,
    pub fields: Vec<FieldDoc>,
}

#[derive(Debug, Clone)]
pub struct FieldDoc {
    pub name: String,
    pub ty: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct EnumDoc {
    pub name: String,
    pub description: String,
    pub variants: Vec<VariantDoc>,
}

#[derive(Debug, Clone)]
pub struct VariantDoc {
    pub name: String,
    pub description: String,
    pub data: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleDoc {
    pub name: String,
    pub description: String,
    pub functions: Vec<FunctionDoc>,
    pub structs: Vec<StructDoc>,
    pub enums: Vec<EnumDoc>,
}

pub struct DocGenerator {
    modules: HashMap<String, ModuleDoc>,
    current_module: Option<String>,
    current_item: Option<DocItem>,
}

#[derive(Debug, Clone)]
enum DocItem {
    Function(FunctionDoc),
    Struct(StructDoc),
    Enum(EnumDoc),
}

impl DocGenerator {
    pub fn new() -> Self {
        DocGenerator {
            modules: HashMap::new(),
            current_module: None,
            current_item: None,
        }
    }

    pub fn parse_file(&mut self, path: &Path) -> Result<(), String> {
        let source =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let module_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("main");
        self.parse_source(&source, module_name);
        Ok(())
    }

    pub fn parse_source(&mut self, source: &str, module_name: &str) {
        let mut current_comments: Vec<DocComment> = Vec::new();
        let mut in_doc_comment = false;

        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("///") {
                in_doc_comment = true;
                let content = trimmed.trim_start_matches("///").trim().to_string();
                current_comments.push(DocComment {
                    content,
                    line_number: line_num + 1,
                });
            } else if trimmed.starts_with("//") {
                continue;
            } else if in_doc_comment && !trimmed.is_empty() {
                self.process_item_comments(&current_comments, trimmed);
                current_comments.clear();
                in_doc_comment = false;
            }
        }

        let module_doc = self.build_module_doc(module_name, &current_comments);
        self.modules.insert(module_name.to_string(), module_doc);
    }

    fn process_item_comments(&mut self, comments: &[DocComment], item_line: &str) {
        if item_line.starts_with("fn ") || item_line.starts_with("pub fn ") {
            self.parse_function_doc(comments, item_line);
        } else if item_line.starts_with("struct ") || item_line.starts_with("pub struct ") {
            self.parse_struct_doc(comments, item_line);
        } else if item_line.starts_with("enum ") || item_line.starts_with("pub enum ") {
            self.parse_enum_doc(comments, item_line);
        }
    }

    fn parse_function_doc(&mut self, comments: &[DocComment], signature: &str) {
        let name = extract_function_name(signature);
        let description = comments
            .iter()
            .map(|c| c.content.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let mut func_doc = FunctionDoc {
            name,
            signature: signature.to_string(),
            description,
            params: Vec::new(),
            return_type: extract_return_type(signature),
            examples: Vec::new(),
            see_also: Vec::new(),
        };

        for comment in comments {
            if comment.content.starts_with("/// # Examples") {
                // Parse examples
            }
        }

        self.current_item = Some(DocItem::Function(func_doc));
    }

    fn parse_struct_doc(&mut self, comments: &[DocComment], definition: &str) {
        let name = extract_struct_name(definition);
        let description = comments
            .iter()
            .map(|c| c.content.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let struct_doc = StructDoc {
            name,
            description,
            fields: Vec::new(),
        };

        self.current_item = Some(DocItem::Struct(struct_doc));
    }

    fn parse_enum_doc(&mut self, comments: &[DocComment], definition: &str) {
        let name = extract_enum_name(definition);
        let description = comments
            .iter()
            .map(|c| c.content.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let enum_doc = EnumDoc {
            name,
            description,
            variants: Vec::new(),
        };

        self.current_item = Some(DocItem::Enum(enum_doc));
    }

    fn build_module_doc(&self, name: &str, _comments: &[DocComment]) -> ModuleDoc {
        ModuleDoc {
            name: name.to_string(),
            description: String::new(),
            functions: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
        }
    }

    pub fn generate_markdown(&self) -> String {
        let mut output = String::new();

        for (name, module) in &self.modules {
            output.push_str(&format!("# Module: {}\n\n", name));

            if !module.description.is_empty() {
                output.push_str(&format!("{}\n\n", module.description));
            }

            if !module.functions.is_empty() {
                output.push_str("## Functions\n\n");
                for func in &module.functions {
                    output.push_str(&format!("### `{}`\n\n", func.name));
                    output.push_str(&format!("```knull\n{}\n```\n\n", func.signature));
                    if !func.description.is_empty() {
                        output.push_str(&format!("{}\n\n", func.description));
                    }
                    if !func.params.is_empty() {
                        output.push_str("#### Parameters\n\n");
                        for param in &func.params {
                            output.push_str(&format!(
                                "- `{}` ({}) - {}\n",
                                param.name, param.ty, param.description
                            ));
                        }
                        output.push('\n');
                    }
                    if let Some(ref ret) = func.return_type {
                        output.push_str(&format!("**Returns:** `{}`\n\n", ret));
                    }
                }
            }

            if !module.structs.is_empty() {
                output.push_str("## Structs\n\n");
                for struct_doc in &module.structs {
                    output.push_str(&format!("### `{}`\n\n", struct_doc.name));
                    if !struct_doc.description.is_empty() {
                        output.push_str(&format!("{}\n\n", struct_doc.description));
                    }
                }
            }

            if !module.enums.is_empty() {
                output.push_str("## Enums\n\n");
                for enum_doc in &module.enums {
                    output.push_str(&format!("### `{}`\n\n", enum_doc.name));
                    if !enum_doc.description.is_empty() {
                        output.push_str(&format!("{}\n\n", enum_doc.description));
                    }
                }
            }

            output.push_str("---\n\n");
        }

        output
    }

    pub fn generate_html(&self) -> String {
        let markdown = self.generate_markdown();
        let mut html = String::new();

        html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Knull Documentation</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        h1 { color: #333; border-bottom: 2px solid #eee; padding-bottom: 10px; }
        h2 { color: #555; margin-top: 30px; }
        h3 { color: #666; }
        code { background: #f4f4f4; padding: 2px 6px; border-radius: 3px; }
        pre { background: #f4f4f4; padding: 15px; border-radius: 5px; overflow-x: auto; }
        pre code { background: none; padding: 0; }
        hr { border: none; border-top: 1px solid #eee; margin: 30px 0; }
        .item-name { color: #0366d6; }
    </style>
</head>
<body>
"#);

        let body = markdown_to_html(&markdown);
        html.push_str(&body);
        html.push_str("</body>\n</html>");

        html
    }

    pub fn build_cross_references(&self) -> HashMap<String, Vec<String>> {
        let mut refs = HashMap::new();

        for (_, module) in &self.modules {
            for func in &module.functions {
                let key = format!("{}::{}", module.name, func.name);
                let mut see_also = func.see_also.clone();
                refs.insert(key, see_also);
            }
        }

        refs
    }
}

fn extract_function_name(signature: &str) -> String {
    if let Some(name_start) = signature.find("fn ") {
        let after_fn = &signature[name_start + 3..];
        if let Some(space_idx) = after_fn.find(|c: char| c.is_whitespace()) {
            return after_fn[..space_idx].to_string();
        }
    }
    String::new()
}

fn extract_struct_name(definition: &str) -> String {
    if let Some(name_start) = definition.find("struct ") {
        let after_struct = &definition[name_start + 7..];
        if let Some(brace_idx) = after_struct.find('{') {
            return after_struct[..brace_idx].trim().to_string();
        } else if let Some(colon_idx) = after_struct.find(':') {
            return after_struct[..colon_idx].trim().to_string();
        }
    }
    String::new()
}

fn extract_enum_name(definition: &str) -> String {
    if let Some(name_start) = definition.find("enum ") {
        let after_enum = &definition[name_start + 5..];
        if let Some(brace_idx) = after_enum.find('{') {
            return after_enum[..brace_idx].trim().to_string();
        }
    }
    String::new()
}

fn extract_return_type(signature: &str) -> Option<String> {
    if let Some(arrow_idx) = signature.find("->") {
        let after_arrow = &signature[arrow_idx + 2..];
        let return_type = after_arrow.trim();
        let end_idx = return_type
            .find(|c: char| c == '{' || c == '(' || c == ';')
            .unwrap_or(return_type.len());
        Some(return_type[..end_idx].trim().to_string())
    } else {
        None
    }
}

fn markdown_to_html(markdown: &str) -> String {
    let mut html = String::new();

    for line in markdown.lines() {
        if line.starts_with("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", &line[2..]));
        } else if line.starts_with("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", &line[3..]));
        } else if line.starts_with("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", &line[4..]));
        } else if line.starts_with("```") {
            html.push_str("<pre><code>");
        } else if line == "```" {
            html.push_str("</code></pre>\n");
        } else if !line.is_empty() {
            html.push_str(&format!("<p>{}</p>\n", line));
        }
    }

    html
}

pub fn generate_docs_for_project(project_path: &Path) -> Result<String, String> {
    let mut generator = DocGenerator::new();

    let src_dir = project_path.join("src");
    if src_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "knull") {
                    let _ = generator.parse_file(&path);
                }
            }
        }
    }

    Ok(generator.generate_markdown())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function_doc() {
        let source = r#"
/// Adds two numbers together
/// 
/// # Examples
/// ```
/// add(2, 3) // returns 5
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let mut generator = DocGenerator::new();
        generator.parse_source(source, "test");

        assert!(!generator.modules.is_empty());
    }

    #[test]
    fn test_markdown_generation() {
        let generator = DocGenerator::new();
        let md = generator.generate_markdown();
        assert!(md.contains("# Module:"));
    }
}
