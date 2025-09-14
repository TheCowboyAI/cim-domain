// FP-aligned JSON Schema implementations for domain types
//
// This module provides JsonSchema implementations that reflect our FP model:
// - Algebraic Data Types (ADTs) with sum and product types
// - Entity monad structure
// - Mealy state machines
// - Domain marker traits
// - Specification patterns

use schemars::{JsonSchema, schema::{Schema, SchemaObject, InstanceType, ObjectValidation}};
use schemars::schema_for;
use std::marker::PhantomData;
use uuid::Uuid;
use crate::{EntityId, DomainCid};
use crate::cid::CidGeneric;

// ============================================================================
// ENTITY ID SCHEMA - Type-safe phantom type schemas
// ============================================================================

impl<T> JsonSchema for EntityId<T> 
where
    T: 'static,
{
    fn schema_name() -> String {
        format!("EntityId_{}", std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap_or("Unknown"))
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        // EntityId is a newtype wrapper around Uuid
        // Schema reflects: { "value": "uuid-string" }
        let mut schema = SchemaObject::default();
        
        schema.instance_type = Some(InstanceType::Object.into());
        
        let mut object = ObjectValidation::default();
        object.properties.insert(
            "value".to_owned(),
            gen.subschema_for::<String>(), // UUID as string
        );
        object.required.insert("value".to_owned());
        
        // Add phantom type information as metadata
        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some(Self::schema_name()),
            description: Some(format!(
                "Type-safe entity ID with phantom type marker for {}",
                std::any::type_name::<T>()
            )),
            ..Default::default()
        }));
        
        schema.object = Some(object);
        Schema::Object(schema)
    }
}

// ============================================================================
// DOMAIN CID SCHEMA - Content-addressed identifiers with domain context
// ============================================================================

impl JsonSchema for DomainCid {
    fn schema_name() -> String {
        "DomainCid".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = SchemaObject::default();
        
        schema.instance_type = Some(InstanceType::Object.into());
        
        let mut object = ObjectValidation::default();
        
        // Inner CID (as string when serialized)
        object.properties.insert(
            "inner".to_owned(),
            Schema::Object({
                let mut s = SchemaObject::default();
                s.instance_type = Some(InstanceType::String.into());
                s.string = Some(Box::new(schemars::schema::StringValidation {
                    pattern: Some("^[a-zA-Z0-9]+$".to_string()),
                    min_length: Some(1),
                    ..Default::default()
                }));
                s
            }),
        );
        
        // Optional domain context
        object.properties.insert(
            "domain".to_owned(),
            gen.subschema_for::<Option<String>>(),
        );
        
        // Content type
        object.properties.insert(
            "content_type".to_owned(),
            gen.subschema_for::<crate::ContentType>(),
        );
        
        object.required.insert("inner".to_owned());
        object.required.insert("content_type".to_owned());
        
        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some("DomainCid".to_string()),
            description: Some("Domain-specific CID with context and content type".to_string()),
            ..Default::default()
        }));
        
        schema.object = Some(object);
        Schema::Object(schema)
    }
}

impl<const N: usize> JsonSchema for CidGeneric<N> {
    fn schema_name() -> String {
        format!("CidGeneric_{}", N)
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        // CidGeneric serializes as a string
        let mut schema = SchemaObject::default();
        
        schema.instance_type = Some(InstanceType::String.into());
        
        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some(format!("CidGeneric<{}>", N)),
            description: Some(format!("Generic CID with {} byte hash", N)),
            ..Default::default()
        }));
        
        schema.string = Some(Box::new(schemars::schema::StringValidation {
            pattern: Some("^[a-zA-Z0-9]+$".to_string()),
            min_length: Some(1),
            ..Default::default()
        }));
        
        Schema::Object(schema)
    }
}

// ============================================================================
// ENTITY MONAD SCHEMA - Monadic container with components
// ============================================================================

impl<A> JsonSchema for crate::EntityMonad<A> 
where
    A: JsonSchema + 'static,
{
    fn schema_name() -> String {
        format!("EntityMonad_{}", A::schema_name())
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = SchemaObject::default();
        
        schema.instance_type = Some(InstanceType::Object.into());
        
        let mut object = ObjectValidation::default();
        
        // Entity has id and components
        object.properties.insert(
            "id".to_owned(),
            gen.subschema_for::<EntityId<A>>(),
        );
        object.properties.insert(
            "components".to_owned(),
            gen.subschema_for::<crate::Components<A>>(),
        );
        
        object.required.insert("id".to_owned());
        object.required.insert("components".to_owned());
        
        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some(Self::schema_name()),
            description: Some(format!(
                "Entity monad wrapping {} - provides monadic composition for domain objects",
                A::schema_name()
            )),
            ..Default::default()
        }));
        
        schema.object = Some(object);
        Schema::Object(schema)
    }
}

// ============================================================================
// COMPONENTS SCHEMA - Type-erased component storage
// ============================================================================

