use serde_json::Value;

/// Wire event name for additive NDJSON stream lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamEventKind {
    Chunk,
    Tool,
    Step,
    Plan,
    Activity,
    Done,
    Error,
    Unknown,
}

/// Strongly typed stream event mirroring the NDJSON contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    Chunk {
        index: u64,
        text: String,
        turn_id: Option<String>,
        sequence: Option<u64>,
    },
    Tool {
        index: u64,
        name: String,
        phase: String,
        detail: String,
        tool_call_id: Option<String>,
        turn_id: Option<String>,
        sequence: Option<u64>,
        elapsed_ms: Option<u64>,
    },
    Step {
        index: u64,
        phase: String,
        summary: String,
        turn_id: Option<String>,
        sequence: Option<u64>,
    },
    Plan {
        index: u64,
        phase: String,
        title: String,
        detail: String,
        sequence: Option<u64>,
    },
    Activity {
        index: u64,
        phase: String,
        summary: String,
        detail: String,
        sequence: Option<u64>,
    },
    Done {
        index: u64,
    },
    Error {
        message: String,
        code: String,
    },
}

impl StreamEvent {
    pub fn kind(&self) -> StreamEventKind {
        match self {
            Self::Chunk { .. } => StreamEventKind::Chunk,
            Self::Tool { .. } => StreamEventKind::Tool,
            Self::Step { .. } => StreamEventKind::Step,
            Self::Plan { .. } => StreamEventKind::Plan,
            Self::Activity { .. } => StreamEventKind::Activity,
            Self::Done { .. } => StreamEventKind::Done,
            Self::Error { .. } => StreamEventKind::Error,
        }
    }

    pub fn turn_id(&self) -> Option<&str> {
        match self {
            Self::Chunk { turn_id, .. }
            | Self::Tool { turn_id, .. }
            | Self::Step { turn_id, .. } => turn_id.as_deref(),
            _ => None,
        }
    }
}

/// Parse one NDJSON line into a [`StreamEvent`].
pub fn parse_stream_line(line: &str) -> Result<StreamEvent, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("empty line".to_string());
    }
    let value: Value =
        serde_json::from_str(trimmed).map_err(|err| format!("invalid json: {err}"))?;
    parse_stream_value(&value)
}

fn parse_stream_value(value: &Value) -> Result<StreamEvent, String> {
    let event = value
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or("chunk");

    match event {
        "chunk" => Ok(StreamEvent::Chunk {
            index: require_u64(value, "index")?,
            text: optional_string(value, "text"),
            turn_id: optional_string_opt(value, "turn_id"),
            sequence: optional_u64(value, "sequence"),
        }),
        "tool" => Ok(StreamEvent::Tool {
            index: require_u64(value, "index")?,
            name: optional_string(value, "name"),
            phase: optional_string(value, "phase"),
            detail: optional_string(value, "detail"),
            tool_call_id: optional_string_opt(value, "tool_call_id"),
            turn_id: optional_string_opt(value, "turn_id"),
            sequence: optional_u64(value, "sequence"),
            elapsed_ms: optional_u64(value, "elapsed_ms"),
        }),
        "step" => Ok(StreamEvent::Step {
            index: require_u64(value, "index")?,
            phase: optional_string(value, "phase"),
            summary: optional_string(value, "summary"),
            turn_id: optional_string_opt(value, "turn_id"),
            sequence: optional_u64(value, "sequence"),
        }),
        "plan" => Ok(StreamEvent::Plan {
            index: require_u64(value, "index")?,
            phase: optional_string(value, "phase"),
            title: optional_string(value, "title"),
            detail: optional_string(value, "detail"),
            sequence: optional_u64(value, "sequence"),
        }),
        "activity" => Ok(StreamEvent::Activity {
            index: require_u64(value, "index")?,
            phase: optional_string(value, "phase"),
            summary: optional_string(value, "summary"),
            detail: optional_string(value, "detail"),
            sequence: optional_u64(value, "sequence"),
        }),
        "done" => Ok(StreamEvent::Done {
            index: require_u64(value, "index")?,
        }),
        "error" => Ok(StreamEvent::Error {
            message: optional_string(value, "message"),
            code: optional_string(value, "code"),
        }),
        other => Err(format!("unknown event: {other}")),
    }
}

fn require_u64(value: &Value, field: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("missing or invalid field: {field}"))
}

fn optional_string(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn optional_string_opt(value: &Value, field: &str) -> Option<String> {
    let s = optional_string(value, field);
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn optional_u64(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(|v| v.as_u64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chunk_event() {
        let event = parse_stream_line(r#"{"event":"chunk","index":0,"text":"hi"}"#).unwrap();
        assert!(matches!(
            event,
            StreamEvent::Chunk {
                index: 0,
                text,
                ..
            } if text == "hi"
        ));
    }

    #[test]
    fn parses_tool_with_call_id() {
        let event = parse_stream_line(
            r#"{"event":"tool","index":1,"name":"fs.read","phase":"running","detail":"a.rs","tool_call_id":"t:1"}"#,
        )
        .unwrap();
        assert!(matches!(
            event,
            StreamEvent::Tool {
                name,
                phase,
                tool_call_id: Some(id),
                ..
            } if name == "fs.read" && phase == "running" && id == "t:1"
        ));
    }
}
