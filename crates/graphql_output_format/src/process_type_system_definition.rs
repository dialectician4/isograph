use std::collections::{hash_map::Entry, BTreeMap, HashMap};

use common_lang_types::{
    GraphQLObjectTypeName, GraphQLScalarTypeName, IsographObjectTypeName, Location, SelectableName,
    Span, UnvalidatedTypeName, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLFieldDefinition, GraphQLInputValueDefinition, GraphQLNamedTypeAnnotation,
    GraphQLNonNullTypeAnnotation, GraphQLScalarTypeDefinition, GraphQLTypeAnnotation,
    GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument, GraphQLTypeSystemExtension,
    GraphQLTypeSystemExtensionDocument, GraphQLTypeSystemExtensionOrDefinition, NameValuePair,
    RootOperationKind,
};
use intern::string_key::{Intern, Lookup};
use isograph_config::CompilerConfigOptions;
use isograph_lang_types::{
    DefinitionLocation, SelectableServerFieldId, ServerObjectId, ServerStrongIdFieldId,
    VariableDefinition,
};
use isograph_schema::{
    EncounteredRootTypes, IsographObjectTypeDefinition, OutputFormat,
    ProcessTypeSystemDocumentOutcome, ProcessedRootTypes, RootOperationName, RootTypes, Schema,
    SchemaObject, SchemaScalar, SchemaServerField, SchemaServerFieldVariant,
    ServerFieldTypeAssociatedData, TypeRefinementMaps, UnvalidatedObjectFieldInfo,
    UnvalidatedSchemaSchemaField, ID_GRAPHQL_TYPE, STRING_JAVASCRIPT_TYPE,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    GraphQLSchemaObjectAssociatedData, GraphQLSchemaOriginalDefinitionType,
    UnvalidatedGraphqlSchema,
};

lazy_static! {
    static ref QUERY_TYPE: IsographObjectTypeName = "Query".intern().into();
    static ref MUTATION_TYPE: IsographObjectTypeName = "Mutation".intern().into();
}

