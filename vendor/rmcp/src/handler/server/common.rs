//! Common utilities shared between tool and prompt handlers

use std::{any::TypeId, collections::HashMap, sync::Arc};

use schemars::JsonSchema;

use crate::{
    RoleServer, model::JsonObject, schemars::generate::SchemaSettings, service::RequestContext,
};

/// A shortcut for generating a JSON schema for a type.
pub fn schema_for_type<T: JsonSchema>() -> JsonObject {
    // explicitly to align json schema version to official specifications.
    // https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/2025-03-26/schema.json
    // TODO: update to 2020-12 waiting for the mcp spec update
    let mut settings = SchemaSettings::draft07();
    let generator = settings.into_generator();
    let schema = generator.into_root_schema_for::<T>();
    let object = serde_json::to_value(schema).expect("failed to serialize schema");
    let mut obj = match object {
        serde_json::Value::Object(object) => object,
        _ => panic!(
            "Schema serialization produced non-object value: expected JSON object but got {:?}",
            object
        ),
    };

    // Transform schema to be Gemini API compatible
    transform_for_gemini(&mut obj);
    obj
}

/// Transform JSON Schema to be compatible with Gemini's function calling API.
pub(crate) fn transform_for_gemini_elicitation(obj: &mut serde_json::Map<String, serde_json::Value>) {
    transform_for_gemini(obj);
}

