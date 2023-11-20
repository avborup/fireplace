// TODO: Include the limitations described here: https://firebase.google.com/docs/firestore/query-data/queries#query_limitations
// TODO: Add doc comment to each implementor

/*
- [x] < less than
- [x] <= less than or equal to
- [x] == equal to
- [x] > greater than
- [x] >= greater than or equal to
- [x] != not equal to
- [x] array-contains
- [x] array-contains-any
- [ ] in
- [ ] not-in
*/

use firestore_grpc::v1::{
    structured_query::{
        composite_filter::Operator as CompositeFilterOperator,
        field_filter::Operator as FieldFilterOperator, filter::FilterType as GrpcFilterType,
        CompositeFilter as GrpcCompositeFilter, FieldFilter as GrpcFieldFilter, FieldReference,
        Filter as GrpcFilter,
    },
    Value,
};
use serde::Serialize;

use crate::error::FirebaseError;

use super::{
    client::FirestoreClient, reference::CollectionReference, serde::serialize_to_value_type,
};

/// Represents a Firestore query operator used to test a field's value against
/// a given filter.
///
/// It will be possible to put any arbitrary struct or value into a query
/// operator (as long as the value satisfies the trait bounds), but you will
/// likely want to stick with primitive types like integers, strings, or lists
/// of the two.
///
/// You will see that many of the implementors of this trait will impose
/// trait bounds on the type parameter `T`. This is only done to help you catch
/// potential logic errors in your program. For example, `LessThan` requires
/// that `T` implements `Ord` - but the value will not actually be compared
/// client-side but only on the server-side by Firestore. Therefore your
/// implementation of `Ord` will not affect the filtering behavior.
///
/// To see which query operators are supported by Firestore, see the [official Firestore documentation](https://firebase.google.com/docs/firestore/query-data/queries#query_operators).
pub trait QueryOperator<T: Serialize> {
    /// Returns the value that the document field will be checked against.
    fn get_value(self) -> T;

    /// Returns the Firestore field filter operator code that represents which
    /// filter operation will be applied to the value by Firestore.
    fn get_operator_code(&self) -> FieldFilterOperator;
}

pub struct GreaterThan<T: Ord + Serialize>(pub T);

impl<T: Ord + Serialize> QueryOperator<T> for GreaterThan<T> {
    fn get_value(self) -> T {
        self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::GreaterThan
    }
}

pub struct GreaterThanOrEqual<T: Ord + Serialize>(pub T);

impl<T: Ord + Serialize> QueryOperator<T> for GreaterThanOrEqual<T> {
    fn get_value(self) -> T {
        self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::GreaterThanOrEqual
    }
}

pub struct LessThan<T: Ord + Serialize>(pub T);

impl<T: Ord + Serialize> QueryOperator<T> for LessThan<T> {
    fn get_value(self) -> T {
        self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::LessThan
    }
}

pub struct LessThanOrEqual<T: Ord + Serialize>(pub T);

impl<T: Ord + Serialize> QueryOperator<T> for LessThanOrEqual<T> {
    fn get_value(self) -> T {
        self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::LessThanOrEqual
    }
}

pub struct EqualTo<T: PartialEq + Serialize>(pub T);

impl<T: PartialEq + Serialize> QueryOperator<T> for EqualTo<T> {
    fn get_value(self) -> T {
        self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::Equal
    }
}

pub struct NotEqual<T: PartialEq + Serialize>(pub T);

impl<T: PartialEq + Serialize> QueryOperator<T> for NotEqual<T> {
    fn get_value(self) -> T {
        self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::NotEqual
    }
}

pub struct ArrayContains<T: Eq + Serialize>(pub T);

impl<T: Eq + Serialize> QueryOperator<T> for ArrayContains<T> {
    fn get_value(self) -> T {
        self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::ArrayContains
    }
}

pub struct ArrayContainsAny<T: Eq + Serialize>(pub T);

impl<T: Eq + Serialize> QueryOperator<T> for ArrayContainsAny<T> {
    fn get_value(self) -> T {
        self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::ArrayContainsAny
    }
}

pub fn filter<'a, T: Serialize + 'a + Send>(
    field: impl Into<String> + 'a,
    check_against: impl QueryOperator<T> + 'a,
) -> Filter<'a> {
    let field_filter = create_field_filter(field.into(), check_against);
    Filter::Single(field_filter)
}

pub enum Filter<'a> {
    Composite(Vec<FieldFilter<'a>>),
    Single(FieldFilter<'a>),
}

pub struct FieldFilter<'a> {
    field: String,
    op: FieldFilterOperator,
    value: Box<dyn erased_serde::Serialize + 'a + Send>,
}