pub fn process_graphql_type_system_document(
    schema: &mut UnvalidatedGraphqlSchema,
    type_system_document: GraphQLTypeSystemDocument,
    options: &CompilerConfigOptions,
) -> ProcessTypeDefinitionResult<ProcessTypeSystemDocumentOutcome> {
    // In the schema, interfaces, unions and objects are the same type of object (SchemaType),
    // with e.g. interfaces "simply" being objects that can be refined to other
    // concrete objects.
    //
    // Processing type system documents is done in two passes:
    // - First, create types for interfaces, objects, scalars, etc.
    // - Then, validate that all implemented interfaces exist, and add refinements
    //   to the found interface.
    let mut supertype_to_subtype_map = HashMap::new();
    let mut subtype_to_supertype_map = HashMap::new();

    let mut encountered_root_types = RootTypes {
        query: None,
        mutation: None,
        subscription: None,
    };
    let mut processed_root_types = None;

    for with_location in type_system_document.0 {
        let WithLocation {
            location,
            item: type_system_definition,
        } = with_location;
        match type_system_definition {
            GraphQLTypeSystemDefinition::ObjectTypeDefinition(object_type_definition) => {
                let concrete_type = Some(object_type_definition.name.item.into());

                for interface_name in object_type_definition.interfaces.iter() {
                    insert_into_type_refinement_maps(
                        interface_name.item.into(),
                        object_type_definition.name.item.into(),
                        &mut supertype_to_subtype_map,
                        &mut subtype_to_supertype_map,
                    );
                }

                let object_type_definition = object_type_definition.into();

                let outcome: ProcessObjectTypeDefinitionOutcome = process_object_type_definition(
                    schema,
                    object_type_definition,
                    true,
                    options,
                    concrete_type,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::Object,
                    },
                )?;
                if let Some(encountered_root_kind) = outcome.encountered_root_kind {
                    encountered_root_types.set_root_type(encountered_root_kind, outcome.object_id);
                }
            }
            GraphQLTypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                process_scalar_definition(schema, scalar_type_definition)?;
                // N.B. we assume that Mutation will be an object, not a scalar
            }
            GraphQLTypeSystemDefinition::InterfaceTypeDefinition(interface_type_definition) => {
                process_object_type_definition(
                    schema,
                    interface_type_definition.into(),
                    true,
                    options,
                    None,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::Interface,
                    },
                )?;
                // N.B. we assume that Mutation will be an object, not an interface
            }
            GraphQLTypeSystemDefinition::InputObjectTypeDefinition(
                input_object_type_definition,
            ) => {
                let concrete_type = Some(input_object_type_definition.name.item.into());
                process_object_type_definition(
                    schema,
                    input_object_type_definition.into(),
                    false,
                    options,
                    // Shouldn't really matter what we pass here
                    concrete_type,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::InputObject,
                    },
                )?;
            }
            GraphQLTypeSystemDefinition::DirectiveDefinition(_) => {
                // For now, Isograph ignores directive definitions,
                // but it might choose to allow-list them.
            }
            GraphQLTypeSystemDefinition::EnumDefinition(enum_definition) => {
                // TODO Do not do this
                process_scalar_definition(
                    schema,
                    GraphQLScalarTypeDefinition {
                        description: enum_definition.description,
                        name: enum_definition.name.map(|x| x.lookup().intern().into()),
                        directives: enum_definition.directives,
                    },
                )?;
            }
            GraphQLTypeSystemDefinition::UnionTypeDefinition(union_definition) => {
                // TODO do something reasonable here, once we add support for type refinements.
                process_object_type_definition(
                    schema,
                    IsographObjectTypeDefinition {
                        description: union_definition.description,
                        name: union_definition.name.map(|x| x.into()),
                        interfaces: vec![],
                        directives: union_definition.directives,
                        fields: vec![],
                    },
                    false,
                    options,
                    None,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::Union,
                    },
                )?;

                for union_member_type in union_definition.union_member_types {
                    insert_into_type_refinement_maps(
                        union_definition.name.item.into(),
                        union_member_type.item.into(),
                        &mut supertype_to_subtype_map,
                        &mut subtype_to_supertype_map,
                    )
                }
            }
            GraphQLTypeSystemDefinition::SchemaDefinition(schema_definition) => {
                if processed_root_types.is_some() {
                    return Err(WithLocation::new(
                        ProcessGraphqlTypeSystemDefinitionError::DuplicateSchemaDefinition,
                        location,
                    ));
                }
                processed_root_types = Some(RootTypes {
                    query: schema_definition.query,
                    mutation: schema_definition.mutation,
                    subscription: schema_definition.subscription,
                })
            }
        }
    }

    let type_refinement_map =
        get_type_refinement_map(schema, supertype_to_subtype_map, subtype_to_supertype_map)?;

    let root_types = process_root_types(schema, processed_root_types, encountered_root_types)?;

    if let Some(query_type_id) = root_types.query {
        schema
            .fetchable_types
            .insert(query_type_id, RootOperationName("query".to_string()));
    }
    if let Some(mutation_type_id) = root_types.mutation {
        schema
            .fetchable_types
            .insert(mutation_type_id, RootOperationName("mutation".to_string()));
    }
    // TODO add support for subscriptions

    Ok(ProcessTypeSystemDocumentOutcome {
        root_types,
        type_refinement_maps: type_refinement_map,
    })
}

