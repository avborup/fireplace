use std::fmt::Display;
use std::future;
use std::pin::Pin;

use anyhow::{anyhow, Context};
use firestore_grpc::tonic;
use firestore_grpc::v1::firestore_client::FirestoreClient as GrpcFirestoreClient;
use firestore_grpc::v1::precondition::ConditionType;
use firestore_grpc::v1::run_query_request::QueryType;
use firestore_grpc::v1::structured_aggregation_query::aggregation;
use firestore_grpc::v1::structured_query::CollectionSelector;
use firestore_grpc::v1::value::ValueType;
use firestore_grpc::v1::{
    run_aggregation_query_request, structured_aggregation_query, CreateDocumentRequest,
    DeleteDocumentRequest, DocumentMask, Precondition, RunAggregationQueryRequest, RunQueryRequest,
    StructuredAggregationQuery, StructuredQuery, UpdateDocumentRequest,
};
use firestore_grpc::{
    tonic::{
        codegen::InterceptedService, metadata::MetadataValue, transport::Channel, Request, Status,
    },
    v1::GetDocumentRequest,
};
use futures::{Stream, StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::FirebaseError;
use crate::firestore::serde::deserialize_firestore_document_fields;
use crate::ServiceAccount;

use super::query::{try_into_grpc_filter, ApiQueryOptions, Filter, FirestoreQuery};
use super::reference::{CollectionReference, DocumentReference};
use super::serde::{strip_reference_prefix, DocumentSerializer};
use super::token_provider::FirestoreTokenProvider;

mod options;

pub use options::FirestoreClientOptions;

type FirebaseStream<'i, T, E> = Pin<Box<dyn Stream<Item = Result<T, E>> + Send + 'i>>;

type InterceptorFunction = Box<dyn FnMut(Request<()>) -> Result<Request<()>, Status> + Send>;

pub struct FirestoreClient {
    options: FirestoreClientOptions,
    client: GrpcFirestoreClient<InterceptedService<Channel, InterceptorFunction>>,
    grpc_channel: Channel,
    project_id: String,
    token_provider: FirestoreTokenProvider,
    root_resource_path: String,
}

#[derive(Eq, PartialEq, Debug)]
pub struct FirestoreDocument<T> {
    /// The resource name of the document, for example
    /// `projects/{project_id}/databases/{database_id}/documents/{document_path}`.
    pub id: String,
    /// The deserialized document data.
    pub data: T,
    /// The time at which the document was created, in seconds of UTC time since Unix epoch.
    pub create_time: Option<i64>,
    /// The time at which the document was last updated, in seconds of UTC time since Unix epoch.
    pub update_time: Option<i64>,
}

impl Clone for FirestoreClient {
    fn clone(&self) -> Self {
        Self::from_channel(
            self.grpc_channel.clone(),
            self.token_provider.clone(),
            &self.project_id,
            self.options.clone(),
        )
    }
}

impl<T> FirestoreDocument<T> {
    /// Obtain a document reference to this document. May fail if the resource
    /// path is invalid.
    pub fn document_reference(&self) -> Result<DocumentReference, FirebaseError> {
        let stripped_of_resource = strip_reference_prefix(&self.id);
        let doc_ref = DocumentReference::try_from(stripped_of_resource)?;
        Ok(doc_ref)
    }
}

fn create_auth_interceptor(mut token_provider: FirestoreTokenProvider) -> InterceptorFunction {
    Box::new(move |mut req: Request<()>| {
        let token = token_provider
            .get_token()
            .map_err(|_| Status::unauthenticated("Could not get token from token provider"))?;

        let bearer_token = format!("Bearer {}", token);
        let mut header_value = MetadataValue::from_str(&bearer_token).map_err(|_| {
            Status::unauthenticated("Failed to construct metadata value for authorization token")
        })?;
        header_value.set_sensitive(true);

        req.metadata_mut().insert("authorization", header_value);

        Ok(req)
    })
}

impl FirestoreClient {
    /// Initialise a new client that can be used to interact with a Firestore
    /// database.
    pub async fn initialise(
        service_account: ServiceAccount,
        options: FirestoreClientOptions,
    ) -> Result<Self, FirebaseError> {
        let channel = Channel::from_shared(options.host_url.clone())
            .context("Failed to create gRPC channel")?
            .connect()
            .await
            .context("Failed to create channel to endpoint")?;

        let project_id = service_account.project_id.clone();
        let token_provider = FirestoreTokenProvider::new(service_account);

        Ok(Self::from_channel(
            channel,
            token_provider,
            &project_id,
            options,
        ))
    }

    fn from_channel(
        channel: Channel,
        token_provider: FirestoreTokenProvider,
        project_id: &str,
        options: FirestoreClientOptions,
    ) -> Self {
        // Cloning a channel is supposedly very cheap and encouraged be tonic's
        // documentation.
        let service = GrpcFirestoreClient::with_interceptor(
            channel.clone(),
            create_auth_interceptor(token_provider.clone()),
        );

        let resource_path = format!("projects/{}/databases/(default)/documents", project_id);

        Self {
            client: service,
            project_id: project_id.to_string(),
            token_provider,
            grpc_channel: channel,
            root_resource_path: resource_path,
            options,
        }
    }

    /// Retrieve a document from Firestore at the given document reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use serde::{Serialize, Deserialize};
    /// # use fireplace::firestore::collection;
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// struct Person {
    ///    name: String,
    /// }
    ///
    /// let collection_ref = collection("people");
    ///
    /// // First we create the document in the database
    /// let doc_id = client
    ///    .create_document(&collection_ref, &Person { name: "Luke Skywalker".to_string() })
    ///    .await
    ///    .unwrap();
    ///
    /// // Then we can retrieve it
    /// let doc_ref = collection_ref.doc(doc_id);
    /// let doc = client
    ///     .get_document(&doc_ref)
    ///     .await
    ///     .unwrap();
    ///
    /// assert_eq!(
    ///     doc,
    ///     Some(Person { name: "Luke Skywalker".to_string() })
    /// );
    ///
    /// // This document doesn't exist in the database, so we get a None.
    /// let doc_ref = collection("people").doc("luke-right-hand");
    /// let doc = client
    ///     .get_document::<Person>(&doc_ref)
    ///     .await
    ///     .unwrap();
    ///
    /// assert_eq!(doc, None);
    /// # }
    /// ```
    pub async fn get_document<'de, T: Deserialize<'de>>(
        &mut self,
        doc_ref: &DocumentReference,
    ) -> Result<Option<T>, FirebaseError> {
        let request = GetDocumentRequest {
            name: self.get_name_with(doc_ref),
            mask: None,
            consistency_selector: None,
        };

        let res = self.client.get_document(request).await;

        match res {
            Ok(res) => {
                let doc = res.into_inner();
                let deserialized = deserialize_firestore_document_fields::<T>(doc.fields)?;
                Ok(Some(deserialized))
            }
            Err(err) if err.code() == tonic::Code::NotFound => Ok(None),
            Err(err) => Err(anyhow!(err).into()),
        }
    }

    /// Creates a document in Firestore in the given collection, letting
    /// Firestore generate the ID for you. The ID of the created document will
    /// be returned.
    ///
    /// Returns an error if the document already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use fireplace::firestore::collection;
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// let collection_ref = collection("greetings");
    /// let doc_to_create = serde_json::json!({ "message": "Hi Mom!" });
    ///
    /// let first_doc_id = client
    ///     .create_document(&collection_ref, &doc_to_create)
    ///     .await
    ///     .unwrap();
    ///
    /// println!("Created document with ID: {}", first_doc_id);
    /// # }
    /// ```
    pub async fn create_document<T: Serialize>(
        &mut self,
        collection_ref: &CollectionReference,
        document: &T,
    ) -> Result<String, FirebaseError> {
        self.create_document_internal(collection_ref, None, document)
            .await
    }

    /// Creates a document in Firestore at the given document reference.
    /// Returns the ID of the created document.
    ///
    /// Returns an error if the document already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use fireplace::{firestore::collection, error::FirebaseError};
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// let collection_ref = collection("greetings");
    /// let doc_to_create = serde_json::json!({ "message": "Hi Mom!" });
    ///
    /// let first_doc_id = client
    ///     .create_document(&collection_ref, &doc_to_create)
    ///     .await
    ///     .unwrap();
    ///
    /// // If we create another document with the same ID, it should fail
    /// let second_create_result = client
    ///     .create_document_at_ref(&collection_ref.doc(first_doc_id), &doc_to_create)
    ///     .await;
    ///
    /// assert!(matches!(
    ///     second_create_result.unwrap_err(),
    ///     FirebaseError::DocumentAlreadyExists(_),
    /// ));
    /// # }
    /// ```
    pub async fn create_document_at_ref<T: Serialize>(
        &mut self,
        doc_ref: &DocumentReference,
        document: &T,
    ) -> Result<String, FirebaseError> {
        self.create_document_internal(&doc_ref.parent(), Some(doc_ref.id().to_string()), document)
            .await
    }

    async fn create_document_internal<T: Serialize>(
        &mut self,
        collection_ref: &CollectionReference,
        document_id: Option<String>,
        document: &T,
    ) -> Result<String, FirebaseError> {
        // We should provide no name or timestamps when creating a document
        // according to Google's Firestore API reference.
        let doc = self.serializer().serialize(document)?;

        let (parent, collection_name) = self.split_collection_parent_and_name(collection_ref);
        let request = CreateDocumentRequest {
            parent,
            collection_id: collection_name,
            // Passing an empty string means that Firestore will generate a
            // document ID for us.
            document_id: document_id.unwrap_or_default(),
            document: Some(doc),
            mask: Some(DocumentMask {
                field_paths: vec![],
            }),
        };

        let res = self.client.create_document(request).await;

        match res {
            Ok(r) => {
                let created_doc = r.into_inner();
                let created_doc_id = created_doc
                    .name
                    .rsplit_once('/')
                    .map(|(_, id)| id.to_string())
                    .context("Could not get document ID from resource path")?;
                Ok(created_doc_id)
            }
            Err(err) if err.code() == tonic::Code::AlreadyExists => Err(
                FirebaseError::DocumentAlreadyExists(err.message().to_string()),
            ),
            Err(err) => Err(anyhow!(err).into()),
        }
    }

    /// Sets a document at the given document reference. If it doesn't already,
    /// exist, it is created - and if it does exist already, it is overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use fireplace::firestore::collection;
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// let doc_ref = collection("greetings").doc("some-doc-id-to-set");
    /// let doc = serde_json::json!({ "message": "Hello, world!".to_string() });
    ///
    /// // We can upsert the document in the database
    /// client.set_document(&doc_ref, &doc).await.unwrap();
    ///
    /// // We can write to the same document reference again, and it will overwrite
    /// // the existing value document
    /// client.set_document(&doc_ref, &doc).await.unwrap();
    /// # }
    /// ```
    pub async fn set_document<T: Serialize>(
        &mut self,
        doc_ref: &DocumentReference,
        document: &T,
    ) -> Result<(), FirebaseError> {
        let name = self.get_name_with(doc_ref);
        let doc = self.serializer().name(name).serialize(document)?;

        let request = UpdateDocumentRequest {
            document: Some(doc),
            update_mask: None,
            mask: Some(DocumentMask {
                field_paths: vec![],
            }),
            current_document: None,
        };

        self.client
            .update_document(request)
            .await
            .map_err(|err| anyhow!(err))?;

        Ok(())
    }

    /// Similar to [`set_document`](Self::set_document) but only upserts the
    /// fields specified in the `fields` argument.
    ///
    /// Generic type parameters: `I` for the input type that's to be serialized
    /// and `O` for the returned (full) document that should be deserialized.
    ///
    /// # Field selectors
    ///
    /// A simple field name contains only characters `a` to `z`, `A` to `Z`, `0`
    /// to `9`, or `_`, and must not start with `0` to `9`. For example,
    /// `foo_bar_17`.
    ///
    /// Field names matching the regular expression `__.*__` are reserved.
    /// Reserved field names are forbidden except in certain documented
    /// contexts. The map keys, represented as UTF-8, must not exceed 1,500
    /// bytes and cannot be empty.
    ///
    /// Field paths may be used in other contexts to refer to structured fields
    /// defined here. For map-like values, the field path is represented by the
    /// simple or quoted field names of the containing fields, delimited by `.`.
    /// For example, the field `"foo": { "x&y": "hello" }` would be represented
    /// by the field path `foo.x&y`.
    ///
    /// The above is a slightly modified description from the [Firestore API reference](https://firebase.google.com/docs/firestore/reference/rpc/google.firestore.v1#document).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use serde::{Deserialize, Serialize};
    /// # use fireplace::firestore::collection;
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// struct TestType {
    ///     label: String,
    ///     nested: NestedItem,
    /// }
    ///
    /// #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// #[serde(rename_all = "camelCase")]
    /// struct NestedItem {
    ///     field_a: String,
    ///     field_b: String,
    /// }
    ///
    /// // First, we set a document in the database
    /// let doc_ref = collection("greetings").doc("some-doc-id-to-set-merge");
    /// client
    ///     .set_document(
    ///         &doc_ref,
    ///         &TestType {
    ///             label: "Hello".to_string(),
    ///             nested: NestedItem {
    ///                 field_a: "A".to_string(),
    ///                 field_b: "B".to_string(),
    ///             },
    ///         },
    ///     )
    ///     .await
    ///     .unwrap();
    ///
    /// // Then we can update some fields of a document in the database. For
    /// // example, we can specify a top-level field ("label") or a nested field
    /// // ("nested.fieldA").
    /// let updated_doc: TestType = client
    ///     .set_document_merge(
    ///         &doc_ref,
    ///         &TestType {
    ///             label: "World".to_string(),
    ///             nested: NestedItem {
    ///                 field_a: "C".to_string(),
    ///                 field_b: "D".to_string(),
    ///             },
    ///         },
    ///         &["label", "nested.fieldB"],
    ///     )
    ///     .await
    ///     .unwrap();
    ///
    /// // Only the specified fields are updated. Despite `nested.field_a` having a
    /// // new value in the update, the value in the database is not changed.
    /// assert_eq!(
    ///     updated_doc,
    ///     TestType {
    ///         label: "World".to_string(),
    ///         nested: NestedItem {
    ///             field_a: "A".to_string(), // Notice this field did not change
    ///             field_b: "D".to_string(),
    ///         },
    ///     }
    /// );
    /// # }
    /// ```
    pub async fn set_document_merge<'de, I: Serialize, O: Deserialize<'de>>(
        &mut self,
        doc_ref: &DocumentReference,
        document: &I,
        // In reality we need a `Vec<String>`, but in by far most of the use-
        // cases, the user will be hard-coding the field names, so this makes
        // it much easier to just do that.
        fields: &[&str],
    ) -> Result<O, FirebaseError> {
        let name = self.get_name_with(doc_ref);
        let doc = self.serializer().name(name).serialize(document)?;

        let request = UpdateDocumentRequest {
            document: Some(doc),
            update_mask: Some(DocumentMask {
                field_paths: fields.iter().map(|s| s.to_string()).collect(),
            }),
            mask: None,
            current_document: None,
        };

        let res = self
            .client
            .update_document(request)
            .await
            .map_err(|err| anyhow!(err))?;

        let doc = res.into_inner();
        let deserialized = deserialize_firestore_document_fields::<O>(doc.fields)?;

        Ok(deserialized)
    }

    /// Updates a document at the given document reference. Differs from
    /// [`set_document`](Self::set_document), in that this function assumes
    /// that the document already exists, and will return a
    /// [`DocumentNotfound`](FirebaseError::DocumentNotfound) error
    /// if it cannot be found.
    ///
    /// # Examples
    /// ```
    /// # use fireplace::{firestore::collection, error::FirebaseError};
    /// # use serde::{Deserialize, Serialize};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// struct Person {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// let doc_ref = collection("people").doc("jake");
    /// let mut jake = Person {
    ///     name: "Jake".to_string(),
    ///     age: 30,
    /// };
    ///
    /// // We set a document in the database
    /// client.set_document(&doc_ref, &jake).await?;
    ///
    /// // Then we update the document
    /// jake.age = 31;
    /// client.update_document(&doc_ref, &jake).await?;
    ///
    /// // We see that the document has been updated in the database
    /// assert_eq!(Some(jake), client.get_document(&doc_ref).await?);
    ///
    /// let doc_ref = collection("people").doc("mary");
    /// let mary = Person {
    ///     name: "Mary".to_string(),
    ///     age: 25,
    /// };
    ///
    /// // If we try to update a document that does not exist, we get an error
    /// let result = client.update_document(&doc_ref, &mary).await;
    /// assert!(matches!(
    ///     result.unwrap_err(),
    ///     FirebaseError::DocumentNotfound(_),
    /// ));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_document<T: Serialize>(
        &mut self,
        doc_ref: &DocumentReference,
        document: &T,
    ) -> Result<(), FirebaseError> {
        let name = self.get_name_with(doc_ref);
        let doc = self.serializer().name(name).serialize(document)?;

        let request = UpdateDocumentRequest {
            document: Some(doc),
            update_mask: None,
            mask: Some(DocumentMask {
                field_paths: vec![],
            }),
            current_document: document_exists_precondition(),
        };

        self.client
            .update_document(request)
            .await
            .map_err(not_found_err())?;

        Ok(())
    }

    /// Deletes a document from the database. Whether the document exists or not
    /// makes no difference.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// use fireplace::firestore::collection;
    /// use ulid::Ulid;
    ///
    /// let doc_ref = collection("pokemon").doc("pikachu");
    ///
    /// client
    ///     .set_document(&doc_ref, &serde_json::json!({ "name": "Pikachu" }))
    ///     .await?;
    ///
    /// client.delete_document(&doc_ref).await?;
    ///
    /// assert_eq!(
    ///     client.get_document::<serde_json::Value>(&doc_ref).await?,
    ///     None
    /// );
    ///
    /// // We can also just "delete" non-existing documents without error
    /// client
    ///     .delete_document(&collection("pokemon").doc(Ulid::new()))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_document(
        &mut self,
        doc_ref: &DocumentReference,
    ) -> Result<(), FirebaseError> {
        let name = self.get_name_with(doc_ref);

        let request = DeleteDocumentRequest {
            name,
            current_document: None,
        };

        self.client
            .delete_document(request)
            .await
            .context("Failed to delete document")?;

        Ok(())
    }

    /// Deletes a document at the given document reference. Differs from
    /// [delete_document](Self::delete_document), in that this function assumes
    /// that the document already exists, and will return a
    /// [`DocumentNotfound`](FirebaseError::DocumentNotfound) error
    /// if it cannot be found.
    ///
    /// # Examples
    /// ```
    /// # use fireplace::{firestore::collection, error::FirebaseError};
    /// # use serde::{Deserialize, Serialize};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// struct Person {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// let doc_ref = collection("people").doc("jake");
    /// let jake = Person {
    ///     name: "Jake".to_string(),
    ///     age: 30,
    /// };
    ///
    /// // We set a document in the database
    /// client.set_document(&doc_ref, &jake).await.unwrap();
    ///
    /// // Then we delete the document
    /// client.delete_existing_document(&doc_ref).await?;
    /// assert_eq!(None, client.get_document::<serde_json::Value>(&doc_ref).await?);
    ///
    /// // If we try to delete a document that does not exist, we get an error
    /// let result = client.delete_existing_document(&doc_ref).await;
    /// assert!(matches!(
    ///     result.unwrap_err(),
    ///     FirebaseError::DocumentNotfound(_),
    /// ));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_existing_document(
        &mut self,
        doc_ref: &DocumentReference,
    ) -> Result<(), FirebaseError> {
        let name = self.get_name_with(doc_ref);

        let request = DeleteDocumentRequest {
            name,
            current_document: document_exists_precondition(),
        };

        self.client
            .delete_document(request)
            .await
            .map_err(not_found_err())?;

        Ok(())
    }

    /// Query a collection for documents that fulfill the given criteria.
    ///
    /// Returns a [`Stream`](futures::stream::Stream) of query results,
    /// allowing you to process results as they are coming in.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use fireplace::firestore::collection;
    /// # use serde::{Deserialize, Serialize};
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await?;
    /// #
    /// use fireplace::firestore::query::{filter, ArrayContains, EqualTo};
    /// use futures::TryStreamExt;
    ///
    /// #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    /// struct Pizza {
    ///     name: String,
    ///     toppings: Vec<String>,
    /// }
    ///
    /// // Instantiate our example pizzas
    /// let pepperoni = Pizza {
    ///     name: "Pepperoni".into(),
    ///     toppings: vec!["pepperoni".into(), "cheese".into()],
    /// };
    /// let hawaii = Pizza {
    ///     name: "Hawaii".into(),
    ///     toppings: vec!["pineapple".into(), "ham".into(), "cheese".into()],
    /// };
    ///
    /// // Create the pizzas in the database
    /// client
    ///     .set_document(&collection("pizzas").doc("pepperoni"), &pepperoni)
    ///     .await?;
    /// client
    ///     .set_document(&collection("pizzas").doc("hawaii"), &hawaii)
    ///     .await?;
    ///
    /// // Query for pizzas whose name field is "Hawaii"
    /// let hawaii_results: Vec<Pizza> = client
    ///     .query(&collection("pizzas"), filter("name", EqualTo("Hawaii")))
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// // We expect a single search hit - the hawaii pizza.
    /// assert_eq!(hawaii_results, vec![hawaii.clone()]);
    ///
    /// // Query for pizzas that have a "cheese" entry in the toppings list.
    /// let mut cheese_results: Vec<Pizza> = client
    ///     .query(
    ///         &collection("pizzas"),
    ///         filter("toppings", ArrayContains("cheese")),
    ///     )
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// // We don't have a guaranteed ordering of the query results, so we sort
    /// // them by name to make sure our equality check works.
    /// cheese_results.sort_by(|a, b| a.name.cmp(&b.name));
    ///
    /// // We expect both pizzas to be found
    /// assert_eq!(cheese_results, vec![hawaii, pepperoni]);
    ///
    /// // Query for pizzas with the name "pasta salad".
    /// let mut pasta_salad_results: Vec<Pizza> = client
    ///     .query(&collection("pizzas"), filter("name", EqualTo("pasta salad")))
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// // We expect no results
    /// assert_eq!(pasta_salad_results, vec![]);
    /// # Ok(())
    /// # }
    pub async fn query<'de, 'a, T: Deserialize<'de> + 'a>(
        &'a mut self,
        collection: &CollectionReference,
        filter: Filter<'a>,
    ) -> Result<FirebaseStream<T, FirebaseError>, FirebaseError> {
        let (parent, collection_name) = self.split_collection_parent_and_name(collection);

        self.query_internal(ApiQueryOptions {
            parent,
            collection_name,
            filter: Some(filter),
            limit: None,
            should_search_descendants: false,
            offset: None,
        })
        .await
    }

    /// The same as [`query`](Self::query), but only returns the first result.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use fireplace::firestore::collection;
    /// # use serde::{Deserialize, Serialize};
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await?;
    /// #
    /// use fireplace::firestore::query::{filter, EqualTo};
    ///
    /// #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    /// struct Pizza {
    ///     name: String,
    /// }
    ///
    /// let margherita = Pizza {
    ///     name: "Margherita".into(),
    /// };
    ///
    /// client
    ///     .set_document(&collection("pizzas").doc("margherita"), &margherita)
    ///     .await?;
    ///
    /// // Query for the Margherita pizza by name
    /// let mut margherita_result: Option<Pizza> = client
    ///     .query_one(
    ///         &collection("pizzas"),
    ///         filter("name", EqualTo("Margherita")),
    ///     )
    ///     .await?;
    ///
    /// // We expect a single search hit - the margherita pizza.
    /// assert_eq!(margherita_result, Some(margherita.clone()));
    ///
    /// // Query for pizzas with the name "pasta salad".
    /// let mut pasta_salad_result: Option<Pizza> = client
    ///     .query_one(&collection("pizzas"), filter("name", EqualTo("pasta salad")))
    ///     .await?;
    ///
    /// // We expect no results
    /// assert_eq!(pasta_salad_result, None);
    /// # Ok(())
    /// # }
    pub async fn query_one<'de, 'a, T: Deserialize<'de>>(
        &mut self,
        collection: &CollectionReference,
        filter: Filter<'a>,
    ) -> Result<Option<T>, FirebaseError> {
        let (parent, collection_name) = self.split_collection_parent_and_name(collection);

        let mut stream = self
            .query_internal(ApiQueryOptions {
                parent,
                collection_name,
                filter: Some(filter),
                limit: Some(1),
                should_search_descendants: false,
                offset: None,
            })
            .await?;

        stream.try_next().await
    }

    async fn query_internal<'de, 'a, T: Deserialize<'de> + 'a>(
        &'a mut self,
        options: ApiQueryOptions<'a>,
    ) -> Result<FirebaseStream<'a, T, FirebaseError>, FirebaseError> {
        let doc_stream = self
            .query_internal_with_metadata(options)
            .await?
            .map(|doc_res| doc_res.map(|doc| doc.data));

        Ok(doc_stream.boxed())
    }

    async fn query_internal_with_metadata<'de, 'a, T: Deserialize<'de>>(
        &mut self,
        options: ApiQueryOptions<'a>,
    ) -> Result<FirebaseStream<FirestoreDocument<T>, FirebaseError>, FirebaseError> {
        let parent = options.parent.clone();
        let structured_query = self.structured_query_from_options(options)?;

        let request = RunQueryRequest {
            parent,
            query_type: Some(QueryType::StructuredQuery(structured_query)),
            consistency_selector: None,
        };

        let res = self
            .client
            .run_query(request)
            .await
            .context("Failed to run query")?;

        let doc_stream = res
            .into_inner()
            // Some of the "results" coming from the gRPC stream don't represent
            // search hits but rather information about query progress. We just
            // ignore those items.
            .filter_map(|res| future::ready(res.map(|inner| inner.document).transpose()))
            .map(|doc_res| {
                let doc = doc_res.map_err(|e| anyhow!(e))?;
                Ok(FirestoreDocument {
                    id: doc.name,
                    data: deserialize_firestore_document_fields::<T>(doc.fields)?,
                    create_time: doc.create_time.map(|t| t.seconds),
                    update_time: doc.update_time.map(|t| t.seconds),
                })
            });

        Ok(doc_stream.boxed())
    }

    /// Fetch all documents from any collection with the given name.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await?;
    /// use fireplace::firestore::collection;
    /// use futures::TryStreamExt;
    /// use serde::Deserialize;
    ///
    /// // Populate the database with some documents across different collections which
    /// // we can fetch
    /// client
    ///     .set_document(
    ///         &collection("cities")
    ///             .doc("SF")
    ///             .collection("landmarks")
    ///             .doc("golden-gate"),
    ///         &serde_json::json!({ "name": "Golden Gate Bridge", "type": "bridge" }),
    ///     )
    ///     .await?;
    /// client
    ///     .set_document(
    ///         &collection("cities")
    ///             .doc("SF")
    ///             .collection("landmarks")
    ///             .doc("legion-honor"),
    ///         &serde_json::json!({ "name": "Legion of Honor", "type": "museum" }),
    ///     )
    ///     .await?;
    /// client
    ///     .set_document(
    ///         &collection("cities")
    ///             .doc("TOK")
    ///             .collection("landmarks")
    ///             .doc("national-science-museum"),
    ///         &serde_json::json!({ "name": "National Museum of Nature and Science", "type": "museum" }),
    ///     )
    ///     .await?;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Landmark {
    ///     pub name: String,
    ///     pub r#type: String,
    /// }
    ///
    /// let mut landmarks: Vec<Landmark> = client
    ///     .collection_group("landmarks")
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// // We don't know which order the documents will be returned in, so we sort them
    /// landmarks.sort_by(|a, b| a.name.cmp(&b.name));
    ///
    /// assert_eq!(
    ///     landmarks,
    ///     vec![
    ///         Landmark {
    ///             name: "Golden Gate Bridge".to_string(),
    ///             r#type: "bridge".to_string()
    ///         },
    ///         Landmark {
    ///             name: "Legion of Honor".to_string(),
    ///             r#type: "museum".to_string()
    ///         },
    ///         Landmark {
    ///             name: "National Museum of Nature and Science".to_string(),
    ///             r#type: "museum".to_string()
    ///         },
    ///     ]
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub async fn collection_group<'de, 'a, T: Deserialize<'de> + 'a>(
        &'a mut self,
        collection_name: impl Into<String>,
    ) -> Result<FirebaseStream<T, FirebaseError>, FirebaseError> {
        self.query_internal(ApiQueryOptions {
            parent: self.root_resource_path.clone(),
            collection_name: collection_name.into(),
            filter: None,
            limit: None,
            should_search_descendants: true,
            offset: None,
        })
        .await
    }

    /// Query documents from any collection with the given name. This requires
    /// you to create a collection group index in the Firebase console,
    /// otherwise you will get an error telling you what to do.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await?;
    /// use fireplace::firestore::{
    ///     collection,
    ///     query::{filter, EqualTo},
    /// };
    /// use futures::TryStreamExt;
    /// use serde::Deserialize;
    ///
    /// client
    ///     .set_document(
    ///         &collection("cities")
    ///             .doc("SF")
    ///             .collection("landmarks")
    ///             .doc("golden-gate"),
    ///         &serde_json::json!({ "name": "Golden Gate Bridge", "type": "bridge" }),
    ///     )
    ///     .await?;
    /// client
    ///     .set_document(
    ///         &collection("cities")
    ///             .doc("SF")
    ///             .collection("landmarks")
    ///             .doc("legion-honor"),
    ///         &serde_json::json!({ "name": "Legion of Honor", "type": "museum" }),
    ///     )
    ///     .await?;
    /// client
    ///     .set_document(
    ///         &collection("cities")
    ///             .doc("TOK")
    ///             .collection("landmarks")
    ///             .doc("national-science-museum"),
    ///         &serde_json::json!({ "name": "National Museum of Nature and Science", "type": "museum" }),
    ///     )
    ///     .await?;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Landmark {
    ///     pub name: String,
    ///     pub r#type: String,
    /// }
    ///
    /// let mut landmarks: Vec<Landmark> = client
    ///     .collection_group_query("landmarks", filter("type", EqualTo("museum")))
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// landmarks.sort_by(|a, b| a.name.cmp(&b.name));
    ///
    /// assert_eq!(
    ///     landmarks,
    ///     vec![
    ///         Landmark {
    ///             name: "Legion of Honor".to_string(),
    ///             r#type: "museum".to_string()
    ///         },
    ///         Landmark {
    ///             name: "National Museum of Nature and Science".to_string(),
    ///             r#type: "museum".to_string()
    ///         },
    ///     ]
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub async fn collection_group_query<'de, 'a, T: Deserialize<'de> + 'a>(
        &'a mut self,
        collection_name: impl Into<String>,
        filter: Filter<'a>,
    ) -> Result<FirebaseStream<T, FirebaseError>, FirebaseError> {
        self.query_internal(ApiQueryOptions {
            parent: self.root_resource_path.clone(),
            collection_name: collection_name.into(),
            filter: Some(filter),
            limit: None,
            should_search_descendants: true,
            offset: None,
        })
        .await
    }

    pub async fn collection_group_query_with_pagination<'de, 'a, T: Deserialize<'de> + 'a>(
        &'a mut self,
        collection_name: impl Into<String>,
        filter: Filter<'a>,
        limit: i32,
        offset: i32,
    ) -> Result<FirebaseStream<T, FirebaseError>, FirebaseError> {
        self.query_internal(ApiQueryOptions {
            parent: self.root_resource_path.clone(),
            collection_name: collection_name.into(),
            filter: Some(filter),
            limit: Some(limit),
            should_search_descendants: true,
            offset: Some(offset),
        })
        .await
    }

    /// Queries documents from any collection with the given name, similarly to
    /// `collection_group_query`, but returns documents with metadata instead. The
    /// metadata contains information about the document ID and when it was created
    /// or updated. This requires you to create a collection group index in the
    /// Firebase console, otherwise you will get an error telling you what to do.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await?;
    /// use fireplace::firestore::{
    ///     collection,
    ///     query::{filter, EqualTo},
    /// };
    /// use futures::TryStreamExt;
    /// use serde::Deserialize;
    /// use fireplace::firestore::client::FirestoreDocument;
    ///
    /// client
    ///     .set_document(
    ///         &collection("cities")
    ///             .doc("SF")
    ///             .collection("landmarks")
    ///             .doc("golden-gate"),
    ///         &serde_json::json!({ "name": "Golden Gate Bridge", "type": "bridge" }),
    ///     )
    ///     .await?;
    /// client
    ///     .set_document(
    ///         &collection("cities")
    ///             .doc("SF")
    ///             .collection("landmarks")
    ///             .doc("legion-honor"),
    ///         &serde_json::json!({ "name": "Legion of Honor", "type": "museum" }),
    ///     )
    ///     .await?;
    /// client
    ///     .set_document(
    ///         &collection("cities")
    ///             .doc("TOK")
    ///             .collection("landmarks")
    ///             .doc("national-science-museum"),
    ///         &serde_json::json!({ "name": "National Museum of Nature and Science", "type": "museum" }),
    ///     )
    ///     .await?;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Landmark {
    ///     pub name: String,
    ///     pub r#type: String,
    /// }
    ///
    /// let mut landmarks: Vec<FirestoreDocument<Landmark>> = client
    ///     .collection_group_query_with_metadata("landmarks", filter("type", EqualTo("museum")))
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// landmarks.sort_by(|a, b| a.data.name.cmp(&b.data.name));
    ///
    /// assert_eq!(landmarks[0].data.name, "Legion of Honor".to_string());
    /// assert!(landmarks[0].id.ends_with("cities/SF/landmarks/legion-honor"));
    /// assert_eq!(landmarks[0].create_time, landmarks[0].update_time);
    ///
    /// assert_eq!(landmarks[1].data.name, "National Museum of Nature and Science".to_string());
    /// assert!(landmarks[1].id.ends_with("cities/TOK/landmarks/national-science-museum"));
    /// assert_eq!(landmarks[1].create_time, landmarks[1].update_time);
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub async fn collection_group_query_with_metadata<'de, 'a, T: Deserialize<'de>>(
        &mut self,
        collection_name: impl Into<String>,
        filter: Filter<'a>,
    ) -> Result<FirebaseStream<FirestoreDocument<T>, FirebaseError>, FirebaseError> {
        self.query_internal_with_metadata(ApiQueryOptions {
            parent: self.root_resource_path.clone(),
            collection_name: collection_name.into(),
            filter: Some(filter),
            limit: None,
            should_search_descendants: true,
            offset: None,
        })
        .await
    }

    /// Fetches all documents in the given collection. This skips documents that
    /// have no fields, which Firebase calls "missing documents".
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await?;
    /// use fireplace::firestore::collection;
    /// use futures::TryStreamExt;
    /// use serde::Deserialize;
    ///
    /// let emojis = vec![("computer", "💻"), ("coffee", "☕")];
    ///
    /// for (id, symbol) in emojis {
    ///     client
    ///         .set_document(
    ///             &collection("emojis").doc(id),
    ///             &serde_json::json!({ "symbol": symbol }),
    ///         )
    ///         .await?;
    /// }
    ///
    /// #[derive(Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// struct Emoji {
    ///     symbol: String,
    /// }
    ///
    /// let mut docs: Vec<Emoji> = client
    ///     .get_documents(&collection("emojis"))
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// docs.sort();
    ///
    /// assert_eq!(
    ///     docs,
    ///     vec![
    ///         Emoji {
    ///             symbol: "☕".into()
    ///         },
    ///         Emoji {
    ///             symbol: "💻".into()
    ///         },
    ///     ]
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_documents<'a, T: DeserializeOwned + Send + 'a>(
        &'a mut self,
        collection_ref: &CollectionReference,
    ) -> Result<FirebaseStream<T, FirebaseError>, FirebaseError> {
        let (parent, collection_name) = self.split_collection_parent_and_name(collection_ref);

        self.query_internal(ApiQueryOptions {
            parent,
            collection_name,
            filter: None,
            limit: None,
            should_search_descendants: false,
            offset: None,
        })
        .await
    }

    /// Counts the number of documents that would be returned by the given query.
    ///
    /// The counting itself is done server-side by Firestore, so using this
    /// function will be more efficient than executing the query and counting
    /// how many documents were returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await?;
    /// use fireplace::firestore::{
    ///     collection, collection_group,
    ///     query::{filter, EqualTo},
    /// };
    ///
    /// let landmarks = vec![
    ///     (
    ///         ("SF", "golden-gate"),
    ///         serde_json::json!({ "name": "Golden Gate Bridge", "type": "bridge" }),
    ///     ),
    ///     (
    ///         ("SF", "legion-honor"),
    ///         serde_json::json!({ "name": "Legion of Honor", "type": "museum" }),
    ///     ),
    ///     (
    ///         ("TOK", "national-science-museum"),
    ///         serde_json::json!({ "name": "National Museum of Nature and Science", "type": "museum" }),
    ///     ),
    /// ];
    ///
    /// for ((city, landmark_id), landmark_data) in landmarks {
    ///     client
    ///         .set_document(
    ///             &collection("cities")
    ///                 .doc(city)
    ///                 .collection("landmarks")
    ///                 .doc(landmark_id),
    ///             &landmark_data,
    ///         )
    ///         .await?;
    /// }
    ///
    /// let number_of_museums = client
    ///     .count(collection_group("landmarks").with_filter(filter("type", EqualTo("museum"))))
    ///     .await?;
    ///
    /// assert_eq!(number_of_museums, 2);
    ///
    /// let number_of_landmarks_in_san_francisco = client
    ///     .count(collection("cities").doc("SF").collection("landmarks"))
    ///     .await?;
    ///
    /// assert_eq!(number_of_landmarks_in_san_francisco, 2);
    ///
    /// let number_of_museums_in_san_francisco = client
    ///     .count(
    ///         collection("cities")
    ///             .doc("SF")
    ///             .collection("landmarks")
    ///             .with_filter(filter("type", EqualTo("museum"))),
    ///     )
    ///     .await?;
    ///
    /// assert_eq!(number_of_museums_in_san_francisco, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count<'a>(
        &'a mut self,
        query: impl FirestoreQuery<'a>,
    ) -> Result<u64, FirebaseError> {
        let options = ApiQueryOptions::from_query(self, query);

        self.count_internal(options).await
    }

    async fn count_internal<'a>(
        &'a mut self,
        options: ApiQueryOptions<'a>,
    ) -> Result<u64, FirebaseError> {
        let parent = options.parent.clone();
        let structured_query = self.structured_query_from_options(options)?;

        let aggregation_request = RunAggregationQueryRequest {
            parent,
            query_type: Some(
                run_aggregation_query_request::QueryType::StructuredAggregationQuery(
                    StructuredAggregationQuery {
                        query_type: Some(structured_aggregation_query::QueryType::StructuredQuery(
                            structured_query,
                        )),
                        aggregations: vec![structured_aggregation_query::Aggregation {
                            alias: "doc_count".to_string(),
                            operator: Some(aggregation::Operator::Count(aggregation::Count {
                                up_to: None,
                            })),
                        }],
                    },
                ),
            ),
            consistency_selector: None,
        };

        let res = self
            .client
            .run_aggregation_query(aggregation_request)
            .await
            .context("Failed to run count aggregation query")?;

        let count = res
            .into_inner()
            .filter_map(|res| future::ready(res.map(|inner| inner.result).transpose()))
            .map(|agg_res| -> Result<u64, FirebaseError> {
                let agg = agg_res.map_err(|e| anyhow!(e))?;
                let doc_count_value = agg
                    .aggregate_fields
                    .get("doc_count")
                    .context("Failed to get count from response")?;

                let doc_count = match doc_count_value.value_type {
                    Some(ValueType::IntegerValue(doc_count)) if doc_count >= 0 => doc_count as u64,
                    ref v => {
                        return Err(FirebaseError::Other(anyhow::anyhow!(
                            "Unexpected value type for count: {v:?}"
                        )))
                    }
                };

                Ok(doc_count)
            })
            .next()
            .await
            .context("No count returned from aggregation query")??;

        Ok(count)
    }

    fn structured_query_from_options(
        &self,
        options: ApiQueryOptions<'_>,
    ) -> Result<StructuredQuery, FirebaseError> {
        let grpc_filter = options
            .filter
            .map(|f| try_into_grpc_filter(f, &self.root_resource_path))
            .transpose()?;

        let structured_query = StructuredQuery {
            select: None,
            from: vec![CollectionSelector {
                collection_id: options.collection_name,
                all_descendants: options.should_search_descendants,
            }],
            r#where: grpc_filter,
            order_by: vec![],
            start_at: None,
            end_at: None,
            offset: options.offset.unwrap_or(0),
            limit: options.limit,
        };

        Ok(structured_query)
    }

    pub(crate) fn get_name_with(&self, item: impl Display) -> String {
        format!("{}/{}", self.root_resource_path, item)
    }

    fn split_collection_parent_and_name(
        &self,
        collection: &CollectionReference,
    ) -> (String, String) {
        let parent = collection
            .parent()
            .map(|p| self.get_name_with(p))
            .unwrap_or_else(|| self.root_resource_path.clone());
        let name = collection.name().to_string();

        (parent, name)
    }

    pub(crate) fn root_resource_path(&self) -> &str {
        &self.root_resource_path
    }

    fn serializer(&self) -> DocumentSerializer {
        DocumentSerializer::new(self.root_resource_path.clone())
    }
}

fn document_exists_precondition() -> Option<Precondition> {
    Some(Precondition {
        condition_type: Some(ConditionType::Exists(true)),
    })
}

fn not_found_err() -> fn(Status) -> FirebaseError {
    |err| {
        if err.code() == tonic::Code::NotFound {
            FirebaseError::DocumentNotfound(err.message().to_string())
        } else {
            anyhow!(err).into()
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn implements_send() {
        fn assert_send<T: Send>() {}
        assert_send::<super::FirestoreClient>();
    }
}
