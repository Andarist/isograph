use std::collections::{hash_map::Entry, HashMap};

use common_lang_types::{
    DefinedField, FieldDefinitionName, FieldId, ObjectTypeName, OutputTypeName, ScalarFieldName,
    TypeId, TypeWithFieldsId, UnvalidatedTypeName, WithSpan,
};
use graphql_lang_types::{
    ObjectTypeDefinition, OutputFieldDefinition, TypeSystemDefinition, TypeSystemDocument,
};
use intern::string_key::Intern;
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{Schema, SchemaField, SchemaObject, UnvalidatedSchema, UnvalidatedSchemaField};

lazy_static! {
    static ref QUERY_TYPE: ObjectTypeName = "Query".intern().into();
}

impl UnvalidatedSchema {
    pub fn process_type_system_document(
        &mut self,
        type_system_document: TypeSystemDocument,
    ) -> ProcessTypeDefinitionResult<()> {
        for type_system_definition in type_system_document.0 {
            match type_system_definition {
                TypeSystemDefinition::ObjectTypeDefinition(object_type_definition) => {
                    self.process_object_type_definition(object_type_definition)?;
                }
            }
        }
        Ok(())
    }

    fn process_object_type_definition(
        &mut self,
        object_type_definition: ObjectTypeDefinition,
    ) -> ProcessTypeDefinitionResult<()> {
        let &mut Schema {
            fields: ref mut existing_fields,
            ref mut schema_data,
            ..
        } = self;
        let next_object_id = schema_data.objects.len().into();
        let ref mut type_names = schema_data.defined_types;
        let ref mut objects = schema_data.objects;
        match type_names.entry(object_type_definition.name.item.into()) {
            Entry::Occupied(_) => {
                return Err(ProcessTypeDefinitionError::DuplicateTypeDefinition {
                    type_definition_type: "object",
                    type_name: object_type_definition.name.item.into(),
                });
            }
            Entry::Vacant(vacant) => {
                let (new_fields, field_ids, encountered_field_names) =
                    get_field_objects_ids_and_names(
                        object_type_definition.fields,
                        existing_fields.len(),
                        TypeWithFieldsId::Object(next_object_id),
                        object_type_definition.name.item.into(),
                    )?;
                objects.push(SchemaObject {
                    description: object_type_definition.description.map(|d| d.item),
                    name: object_type_definition.name.item,
                    id: next_object_id,
                    fields: field_ids,
                    encountered_field_names,
                });

                // ----- HACK -----
                // Instead of this, we should parse GraphQL schema declarations.
                if object_type_definition.name.item == *QUERY_TYPE {
                    self.query_type = Some(next_object_id);
                }
                // --- END HACK ---

                existing_fields.extend(new_fields);
                vacant.insert(TypeId::Object(next_object_id));
            }
        }
        Ok(())
    }
}

/// Given a vector of fields from the schema AST all belonging to the same object/interface,
/// return a vector of unvalidated fields and a set of field names.
fn get_field_objects_ids_and_names(
    new_fields: Vec<WithSpan<OutputFieldDefinition>>,
    next_field_id: usize,
    parent_type: TypeWithFieldsId,
    parent_type_name: OutputTypeName,
) -> ProcessTypeDefinitionResult<(
    Vec<UnvalidatedSchemaField>,
    Vec<FieldId>,
    HashMap<FieldDefinitionName, DefinedField<UnvalidatedTypeName, ScalarFieldName>>,
)> {
    let new_field_count = new_fields.len();
    let mut field_names_to_type_name = HashMap::with_capacity(new_field_count);
    let mut unvalidated_fields = Vec::with_capacity(new_field_count);
    let mut field_ids = Vec::with_capacity(new_field_count);
    for (current_field_index, field) in new_fields.iter().enumerate() {
        // TODO use entry
        match field_names_to_type_name.insert(
            field.item.name.item,
            DefinedField::ServerField(*field.item.type_.inner()),
        ) {
            None => {
                unvalidated_fields.push(SchemaField {
                    description: field.item.description.map(|d| d.item),
                    name: field.item.name.item,
                    id: (next_field_id + current_field_index).into(),
                    field_type: DefinedField::ServerField(*field.item.type_.inner()),
                    parent_type_id: parent_type,
                });
                field_ids.push((next_field_id + current_field_index).into());
            }
            Some(_) => {
                return Err(ProcessTypeDefinitionError::DuplicateField {
                    field_name: field.item.name.item,
                    parent_type: parent_type_name,
                });
            }
        }
    }
    Ok((unvalidated_fields, field_ids, field_names_to_type_name))
}

type ProcessTypeDefinitionResult<T> = Result<T, ProcessTypeDefinitionError>;

/// Errors tha make semantic sense when referring to creating a GraphQL schema in-memory representation
#[derive(Error, Debug)]
pub enum ProcessTypeDefinitionError {
    #[error("Duplicate type definition ({type_definition_type}) named \"{type_name}\"")]
    DuplicateTypeDefinition {
        type_definition_type: &'static str,
        type_name: UnvalidatedTypeName,
    },

    #[error("Duplicate field named \"{field_name}\" on type \"{parent_type}\"")]
    DuplicateField {
        field_name: FieldDefinitionName,
        parent_type: OutputTypeName,
    },
}