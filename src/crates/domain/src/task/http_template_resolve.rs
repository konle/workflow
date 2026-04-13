//! Resolve HTTP task templates against a merged JSON context.
//!
//! - **Task template `form`** (HTTP task designer defaults): each row is resolved **only** against
//!   the incoming `ctx` (tenant / workflow / instance / node merge). The resulting key-value map
//!   is merged **on top of** `ctx` (same keys **overwritten**) to form `effective_ctx`.
//! - **`url` / `headers` / `body`**: resolved against `effective_ctx`, so `Variable` rows can bind
//!   keys supplied only via task `form` (e.g. body `{{password}}` + form `password: "123"`).
//! - **Snapshot `form` field**: still the task-`form` rows evaluated against **base `ctx` only**
//!   (the overlay map), not `effective_ctx`, so it does not reflect keys invented only by overlay.
//! - **Per-row types** (`headers` / `body` / `form` rows): `String` / `Number` / `Bool` / `Json` are
//!   literal; only `Variable` runs `{{path}}` template substitution (including mixed text).

use crate::shared::form::{Form, FormValue, FormValueType};
use crate::task::entity::task_definition::{HttpMethod, TaskHttpTemplate};
use serde_json::{json, Map, Value as JsonValue};

fn get_by_path(ctx: &JsonValue, path: &str) -> Option<JsonValue> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    let mut cur = ctx;
    for seg in path.split('.').filter(|s| !s.is_empty()) {
        cur = cur.get(seg)?;
    }
    Some(cur.clone())
}

/// Replace `{{ key.path }}` segments using dot-path lookup in `ctx`. Missing keys keep the original segment.
pub fn resolve_template_placeholders(s: &str, ctx: &JsonValue) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut rest = s;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        let Some(end) = rest.find("}}") else {
            out.push_str("{{");
            out.push_str(rest);
            return out;
        };
        let key = rest[..end].trim();
        let resolved = get_by_path(ctx, key).map(|v| match v {
            JsonValue::String(s) => s,
            JsonValue::Null => String::new(),
            other => other.to_string(),
        });
        match resolved {
            Some(r) => out.push_str(&r),
            None => {
                out.push_str("{{");
                out.push_str(key);
                out.push_str("}}");
            }
        }
        rest = &rest[end + 2..];
    }
    out.push_str(rest);
    out
}

fn resolve_form_to_json(form: &Form, ctx: &JsonValue) -> JsonValue {
    match form.value_type {
        FormValueType::Variable => match &form.value {
            FormValue::String(s) => {
                JsonValue::String(resolve_template_placeholders(s, ctx))
            }
            _ => JsonValue::Null,
        },
        FormValueType::String => match &form.value {
            FormValue::String(s) => JsonValue::String(s.clone()),
            FormValue::Number(n) => JsonValue::Number(serde_json::Number::from_f64(*n).unwrap_or(0.into())),
            FormValue::Bool(b) => JsonValue::Bool(*b),
            FormValue::Json(j) => j.clone(),
        },
        FormValueType::Number => match &form.value {
            FormValue::Number(n) => JsonValue::Number(serde_json::Number::from_f64(*n).unwrap_or(0.into())),
            FormValue::String(s) => {
                if let Ok(n) = s.parse::<f64>() {
                    JsonValue::Number(serde_json::Number::from_f64(n).unwrap_or(0.into()))
                } else {
                    JsonValue::String(s.clone())
                }
            }
            _ => JsonValue::Null,
        },
        FormValueType::Bool => match &form.value {
            FormValue::Bool(b) => JsonValue::Bool(*b),
            FormValue::String(s) => JsonValue::String(s.clone()),
            _ => JsonValue::Null,
        },
        FormValueType::Json => match &form.value {
            FormValue::Json(j) => j.clone(),
            FormValue::String(s) => JsonValue::String(s.clone()),
            _ => JsonValue::Null,
        },
    }
}

/// `base` object extended with `layer` entries; **layer overwrites** existing keys (task `form` wins).
fn merge_ctx_with_task_form_layer(base: &JsonValue, layer: &Map<String, JsonValue>) -> JsonValue {
    if layer.is_empty() {
        return base.clone();
    }
    let mut map = base.as_object().cloned().unwrap_or_default();
    for (k, v) in layer {
        map.insert(k.clone(), v.clone());
    }
    JsonValue::Object(map)
}

/// Build the canonical **resolved** HTTP request snapshot: `url`, `method`, `headers`, `body`, optional `form`.
pub fn resolved_http_request_snapshot(template: &TaskHttpTemplate, ctx: &JsonValue) -> JsonValue {
    let form_layer: Map<String, JsonValue> = template
        .form
        .iter()
        .map(|f| (f.key.clone(), resolve_form_to_json(f, ctx)))
        .collect();

    let effective_ctx = merge_ctx_with_task_form_layer(ctx, &form_layer);

    let url = resolve_template_placeholders(&template.url, &effective_ctx);
    let method_str = format!("{:?}", template.method);

    let headers: Map<String, JsonValue> = template
        .headers
        .iter()
        .map(|f| (f.key.clone(), resolve_form_to_json(f, &effective_ctx)))
        .collect();

    let body: Map<String, JsonValue> = template
        .body
        .iter()
        .map(|f| (f.key.clone(), resolve_form_to_json(f, &effective_ctx)))
        .collect();
    let body_v = if body.is_empty() {
        JsonValue::Null
    } else {
        JsonValue::Object(body)
    };

    let form_v = if form_layer.is_empty() {
        JsonValue::Null
    } else {
        JsonValue::Object(form_layer)
    };

    json!({
        "url": url,
        "method": method_str,
        "headers": headers,
        "body": body_v,
        "form": form_v,
    })
}

