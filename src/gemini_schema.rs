use serde_json::{Map, Value};

/// Transform JSON Schema to be compatible with Gemini's function calling API.
///
/// Gemini's API uses a subset of JSON Schema and doesn't support:
/// - Type arrays (e.g., ["string", "null"])
/// - anyOf/oneOf/allOf
/// - nullable keyword
///
/// For optional fields, just omit them from the required array.
pub fn make_gemini_compatible(mut schema: Map<String, Value>) -> Map<String, Value> {
    transform_schema_value(&mut Value::Object(schema.clone()));
    schema
}

fn transform_schema_value(value: &mut Value) {
    match value {
        Value::Object(obj) => transform_object(obj),
        Value::Array(arr) => {
            for item in arr {
                transform_schema_value(item);
            }
        },
        _ => {},
    }
}

fn transform_object(obj: &mut Map<String, Value>) {
    // Remove nullable field if present
    obj.remove("nullable");

    // Transform type arrays to single type
    if let Some(Value::Array(type_array)) = obj.get("type") {
        // Find first non-null type
        if let Some(non_null_type) = type_array
            .iter()
            .find(|t| t.as_str() != Some("null"))
            .cloned()
        {
            obj.insert("type".to_string(), non_null_type);
        }
    }

    // Transform anyOf patterns where one option is null
    if let Some(Value::Array(any_of)) = obj.get("anyOf") {
        let non_null_options: Vec<_> = any_of
            .iter()
            .filter(|opt| !is_null_schema(opt))
            .cloned()
            .collect();

        if non_null_options.len() == 1 {
            // Single non-null option - flatten it
            if let Value::Object(single_opt) = &non_null_options[0] {
                obj.remove("anyOf");
                for (key, value) in single_opt {
                    if key != "description" || !obj.contains_key("description") {
                        obj.insert(key.clone(), value.clone());
                    }
                }
            }
        } else if !non_null_options.is_empty() {
            // Multiple non-null options - keep anyOf but remove null
            obj.insert("anyOf".to_string(), Value::Array(non_null_options));
        }
    }

    // Recursively transform nested objects
    for value in obj.values_mut() {
        transform_schema_value(value);
    }
}

fn is_null_schema(value: &Value) -> bool {
    match value {
        Value::Object(obj) => {
            obj.get("type") == Some(&Value::String("null".to_string()))
                || obj.get("nullable") == Some(&Value::Bool(true))
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_type_array_conversion() {
        let mut schema = serde_json::from_value(json!({
            "type": ["string", "null"]
        }))
        .unwrap();

        schema = make_gemini_compatible(schema);

        assert_eq!(schema.get("type"), Some(&Value::String("string".to_string())));
    }

    #[test]
    fn test_anyof_with_null() {
        let mut schema = serde_json::from_value(json!({
            "anyOf": [
                {"$ref": "#/definitions/Priority"},
                {"type": "null"}
            ],
            "description": "Priority level"
        }))
        .unwrap();

        schema = make_gemini_compatible(schema);

        assert_eq!(schema.get("$ref"), Some(&Value::String("#/definitions/Priority".to_string())));
        assert_eq!(schema.get("anyOf"), None);
    }

    #[test]
    fn test_nullable_removal() {
        let mut schema = serde_json::from_value(json!({
            "type": "string",
            "nullable": true
        }))
        .unwrap();

        schema = make_gemini_compatible(schema);

        assert_eq!(schema.get("nullable"), None);
    }
}