pub fn process_graphql_type_extension_document(
    schema: &mut UnvalidatedGraphqlSchema,
    extension_document: GraphQLTypeSystemExtensionDocument,
    options: &CompilerConfigOptions,
) -> ProcessTypeDefinitionResult<ProcessTypeSystemDocumentOutcome> {
    let mut definitions = Vec::with_capacity(extension_document.0.len());
    let mut extensions = Vec::with_capacity(extension_document.0.len());

    for extension_or_definition in extension_document.0 {
        let WithLocation { location, item } = extension_or_definition;
        match item {
            GraphQLTypeSystemExtensionOrDefinition::Definition(definition) => {
                definitions.push(WithLocation::new(definition, location));
            }
            GraphQLTypeSystemExtensionOrDefinition::Extension(extension) => {
                extensions.push(WithLocation::new(extension, location))
            }
        }
    }

    // N.B. we should probably restructure this...?
    // Like, we could discover the mutation type right now!
    let outcome = process_graphql_type_system_document(
        schema,
        GraphQLTypeSystemDocument(definitions),
        options,
    )?;

    for extension in extensions.into_iter() {
        // TODO collect errors into vec
        // TODO we can encounter new interface implementations; we should account for that
        process_graphql_type_system_extension(schema, extension)?;
    }

    Ok(outcome)
}

pub(crate) type ProcessTypeDefinitionResult<T> =
    Result<T, WithLocation<ProcessGraphqlTypeSystemDefinitionError>>;

fn insert_into_type_refinement_maps(
    supertype_name: UnvalidatedTypeName,
    subtype_name: UnvalidatedTypeName, // aka the concrete type or union member
    supertype_to_subtype_map: &mut UnvalidatedTypeRefinementMap,
    subtype_to_supertype_map: &mut UnvalidatedTypeRefinementMap,
) {
    supertype_to_subtype_map
        .entry(supertype_name)
        .or_default()
        .push(subtype_name);
    subtype_to_supertype_map
        .entry(subtype_name)
        .or_default()
        .push(supertype_name);
}

#[derive(Error, Eq, PartialEq, Debug)]
pub(crate) enum ProcessGraphqlTypeSystemDefinitionError {
    // TODO include info about where the type was previously defined
    // TODO the type_definition_name refers to the second object being defined, which isn't
    // all that helpful
    #[error("Duplicate type definition ({type_definition_type}) named \"{type_name}\"")]
    DuplicateTypeDefinition {
        type_definition_type: &'static str,
        type_name: UnvalidatedTypeName,
    },

    // TODO include info about where the field was previously defined
    #[error("Duplicate field named \"{field_name}\" on type \"{parent_type}\"")]
    DuplicateField {
        field_name: SelectableName,
        parent_type: IsographObjectTypeName,
    },

    // TODO
    // This is held in a span pointing to one place the non-existent type was referenced.
    // We should perhaps include info about all the places it was referenced.
    //
    // When type Foo implements Bar and Bar is not defined:
    #[error("Type \"{type_name}\" is never defined.")]
    IsographObjectTypeNameNotDefined { type_name: UnvalidatedTypeName },

    #[error("Expected {type_name} to be an object, but it was a scalar.")]
    GenericObjectIsScalar { type_name: UnvalidatedTypeName },

    #[error(
        "You cannot manually defined the \"__typename\" field, which is defined in \"{parent_type}\"."
    )]
    TypenameCannotBeDefined { parent_type: IsographObjectTypeName },

    #[error(
        "The {strong_field_name} field on \"{parent_type}\" must have type \"ID!\".\n\
    This error can be suppressed using the \"on_invalid_id_type\" config parameter."
    )]
    IdFieldMustBeNonNullIdType {
        parent_type: IsographObjectTypeName,
        strong_field_name: &'static str,
    },

    #[error(
        "The type `{type_name}` is {is_type}, but it is being extended as {extended_as_type}."
    )]
    TypeExtensionMismatch {
        type_name: UnvalidatedTypeName,
        is_type: &'static str,
        extended_as_type: &'static str,
    },

    #[error("Duplicate schema definition")]
    DuplicateSchemaDefinition,

    #[error("Root types must be objects. This type is a scalar.")]
    RootTypeMustBeObject,
}

type UnvalidatedTypeRefinementMap = HashMap<UnvalidatedTypeName, Vec<UnvalidatedTypeName>>;
// When constructing the final map, we can replace object type names with ids.
pub type ValidatedTypeRefinementMap = HashMap<ServerObjectId, Vec<ServerObjectId>>;

