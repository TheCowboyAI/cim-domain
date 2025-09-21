// FP-aligned JSON Schema implementations for domain types
//
// This module provides JsonSchema implementations that reflect our FP model:
// - Algebraic Data Types (ADTs) with sum and product types
// - Entity monad structure
// - Mealy state machines
// - Domain marker traits
// - Specification patterns

//! JSON Schema generators for FP primitives used across the domain library.
//!
//! Includes schemas for:
//! - `EntityId<T>` phantom-typed identifiers
//! - Domain CIDs (`DomainCid`)
//! - The Entity monad and type-erased components
//! - Mealy state machine and specification helpers
//! - Marker trait wrappers for ValueObject/Entity/Aggregate/etc.

use crate::{
    domain_path::DomainPath,
    subject::{Subject, SubjectPattern},
    DomainCid, EntityId,
};
use schemars::{
    schema::{InstanceType, ObjectValidation, Schema, SchemaObject},
    JsonSchema,
};

// ============================================================================
// ENTITY ID SCHEMA - Type-safe phantom type schemas
// ============================================================================

impl<T> JsonSchema for EntityId<T>
where
    T: 'static,
{
    fn schema_name() -> String {
        format!(
            "EntityId_{}",
            std::any::type_name::<T>()
                .split("::")
                .last()
                .unwrap_or("Unknown")
        )
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        // EntityId is a newtype wrapper around Uuid
        // Schema reflects: { "value": "uuid-string" }
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            ..Default::default()
        };

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

        schema.object = Some(Box::new(object));
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
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            ..Default::default()
        };

        let mut object = ObjectValidation::default();

        // Inner CID (as string when serialized)
        object.properties.insert(
            "inner".to_owned(),
            Schema::Object({
                SchemaObject {
                    instance_type: Some(InstanceType::String.into()),
                    string: Some(Box::new(schemars::schema::StringValidation {
                        pattern: Some("^[a-zA-Z0-9]+$".to_string()),
                        min_length: Some(1),
                        ..Default::default()
                    })),
                    ..Default::default()
                }
            }),
        );

        // Optional domain context
        object
            .properties
            .insert("domain".to_owned(), gen.subschema_for::<Option<String>>());

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

        schema.object = Some(Box::new(object));
        Schema::Object(schema)
    }
}

// ============================================================================
// SUBJECT SCHEMAS - String representations of the subject algebra
// ============================================================================

impl JsonSchema for Subject {
    fn schema_name() -> String {
        "Subject".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        };

        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some("Subject".to_string()),
            description: Some(
                "Subject string composed of `.` separated validated segments (no whitespace, `*`, `>`, or dots within a segment)."
                    .to_string(),
            ),
            ..Default::default()
        }));

        schema.string = Some(Box::new(schemars::schema::StringValidation {
            pattern: Some("^[^.*>\\s]+(\\.[^.*>\\s]+)*$".to_string()),
            ..Default::default()
        }));

        Schema::Object(schema)
    }
}

impl JsonSchema for SubjectPattern {
    fn schema_name() -> String {
        "SubjectPattern".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        };

        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some("SubjectPattern".to_string()),
            description: Some(
                "Subject pattern supporting `*` (single segment) and a terminal `>` (multi-segment) wildcard."
                    .to_string(),
            ),
            ..Default::default()
        }));

        Schema::Object(schema)
    }
}

impl JsonSchema for DomainPath {
    fn schema_name() -> String {
        "DomainPath".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        };

        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some("DomainPath".to_string()),
            description: Some(
                "Canonical domain path beginning with 'cim.domain' followed by bounded context and facet segments."
                    .to_string(),
            ),
            ..Default::default()
        }));

        schema.string = Some(Box::new(schemars::schema::StringValidation {
            pattern: Some("^cim\\.domain(\\.[a-z0-9_-]+)*$".to_string()),
            ..Default::default()
        }));

        Schema::Object(schema)
    }
}

