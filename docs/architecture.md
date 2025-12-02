# ADP Architecture Overview

This document provides a high-level architectural overview of the Agent Definition Protocol (ADP).

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         ADP Manifest                            │
│  ┌──────────────┐  ┌──────────┐  ┌──────────────┐               │
│  │   Identity   │  │ Runtime  │  │    Flow      │               │
│  │  (id, name)  │  │(execution│  │  (AFG v0.1)  │               │
│  └──────────────┘  │ backends)│  └──────────────┘               │
│                    └──────────┘                                 │
│  ┌──────────────┐  ┌──────────┐  ┌──────────────┐               │
│  │   Tools      │  │Evaluation│  │ Governance   │               │
│  │ (MCP/HTTP)   │  │ (suites) │  │ (scopes)     │               │
│  └──────────────┘  └──────────┘  └──────────────┘               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ packages into
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    ADPKG (OCI Package)                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              OCI Image Layout                            │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────┐            │   │
│  │  │  Config  │  │ Manifest │  │   Layers     │            │   │
│  │  │  Blob    │  │          │  │  (Package)   │            │   │
│  │  └──────────┘  └──────────┘  └──────────────┘            │   │
│  │                                                          │   │
│  │  Package Layer Contents:                                 │   │
│  │  • /adp/agent.yaml (required)                            │   │
│  │  • /eval/ (evaluation suites)                            │   │
│  │  • /tools/ (tool definitions)                            │   │
│  │  • /src/ (source code)                                   │   │
│  │  • /metadata/ (package metadata)                         │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Provenance & Security                       │   │
│  │  • Notary v2 Signatures                                  │   │
│  │  • SPDX SBOM                                             │   │
│  │  • Build Metadata                                        │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ distributes via
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    OCI Registry                                 │
│  (Docker Hub, GHCR, Azure Container Registry, etc.)             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ consumed by
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Runtime Environments                         │
│  ┌──────────────┐  ┌──────────┐  ┌──────────────┐               │
│  │   Docker     │  │   WASM   │  │   Python     │               │
│  │  Container   │  │  Runtime │  │  Runtime     │               │
│  └──────────────┘  └──────────┘  └──────────────┘               │
│  ┌──────────────┐  ┌──────────┐  ┌──────────────┐               │
│  │ TypeScript   │  │  Binary  │  │   Custom     │               │
│  │  Runtime     │  │  Runtime │  │   Runtime    │               │
│  └──────────────┘  └──────────┘  └──────────────┘               │
└─────────────────────────────────────────────────────────────────┘
```

## Component Relationships

### ADP Manifest Components

```
ADP Manifest
├── Identity & Metadata
│   ├── id (required)
│   ├── name, description, owner
│   └── tags
├── Runtime
│   └── execution[] (multi-backend)
│       ├── docker
│       ├── wasm
│       ├── python
│       ├── typescript
│       ├── binary
│       └── custom
├── Flow (AFG v0.1)
│   ├── graph
│   │   ├── nodes[]
│   │   ├── edges[]
│   │   ├── start_nodes[]
│   │   └── end_nodes[]
│   └── ui (metadata)
├── Tools
│   ├── mcp_servers[]
│   ├── http_apis[]
│   └── sql_functions[]
├── Evaluation
│   ├── suites[]
│   ├── kpis[]
│   └── promotion_policy
├── Memory
│   ├── provider
│   ├── endpoint
│   └── index
├── Governance
│   ├── identity
│   ├── data_scopes
│   ├── telemetry_endpoint
│   └── guardrail_policy_set
└── Deployment
    └── environments[]
```

## Data Flow

```
┌─────────────┐
│   Developer │
│   creates   │
│  ADP manifest│
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│  SDK validates  │
│  against schema │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│  SDK packages   │
│  into ADPKG     │
│  (OCI format)   │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│  Sign & attach  │
│  SBOM (optional)│
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│  Push to OCI    │
│  Registry       │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│  Runtime pulls  │
│  & unpacks      │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│  Runtime        │
│  executes agent │
│  using manifest │
└─────────────────┘
```

## Integration Points

### External Protocols

```
ADP
├── MCP (Model Context Protocol)
│   └── Tools integration
├── OCI (Open Container Initiative)
│   └── Packaging & distribution
├── OpenTelemetry
│   └── Telemetry & observability
├── A2A (Agent-to-Agent)
│   └── Interoperability mapping
└── OpenAPI
    └── HTTP API tool definitions
```

### SDK Ecosystem

```
┌─────────────────────────────────────────┐
│         ADP Specification               │
│      (spec/adp-v0.1.0.md)               │
└──────────────┬──────────────────────────┘
               │
       ┌───────┴───────┐
       │               │
       ▼               ▼
┌──────────┐     ┌──────────┐
│  Schemas │     │ Examples │
│  (JSON)  │     │  (YAML)  │
└────┬─────┘     └────┬─────┘
     │                │
     └───────┬────────┘
             │
             ▼
┌─────────────────────────────────────────┐
│         Reference SDKs                  │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐    │
│  │Python│ │  TS  │ │ Rust │ │  Go  │    │
│  └──────┘ └──────┘ └──────┘ └──────┘    │
└─────────────────────────────────────────┘
```

## Conformance Classes

```
┌─────────────────────────────────────────┐
│         ADP Conformance                 │
│                                         │
│  ┌───────────────────────────────────┐  │
│  │      ADP-Minimal                  │  │
│  │  • Required fields only           │  │
│  │  • Basic runtime                  │  │
│  │  • Empty flow/eval OK             │  │
│  └───────────────────────────────────┘  │
│              │                          │
│              │ extends                  │
│              ▼                          │
│  ┌───────────────────────────────────┐  │
│  │      ADP-Full                     │  │
│  │  • All required fields            │  │
│  │  • Complete runtime               │  │
│  │  • Flow & evaluation              │  │
│  │  • OCI packaging                  │  │
│  │  • Schema validation              │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## Key Design Principles

1. **Multi-Backend Runtime**: Support multiple execution environments (Docker, WASM, Python, TypeScript, binary, custom)

2. **Portable Packaging**: OCI-based distribution for compatibility with existing container ecosystems

3. **Composable**: Flow graphs allow complex agent workflows

4. **Evaluable**: Built-in evaluation framework for quality assurance

5. **Governable**: Data scopes, telemetry, and guardrails for enterprise use

6. **Extensible**: Vendor extensions via `extensions.x_<vendor>` namespace

7. **Validatable**: JSON Schemas for all components

8. **Interoperable**: Mappings to MCP, OCI, A2A, OpenTelemetry

## References

- [ADP Specification](../spec/adp-v0.1.0.md)
- [Runtime Specification](../spec/runtime.md)
- [Flow Specification](../spec/flow.md)
- [Evaluation Specification](../spec/evaluation.md)
- [ADPKG over OCI](../spec/adpkg-oci.md)