pub(crate) fn process_object_type_definition(
    schema: &mut UnvalidatedGraphqlSchema,
    object_type_definition: IsographObjectTypeDefinition,
    // TODO this smells! We should probably pass Option<ServerIdFieldId>
    may_have_id_field: bool,
    options: &CompilerConfigOptions,
    concrete_type: Option<IsographObjectTypeName>,
    associated_data: GraphQLSchemaObjectAssociatedData,
) -> ProcessTypeDefinitionResult<ProcessObjectTypeDefinitionOutcome> {
    let &mut Schema {
        server_fields: ref mut schema_fields,
        server_field_data: ref mut schema_data,
        ..
    } = schema;
    let next_object_id = schema_data.server_objects.len().into();
    let string_type_for_typename = schema_data.scalar(schema_data.string_type_id).name;
    let type_names = &mut schema_data.defined_types;
    let objects = &mut schema_data.server_objects;
    let encountered_root_kind = match type_names.entry(object_type_definition.name.item.into()) {
        Entry::Occupied(_) => {
            return Err(WithLocation::new(
                ProcessGraphqlTypeSystemDefinitionError::DuplicateTypeDefinition {
                    // BUG: this could be an interface, actually
                    type_definition_type: "object",
                    type_name: object_type_definition.name.item.into(),
                },
                object_type_definition.name.location,
            ));
        }
        Entry::Vacant(vacant) => {
            // TODO avoid this
            let type_def_2 = object_type_definition.clone();
            let FieldObjectIdsEtc {
                unvalidated_schema_fields,
                encountered_fields,
                id_field,
            } = get_field_objects_ids_and_names(
                type_def_2.fields,
                schema_fields.len(),
                next_object_id,
                type_def_2.name.item,
                get_typename_type(string_type_for_typename.item),
                may_have_id_field,
                options,
            )?;

            objects.push(SchemaObject {
                description: object_type_definition.description.map(|d| d.item),
                name: object_type_definition.name.item,
                id: next_object_id,
                encountered_fields,
                id_field,
                directives: object_type_definition.directives,
                concrete_type,
                output_associated_data: associated_data,
            });

            schema_fields.extend(unvalidated_schema_fields);
            vacant.insert(SelectableServerFieldId::Object(next_object_id));

            // TODO default types are a GraphQL-land concept, but this is Isograph-land
            if object_type_definition.name.item == *QUERY_TYPE {
                Some(RootOperationKind::Query)
            } else if object_type_definition.name.item == *MUTATION_TYPE {
                Some(RootOperationKind::Mutation)
            } else {
                // TODO subscription
                None
            }
        }
    };

    Ok(ProcessObjectTypeDefinitionOutcome {
        object_id: next_object_id,
        encountered_root_kind,
    })
}

// TODO This is currently a completely useless function, serving only to surface
// some validation errors. It might be necessary once we handle __asNode etc.
// style fields.
fn get_type_refinement_map(
    schema: &mut UnvalidatedGraphqlSchema,
    unvalidated_supertype_to_subtype_map: UnvalidatedTypeRefinementMap,
    unvalidated_subtype_to_supertype_map: UnvalidatedTypeRefinementMap,
) -> ProcessTypeDefinitionResult<TypeRefinementMaps> {
    let supertype_to_subtype_map =
        validate_type_refinement_map(schema, unvalidated_supertype_to_subtype_map)?;
    let subtype_to_supertype_map =
        validate_type_refinement_map(schema, unvalidated_subtype_to_supertype_map)?;

    Ok(TypeRefinementMaps {
        subtype_to_supertype_map,
        supertype_to_subtype_map,
    })
}

