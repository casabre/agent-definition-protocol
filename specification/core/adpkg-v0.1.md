# Agent Package (ADPKG) v0.1

## Purpose

ADPKG is an OPC/ZIP-based package for distributing agents alongside their runtime, evaluation, and metadata. Recommended extension: `.adpkg`.

## Layout

```
/[root]
  [Content_Types].xml
  /adp/agent.yaml
  /acs/container.yaml
  /eval/...
  /src/...
  /tools/...
  /metadata/version.json
  /metadata/build-info.json
  /docs/readme.md
```

- `/adp/agent.yaml` and `/acs/container.yaml` are required parts.
- `/eval`, `/tools`, `/src`, `/docs` are optional but recommended for reproducibility.
- `/metadata/version.json` records package identity and versioning.
- `/metadata/build-info.json` can capture build tool metadata (free-form at v0.1).

## [Content_Types].xml

A minimal content types file declaring ADP and ACS parts:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="yaml" ContentType="application/x-yaml"/>
  <Override PartName="/adp/agent.yaml" ContentType="application/vnd.adp.agent+yaml"/>
  <Override PartName="/acs/container.yaml" ContentType="application/vnd.adp.container+yaml"/>
  <Override PartName="/metadata/version.json" ContentType="application/json"/>
</Types>
```

## Metadata schema

`/metadata/version.json` should follow a simple schema:

```json
{
  "agent_id": "agent.acme.analytics",
  "agent_version": "0.1.0",
  "spec_version": "0.1",
  "build_timestamp": "2024-01-01T00:00:00Z"
}
```

A JSON Schema for this file is provided at `specification/schemas/adpkg-metadata-v0.1.schema.json`.

## Versioning

This document defines v0.1 of the ADPKG format. Future versions may add signing, digests, and richer metadata parts.
