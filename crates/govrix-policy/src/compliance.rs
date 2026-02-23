use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Compliance Report Templates (SOC 2, EU AI Act, HIPAA, NIST)
// ---------------------------------------------------------------------------

/// Supported compliance frameworks for report generation.
#[derive(Debug, Clone, Serialize)]
pub enum Framework {
    Soc2,
    EuAiAct,
    Hipaa,
    Nist,
}

/// A single compliance control check result.
#[derive(Debug, Clone, Serialize)]
pub struct ControlCheck {
    pub framework: Framework,
    pub control_id: String,
    pub control_name: String,
    pub status: ControlStatus,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum ControlStatus {
    Pass,
    Fail,
    Warn,
    NotApplicable,
}

/// Full compliance report for a framework.
#[derive(Debug, Clone, Serialize)]
pub struct ComplianceReport {
    pub framework: Framework,
    pub generated_at: String,
    pub controls: Vec<ControlCheck>,
    pub pass_count: usize,
    pub fail_count: usize,
    pub warn_count: usize,
}

impl ComplianceReport {
    /// Generate a SOC 2 compliance report based on platform state.
    pub fn soc2(
        audit_trail_enabled: bool,
        policy_enabled: bool,
        pii_masking_enabled: bool,
        mtls_enabled: bool,
        _budget_tracking_enabled: bool,
    ) -> Self {
        let controls = vec![
            ControlCheck {
                framework: Framework::Soc2,
                control_id: "CC6.1".to_string(),
                control_name: "Logical Access Security".to_string(),
                status: if mtls_enabled { ControlStatus::Pass } else { ControlStatus::Warn },
                evidence: if mtls_enabled {
                    "mTLS enabled — agent identity verified via X.509 certificates".to_string()
                } else {
                    "mTLS not enabled — agents authenticate via bearer tokens only".to_string()
                },
            },
            ControlCheck {
                framework: Framework::Soc2,
                control_id: "CC7.2".to_string(),
                control_name: "System Monitoring".to_string(),
                status: if audit_trail_enabled { ControlStatus::Pass } else { ControlStatus::Fail },
                evidence: if audit_trail_enabled {
                    "Audit trail enabled — all agent API calls captured with tamper-evident hash chain".to_string()
                } else {
                    "Audit trail disabled — no event capture".to_string()
                },
            },
            ControlCheck {
                framework: Framework::Soc2,
                control_id: "CC6.7".to_string(),
                control_name: "Data Classification and Protection".to_string(),
                status: if pii_masking_enabled { ControlStatus::Pass } else { ControlStatus::Warn },
                evidence: if pii_masking_enabled {
                    "PII masking enabled — SSN, email, phone, credit card, IP detected and flagged".to_string()
                } else {
                    "PII masking disabled — sensitive data may pass through undetected".to_string()
                },
            },
            ControlCheck {
                framework: Framework::Soc2,
                control_id: "CC8.1".to_string(),
                control_name: "Change Management".to_string(),
                status: if policy_enabled { ControlStatus::Pass } else { ControlStatus::Warn },
                evidence: if policy_enabled {
                    "Policy engine enabled — YAML rules enforce governance with hot-reload".to_string()
                } else {
                    "Policy engine disabled — no runtime governance enforcement".to_string()
                },
            },
        ];

        let pass_count = controls.iter().filter(|c| matches!(c.status, ControlStatus::Pass)).count();
        let fail_count = controls.iter().filter(|c| matches!(c.status, ControlStatus::Fail)).count();
        let warn_count = controls.iter().filter(|c| matches!(c.status, ControlStatus::Warn)).count();

        Self {
            framework: Framework::Soc2,
            generated_at: chrono::Utc::now().to_rfc3339(),
            controls,
            pass_count,
            fail_count,
            warn_count,
        }
    }

