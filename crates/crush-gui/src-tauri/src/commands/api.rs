use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::time::SystemTime;
use crush_apispec::{ApiModel, CapturedExample, SavedResponse, Header};

// --- Command Data Structures ---

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiSendRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiResponse {
    pub status: u16,
    pub headers: Vec<Header>,
    pub body: Option<String>,
    pub timing_ms: u64,
    pub size_bytes: u64,
}

// --- Persistence Helpers ---

fn get_api_dir(project_path: &str) -> PathBuf {
    Path::new(project_path).join(".crush").join("api")
}

fn save_docs(project_path: &str, model: &ApiModel) -> std::io::Result<()> {
    let api_dir = get_api_dir(project_path);
    fs::create_dir_all(&api_dir)?;

    // Save manifest.json (atomically)
    let manifest_path = api_dir.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(model)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    let tmp_manifest = manifest_path.with_extension("tmp");
    fs::write(&tmp_manifest, &manifest_json)?;
    fs::rename(&tmp_manifest, &manifest_path)?;

    // For each group, write group_<slug>.md
    for group in &model.groups {
        let slug = group.name.replace("/", "_").replace(" ", "_").to_lowercase();
        let md_path = api_dir.join(format!("group_{}.md", slug));
        
        let mut md_content = format!("# Group {}\n\n", group.name);
        for req in &group.requests {
            md_content.push_str(&format!("## {} {}\n", req.method, req.path));
            if !req.doc.summary.is_empty() {
                md_content.push_str(&format!("*Summary: {}*\n\n", req.doc.summary));
            }
            md_content.push_str(&format!("<!-- id: {} -->\n", req.id));
            md_content.push_str(&req.doc.description_md);
            md_content.push_str("\n\n---\n\n");
        }

        let tmp_md = md_path.with_extension("tmp");
        fs::write(&tmp_md, &md_content)?;
        fs::rename(&tmp_md, &md_path)?;
    }

    Ok(())
}

fn load_docs(project_path: &str) -> Result<Option<ApiModel>, String> {
    let api_dir = get_api_dir(project_path);
    let manifest_path = api_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Ok(None);
    }

    let manifest_str = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let mut model: ApiModel = serde_json::from_str(&manifest_str).map_err(|e| e.to_string())?;

    // For each group, load descriptions from group_<slug>.md
    for group in &mut model.groups {
        let slug = group.name.replace("/", "_").replace(" ", "_").to_lowercase();
        let md_path = api_dir.join(format!("group_{}.md", slug));
        if md_path.exists() {
            if let Ok(md_content) = fs::read_to_string(&md_path) {
                let descriptions = parse_markdown_descriptions(&md_content);
                for req in &mut group.requests {
                    if let Some(desc) = descriptions.get(&req.id) {
                        req.doc.description_md = desc.clone();
                    }
                }
            }
        }
    }

    Ok(Some(model))
}

fn parse_markdown_descriptions(content: &str) -> HashMap<String, String> {
    let mut descriptions = HashMap::new();
    let mut current_id = None;
    let mut current_lines = Vec::new();

    for line in content.lines() {
        if line.starts_with("<!-- id:") && line.ends_with("-->") {
            if let Some(id) = current_id.take() {
                descriptions.insert(id, current_lines.join("\n").trim().to_string());
                current_lines.clear();
            }
            let id = line["<!-- id:".len()..line.len() - "-->".len()].trim().to_string();
            current_id = Some(id);
        } else if current_id.is_some() {
            if line == "---" {
                if let Some(id) = current_id.take() {
                    descriptions.insert(id, current_lines.join("\n").trim().to_string());
                    current_lines.clear();
                }
            } else {
                current_lines.push(line);
            }
        }
    }

    if let Some(id) = current_id {
        descriptions.insert(id, current_lines.join("\n").trim().to_string());
    }

    descriptions
}

fn load_project_env(project_path: &str) -> HashMap<String, String> {
    let mut env = HashMap::new();
    let env_path = Path::new(project_path).join(".env");
    if env_path.exists() {
        if let Ok(content) = fs::read_to_string(&env_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once('=') {
                    let key = k.trim().to_string();
                    let value = v.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                    env.insert(key, value);
                }
            }
        }
    }
    env
}

