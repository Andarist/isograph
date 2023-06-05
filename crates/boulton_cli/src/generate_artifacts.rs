use std::{
    collections::HashSet,
    fmt::Debug,
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

use boulton_lang_types::{
    FieldSelection::{LinkedField, ScalarField},
    NonConstantValue, Selection, SelectionFieldArgument,
};
use boulton_schema::{
    merge_selection_set, MergedSelectionSet, SchemaObject, SchemaTypeWithFields, ValidatedSchema,
    ValidatedSchemaResolverDefinitionInfo, ValidatedSelectionSetAndUnwraps,
    ValidatedVariableDefinition,
};
use common_lang_types::{
    DefinedField, FieldDefinitionName, HasName, ObjectId, QueryOperationName,
    ResolverDefinitionPath, TypeWithFieldsId, TypeWithFieldsName, TypeWithoutFieldsId,
    UnvalidatedTypeName, WithSpan,
};
use graphql_lang_types::TypeAnnotation;
use thiserror::Error;

pub(crate) fn generate_artifacts(
    schema: &ValidatedSchema,
    project_root: &PathBuf,
) -> Result<String, GenerateArtifactsError> {
    let query_type = schema.query_type.expect("Expect Query to be defined");
    let query = schema.schema_data.object(query_type);

    write_artifacts(get_all_artifacts(query, schema, query_type), project_root)?;

    Ok("".into())
}

fn get_all_artifacts<'schema>(
    query: &'schema SchemaObject,
    schema: &'schema ValidatedSchema,
    query_type: ObjectId,
) -> impl Iterator<Item = Result<Artifact<'schema>, GenerateArtifactsError>> + 'schema {
    let mut fields = schema.fields.iter();
    std::iter::from_fn(move || {
        while let Some(field) = fields.next() {
            // let field = schema.field(*field_id);
            if let Some(resolver_field) = field.field_type.as_resolver_field() {
                if field.parent_type_id == query_type.into() {
                    return Some(
                        generate_fetchable_resolver_artifact(schema, resolver_field)
                            .map(|x| Artifact::FetchableResolver(x)),
                    );
                } else {
                    return Some(
                        generate_non_fetchable_resolver_artifact(schema, resolver_field)
                            .map(|x| Artifact::NonFetchableResolver(x)),
                    );
                }
            }
        }
        // Non query artifacts
        None
    })
}

#[derive(Debug)]
pub struct QueryText(pub String);

fn generate_fetchable_resolver_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    resolver_definition: &ValidatedSchemaResolverDefinitionInfo,
) -> Result<FetchableResolver<'schema>, GenerateArtifactsError> {
    if let Some(ref selection_set_and_unwraps) = resolver_definition.selection_set_and_unwraps {
        let field = schema.field(resolver_definition.field_id);
        let query_name: QueryOperationName = field.name.into();

        let merged_selection_set = merge_selection_set(
            schema,
            schema
                .schema_data
                .object(schema.query_type.expect("expect query type to exist"))
                .into(),
            selection_set_and_unwraps,
        );

        let query_object_id = schema.query_type.expect("expected query type to exist");
        let query_type = schema
            .schema_data
            .lookup_type_with_fields(query_object_id.into());
        let query_text = generate_query_text(
            query_name,
            schema,
            &merged_selection_set,
            &resolver_definition.variable_definitions,
        );
        let query_type_declaration =
            generate_query_type_declaration(schema, &merged_selection_set, 1)?;
        let resolver_import_statement = generate_resolver_import_statement(
            field.name,
            resolver_definition.resolver_definition_path,
        );
        let resolver_response_type_declaration =
            ResolverResponseTypeDeclaration("foo: string".to_string());
        let user_response_type_declaration = UserResponseTypeDeclaration("foo: string".to_string());
        let mut nested_resolver_artifact_imports = HashSet::new();
        let reader_ast = generate_reader_ast(
            schema,
            selection_set_and_unwraps,
            query_type,
            0,
            &mut nested_resolver_artifact_imports,
        );

        Ok(FetchableResolver {
            query_text,
            query_name,
            parent_type: query_type,
            query_type_declaration,
            resolver_import_statement,
            resolver_response_type_declaration,
            user_response_type_declaration,
            reader_ast,
            nested_resolver_artifact_imports,
        })
    } else {
        // TODO convert to error
        todo!("Unsupported: resolvers on query with no selection set")
    }
}

