use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, ClientSelectableName, DescriptionValue,
    ObjectTypeAndFieldName, WithSpan,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    ClientFieldId, ClientPointerId, SelectionType, ServerEntityId, ServerObjectId, TypeAnnotation,
    VariableDefinition,
};

use crate::{
    ClientFieldVariant, OutputFormat, RefetchStrategy, UserWrittenClientPointerInfo,
    ValidatedLinkedFieldAssociatedData, ValidatedScalarSelectionAssociatedData, ValidatedSelection,
};

pub type ClientFieldOrPointerId = SelectionType<ClientFieldId, ClientPointerId>;

#[derive(Debug)]
pub struct ClientField<TOutputFormat: OutputFormat> {
    pub description: Option<DescriptionValue>,
    pub name: ClientScalarSelectableName,
    pub id: ClientFieldId,
    pub reader_selection_set: Vec<WithSpan<ValidatedSelection>>,

    // None -> not refetchable
    // TODO - this is only used if variant === imperatively loaded field
    // consider moving it into that struct.
    pub refetch_strategy: Option<
        RefetchStrategy<ValidatedScalarSelectionAssociatedData, ValidatedLinkedFieldAssociatedData>,
    >,

    // TODO we should probably model this differently
    pub variant: ClientFieldVariant,

    pub variable_definitions: Vec<WithSpan<VariableDefinition<ServerEntityId>>>,

    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldName,

    pub parent_object_id: ServerObjectId,
    pub output_format: PhantomData<TOutputFormat>,
}

#[derive(Debug)]
pub struct ClientPointer<TOutputFormat: OutputFormat> {
    pub description: Option<DescriptionValue>,
    pub name: ClientObjectSelectableName,
    pub id: ClientPointerId,
    pub to: TypeAnnotation<ServerObjectId>,

    pub reader_selection_set: Vec<WithSpan<ValidatedSelection>>,

    pub refetch_strategy:
        RefetchStrategy<ValidatedScalarSelectionAssociatedData, ValidatedLinkedFieldAssociatedData>,

    pub variable_definitions: Vec<WithSpan<VariableDefinition<ServerEntityId>>>,

    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldName,

    pub parent_object_id: ServerObjectId,

    pub output_format: PhantomData<TOutputFormat>,
    pub info: UserWrittenClientPointerInfo,
}

#[impl_for_selection_type]
pub trait ClientFieldOrPointer {
    fn description(&self) -> Option<DescriptionValue>;
    fn name(&self) -> ClientSelectableName;
    fn id(&self) -> ClientFieldOrPointerId;
    fn type_and_field(&self) -> ObjectTypeAndFieldName;
    fn parent_object_id(&self) -> ServerObjectId;
    // the following are unsupported, for now, because the return values include a generic
    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>];
    fn refetch_strategy(
        &self,
    ) -> Option<
        &RefetchStrategy<
            ValidatedScalarSelectionAssociatedData,
            ValidatedLinkedFieldAssociatedData,
        >,
    >;
    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>];

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityId>>];

    fn client_type(&self) -> &'static str;
}

impl<TOutputFormat: OutputFormat> ClientFieldOrPointer for &ClientField<TOutputFormat> {
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.into()
    }

    fn id(&self) -> ClientFieldOrPointerId {
        SelectionType::Scalar(self.id)
    }

    fn type_and_field(&self) -> ObjectTypeAndFieldName {
        self.type_and_field
    }

    fn parent_object_id(&self) -> ServerObjectId {
        self.parent_object_id
    }

    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>] {
        &self.reader_selection_set
    }

    fn refetch_strategy(
        &self,
    ) -> Option<
        &RefetchStrategy<
            ValidatedScalarSelectionAssociatedData,
            ValidatedLinkedFieldAssociatedData,
        >,
    > {
        self.refetch_strategy.as_ref()
    }

    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>] {
        match self.variant {
            ClientFieldVariant::ImperativelyLoadedField(_) => self
                .refetch_strategy
                .as_ref()
                .map(|strategy| strategy.refetch_selection_set())
                .expect(
                    "Expected imperatively loaded field to have refetch selection set. \
                    This is indicative of a bug in Isograph.",
                ),
            _ => &self.reader_selection_set,
        }
    }

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityId>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "field"
    }
}

impl<TOutputFormat: OutputFormat> ClientFieldOrPointer for &ClientPointer<TOutputFormat> {
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.into()
    }

    fn id(&self) -> ClientFieldOrPointerId {
        SelectionType::Object(self.id)
    }

    fn type_and_field(&self) -> ObjectTypeAndFieldName {
        self.type_and_field
    }

    fn parent_object_id(&self) -> ServerObjectId {
        self.parent_object_id
    }

    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>] {
        &self.reader_selection_set
    }

    fn refetch_strategy(
        &self,
    ) -> Option<
        &RefetchStrategy<
            ValidatedScalarSelectionAssociatedData,
            ValidatedLinkedFieldAssociatedData,
        >,
    > {
        Some(&self.refetch_strategy)
    }

    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>] {
        &self.reader_selection_set
    }

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityId>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "pointer"
    }
}