impl<A> JsonSchema for crate::Components<A> 
where
    A: 'static,
{
    fn schema_name() -> String {
        format!("Components_{}", std::any::type_name::<A>()
            .split("::")
            .last()
            .unwrap_or("Unknown"))
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = SchemaObject::default();
        
        // Components is an opaque map of type-erased values
        schema.instance_type = Some(InstanceType::Object.into());
        
        // Allow any additional properties since components are dynamic
        let mut object = ObjectValidation::default();
        object.additional_properties = Some(Box::new(Schema::Bool(true)));
        
        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some(Self::schema_name()),
            description: Some("Dynamic component storage for ECS pattern".to_string()),
            ..Default::default()
        }));
        
        schema.object = Some(object);
        Schema::Object(schema)
    }
}

// ============================================================================
// AGGREGATE STATE SCHEMA - Sum type for state machines
// ============================================================================

/// Schema for aggregate states (algebraic data type)
pub fn aggregate_state_schema<S: JsonSchema>(
    states: Vec<&str>,
    gen: &mut schemars::gen::SchemaGenerator,
) -> Schema {
    let mut schema = SchemaObject::default();
    
    // Use oneOf for sum types (algebraic data types)
    let variants: Vec<Schema> = states.iter().map(|state| {
        let mut variant = SchemaObject::default();
        variant.instance_type = Some(InstanceType::Object.into());
        
        let mut object = ObjectValidation::default();
        object.properties.insert(
            "state".to_owned(),
            Schema::Object({
                let mut s = SchemaObject::default();
                s.instance_type = Some(InstanceType::String.into());
                s.enum_values = Some(vec![serde_json::json!(state)]);
                s
            }),
        );
        object.required.insert("state".to_owned());
        
        variant.object = Some(object);
        Schema::Object(variant)
    }).collect();
    
    schema.subschemas = Some(Box::new(schemars::schema::SubschemaValidation {
        one_of: Some(variants),
        ..Default::default()
    }));
    
    schema.metadata = Some(Box::new(schemars::schema::Metadata {
        title: Some("AggregateState".to_string()),
        description: Some("Algebraic data type representing aggregate state machine states".to_string()),
        ..Default::default()
    }));
    
    Schema::Object(schema)
}

// ============================================================================
// MEALY STATE MACHINE SCHEMA
// ============================================================================

/// Schema for Mealy state machines
pub fn mealy_machine_schema(
    states: Vec<&str>,
    inputs: Vec<&str>,
    outputs: Vec<&str>,
) -> Schema {
    let mut schema = SchemaObject::default();
    
    schema.instance_type = Some(InstanceType::Object.into());
    
    let mut object = ObjectValidation::default();
    
    // Transition table: (State, Input) -> State
    object.properties.insert(
        "transitions".to_owned(),
        Schema::Object({
            let mut s = SchemaObject::default();
            s.instance_type = Some(InstanceType::Object.into());
            s.metadata = Some(Box::new(schemars::schema::Metadata {
                description: Some("State transition function: (State, Input) -> State".to_string()),
                ..Default::default()
            }));
            s
        }),
    );
    
    // Output table: (State, Input) -> Output
    object.properties.insert(
        "outputs".to_owned(),
        Schema::Object({
            let mut s = SchemaObject::default();
            s.instance_type = Some(InstanceType::Object.into());
            s.metadata = Some(Box::new(schemars::schema::Metadata {
                description: Some("Output function: (State, Input) -> Output".to_string()),
                ..Default::default()
            }));
            s
        }),
    );
    
    // Current state
    object.properties.insert(
        "current_state".to_owned(),
        Schema::Object({
            let mut s = SchemaObject::default();
            s.instance_type = Some(InstanceType::String.into());
            s.enum_values = Some(states.iter().map(|s| serde_json::json!(s)).collect());
            s
        }),
    );
    
    object.required.insert("transitions".to_owned());
    object.required.insert("outputs".to_owned());
    object.required.insert("current_state".to_owned());
    
    schema.metadata = Some(Box::new(schemars::schema::Metadata {
        title: Some("MealyStateMachine".to_string()),
        description: Some("Mealy state machine where outputs depend on both state and input".to_string()),
        ..Default::default()
    }));
    
    schema.object = Some(object);
    Schema::Object(schema)
}

// ============================================================================
// SPECIFICATION SCHEMA - Boolean algebra for validation
// ============================================================================

