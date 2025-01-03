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
) {
    let logger = &*LOGGER;
    let width = crossterm::terminal::size().map(|ws| ws.0).unwrap_or(80) as usize - right_pad + 1;
    let cursor_pos = logger.cursor_pos.load(Relaxed) as usize;
    let available_width = width.saturating_sub(cursor_pos);

    if text.chars().count() > available_width {
        let split_point = text
            .char_indices()
            .take(available_width)
            .last()
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        logger.write_string(&text[..split_point], color);
        logger.write_string(" ", color);
        if print_filename {
            logger.write_string(file_name, None);
        }
        logger.add_newline();
        logger.pad_to_column(logger.tab_stop(TabStop::Content));
        safe_wrap_print_json(&text[split_point..], color, right_pad, "", false);
    } else {
        logger.write_string(text, color);
    }
}

pub(crate) fn print_json_color(record: &RichLoggerRecord, j: &[JsonToken]) {
    for token in j {
        match &token.kind {
            TokenKind::Operator(o) => match o {
                Operator::JsonLBrace => {
                    safe_wrap_print_json(
                        "{",
                        None,
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
                Operator::JsonRBrace => {
                    safe_wrap_print_json(
                        "}",
                        None,
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
                Operator::JsonLBracket => {
                    safe_wrap_print_json(
                        "[",
                        None,
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
                Operator::JsonRBracket => {
                    safe_wrap_print_json(
                        "]",
                        None,
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
                Operator::JsonColon => {
                    safe_wrap_print_json(
                        ": ",
                        None,
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
                Operator::JsonComma => {
                    safe_wrap_print_json(
                        ", ",
                        None,
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
            },
            TokenKind::Literal(l) => match l {
                Literal::StringLiteral(s) => {
                    safe_wrap_print_json(
                        &format!(r#""{}""#, &s),
                        Some(Colors {
                            foreground: Some(Color::Green),
                            background: None,
                        }),
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
                Literal::NumberLiteral(n) => {
                    safe_wrap_print_json(
                        &n.to_string(),
                        Some(Colors {
                            foreground: Some(Color::DarkBlue),
                            background: None,
                        }),
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
                Literal::BooleanLiteral(b) => {
                    safe_wrap_print_json(
                        &b.to_string(),
                        Some(Colors {
                            foreground: Some(Color::Red),
                            background: None,
                        }),
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
                Literal::NullLiteral => {
                    safe_wrap_print_json(
                        "null",
                        Some(Colors {
                            foreground: Some(Color::Yellow),
                            background: None,
                        }),
                        record.file_name.len() + 1,
                        record.file_name.as_str(),
                        true,
                    );
                }
            },
        }
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
