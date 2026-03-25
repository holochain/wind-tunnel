use std::fmt::Write;

/// Escape a measurement name for InfluxDB line protocol.
/// Commas and spaces must be escaped with a backslash.
fn escape_measurement(s: &str) -> String {
    s.replace(',', "\\,").replace(' ', "\\ ")
}

/// Escape a tag key, tag value, or field key for InfluxDB line protocol.
/// Commas, equals signs, and spaces must be escaped with a backslash.
fn escape_tag(s: &str) -> String {
    s.replace(',', "\\,")
        .replace('=', "\\=")
        .replace(' ', "\\ ")
}

/// A single field value in InfluxDB line protocol.
#[allow(dead_code)]
pub enum FieldValue {
    Float(f64),
    Integer(i64),
    UnsignedInteger(u64),
    Bool(bool),
    String(String),
}

impl std::fmt::Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValue::Float(v) => {
                if v.is_infinite() || v.is_nan() {
                    // InfluxDB doesn't accept NaN/Inf, write 0.0 as fallback
                    write!(f, "0.0")
                } else {
                    write!(f, "{v}")
                }
            }
            FieldValue::Integer(v) => write!(f, "{v}i"),
            FieldValue::UnsignedInteger(v) => write!(f, "{v}u"),
            FieldValue::Bool(v) => {
                if *v {
                    write!(f, "t")
                } else {
                    write!(f, "f")
                }
            }
            FieldValue::String(v) => write!(f, "\"{}\"", v.replace('"', "\\\"")),
        }
    }
}

/// Write a single InfluxDB line protocol entry to the given writer.
///
/// Format: `measurement,tag1=val1,tag2=val2 field1=val1,field2=val2 timestamp_ns`
pub fn write_line_protocol(
    w: &mut dyn Write,
    measurement: &str,
    tags: &[(&str, &str)],
    fields: &[(&str, FieldValue)],
    timestamp_ns: u128,
) -> std::fmt::Result {
    if fields.is_empty() {
        return Ok(());
    }

    // Measurement name
    w.write_str(&escape_measurement(measurement))?;

    // Tags (sorted for deterministic output)
    let mut sorted_tags: Vec<_> = tags.iter().collect();
    sorted_tags.sort_by_key(|(k, _)| *k);
    for (key, value) in &sorted_tags {
        write!(w, ",{}={}", escape_tag(key), escape_tag(value))?;
    }

    // Space separator between tags and fields
    w.write_char(' ')?;

    // Fields
    for (i, (key, value)) in fields.iter().enumerate() {
        if i > 0 {
            w.write_char(',')?;
        }
        write!(w, "{}={}", escape_tag(key), value)?;
    }

    // Timestamp
    write!(w, " {timestamp_ns}")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_line_protocol() {
        let mut buf = String::new();
        write_line_protocol(
            &mut buf,
            "wt.instruments.operation_duration",
            &[("run_id", "abc123"), ("operation_id", "call_zome")],
            &[("value", FieldValue::Float(0.123456))],
            1700000000000000000,
        )
        .unwrap();
        assert_eq!(
            buf,
            "wt.instruments.operation_duration,operation_id=call_zome,run_id=abc123 value=0.123456 1700000000000000000"
        );
    }

    #[test]
    fn multiple_fields() {
        let mut buf = String::new();
        write_line_protocol(
            &mut buf,
            "wt.instruments.operation_duration",
            &[("run_id", "abc")],
            &[
                ("count", FieldValue::UnsignedInteger(100)),
                ("sum", FieldValue::Float(12.5)),
                ("min", FieldValue::Float(0.01)),
                ("max", FieldValue::Float(0.5)),
            ],
            1700000000000000000,
        )
        .unwrap();
        assert_eq!(
            buf,
            "wt.instruments.operation_duration,run_id=abc count=100u,sum=12.5,min=0.01,max=0.5 1700000000000000000"
        );
    }

    #[test]
    fn escaping() {
        let mut buf = String::new();
        write_line_protocol(
            &mut buf,
            "my measurement",
            &[("tag with,special=chars", "val=ue")],
            &[("field", FieldValue::Integer(42))],
            1000,
        )
        .unwrap();
        assert_eq!(
            buf,
            r"my\ measurement,tag\ with\,special\=chars=val\=ue field=42i 1000"
        );
    }

    #[test]
    fn string_field_escaping() {
        let mut buf = String::new();
        write_line_protocol(
            &mut buf,
            "test",
            &[],
            &[("msg", FieldValue::String("hello \"world\"".to_string()))],
            1000,
        )
        .unwrap();
        assert_eq!(buf, r#"test msg="hello \"world\"" 1000"#);
    }

    #[test]
    fn bool_field() {
        let mut buf = String::new();
        write_line_protocol(
            &mut buf,
            "test",
            &[],
            &[("ok", FieldValue::Bool(true))],
            1000,
        )
        .unwrap();
        assert_eq!(buf, "test ok=t 1000");
    }

    #[test]
    fn empty_fields_produces_no_output() {
        let mut buf = String::new();
        write_line_protocol(&mut buf, "test", &[("t", "v")], &[], 1000).unwrap();
        assert_eq!(buf, "");
    }
}
