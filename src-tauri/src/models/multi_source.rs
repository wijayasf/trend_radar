#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryEntityType {
    AgentTool,
    FrameworkSdk,
    SkillMode,
    Protocol,
    ConnectorPlugin,
    RegistryDiscovery,
    AppBuilder,
    Other,
}

impl PrimaryEntityType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AgentTool => "agent_tool",
            Self::FrameworkSdk => "framework_sdk",
            Self::SkillMode => "skill_mode",
            Self::Protocol => "protocol",
            Self::ConnectorPlugin => "connector_plugin",
            Self::RegistryDiscovery => "registry_discovery",
            Self::AppBuilder => "app_builder",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "agent_tool" => Ok(Self::AgentTool),
            "framework_sdk" => Ok(Self::FrameworkSdk),
            "skill_mode" => Ok(Self::SkillMode),
            "protocol" => Ok(Self::Protocol),
            "connector_plugin" => Ok(Self::ConnectorPlugin),
            "registry_discovery" => Ok(Self::RegistryDiscovery),
            "app_builder" => Ok(Self::AppBuilder),
            "other" => Ok(Self::Other),
            _ => Err(format!(
                "Unsupported canonical entity primary type: {value}"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityStatus {
    Active,
    Archived,
}

impl EntityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("Unsupported canonical entity status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalSource {
    Threads,
    ExplainX,
    GitHub,
    HackerNews,
    ProductHunt,
}

impl ExternalSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Threads => "threads",
            Self::ExplainX => "explainx",
            Self::GitHub => "github",
            Self::HackerNews => "hacker_news",
            Self::ProductHunt => "product_hunt",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "threads" => Ok(Self::Threads),
            "explainx" => Ok(Self::ExplainX),
            "github" => Ok(Self::GitHub),
            "hacker_news" => Ok(Self::HackerNews),
            "product_hunt" => Ok(Self::ProductHunt),
            _ => Err(format!("Unsupported external source: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionMode {
    Scheduled,
    Manual,
    Import,
    Replay,
}

impl CollectionMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Manual => "manual",
            Self::Import => "import",
            Self::Replay => "replay",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "scheduled" => Ok(Self::Scheduled),
            "manual" => Ok(Self::Manual),
            "import" => Ok(Self::Import),
            "replay" => Ok(Self::Replay),
            _ => Err(format!("Unsupported source collection mode: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionRunStatus {
    Running,
    Completed,
    Partial,
    Failed,
}

impl CollectionRunStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Partial => "partial",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "partial" => Ok(Self::Partial),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("Unsupported source collection run status: {value}")),
        }
    }

    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Running)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionState {
    SingleEntity,
    MultipleEntities,
    NoProductEntity,
    Unresolved,
}

