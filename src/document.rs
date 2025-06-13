//! Document aggregate and related components
//!
//! A Document is an aggregate that represents business documents stored in a
//! content-addressed object store using CIDs (Content Identifiers).

use crate::{
    AggregateRoot, Entity, EntityId, DomainError, DomainResult, Component, ComponentStorage,
};
use cid::Cid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use std::any::Any;

/// Document aggregate - represents a business document with CID-based storage
#[derive(Debug, Clone)]
pub struct Document {
    /// Core entity data
    entity: Entity<DocumentMarker>,

    /// Version for optimistic concurrency control
    version: u64,

    /// Components attached to this document
    components: ComponentStorage,

    /// Component metadata (when added, by whom, etc.)
    component_metadata: HashMap<String, ComponentMetadata>,
}

/// Marker type for Document entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentMarker;

/// Metadata about when and why a component was added
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetadata {
    /// When this component was added
    pub added_at: std::time::SystemTime,

    /// Who added this component
    pub added_by: String,

    /// Reason or context for adding
    pub reason: Option<String>,
}

// Core document components

/// Basic document information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentInfoComponent {
    /// Document title
    pub title: String,

    /// Document description
    pub description: Option<String>,

    /// MIME type of the document
    pub mime_type: String,

    /// Original filename (if applicable)
    pub filename: Option<String>,

    /// File size in bytes
    pub size_bytes: u64,

    /// Document language (ISO 639-1 code)
    pub language: Option<String>,
}

/// Content addressing information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentAddressComponent {
    /// CID of the document content
    pub content_cid: Cid,

    /// CID of the metadata DAG
    pub metadata_cid: Option<Cid>,

    /// Hash algorithm used
    pub hash_algorithm: String,

    /// Encoding used for storage
    pub encoding: String,

    /// Whether content is chunked into multiple blocks
    pub is_chunked: bool,

    /// CIDs of chunks if chunked
    pub chunk_cids: Vec<Cid>,
}

/// Document classification and categorization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassificationComponent {
    /// Document type (contract, report, invoice, etc.)
    pub document_type: String,

    /// Business category
    pub category: String,

    /// Subcategories
    pub subcategories: Vec<String>,

    /// Tags for searchability
    pub tags: Vec<String>,

    /// Confidentiality level
    pub confidentiality: ConfidentialityLevel,
}

/// Confidentiality levels for documents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidentialityLevel {
    /// Public documents
    Public,
    /// Internal use only
    Internal,
    /// Confidential
    Confidential,
    /// Highly confidential
    HighlyConfidential,
    /// Restricted access
    Restricted,
}

/// Document ownership and authorship
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnershipComponent {
    /// Owner ID (person or organization)
    pub owner_id: Uuid,

    /// Author IDs
    pub authors: Vec<Uuid>,

    /// Department or team
    pub department: Option<String>,

    /// Project or case reference
    pub project_id: Option<Uuid>,

    /// Copyright information
    pub copyright: Option<String>,
}

/// Document lifecycle and versioning
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleComponent {
    /// Document status
    pub status: DocumentStatus,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last modification timestamp
    pub modified_at: chrono::DateTime<chrono::Utc>,

    /// Version number
    pub version_number: String,

    /// Previous version CID (for version history)
    pub previous_version_cid: Option<Cid>,

    /// Expiration date (if applicable)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Retention policy
    pub retention_policy: Option<String>,
}

/// Document status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentStatus {
    /// Draft document
    Draft,
    /// Under review
    UnderReview,
    /// Approved and published
    Published,
    /// Archived
    Archived,
    /// Marked for deletion
    MarkedForDeletion,
    /// Superseded by newer version
    Superseded,
}

/// Access control for documents
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessControlComponent {
    /// Read permissions (user/group IDs)
    pub read_access: Vec<Uuid>,

    /// Write permissions (user/group IDs)
    pub write_access: Vec<Uuid>,

    /// Share permissions (user/group IDs)
    pub share_access: Vec<Uuid>,

    /// Access log enabled
    pub audit_access: bool,

    /// Encryption key ID (if encrypted)
    pub encryption_key_id: Option<String>,
}