/// Pointer path for `items_path` (same convention as Parallel plugin).
pub fn items_json_pointer(items_path: &str) -> String {
    if items_path.starts_with('/') {
        items_path.to_string()
    } else {
        format!("/{}", items_path.replace('.', "/"))
    }
}

/// Merge workflow `instance.context` with one array element under `item_alias` (for Parallel children).
pub fn context_with_parallel_item(
    instance_context: &JsonValue,
    items_path: &str,
    item_alias: &str,
    item_index: usize,
) -> JsonValue {
    let ptr = items_json_pointer(items_path);
    let mut base_map = instance_context
        .as_object()
        .cloned()
        .unwrap_or_default();

    if let Some(JsonValue::Array(arr)) = instance_context.pointer(&ptr) {
        if let Some(item) = arr.get(item_index) {
            base_map.insert(item_alias.to_string(), item.clone());
        }
    }

    JsonValue::Object(base_map)
}

pub fn parse_method_str(s: &str) -> HttpMethod {
    match s.trim().to_ascii_lowercase().as_str() {
        "post" => HttpMethod::Post,
        "put" => HttpMethod::Put,
        "delete" => HttpMethod::Delete,
        "head" => HttpMethod::Head,
        _ => HttpMethod::Get,
    }
}

/// Interpret `task_instance.input` as a resolved snapshot, or build from template + `ctx`.
pub fn effective_http_request(
    task_instance: &crate::task::entity::task_definition::TaskInstanceEntity,
    config: &TaskHttpTemplate,
    fallback_ctx: &JsonValue,
) -> (JsonValue, String, HttpMethod, serde_json::Map<String, JsonValue>, Option<JsonValue>) {
    let snapshot = if let Some(inp) = &task_instance.input {
        if inp
            .get("url")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .is_some()
        {
            inp.clone()
        } else {
            resolved_http_request_snapshot(config, fallback_ctx)
        }
    } else {
        resolved_http_request_snapshot(config, fallback_ctx)
    };

    let url = snapshot
        .get("url")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default();

    let method = snapshot
        .get("method")
        .and_then(|v| v.as_str())
        .map(parse_method_str)
        .unwrap_or_else(|| config.method.clone());

    let headers_obj = snapshot
        .get("headers")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let body = snapshot.get("body").cloned();
    let body = match body {
        Some(JsonValue::Null) | None => None,
        Some(JsonValue::Object(m)) if m.is_empty() => None,
        Some(o) => Some(o),
    };

    (snapshot, url, method, headers_obj, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::form::{Form, FormValue, FormValueType};
    use crate::task::entity::task_definition::{HttpMethod, TaskHttpTemplate};

    fn form(key: &str, value: FormValue, value_type: FormValueType) -> Form {
        Form {
            key: key.to_string(),
            value,
            value_type,
            description: None,
        }
    }

    fn tpl(body: Vec<Form>) -> TaskHttpTemplate {
        tpl_with_form(body, vec![])
    }

    fn tpl_with_form(body: Vec<Form>, form: Vec<Form>) -> TaskHttpTemplate {
        TaskHttpTemplate {
            url: "http://example.test/x".to_string(),
            method: HttpMethod::Post,
            headers: vec![],
            body,
            form,
            retry_count: 0,
            retry_delay: 0,
            timeout: 0,
            success_condition: None,
        }
    }

    #[test]
    fn variable_row_substitutes_braced_placeholders() {
        let template = tpl(vec![form(
            "name",
            FormValue::String("{{name}}".into()),
            FormValueType::Variable,
        )]);
        let ctx = json!({ "name": "WOAA2" });
        let snap = resolved_http_request_snapshot(&template, &ctx);
        assert_eq!(snap["body"]["name"], json!("WOAA2"));
    }

    #[test]
    fn variable_row_supports_mixed_template_text() {
        let template = tpl(vec![form(
            "greeting",
            FormValue::String("my name is {{name}}".into()),
            FormValueType::Variable,
        )]);
        let ctx = json!({ "name": "Ada" });
        let snap = resolved_http_request_snapshot(&template, &ctx);
        assert_eq!(snap["body"]["greeting"], json!("my name is Ada"));
    }

    #[test]
    fn string_row_is_literal_and_does_not_resolve_placeholders() {
        let template = tpl(vec![form(
            "raw",
            FormValue::String("{{name}}".into()),
            FormValueType::String,
        )]);
        let ctx = json!({ "name": "WOAA2" });
        let snap = resolved_http_request_snapshot(&template, &ctx);
        assert_eq!(snap["body"]["raw"], json!("{{name}}"));
    }

    #[test]
    fn body_variable_fills_from_task_form_when_missing_in_base_ctx() {
        let template = tpl_with_form(
            vec![form(
                "password",
                FormValue::String("{{password}}".into()),
                FormValueType::Variable,
            )],
            vec![form(
                "password",
                FormValue::String("123".into()),
                FormValueType::String,
            )],
        );
        let ctx = json!({ "name": "u1" });
        let snap = resolved_http_request_snapshot(&template, &ctx);
        assert_eq!(snap["body"]["password"], json!("123"));
        assert_eq!(snap["form"]["password"], json!("123"));
    }

    #[test]
    fn task_form_overrides_base_ctx_for_body_resolution() {
        let template = tpl_with_form(
            vec![form(
                "password",
                FormValue::String("{{password}}".into()),
                FormValueType::Variable,
            )],
            vec![form(
                "password",
                FormValue::String("from_form".into()),
                FormValueType::String,
            )],
        );
        let ctx = json!({ "password": "from_instance" });
        let snap = resolved_http_request_snapshot(&template, &ctx);
        assert_eq!(snap["body"]["password"], json!("from_form"));
    }
}
