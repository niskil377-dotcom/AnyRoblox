use serde::Serialize;
use std::{fs, path::Path};

/// An inventory of declarations in a contributing model.cfg, before inheritance or preprocessing.
#[derive(Default, Serialize)]
pub struct RigConfig {
    pub path: String,
    pub skeletons: Vec<String>,
    pub animations: Vec<String>,
    pub note: Option<String>,
}

fn tokens(text: &str) -> Vec<String> {
    let mut c = text.chars().peekable();
    let mut result = Vec::new();
    while let Some(ch) = c.next() {
        match ch {
            '/' if c.peek() == Some(&'/') => {
                c.by_ref().take_while(|v| *v != '\n').for_each(drop);
            }
            '/' if c.peek() == Some(&'*') => {
                c.next();
                while let Some(v) = c.next() {
                    if v == '*' && c.peek() == Some(&'/') {
                        c.next();
                        break;
                    }
                }
            }
            '"' => {
                while let Some(v) = c.next() {
                    if v == '"' {
                        if c.peek() != Some(&'"') {
                            break;
                        }
                        c.next();
                    }
                }
                result.push(String::from("\"\""));
            }
            '{' | '}' | ';' => result.push(ch.to_string()),
            v if v.is_alphanumeric() || v == '_' => {
                let mut word = v.to_string();
                while let Some(v) = c.next_if(|v| v.is_alphanumeric() || *v == '_') {
                    word.push(v);
                }
                result.push(word);
            }
            _ => (),
        }
    }
    result
}

pub fn inventory(file: &Path, path: String) -> RigConfig {
    let mut info = RigConfig {
        path,
        ..Default::default()
    };
    let text = fs::metadata(file)
        .ok()
        .filter(|m| m.len() <= 1024 * 1024)
        .and_then(|_| fs::read_to_string(file).ok());
    let Some(text) = text.filter(|s| !s.contains('\0')) else {
        info.note = Some("Binary or unreadable configuration, or file exceeds 1 MiB".into());
        return info;
    };
    let tokens = tokens(&text);
    let mut classes: Vec<&str> = Vec::new();
    let mut pending = None;
    for (i, t) in tokens.iter().enumerate() {
        if t.eq_ignore_ascii_case("class") {
            pending = tokens.get(i + 1).map(String::as_str);
        }
        match t.as_str() {
            "{" => {
                let name = pending.take().unwrap_or("");
                if !name.is_empty() {
                    if classes
                        .last()
                        .is_some_and(|s| s.eq_ignore_ascii_case("CfgSkeletons"))
                    {
                        info.skeletons.push(name.to_owned());
                    }
                    if classes
                        .last()
                        .is_some_and(|s| s.eq_ignore_ascii_case("Animations"))
                    {
                        info.animations
                            .push(format!("{}::{name}", classes.join("::")));
                    }
                }
                classes.push(name);
            }
            "}" => {
                classes.pop();
                pending = None;
            }
            ";" => pending = None,
            _ => (),
        }
    }
    info
}