/// Document relationships
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipsComponent {
    /// Parent document (if this is an attachment/appendix)
    pub parent_document_id: Option<Uuid>,

    /// Related documents
    pub related_documents: Vec<DocumentRelation>,

    /// References to external documents
    pub external_references: Vec<ExternalReference>,
}

/// Relation to another document
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRelation {
    /// Related document ID
    pub document_id: Uuid,

    /// Type of relationship
    pub relation_type: RelationType,

    /// Description of the relationship
    pub description: Option<String>,
}

/// Types of document relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    /// This document supersedes the related one
    Supersedes,
    /// This document is superseded by the related one
    SupersededBy,
    /// This document references the related one
    References,
    /// This document is referenced by the related one
    ReferencedBy,
    /// This document is an attachment to the related one
    AttachmentOf,
    /// The related document is an attachment to this one
    HasAttachment,
    /// This document is a translation of the related one
    TranslationOf,
    /// The related document is a translation of this one
    HasTranslation,
}

/// External reference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalReference {
    /// Type of reference (URL, DOI, ISBN, etc.)
    pub reference_type: String,

    /// Reference value
    pub reference_value: String,

    /// Description
    pub description: Option<String>,
}

/// Document processing metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessingComponent {
    /// Text extraction status
    pub text_extracted: bool,

    /// Extracted text CID (if applicable)
    pub extracted_text_cid: Option<Cid>,

    /// OCR performed
    pub ocr_performed: bool,

    /// Thumbnails generated
    pub thumbnails_generated: bool,

    /// Thumbnail CIDs
    pub thumbnail_cids: Vec<ThumbnailInfo>,

    /// Indexing status
    pub indexed: bool,

    /// Processing errors
    pub processing_errors: Vec<String>,
}

/// Thumbnail information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThumbnailInfo {
    /// Size identifier (small, medium, large)
    pub size: String,

    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// Thumbnail CID
    pub cid: Cid,
}

impl Document {
    /// Create a new document with basic info and content CID
    pub fn new(
        id: EntityId<DocumentMarker>,
        info: DocumentInfoComponent,
        content_cid: Cid,
    ) -> Self {
        let mut components = ComponentStorage::new();
        components.add(info).unwrap();

        // Create content address component
        let content_address = ContentAddressComponent {
            content_cid,
            metadata_cid: None,
            hash_algorithm: "sha2-256".to_string(),
            encoding: "raw".to_string(),
            is_chunked: false,
            chunk_cids: vec![],
        };
        components.add(content_address).unwrap();

        let mut component_metadata = HashMap::new();
        component_metadata.insert(
            "DocumentInfo".to_string(),
            ComponentMetadata {
                added_at: std::time::SystemTime::now(),
                added_by: "system".to_string(),
                reason: Some("Initial document info".to_string()),
            },
        );
        component_metadata.insert(
            "ContentAddress".to_string(),
            ComponentMetadata {
                added_at: std::time::SystemTime::now(),
                added_by: "system".to_string(),
                reason: Some("Initial content address".to_string()),
            },
        );

        Self {
            entity: Entity::with_id(id),
            version: 0,
            components,
            component_metadata,
        }
    }

    /// Create a chunked document with multiple content blocks
    pub fn new_chunked(
        id: EntityId<DocumentMarker>,
        info: DocumentInfoComponent,
        chunk_cids: Vec<Cid>,
        metadata_cid: Cid,
    ) -> Self {
        let mut components = ComponentStorage::new();
        components.add(info).unwrap();

        // Create content address component for chunked content
        let content_address = ContentAddressComponent {
            content_cid: metadata_cid, // For chunked docs, content_cid points to the metadata DAG
            metadata_cid: Some(metadata_cid),
            hash_algorithm: "sha2-256".to_string(),
            encoding: "dag-pb".to_string(),
            is_chunked: true,
            chunk_cids,
        };
        components.add(content_address).unwrap();

        let mut component_metadata = HashMap::new();
        component_metadata.insert(
            "DocumentInfo".to_string(),
            ComponentMetadata {
                added_at: std::time::SystemTime::now(),
                added_by: "system".to_string(),
                reason: Some("Initial document info".to_string()),
            },
        );
        component_metadata.insert(
            "ContentAddress".to_string(),
            ComponentMetadata {
                added_at: std::time::SystemTime::now(),
                added_by: "system".to_string(),
                reason: Some("Chunked content address".to_string()),
            },
        );

        Self {
            entity: Entity::with_id(id),
            version: 0,
            components,
            component_metadata,
        }
    }