/// Schema for specifications (validation rules)
pub fn specification_schema<T: JsonSchema>() -> Schema {
    let mut schema = SchemaObject::default();
    
    // Specifications form a boolean algebra (and, or, not)
    schema.subschemas = Some(Box::new(schemars::schema::SubschemaValidation {
        one_of: Some(vec![
            // Leaf specification
            Schema::Object({
                let mut s = SchemaObject::default();
                s.instance_type = Some(InstanceType::Object.into());
                let mut obj = ObjectValidation::default();
                obj.properties.insert("rule".to_owned(), Schema::Bool(true));
                obj.required.insert("rule".to_owned());
                s.object = Some(obj);
                s
            }),
            // AND composition
            Schema::Object({
                let mut s = SchemaObject::default();
                s.instance_type = Some(InstanceType::Object.into());
                let mut obj = ObjectValidation::default();
                obj.properties.insert("and".to_owned(), Schema::Object({
                    let mut arr = SchemaObject::default();
                    arr.instance_type = Some(InstanceType::Array.into());
                    arr
                }));
                obj.required.insert("and".to_owned());
                s.object = Some(obj);
                s
            }),
            // OR composition
            Schema::Object({
                let mut s = SchemaObject::default();
                s.instance_type = Some(InstanceType::Object.into());
                let mut obj = ObjectValidation::default();
                obj.properties.insert("or".to_owned(), Schema::Object({
                    let mut arr = SchemaObject::default();
                    arr.instance_type = Some(InstanceType::Array.into());
                    arr
                }));
                obj.required.insert("or".to_owned());
                s.object = Some(obj);
                s
            }),
            // NOT negation
            Schema::Object({
                let mut s = SchemaObject::default();
                s.instance_type = Some(InstanceType::Object.into());
                let mut obj = ObjectValidation::default();
                obj.properties.insert("not".to_owned(), Schema::Bool(true));
                obj.required.insert("not".to_owned());
                s.object = Some(obj);
                s
            }),
        ]),
        ..Default::default()
    }));
    
    schema.metadata = Some(Box::new(schemars::schema::Metadata {
        title: Some(format!("Specification_{}", T::schema_name())),
        description: Some("Boolean algebra for composable validation rules".to_string()),
        ..Default::default()
    }));
    
    Schema::Object(schema)
}

// ============================================================================
// DOMAIN MARKER TRAIT SCHEMAS
// ============================================================================

/// Schema for domain marker traits (ValueObject, Entity, Aggregate, etc.)
pub fn domain_trait_schema(trait_name: &str) -> Schema {
    let mut schema = SchemaObject::default();
    
    schema.instance_type = Some(InstanceType::Object.into());
    
    let mut object = ObjectValidation::default();
    object.properties.insert(
        "_domain_trait".to_owned(),
        Schema::Object({
            let mut s = SchemaObject::default();
            s.instance_type = Some(InstanceType::String.into());
            s.enum_values = Some(vec![serde_json::json!(trait_name)]);
            s
        }),
    );
    
    // Allow additional properties for the actual domain object
    object.additional_properties = Some(Box::new(Schema::Bool(true)));
    
    schema.metadata = Some(Box::new(schemars::schema::Metadata {
        title: Some(trait_name.to_string()),
        description: Some(match trait_name {
            "ValueObject" => "Immutable object compared by value, not identity",
            "DomainEntity" => "Object with identity that persists beyond its attributes",
            "Aggregate" => "Consistency boundary with Mealy state machine behavior",
            "Policy" => "Pure business rule function",
            "Saga" => "Composed aggregate managing distributed transactions",
            _ => "Domain concept marker",
        }.to_string()),
        ..Default::default()
    }));
    
    schema.object = Some(object);
    Schema::Object(schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;
    
    #[test]
    fn test_entity_id_schema() {
        #[derive(JsonSchema)]
        struct TestMarker;
        
        let schema = schema_for!(EntityId<TestMarker>);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        println!("EntityId schema:\n{}", json);
        
        assert!(json.contains("EntityId"));
        assert!(json.contains("phantom type marker"));
    }
    
    #[test]
    fn test_domain_cid_schema() {
        let schema = schema_for!(DomainCid);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        println!("DomainCid schema:\n{}", json);
        
        assert!(json.contains("DomainCid"));
        assert!(json.contains("content_type"));
    }
    
    #[test]
    fn test_domain_trait_schemas() {
        let value_object = domain_trait_schema("ValueObject");
        let entity = domain_trait_schema("DomainEntity");
        let aggregate = domain_trait_schema("Aggregate");
        
        println!("ValueObject schema:\n{}", 
                 serde_json::to_string_pretty(&value_object).unwrap());
        println!("Entity schema:\n{}", 
                 serde_json::to_string_pretty(&entity).unwrap());
        println!("Aggregate schema:\n{}", 
                 serde_json::to_string_pretty(&aggregate).unwrap());
    }
    
    #[test]
    fn test_mealy_machine_schema() {
        let schema = mealy_machine_schema(
            vec!["Idle", "Processing", "Complete"],
            vec!["Start", "Process", "Finish"],
            vec!["Started", "InProgress", "Done"],
        );
        
        let json = serde_json::to_string_pretty(&schema).unwrap();
        println!("Mealy machine schema:\n{}", json);
        
        assert!(json.contains("transitions"));
        assert!(json.contains("outputs"));
        assert!(json.contains("current_state"));
    }
}