fn resolve_vars(text: &str, env: &HashMap<String, String>) -> String {
    let mut resolved = text.to_string();
    for (k, v) in env {
        let placeholder = format!("{{{{{}}}}}", k);
        resolved = resolved.replace(&placeholder, v);
    }
    resolved
}

// --- Tauri Commands ---

#[tauri::command]
pub async fn api_load_spec(project_path: String) -> Result<Option<ApiModel>, String> {
    load_docs(&project_path)
}

#[tauri::command]
pub async fn api_import_spec(project_path: String, spec_content: String) -> Result<ApiModel, String> {
    let model = crush_apispec::parse_spec(spec_content.as_bytes())?;
    save_docs(&project_path, &model).map_err(|e| e.to_string())?;
    Ok(model)
}

#[tauri::command]
pub async fn api_scan_project(project_path: String) -> Result<Option<String>, String> {
    let root = Path::new(&project_path);
    let candidates = vec![
        root.join("openapi.json"),
        root.join("openapi.yaml"),
        root.join("openapi.yml"),
        root.join("swagger.json"),
        root.join("postman_collection.json"),
        root.join("docs").join("openapi.json"),
        root.join("docs").join("openapi.yaml"),
        root.join("docs").join("swagger.json"),
    ];

    for path in candidates {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                return Ok(Some(content));
            }
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn api_probe_live(base_url: String) -> Result<Option<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()
        .map_err(|e| e.to_string())?;

    let endpoints = vec![
        "/openapi.json",
        "/v3/api-docs",
        "/swagger.json",
        "/api-docs",
    ];

    for endpoint in endpoints {
        let url = format!("{}{}", base_url.trim_end_matches('/'), endpoint);
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.text().await {
                    return Ok(Some(body));
                }
            }
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn api_send(project_path: String, req: ApiSendRequest) -> Result<ApiResponse, String> {
    let env = load_project_env(&project_path);
    
    let resolved_url = resolve_vars(&req.url, &env);
    let resolved_method = resolve_vars(&req.method, &env);
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let method = match resolved_method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "HEAD" => reqwest::Method::HEAD,
        _ => reqwest::Method::GET,
    };

    let mut builder = client.request(method, &resolved_url);

    for header in &req.headers {
        let name_res = resolve_vars(&header.name, &env);
        let val_res = resolve_vars(&header.value, &env);
        builder = builder.header(name_res, val_res);
    }

    if let Some(body_str) = &req.body {
        let resolved_body = resolve_vars(body_str, &env);
        builder = builder.body(resolved_body);
    }

    let start = std::time::Instant::now();
    let resp = builder.send().await.map_err(|e| e.to_string())?;
    let elapsed = start.elapsed().as_millis() as u64;

    let status = resp.status().as_u16();
    let mut resp_headers = Vec::new();
    for (name, val) in resp.headers() {
        if let Ok(val_str) = val.to_str() {
            resp_headers.push(Header {
                name: name.as_str().to_string(),
                value: val_str.to_string(),
                description: None,
            });
        }
    }

    let body_text = resp.text().await.ok();
    let size_bytes = body_text.as_ref().map(|b| b.len() as u64).unwrap_or(0);

    Ok(ApiResponse {
        status,
        headers: resp_headers,
        body: body_text,
        timing_ms: elapsed,
        size_bytes,
    })
}

#[tauri::command]
pub async fn api_save_example(
    project_path: String,
    group_name: String,
    request_id: String,
    is_error: bool,
    example: CapturedExample,
) -> Result<ApiModel, String> {
    let mut model = load_docs(&project_path)?
        .ok_or_else(|| "No API model loaded. Import a spec first.".to_string())?;

    let group = model.groups.iter_mut()
        .find(|g| g.name == group_name)
        .ok_or_else(|| format!("Group not found: {}", group_name))?;

    let req = group.requests.iter_mut()
        .find(|r| r.id == request_id)
        .ok_or_else(|| format!("Request not found: {}", request_id))?;

    if is_error {
        req.doc.error_examples.push(example);
    } else {
        req.doc.examples.push(example);
    }

    save_docs(&project_path, &model).map_err(|e| e.to_string())?;
    Ok(model)
}

