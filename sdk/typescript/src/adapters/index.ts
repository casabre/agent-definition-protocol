/**
 * Framework adapters for ADP v0.3.0.
 *
 * This package provides adapter modules that convert between ADP manifests
 * and framework-native configuration objects.
 *
 * Supported frameworks:
 * - langgraph: LangGraph StateGraph and related constructs
 * - autogen: AutoGen GroupChat, Assistants, and tools
 * - crewai: CrewAI agents and tasks
 * - llamaindex: LlamaIndex QueryEngine and pipelines
 * - google_adk: Google Agent Development Kit
 * - openai_agents: OpenAI Agents SDK
 * - pydantic_ai: Pydantic AI
 * - semantic_kernel: Semantic Kernel
 */

export { AdapterBase } from "./base";
export { AdapterRegistry } from "./registry";

// Import all adapters to register them
export { LangGraphAdapter } from "./langgraph";
export { AutoGenAdapter } from "./autogen";
export { CrewAIAdapter } from "./crewai";
export { LlamaIndexAdapter } from "./llamaindex";
export { GoogleADKAdapter } from "./google_adk";
export { OpenAIAgentsAdapter } from "./openai_agents";
export { PydanticAIAdapter } from "./pydantic_ai";
export { SemanticKernelAdapter } from "./semantic_kernel";
