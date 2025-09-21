use cim_domain::domain_path::DomainPath;
use cim_domain::projections::Projection;
use cim_domain::{
    CqrsQueryHandler, DomainEvent, InMemoryReadModel, Query, QueryAcknowledgment, QueryCriteria,
    QueryEnvelope, QueryResponse, QueryStatus, ReadModelStorage, Subject,
};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, serde::Serialize)]
struct TopPolicyQuery {
    limit: usize,
}

impl Query for TopPolicyQuery {}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
struct PolicyView {
    id: String,
    issued: bool,
}

#[derive(Debug)]
struct PolicyIssued(Uuid);

impl DomainEvent for PolicyIssued {
    fn aggregate_id(&self) -> Uuid {
        self.0
    }

    fn event_type(&self) -> &'static str {
        "PolicyIssued"
    }
}

#[tokio::test]
async fn projection_handles_and_query_responds() {
    // Arrange projection (reuse CounterProjection logic inline)
    #[derive(Default)]
    struct PolicyProjection {
        issued: bool,
    }

    #[async_trait::async_trait]
    impl cim_domain::projections::Projection for PolicyProjection {
        async fn handle_event(&mut self, event: &dyn DomainEvent) -> Result<(), String> {
            if event.event_type() == "PolicyIssued" {
                self.issued = true;
            }
            Ok(())
        }

        async fn clear(&mut self) -> Result<(), String> {
            self.issued = false;
            Ok(())
        }
    }

    let mut projection = PolicyProjection::default();
    let person_id = Uuid::new_v4();
    projection
        .handle_event(&PolicyIssued(person_id))
        .await
        .expect("handle event");
    assert!(projection.issued);
    projection.clear().await.expect("clear");
    assert!(!projection.issued);

    // Arrange read model + query handler
    let rm: InMemoryReadModel<PolicyView> = InMemoryReadModel::new();
    rm.insert(
        "policy-1".into(),
        PolicyView {
            id: "policy-1".into(),
            issued: true,
        },
    );
    rm.insert(
        "policy-2".into(),
        PolicyView {
            id: "policy-2".into(),
            issued: false,
        },
    );

    struct PolicyQueryHandler {
        rm: InMemoryReadModel<PolicyView>,
    }

    impl PolicyQueryHandler {
        fn new(rm: InMemoryReadModel<PolicyView>) -> Self {
            Self { rm }
        }
    }

    impl cim_domain::CqrsQueryHandler<TopPolicyQuery> for PolicyQueryHandler {
        fn handle(&self, envelope: QueryEnvelope<TopPolicyQuery>) -> QueryResponse {
            let mut views = self.rm.all();
            views.sort_by_key(|v| v.id.clone());
            views.truncate(envelope.query.limit);
            QueryResponse {
                query_id: *envelope.id.as_uuid(),
                correlation_id: envelope.identity.correlation_id,
                result: json!(views),
            }
        }
    }

    let handler = PolicyQueryHandler::new(rm.clone());
    let envelope = QueryEnvelope::new(TopPolicyQuery { limit: 1 }, "system".into());
    let ack = QueryAcknowledgment {
        query_id: envelope.id,
        correlation_id: envelope.identity.correlation_id,
        status: QueryStatus::Accepted,
        reason: None,
    };
    assert_eq!(ack.status, QueryStatus::Accepted);
    let response = handler.handle(envelope);
    assert_eq!(response.result[0]["id"], json!("policy-1"));

    // Validate DomainPath and Subject interplay
    let path = DomainPath::command("person", "register").unwrap();
    assert_eq!(
        path.artifact_kind(),
        Some(cim_domain::domain_path::DomainArtifactKind::Command)
    );
    let subject = Subject::parse("person.policy.issued").unwrap();
    assert_eq!(subject.to_string(), "person.policy.issued");

    // Ensure read model criteria function as expected
    let issued = rm
        .query(&QueryCriteria::new())
        .into_iter()
        .filter(|p| p.issued)
        .count();
    assert_eq!(issued, 1);
}