// CidGeneric JsonSchema implementation removed
// Cannot implement foreign trait for foreign type (orphan rule)
// This should be implemented in cim-ipld or wherever CidGeneric is defined

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
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            ..Default::default()
        };

        let mut object = ObjectValidation::default();

        // Entity has id and components
        object
            .properties
            .insert("id".to_owned(), gen.subschema_for::<EntityId<A>>());
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

        schema.object = Some(Box::new(object));
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
        format!(
            "Components_{}",
            std::any::type_name::<A>()
                .split("::")
                .last()
                .unwrap_or("Unknown")
        )
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            ..Default::default()
        };

        // Allow any additional properties since components are dynamic
        let object = ObjectValidation {
            additional_properties: Some(Box::new(Schema::Bool(true))),
            ..Default::default()
        };

        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some(Self::schema_name()),
            description: Some("Dynamic component storage for ECS pattern".to_string()),
            ..Default::default()
        }));

        schema.object = Some(Box::new(object));
        Schema::Object(schema)
    }
}

// ============================================================================
// AGGREGATE STATE SCHEMA - Sum type for state machines
// ============================================================================

/// Schema for aggregate states (algebraic data type)
pub fn aggregate_state_schema<S: JsonSchema>(
    states: Vec<&str>,
    _gen: &mut schemars::gen::SchemaGenerator,
) -> Schema {
    let mut schema = SchemaObject {
        instance_type: None,
        ..Default::default()
    };

    // Use oneOf for sum types (algebraic data types)
    let variants: Vec<Schema> = states
        .iter()
        .map(|state| {
            let mut variant = SchemaObject {
                instance_type: Some(InstanceType::Object.into()),
                ..Default::default()
            };

            let mut object = ObjectValidation::default();
            object.properties.insert(
                "state".to_owned(),
                Schema::Object({
                    SchemaObject {
                        instance_type: Some(InstanceType::String.into()),
                        enum_values: Some(vec![serde_json::json!(state)]),
                        ..Default::default()
                    }
                }),
            );
            object.required.insert("state".to_owned());

            variant.object = Some(Box::new(object));
            Schema::Object(variant)
        })
        .collect();

    schema.subschemas = Some(Box::new(schemars::schema::SubschemaValidation {
        one_of: Some(variants),
        ..Default::default()
    }));

    schema.metadata = Some(Box::new(schemars::schema::Metadata {
        title: Some("AggregateState".to_string()),
        description: Some(
            "Algebraic data type representing aggregate state machine states".to_string(),
        ),
        ..Default::default()
    }));

    Schema::Object(schema)
}

// ============================================================================
// MEALY STATE MACHINE SCHEMA
// ============================================================================