#[tauri::command]
pub async fn api_verify_example(
    project_path: String,
    group_name: String,
    request_id: String,
    label: String,
    is_error: bool,
) -> Result<ApiModel, String> {
    let mut model = load_docs(&project_path)?
        .ok_or_else(|| "No API model loaded. Import a spec first.".to_string())?;

    let (req_method, req_path, req_headers, req_body, req_schema) = {
        let group = model.groups.iter()
            .find(|g| g.name == group_name)
            .ok_or_else(|| format!("Group not found: {}", group_name))?;

        let req = group.requests.iter()
            .find(|r| r.id == request_id)
            .ok_or_else(|| format!("Request not found: {}", request_id))?;

        let example = if is_error {
            req.doc.error_examples.iter().find(|ex| ex.label == label)
        } else {
            req.doc.examples.iter().find(|ex| ex.label == label)
        }.ok_or_else(|| format!("Example not found: {}", label))?;

        let schema = req.body.as_ref().and_then(|b| b.schema.clone());

        (
            example.request.method.clone(),
            example.request.url.clone(),
            example.request.headers.clone(),
            example.request.body.clone(),
            schema,
        )
    };

    // Replay request
    let response = api_send(project_path.clone(), ApiSendRequest {
        method: req_method,
        url: req_path,
        headers: req_headers,
        body: req_body,
    }).await?;

    let now_secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let schema_ok = if let Some(schema) = &req_schema {
        if let Some(resp_body) = &response.body {
            if let Ok(json_val) = serde_json::from_str(resp_body) {
                Some(crush_apispec::validate_json_schema(&json_val, schema))
            } else {
                Some(false)
            }
        } else {
            Some(false)
        }
    } else {
        None
    };

    // Update verified example in model
    let group = model.groups.iter_mut()
        .find(|g| g.name == group_name)
        .ok_or_else(|| format!("Group not found: {}", group_name))?;

    let req = group.requests.iter_mut()
        .find(|r| r.id == request_id)
        .ok_or_else(|| format!("Request not found: {}", request_id))?;

    let example = if is_error {
        req.doc.error_examples.iter_mut().find(|ex| ex.label == label)
    } else {
        req.doc.examples.iter_mut().find(|ex| ex.label == label)
    }.ok_or_else(|| format!("Example not found: {}", label))?;

    example.response = SavedResponse {
        status: response.status,
        headers: response.headers,
        body: response.body,
        timing_ms: response.timing_ms,
        size_bytes: response.size_bytes,
    };
    example.verified_at = now_secs;
    example.schema_ok = schema_ok;

    save_docs(&project_path, &model).map_err(|e| e.to_string())?;
    Ok(model)
}

#[tauri::command]
pub async fn api_verify_all(project_path: String) -> Result<ApiModel, String> {
    let mut model = load_docs(&project_path)?
        .ok_or_else(|| "No API model loaded. Import a spec first.".to_string())?;

    // We collect list of items to verify to avoid nested borrows/async closures
    let mut verifications = Vec::new();

    for group in &model.groups {
        for req in &group.requests {
            let schema = req.body.as_ref().and_then(|b| b.schema.clone());
            for ex in &req.doc.examples {
                verifications.push((group.name.clone(), req.id.clone(), ex.label.clone(), false, schema.clone(), ex.request.clone()));
            }
            for ex in &req.doc.error_examples {
                verifications.push((group.name.clone(), req.id.clone(), ex.label.clone(), true, schema.clone(), ex.request.clone()));
            }
        }
    }

    for (group_name, req_id, label, is_error, req_schema, req_spec) in verifications {
        let response = match api_send(project_path.clone(), ApiSendRequest {
            method: req_spec.method,
            url: req_spec.url,
            headers: req_spec.headers,
            body: req_spec.body,
        }).await {
            Ok(r) => r,
            Err(_) => continue, // Skip failed connections during verify_all
        };

        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let schema_ok = if let Some(schema) = &req_schema {
            if let Some(resp_body) = &response.body {
                if let Ok(json_val) = serde_json::from_str(resp_body) {
                    Some(crush_apispec::validate_json_schema(&json_val, schema))
                } else {
                    Some(false)
                }
            } else {
                Some(false)
            }
        } else {
            None
        };

        // Find and update
        if let Some(group) = model.groups.iter_mut().find(|g| g.name == group_name) {
            if let Some(req) = group.requests.iter_mut().find(|r| r.id == req_id) {
                let example = if is_error {
                    req.doc.error_examples.iter_mut().find(|ex| ex.label == label)
                } else {
                    req.doc.examples.iter_mut().find(|ex| ex.label == label)
                };
                if let Some(ex) = example {
                    ex.response = SavedResponse {
                        status: response.status,
                        headers: response.headers,
                        body: response.body,
                        timing_ms: response.timing_ms,
                        size_bytes: response.size_bytes,
                    };
                    ex.verified_at = now_secs;
                    ex.schema_ok = schema_ok;
                }
            }
        }
    }

    save_docs(&project_path, &model).map_err(|e| e.to_string())?;
    Ok(model)
}