    /// Add a component to this document
    pub fn add_component<C: Component + 'static>(
        &mut self,
        component: C,
        added_by: &str,
        reason: Option<String>,
    ) -> DomainResult<()> {
        let component_type = component.type_name().to_string();

        // Add the component
        self.components.add(component)?;

        // Add metadata
        self.component_metadata.insert(
            component_type,
            ComponentMetadata {
                added_at: std::time::SystemTime::now(),
                added_by: added_by.to_string(),
                reason,
            },
        );

        self.entity.touch();
        self.version += 1;

        Ok(())
    }

    /// Remove a component
    pub fn remove_component<C: Component + 'static>(&mut self) -> DomainResult<()> {
        let component_type = std::any::type_name::<C>();

        if self.components.remove::<C>().is_some() {
            self.component_metadata.remove(component_type);
            self.entity.touch();
            self.version += 1;
            Ok(())
        } else {
            Err(DomainError::ComponentNotFound(format!(
                "Component {} not found",
                component_type
            )))
        }
    }

    /// Get a component
    pub fn get_component<C: Component + 'static>(&self) -> Option<&C> {
        self.components.get::<C>()
    }

    /// Check if document has a component
    pub fn has_component<C: Component + 'static>(&self) -> bool {
        self.components.has::<C>()
    }

    /// Get all component types
    pub fn component_types(&self) -> Vec<String> {
        self.component_metadata.keys().cloned().collect()
    }

    /// Get the content CID
    pub fn content_cid(&self) -> Option<Cid> {
        self.get_component::<ContentAddressComponent>()
            .map(|c| c.content_cid)
    }

    /// Check if document is chunked
    pub fn is_chunked(&self) -> bool {
        self.get_component::<ContentAddressComponent>()
            .map(|c| c.is_chunked)
            .unwrap_or(false)
    }

    /// Get chunk CIDs if document is chunked
    pub fn chunk_cids(&self) -> Vec<Cid> {
        self.get_component::<ContentAddressComponent>()
            .map(|c| c.chunk_cids.clone())
            .unwrap_or_default()
    }
}

impl AggregateRoot for Document {
    type Id = EntityId<DocumentMarker>;

    fn id(&self) -> Self::Id {
        self.entity.id
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn increment_version(&mut self) {
        self.version += 1;
        self.entity.touch();
    }
}

// Component trait implementations

impl Component for DocumentInfoComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "DocumentInfo"
    }
}

impl Component for ContentAddressComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ContentAddress"
    }
}

impl Component for ClassificationComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Classification"
    }
}

impl Component for OwnershipComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Ownership"
    }
}

impl Component for LifecycleComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Lifecycle"
    }
}

impl Component for AccessControlComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "AccessControl"
    }
}

impl Component for RelationshipsComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Relationships"
    }
}

impl Component for ProcessingComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Processing"
    }
}

// View projections

/// Public document view (for external sharing)
pub struct PublicDocumentView {
    /// The document's unique identifier
    pub document_id: EntityId<DocumentMarker>,
    /// Basic document information
    pub info: DocumentInfoComponent,
    /// Classification information if available
    pub classification: Option<ClassificationComponent>,
    /// Lifecycle information if available
    pub lifecycle: Option<LifecycleComponent>,
    /// Content identifier for retrieval
    pub content_cid: Cid,
}

impl PublicDocumentView {
    /// Create public view from document
    pub fn from_document(document: &Document) -> DomainResult<Self> {
        let info = document.get_component::<DocumentInfoComponent>()
            .ok_or_else(|| DomainError::ValidationError(
                "Document missing info component".to_string()
            ))?
            .clone();

        let content_cid = document.content_cid()
            .ok_or_else(|| DomainError::ValidationError(
                "Document missing content address".to_string()
            ))?;

        Ok(Self {
            document_id: document.id(),
            info,
            classification: document.get_component::<ClassificationComponent>().cloned(),
            lifecycle: document.get_component::<LifecycleComponent>().cloned(),
            content_cid,
        })
    }
}

