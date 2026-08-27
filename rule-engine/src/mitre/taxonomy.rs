use std::{collections::HashMap, path::Path};

use serde::Deserialize;

use crate::{
    error::AppError,
    models::{Alert, AlertSeverity},
};

#[derive(Debug, Clone, Deserialize)]
pub struct MitreTechnique {
    pub technique_id: String,
    pub technique_name: String,
    pub tactic: String,
    pub tactic_id: String,
    pub default_severity: AlertSeverity,
    pub base_threat_score: f32,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct MitreTaxonomy {
    techniques: HashMap<String, MitreTechnique>,
}

pub trait MitreCatalog: Send + Sync {
    fn resolve_technique(&self, technique_id: &str) -> Option<&MitreTechnique>;
    fn enrich_alert(&self, technique_id: &str, alert: &mut Alert);
}

impl MitreTaxonomy {
    pub fn load_from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let techniques: HashMap<String, MitreTechnique> = serde_json::from_str(json_str)?;
        Ok(Self { techniques })
    }

    pub fn load_from_file(path: &Path) -> Result<Self, AppError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AppError::MitreTaxonomyLoad(format!("Failed to read {}: {e}", path.display())))?;
        Self::load_from_json(&content).map_err(|e| AppError::MitreTaxonomyLoad(format!("Failed to parse JSON: {e}")))
    }

    pub fn len(&self) -> usize {
        self.techniques.len()
    }

    pub fn is_empty(&self) -> bool {
        self.techniques.is_empty()
    }
}

impl MitreCatalog for MitreTaxonomy {
    #[inline]
    fn resolve_technique(&self, technique_id: &str) -> Option<&MitreTechnique> {
        self.techniques.get(technique_id)
    }

    fn enrich_alert(&self, technique_id: &str, alert: &mut Alert) {
        if let Some(tech) = self.resolve_technique(technique_id) {
            alert.mitre_tactic = Some(tech.tactic.clone());
            if alert.description.is_empty() {
                alert.description = tech.description.clone();
            }
            if alert.threat_score == 0.0 {
                alert.threat_score = tech.base_threat_score;
            }
        }
    }
}