impl<'a> Filter<'a> {
    pub fn and<T: Serialize + 'a + Send>(
        self,
        field: impl Into<String> + 'a,
        check_against: impl QueryOperator<T> + 'a,
    ) -> Self {
        let other_field_filter = create_field_filter(field.into(), check_against);

        let new_filter = match self {
            Filter::Composite(mut filters) => {
                filters.push(other_field_filter);
                Filter::Composite(filters)
            }
            Filter::Single(filter) => Filter::Composite(vec![filter, other_field_filter]),
        };

        new_filter
    }
}

fn create_field_filter<'a, T, Q>(field: String, query_op: Q) -> FieldFilter<'a>
where
    T: Serialize + 'a + Send,
    Q: QueryOperator<T> + 'a,
{
    let op = query_op.get_operator_code();
    let value = query_op.get_value();

    FieldFilter {
        field,
        op,
        value: Box::new(value),
    }
}

pub(crate) fn try_into_grpc_filter(
    filter: Filter,
    root_resource_path: &str,
) -> Result<GrpcFilter, FirebaseError> {
    let filter_type = match filter {
        Filter::Single(filter) => {
            GrpcFilterType::FieldFilter(try_into_grpc_field_filter(filter, root_resource_path)?)
        }
        Filter::Composite(filters) => {
            let f = filters
                .into_iter()
                .map(|f| {
                    try_into_grpc_filter_type(f, root_resource_path).map(|ft| GrpcFilter {
                        filter_type: Some(ft),
                    })
                })
                .collect::<Result<Vec<_>, FirebaseError>>()?;
            GrpcFilterType::CompositeFilter(GrpcCompositeFilter {
                op: CompositeFilterOperator::And as i32,
                filters: f,
            })
        }
    };

    Ok(GrpcFilter {
        filter_type: Some(filter_type),
    })
}

fn try_into_grpc_filter_type(
    field_filter: FieldFilter,
    root_resource_path: &str,
) -> Result<GrpcFilterType, FirebaseError> {
    let value = serialize_to_value_type(&field_filter.value, root_resource_path)?;
    let firestore_value = Value {
        value_type: Some(value),
    };

    let filter_type = GrpcFilterType::FieldFilter(GrpcFieldFilter {
        field: Some(firestore_grpc::v1::structured_query::FieldReference {
            field_path: field_filter.field,
        }),
        op: field_filter.op as i32,
        value: Some(firestore_value),
    });

    Ok(filter_type)
}

fn try_into_grpc_field_filter(
    field_filter: FieldFilter,
    root_resource_path: &str,
) -> Result<GrpcFieldFilter, FirebaseError> {
    let value_type = serialize_to_value_type(&field_filter.value, root_resource_path)?;
    let value = Value {
        value_type: Some(value_type),
    };

    let grpc_field_filter = GrpcFieldFilter {
        field: Some(FieldReference {
            field_path: field_filter.field,
        }),
        op: field_filter.op as i32,
        value: Some(value),
    };

    Ok(grpc_field_filter)
}

pub(crate) struct ApiQueryOptions<'a> {
    pub parent: String,
    pub collection_name: String,
    pub filter: Option<Filter<'a>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    /// Whether to search descendant collections with the same name
    pub should_search_descendants: bool,
}

impl<'a> ApiQueryOptions<'a> {
    pub(crate) fn from_query<T>(client: &FirestoreClient, query: T) -> Self
    where
        T: FirestoreQuery<'a>,
    {
        let parent_path = query
            .parent_path()
            .map(|p| client.get_name_with(p))
            .unwrap_or_else(|| client.root_resource_path().to_string());

        Self {
            parent: parent_path,
            collection_name: query.collection_name().to_string(),
            limit: query.limit(),
            offset: query.offset(),
            should_search_descendants: query.should_search_descendants(),
            filter: query.filter(),
        }
    }
}

pub trait FirestoreQuery<'a> {
    fn filter(self) -> Option<Filter<'a>>;
    fn collection_name(&self) -> &str;
    fn parent_path(&self) -> Option<String>;
    fn should_search_descendants(&self) -> bool;
    fn limit(&self) -> Option<i32>;
    fn offset(&self) -> Option<i32>;
}

pub struct CollectionGroupQuery<'a> {
    collection_name: String,
    filter: Option<Filter<'a>>,
}

pub fn collection_group<'a>(collection_name: impl Into<String>) -> CollectionGroupQuery<'a> {
    CollectionGroupQuery::new(collection_name)
}

