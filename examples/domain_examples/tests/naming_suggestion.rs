use std::collections::BTreeMap;

use cim_domain::{
    QualityDimension, QualitySchema, QualityVector, ScaleType,
    suggest_by_prototypes,
};

#[test]
fn entity_naming_selects_best_concept() {
    let schema = QualitySchema::new(vec![
        QualityDimension { id: "has_amount".into(), name: "Has Amount".into(), scale: ScaleType::Nominal },
        QualityDimension { id: "has_party".into(), name: "Has Party".into(), scale: ScaleType::Nominal },
        QualityDimension { id: "has_date".into(), name: "Has Date".into(), scale: ScaleType::Nominal },
    ]);

    // Concept prototypes under schema
    let mut protos: BTreeMap<String, QualityVector> = BTreeMap::new();
    protos.insert("invoice".into(), QualityVector { values: vec![1.0, 1.0, 1.0] });
    protos.insert("payment".into(), QualityVector { values: vec![1.0, 1.0, 0.0] });
    protos.insert("profile".into(), QualityVector { values: vec![0.0, 1.0, 0.0] });

    // Entity features extracted upstream (e.g., from an aggregate snapshot)
    let mut feat = BTreeMap::new();
    feat.insert("has_amount".into(), 1.0);
    feat.insert("has_party".into(), 1.0);
    feat.insert("has_date".into(), 0.9);

    // Name suggestion
    let top = suggest_by_prototypes(&schema, &feat, &protos, 1);
    assert_eq!(top[0].0, "invoice");
}
