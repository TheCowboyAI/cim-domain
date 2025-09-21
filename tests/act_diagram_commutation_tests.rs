// Copyright (c) 2025 - Cowboy AI, LLC.

mod tests {
    pub mod act {
        use cim_domain::domain_path::{DomainArtifactKind, DomainPath};
        use cim_domain::subject::{Subject, SubjectError, SubjectSegment};

        fn subject(tokens: &[&str]) -> Result<Subject, SubjectError> {
            let mut value = Subject::root();
            for token in tokens {
                value = value.append(SubjectSegment::new((*token).to_string())?);
            }
            Ok(value)
        }

        #[test]
        fn subject_algebra_diagram_commutes() {
            let s1 = subject(&["sales", "orders"]).expect("segments");
            let s2 = subject(&["created"]).expect("segments");
            let s3 = subject(&["v1"]).expect("segments");

            let left = s1.concat(&s2).concat(&s3);
            let right = s1.concat(&s2.concat(&s3));
            assert_eq!(left, right, "diagram: subject_algebra_v2 commutation");

            let id = Subject::root();
            assert_eq!(id.concat(&s1), s1, "left identity");
            assert_eq!(s1.concat(&id), s1, "right identity");
        }

        #[test]
        fn domain_path_algebra_diagram_commutes() {
            let root = DomainPath::root();
            let bounded = DomainPath::parse("cim.domain.billing").expect("bounded context");
            let facet = DomainPath::parse("cim.domain.billing.command").expect("facet path");
            let named =
                DomainPath::parse("cim.domain.billing.command.authorize").expect("named path");

            let left = bounded.concat(&facet).concat(&named);
            let right = bounded.concat(&facet.concat(&named));
            assert_eq!(left, right, "diagram: domain_path_algebra_v2 associativity");

            assert_eq!(root.concat(&named), named, "root is left identity");
            assert_eq!(named.concat(&root), named, "root is right identity");

            assert_eq!(
                named.artifact_kind(),
                Some(DomainArtifactKind::Command),
                "facet node uses command morphism"
            );
        }
    }
}
