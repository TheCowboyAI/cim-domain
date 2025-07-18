use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cim_domain::{
    EntityId,
    DomainEntity,
    persistence::*,
};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Copy)]
struct BenchEntityMarker;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchEntity {
    id: EntityId<BenchEntityMarker>,
    name: String,
    data: Vec<u8>,
    version: u64,
}

impl DomainEntity for BenchEntity {
    type IdType = BenchEntityMarker;
    
    fn id(&self) -> EntityId<Self::IdType> {
        self.id
    }
}

impl BenchEntity {
    fn new(size: usize) -> Self {
        Self {
            id: EntityId::new(),
            name: format!("bench_entity_{}", size),
            data: vec![0u8; size],
            version: 1,
        }
    }
}

fn setup_runtime() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn benchmark_simple_repository_save(c: &mut Criterion) {
    let rt = setup_runtime();
    
    // Skip if NATS not available
    let client = match rt.block_on(async_nats::connect("nats://localhost:4222")) {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping benchmarks");
            return;
        }
    };
    
    let repo = rt.block_on(async {
        NatsSimpleRepository::new(
            client,
            "bench-simple".to_string(),
            "BenchEntity".to_string(),
        ).await.unwrap()
    });
    
    let mut group = c.benchmark_group("simple_repository_save");
    
    for size in [100, 1_000, 10_000, 100_000].iter() {
        let entity = BenchEntity::new(*size);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        repo.save(&entity).await.unwrap()
                    })
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_simple_repository_load(c: &mut Criterion) {
    let rt = setup_runtime();
    
    let client = match rt.block_on(async_nats::connect("nats://localhost:4222")) {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping benchmarks");
            return;
        }
    };
    
    let repo = rt.block_on(async {
        NatsSimpleRepository::new(
            client,
            "bench-simple-load".to_string(),
            "BenchEntity".to_string(),
        ).await.unwrap()
    });
    
    // Pre-save entities
    let entities: Vec<_> = [100, 1_000, 10_000, 100_000]
        .iter()
        .map(|&size| {
            let entity = BenchEntity::new(size);
            rt.block_on(async {
                repo.save(&entity).await.unwrap();
            });
            entity
        })
        .collect();
    
    let mut group = c.benchmark_group("simple_repository_load");
    
    for (i, size) in [100, 1_000, 10_000, 100_000].iter().enumerate() {
        let entity_id = entities[i].id();
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let _loaded: Option<BenchEntity> = repo.load(&entity_id).await.unwrap();
                    })
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_kv_repository_with_ttl(c: &mut Criterion) {
    let rt = setup_runtime();
    
    let client = match rt.block_on(async_nats::connect("nats://localhost:4222")) {
        Ok(client) => client,
        Err(_) => return,
    };
    
    let repo: NatsKvRepository<BenchEntity> = rt.block_on(async {
        NatsKvRepositoryBuilder::new()
            .client(client)
            .bucket_name("bench-kv")
            .aggregate_type("BenchEntity")
            .ttl_seconds(300) // 5 minutes
            .build()
            .await
            .unwrap()
    });
    
    c.bench_function("kv_repository_save_with_ttl", |b| {
        let entity = BenchEntity::new(1_000);
        b.iter(|| {
            rt.block_on(async {
                repo.save(&entity).await.unwrap()
            })
        });
    });
}

fn benchmark_read_model_store(c: &mut Criterion) {
    let rt = setup_runtime();
    
    let client = match rt.block_on(async_nats::connect("nats://localhost:4222")) {
        Ok(client) => client,
        Err(_) => return,
    };
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct BenchReadModel {
        id: String,
        count: u32,
        data: Vec<u8>,
    }
    
    impl ReadModel for BenchReadModel {
        fn model_type() -> &'static str {
            "BenchReadModel"
        }
        
        fn id(&self) -> &str {
            &self.id
        }
        
        fn apply_event(&mut self, _: &dyn cim_domain::DomainEvent) -> Result<(), cim_domain::DomainError> {
            self.count += 1;
            Ok(())
        }
    }
    
    let store = rt.block_on(async {
        NatsReadModelStore::new(client, "bench-read-models".to_string()).await.unwrap()
    });
    
    let model = BenchReadModel {
        id: "bench-model-1".to_string(),
        count: 0,
        data: vec![0u8; 10_000],
    };
    
    let metadata = ReadModelMetadata {
        id: model.id.clone(),
        model_type: BenchReadModel::model_type().to_string(),
        schema_version: 1,
        last_updated: chrono::Utc::now(),
        last_event_position: 0,
        metadata: std::collections::HashMap::new(),
    };
    
    c.bench_function("read_model_store_save", |b| {
        b.iter(|| {
            rt.block_on(async {
                store.save(&model, metadata.clone()).await.unwrap()
            })
        });
    });
    
    // Benchmark load (with caching)
    c.bench_function("read_model_store_load_cached", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _loaded = store.load::<BenchReadModel>(&model.id).await.unwrap();
            })
        });
    });
}

fn benchmark_query_building(c: &mut Criterion) {
    c.bench_function("query_builder_simple", |b| {
        b.iter(|| {
            QueryBuilder::new()
                .filter("status", serde_json::json!("active"))
                .limit(100)
                .build()
        });
    });
    
    c.bench_function("query_builder_complex", |b| {
        b.iter(|| {
            QueryBuilder::new()
                .filter("status", serde_json::json!("active"))
                .filter("category", serde_json::json!("electronics"))
                .filter("price_min", serde_json::json!(100))
                .filter("price_max", serde_json::json!(1000))
                .sort_by("created_at", SortDirection::Descending)
                .limit(50)
                .offset(100)
                .build()
        });
    });
    
    c.bench_function("pagination_calculation", |b| {
        b.iter(|| {
            for page in 0..100 {
                let _ = Pagination::from_query(20, page * 20, 10_000);
            }
        });
    });
}

criterion_group!(
    benches,
    benchmark_simple_repository_save,
    benchmark_simple_repository_load,
    benchmark_kv_repository_with_ttl,
    benchmark_read_model_store,
    benchmark_query_building
);

criterion_main!(benches);