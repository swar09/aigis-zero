use chrono::Utc;
use uuid::Uuid;

use crate::{
    mitre::{MitreCatalog, MitreTaxonomy},
    models::{Alert, AlertSeverity, DetectionSource, TelemetryEvent},
};

/// Builds an Alert from a YARA-X rule match and its metadata.
pub fn build_alert(event: &TelemetryEvent, matched_rule: &yara_x::Rule<'_, '_>, mitre: &MitreTaxonomy) -> Alert {
    let mut technique_id = None;
    let mut severity = AlertSeverity::Medium;
    let mut threat_score: f32 = 5.0;
    let mut description = format!("YARA rule '{}' matched", matched_rule.identifier());

    for (key, value) in matched_rule.metadata() {
        match key {
            "mitre_technique" => {
                if let yara_x::MetaValue::String(v) = value {
                    technique_id = Some(v.to_string());
                }
            }
            "severity" => {
                if let yara_x::MetaValue::String(v) = value {
                    severity = match v.to_lowercase().as_str() {
                        "critical" => AlertSeverity::Critical,
                        "high" => AlertSeverity::High,
                        "medium" => AlertSeverity::Medium,
                        _ => AlertSeverity::Low,
                    };
                }
            }
            "threat_score" => {
                if let yara_x::MetaValue::Integer(v) = value {
                    threat_score = v as f32;
                } else if let yara_x::MetaValue::Float(v) = value {
                    threat_score = v as f32;
                }
            }
            "description" => {
                if let yara_x::MetaValue::String(v) = value {
                    description = v.to_string();
                }
            }
            _ => {}
        }
    }

    let mut alert = Alert {
        alert_id: Uuid::new_v4(),
        node_id: event.node_id,
        hostname: event.hostname.clone(),
        severity,
        source: DetectionSource::YaraX,
        mitre_technique_id: technique_id.clone(),
        mitre_tactic: None,
        description,
        triggering_event_id: Some(event.id.clone()),
        threat_score,
        status: "open".to_string(),
        created_at: Utc::now(),
    };

    if let Some(ref tech_id) = technique_id {
        mitre.enrich_alert(tech_id, &mut alert);
    }

    alert
}
