# CIM Domain Event Schemas

This directory contains **standalone JSON Schema definitions** for all CIM Domain event payload types. These schemas can be used **without requiring the cim-domain Rust crate** as a dependency.

## 📋 Generated Schemas (22 total)

- **WorkflowCompleted** - WorkflowCompleted.json
- **WorkflowSuspended** - WorkflowSuspended.json
- **Pagination** - Pagination.json
- **DomainEventEnvelope** - DomainEventEnvelope.json
- **EventMetadata** - EventMetadata.json
- **WorkflowStarted** - WorkflowStarted.json
- **WorkflowTransitioned** - WorkflowTransitioned.json
- **WorkflowFailed** - WorkflowFailed.json
- **WorkflowEvent** - WorkflowEvent.json
- **QueryResult** - QueryResult.json
- **QueryCriteria** - QueryCriteria.json
- **EventEnvelope** - EventEnvelope.json
- **WorkflowTransitionExecuted** - WorkflowTransitionExecuted.json
- **WorkflowCancelled** - WorkflowCancelled.json
- **CommandStatus** - CommandStatus.json
- **PropagationScope** - PropagationScope.json
- **CommandAcknowledgment** - CommandAcknowledgment.json
- **QueryResponse** - QueryResponse.json
- **SagaEvent** - SagaEvent.json
- **QueryStatus** - QueryStatus.json
- **SortDirection** - SortDirection.json
- **WorkflowResumed** - WorkflowResumed.json

## 🎯 What Are These Schemas?

These JSON schemas define the exact structure and validation rules for:
- **Workflow Events** - State changes in workflow execution
- **CQRS Types** - Command/Query acknowledgments and responses  
- **Saga Events** - Distributed transaction orchestration
- **Query Support** - Data retrieval and pagination
- **Infrastructure** - Event envelopes and metadata

## 🚀 Usage Examples

### Validation with ajv (JavaScript/Node.js)
```bash
npm install ajv ajv-cli
ajv validate -s WorkflowStarted.json -d your-event.json
```

### Code Generation

#### TypeScript
```bash
npx quicktype --src-lang schema --lang typescript WorkflowStarted.json
```

#### Python
```bash
pip install datamodel-code-generator
datamodel-codegen --input WorkflowStarted.json --output workflow_models.py
```

#### Go
```bash
go install github.com/atombender/go-jsonschema/cmd/gojsonschema@latest
gojsonschema --package main WorkflowStarted.json > workflow_models.go
```

#### Java
```bash
# Using jsonschema2pojo
jsonschema2pojo --source WorkflowStarted.json --target java-src
```

### OpenAPI Integration
```yaml
components:
  schemas:
    WorkflowStarted:
      $ref: 'https://schemas.cim-domain.ai/WorkflowStarted.json'
```

## 📁 Files

- `index.json` - Schema catalog/index with metadata
- `all-schemas.json` - Combined schemas in single file
- `README.md` - This documentation
- `[SchemaName].json` - Individual schema files

## 🌐 Schema URLs

All schemas are available at:
```
https://schemas.cim-domain.ai/[SchemaName].json
```

Generated: 2025-08-10 06:28:36 UTC
Version: 0.5.0
Source: https://github.com/thecowboyai/cim-domain
