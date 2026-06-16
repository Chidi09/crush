use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Unified API Model ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ApiModel {
    pub servers: Vec<Server>,
    pub groups: Vec<Group>,
    pub auth: Option<AuthScheme>,
    pub variables: Vec<Variable>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub requests: Vec<Request>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Request {
    pub id: String,
    pub method: String,
    pub path: String,
    pub params: Vec<Param>,
    pub headers: Vec<Header>,
    pub body: Option<BodySpec>,
    pub auth: Option<AuthScheme>,
    pub doc: RequestDoc,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub in_location: String, // "query", "path", "header", "cookie"
    pub required: bool,
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Header {
    pub name: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BodySpec {
    pub mime_type: String,
    pub schema: Option<serde_json::Value>,
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RequestDoc {
    pub summary: String,
    pub description_md: String,
    pub examples: Vec<CapturedExample>,
    pub error_examples: Vec<CapturedExample>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CapturedExample {
    pub label: String,
    pub request: SavedRequest,
    pub response: SavedResponse,
    pub verified_at: u64,
    pub schema_ok: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SavedRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SavedResponse {
    pub status: u16,
    pub headers: Vec<Header>,
    pub body: Option<String>,
    pub timing_ms: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AuthScheme {
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { key: String, value: String, in_location: String }, // "header", "query"
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Variable {
    pub name: String,
    pub value: String,
}

// --- Postman v2.1 Serde Structures ---

#[derive(Debug, Deserialize, Serialize)]
struct PostmanCollection {
    info: PostmanInfo,
    item: Vec<PostmanItem>,
    variable: Option<Vec<PostmanVariable>>,
    auth: Option<PostmanAuth>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PostmanInfo {
    name: String,
    schema: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum PostmanItem {
    Group {
        name: String,
        item: Vec<PostmanItem>,
        description: Option<serde_json::Value>,
    },
    Request {
        name: String,
        request: PostmanRequest,
        response: Option<Vec<PostmanResponse>>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum PostmanRequest {
    String(String),
    Detailed {
        method: String,
        url: Option<PostmanUrl>,
        header: Option<Vec<PostmanHeader>>,
        body: Option<PostmanBody>,
        description: Option<serde_json::Value>,
        auth: Option<PostmanAuth>,
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum PostmanUrl {
    String(String),
    Detailed {
        raw: Option<String>,
        host: Option<Vec<String>>,
        path: Option<Vec<String>>,
        query: Option<Vec<PostmanQueryParam>>,
        variable: Option<Vec<PostmanVariable>>,
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PostmanQueryParam {
    key: Option<String>,
    value: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PostmanVariable {
    key: String,
    value: Option<serde_json::Value>,
    #[serde(rename = "type")]
    var_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PostmanHeader {
    key: String,
    value: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PostmanBody {
    mode: Option<String>,
    raw: Option<String>,
    options: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
struct PostmanResponse {
    name: Option<String>,
    originalRequest: Option<PostmanRequest>,
    status: Option<String>,
    code: Option<u16>,
    header: Option<Vec<PostmanHeader>>,
    body: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PostmanAuth {
    #[serde(rename = "type")]
    auth_type: String,
    bearer: Option<Vec<PostmanAuthAttr>>,
    basic: Option<Vec<PostmanAuthAttr>>,
    apikey: Option<Vec<PostmanAuthAttr>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PostmanAuthAttr {
    key: String,
    value: serde_json::Value,
    #[serde(rename = "type")]
    attr_type: Option<String>,
}

// --- Main Parser Function ---

pub fn parse_spec(content: &[u8]) -> Result<ApiModel, String> {
    // 1. Try to parse as JSON or YAML to Value to detect type
    let value: serde_json::Value = if let Ok(v) = serde_json::from_slice(content) {
        v
    } else if let Ok(yaml_val) = serde_yaml::from_slice::<serde_json::Value>(content) {
        yaml_val
    } else {
        return Err("Failed to parse bytes as either valid JSON or YAML".to_string());
    };

    // 2. Detect Postman Collection v2
    if let Some(info) = value.get("info").and_then(|i| i.as_object()) {
        if let Some(schema) = info.get("schema").and_then(|s| s.as_str()) {
            if schema.contains("collection/v2.1") || schema.contains("collection/v2.0") {
                let collection: PostmanCollection = serde_json::from_value(value)
                    .map_err(|e| format!("Failed to parse Postman collection schema: {}", e))?;
                return parse_postman(collection);
            }
        }
    }

    // 3. Detect OpenAPI 3.x
    if value.get("openapi").is_some() {
        let openapi: openapiv3::OpenAPI = serde_json::from_value(value)
            .map_err(|e| format!("Failed to parse OpenAPI v3 spec: {}", e))?;
        return parse_openapi_v3(openapi);
    }

    // 4. Detect OpenAPI 2.0 (Swagger)
    if value.get("swagger").is_some() {
        return parse_swagger_v2(value);
    }

    Err("Unrecognized API Spec format. Must be OpenAPI 3.x, Swagger 2.0, or Postman Collection 2.0/2.1".to_string())
}

// --- OpenAPI v3 Parser ---

fn resolve_schema<'a>(spec: &'a openapiv3::OpenAPI, ref_or: &'a openapiv3::ReferenceOr<openapiv3::Schema>) -> Option<&'a openapiv3::Schema> {
    match ref_or {
        openapiv3::ReferenceOr::Item(schema) => Some(schema),
        openapiv3::ReferenceOr::Reference { reference } => {
            let prefix = "#/components/schemas/";
            if reference.starts_with(prefix) {
                let name = &reference[prefix.len()..];
                let next = spec.components.as_ref()?.schemas.get(name)?;
                resolve_schema(spec, next)
            } else {
                None
            }
        }
    }
}

fn resolve_parameter<'a>(spec: &'a openapiv3::OpenAPI, ref_or: &'a openapiv3::ReferenceOr<openapiv3::Parameter>) -> Option<&'a openapiv3::Parameter> {
    match ref_or {
        openapiv3::ReferenceOr::Item(param) => Some(param),
        openapiv3::ReferenceOr::Reference { reference } => {
            let prefix = "#/components/parameters/";
            if reference.starts_with(prefix) {
                let name = &reference[prefix.len()..];
                let next = spec.components.as_ref()?.parameters.get(name)?;
                resolve_parameter(spec, next)
            } else {
                None
            }
        }
    }
}

fn resolve_request_body<'a>(spec: &'a openapiv3::OpenAPI, ref_or: &'a openapiv3::ReferenceOr<openapiv3::RequestBody>) -> Option<&'a openapiv3::RequestBody> {
    match ref_or {
        openapiv3::ReferenceOr::Item(body) => Some(body),
        openapiv3::ReferenceOr::Reference { reference } => {
            let prefix = "#/components/requestBodies/";
            if reference.starts_with(prefix) {
                let name = &reference[prefix.len()..];
                let next = spec.components.as_ref()?.request_bodies.get(name)?;
                resolve_request_body(spec, next)
            } else {
                None
            }
        }
    }
}

fn parse_openapi_v3(spec: openapiv3::OpenAPI) -> Result<ApiModel, String> {
    let servers = spec.servers.iter().map(|s| Server {
        url: s.url.clone(),
        description: s.description.clone(),
    }).collect::<Vec<_>>();

    let global_auth = spec.security.as_ref().and_then(|secs| {
        secs.first().and_then(|req| {
            req.keys().next().and_then(|scheme_name| {
                spec.components.as_ref().and_then(|c| {
                    c.security_schemes.get(scheme_name).and_then(|ref_or| {
                        match ref_or {
                            openapiv3::ReferenceOr::Item(scheme) => map_openapi_auth(scheme),
                            _ => None,
                        }
                    })
                })
            })
        })
    });

    let mut group_requests: HashMap<String, Vec<Request>> = HashMap::new();

    for (path, ref_or_path_item) in spec.paths.iter() {
        let path_item = match ref_or_path_item {
            openapiv3::ReferenceOr::Item(item) => item,
            _ => continue, // For simplicity, skip referenced path items or resolve if needed
        };

        // Combine path-level parameters and operation-level parameters
        let path_params = &path_item.parameters;

        let mut process_op = |method: &str, op: &openapiv3::Operation| {
            let id = op.operation_id.clone().unwrap_or_else(|| {
                format!("{}_{}", method.to_lowercase(), path.replace("/", "_").trim_start_matches('_'))
            });

            let mut params = Vec::new();
            let mut headers = Vec::new();

            let all_params = path_params.iter().chain(op.parameters.iter());
            for ref_or_param in all_params {
                if let Some(param) = resolve_parameter(&spec, ref_or_param) {
                    let in_loc = match param {
                        openapiv3::Parameter::Query { .. } => "query",
                        openapiv3::Parameter::Path { .. } => "path",
                        openapiv3::Parameter::Header { .. } => "header",
                        openapiv3::Parameter::Cookie { .. } => "cookie",
                    };

                    let param_data = match param {
                        openapiv3::Parameter::Query { parameter_data, .. } => parameter_data,
                        openapiv3::Parameter::Path { parameter_data, .. } => parameter_data,
                        openapiv3::Parameter::Header { parameter_data, .. } => parameter_data,
                        openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data,
                    };

                    let required = match param {
                        openapiv3::Parameter::Path { .. } => true,
                        openapiv3::Parameter::Query { parameter_data, .. } |
                        openapiv3::Parameter::Header { parameter_data, .. } |
                        openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data.required,
                    };

                    let schema_val = match &param_data.format {
                        openapiv3::ParameterSchemaOrContent::Schema(ref_or_schema) => {
                            resolve_schema(&spec, ref_or_schema).and_then(|s| serde_json::to_value(s).ok())
                        }
                        _ => None,
                    };

                    params.push(Param {
                        name: param_data.name.clone(),
                        in_location: in_loc.to_string(),
                        required,
                        schema: schema_val,
                        description: param_data.description.clone(),
                    });

                    if in_loc == "header" {
                        headers.push(Header {
                            name: param_data.name.clone(),
                            value: "".to_string(),
                            description: param_data.description.clone(),
                        });
                    }
                }
            }

            let body = op.request_body.as_ref().and_then(|ref_or_body| {
                resolve_request_body(&spec, ref_or_body).and_then(|body_spec| {
                    // Find first available media type, preference for application/json
                    let (mime, media_type) = if let Some(mt) = body_spec.content.get("application/json") {
                        ("application/json", mt)
                    } else {
                        let first = body_spec.content.iter().next()?;
                        (first.0.as_str(), first.1)
                    };

                    let schema_val = match &media_type.schema {
                        Some(ref_or_schema) => resolve_schema(&spec, ref_or_schema).and_then(|s| serde_json::to_value(s).ok()),
                        None => None,
                    };

                    let example_val = media_type.example.clone().or_else(|| {
                        media_type.examples.iter().next().map(|(_, e)| {
                            match e {
                                openapiv3::ReferenceOr::Item(ex) => ex.value.clone().unwrap_or(serde_json::Value::Null),
                                _ => serde_json::Value::Null,
                            }
                        })
                    });

                    Some(BodySpec {
                        mime_type: mime.to_string(),
                        schema: schema_val,
                        example: example_val,
                    })
                })
            });

            let op_auth = op.security.as_ref().and_then(|secs| {
                secs.first().and_then(|req| {
                    req.keys().next().and_then(|scheme_name| {
                        spec.components.as_ref().and_then(|c| {
                            c.security_schemes.get(scheme_name).and_then(|ref_or| {
                                match ref_or {
                                    openapiv3::ReferenceOr::Item(scheme) => map_openapi_auth(scheme),
                                    _ => None,
                                }
                            })
                        })
                    })
                })
            });

            let doc = RequestDoc {
                summary: op.summary.clone().unwrap_or_default(),
                description_md: op.description.clone().unwrap_or_default(),
                examples: Vec::new(),
                error_examples: Vec::new(),
            };

            let req = Request {
                id,
                method: method.to_string(),
                path: path.clone(),
                params,
                headers,
                body,
                auth: op_auth,
                doc,
            };

            let tag = op.tags.first().cloned().unwrap_or_else(|| "Default".to_string());
            group_requests.entry(tag).or_default().push(req);
        };

        if let Some(op) = &path_item.get { process_op("GET", op); }
        if let Some(op) = &path_item.post { process_op("POST", op); }
        if let Some(op) = &path_item.put { process_op("PUT", op); }
        if let Some(op) = &path_item.delete { process_op("DELETE", op); }
        if let Some(op) = &path_item.options { process_op("OPTIONS", op); }
        if let Some(op) = &path_item.head { process_op("HEAD", op); }
        if let Some(op) = &path_item.patch { process_op("PATCH", op); }
        if let Some(op) = &path_item.trace { process_op("TRACE", op); }
    }

    let groups = group_requests.into_iter().map(|(name, requests)| Group {
        name,
        requests,
    }).collect::<Vec<_>>();

    Ok(ApiModel {
        servers,
        groups,
        auth: global_auth,
        variables: Vec::new(),
    })
}

fn map_openapi_auth(scheme: &openapiv3::SecurityScheme) -> Option<AuthScheme> {
    match scheme {
        openapiv3::SecurityScheme::HTTP { scheme, .. } => {
            if scheme == "bearer" {
                Some(AuthScheme::Bearer { token: "".to_string() })
            } else if scheme == "basic" {
                Some(AuthScheme::Basic { username: "".to_string(), password: "".to_string() })
            } else {
                None
            }
        }
        openapiv3::SecurityScheme::APIKey { name, location, .. } => {
            let in_loc = match location {
                openapiv3::APIKeyLocation::Header => "header",
                openapiv3::APIKeyLocation::Query => "query",
                openapiv3::APIKeyLocation::Cookie => "cookie",
            };
            Some(AuthScheme::ApiKey {
                key: name.clone(),
                value: "".to_string(),
                in_location: in_loc.to_string(),
            })
        }
        _ => None,
    }
}

// --- Swagger v2 (OpenAPI 2) Parser ---

fn parse_swagger_v2(value: serde_json::Value) -> Result<ApiModel, String> {
    let mut servers = Vec::new();
    let host = value.get("host").and_then(|h| h.as_str());
    let base_path = value.get("basePath").and_then(|b| b.as_str()).unwrap_or("");
    let schemes = value.get("schemes").and_then(|s| s.as_array());

    if let Some(h) = host {
        if let Some(schs) = schemes {
            for scheme in schs {
                if let Some(sch) = scheme.as_str() {
                    servers.push(Server {
                        url: format!("{}://{}{}", sch, h, base_path),
                        description: None,
                    });
                }
            }
        } else {
            servers.push(Server {
                url: format!("http://{}{}", h, base_path),
                description: None,
            });
        }
    } else if !base_path.is_empty() {
        servers.push(Server {
            url: base_path.to_string(),
            description: None,
        });
    }

    let mut group_requests: HashMap<String, Vec<Request>> = HashMap::new();

    if let Some(paths_obj) = value.get("paths").and_then(|p| p.as_object()) {
        for (path, path_item_val) in paths_obj {
            if let Some(path_item) = path_item_val.as_object() {
                for (method, op_val) in path_item {
                    let method_upper = method.to_uppercase();
                    if !["GET", "POST", "PUT", "DELETE", "OPTIONS", "HEAD", "PATCH"].contains(&method_upper.as_str()) {
                        continue;
                    }

                    if let Some(op) = op_val.as_object() {
                        let id = op.get("operationId").and_then(|v| v.as_str()).map(String::from).unwrap_or_else(|| {
                            format!("{}_{}", method.to_lowercase(), path.replace("/", "_").trim_start_matches('_'))
                        });

                        let summary = op.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let description = op.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();

                        let mut params = Vec::new();
                        let mut headers = Vec::new();

                        if let Some(params_arr) = op.get("parameters").and_then(|p| p.as_array()) {
                            for param_val in params_arr {
                                if let Some(param) = param_val.as_object() {
                                    let name = param.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let in_loc = param.get("in").and_then(|v| v.as_str()).unwrap_or("query").to_string();
                                    let required = param.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                                    let desc = param.get("description").and_then(|v| v.as_str()).map(String::from);

                                    // Build simple schema representation for param
                                    let schema_val = param.get("schema").cloned().or_else(|| {
                                        let mut s = serde_json::Map::new();
                                        if let Some(t) = param.get("type") { s.insert("type".to_string(), t.clone()); }
                                        if let Some(f) = param.get("format") { s.insert("format".to_string(), f.clone()); }
                                        Some(serde_json::Value::Object(s))
                                    });

                                    params.push(Param {
                                        name: name.clone(),
                                        in_location: in_loc.clone(),
                                        required,
                                        schema: schema_val,
                                        description: desc,
                                    });

                                    if in_loc == "header" {
                                        headers.push(Header {
                                            name,
                                            value: "".to_string(),
                                            description: None,
                                        });
                                    }
                                }
                            }
                        }

                        // Swagger v2 body parameter is defined as a parameter with in: "body"
                        let body = op.get("parameters").and_then(|p| p.as_array()).and_then(|arr| {
                            arr.iter().find(|p| p.get("in").and_then(|v| v.as_str()) == Some("body")).and_then(|p| {
                                let schema_val = p.get("schema").cloned();
                                Some(BodySpec {
                                    mime_type: "application/json".to_string(), // Default Swagger body
                                    schema: schema_val,
                                    example: None,
                                })
                            })
                        });

                        let doc = RequestDoc {
                            summary,
                            description_md: description,
                            examples: Vec::new(),
                            error_examples: Vec::new(),
                        };

                        let req = Request {
                            id,
                            method: method_upper,
                            path: path.clone(),
                            params,
                            headers,
                            body,
                            auth: None, // Simplified for v2
                            doc,
                        };

                        let tag = op.get("tags").and_then(|t| t.as_array()).and_then(|a| a.first()).and_then(|v| v.as_str()).unwrap_or("Default").to_string();
                        group_requests.entry(tag).or_default().push(req);
                    }
                }
            }
        }
    }

    let groups = group_requests.into_iter().map(|(name, requests)| Group {
        name,
        requests,
    }).collect::<Vec<_>>();

    Ok(ApiModel {
        servers,
        groups,
        auth: None,
        variables: Vec::new(),
    })
}

// --- Postman Collection Parser ---

fn parse_postman(collection: PostmanCollection) -> Result<ApiModel, String> {
    let mut groups = Vec::new();
    collect_postman_items(&collection.item, "", &mut groups);

    let variables = collection.variable.unwrap_or_default().into_iter().map(|v| {
        let val_str = match v.value {
            Some(serde_json::Value::String(s)) => s,
            Some(val) => val.to_string(),
            None => "".to_string(),
        };
        Variable {
            name: v.key,
            value: val_str,
        }
    }).collect::<Vec<_>>();

    let global_auth = collection.auth.as_ref().and_then(map_postman_auth);

    Ok(ApiModel {
        servers: Vec::new(), // Postman URLs are full URLs, no base server is explicitly defined
        groups,
        auth: global_auth,
        variables,
    })
}

fn collect_postman_items(items: &[PostmanItem], current_group_name: &str, groups: &mut Vec<Group>) {
    for item in items {
        match item {
            PostmanItem::Group { name, item: sub_items, description: _ } => {
                let next_group = if current_group_name.is_empty() {
                    name.clone()
                } else {
                    format!("{} / {}", current_group_name, name)
                };
                collect_postman_items(sub_items, &next_group, groups);
            }
            PostmanItem::Request { name, request, response } => {
                let mut req_headers = Vec::new();
                let mut req_params = Vec::new();
                let mut req_body = None;
                let mut req_auth = None;

                let (method, raw_url, desc_md) = match request {
                    PostmanRequest::String(url_str) => ("GET".to_string(), url_str.clone(), "".to_string()),
                    PostmanRequest::Detailed { method, url, header, body, description, auth } => {
                        let url_str = match url {
                            Some(PostmanUrl::String(s)) => s.clone(),
                            Some(PostmanUrl::Detailed { raw, host, path, query, variable }) => {
                                // Extract query params
                                if let Some(q_arr) = query {
                                    for q in q_arr {
                                        if let Some(key) = &q.key {
                                            req_params.push(Param {
                                                name: key.clone(),
                                                in_location: "query".to_string(),
                                                required: false,
                                                schema: None,
                                                description: q.description.clone(),
                                            });
                                        }
                                    }
                                }

                                // Extract path variables
                                if let Some(v_arr) = variable {
                                    for v in v_arr {
                                        req_params.push(Param {
                                            name: v.key.clone(),
                                            in_location: "path".to_string(),
                                            required: true,
                                            schema: None,
                                            description: None,
                                        });
                                    }
                                }

                                raw.clone().unwrap_or_else(|| {
                                    let h = host.as_ref().map(|hs| hs.join(".")).unwrap_or_default();
                                    let p = path.as_ref().map(|ps| ps.join("/")).unwrap_or_default();
                                    format!("{}/{}", h, p)
                                })
                            }
                            None => "".to_string(),
                        };

                        if let Some(h_arr) = header {
                            for h in h_arr {
                                req_headers.push(Header {
                                    name: h.key.clone(),
                                    value: h.value.clone(),
                                    description: h.description.clone(),
                                });
                            }
                        }

                        if let Some(b) = body {
                            if b.mode.as_deref() == Some("raw") {
                                req_body = Some(BodySpec {
                                    mime_type: "application/json".to_string(), // Typical fallback
                                    schema: None,
                                    example: b.raw.as_ref().and_then(|r| serde_json::from_str(r).ok()),
                                });
                            }
                        }

                        req_auth = auth.as_ref().and_then(map_postman_auth);

                        let desc_str = description.as_ref().map(|d| {
                            match d {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Object(o) => o.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                                _ => "".to_string(),
                            }
                        }).unwrap_or_default();

                        (method.clone(), url_str, desc_str)
                    }
                };

                let mut examples = Vec::new();
                let mut error_examples = Vec::new();

                if let Some(resp_arr) = response {
                    for r in resp_arr {
                        let label = r.name.clone().unwrap_or_else(|| "Example".to_string());
                        let status = r.code.unwrap_or(200);

                        let resp_headers = r.header.as_ref().map(|ha| {
                            ha.iter().map(|h| Header {
                                name: h.key.clone(),
                                value: h.value.clone(),
                                description: h.description.clone(),
                            }).collect::<Vec<_>>()
                        }).unwrap_or_default();

                        let saved_req = SavedRequest {
                            method: method.clone(),
                            url: raw_url.clone(),
                            headers: req_headers.clone(),
                            body: None, // Postman doesn't save request bodies directly in responses in a standard way
                        };

                        let saved_resp = SavedResponse {
                            status,
                            headers: resp_headers,
                            body: r.body.clone(),
                            timing_ms: 0,
                            size_bytes: r.body.as_ref().map(|b| b.len() as u64).unwrap_or(0),
                        };

                        let example = CapturedExample {
                            label,
                            request: saved_req,
                            response: saved_resp,
                            verified_at: 0,
                            schema_ok: None,
                        };

                        if status >= 400 {
                            error_examples.push(example);
                        } else {
                            examples.push(example);
                        }
                    }
                }

                let req = Request {
                    id: name.replace(" ", "_").to_lowercase(),
                    method,
                    path: raw_url,
                    params: req_params,
                    headers: req_headers,
                    body: req_body,
                    auth: req_auth,
                    doc: RequestDoc {
                        summary: name.clone(),
                        description_md: desc_md,
                        examples,
                        error_examples,
                    },
                };

                let group_name = if current_group_name.is_empty() { "Default" } else { current_group_name };
                if let Some(g) = groups.iter_mut().find(|g| g.name == group_name) {
                    g.requests.push(req);
                } else {
                    groups.push(Group {
                        name: group_name.to_string(),
                        requests: vec![req],
                    });
                }
            }
        }
    }
}

fn map_postman_auth(auth: &PostmanAuth) -> Option<AuthScheme> {
    if auth.auth_type == "bearer" {
        let token = auth.bearer.as_ref()
            .and_then(|v| v.iter().find(|attr| attr.key == "token"))
            .and_then(|attr| attr.value.as_str())
            .unwrap_or("")
            .to_string();
        Some(AuthScheme::Bearer { token })
    } else if auth.auth_type == "basic" {
        let username = auth.basic.as_ref()
            .and_then(|v| v.iter().find(|attr| attr.key == "username"))
            .and_then(|attr| attr.value.as_str())
            .unwrap_or("")
            .to_string();
        let password = auth.basic.as_ref()
            .and_then(|v| v.iter().find(|attr| attr.key == "password"))
            .and_then(|attr| attr.value.as_str())
            .unwrap_or("")
            .to_string();
        Some(AuthScheme::Basic { username, password })
    } else if auth.auth_type == "apikey" {
        let key = auth.apikey.as_ref()
            .and_then(|v| v.iter().find(|attr| attr.key == "key"))
            .and_then(|attr| attr.value.as_str())
            .unwrap_or("Authorization")
            .to_string();
        let value = auth.apikey.as_ref()
            .and_then(|v| v.iter().find(|attr| attr.key == "value"))
            .and_then(|attr| attr.value.as_str())
            .unwrap_or("")
            .to_string();
        let in_loc = auth.apikey.as_ref()
            .and_then(|v| v.iter().find(|attr| attr.key == "in"))
            .and_then(|attr| attr.value.as_str())
            .unwrap_or("header")
            .to_string();
        Some(AuthScheme::ApiKey { key, value, in_location: in_loc })
    } else {
        None
    }
}

// --- Schema Validation Layer ---

pub fn validate_json_schema(value: &serde_json::Value, schema: &serde_json::Value) -> bool {
    let schema_obj = match schema.as_object() {
        Some(o) => o,
        None => return true,
    };

    if let Some(type_val) = schema_obj.get("type").and_then(|v| v.as_str()) {
        match type_val {
            "string" => {
                if !value.is_string() { return false; }
            }
            "number" => {
                if !value.is_number() { return false; }
            }
            "integer" => {
                if !value.is_i64() && !value.is_u64() {
                    if let Some(n) = value.as_f64() {
                        if n.fract() != 0.0 { return false; }
                    } else {
                        return false;
                    }
                }
            }
            "boolean" => {
                if !value.is_boolean() { return false; }
            }
            "array" => {
                let arr = match value.as_array() {
                    Some(a) => a,
                    None => return false,
                };
                if let Some(items_schema) = schema_obj.get("items") {
                    for item in arr {
                        if !validate_json_schema(item, items_schema) {
                            return false;
                        }
                    }
                }
            }
            "object" => {
                let obj = match value.as_object() {
                    Some(o) => o,
                    None => return false,
                };
                if let Some(required_val) = schema_obj.get("required").and_then(|v| v.as_array()) {
                    for req_key in required_val {
                        if let Some(key_str) = req_key.as_str() {
                            if !obj.contains_key(key_str) {
                                return false;
                            }
                        }
                    }
                }
                if let Some(properties) = schema_obj.get("properties").and_then(|v| v.as_object()) {
                    for (k, v) in obj {
                        if let Some(prop_schema) = properties.get(k) {
                            if !validate_json_schema(v, prop_schema) {
                                return false;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_json_schema_basic() {
        let schema = serde_json::json!({
            "type": "string"
        });
        assert!(validate_json_schema(&serde_json::json!("hello"), &schema));
        assert!(!validate_json_schema(&serde_json::json!(123), &schema));

        let schema_int = serde_json::json!({
            "type": "integer"
        });
        assert!(validate_json_schema(&serde_json::json!(42), &schema_int));
        assert!(!validate_json_schema(&serde_json::json!(3.14), &schema_int));
    }

    #[test]
    fn test_validate_json_schema_object() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name", "age"],
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            }
        });

        let valid = serde_json::json!({
            "name": "Alice",
            "age": 30
        });
        let invalid_missing = serde_json::json!({
            "name": "Alice"
        });
        let invalid_type = serde_json::json!({
            "name": "Alice",
            "age": "thirty"
        });

        assert!(validate_json_schema(&valid, &schema));
        assert!(!validate_json_schema(&invalid_missing, &schema));
        assert!(!validate_json_schema(&invalid_type, &schema));
    }

    #[test]
    fn test_validate_json_schema_array() {
        let schema = serde_json::json!({
            "type": "array",
            "items": { "type": "string" }
        });

        let valid = serde_json::json!(["a", "b", "c"]);
        let invalid = serde_json::json!(["a", 2, "c"]);

        assert!(validate_json_schema(&valid, &schema));
        assert!(!validate_json_schema(&invalid, &schema));
    }

    #[test]
    fn test_parse_postman_basic() {
        let data = serde_json::json!({
            "info": {
                "name": "Test Postman",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Get User",
                    "request": {
                        "method": "GET",
                        "url": "http://localhost:3000/users/1"
                    }
                }
            ]
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert_eq!(api_model.groups.len(), 1);
        assert_eq!(api_model.groups[0].name, "Default");
        assert_eq!(api_model.groups[0].requests[0].method, "GET");
        assert_eq!(api_model.groups[0].requests[0].path, "http://localhost:3000/users/1");
    }

    #[test]
    fn test_parse_postman_nested_groups() {
        let data = serde_json::json!({
            "info": {
                "name": "Test Nested",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Users Folder",
                    "item": [
                        {
                            "name": "Admin",
                            "item": [
                                {
                                    "name": "Create User",
                                    "request": {
                                        "method": "POST",
                                        "url": "http://localhost/users"
                                    }
                                }
                            ]
                        }
                    ]
                }
            ]
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert_eq!(api_model.groups[0].name, "Users Folder / Admin");
        assert_eq!(api_model.groups[0].requests[0].method, "POST");
    }

    #[test]
    fn test_parse_postman_variables() {
        let data = serde_json::json!({
            "info": {
                "name": "Test Variables",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [],
            "variable": [
                { "key": "baseUrl", "value": "http://localhost:8080" },
                { "key": "token", "value": "secret" }
            ]
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert_eq!(api_model.variables.len(), 2);
        assert_eq!(api_model.variables[0].name, "baseUrl");
        assert_eq!(api_model.variables[0].value, "http://localhost:8080");
    }

    #[test]
    fn test_parse_postman_headers() {
        let data = serde_json::json!({
            "info": {
                "name": "Test Headers",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Get Info",
                    "request": {
                        "method": "GET",
                        "url": "http://localhost/info",
                        "header": [
                            { "key": "X-Test", "value": "test-value", "description": "some desc" }
                        ]
                    }
                }
            ]
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        let req = &api_model.groups[0].requests[0];
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].name, "X-Test");
        assert_eq!(req.headers[0].value, "test-value");
    }

    #[test]
    fn test_parse_postman_body() {
        let data = serde_json::json!({
            "info": {
                "name": "Test Body",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Create Post",
                    "request": {
                        "method": "POST",
                        "url": "http://localhost/posts",
                        "body": {
                            "mode": "raw",
                            "raw": "{\"title\":\"Hello\"}"
                        }
                    }
                }
            ]
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        let req = &api_model.groups[0].requests[0];
        let body = req.body.as_ref().unwrap();
        assert_eq!(body.mime_type, "application/json");
        assert_eq!(body.example.as_ref().unwrap().get("title").unwrap().as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_parse_postman_responses() {
        let data = serde_json::json!({
            "info": {
                "name": "Test Responses",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Get Data",
                    "request": {
                        "method": "GET",
                        "url": "http://localhost/data"
                    },
                    "response": [
                        {
                            "name": "Success",
                            "code": 200,
                            "body": "{\"ok\":true}"
                        },
                        {
                            "name": "Not Found",
                            "code": 404,
                            "body": "{\"error\":\"not found\"}"
                        }
                    ]
                }
            ]
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        let req = &api_model.groups[0].requests[0];
        assert_eq!(req.doc.examples.len(), 1);
        assert_eq!(req.doc.error_examples.len(), 1);
        assert_eq!(req.doc.examples[0].label, "Success");
        assert_eq!(req.doc.examples[0].response.status, 200);
        assert_eq!(req.doc.error_examples[0].label, "Not Found");
        assert_eq!(req.doc.error_examples[0].response.status, 404);
    }

    #[test]
    fn test_parse_postman_auth_bearer() {
        let data = serde_json::json!({
            "info": {
                "name": "Test Bearer Auth",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [],
            "auth": {
                "type": "bearer",
                "bearer": [
                    { "key": "token", "value": "mytoken", "type": "string" }
                ]
            }
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert!(matches!(api_model.auth.as_ref().unwrap(), AuthScheme::Bearer { token } if token == "mytoken"));
    }

    #[test]
    fn test_parse_postman_auth_basic() {
        let data = serde_json::json!({
            "info": {
                "name": "Test Basic Auth",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [],
            "auth": {
                "type": "basic",
                "basic": [
                    { "key": "username", "value": "user123", "type": "string" },
                    { "key": "password", "value": "pass123", "type": "string" }
                ]
            }
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert!(matches!(api_model.auth.as_ref().unwrap(), AuthScheme::Basic { username, password } if username == "user123" && password == "pass123"));
    }

    #[test]
    fn test_parse_postman_auth_apikey() {
        let data = serde_json::json!({
            "info": {
                "name": "Test ApiKey Auth",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [],
            "auth": {
                "type": "apikey",
                "apikey": [
                    { "key": "key", "value": "X-API-Key", "type": "string" },
                    { "key": "value", "value": "my-key-value", "type": "string" },
                    { "key": "in", "value": "header", "type": "string" }
                ]
            }
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert!(matches!(api_model.auth.as_ref().unwrap(), AuthScheme::ApiKey { key, value, in_location } if key == "X-API-Key" && value == "my-key-value" && in_location == "header"));
    }

    #[test]
    fn test_parse_openapi3_minimal() {
        let data = serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Minimal API",
                "version": "1.0.0"
            },
            "paths": {
                "/ping": {
                    "get": {
                        "summary": "Ping service",
                        "responses": {
                            "200": {
                                "description": "OK"
                            }
                        }
                    }
                }
            }
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert_eq!(api_model.groups.len(), 1);
        assert_eq!(api_model.groups[0].requests.len(), 1);
        assert_eq!(api_model.groups[0].requests[0].method, "GET");
        assert_eq!(api_model.groups[0].requests[0].path, "/ping");
        assert_eq!(api_model.groups[0].requests[0].doc.summary, "Ping service");
    }

    #[test]
    fn test_parse_openapi3_parameters() {
        let data = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "API with Params", "version": "1.0" },
            "paths": {
                "/items/{id}": {
                    "get": {
                        "parameters": [
                            {
                                "name": "id",
                                "in": "path",
                                "required": true,
                                "schema": { "type": "integer" }
                            },
                            {
                                "name": "q",
                                "in": "query",
                                "required": false,
                                "schema": { "type": "string" }
                            }
                        ],
                        "responses": { "200": { "description": "success" } }
                    }
                }
            }
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        let req = &api_model.groups[0].requests[0];
        assert_eq!(req.params.len(), 2);
        assert_eq!(req.params[0].name, "id");
        assert_eq!(req.params[0].in_location, "path");
        assert!(req.params[0].required);
        assert_eq!(req.params[1].name, "q");
        assert_eq!(req.params[1].in_location, "query");
        assert!(!req.params[1].required);
    }

    #[test]
    fn test_parse_openapi3_request_body() {
        let data = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "API with Body", "version": "1.0" },
            "paths": {
                "/users": {
                    "post": {
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "username": { "type": "string" }
                                        }
                                    }
                                }
                            }
                        },
                        "responses": { "201": { "description": "created" } }
                    }
                }
            }
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        let req = &api_model.groups[0].requests[0];
        let body = req.body.as_ref().unwrap();
        assert_eq!(body.mime_type, "application/json");
        let schema_val = body.schema.as_ref().unwrap();
        assert_eq!(schema_val.get("type").unwrap().as_str().unwrap(), "object");
    }

    #[test]
    fn test_parse_openapi3_auth_bearer() {
        let data = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "API with Auth", "version": "1.0" },
            "security": [
                { "bearerAuth": [] }
            ],
            "paths": {
                "/secure": {
                    "get": {
                        "responses": { "200": { "description": "OK" } }
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer",
                        "bearerFormat": "JWT"
                    }
                }
            }
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert!(matches!(api_model.auth.as_ref().unwrap(), AuthScheme::Bearer { .. }));
    }

    #[test]
    fn test_parse_openapi3_auth_apikey() {
        let data = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "API with ApiKey", "version": "1.0" },
            "security": [
                { "apiKeyAuth": [] }
            ],
            "paths": {
                "/secure": {
                    "get": {
                        "responses": { "200": { "description": "OK" } }
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "apiKeyAuth": {
                        "type": "apiKey",
                        "name": "X-API-Key",
                        "in": "header"
                    }
                }
            }
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert!(matches!(api_model.auth.as_ref().unwrap(), AuthScheme::ApiKey { key, in_location, .. } if key == "X-API-Key" && in_location == "header"));
    }

    #[test]
    fn test_parse_swagger2_basic() {
        let data = serde_json::json!({
            "swagger": "2.0",
            "info": {
                "title": "Swagger Test",
                "version": "1.0"
            },
            "host": "api.example.com",
            "basePath": "/v1",
            "schemes": ["https"],
            "paths": {
                "/users": {
                    "get": {
                        "summary": "Get users",
                        "tags": ["Users"],
                        "parameters": [
                            {
                                "name": "limit",
                                "in": "query",
                                "type": "integer",
                                "required": false
                            }
                        ]
                    }
                }
            }
        });
        let content = serde_json::to_vec(&data).unwrap();
        let api_model = parse_spec(&content).unwrap();
        assert_eq!(api_model.servers[0].url, "https://api.example.com/v1");
        assert_eq!(api_model.groups[0].name, "Users");
        assert_eq!(api_model.groups[0].requests[0].method, "GET");
        assert_eq!(api_model.groups[0].requests[0].params[0].name, "limit");
    }

    #[test]
    fn test_parse_unrecognized() {
        let data = serde_json::json!({
            "hello": "world"
        });
        let content = serde_json::to_vec(&data).unwrap();
        let err = parse_spec(&content).unwrap_err();
        assert!(err.contains("Unrecognized API Spec format"));
    }

    #[test]
    fn test_parse_invalid_json_yaml() {
        let content = b"not a valid json or yaml: { : }";
        let err = parse_spec(content).unwrap_err();
        assert!(err.contains("Failed to parse bytes"));
    }
}
