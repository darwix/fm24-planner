use serde_json::Value;

/// Turns a JSON filter object into the query string the GET search takes.
/// Arrays become comma lists, booleans 1 or 0, nulls are skipped.
pub fn query_from_body(body: &Value) -> String {
    let mut out = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in body.as_object().into_iter().flatten() {
        let text = match v {
            Value::Null => continue,
            Value::Bool(b) => (*b as u8).to_string(),
            Value::String(s) => s.clone(),
            Value::Array(a) => a
                .iter()
                .map(|x| x.as_str().map_or_else(|| x.to_string(), str::to_string))
                .collect::<Vec<_>>()
                .join(","),
            other => other.to_string(),
        };
        out.append_pair(k, &text);
    }
    out.finish()
}
