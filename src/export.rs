//! Export audit entries to JSON Lines and CSV formats.
//!
//! All export functions write to any [`std::io::Write`] target, making them
//! composable with files, buffers, network streams, and stdout.

use std::io::Write;

use crate::entry::AuditEntry;

/// Export entries as JSON Lines (one JSON object per line).
pub fn to_jsonl(entries: &[AuditEntry], mut writer: impl Write) -> crate::Result<()> {
    for entry in entries {
        let json = serde_json::to_string(entry)?;
        writeln!(writer, "{json}")?;
    }
    Ok(())
}

/// Export entries as CSV with a header row.
///
/// Columns: `id,timestamp,severity,source,action,details,agent_id,prev_hash,hash`
///
/// The `details` field is serialized as a JSON string within the CSV cell.
pub fn to_csv(entries: &[AuditEntry], mut writer: impl Write) -> crate::Result<()> {
    writeln!(
        writer,
        "id,timestamp,severity,source,action,details,agent_id,prev_hash,hash"
    )?;
    for entry in entries {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{}",
            entry.id(),
            entry.timestamp().to_rfc3339(),
            entry.severity().as_str(),
            csv_escape(entry.source()),
            csv_escape(entry.action()),
            csv_escape(&entry.details().to_string()),
            csv_escape(entry.agent_id().unwrap_or("")),
            entry.prev_hash(),
            entry.hash(),
        )?;
    }
    Ok(())
}

/// Escape a CSV field: wrap in quotes if it contains commas, quotes, or newlines.
fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EventSeverity;

    fn sample_entries() -> Vec<AuditEntry> {
        let e1 = AuditEntry::new(
            EventSeverity::Info,
            "daimon",
            "agent.start",
            serde_json::json!({"agent": "a1"}),
            "",
        )
        .with_agent("agent-01");
        let e2 = AuditEntry::new(
            EventSeverity::Security,
            "aegis",
            "alert",
            serde_json::json!({"ip": "10.0.0.1"}),
            e1.hash(),
        );
        vec![e1, e2]
    }

    #[test]
    fn jsonl_export() {
        let entries = sample_entries();
        let mut buf = Vec::new();
        to_jsonl(&entries, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        let lines: Vec<&str> = output.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);

        // Each line should be valid JSON that deserializes back
        for (i, line) in lines.iter().enumerate() {
            let parsed: AuditEntry = serde_json::from_str(line).unwrap();
            assert_eq!(parsed.hash(), entries[i].hash());
            assert!(parsed.verify());
        }
    }

    #[test]
    fn csv_export() {
        let entries = sample_entries();
        let mut buf = Vec::new();
        to_csv(&entries, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        let lines: Vec<&str> = output.trim().split('\n').collect();
        assert_eq!(lines.len(), 3); // header + 2 entries
        assert!(lines[0].starts_with("id,timestamp,severity,"));
        assert!(lines[1].contains("daimon"));
        assert!(lines[2].contains("aegis"));
    }

    #[test]
    fn csv_escapes_special_chars() {
        let entry = AuditEntry::new(
            EventSeverity::Info,
            "source,with,commas",
            "action \"quoted\"",
            serde_json::json!({}),
            "",
        );
        let mut buf = Vec::new();
        to_csv(&[entry], &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        // Source should be quoted
        assert!(output.contains("\"source,with,commas\""));
        // Quotes in action should be doubled
        assert!(output.contains("\"action \"\"quoted\"\"\""));
    }

    #[test]
    fn csv_escapes_agent_id() {
        let entry = AuditEntry::new(EventSeverity::Info, "src", "act", serde_json::json!({}), "")
            .with_agent("agent,with,commas");
        let mut buf = Vec::new();
        to_csv(&[entry], &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("\"agent,with,commas\""));
    }

    #[test]
    fn jsonl_roundtrip_verify() {
        let entries = sample_entries();
        let mut buf = Vec::new();
        to_jsonl(&entries, &mut buf).unwrap();

        // Re-import
        let output = String::from_utf8(buf).unwrap();
        let reimported: Vec<AuditEntry> = output
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        // Verify chain integrity is preserved through export/import cycle
        assert!(crate::verify_chain(&reimported).is_ok());
    }

    #[test]
    fn csv_details_with_newlines() {
        let entry = AuditEntry::new(
            EventSeverity::Info,
            "src",
            "act",
            serde_json::json!({"multi": "line\nvalue"}),
            "",
        );
        let mut buf = Vec::new();
        to_csv(&[entry], &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        // The JSON details contain a newline, so the field should be quoted
        assert!(output.contains("\"{")); // details field is quoted
    }

    #[test]
    fn jsonl_empty() {
        let mut buf = Vec::new();
        to_jsonl(&[], &mut buf).unwrap();
        assert!(buf.is_empty());
    }

    #[test]
    fn csv_empty() {
        let mut buf = Vec::new();
        to_csv(&[], &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        // Should have just the header
        assert_eq!(output.trim().lines().count(), 1);
    }
}