/// Schema for Mealy state machines
pub fn mealy_machine_schema(states: Vec<&str>, _inputs: Vec<&str>, _outputs: Vec<&str>) -> Schema {
    let mut schema = SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        ..Default::default()
    };

    let mut object = ObjectValidation {
        additional_properties: Some(Box::new(Schema::Bool(true))),
        ..Default::default()
    };

    // Transition table: (State, Input) -> State
    object.properties.insert(
        "transitions".to_owned(),
        Schema::Object({
            SchemaObject {
                instance_type: Some(InstanceType::Object.into()),
                metadata: Some(Box::new(schemars::schema::Metadata {
                    description: Some(
                        "State transition function: (State, Input) -> State".to_string(),
                    ),
                    ..Default::default()
                })),
                ..Default::default()
            }
        }),
    );

    // Output table: (State, Input) -> Output
    object.properties.insert(
        "outputs".to_owned(),
        Schema::Object({
            SchemaObject {
                instance_type: Some(InstanceType::Object.into()),
                metadata: Some(Box::new(schemars::schema::Metadata {
                    description: Some("Output function: (State, Input) -> Output".to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            }
        }),
    );

    // Current state
    object.properties.insert(
        "current_state".to_owned(),
        Schema::Object({
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                enum_values: Some(states.iter().map(|s| serde_json::json!(s)).collect()),
                ..Default::default()
            }
        }),
    );

    object.required.insert("transitions".to_owned());
    object.required.insert("outputs".to_owned());
    object.required.insert("current_state".to_owned());

    schema.metadata = Some(Box::new(schemars::schema::Metadata {
        title: Some("MealyStateMachine".to_string()),
        description: Some(
            "Mealy state machine where outputs depend on both state and input".to_string(),
        ),
        ..Default::default()
    }));

    schema.object = Some(Box::new(object));
    Schema::Object(schema)
}

// ============================================================================
// SPECIFICATION SCHEMA - Boolean algebra for validation
// ============================================================================

/// Schema for specifications (validation rules)
pub fn specification_schema<T: JsonSchema>() -> Schema {
    let mut schema = SchemaObject {
        instance_type: None,
        ..Default::default()
    };

    // Specifications form a boolean algebra (and, or, not)
    schema.subschemas = Some(Box::new(schemars::schema::SubschemaValidation {
        one_of: Some(vec![
            // Leaf specification
            Schema::Object({
                let mut s = SchemaObject {
                    instance_type: Some(InstanceType::Object.into()),
                    ..Default::default()
                };
                let mut obj = ObjectValidation::default();
                obj.properties.insert("rule".to_owned(), Schema::Bool(true));
                obj.required.insert("rule".to_owned());
                s.object = Some(Box::new(obj));
                s
            }),
            // AND composition
            Schema::Object({
                let mut s = SchemaObject {
                    instance_type: Some(InstanceType::Object.into()),
                    ..Default::default()
                };
                let mut obj = ObjectValidation::default();
                obj.properties.insert(
                    "and".to_owned(),
                    Schema::Object(SchemaObject {
                        instance_type: Some(InstanceType::Array.into()),
                        ..Default::default()
                    }),
                );
                obj.required.insert("and".to_owned());
                s.object = Some(Box::new(obj));
                s
            }),
            // OR composition
            Schema::Object({
                let mut s = SchemaObject {
                    instance_type: Some(InstanceType::Object.into()),
                    ..Default::default()
                };
                let mut obj = ObjectValidation::default();
                obj.properties.insert(
                    "or".to_owned(),
                    Schema::Object(SchemaObject {
                        instance_type: Some(InstanceType::Array.into()),
                        ..Default::default()
                    }),
                );
                obj.required.insert("or".to_owned());
                s.object = Some(Box::new(obj));
                s
            }),
            // NOT negation
            Schema::Object({
                let mut s = SchemaObject {
                    instance_type: Some(InstanceType::Object.into()),
                    ..Default::default()
                };
                let mut obj = ObjectValidation::default();
                obj.properties.insert("not".to_owned(), Schema::Bool(true));
                obj.required.insert("not".to_owned());
                s.object = Some(Box::new(obj));
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
    let mut schema = SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        ..Default::default()
    };

    let mut object = ObjectValidation::default();
    object.properties.insert(
        "_domain_trait".to_owned(),
        Schema::Object(SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            enum_values: Some(vec![serde_json::json!(trait_name)]),
            ..Default::default()
        }),
    );

    // Allow additional properties for the actual domain object (already set in initializer)

    schema.metadata = Some(Box::new(schemars::schema::Metadata {
        title: Some(trait_name.to_string()),
        description: Some(
            match trait_name {
                "ValueObject" => "Immutable object compared by value, not identity",
                "DomainEntity" => "Object with identity that persists beyond its attributes",
                "Aggregate" => "Consistency boundary with Mealy state machine behavior",
                "Policy" => "Pure business rule function",
                "Saga" => "Composed aggregate managing distributed transactions",
                _ => "Domain concept marker",
            }
            .to_string(),
        ),
        ..Default::default()
    }));

    schema.object = Some(Box::new(object));
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

        println!(
            "ValueObject schema:\n{}",
            serde_json::to_string_pretty(&value_object).unwrap()
        );
        println!(
            "Entity schema:\n{}",
            serde_json::to_string_pretty(&entity).unwrap()
        );
        println!(
            "Aggregate schema:\n{}",
            serde_json::to_string_pretty(&aggregate).unwrap()
        );
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