fn generate_non_fetchable_resolver_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    resolver_definition: &ValidatedSchemaResolverDefinitionInfo,
) -> Result<NonFetchableResolver<'schema>, GenerateArtifactsError> {
    if let Some(selection_set_and_unwraps) = &resolver_definition.selection_set_and_unwraps {
        let field = schema.field(resolver_definition.field_id);
        let parent_type = schema
            .schema_data
            .lookup_type_with_fields(field.parent_type_id);
        let mut nested_resolver_artifact_imports = HashSet::new();
        let reader_ast = generate_reader_ast(
            schema,
            selection_set_and_unwraps,
            parent_type,
            0,
            &mut nested_resolver_artifact_imports,
        );
        let resolver_import_statement = generate_resolver_import_statement(
            field.name,
            resolver_definition.resolver_definition_path,
        );
        Ok(NonFetchableResolver {
            parent_type: schema
                .schema_data
                .lookup_type_with_fields(field.parent_type_id),
            resolver_field_name: field.name,
            reader_ast,
            nested_resolver_artifact_imports,
            resolver_import_statement,
        })
    } else {
        panic!("Unsupported: resolvers not on query with no selection set")
    }
}

#[derive(Debug)]
pub enum Artifact<'schema> {
    FetchableResolver(FetchableResolver<'schema>),
    NonFetchableResolver(NonFetchableResolver<'schema>),
}

#[derive(Debug)]
pub struct QueryTypeDeclaration(pub String);

#[derive(Debug)]
pub struct ResolverImportStatement(pub String);

#[derive(Debug)]
pub struct ResolverResponseTypeDeclaration(pub String);

#[derive(Debug)]
pub struct UserResponseTypeDeclaration(pub String);

#[derive(Debug)]
pub struct ReaderAst(pub String);

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct NestedResolverName(pub String);

#[derive(Debug)]
pub struct FetchableResolver<'schema> {
    pub query_text: QueryText,
    pub query_name: QueryOperationName,
    pub parent_type: SchemaTypeWithFields<'schema>,
    pub query_type_declaration: QueryTypeDeclaration,
    pub resolver_import_statement: ResolverImportStatement,
    pub resolver_response_type_declaration: ResolverResponseTypeDeclaration,
    pub user_response_type_declaration: UserResponseTypeDeclaration,
    pub reader_ast: ReaderAst,
    pub nested_resolver_artifact_imports: HashSet<NestedResolverName>,
}

impl<'schema> FetchableResolver<'schema> {
    fn file_contents(&self) -> String {
        // TODO don't use merged, use regular selection set when generating fragment type
        // (i.e. we are not data masking)
        format!(
            "import type {{BoultonFetchableResolver, ReaderAst}} from '@boulton/react';\n\
            {}\n\
            {}\n\
            const queryText = '{}';\n\n\
            const normalizationAst = {{notNeededForDemo: true}};\n\
            const readerAst: ReaderAst = {};\n\n\
            // The type, when passed to the resolver (currently this is the raw response type, it should be the response type)\n\
            export type ResolverParameterType = {{\n{}}};\n\n\
            // The type, when returned from the resolver\n\
            type ResolverResponse = {{\n  {}\n}};\n\n\
            // The type, when read out\n\
            type UserResponse = {{\n  {}\n}};\n\n\
            const artifact: BoultonFetchableResolver<ResolverParamaterType, ResolverResponse, UserResponse> = {{\n\
            {}kind: 'FetchableResolver',\n\
            {}queryText,\n\
            {}normalizationAst,\n\
            {}readerAst,\n\
            {}resolver,\n\
            }};\n\n\
            export default artifact;\n",
            self.resolver_import_statement.0,
            nested_resolver_names_to_import_statement(&self.nested_resolver_artifact_imports),
            self.query_text.0,
            self.reader_ast.0,
            self.query_type_declaration.0,
            self.resolver_response_type_declaration.0,
            self.user_response_type_declaration.0,
            "  ",
            "  ",
            "  ",
            "  ",
            "  "
        )
    }
}