#[tauri::command]
pub async fn api_publish_docs(project_path: String) -> Result<String, String> {
    let api_dir = get_api_dir(&project_path);
    let model = load_docs(&project_path)?
        .ok_or_else(|| "No API model loaded. Import a spec first.".to_string())?;

    let pub_dir = api_dir.join("public");
    fs::create_dir_all(&pub_dir).map_err(|e| e.to_string())?;

    let mut html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Crush API Studio - Published Docs</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #0b0d11; color: #e2e8f0; margin: 0; padding: 0; }
        .container { display: flex; height: 100vh; }
        .sidebar { width: 300px; border-right: 1px solid #1f2937; background: #0f1115; overflow-y: auto; padding: 20px; }
        .main { flex: 1; overflow-y: auto; padding: 40px; }
        h1, h2, h3 { color: #f8fafc; }
        .method-GET { color: #10b981; }
        .method-POST { color: #f59e0b; }
        .method-PUT { color: #3b82f6; }
        .method-DELETE { color: #ef4444; }
        .endpoint-card { background: #13171e; border: 1px solid #1f2937; border-radius: 8px; padding: 20px; margin-bottom: 20px; }
        .badge { font-family: monospace; font-weight: bold; font-size: 12px; padding: 2px 6px; border-radius: 4px; background: rgba(255,255,255,0.05); }
        pre { background: #0b0d11; border: 1px solid #1f2937; border-radius: 6px; padding: 15px; overflow-x: auto; font-family: monospace; }
        code { font-family: monospace; }
        .nav-link { display: block; padding: 8px 12px; color: #94a3b8; text-decoration: none; border-radius: 6px; margin-bottom: 4px; font-size: 14px; }
        .nav-link:hover { background: #1e293b; color: #f1f5f9; }
    </style>
</head>
<body>
<div class="container">
    <div class="sidebar">
        <h2>API Spec Docs</h2>
        <div class="nav-section">
            <h3>Endpoints</h3>
"#.to_string();

    for group in &model.groups {
        html.push_str(&format!("<h4>{}</h4>\n", group.name));
        for req in &group.requests {
            html.push_str(&format!(
                "<a class=\"nav-link\" href=\"#{}\"><span class=\"method-{}\">{}</span> {}</a>\n",
                req.id, req.method, req.method, req.path
            ));
        }
    }

    html.push_str(r#"
        </div>
    </div>
    <div class="main">
        <h1>Published Reference Docs</h1>
"#);

    for group in &model.groups {
        html.push_str(&format!("<h2>Group: {}</h2>\n", group.name));
        for req in &group.requests {
            html.push_str(&format!(
                "<div class=\"endpoint-card\" id=\"{}\">\n<h3><span class=\"badge method-{}\">{}</span> {}</h3>\n",
                req.id, req.method, req.method, req.path
            ));
            if !req.doc.summary.is_empty() {
                html.push_str(&format!("<p><strong>{}</strong></p>\n", req.doc.summary));
            }
            if !req.doc.description_md.is_empty() {
                html.push_str(&format!("<p>{}</p>\n", req.doc.description_md));
            }
            
            // Examples
            if !req.doc.examples.is_empty() {
                html.push_str("<h4>Examples</h4>\n");
                for ex in &req.doc.examples {
                    html.push_str(&format!("<h5>{} (Status: {})</h5>\n", ex.label, ex.response.status));
                    if let Some(body) = &ex.response.body {
                        html.push_str(&format!("<pre><code>{}</code></pre>\n", body));
                    }
                }
            }
            html.push_str("</div>\n");
        }
    }

    html.push_str(r#"
    </div>
</div>
</body>
</html>
"#);

    let html_path = pub_dir.join("index.html");
    let tmp_html = html_path.with_extension("tmp");
    fs::write(&tmp_html, &html).map_err(|e| e.to_string())?;
    fs::rename(&tmp_html, &html_path).map_err(|e| e.to_string())?;

    Ok(html_path.to_string_lossy().to_string())
}