impl ResolutionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleEntity => "single_entity",
            Self::MultipleEntities => "multiple_entities",
            Self::NoProductEntity => "no_product_entity",
            Self::Unresolved => "unresolved",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "single_entity" => Ok(Self::SingleEntity),
            "multiple_entities" => Ok(Self::MultipleEntities),
            "no_product_entity" => Ok(Self::NoProductEntity),
            "unresolved" => Ok(Self::Unresolved),
            _ => Err(format!(
                "Unsupported source record resolution state: {value}"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipType {
    SameEntity,
    ChildResource,
    RelatedEntity,
    MentionedEntity,
}

impl RelationshipType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SameEntity => "same_entity",
            Self::ChildResource => "child_resource",
            Self::RelatedEntity => "related_entity",
            Self::MentionedEntity => "mentioned_entity",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "same_entity" => Ok(Self::SameEntity),
            "child_resource" => Ok(Self::ChildResource),
            "related_entity" => Ok(Self::RelatedEntity),
            "mentioned_entity" => Ok(Self::MentionedEntity),
            _ => Err(format!(
                "Unsupported source/entity relationship type: {value}"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkReviewState {
    Pending,
    Approved,
    Rejected,
    Ambiguous,
}

impl LinkReviewState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Ambiguous => "ambiguous",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "ambiguous" => Ok(Self::Ambiguous),
            _ => Err(format!(
                "Unsupported source/entity link review state: {value}"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewCanonicalEntity {
    pub canonical_name: String,
    pub primary_type: PrimaryEntityType,
    pub description: Option<String>,
    pub primary_website: Option<String>,
    pub primary_repository: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CanonicalEntity {
    pub entity_id: String,
    pub canonical_name: String,
    pub normalized_name: String,
    pub primary_type: PrimaryEntityType,
    pub status: EntityStatus,
    pub description: Option<String>,
    pub primary_website: Option<String>,
    pub primary_repository: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CanonicalEntityMetadataUpdate {
    pub canonical_name: String,
    pub primary_type: PrimaryEntityType,
    pub status: EntityStatus,
    pub description: Option<String>,
    pub primary_website: Option<String>,
    pub primary_repository: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewCollectionRun {
    pub source: ExternalSource,
    pub collection_mode: CollectionMode,
    pub scope_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SourceCollectionRun {
    pub collection_run_id: String,
    pub source: ExternalSource,
    pub collection_mode: CollectionMode,
    pub scope_json: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: CollectionRunStatus,
    pub records_seen: i64,
    pub observations_saved: i64,
    pub error_summary: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SourceRecordUpsert {
    pub source: ExternalSource,
    pub source_record_key: String,
    pub record_type: String,
    pub resolution_state: ResolutionState,
    pub title: Option<String>,
    pub external_url: Option<String>,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub source_category: Option<String>,
    pub repository_url: Option<String>,
    pub published_at: Option<String>,
    pub listed_at: Option<String>,
    pub metadata_json: Option<String>,
    pub seen_at: String,
}

#[derive(Debug, Clone)]
pub struct SourceRecord {
    pub source_record_id: String,
    pub source: ExternalSource,
    pub source_record_key: String,
    pub record_type: String,
    pub resolution_state: ResolutionState,
    pub title: Option<String>,
    pub external_url: Option<String>,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub source_category: Option<String>,
    pub repository_url: Option<String>,
    pub published_at: Option<String>,
    pub listed_at: Option<String>,
    pub metadata_json: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct NewSourceObservation {
    pub observed_at: String,
    pub surface: String,
    pub observation_kind: String,
    pub time_window: String,
    pub rank: Option<i64>,
    pub source_score: Option<f64>,
    pub views: Option<i64>,
    pub installs_total: Option<i64>,
    pub installs_period: Option<i64>,
    pub github_stars: Option<i64>,
    pub upvotes: Option<i64>,
    pub payload_hash: Option<String>,
    pub source_payload_json: String,
}

#[derive(Debug, Clone)]
pub struct SourceObservation {
    pub observation_id: String,
    pub collection_run_id: String,
    pub source_record_id: String,
    pub observed_at: String,
    pub surface: String,
    pub observation_kind: String,
    pub time_window: String,
    pub rank: Option<i64>,
    pub source_score: Option<f64>,
    pub views: Option<i64>,
    pub installs_total: Option<i64>,
    pub installs_period: Option<i64>,
    pub github_stars: Option<i64>,
    pub upvotes: Option<i64>,
    pub payload_hash: Option<String>,
    pub source_payload_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AppendObservationResult {
    pub observation: SourceObservation,
    pub inserted: bool,
}

#[derive(Debug, Clone)]
pub struct NewSourceRecordEntityLink {
    pub source_record_id: String,
    pub entity_id: String,
    pub relationship_type: RelationshipType,
    pub match_method: String,
    pub match_confidence: Option<f64>,
    pub review_state: LinkReviewState,
    pub evidence_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SourceRecordEntityLink {
    pub link_id: String,
    pub source_record_id: String,
    pub entity_id: String,
    pub relationship_type: RelationshipType,
    pub match_method: String,
    pub match_confidence: Option<f64>,
    pub review_state: LinkReviewState,
    pub evidence_json: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn normalize_entity_name(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