// TODO this should accept an IsographScalarTypeDefinition
fn process_scalar_definition(
    schema: &mut UnvalidatedGraphqlSchema,
    scalar_type_definition: GraphQLScalarTypeDefinition,
) -> ProcessTypeDefinitionResult<()> {
    let &mut Schema {
        server_field_data: ref mut schema_data,
        ..
    } = schema;
    let next_scalar_id = schema_data.server_scalars.len().into();
    let type_names = &mut schema_data.defined_types;
    let scalars = &mut schema_data.server_scalars;
    match type_names.entry(scalar_type_definition.name.item.into()) {
        Entry::Occupied(_) => {
            return Err(WithLocation::new(
                ProcessGraphqlTypeSystemDefinitionError::DuplicateTypeDefinition {
                    type_definition_type: "scalar",
                    type_name: scalar_type_definition.name.item.into(),
                },
                scalar_type_definition.name.location,
            ));
        }
        Entry::Vacant(vacant) => {
            scalars.push(SchemaScalar {
                description: scalar_type_definition.description,
                name: scalar_type_definition.name,
                id: next_scalar_id,
                javascript_name: *STRING_JAVASCRIPT_TYPE,
                output_format: std::marker::PhantomData,
            });

            vacant.insert(SelectableServerFieldId::Scalar(next_scalar_id));
        }
    }
    Ok(())
}

fn process_root_types(
    schema: &UnvalidatedGraphqlSchema,
    processed_root_types: Option<ProcessedRootTypes>,
    encountered_root_types: EncounteredRootTypes,
) -> ProcessTypeDefinitionResult<EncounteredRootTypes> {
    match processed_root_types {
        Some(processed_root_types) => {
            let RootTypes {
                query: query_type_name,
                mutation: mutation_type_name,
                subscription: subscription_type_name,
            } = processed_root_types;

            let query_id = query_type_name
                .map(|query_type_name| look_up_root_type(schema, query_type_name))
                .transpose()?;
            let mutation_id = mutation_type_name
                .map(|mutation_type_name| look_up_root_type(schema, mutation_type_name))
                .transpose()?;
            let subscription_id = subscription_type_name
                .map(|subscription_type_name| look_up_root_type(schema, subscription_type_name))
                .transpose()?;

            Ok(RootTypes {
                query: query_id,
                mutation: mutation_id,
                subscription: subscription_id,
            })
        }
        None => Ok(encountered_root_types),
    }
}

fn look_up_root_type(
    schema: &UnvalidatedGraphqlSchema,
    type_name: WithLocation<GraphQLObjectTypeName>,
) -> ProcessTypeDefinitionResult<ServerObjectId> {
    match schema
        .server_field_data
        .defined_types
        .get(&type_name.item.into())
    {
        Some(SelectableServerFieldId::Object(object_id)) => Ok(*object_id),
        Some(SelectableServerFieldId::Scalar(_)) => Err(WithLocation::new(
            ProcessGraphqlTypeSystemDefinitionError::RootTypeMustBeObject,
            type_name.location,
        )),
        None => Err(WithLocation::new(
            ProcessGraphqlTypeSystemDefinitionError::IsographObjectTypeNameNotDefined {
                type_name: type_name.item.into(),
            },
            type_name.location,
        )),
    }
}

fn process_graphql_type_system_extension(
    schema: &mut UnvalidatedGraphqlSchema,
    extension: WithLocation<GraphQLTypeSystemExtension>,
) -> ProcessTypeDefinitionResult<()> {
    match extension.item {
        GraphQLTypeSystemExtension::ObjectTypeExtension(object_extension) => {
            let name = object_extension.name.item;

            let id = schema
                .server_field_data
                .defined_types
                .get(&name.into())
                .expect(
                    "TODO why does this id not exist. This probably indicates a bug in Isograph.",
                );

            match *id {
                SelectableServerFieldId::Object(object_id) => {
                    let schema_object = schema.server_field_data.object_mut(object_id);

                    if !object_extension.fields.is_empty() {
                        panic!("Adding fields in schema extensions is not allowed, yet.");
                    }
                    if !object_extension.interfaces.is_empty() {
                        panic!("Adding interfaces in schema extensions is not allowed, yet.");
                    }

                    schema_object.directives.extend(object_extension.directives);

                    Ok(())
                }
                SelectableServerFieldId::Scalar(_) => Err(WithLocation::new(
                    ProcessGraphqlTypeSystemDefinitionError::TypeExtensionMismatch {
                        type_name: name.into(),
                        is_type: "a scalar",
                        extended_as_type: "an object",
                    },
                    object_extension.name.location,
                )),
            }
        }
    }
}

