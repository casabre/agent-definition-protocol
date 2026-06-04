/**
 * Framework adapters for ADP v0.3.0.
 *
 * This module provides adapter modules that convert between ADP manifests
 * and framework-native configuration objects.
 *
 * Supported frameworks (stubs for future implementation):
 * - langgraph: LangGraph StateGraph and related constructs
 * - autogen: AutoGen GroupChat, Assistants, and tools
 * - crewai: CrewAI agents and tasks
 * - llamaindex: LlamaIndex QueryEngine and pipelines
 * - google_adk: Google Agent Development Kit
 * - openai_agents: OpenAI Agents SDK
 * - pydantic_ai: Pydantic AI
 * - semantic_kernel: Semantic Kernel
 */

use crate::adp::Adp;
use serde_json::Value;

/// Trait for all framework adapters
pub trait Adapter {
    /// Unique identifier for the framework
    fn framework_id(&self) -> &str;

    /// Export ADP manifest to framework-native config
    fn export(&self, manifest: &Adp) -> Value;

    /// Import framework-native config into an ADP manifest
    fn import_from(&self, config: &Value) -> Adp;

    /// Return coverage per ADP section
    fn roundtrip_fidelity(&self) -> std::collections::HashMap<String, String> {
        let mut fidelity = std::collections::HashMap::new();
        fidelity.insert("flow.graph".to_string(), "faithful".to_string());
        fidelity.insert("tools".to_string(), "faithful".to_string());
        fidelity.insert("runtime.models".to_string(), "faithful".to_string());
        fidelity.insert("tools.policy".to_string(), "lossy".to_string());
        fidelity.insert("memory.stores".to_string(), "lossy".to_string());
        fidelity.insert("memory.working".to_string(), "lossy".to_string());
        fidelity.insert("loop.termination".to_string(), "lossy".to_string());
        fidelity.insert("guardrails.interrupts".to_string(), "lossy".to_string());
        fidelity.insert("workspace".to_string(), "unsupported".to_string());
        fidelity.insert("sandbox".to_string(), "unsupported".to_string());
        fidelity.insert("artifacts".to_string(), "unsupported".to_string());
        fidelity.insert("observability".to_string(), "faithful".to_string());
        fidelity
    }
}

/// Registry for framework adapters
pub struct AdapterRegistry {
    adapters: std::collections::HashMap<String, Box<dyn Adapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        AdapterRegistry {
            adapters: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, framework_id: &str, adapter: Box<dyn Adapter>) {
        self.adapters.insert(framework_id.to_string(), adapter);
    }

    pub fn get(&self, framework_id: &str) -> Option<&dyn Adapter> {
        self.adapters.get(framework_id).map(|a| a.as_ref())
    }

    pub fn available(&self) -> Vec<String> {
        self.adapters.keys().cloned().collect()
    }

    pub fn is_available(&self, framework_id: &str) -> bool {
        self.adapters.contains_key(framework_id)
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Placeholder implementations for each framework adapter
// These are stubs that will be fully implemented in future work

pub struct LangGraphAdapter;
impl Adapter for LangGraphAdapter {
    fn framework_id(&self) -> &str { "langgraph" }
    fn export(&self, manifest: &Adp) -> Value { serde_json::to_value(manifest).unwrap_or(Value::Null) }
    fn import_from(&self, config: &Value) -> Adp { Adp::default() }
}

pub struct AutoGenAdapter;
impl Adapter for AutoGenAdapter {
    fn framework_id(&self) -> &str { "autogen" }
    fn export(&self, manifest: &Adp) -> Value { serde_json::to_value(manifest).unwrap_or(Value::Null) }
    fn import_from(&self, config: &Value) -> Adp { Adp::default() }
}

pub struct CrewAIAdapter;
impl Adapter for CrewAIAdapter {
    fn framework_id(&self) -> &str { "crewai" }
    fn export(&self, manifest: &Adp) -> Value { serde_json::to_value(manifest).unwrap_or(Value::Null) }
    fn import_from(&self, config: &Value) -> Adp { Adp::default() }
}

pub struct LlamaIndexAdapter;
impl Adapter for LlamaIndexAdapter {
    fn framework_id(&self) -> &str { "llamaindex" }
    fn export(&self, manifest: &Adp) -> Value { serde_json::to_value(manifest).unwrap_or(Value::Null) }
    fn import_from(&self, config: &Value) -> Adp { Adp::default() }
}

pub struct GoogleADKAdapter;
impl Adapter for GoogleADKAdapter {
    fn framework_id(&self) -> &str { "google_adk" }
    fn export(&self, manifest: &Adp) -> Value { serde_json::to_value(manifest).unwrap_or(Value::Null) }
    fn import_from(&self, config: &Value) -> Adp { Adp::default() }
}

pub struct OpenAIAgentsAdapter;
impl Adapter for OpenAIAgentsAdapter {
    fn framework_id(&self) -> &str { "openai_agents" }
    fn export(&self, manifest: &Adp) -> Value { serde_json::to_value(manifest).unwrap_or(Value::Null) }
    fn import_from(&self, config: &Value) -> Adp { Adp::default() }
}

pub struct PydanticAIAdapter;
impl Adapter for PydanticAIAdapter {
    fn framework_id(&self) -> &str { "pydantic_ai" }
    fn export(&self, manifest: &Adp) -> Value { serde_json::to_value(manifest).unwrap_or(Value::Null) }
    fn import_from(&self, config: &Value) -> Adp { Adp::default() }
}

pub struct SemanticKernelAdapter;
impl Adapter for SemanticKernelAdapter {
    fn framework_id(&self) -> &str { "semantic_kernel" }
    fn export(&self, manifest: &Adp) -> Value { serde_json::to_value(manifest).unwrap_or(Value::Null) }
    fn import_from(&self, config: &Value) -> Adp { Adp::default() }
}
