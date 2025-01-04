use crate::{RichLoggerRecord, TabStop, LOGGER};
use crossterm::style::{Color, Colors};
use serde_json::Value;
use std::sync::atomic::Ordering::Relaxed;
#[cfg(not(feature = "async"))]
use {crate::log_impl, crate::ContentType, log::Level, serde::Serialize};

pub enum Operator {
    JsonLBrace,
    JsonRBrace,
    JsonLBracket,
    JsonRBracket,
    JsonColon,
    JsonComma,
}

pub enum Literal {
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    NullLiteral,
}

pub enum TokenKind {
    Operator(Operator),
    Literal(Literal),
}

pub struct JsonToken {
    pub kind: TokenKind,
    pub content: String,
}

fn safe_wrap_print_json(
    text: &str,
    color: Option<Colors>,
    right_pad: usize,
    file_name: &str,
    print_filename: bool,
) -> bool {
    let logger = &*LOGGER;
    let width = crossterm::terminal::size().map(|ws| ws.0).unwrap_or(80) as usize - right_pad + 1;
    let cursor_pos = logger.cursor_pos.load(Relaxed) as usize;
    let available_width = width.saturating_sub(cursor_pos);

    if text.len() >= available_width {
        let split_point = text
            .char_indices()
            .take(available_width)
            .last()
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        logger.write_string(&text[..=split_point], color);
        logger.pad_to_column((width + 1) as i32);
        let mut printed_filename = false;
        if print_filename {
            logger.write_string(file_name, None);
            printed_filename = true;
        }
        logger.add_newline();
        logger.pad_to_column(logger.tab_stop(TabStop::Content));
        return safe_wrap_print_json(&text[(split_point + 1)..], color, right_pad, "", false)
            || printed_filename;
    } else {
        logger.write_string(text, color);
        return false;
    }
}

pub(crate) fn print_json_color(record: &RichLoggerRecord, j: &[JsonToken]) {
    let mut should_print_filename = true;
    for token in j {
        match &token.kind {
            TokenKind::Operator(o) => match o {
                Operator::JsonLBrace => {
                    should_print_filename = safe_wrap_print_json(
                        "{",
                        None,
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
                Operator::JsonRBrace => {
                    should_print_filename = safe_wrap_print_json(
                        "}",
                        None,
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
                Operator::JsonLBracket => {
                    should_print_filename = safe_wrap_print_json(
                        "[",
                        None,
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
                Operator::JsonRBracket => {
                    should_print_filename = safe_wrap_print_json(
                        "]",
                        None,
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
                Operator::JsonColon => {
                    should_print_filename = safe_wrap_print_json(
                        ": ",
                        None,
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
                Operator::JsonComma => {
                    should_print_filename = safe_wrap_print_json(
                        ", ",
                        None,
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
            },
            TokenKind::Literal(l) => match l {
                Literal::StringLiteral(s) => {
                    should_print_filename = safe_wrap_print_json(
                        &format!(r#""{}""#, &s),
                        Some(Colors {
                            foreground: Some(Color::Green),
                            background: None,
                        }),
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
                Literal::NumberLiteral(n) => {
                    should_print_filename = safe_wrap_print_json(
                        &n.to_string(),
                        Some(Colors {
                            foreground: Some(Color::DarkBlue),
                            background: None,
                        }),
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
                Literal::BooleanLiteral(b) => {
                    should_print_filename = safe_wrap_print_json(
                        &b.to_string(),
                        Some(Colors {
                            foreground: Some(Color::Red),
                            background: None,
                        }),
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
                Literal::NullLiteral => {
                    should_print_filename = safe_wrap_print_json(
                        "null",
                        Some(Colors {
                            foreground: Some(Color::Yellow),
                            background: None,
                        }),
                        record.file_name.len(),
                        record.file_name.as_str(),
                        should_print_filename,
                    ) ^ should_print_filename;
                }
            },
        }
    }

    if should_print_filename {
        let logger = &*LOGGER;
        let width = crossterm::terminal::size().map(|ws| ws.0).unwrap_or(80) as usize;
        logger.pad_to_column((width - record.file_name.len()) as i32);
        logger.write_string(record.file_name.as_str(), None);
    }
}

pub(crate) fn tokenize_json_value(json_value: &Value) -> Vec<JsonToken> {
    let mut tokens = Vec::new();

    match json_value {
        Value::Null => tokens.push(JsonToken {
            kind: TokenKind::Literal(Literal::NullLiteral),
            content: "null".to_string(),
        }),
        Value::Bool(b) => tokens.push(JsonToken {
            kind: TokenKind::Literal(Literal::BooleanLiteral(*b)),
            content: if *b {
                "true".to_string()
            } else {
                "false".to_string()
            },
        }),
        Value::Number(n) => tokens.push(JsonToken {
            kind: TokenKind::Literal(Literal::NumberLiteral(n.as_f64().unwrap())),
            content: n.to_string(),
        }),
        Value::String(s) => tokens.push(JsonToken {
            kind: TokenKind::Literal(Literal::StringLiteral(s.clone())),
            content: format!("\"{}\"", s),
        }),
        Value::Array(a) => {
            tokens.push(JsonToken {
                kind: TokenKind::Operator(Operator::JsonLBracket),
                content: "[".to_string(),
            });

            for (i, v) in a.iter().enumerate() {
                tokens.extend(tokenize_json_value(v));

                if i < a.len() - 1 {
                    tokens.push(JsonToken {
                        kind: TokenKind::Operator(Operator::JsonComma),
                        content: ",".to_string(),
                    });
                }
            }

            tokens.push(JsonToken {
                kind: TokenKind::Operator(Operator::JsonRBracket),
                content: "]".to_string(),
            });
        }
        Value::Object(o) => {
            tokens.push(JsonToken {
                kind: TokenKind::Operator(Operator::JsonLBrace),
                content: "{".to_string(),
            });

            for (i, (k, v)) in o.iter().enumerate() {
                tokens.push(JsonToken {
                    kind: TokenKind::Literal(Literal::StringLiteral(k.clone())),
                    content: format!("\"{}\"", k),
                });

                tokens.push(JsonToken {
                    kind: TokenKind::Operator(Operator::JsonColon),
                    content: ":".to_string(),
                });

                tokens.extend(tokenize_json_value(v));

                if i < o.len() - 1 {
                    tokens.push(JsonToken {
                        kind: TokenKind::Operator(Operator::JsonComma),
                        content: ",".to_string(),
                    });
                }
            }

            tokens.push(JsonToken {
                kind: TokenKind::Operator(Operator::JsonRBrace),
                content: "}".to_string(),
            });
        }
    }

    tokens
}

#[cfg(not(feature = "async"))]
pub(crate) fn print_json_pretty<T: Serialize>(value: &T, file_name: String, level: Level) {
    let json_value = serde_json::to_value(value).unwrap();
    let json_content = ContentType::JsonContent(tokenize_json_value(&json_value));
    log_impl(RichLoggerRecord {
        file_name,
        level,
        content: json_content,
    });
}
