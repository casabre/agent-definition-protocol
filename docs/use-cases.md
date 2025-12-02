# ADP Use Cases

This document illustrates concrete scenarios where ADP provides value. Each scenario shows "before ADP" vs "with ADP" workflows.

## Use Case 1: Porting a LangGraph Agent to Production

### Problem

You've built a LangGraph agent locally and want to deploy it to production. Currently, you need to:
- Manually document runtime requirements
- Write deployment scripts for each environment
- Set up evaluation separately
- Configure telemetry manually
- Document tool integrations

### Before ADP

```bash
# Manual deployment process
1. Document Python version, dependencies in README
2. Write Dockerfile manually
3. Create separate eval scripts
4. Configure telemetry in code
5. Document MCP servers in wiki
6. Deploy to dev → staging → prod manually
7. Hope nothing breaks between environments
```

**Issues**:
- Context scattered across multiple files
- No guarantee dev = staging = prod
- Evaluation not tied to agent
- Manual documentation drift

### With ADP

```yaml
# Single ADP manifest defines everything
adp_version: "0.1.0"
id: "acme.analytics.agent"
runtime:
  execution:
    - backend: "python"
      id: "langgraph-backend"
      entrypoint: "acme_agents.main:app"
      environment:
        python_version: "3.11"
      dependencies:
        - "langgraph==0.2.1"
        - "openai==1.40.0"
flow:
  id: "acme.flow"
  graph:
    nodes: [...]
evaluation:
  suites:
    - id: "regression"
      metrics: [...]
tools:
  mcp_servers:
    - id: "metrics-mcp"
      endpoint: "bin/metrics-mcp"
```

**Benefits**:
- Single source of truth
- Package once, deploy anywhere
- Evaluation attached to agent
- Automated deployment from manifest

```bash
# Simple deployment
adp pack --source . --output oci-package
adp push oci-package registry.acme.com/agents/analytics:1.0.0
adp deploy registry.acme.com/agents/analytics:1.0.0 --env prod
```

## Use Case 2: Multi-Backend Agent Deployment

### Problem

Your agent uses multiple backends:
- Python for LLM orchestration
- WASM for fast embeddings
- Docker container for legacy service
- TypeScript for web tooling

Each backend has different deployment requirements.

### Before ADP

```bash
# Separate deployment for each component
docker build -t planner:1.0.0 ./planner
docker push planner:1.0.0

# Deploy WASM separately
wasm-pack build --target wasm32-wasi ./embedder
# Upload to CDN...

# Python dependencies
pip install -r requirements.txt
# Deploy to Lambda/ECS/...

# TypeScript build
npm run build
# Deploy to Vercel/...

# Hope they all work together
```

**Issues**:
- No single deployment unit
- Version drift between components
- Complex orchestration
- Hard to test end-to-end

### With ADP

```yaml
runtime:
  execution:
    - backend: "docker"
      id: "planner"
      image: "acme/planner:1.0.0"
    - backend: "wasm"
      id: "embedder"
      module: "embedder.wasm"
    - backend: "python"
      id: "orchestrator"
      entrypoint: "acme_agents.main:app"
    - backend: "typescript"
      id: "tools"
      entrypoint: "dist/index.js"
```

**Benefits**:
- Single manifest describes all backends
- Atomic deployment of entire agent
- Version consistency guaranteed
- Runtime handles orchestration

```bash
# Single command deploys everything
adp deploy agent.yaml --env prod
# Runtime handles:
# - Docker container
# - WASM module
# - Python environment
# - TypeScript build
```

## Use Case 3: Evaluation-Driven Promotion

### Problem

You want to automatically promote agents from dev → staging → prod only if evaluation passes. Currently, this requires:
- Manual evaluation runs
- Manual comparison of results
- Manual promotion decisions
- No audit trail

### Before ADP

```bash
# Manual process
1. Run eval script manually
2. Check results in spreadsheet
3. Compare thresholds manually
4. If pass, manually deploy to staging
5. Repeat for staging → prod
6. Hope you didn't miss anything
```

**Issues**:
- Manual, error-prone process
- No automated gates
- Evaluation not tied to agent version
- No audit trail

### With ADP

```yaml
evaluation:
  suites:
    - id: "groundedness"
      metrics:
        - id: "groundedness.score"
          type: "llm_judge"
          threshold: 4.0
    - id: "latency"
      metrics:
        - id: "latency.p95_ms"
          type: "telemetry"
          threshold: 500
  promotion_policy:
    require_passing_suites:
      - "groundedness"
      - "latency"
```