/// Search index projection
pub struct SearchIndexProjection {
    /// Document unique identifier
    pub document_id: Uuid,
    /// Document title for search results
    pub title: String,
    /// Document description for search context
    pub description: Option<String>,
    /// MIME type for filtering
    pub mime_type: String,
    /// Tags for search and categorization
    pub tags: Vec<String>,
    /// Author IDs for author-based search
    pub authors: Vec<Uuid>,
    /// Creation timestamp for date filtering
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last modification timestamp for recency
    pub modified_at: chrono::DateTime<chrono::Utc>,
    /// Content CID as string for retrieval
    pub content_cid: String,
    /// File size for filtering and display
    pub size_bytes: u64,
}

impl SearchIndexProjection {
    /// Create search projection from document
    pub fn from_document(document: &Document) -> DomainResult<Self> {
        let info = document.get_component::<DocumentInfoComponent>()
            .ok_or_else(|| DomainError::ValidationError(
                "Document missing info component".to_string()
            ))?;

        let content_cid = document.content_cid()
            .ok_or_else(|| DomainError::ValidationError(
                "Document missing content address".to_string()
            ))?;

        let classification = document.get_component::<ClassificationComponent>();
        let ownership = document.get_component::<OwnershipComponent>();
        let lifecycle = document.get_component::<LifecycleComponent>();

        Ok(Self {
            document_id: *document.id().as_uuid(),
            title: info.title.clone(),
            description: info.description.clone(),
            mime_type: info.mime_type.clone(),
            tags: classification.map(|c| c.tags.clone()).unwrap_or_default(),
            authors: ownership.map(|o| o.authors.clone()).unwrap_or_default(),
            created_at: lifecycle.map(|l| l.created_at).unwrap_or_else(chrono::Utc::now),
            modified_at: lifecycle.map(|l| l.modified_at).unwrap_or_else(chrono::Utc::now),
            content_cid: content_cid.to_string(),
            size_bytes: info.size_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let id = EntityId::new();
        let info = DocumentInfoComponent {
            title: "Test Document".to_string(),
            description: Some("A test document".to_string()),
            mime_type: "application/pdf".to_string(),
            filename: Some("test.pdf".to_string()),
            size_bytes: 1024,
            language: Some("en".to_string()),
        };

        // Create a test CID
        let content_cid = Cid::default();

        let document = Document::new(id, info.clone(), content_cid);

        assert_eq!(document.id(), id);
        assert_eq!(document.version(), 0);

        let stored_info = document.get_component::<DocumentInfoComponent>().unwrap();
        assert_eq!(stored_info.title, "Test Document");
        assert_eq!(stored_info.mime_type, "application/pdf");

        let content_addr = document.get_component::<ContentAddressComponent>().unwrap();
        assert_eq!(content_addr.content_cid, content_cid);
        assert!(!content_addr.is_chunked);
    }

    #[test]
    fn test_chunked_document() {
        let id = EntityId::new();
        let info = DocumentInfoComponent {
            title: "Large Document".to_string(),
            description: Some("A large chunked document".to_string()),
            mime_type: "video/mp4".to_string(),
            filename: Some("video.mp4".to_string()),
            size_bytes: 1_000_000_000, // 1GB
            language: None,
        };

        // Create test CIDs for chunks
        let chunk_cids = vec![Cid::default(), Cid::default(), Cid::default()];
        let metadata_cid = Cid::default();

        let document = Document::new_chunked(id, info, chunk_cids.clone(), metadata_cid);

        assert!(document.is_chunked());
        assert_eq!(document.chunk_cids(), chunk_cids);

        let content_addr = document.get_component::<ContentAddressComponent>().unwrap();
        assert_eq!(content_addr.metadata_cid, Some(metadata_cid));
        assert_eq!(content_addr.encoding, "dag-pb");
    }

    #[test]
    fn test_document_components() {
        let id = EntityId::new();
        let info = DocumentInfoComponent {
            title: "Component Test".to_string(),
            description: None,
            mime_type: "text/plain".to_string(),
            filename: Some("test.txt".to_string()),
            size_bytes: 100,
            language: Some("en".to_string()),
        };

        let mut document = Document::new(id, info, Cid::default());

        // Add classification
        let classification = ClassificationComponent {
            document_type: "report".to_string(),
            category: "technical".to_string(),
            subcategories: vec!["architecture".to_string()],
            tags: vec!["rust".to_string(), "ddd".to_string()],
            confidentiality: ConfidentialityLevel::Internal,
        };

        document.add_component(classification.clone(), "test_user", Some("Adding classification".to_string())).unwrap();

        assert!(document.has_component::<ClassificationComponent>());
        let stored_class = document.get_component::<ClassificationComponent>().unwrap();
        assert_eq!(stored_class.document_type, "report");
        assert_eq!(stored_class.confidentiality, ConfidentialityLevel::Internal);

        // Add ownership
        let ownership = OwnershipComponent {
            owner_id: Uuid::new_v4(),
            authors: vec![Uuid::new_v4()],
            department: Some("Engineering".to_string()),
            project_id: None,
            copyright: Some("© 2024 Test Corp".to_string()),
        };

        document.add_component(ownership, "test_user", None).unwrap();
        assert!(document.has_component::<OwnershipComponent>());

        // Test component removal
        document.remove_component::<ClassificationComponent>().unwrap();
        assert!(!document.has_component::<ClassificationComponent>());
    }

    #[test]
    fn test_public_view() {
        let id = EntityId::new();
        let info = DocumentInfoComponent {
            title: "Public Document".to_string(),
            description: Some("A document for public viewing".to_string()),
            mime_type: "application/pdf".to_string(),
            filename: Some("public.pdf".to_string()),
            size_bytes: 2048,
            language: Some("en".to_string()),
        };

        let content_cid = Cid::default();
        let mut document = Document::new(id, info, content_cid);

        // Add classification
        let classification = ClassificationComponent {
            document_type: "whitepaper".to_string(),
            category: "public".to_string(),
            subcategories: vec![],
            tags: vec!["blockchain".to_string()],
            confidentiality: ConfidentialityLevel::Public,
        };

        document.add_component(classification, "system", None).unwrap();

        // Create public view
        let public_view = PublicDocumentView::from_document(&document).unwrap();

        assert_eq!(public_view.document_id, id);
        assert_eq!(public_view.info.title, "Public Document");
        assert_eq!(public_view.content_cid, content_cid);
        assert!(public_view.classification.is_some());
    }

    #[test]
    fn test_search_projection() {
        let id = EntityId::new();
        let info = DocumentInfoComponent {
            title: "Searchable Document".to_string(),
            description: Some("Document for search indexing".to_string()),
            mime_type: "text/markdown".to_string(),
            filename: Some("search.md".to_string()),
            size_bytes: 512,
            language: Some("en".to_string()),
        };

        let content_cid = Cid::default();
        let mut document = Document::new(id, info, content_cid);

        // Add components for search
        let classification = ClassificationComponent {
            document_type: "documentation".to_string(),
            category: "technical".to_string(),
            subcategories: vec!["api".to_string()],
            tags: vec!["rest".to_string(), "api".to_string(), "documentation".to_string()],
            confidentiality: ConfidentialityLevel::Internal,
        };

        let author_id = Uuid::new_v4();
        let ownership = OwnershipComponent {
            owner_id: Uuid::new_v4(),
            authors: vec![author_id],
            department: Some("Engineering".to_string()),
            project_id: Some(Uuid::new_v4()),
            copyright: None,
        };

        document.add_component(classification, "system", None).unwrap();
        document.add_component(ownership, "system", None).unwrap();

        // Create search projection
        let search_proj = SearchIndexProjection::from_document(&document).unwrap();

        assert_eq!(search_proj.title, "Searchable Document");
        assert_eq!(search_proj.mime_type, "text/markdown");
        assert_eq!(search_proj.tags.len(), 3);
        assert_eq!(search_proj.authors.len(), 1);
        assert_eq!(search_proj.authors[0], author_id);
        assert_eq!(search_proj.size_bytes, 512);
    }
}