    /// Generate an EU AI Act compliance report.
    pub fn eu_ai_act(
        audit_trail_enabled: bool,
        policy_enabled: bool,
        pii_masking_enabled: bool,
        budget_tracking_enabled: bool,
    ) -> Self {
        let controls = vec![
            ControlCheck {
                framework: Framework::EuAiAct,
                control_id: "Art.12".to_string(),
                control_name: "Record-Keeping".to_string(),
                status: if audit_trail_enabled { ControlStatus::Pass } else { ControlStatus::Fail },
                evidence: if audit_trail_enabled {
                    "All agent interactions logged with tamper-evident hash chain".to_string()
                } else {
                    "No audit trail — cannot demonstrate record-keeping".to_string()
                },
            },
            ControlCheck {
                framework: Framework::EuAiAct,
                control_id: "Art.14".to_string(),
                control_name: "Human Oversight".to_string(),
                status: if policy_enabled { ControlStatus::Pass } else { ControlStatus::Fail },
                evidence: if policy_enabled {
                    "Policy engine provides real-time block/allow/alert controls for human oversight".to_string()
                } else {
                    "No policy enforcement — human oversight not implemented".to_string()
                },
            },
            ControlCheck {
                framework: Framework::EuAiAct,
                control_id: "Art.9".to_string(),
                control_name: "Data Governance".to_string(),
                status: if pii_masking_enabled { ControlStatus::Pass } else { ControlStatus::Warn },
                evidence: if pii_masking_enabled {
                    "PII detection and masking active — data governance controls in place".to_string()
                } else {
                    "PII masking disabled — limited data governance".to_string()
                },
            },
            ControlCheck {
                framework: Framework::EuAiAct,
                control_id: "Art.15".to_string(),
                control_name: "Accuracy and Robustness".to_string(),
                status: if budget_tracking_enabled { ControlStatus::Pass } else { ControlStatus::Warn },
                evidence: if budget_tracking_enabled {
                    "Budget tracking prevents runaway agents — robustness control active".to_string()
                } else {
                    "No budget limits — agents may consume unlimited resources".to_string()
                },
            },
        ];

        let pass_count = controls.iter().filter(|c| matches!(c.status, ControlStatus::Pass)).count();
        let fail_count = controls.iter().filter(|c| matches!(c.status, ControlStatus::Fail)).count();
        let warn_count = controls.iter().filter(|c| matches!(c.status, ControlStatus::Warn)).count();

        Self {
            framework: Framework::EuAiAct,
            generated_at: chrono::Utc::now().to_rfc3339(),
            controls,
            pass_count,
            fail_count,
            warn_count,
        }
    }