struct FieldObjectIdsEtc<TOutputFormat: OutputFormat> {
    unvalidated_schema_fields: Vec<UnvalidatedSchemaSchemaField<TOutputFormat>>,
    // TODO this should be BTreeMap<_, WithLocation<_>> or something
    encountered_fields: BTreeMap<SelectableName, UnvalidatedObjectFieldInfo>,
    // TODO this should not be a ServerFieldId, but a special type
    id_field: Option<ServerStrongIdFieldId>,
}

/// Given a vector of fields from the schema AST all belonging to the same object/interface,
/// return a vector of unvalidated fields and a set of field names.
fn get_field_objects_ids_and_names<TOutputFormat: OutputFormat>(
    new_fields: Vec<WithLocation<GraphQLFieldDefinition>>,
    next_field_id: usize,
    parent_type_id: ServerObjectId,
    parent_type_name: IsographObjectTypeName,
    typename_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    // TODO this is hacky
    may_have_field_id: bool,
    options: &CompilerConfigOptions,
) -> ProcessTypeDefinitionResult<FieldObjectIdsEtc<TOutputFormat>> {
    let new_field_count = new_fields.len();
    let mut encountered_fields = BTreeMap::new();
    let mut unvalidated_fields = Vec::with_capacity(new_field_count);
    let mut server_field_ids = Vec::with_capacity(new_field_count + 1); // +1 for the typename
    let mut id_field = None;
    let id_name = "id".intern().into();
    for (current_field_index, field) in new_fields.into_iter().enumerate() {
        let next_server_field_id_usize = next_field_id + current_field_index;
        let next_server_field_id = next_server_field_id_usize.into();

        match encountered_fields.insert(
            field.item.name.item.into(),
            DefinitionLocation::Server(next_server_field_id),
        ) {
            None => {
                // TODO check for @strong directive instead!
                if may_have_field_id && field.item.name.item == id_name {
                    set_and_validate_id_field(
                        &mut id_field,
                        next_server_field_id_usize,
                        &field,
                        parent_type_name,
                        options,
                    )?;
                }

                unvalidated_fields.push(SchemaServerField {
                    description: field.item.description.map(|d| d.item),
                    name: field.item.name,
                    id: next_server_field_id,
                    associated_data: ServerFieldTypeAssociatedData {
                        type_name: field.item.type_,
                        variant: SchemaServerFieldVariant::LinkedField,
                    },
                    parent_type_id,
                    arguments: field
                        .item
                        .arguments
                        .into_iter()
                        .map(graphql_input_value_definition_to_variable_definition)
                        .collect::<Result<Vec<_>, _>>()?,
                    is_discriminator: false,
                    phantom_data: std::marker::PhantomData,
                });
                server_field_ids.push(next_server_field_id);
            }
            Some(_) => {
                return Err(WithLocation::new(
                    ProcessGraphqlTypeSystemDefinitionError::DuplicateField {
                        field_name: field.item.name.item.into(),
                        parent_type: parent_type_name,
                    },
                    field.item.name.location,
                ));
            }
        }
    }

    // ------- HACK -------
    // Magic __typename field
    // TODO: find a way to do this that is less tied to GraphQL
    // TODO: the only way to determine that a field is a magic __typename field is
    // to check the name! That's a bit unfortunate. We should model these differently,
    // perhaps fields should contain an enum (IdField, TypenameField, ActualField)
    let typename_field_id = (next_field_id + server_field_ids.len()).into();
    let typename_name = WithLocation::new("__typename".intern().into(), Location::generated());
    server_field_ids.push(typename_field_id);
    unvalidated_fields.push(SchemaServerField {
        description: None,
        name: typename_name,
        id: typename_field_id,
        associated_data: ServerFieldTypeAssociatedData {
            type_name: typename_type.clone(),
            variant: SchemaServerFieldVariant::LinkedField,
        },
        parent_type_id,
        arguments: vec![],
        is_discriminator: true,
        phantom_data: std::marker::PhantomData,
    });

    if encountered_fields
        .insert(
            typename_name.item.into(),
            DefinitionLocation::Server(typename_field_id),
        )
        .is_some()
    {
        return Err(WithLocation::new(
            ProcessGraphqlTypeSystemDefinitionError::TypenameCannotBeDefined {
                parent_type: parent_type_name,
            },
            // This is blatantly incorrect, we should have the location
            // of the previously defined typename
            Location::generated(),
        ));
    }
    // ----- END HACK -----

    Ok(FieldObjectIdsEtc {
        unvalidated_schema_fields: unvalidated_fields,
        encountered_fields,
        id_field,
    })
}