#[derive(Debug)]
pub struct NonFetchableResolver<'schema> {
    pub parent_type: SchemaTypeWithFields<'schema>,
    pub resolver_field_name: FieldDefinitionName,
    pub nested_resolver_artifact_imports: HashSet<NestedResolverName>,
    pub reader_ast: ReaderAst,
    pub resolver_import_statement: ResolverImportStatement,
}

impl<'schema> NonFetchableResolver<'schema> {
    pub fn file_contents(&self) -> String {
        format!(
            "import type {{BoultonNonFetchableResolver, ReaderAst}} from '@boulton/react';\n\
            {}\n\
            {}\n\
            const readerAst: ReaderAst = {};\n\n\
            const artifact: BoultonNonFetchableResolver = {{\n\
            {}kind: 'NonFetchableResolver',\n\
            {}resolver,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            self.resolver_import_statement.0,
            nested_resolver_names_to_import_statement(&self.nested_resolver_artifact_imports),
            self.reader_ast.0,
            "  ",
            "  ",
            "  ",
        )
    }
}

fn generate_query_text(
    query_name: QueryOperationName,
    schema: &ValidatedSchema,
    merged_selection_set: &MergedSelectionSet,
    query_variables: &[WithSpan<ValidatedVariableDefinition>],
) -> QueryText {
    let mut query_text = String::new();

    let variable_text = write_variables_to_string(schema, query_variables);

    query_text.push_str(&format!("query {} {} {{\\\n", query_name, variable_text));
    write_selections(
        &mut query_text,
        schema,
        // TODO do not do this here, instead do it during validation, and topologically sort first
        &merged_selection_set,
        1,
    );
    query_text.push_str("}");
    QueryText(query_text)
}

fn write_variables_to_string(
    schema: &ValidatedSchema,
    variables: &[WithSpan<ValidatedVariableDefinition>],
) -> String {
    if variables.is_empty() {
        String::new()
    } else {
        let mut variable_text = String::new();
        variable_text.push('(');
        for (i, variable) in variables.iter().enumerate() {
            if i != 0 {
                variable_text.push_str(", ");
            }
            // TODO can we consume the variables here?
            let x: TypeAnnotation<UnvalidatedTypeName> =
                variable.item.type_.clone().map(|input_type_id| {
                    // schema.
                    let schema_input_type = schema.schema_data.lookup_input_type(input_type_id);
                    schema_input_type.name().into()
                });
            variable_text.push_str(&format!("${}: {}", variable.item.name, x));
        }
        variable_text.push(')');
        variable_text
    }
}

#[derive(Debug, Error)]
pub enum GenerateArtifactsError {
    #[error("Unable to write to artifact file at path {path:?}.\nMessage: {message:?}")]
    UnableToWriteToArtifactFile { path: PathBuf, message: io::Error },

    #[error("Unable to create directory at path {path:?}.\nMessage: {message:?}")]
    UnableToCreateDirectory { path: PathBuf, message: io::Error },

    #[error("Unable to delete directory at path {path:?}.\nMessage: {message:?}")]
    UnableToDeleteDirectory { path: PathBuf, message: io::Error },

    #[error("Unable to canonicalize path: {path:?}.\nMessage: {message:?}")]
    UnableToCanonicalizePath { path: PathBuf, message: io::Error },
}