    /// Render the report as a markdown string.
    pub fn to_markdown(&self) -> String {
        let mut md = format!("# {:?} Compliance Report\n\n", self.framework);
        md.push_str(&format!("Generated: {}\n\n", self.generated_at));
        md.push_str(&format!(
            "| Pass | Fail | Warn |\n|------|------|------|\n| {} | {} | {} |\n\n",
            self.pass_count, self.fail_count, self.warn_count
        ));
        md.push_str("| Control | Name | Status | Evidence |\n|---------|------|--------|----------|\n");
        for c in &self.controls {
            md.push_str(&format!(
                "| {} | {} | {:?} | {} |\n",
                c.control_id, c.control_name, c.status, c.evidence
            ));
        }
        md
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceFramework {
    Soc2,
    EuAiAct,
    Hipaa,
}

/// A compliance requirement that must be met.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequirement {
    pub id: String,
    pub framework: ComplianceFramework,
    pub name: String,
    pub description: String,
    pub check: ComplianceCheck,
}

/// What the system checks for compliance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceCheck {
    /// PII masking must be enabled
    PiiMaskingEnabled,
    /// Event retention must be at least N days
    MinRetentionDays(u32),
    /// Policy engine must be enabled
    PolicyEngineEnabled,
    /// Audit logging must be enabled (events stored)
    AuditLoggingEnabled,
    /// Max agents must not exceed N
    MaxAgentsLimit(u32),
}

/// Result of a compliance check.
#[derive(Debug, Clone)]
pub struct ComplianceResult {
    pub requirement: ComplianceRequirement,
    pub passed: bool,
    pub details: String,
}

/// System state for compliance evaluation.
pub struct SystemState {
    pub pii_masking_enabled: bool,
    pub policy_engine_enabled: bool,
    pub audit_logging_enabled: bool,
    pub retention_days: u32,
    pub active_agents: u32,
}

impl ComplianceFramework {
    /// Get all requirements for this framework.
    pub fn requirements(&self) -> Vec<ComplianceRequirement> {
        match self {
            ComplianceFramework::Soc2 => vec![
                ComplianceRequirement {
                    id: "soc2-001".to_string(),
                    framework: ComplianceFramework::Soc2,
                    name: "Audit Logging".to_string(),
                    description: "All agent events must be logged for audit purposes.".to_string(),
                    check: ComplianceCheck::AuditLoggingEnabled,
                },
                ComplianceRequirement {
                    id: "soc2-002".to_string(),
                    framework: ComplianceFramework::Soc2,
                    name: "Policy Engine".to_string(),
                    description: "Policy engine must be active to enforce access controls."
                        .to_string(),
                    check: ComplianceCheck::PolicyEngineEnabled,
                },
                ComplianceRequirement {
                    id: "soc2-003".to_string(),
                    framework: ComplianceFramework::Soc2,
                    name: "Minimum Retention".to_string(),
                    description: "Events must be retained for at least 90 days.".to_string(),
                    check: ComplianceCheck::MinRetentionDays(90),
                },
            ],
            ComplianceFramework::EuAiAct => vec![
                ComplianceRequirement {
                    id: "euai-001".to_string(),
                    framework: ComplianceFramework::EuAiAct,
                    name: "PII Masking".to_string(),
                    description: "Personal data in AI inputs and outputs must be masked."
                        .to_string(),
                    check: ComplianceCheck::PiiMaskingEnabled,
                },
                ComplianceRequirement {
                    id: "euai-002".to_string(),
                    framework: ComplianceFramework::EuAiAct,
                    name: "Audit Logging".to_string(),
                    description: "All AI system interactions must be logged for traceability."
                        .to_string(),
                    check: ComplianceCheck::AuditLoggingEnabled,
                },
                ComplianceRequirement {
                    id: "euai-003".to_string(),
                    framework: ComplianceFramework::EuAiAct,
                    name: "Policy Engine".to_string(),
                    description: "Automated policy controls must be enforced on AI agent actions."
                        .to_string(),
                    check: ComplianceCheck::PolicyEngineEnabled,
                },
                ComplianceRequirement {
                    id: "euai-004".to_string(),
                    framework: ComplianceFramework::EuAiAct,
                    name: "Minimum Retention".to_string(),
                    description: "Events must be retained for at least 180 days per EU AI Act."
                        .to_string(),
                    check: ComplianceCheck::MinRetentionDays(180),
                },
            ],
            ComplianceFramework::Hipaa => vec![
                ComplianceRequirement {
                    id: "hipaa-001".to_string(),
                    framework: ComplianceFramework::Hipaa,
                    name: "PII Masking".to_string(),
                    description: "Protected health information (PHI) must be masked at all times."
                        .to_string(),
                    check: ComplianceCheck::PiiMaskingEnabled,
                },
                ComplianceRequirement {
                    id: "hipaa-002".to_string(),
                    framework: ComplianceFramework::Hipaa,
                    name: "Audit Logging".to_string(),
                    description: "All access to PHI must be logged per HIPAA audit controls."
                        .to_string(),
                    check: ComplianceCheck::AuditLoggingEnabled,
                },
                ComplianceRequirement {
                    id: "hipaa-003".to_string(),
                    framework: ComplianceFramework::Hipaa,
                    name: "Policy Engine".to_string(),
                    description: "Policy engine must enforce minimum necessary access to PHI."
                        .to_string(),
                    check: ComplianceCheck::PolicyEngineEnabled,
                },
                ComplianceRequirement {
                    id: "hipaa-004".to_string(),
                    framework: ComplianceFramework::Hipaa,
                    name: "Minimum Retention".to_string(),
                    description: "Audit logs must be retained for at least 365 days per HIPAA."
                        .to_string(),
                    check: ComplianceCheck::MinRetentionDays(365),
                },
                ComplianceRequirement {
                    id: "hipaa-005".to_string(),
                    framework: ComplianceFramework::Hipaa,
                    name: "Agent Limit".to_string(),
                    description:
                        "No more than 50 active agents allowed to enforce data minimization."
                            .to_string(),
                    check: ComplianceCheck::MaxAgentsLimit(50),
                },
            ],
        }
    }
}

fn evaluate_check(check: &ComplianceCheck, state: &SystemState) -> (bool, String) {
    match check {
        ComplianceCheck::PiiMaskingEnabled => {
            if state.pii_masking_enabled {
                (true, "PII masking is enabled.".to_string())
            } else {
                (false, "PII masking is disabled.".to_string())
            }
        }
        ComplianceCheck::PolicyEngineEnabled => {
            if state.policy_engine_enabled {
                (true, "Policy engine is enabled.".to_string())
            } else {
                (false, "Policy engine is disabled.".to_string())
            }
        }
        ComplianceCheck::AuditLoggingEnabled => {
            if state.audit_logging_enabled {
                (true, "Audit logging is enabled.".to_string())
            } else {
                (false, "Audit logging is disabled.".to_string())
            }
        }
        ComplianceCheck::MinRetentionDays(required) => {
            if state.retention_days >= *required {
                (
                    true,
                    format!(
                        "Retention of {} days meets the minimum of {} days.",
                        state.retention_days, required
                    ),
                )
            } else {
                (
                    false,
                    format!(
                        "Retention of {} days is below the required minimum of {} days.",
                        state.retention_days, required
                    ),
                )
            }
        }
        ComplianceCheck::MaxAgentsLimit(limit) => {
            if state.active_agents <= *limit {
                (
                    true,
                    format!(
                        "{} active agents is within the limit of {}.",
                        state.active_agents, limit
                    ),
                )
            } else {
                (
                    false,
                    format!(
                        "{} active agents exceeds the limit of {}.",
                        state.active_agents, limit
                    ),
                )
            }
        }
    }
}

/// Check system compliance against a framework.
pub fn check_compliance(
    framework: &ComplianceFramework,
    state: &SystemState,
) -> Vec<ComplianceResult> {
    framework
        .requirements()
        .into_iter()
        .map(|req| {
            let (passed, details) = evaluate_check(&req.check, state);
            ComplianceResult {
                requirement: req,
                passed,
                details,
            }
        })
        .collect()
}

/// Check if ALL requirements pass.
pub fn is_compliant(framework: &ComplianceFramework, state: &SystemState) -> bool {
    check_compliance(framework, state).iter().all(|r| r.passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fully_compliant_state() -> SystemState {
        SystemState {
            pii_masking_enabled: true,
            policy_engine_enabled: true,
            audit_logging_enabled: true,
            retention_days: 365,
            active_agents: 10,
        }
    }

    #[test]
    fn soc2_passes_with_all_requirements_met() {
        let state = SystemState {
            pii_masking_enabled: false, // not required by SOC2
            policy_engine_enabled: true,
            audit_logging_enabled: true,
            retention_days: 90,
            active_agents: 100,
        };
        assert!(is_compliant(&ComplianceFramework::Soc2, &state));
    }

    #[test]
    fn soc2_fails_when_retention_too_low() {
        let state = SystemState {
            pii_masking_enabled: false,
            policy_engine_enabled: true,
            audit_logging_enabled: true,
            retention_days: 89,
            active_agents: 5,
        };
        assert!(!is_compliant(&ComplianceFramework::Soc2, &state));

        let results = check_compliance(&ComplianceFramework::Soc2, &state);
        let retention_result = results
            .iter()
            .find(|r| matches!(r.requirement.check, ComplianceCheck::MinRetentionDays(_)))
            .expect("retention check should exist");
        assert!(!retention_result.passed);
    }

    #[test]
    fn eu_ai_act_requires_pii_masking() {
        let state = SystemState {
            pii_masking_enabled: false,
            policy_engine_enabled: true,
            audit_logging_enabled: true,
            retention_days: 180,
            active_agents: 5,
        };
        assert!(!is_compliant(&ComplianceFramework::EuAiAct, &state));

        let results = check_compliance(&ComplianceFramework::EuAiAct, &state);
        let pii_result = results
            .iter()
            .find(|r| matches!(r.requirement.check, ComplianceCheck::PiiMaskingEnabled))
            .expect("PII masking check should exist");
        assert!(!pii_result.passed);
    }

    #[test]
    fn hipaa_requires_all_checks_including_max_agents() {
        // Passes everything except agent limit
        let state = SystemState {
            pii_masking_enabled: true,
            policy_engine_enabled: true,
            audit_logging_enabled: true,
            retention_days: 365,
            active_agents: 51,
        };
        assert!(!is_compliant(&ComplianceFramework::Hipaa, &state));

        let results = check_compliance(&ComplianceFramework::Hipaa, &state);
        let agents_result = results
            .iter()
            .find(|r| matches!(r.requirement.check, ComplianceCheck::MaxAgentsLimit(_)))
            .expect("max agents check should exist");
        assert!(!agents_result.passed);

        // Verify all other HIPAA checks pass
        let other_results: Vec<_> = results
            .iter()
            .filter(|r| !matches!(r.requirement.check, ComplianceCheck::MaxAgentsLimit(_)))
            .collect();
        assert!(other_results.iter().all(|r| r.passed));
    }

    #[test]
    fn fully_compliant_system_passes_all_frameworks() {
        let state = fully_compliant_state();
        assert!(is_compliant(&ComplianceFramework::Soc2, &state));
        assert!(is_compliant(&ComplianceFramework::EuAiAct, &state));
        assert!(is_compliant(&ComplianceFramework::Hipaa, &state));
    }

    #[test]
    fn non_compliant_system_fails_appropriately() {
        let state = SystemState {
            pii_masking_enabled: false,
            policy_engine_enabled: false,
            audit_logging_enabled: false,
            retention_days: 0,
            active_agents: 200,
        };
        assert!(!is_compliant(&ComplianceFramework::Soc2, &state));
        assert!(!is_compliant(&ComplianceFramework::EuAiAct, &state));
        assert!(!is_compliant(&ComplianceFramework::Hipaa, &state));

        // All HIPAA results should fail
        let hipaa_results = check_compliance(&ComplianceFramework::Hipaa, &state);
        assert!(hipaa_results.iter().all(|r| !r.passed));
    }

    #[test]
    fn soc2_does_not_require_pii_masking() {
        // SOC2 should pass even without PII masking as long as other checks pass
        let state = SystemState {
            pii_masking_enabled: false,
            policy_engine_enabled: true,
            audit_logging_enabled: true,
            retention_days: 365,
            active_agents: 200,
        };
        assert!(is_compliant(&ComplianceFramework::Soc2, &state));
    }

    #[test]
    fn requirement_count_per_framework() {
        assert_eq!(ComplianceFramework::Soc2.requirements().len(), 3);
        assert_eq!(ComplianceFramework::EuAiAct.requirements().len(), 4);
        assert_eq!(ComplianceFramework::Hipaa.requirements().len(), 5);
    }

    // -------------------------------------------------------------------
    // Compliance Report Template Tests
    // -------------------------------------------------------------------

    #[test]
    fn soc2_report_all_enabled() {
        let report = ComplianceReport::soc2(true, true, true, true, true);
        assert_eq!(report.controls.len(), 4);
        assert_eq!(report.pass_count, 4);
        assert_eq!(report.fail_count, 0);
        assert_eq!(report.warn_count, 0);
        assert!(matches!(report.framework, Framework::Soc2));
        assert!(!report.generated_at.is_empty());
    }

    #[test]
    fn soc2_report_nothing_enabled() {
        let report = ComplianceReport::soc2(false, false, false, false, false);
        assert_eq!(report.controls.len(), 4);
        assert_eq!(report.pass_count, 0);
        assert_eq!(report.fail_count, 1); // CC7.2 audit trail
        assert_eq!(report.warn_count, 3); // CC6.1, CC6.7, CC8.1
    }

    #[test]
    fn soc2_report_partial() {
        let report = ComplianceReport::soc2(true, false, false, true, false);
        assert_eq!(report.pass_count, 2); // CC7.2 (audit) + CC6.1 (mtls)
        assert_eq!(report.warn_count, 2); // CC6.7 (pii) + CC8.1 (policy)
        assert_eq!(report.fail_count, 0);
    }

    #[test]
    fn eu_ai_act_report_all_enabled() {
        let report = ComplianceReport::eu_ai_act(true, true, true, true);
        assert_eq!(report.controls.len(), 4);
        assert_eq!(report.pass_count, 4);
        assert_eq!(report.fail_count, 0);
        assert_eq!(report.warn_count, 0);
        assert!(matches!(report.framework, Framework::EuAiAct));
    }

    #[test]
    fn eu_ai_act_report_nothing_enabled() {
        let report = ComplianceReport::eu_ai_act(false, false, false, false);
        assert_eq!(report.controls.len(), 4);
        assert_eq!(report.pass_count, 0);
        assert_eq!(report.fail_count, 2); // Art.12 + Art.14
        assert_eq!(report.warn_count, 2); // Art.9 + Art.15
    }

    #[test]
    fn eu_ai_act_report_partial() {
        let report = ComplianceReport::eu_ai_act(true, false, true, false);
        assert_eq!(report.pass_count, 2); // Art.12 (audit) + Art.9 (pii)
        assert_eq!(report.fail_count, 1); // Art.14 (policy)
        assert_eq!(report.warn_count, 1); // Art.15 (budget)
    }

    #[test]
    fn soc2_report_to_markdown_contains_controls() {
        let report = ComplianceReport::soc2(true, true, false, false, false);
        let md = report.to_markdown();
        assert!(md.contains("# Soc2 Compliance Report"));
        assert!(md.contains("CC6.1"));
        assert!(md.contains("CC7.2"));
        assert!(md.contains("CC6.7"));
        assert!(md.contains("CC8.1"));
        assert!(md.contains("| Pass | Fail | Warn |"));
    }

    #[test]
    fn eu_ai_act_report_to_markdown_contains_controls() {
        let report = ComplianceReport::eu_ai_act(true, true, true, true);
        let md = report.to_markdown();
        assert!(md.contains("# EuAiAct Compliance Report"));
        assert!(md.contains("Art.12"));
        assert!(md.contains("Art.14"));
        assert!(md.contains("Art.9"));
        assert!(md.contains("Art.15"));
    }

    #[test]
    fn report_generated_at_is_valid_rfc3339() {
        let report = ComplianceReport::soc2(true, true, true, true, true);
        // Should parse as a valid RFC 3339 timestamp
        assert!(chrono::DateTime::parse_from_rfc3339(&report.generated_at).is_ok());
    }

    #[test]
    fn control_status_serializes_correctly() {
        let check = ControlCheck {
            framework: Framework::Soc2,
            control_id: "TEST".to_string(),
            control_name: "Test Control".to_string(),
            status: ControlStatus::Pass,
            evidence: "test".to_string(),
        };
        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("\"Pass\""));
    }
}