fn get_typename_type(
    string_type_for_typename: GraphQLScalarTypeName,
) -> GraphQLTypeAnnotation<UnvalidatedTypeName> {
    GraphQLTypeAnnotation::NonNull(Box::new(GraphQLNonNullTypeAnnotation::Named(
        GraphQLNamedTypeAnnotation(WithSpan::new(
            string_type_for_typename.into(),
            // TODO we probably need a generated or built-in span type
            Span::todo_generated(),
        )),
    )))
}

/// If we have encountered an id field, we can:
/// - validate that the id field is properly defined, i.e. has type ID!
/// - set the id field
fn set_and_validate_id_field(
    id_field: &mut Option<ServerStrongIdFieldId>,
    current_field_id: usize,
    field: &WithLocation<GraphQLFieldDefinition>,
    parent_type_name: IsographObjectTypeName,
    options: &CompilerConfigOptions,
) -> ProcessTypeDefinitionResult<()> {
    // N.B. id_field is guaranteed to be None; otherwise field_names_to_type_name would
    // have contained this field name already.
    debug_assert!(id_field.is_none(), "id field should not be defined twice");

    // We should change the type here! It should not be ID! It should be a
    // type specific to the concrete type, e.g. UserID.
    *id_field = Some(current_field_id.into());

    match field.item.type_.inner_non_null_named_type() {
        Some(type_) => {
            if type_.0.item.lookup() != ID_GRAPHQL_TYPE.lookup() {
                options.on_invalid_id_type.on_failure(|| {
                    WithLocation::new(
                        ProcessGraphqlTypeSystemDefinitionError::IdFieldMustBeNonNullIdType {
                            strong_field_name: "id",
                            parent_type: parent_type_name,
                        },
                        // TODO this shows the wrong span?
                        field.location,
                    )
                })?;
            }
            Ok(())
        }
        None => {
            options.on_invalid_id_type.on_failure(|| {
                WithLocation::new(
                    ProcessGraphqlTypeSystemDefinitionError::IdFieldMustBeNonNullIdType {
                        strong_field_name: "id",
                        parent_type: parent_type_name,
                    },
                    // TODO this shows the wrong span?
                    field.location,
                )
            })?;
            Ok(())
        }
    }
}

fn validate_type_refinement_map(
    schema: &mut UnvalidatedGraphqlSchema,
    unvalidated_type_refinement_map: UnvalidatedTypeRefinementMap,
) -> ProcessTypeDefinitionResult<ValidatedTypeRefinementMap> {
    let supertype_to_subtype_map = unvalidated_type_refinement_map
        .into_iter()
        .map(|(key_type_name, values_type_names)| {
            let key_id = lookup_object_in_schema(schema, key_type_name)?;

            let value_type_ids = values_type_names
                .into_iter()
                .map(|value_type_name| lookup_object_in_schema(schema, value_type_name))
                .collect::<Result<Vec<_>, _>>()?;

            Ok((key_id, value_type_ids))
        })
        .collect::<Result<HashMap<_, _>, WithLocation<ProcessGraphqlTypeSystemDefinitionError>>>(
        )?;
    Ok(supertype_to_subtype_map)
}