fn generated_file_name(
    parent_type_name: TypeWithFieldsName,
    field_name: FieldDefinitionName,
) -> PathBuf {
    PathBuf::from(format!("{}__{}.boulton.ts", parent_type_name, field_name))
}

fn generated_file_path(project_root: &PathBuf, file_name: &PathBuf) -> PathBuf {
    project_root.join(file_name)
}

fn write_selections(
    query_text: &mut String,
    schema: &ValidatedSchema,
    items: &[WithSpan<Selection<TypeWithoutFieldsId, TypeWithFieldsId>>],
    indentation_level: u8,
) {
    for item in items.iter() {
        query_text.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
        match &item.item {
            Selection::Field(field) => match field {
                ScalarField(scalar_field) => {
                    if let Some(alias) = scalar_field.normalization_alias {
                        query_text.push_str(&format!("{}: ", alias));
                    }
                    let name = scalar_field.name.item;
                    let arguments = get_serialized_arguments(&scalar_field.arguments);
                    query_text.push_str(&format!("{}{},\\\n", name, arguments));
                }
                LinkedField(linked_field) => {
                    if let Some(alias) = linked_field.normalization_alias {
                        query_text.push_str(&format!("{}: ", alias));
                    }
                    let name = linked_field.name.item;
                    let arguments = get_serialized_arguments(&linked_field.arguments);
                    query_text.push_str(&format!("{}{} {{\\\n", name, arguments));
                    write_selections(
                        query_text,
                        schema,
                        &linked_field.selection_set_and_unwraps.selection_set,
                        indentation_level + 1,
                    );
                    query_text.push_str(&format!(
                        "{}}},\\\n",
                        "  ".repeat(indentation_level as usize)
                    ));
                }
            },
        }
    }
}

fn write_artifacts<'schema>(
    artifacts: impl Iterator<Item = Result<Artifact<'schema>, GenerateArtifactsError>> + 'schema,
    project_root: &PathBuf,
) -> Result<(), GenerateArtifactsError> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let project_root = current_dir.join(project_root).canonicalize().map_err(|e| {
        GenerateArtifactsError::UnableToCanonicalizePath {
            path: project_root.clone(),
            message: e,
        }
    })?;

    let generated_folder_root = project_root.join("__boulton");

    fs::remove_dir_all(&generated_folder_root).map_err(|e| {
        GenerateArtifactsError::UnableToDeleteDirectory {
            path: project_root.clone(),
            message: e,
        }
    })?;
    fs::create_dir_all(&generated_folder_root).map_err(|e| {
        GenerateArtifactsError::UnableToCreateDirectory {
            path: project_root.clone(),
            message: e,
        }
    })?;
    for artifact in artifacts {
        let artifact = artifact?;
        match artifact {
            Artifact::FetchableResolver(fetchable_resolver) => {
                let FetchableResolver {
                    query_name,
                    parent_type,
                    ..
                } = &fetchable_resolver;

                let generated_file_name =
                    generated_file_name(parent_type.name(), (*query_name).into());
                let generated_file_path =
                    generated_file_path(&generated_folder_root, &generated_file_name);

                let mut file = File::create(&generated_file_path).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;

                let file_contents = fetchable_resolver.file_contents();

                file.write(file_contents.as_bytes()).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;
            }
            Artifact::NonFetchableResolver(non_fetchable_resolver) => {
                let NonFetchableResolver {
                    parent_type,
                    resolver_field_name,
                    ..
                } = &non_fetchable_resolver;

                let generated_file_name =
                    generated_file_name(parent_type.name(), *resolver_field_name);
                let generated_file_path =
                    generated_file_path(&generated_folder_root, &generated_file_name);

                let mut file = File::create(&generated_file_path).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;

                let file_contents = non_fetchable_resolver.file_contents();

                file.write(file_contents.as_bytes()).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;
            }
        }
    }
    Ok(())
}