**Benefits**:
- Evaluation defined with agent
- Automated promotion gates
- Clear pass/fail criteria
- Audit trail in package

```bash
# Automated promotion
adp eval agent.yaml
# If all suites pass:
adp promote agent.yaml --from dev --to staging
# Evaluation results stored in package metadata
```

## Use Case 4: Cross-Platform Agent Distribution

### Problem

You want to distribute your agent to multiple platforms:
- Cloud providers (AWS, Azure, GCP)
- Edge devices
- On-premises deployments
- Different container registries

Each platform has different requirements and formats.

### Before ADP

```bash
# Platform-specific packaging
# AWS
aws ecr push acme/agent:1.0.0

# Azure
az acr push acme.azurecr.io/agent:1.0.0

# GCP
gcloud container images push gcr.io/acme/agent:1.0.0

# Each with different metadata, tags, etc.
```

**Issues**:
- Platform-specific tooling
- Inconsistent formats
- Manual porting between platforms
- No standard way to describe agent

### With ADP

```bash
# Package once
adp pack --source . --output oci-package

# Push to any OCI-compatible registry
adp push oci-package aws-ecr://acme/agent:1.0.0
adp push oci-package azure-acr://acme/agent:1.0.0
adp push oci-package gcr://acme/agent:1.0.0
adp push oci-package docker.io/acme/agent:1.0.0

# Same package, any registry
```

**Benefits**:
- Standard OCI format works everywhere
- Single package, multiple registries
- Consistent metadata
- No platform-specific code

## Use Case 5: Agent Composition and Reuse

### Problem

You have multiple agents sharing common components:
- Same evaluation suite
- Same tool integrations
- Similar flow patterns

You want to reuse these without copy-paste.

### Before ADP

```yaml
# agent-1.yaml
evaluation:
  suites:
    - id: "groundedness"
      # ... 50 lines of config

# agent-2.yaml (copy-paste)
evaluation:
  suites:
    - id: "groundedness"
      # ... same 50 lines (now duplicated)
```

**Issues**:
- Copy-paste leads to drift
- Updates require changes in multiple places
- No versioning of shared components

### With ADP (Future: Composition)

```yaml
# base-agent.yaml
evaluation:
  suites:
    - id: "groundedness"
      # ... shared config

# agent-1.yaml
extends: "base-agent:v1.0.0"
id: "agent-1"
# Only agent-specific fields

# agent-2.yaml
extends: "base-agent:v1.0.0"
id: "agent-2"
# Only agent-specific fields
```

**Benefits**:
- Single source of truth for shared components
- Versioned base agents
- Easy updates (change base, all agents benefit)
- Composition coming in v0.2.0

## Use Case 6: Enterprise Governance and Compliance

### Problem

Enterprise requirements:
- Data access controls
- Audit trails
- Telemetry requirements
- Guardrail policies
- SBOM for security scanning

Currently requires manual configuration and documentation.

### Before ADP

```bash
# Scattered configuration
# 1. Data access in IAM policies
# 2. Telemetry in code
# 3. Guardrails in separate policy files
# 4. SBOM generated separately
# 5. Documentation in wiki
```

**Issues**:
- Configuration scattered
- No single source of truth
- Hard to audit
- Compliance checks manual

### With ADP

```yaml
governance:
  identity: "service-principal://acme/analytics-agent"
  allowed_data_scopes: ["public", "analytics"]
  forbidden_data_scopes: ["pii", "financial"]
  telemetry_endpoint: "https://telemetry.acme.com"
  guardrail_policy_set: "policies/acme-guardrails.yaml"
```

**Benefits**:
- Governance defined with agent
- Automated compliance checks
- Audit trail in package
- SBOM attached automatically
- Signatures for trust

```bash
# Package includes governance
adp pack --source . --output oci-package

# Verify compliance
adp verify oci-package --check-governance

# Check SBOM
adp inspect oci-package --show-sbom
```

## Summary

ADP provides value by:

1. **Single Source of Truth**: All agent context in one manifest
2. **Portability**: Package once, deploy anywhere
3. **Automation**: Evaluation-driven promotion, automated deployment
4. **Governance**: Built-in compliance and audit trails
5. **Interoperability**: Works with existing tools (OCI, MCP, OTel)
6. **Composability**: (Future) Reuse shared components

## Next Steps

- Try the [getting started guide](getting-started.md)
- Explore [examples](../examples/)
- Read the [ADP specification](../spec/adp-v0.1.0.md)

