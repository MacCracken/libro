//! Kernel audit subsystem integration.
//!
//! Bridges the Linux kernel audit subsystem (via agnosys) with libro's
//! cryptographic audit chain. Reads raw kernel audit events and converts
//! them into [`AuditEntry`] records for tamper-proof storage.
//!
//! Requires the `kernel-audit` feature. Without it, all functions return errors.

use crate::entry::AuditEntry;
#[cfg(feature = "kernel-audit")]
use crate::entry::EventSeverity;
use crate::error::LibroError;

/// Ingest raw kernel audit events into audit entries.
///
/// Reads events from the AGNOS `/proc/agnos/audit` interface and converts
/// each into an [`AuditEntry`] with proper severity mapping.
///
/// Requires the `kernel-audit` feature and a running AGNOS kernel.
/// Default AGNOS audit proc path.
#[cfg(feature = "kernel-audit")]
const AGNOS_AUDIT_PATH: &str = "/proc/agnos/audit";

pub fn read_kernel_events() -> crate::Result<Vec<AuditEntry>> {
    #[cfg(feature = "kernel-audit")]
    {
        let raw_events = agnosys::audit::read_agnos_audit_events(AGNOS_AUDIT_PATH)
            .map_err(|e| LibroError::Store(format!("kernel audit read failed: {e}")))?;

        let entries: Vec<AuditEntry> = raw_events
            .iter()
            .map(|raw| {
                let severity = classify_kernel_result(raw.result);
                AuditEntry::new(
                    severity,
                    "kernel",
                    &raw.action_type,
                    serde_json::json!({
                        "sequence": raw.sequence,
                        "timestamp_ns": raw.timestamp_ns,
                        "result": raw.result,
                        "payload": raw.payload,
                        "kernel_hash": raw.hash,
                    }),
                    &raw.prev_hash,
                )
            })
            .collect();

        Ok(entries)
    }
    #[cfg(not(feature = "kernel-audit"))]
    {
        Err(LibroError::Store(
            "kernel audit requires the 'kernel-audit' feature".into(),
        ))
    }
}

/// Log a syscall event to the AGNOS kernel audit interface.
///
/// Requires the `kernel-audit` feature.
pub fn log_syscall(action: &str, data: &str, result: i32) -> crate::Result<()> {
    #[cfg(feature = "kernel-audit")]
    {
        agnosys::audit::agnos_audit_log_syscall(action, data, result)
            .map_err(|e| LibroError::Store(format!("kernel audit log failed: {e}")))
    }
    #[cfg(not(feature = "kernel-audit"))]
    {
        let _ = (action, data, result);
        Err(LibroError::Store(
            "kernel audit requires the 'kernel-audit' feature".into(),
        ))
    }
}

/// Check if the kernel audit subsystem is available.
#[must_use]
pub fn kernel_audit_available() -> bool {
    #[cfg(feature = "kernel-audit")]
    {
        std::path::Path::new(AGNOS_AUDIT_PATH).exists()
    }
    #[cfg(not(feature = "kernel-audit"))]
    {
        false
    }
}

/// Map a kernel audit result code to an [`EventSeverity`].
#[cfg(feature = "kernel-audit")]
fn classify_kernel_result(result: i32) -> EventSeverity {
    match result {
        0 => EventSeverity::Info,
        r if r > 0 => EventSeverity::Warning,
        -1 => EventSeverity::Error,
        -13 => EventSeverity::Security, // EACCES
        _ => EventSeverity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "kernel-audit")]
    mod classify_tests {
        use super::*;

        #[test]
        fn classify_result_info() {
            assert_eq!(classify_kernel_result(0), EventSeverity::Info);
        }

        #[test]
        fn classify_result_warning() {
            assert_eq!(classify_kernel_result(1), EventSeverity::Warning);
            assert_eq!(classify_kernel_result(42), EventSeverity::Warning);
        }

        #[test]
        fn classify_result_error() {
            assert_eq!(classify_kernel_result(-1), EventSeverity::Error);
            assert_eq!(classify_kernel_result(-99), EventSeverity::Error);
        }

        #[test]
        fn classify_result_security() {
            assert_eq!(classify_kernel_result(-13), EventSeverity::Security);
        }
    }

    #[test]
    fn read_kernel_events_without_kernel() {
        let result = read_kernel_events();
        // Without feature: error. With feature but no AGNOS kernel: Ok(empty).
        if let Ok(entries) = result {
            assert!(entries.is_empty());
        }
    }

    #[test]
    fn log_syscall_without_feature_or_kernel() {
        let result = log_syscall("open", "test", 0);
        assert!(result.is_err());
    }

    #[test]
    fn kernel_audit_available_check() {
        // Just verify no panic — will be false without feature or kernel
        let _ = kernel_audit_available();
    }
}