fn generate_query_type_declaration(
    schema: &ValidatedSchema,
    selection_set: &MergedSelectionSet,
    indentation_level: u8,
) -> Result<QueryTypeDeclaration, GenerateArtifactsError> {
    // TODO use unwraps
    let mut query_type_declaration = String::new();
    for selection in selection_set.iter() {
        write_query_types_from_selection(
            schema,
            &mut query_type_declaration,
            selection,
            indentation_level,
        )?;
    }
    Ok(QueryTypeDeclaration(query_type_declaration))
}

fn write_query_types_from_selection(
    schema: &ValidatedSchema,
    query_type_declaration: &mut String,
    selection: &WithSpan<Selection<TypeWithoutFieldsId, TypeWithFieldsId>>,
    indentation_level: u8,
) -> Result<(), GenerateArtifactsError> {
    query_type_declaration.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));

    match &selection.item {
        Selection::Field(field) => match field {
            ScalarField(scalar_field) => {
                let name_or_alias = scalar_field.name_or_alias();
                let type_ = schema
                    .schema_data
                    .lookup_type_without_fields(scalar_field.field)
                    .javascript_name();
                query_type_declaration.push_str(&format!("{}: {},\n", name_or_alias, type_));
            }
            LinkedField(linked_field) => {
                let name_or_alias = linked_field.name_or_alias();
                let inner = generate_query_type_declaration(
                    schema,
                    &linked_field.selection_set_and_unwraps.selection_set,
                    indentation_level + 1,
                )?;
                query_type_declaration.push_str(&format!(
                    "{}: {{\n{}{}}},\n",
                    name_or_alias,
                    inner.0,
                    "  ".repeat(indentation_level as usize)
                ));
            }
        },
    }
    Ok(())
}

fn generate_resolver_import_statement(
    resolver_name: FieldDefinitionName,
    resolver_path: ResolverDefinitionPath,
) -> ResolverImportStatement {
    // ../ gets us to the project root from the __boulton folder
    ResolverImportStatement(format!(
        "import {{ {} as resolver }} from '../{}';",
        resolver_name, resolver_path
    ))
}

fn generate_reader_ast<'schema>(
    schema: &'schema ValidatedSchema,
    selection_set_and_unwraps: &'schema ValidatedSelectionSetAndUnwraps,
    parent_type: SchemaTypeWithFields<'schema>,
    indentation_level: u8,
    nested_resolver_imports: &mut HashSet<NestedResolverName>,
) -> ReaderAst {
    let mut reader_ast = "[\n".to_string();
    for item in &selection_set_and_unwraps.selection_set {
        let s = generate_reader_ast_node(
            item,
            parent_type,
            schema,
            indentation_level + 1,
            nested_resolver_imports,
        );
        reader_ast.push_str(&s);
    }
    reader_ast.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    ReaderAst(reader_ast)
}