fn lookup_object_in_schema(
    schema: &mut UnvalidatedGraphqlSchema,
    unvalidated_type_name: UnvalidatedTypeName,
) -> ProcessTypeDefinitionResult<ServerObjectId> {
    let result = (*schema
        .server_field_data
        .defined_types
        .get(&unvalidated_type_name)
        .ok_or_else(|| {
            WithLocation::new(
                ProcessGraphqlTypeSystemDefinitionError::IsographObjectTypeNameNotDefined {
                    type_name: unvalidated_type_name,
                },
                // TODO don't do this
                Location::Generated,
            )
        })?)
    .try_into()
    .map_err(|_| {
        WithLocation::new(
            ProcessGraphqlTypeSystemDefinitionError::GenericObjectIsScalar {
                type_name: unvalidated_type_name,
            },
            // TODO don't do this
            Location::Generated,
        )
    })?;

    Ok(result)
}

pub fn graphql_input_value_definition_to_variable_definition(
    input_value_definition: WithLocation<GraphQLInputValueDefinition>,
) -> ProcessTypeDefinitionResult<WithLocation<VariableDefinition<UnvalidatedTypeName>>> {
    let default_value = input_value_definition
        .item
        .default_value
        .map(|graphql_constant_value| {
            Ok::<_, WithLocation<ProcessGraphqlTypeSystemDefinitionError>>(WithLocation::new(
                convert_graphql_constant_value_to_isograph_constant_value(
                    graphql_constant_value.item,
                ),
                graphql_constant_value.location,
            ))
        })
        .transpose()?;
    Ok(WithLocation::new(
        VariableDefinition {
            name: input_value_definition.item.name.map(VariableName::from),
            type_: input_value_definition
                .item
                .type_
                .map(UnvalidatedTypeName::from),
            default_value,
        },
        input_value_definition.location,
    ))
}

fn convert_graphql_constant_value_to_isograph_constant_value(
    graphql_constant_value: graphql_lang_types::GraphQLConstantValue,
) -> isograph_lang_types::ConstantValue {
    match graphql_constant_value {
        graphql_lang_types::GraphQLConstantValue::Int(i) => {
            isograph_lang_types::ConstantValue::Integer(i)
        }
        graphql_lang_types::GraphQLConstantValue::Boolean(b) => {
            isograph_lang_types::ConstantValue::Boolean(b)
        }
        graphql_lang_types::GraphQLConstantValue::String(s) => {
            isograph_lang_types::ConstantValue::String(s)
        }
        graphql_lang_types::GraphQLConstantValue::Float(f) => {
            isograph_lang_types::ConstantValue::Float(f)
        }
        graphql_lang_types::GraphQLConstantValue::Null => isograph_lang_types::ConstantValue::Null,
        graphql_lang_types::GraphQLConstantValue::Enum(e) => {
            isograph_lang_types::ConstantValue::Enum(e)
        }
        graphql_lang_types::GraphQLConstantValue::List(l) => {
            let converted_list = l
                .into_iter()
                .map(|x| {
                    WithLocation::new(
                        convert_graphql_constant_value_to_isograph_constant_value(x.item),
                        x.location,
                    )
                })
                .collect::<Vec<_>>();
            isograph_lang_types::ConstantValue::List(converted_list)
        }
        graphql_lang_types::GraphQLConstantValue::Object(o) => {
            let converted_object = o
                .into_iter()
                .map(|name_value_pair| NameValuePair {
                    name: name_value_pair.name,
                    value: WithLocation::new(
                        convert_graphql_constant_value_to_isograph_constant_value(
                            name_value_pair.value.item,
                        ),
                        name_value_pair.value.location,
                    ),
                })
                .collect::<Vec<_>>();
            isograph_lang_types::ConstantValue::Object(converted_object)
        }
    }
}

pub struct ProcessObjectTypeDefinitionOutcome {
    pub object_id: ServerObjectId,
    pub encountered_root_kind: Option<RootOperationKind>,
}