impl<'a> CollectionGroupQuery<'a> {
    pub fn new(collection_name: impl Into<String>) -> Self {
        CollectionGroupQuery {
            collection_name: collection_name.into(),
            filter: None,
        }
    }

    pub fn with_filter(mut self, filter: Filter<'a>) -> Self {
        self.filter = Some(filter);
        self
    }
}

impl<'a> FirestoreQuery<'a> for CollectionGroupQuery<'a> {
    fn filter(self) -> Option<Filter<'a>> {
        self.filter
    }

    fn collection_name(&self) -> &str {
        &self.collection_name
    }

    fn parent_path(&self) -> Option<String> {
        None
    }

    fn should_search_descendants(&self) -> bool {
        true
    }

    fn limit(&self) -> Option<i32> {
        None
    }

    fn offset(&self) -> Option<i32> {
        None
    }
}

impl<'a> FirestoreQuery<'a> for CollectionReference {
    fn filter(self) -> Option<Filter<'a>> {
        None
    }

    fn parent_path(&self) -> Option<String> {
        self.parent().map(|p| p.to_string())
    }

    fn collection_name(&self) -> &str {
        self.name()
    }

    fn should_search_descendants(&self) -> bool {
        false
    }

    fn limit(&self) -> Option<i32> {
        None
    }

    fn offset(&self) -> Option<i32> {
        None
    }
}

pub struct CollectionQuery<'a> {
    collection: CollectionReference,
    filter: Option<Filter<'a>>,
}

impl<'a> CollectionQuery<'a> {
    pub fn new(collection: CollectionReference) -> Self {
        CollectionQuery {
            collection,
            filter: None,
        }
    }

    pub fn with_filter(mut self, filter: Filter<'a>) -> Self {
        self.filter = Some(filter);
        self
    }
}

impl<'a> FirestoreQuery<'a> for CollectionQuery<'a> {
    fn filter(self) -> Option<Filter<'a>> {
        self.filter
    }

    fn parent_path(&self) -> Option<String> {
        self.collection.parent_path()
    }

    fn collection_name(&self) -> &str {
        self.collection.collection_name()
    }

    fn should_search_descendants(&self) -> bool {
        self.collection.should_search_descendants()
    }

    fn limit(&self) -> Option<i32> {
        self.collection.limit()
    }

    fn offset(&self) -> Option<i32> {
        None
    }
}

#[cfg(test)]
mod tests {
    use firestore_grpc::v1::value::ValueType;

    use crate::firestore::collection;

    use super::*;

    #[test]
    fn combine_operators() {
        let query = filter("age", LessThan(42)).and("name", EqualTo("Bob"));
        let serialized = try_into_grpc_filter(query, "").unwrap();

        let expected = GrpcFilter {
            filter_type: Some(GrpcFilterType::CompositeFilter(GrpcCompositeFilter {
                op: CompositeFilterOperator::And as i32,
                filters: vec![
                    GrpcFilter {
                        filter_type: Some(GrpcFilterType::FieldFilter(GrpcFieldFilter {
                            field: Some(FieldReference {
                                field_path: "age".to_string(),
                            }),
                            op: FieldFilterOperator::LessThan as i32,
                            value: Some(Value {
                                value_type: Some(ValueType::IntegerValue(42)),
                            }),
                        })),
                    },
                    GrpcFilter {
                        filter_type: Some(GrpcFilterType::FieldFilter(GrpcFieldFilter {
                            field: Some(FieldReference {
                                field_path: "name".to_string(),
                            }),
                            op: FieldFilterOperator::Equal as i32,
                            value: Some(Value {
                                value_type: Some(ValueType::StringValue("Bob".to_string())),
                            }),
                        })),
                    },
                ],
            })),
        };

        assert_eq!(serialized, expected);
    }

    #[test]
    fn single_operator() {
        let query = filter("age", EqualTo(collection("users").doc("bob")));
        let serialized = try_into_grpc_filter(query, "prefix").unwrap();

        let expected = GrpcFilter {
            filter_type: Some(GrpcFilterType::FieldFilter(GrpcFieldFilter {
                field: Some(FieldReference {
                    field_path: "age".to_string(),
                }),
                op: FieldFilterOperator::Equal as i32,
                value: Some(Value {
                    value_type: Some(ValueType::ReferenceValue("prefix/users/bob".to_string())),
                }),
            })),
        };

        assert_eq!(serialized, expected);
    }

    #[test]
    fn implements_send() {
        fn assert_send<T: Send>() {}
        assert_send::<super::Filter>();
    }
}
