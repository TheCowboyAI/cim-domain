// Copyright 2025 Cowboy AI, LLC.

//! Schema migration support for persistence layer

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema version information
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
    /// Patch version
    pub patch: u32,
}

impl SchemaVersion {
    /// Create a new schema version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    
    /// Parse from string (e.g., "1.2.3")
    pub fn parse(s: &str) -> Result<Self, MigrationError> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(MigrationError::InvalidVersion(s.to_string()));
        }
        
        Ok(Self {
            major: parts[0].parse().map_err(|_| MigrationError::InvalidVersion(s.to_string()))?,
            minor: parts[1].parse().map_err(|_| MigrationError::InvalidVersion(s.to_string()))?,
            patch: parts[2].parse().map_err(|_| MigrationError::InvalidVersion(s.to_string()))?,
        })
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::cmp::PartialOrd for SchemaVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for SchemaVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major.cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

/// Migration errors
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    /// Invalid version format
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),
    
    /// Migration failed
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    
    /// Version conflict
    #[error("Version conflict: current {current}, target {target}")]
    VersionConflict { current: SchemaVersion, target: SchemaVersion },
    
    /// No migration path
    #[error("No migration path from {from} to {to}")]
    NoMigrationPath { from: SchemaVersion, to: SchemaVersion },
}

/// Migration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationMetadata {
    /// Migration ID
    pub id: String,
    /// From version
    pub from_version: SchemaVersion,
    /// To version
    pub to_version: SchemaVersion,
    /// Description
    pub description: String,
    /// Applied at timestamp
    pub applied_at: Option<DateTime<Utc>>,
    /// Migration duration in seconds
    pub duration_seconds: Option<u64>,
}

/// Migration trait
#[async_trait]
pub trait Migration: Send + Sync {
    /// Get migration metadata
    fn metadata(&self) -> MigrationMetadata;
    
    /// Apply the migration
    async fn up(&self) -> Result<(), MigrationError>;
    
    /// Rollback the migration
    async fn down(&self) -> Result<(), MigrationError>;
    
    /// Validate migration can be applied
    async fn validate(&self) -> Result<(), MigrationError>;
}

/// Migration runner
pub struct MigrationRunner {
    migrations: Vec<Box<dyn Migration>>,
    applied_migrations: HashMap<String, MigrationMetadata>,
    current_version: SchemaVersion,
}

impl MigrationRunner {
    /// Create a new migration runner
    pub fn new(current_version: SchemaVersion) -> Self {
        Self {
            migrations: Vec::new(),
            applied_migrations: HashMap::new(),
            current_version,
        }
    }
    
    /// Add a migration
    pub fn add_migration(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(migration);
    }
    
    /// Get current schema version
    pub fn current_version(&self) -> &SchemaVersion {
        &self.current_version
    }
    
    /// Find migration path to target version
    pub fn find_migration_path(
        &self,
        target: &SchemaVersion,
    ) -> Result<Vec<&Box<dyn Migration>>, MigrationError> {
        let mut path = Vec::new();
        let mut current = self.current_version.clone();
        
        while current < *target {
            let next_migration = self.migrations.iter()
                .find(|m| m.metadata().from_version == current)
                .ok_or_else(|| MigrationError::NoMigrationPath {
                    from: current.clone(),
                    to: target.clone(),
                })?;
            
            path.push(next_migration);
            current = next_migration.metadata().to_version.clone();
        }
        
        Ok(path)
    }
    
    /// Run migrations to target version
    pub async fn migrate_to(&mut self, target: SchemaVersion) -> Result<(), MigrationError> {
        if self.current_version == target {
            return Ok(());
        }
        
        if self.current_version > target {
            return self.rollback_to(target).await;
        }
        
        let migration_path = self.find_migration_path(&target)?;
        
        // Collect migration indices to avoid borrow checker issues
        let migration_indices: Vec<usize> = migration_path
            .into_iter()
            .map(|m| self.migrations.iter().position(|x| x.metadata().id == m.metadata().id).unwrap())
            .collect();
        
        for idx in migration_indices {
            let migration = &self.migrations[idx];
            let metadata = migration.metadata();
            
            // Validate migration
            migration.validate().await?;
            
            // Apply migration
            let start = std::time::Instant::now();
            migration.up().await?;
            let duration = start.elapsed().as_secs();
            
            // Update metadata
            let mut applied_metadata = metadata.clone();
            applied_metadata.applied_at = Some(Utc::now());
            applied_metadata.duration_seconds = Some(duration);
            
            self.applied_migrations.insert(
                applied_metadata.id.clone(),
                applied_metadata,
            );
            
            self.current_version = metadata.to_version.clone();
        }
        
        Ok(())
    }
    
    /// Rollback to target version
    pub async fn rollback_to(&mut self, target: SchemaVersion) -> Result<(), MigrationError> {
        if self.current_version <= target {
            return Err(MigrationError::VersionConflict {
                current: self.current_version.clone(),
                target,
            });
        }
        
        // Find migrations to rollback
        let mut rollback_migrations = Vec::new();
        let mut current = self.current_version.clone();
        
        while current > target {
            let migration = self.migrations.iter()
                .find(|m| m.metadata().to_version == current)
                .ok_or_else(|| MigrationError::NoMigrationPath {
                    from: current.clone(),
                    to: target.clone(),
                })?;
            
            rollback_migrations.push(migration);
            current = migration.metadata().from_version.clone();
        }
        
        // Apply rollbacks in reverse order
        for migration in rollback_migrations {
            migration.down().await?;
            
            self.applied_migrations.remove(&migration.metadata().id);
            self.current_version = migration.metadata().from_version.clone();
        }
        
        Ok(())
    }
    
    /// Get applied migrations
    pub fn applied_migrations(&self) -> &HashMap<String, MigrationMetadata> {
        &self.applied_migrations
    }
    
    /// Get pending migrations
    pub fn pending_migrations(&self) -> Vec<&Box<dyn Migration>> {
        self.migrations.iter()
            .filter(|m| !self.applied_migrations.contains_key(&m.metadata().id))
            .collect()
    }
}

/// Example migration implementation
pub struct ExampleMigration {
    metadata: MigrationMetadata,
}

impl ExampleMigration {
    pub fn new() -> Self {
        Self {
            metadata: MigrationMetadata {
                id: "001_initial".to_string(),
                from_version: SchemaVersion::new(0, 0, 0),
                to_version: SchemaVersion::new(1, 0, 0),
                description: "Initial schema".to_string(),
                applied_at: None,
                duration_seconds: None,
            },
        }
    }
}

#[async_trait]
impl Migration for ExampleMigration {
    fn metadata(&self) -> MigrationMetadata {
        self.metadata.clone()
    }
    
    async fn up(&self) -> Result<(), MigrationError> {
        // Apply migration logic
        Ok(())
    }
    
    async fn down(&self) -> Result<(), MigrationError> {
        // Rollback logic
        Ok(())
    }
    
    async fn validate(&self) -> Result<(), MigrationError> {
        // Validation logic
        Ok(())
    }
}