fn transform_for_gemini(obj: &mut serde_json::Map<String, serde_json::Value>) {
    use serde_json::Value;

    // Step 1: Extract and resolve all $ref definitions
    let definitions = obj.get("definitions").cloned();
    if let Some(Value::Object(defs)) = definitions {
        let mut value = Value::Object(obj.clone());
        resolve_all_refs(&mut value, &defs);
        if let Value::Object(resolved) = value {
            *obj = resolved;
        }
    }

    // Step 2: Remove definitions section (Gemini doesn't support it)
    obj.remove("definitions");

    // Step 3: Remove Gemini-unsupported fields
    obj.remove("nullable");
    obj.remove("$schema");
    obj.remove("title");
    obj.remove("format");
    obj.remove("minimum");
    obj.remove("maximum");
    obj.remove("minLength");
    obj.remove("maxLength");
    obj.remove("pattern");
    obj.remove("minItems");
    obj.remove("maxItems");
    obj.remove("uniqueItems");
    obj.remove("minProperties");
    obj.remove("maxProperties");

    // Step 4: Transform type arrays to single type
    if let Some(Value::Array(type_array)) = obj.get("type") {
        if let Some(non_null_type) = type_array
            .iter()
            .find(|t| t.as_str() != Some("null"))
            .cloned()
        {
            obj.insert("type".to_string(), non_null_type);
        }
    }

    // Step 5: Transform allOf with single element (flatten it)
    if let Some(Value::Array(all_of)) = obj.get("allOf").cloned() {
        if all_of.len() == 1 {
            if let Value::Object(single_opt) = &all_of[0] {
                obj.remove("allOf");
                for (key, value) in single_opt {
                    if key != "description" || !obj.contains_key("description") {
                        obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }

    // Step 6: Transform anyOf patterns where one option is null
    if let Some(Value::Array(any_of)) = obj.get("anyOf").cloned() {
        let non_null_options: Vec<_> = any_of
            .iter()
            .filter(|opt| {
                !matches!(opt, Value::Object(o) if o.get("type") == Some(&Value::String("null".to_string())))
            })
            .cloned()
            .collect();

        if non_null_options.len() == 1 {
            if let Value::Object(single_opt) = &non_null_options[0] {
                obj.remove("anyOf");
                for (key, value) in single_opt {
                    if key != "description" || !obj.contains_key("description") {
                        obj.insert(key.clone(), value.clone());
                    }
                }
            }
        } else if !non_null_options.is_empty() {
            obj.insert("anyOf".to_string(), Value::Array(non_null_options));
        }
    }

    // Step 7: Recursively transform nested objects
    for value in obj.values_mut() {
        transform_value_for_gemini(value);
    }
}

/// Resolve all $ref fields by inlining the definitions
fn resolve_all_refs(value: &mut serde_json::Value, definitions: &serde_json::Map<String, serde_json::Value>) {
    use serde_json::Value;

    match value {
        Value::Object(obj) => {
            // Check if this object has a $ref
            if let Some(Value::String(ref_path)) = obj.get("$ref") {
                // Extract the definition name from "#/definitions/Name"
                if let Some(def_name) = ref_path.strip_prefix("#/definitions/") {
                    if let Some(definition) = definitions.get(def_name) {
                        // Preserve description if it exists
                        let description = obj.get("description").cloned();

                        // Replace the entire object with the definition
                        *obj = match definition.clone() {
                            Value::Object(def_obj) => def_obj,
                            _ => return,
                        };

                        // Restore description if it was present
                        if let Some(desc) = description {
                            if !obj.contains_key("description") {
                                obj.insert("description".to_string(), desc);
                            }
                        }
                    }
                }
            }

            // Recursively resolve refs in nested objects
            for val in obj.values_mut() {
                resolve_all_refs(val, definitions);
            }
        },
        Value::Array(arr) => {
            for item in arr {
                resolve_all_refs(item, definitions);
            }
        },
        _ => {},
    }
}

fn transform_value_for_gemini(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(obj) => transform_for_gemini(obj),
        serde_json::Value::Array(arr) => {
            for item in arr {
                transform_value_for_gemini(item);
            }
        },
        _ => {},
    }
}

/// Call [`schema_for_type`] with a cache
pub fn cached_schema_for_type<T: JsonSchema + std::any::Any>() -> Arc<JsonObject> {
    thread_local! {
        static CACHE_FOR_TYPE: std::sync::RwLock<HashMap<TypeId, Arc<JsonObject>>> = Default::default();
    };
    CACHE_FOR_TYPE.with(|cache| {
        if let Some(x) = cache
            .read()
            .expect("schema cache lock poisoned")
            .get(&TypeId::of::<T>())
        {
            x.clone()
        } else {
            let schema = schema_for_type::<T>();
            let schema = Arc::new(schema);
            cache
                .write()
                .expect("schema cache lock poisoned")
                .insert(TypeId::of::<T>(), schema.clone());
            schema
        }
    })
}

/// Trait for extracting parts from a context, unifying tool and prompt extraction
pub trait FromContextPart<C>: Sized {
    fn from_context_part(context: &mut C) -> Result<Self, crate::ErrorData>;
}

/// Common extractors that can be used by both tool and prompt handlers
impl<C> FromContextPart<C> for RequestContext<RoleServer>
where
    C: AsRequestContext,
{
    fn from_context_part(context: &mut C) -> Result<Self, crate::ErrorData> {
        Ok(context.as_request_context().clone())
    }
}

impl<C> FromContextPart<C> for tokio_util::sync::CancellationToken
where
    C: AsRequestContext,
{
    fn from_context_part(context: &mut C) -> Result<Self, crate::ErrorData> {
        Ok(context.as_request_context().ct.clone())
    }
}

impl<C> FromContextPart<C> for crate::model::Extensions
where
    C: AsRequestContext,
{
    fn from_context_part(context: &mut C) -> Result<Self, crate::ErrorData> {
        Ok(context.as_request_context().extensions.clone())
    }
}

pub struct Extension<T>(pub T);

impl<C, T> FromContextPart<C> for Extension<T>
where
    C: AsRequestContext,
    T: Send + Sync + 'static + Clone,
{
    fn from_context_part(context: &mut C) -> Result<Self, crate::ErrorData> {
        let extension = context
            .as_request_context()
            .extensions
            .get::<T>()
            .cloned()
            .ok_or_else(|| {
                crate::ErrorData::invalid_params(
                    format!("missing extension {}", std::any::type_name::<T>()),
                    None,
                )
            })?;
        Ok(Extension(extension))
    }
}

impl<C> FromContextPart<C> for crate::Peer<RoleServer>
where
    C: AsRequestContext,
{
    fn from_context_part(context: &mut C) -> Result<Self, crate::ErrorData> {
        Ok(context.as_request_context().peer.clone())
    }
}

impl<C> FromContextPart<C> for crate::model::Meta
where
    C: AsRequestContext,
{
    fn from_context_part(context: &mut C) -> Result<Self, crate::ErrorData> {
        let request_context = context.as_request_context_mut();
        let mut meta = crate::model::Meta::default();
        std::mem::swap(&mut meta, &mut request_context.meta);
        Ok(meta)
    }
}

pub struct RequestId(pub crate::model::RequestId);

impl<C> FromContextPart<C> for RequestId
where
    C: AsRequestContext,
{
    fn from_context_part(context: &mut C) -> Result<Self, crate::ErrorData> {
        Ok(RequestId(context.as_request_context().id.clone()))
    }
}

/// Trait for types that can provide access to RequestContext
pub trait AsRequestContext {
    fn as_request_context(&self) -> &RequestContext<RoleServer>;
    fn as_request_context_mut(&mut self) -> &mut RequestContext<RoleServer>;
}