fn generate_reader_ast_node(
    item: &WithSpan<Selection<DefinedField<TypeWithoutFieldsId, ()>, TypeWithFieldsId>>,
    parent_type: SchemaTypeWithFields,
    schema: &ValidatedSchema,
    indentation_level: u8,
    nested_resolver_imports: &mut HashSet<NestedResolverName>,
) -> String {
    match &item.item {
        Selection::Field(field) => match field {
            ScalarField(scalar_field) => {
                let field_name = scalar_field.name.item;

                match scalar_field.field {
                    DefinedField::ServerField(_server_field) => {
                        let alias = scalar_field
                            .reader_alias
                            .map(|x| format!("\"{}\"", x.item))
                            .unwrap_or("null".to_string());
                        format!(
                            "{}{{\n{}kind: \"Scalar\",\n{}response_name: \"{}\",\n{}alias: {},\n{}}},\n",
                            "  ".repeat(indentation_level as usize),
                            "  ".repeat((indentation_level + 1) as usize),
                            "  ".repeat((indentation_level + 1) as usize),
                            field_name,
                            "  ".repeat((indentation_level + 1) as usize),
                            alias,
                            "  ".repeat((indentation_level) as usize),
                        )
                    }
                    DefinedField::ResolverField(_) => {
                        let alias = scalar_field.name_or_alias().item;
                        // This field is a resolver, so we need to look up the field in the
                        // schema.
                        let resolver_field_name = scalar_field.name.item;
                        let parent_field_id = parent_type
                            .fields()
                            .iter()
                            .find(|parent_field_id| {
                                let field = schema.field(**parent_field_id);
                                field.name == resolver_field_name.into()
                            })
                            .expect("expect field to exist");
                        let resolver_field = schema.field(*parent_field_id);
                        match &resolver_field.field_type {
                            DefinedField::ServerField(_) => panic!("Expected resolver"),
                            DefinedField::ResolverField(resolver_field) => {
                                let resolver_import_name = NestedResolverName(format!(
                                    "{}__{}",
                                    parent_type.name(),
                                    field_name
                                ));
                                let res = format!(
                                    "{}{{\n{}kind: \"Resolver\",\n{}alias: \"{}\",\n{}resolver: {},\n{}variant: {},\n{}}},\n",
                                    "  ".repeat(indentation_level as usize),
                                    "  ".repeat((indentation_level + 1) as usize),
                                    "  ".repeat((indentation_level + 1) as usize),
                                    alias,
                                    "  ".repeat((indentation_level + 1) as usize),
                                    resolver_import_name.0,
                                    "  ".repeat((indentation_level + 1) as usize),
                                    resolver_field.variant.map(|x| format!("\"{}\"", x)).unwrap_or_else(|| "null".to_string()),
                                    "  ".repeat(indentation_level as usize),
                                );
                                nested_resolver_imports.insert(resolver_import_name);
                                res
                            }
                        }
                    }
                }
            }
            LinkedField(linked_field) => {
                let name = linked_field.name.item;
                let alias = linked_field
                    .reader_alias
                    .map(|x| format!("\"{}\"", x.item))
                    .unwrap_or("null".to_string());
                let linked_field_type = schema
                    .schema_data
                    .lookup_type_with_fields(linked_field.field);
                let inner_reader_ast = generate_reader_ast(
                    schema,
                    &linked_field.selection_set_and_unwraps,
                    linked_field_type,
                    indentation_level + 1,
                    nested_resolver_imports,
                );
                format!(
                    "{}{{\n{}kind: \"Linked\",\n{}response_name: \"{}\",\n{}alias: {},\n{}selections: {},\n{}}},\n",
                    "  ".repeat(indentation_level as usize),
                    "  ".repeat((indentation_level + 1) as usize),
                    "  ".repeat((indentation_level + 1) as usize),
                    name,
                    "  ".repeat((indentation_level + 1) as usize),
                    alias,
                    "  ".repeat((indentation_level + 1) as usize),
                    inner_reader_ast.0, "  ".repeat(indentation_level as usize),
                )
            }
        },
    }
}

fn nested_resolver_names_to_import_statement(
    nested_resolver_imports: &HashSet<NestedResolverName>,
) -> String {
    let mut s = String::new();
    for import in nested_resolver_imports {
        s.push_str(&format!(
            "import {} from './{}.boulton';\n",
            import.0, import.0
        ));
    }
    s
}

fn get_serialized_arguments(arguments: &[WithSpan<SelectionFieldArgument>]) -> String {
    if arguments.is_empty() {
        return "".to_string();
    } else {
        let mut arguments = arguments.iter();
        let first = arguments.next().unwrap();
        let mut s = format!(
            "({}: {}",
            first.item.name.item,
            serialize_non_constant_value(&first.item.value.item)
        );
        for argument in arguments {
            s.push_str(&format!(
                ", {}: {}",
                argument.item.name.item,
                serialize_non_constant_value(&argument.item.value.item)
            ));
        }
        s.push_str(")");
        s
    }
}

fn serialize_non_constant_value(value: &NonConstantValue) -> String {
    match value {
        NonConstantValue::Variable(variable_name) => format!("${}", variable_name),
    }
}