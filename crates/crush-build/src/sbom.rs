use std::path::Path;
use std::collections::HashMap;
use crush_types::{Result, CrushError};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CycloneDxBom {
    #[serde(rename = "bomFormat")]
    bom_format: String,
    #[serde(rename = "specVersion")]
    spec_version: String,
    version: i32,
    metadata: Metadata,
    components: Vec<Component>,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Serialize)]
struct Metadata {
    timestamp: String,
    tools: Vec<Tool>,
    component: Component,
}

#[derive(Debug, Serialize)]
struct Tool {
    vendor: String,
    name: String,
    version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Component {
    #[serde(rename = "type")]
    comp_type: String,
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    purl: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    licences: Option<Vec<HashMap<String, String>>>,
}

#[derive(Debug, Serialize)]
struct Dependency {
    #[serde(rename = "ref")]
    dep_ref: String,
    depends_on: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SpdxDocument {
    #[serde(rename = "spdxVersion")]
    spdx_version: String,
    #[serde(rename = "dataLicense")]
    data_license: String,
    name: String,
    #[serde(rename = "SPDXID")]
    spdx_id: String,
    #[serde(rename = "creationInfo")]
    creation_info: SpdxCreationInfo,
    packages: Vec<SpdxPackage>,
    relationships: Vec<SpdxRelationship>,
}

#[derive(Debug, Serialize)]
struct SpdxCreationInfo {
    creators: Vec<String>,
    created: String,
}

#[derive(Debug, Serialize)]
pub struct SpdxPackage {
    name: String,
    version_info: String,
    #[serde(rename = "SPDXID")]
    spdx_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    license_concluded: Option<String>,
    #[serde(rename = "downloadLocation")]
    download_location: String,
}

#[derive(Debug, Serialize)]
struct SpdxRelationship {
    #[serde(rename = "spdxElementId")]
    spdx_element_id: String,
    #[serde(rename = "relationshipType")]
    relationship_type: String,
    #[serde(rename = "relatedSpdxElement")]
    related_spdx_element: String,
}

pub struct SbomGenerator;

impl SbomGenerator {
    pub fn generate_cyclonedx(project_name: &str, version: &str, deps: &[Component]) -> Result<String> {
        let bom = CycloneDxBom {
            bom_format: "CycloneDX".to_string(),
            spec_version: "1.4".to_string(),
            version: 1,
            metadata: Metadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                tools: vec![Tool {
                    vendor: "Crush Runtime".to_string(),
                    name: "crush-build".to_string(),
                    version: "0.1.0".to_string(),
                }],
                component: Component {
                    comp_type: "application".to_string(),
                    name: project_name.to_string(),
                    version: version.to_string(),
                    purl: Some(format!("pkg:crush/{}@{}", project_name, version)),
                    licences: None,
                },
            },
            components: deps.to_vec(),
            dependencies: vec![Dependency {
                dep_ref: format!("pkg:crush/{}@{}", project_name, version),
                depends_on: deps.iter().filter_map(|c| c.purl.clone()).collect(),
            }],
        };

        serde_json::to_string_pretty(&bom)
            .map_err(|e| CrushError::ImageError(format!("SBOM generation error: {}", e)))
    }

    pub fn generate_spdx(project_name: &str, version: &str, deps: &[SpdxPackage]) -> Result<String> {
        let doc = SpdxDocument {
            spdx_version: "SPDX-2.3".to_string(),
            data_license: "CC0-1.0".to_string(),
            name: format!("{}-{}", project_name, version),
            spdx_id: "SPDXRef-DOCUMENT".to_string(),
            creation_info: SpdxCreationInfo {
                creators: vec![
                    "Tool: Crush Build".to_string(),
                    "Organization: Crush Runtime".to_string(),
                ],
                created: chrono::Utc::now().to_rfc3339(),
            },
            packages: deps.to_vec(),
            relationships: vec![SpdxRelationship {
                spdx_element_id: "SPDXRef-DOCUMENT".to_string(),
                relationship_type: "DESCRIBES".to_string(),
                related_spdx_element: "SPDXRef-PACKAGE-ROOT".to_string(),
            }],
        };

        serde_json::to_string_pretty(&doc)
            .map_err(|e| CrushError::ImageError(format!("SPDX generation error: {}", e)))
    }

    pub fn detect_dependencies(root: &Path) -> Result<Vec<Component>> {
        let mut components = Vec::new();

        let lockfiles: Vec<(&str, &str, &str)> = vec![
            ("package-lock.json", "npm", "pkg:npm/"),
            ("yarn.lock", "yarn", "pkg:npm/"),
            ("Cargo.lock", "cargo", "pkg:cargo/"),
            ("go.sum", "go", "pkg:golang/"),
            ("Gemfile.lock", "bundler", "pkg:gem/"),
            ("poetry.lock", "poetry", "pkg:pypi/"),
        ];

        for (filename, ecosystem, purl_prefix) in &lockfiles {
            let path = root.join(filename);
            if path.exists() {
                if *ecosystem == "cargo" {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        for line in content.lines() {
                            if let Some(pkg) = line.strip_prefix("name = ") {
                                components.push(Component {
                                    comp_type: "library".to_string(),
                                    name: pkg.trim_matches('"').to_string(),
                                    version: "unknown".to_string(),
                                    purl: Some(format!("{}{}", purl_prefix, pkg.trim_matches('"'))),
                                    licences: None,
                                });
                            }
                        }
                    }
                } else if *ecosystem == "go" {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        for line in content.lines() {
                            if let Some(pkg) = line.split_whitespace().next() {
                                if pkg.contains('/') {
                                    components.push(Component {
                                        comp_type: "library".to_string(),
                                        name: pkg.to_string(),
                                        version: "unknown".to_string(),
                                        purl: Some(format!("{}{}", purl_prefix, pkg)),
                                        licences: None,
                                    });
                                }
                            }
                        }
                    }
                }
                break;
            }
        }

        Ok(components)
    }
}
