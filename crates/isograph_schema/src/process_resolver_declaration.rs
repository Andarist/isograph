use std::fmt;

use common_lang_types::{
    DefinedField, IsographDirectiveName, IsographObjectTypeName, ServerFieldDefinitionName,
    UnvalidatedTypeName, WithSpan,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    EncounteredTypeId, FragmentDirectiveUsage, ObjectId, ResolverDeclaration,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{SchemaResolver, UnvalidatedSchema};

impl UnvalidatedSchema {
    pub fn process_resolver_declaration(
        &mut self,
        resolver_declaration: WithSpan<ResolverDeclaration>,
    ) -> ProcessResolverDeclarationResult<()> {
        let parent_type_id = self
            .schema_data
            .defined_types
            .get(&resolver_declaration.item.parent_type.item.into())
            .ok_or(ProcessResolverDeclarationError::MissingParent {
                parent_type_name: resolver_declaration.item.parent_type.item,
            })?;

        match parent_type_id {
            EncounteredTypeId::Object(object_id) => {
                self.add_resolver_field_to_object(*object_id, resolver_declaration)?;
            }
            EncounteredTypeId::Scalar(scalar_id) => {
                let scalar_name = self.schema_data.scalars[scalar_id.as_usize()].name;
                return Err(ProcessResolverDeclarationError::InvalidParentType {
                    parent_type: "scalar",
                    parent_type_name: scalar_name.into(),
                });
            }
        }

        Ok(())
    }

    fn add_resolver_field_to_object(
        &mut self,
        parent_object_id: ObjectId,
        resolver_declaration: WithSpan<ResolverDeclaration>,
    ) -> ProcessResolverDeclarationResult<()> {
        let object = &mut self.schema_data.objects[parent_object_id.as_usize()];
        let resolver_field_name = resolver_declaration.item.resolver_field_name.item;

        if object
            .encountered_field_names
            .insert(
                resolver_field_name.into(),
                DefinedField::ResolverField(resolver_field_name),
            )
            .is_some()
        {
            // Did not insert, so this object already has a field with the same name :(
            return Err(ProcessResolverDeclarationError::ParentAlreadyHasField {
                parent_type: "object",
                parent_type_name: object.name.into(),
                resolver_field_name: resolver_field_name.into(),
            });
        }

        let next_resolver_id = self.resolvers.len().into();
        object.resolvers.push(next_resolver_id);

        let name = resolver_declaration.item.resolver_field_name.item.into();
        let variant = get_resolver_variant(&resolver_declaration.item.directives);
        let has_associated_js_function = resolver_declaration.item.has_associated_js_function;

        // TODO variant should carry payloads, instead of this check
        if variant.as_ref().map(|span| span.item) == Some(ResolverVariant::Component) {
            if !has_associated_js_function {
                return Err(ProcessResolverDeclarationError::ComponentResolverMissingJsFunction {});
            }
        }

        self.resolvers.push(SchemaResolver {
            description: resolver_declaration.item.description.map(|d| d.item),
            name,
            id: next_resolver_id,
            resolver_definition_path: resolver_declaration.item.resolver_definition_path,
            selection_set_and_unwraps: resolver_declaration.item.selection_set_and_unwraps,
            variant,
            is_fetchable: is_fetchable(&resolver_declaration.item.directives),
            variable_definitions: resolver_declaration.item.variable_definitions,
            type_and_field: format!("{}__{}", object.name, name).intern().into(),
            has_associated_js_function,
            parent_object_id,
        });
        Ok(())
    }
}

type ProcessResolverDeclarationResult<T> = Result<T, ProcessResolverDeclarationError>;

#[derive(Error, Debug)]
pub enum ProcessResolverDeclarationError {
    #[error("Missing parent type. Type: `{parent_type_name}`")]
    MissingParent {
        parent_type_name: UnvalidatedTypeName,
    },

    #[error("Invalid parent type. `{parent_type_name}` is a {parent_type}, but it should be an object or interface.")]
    InvalidParentType {
        parent_type: &'static str,
        parent_type_name: UnvalidatedTypeName,
    },

    #[error(
        "The {parent_type} {parent_type_name} already has a field named `{resolver_field_name}`."
    )]
    ParentAlreadyHasField {
        parent_type: &'static str,
        parent_type_name: IsographObjectTypeName,
        resolver_field_name: ServerFieldDefinitionName,
    },

    #[error(
        "Resolvers with @component must have associated javascript (i.e. iso`...` must be called as a function, as in iso`...`(MyComponent))"
    )]
    ComponentResolverMissingJsFunction {
        // TODO add parent type and resolver field name
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResolverVariant {
    Component,
    Eager,
}

impl fmt::Display for ResolverVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolverVariant::Component => write!(f, "Component"),
            ResolverVariant::Eager => write!(f, "Eager"),
        }
    }
}

lazy_static! {
    // This is regex is inadequate, as iso<typeof foo`...`>, and it's certainly possible
    // to want that.
    static ref EAGER: IsographDirectiveName = "eager".intern().into();
    static ref COMPONENT: IsographDirectiveName = "component".intern().into();
    static ref FETCHABLE: IsographDirectiveName = "fetchable".intern().into();
}

// TODO validate that the type is actually fetchable, and that we don't have both
fn get_resolver_variant(
    directives: &[WithSpan<FragmentDirectiveUsage>],
) -> Option<WithSpan<ResolverVariant>> {
    for directive in directives.iter() {
        let span = directive.span;
        if directive.item.name.item == *EAGER {
            return Some(WithSpan::new(ResolverVariant::Eager, span));
        } else if directive.item.name.item == *COMPONENT {
            return Some(WithSpan::new(ResolverVariant::Component, span));
        }
    }
    None
}

fn is_fetchable(directives: &[WithSpan<FragmentDirectiveUsage>]) -> bool {
    for directive in directives.iter() {
        if directive.item.name.item == *FETCHABLE {
            return true;
        }
    }
    false
}