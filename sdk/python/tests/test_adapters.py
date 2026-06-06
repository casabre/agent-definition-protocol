"""Tests for all ADP framework adapters: registry, pydantic_ai, semantic_kernel,
langgraph, autogen, crewai, llamaindex, google_adk, openai_agents."""

import pytest
from adp_sdk.adp_model import ADP, RuntimeModel, RuntimeEntry, FlowModel, GraphModel, ToolsModel, HTTPAPIModel


# ---------------------------------------------------------------------------
# Helper: minimal ADP manifest for adapter tests
# ---------------------------------------------------------------------------

def _make_adp(
    nodes=None, edges=None, start_nodes=None, end_nodes=None,
    tools_data=None, models=None, adapter_hints=None,
    extra_kwargs=None,
) -> ADP:
    """Build a minimal ADP object for adapter export/import tests."""
    nodes = nodes or [{"id": "n1", "kind": "llm", "model_ref": "gpt4"}]
    edges = edges or []
    start_nodes = start_nodes or [nodes[0]["id"]]
    end_nodes = end_nodes or []

    runtime_kwargs = {}
    if models:
        runtime_kwargs["models"] = models
    if adapter_hints:
        runtime_kwargs["adapter_hints"] = adapter_hints

    return ADP(
        adp_version="0.3.0",
        id="test.adapter",
        runtime=RuntimeModel(
            execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")],
            **runtime_kwargs,
        ),
        flow=FlowModel(
            id="test-flow",
            graph=GraphModel(
                nodes=nodes,
                edges=edges,
                start_nodes=start_nodes,
                end_nodes=end_nodes,
            ),
        ),
        **(extra_kwargs or {}),
    )


# ===========================================================================
# AdapterRegistry tests
# ===========================================================================

class TestAdapterRegistry:
    def test_register_and_get(self):
        from adp_sdk.adapters.registry import AdapterRegistry
        from adp_sdk.adapters.base import AdapterBase

        class _TestAdapter(AdapterBase):
            framework_id = "_test_registry_adapter"

            def export(self, manifest):
                return {}

            def import_from(self, config):
                return _make_adp()

        AdapterRegistry.register(_TestAdapter)
        adapter = AdapterRegistry.get("_test_registry_adapter")
        assert isinstance(adapter, _TestAdapter)

    def test_get_unknown_raises(self):
        from adp_sdk.adapters.registry import AdapterRegistry
        with pytest.raises(ValueError, match="Unknown framework"):
            AdapterRegistry.get("nonexistent_framework_xyz")

    def test_available_returns_list(self):
        from adp_sdk.adapters.registry import AdapterRegistry
        result = AdapterRegistry.available()
        assert isinstance(result, list)

    def test_is_available_true(self):
        from adp_sdk.adapters.pydantic_ai import PydanticAIAdapter
        from adp_sdk.adapters.registry import AdapterRegistry
        # Instantiating registers the adapter class
        PydanticAIAdapter()
        assert AdapterRegistry.is_available("pydantic_ai") is True

    def test_is_available_false(self):
        from adp_sdk.adapters.registry import AdapterRegistry
        assert AdapterRegistry.is_available("nonexistent_xyz") is False


# ===========================================================================
# AdapterBase tests
# ===========================================================================

class TestAdapterBase:
    def test_roundtrip_fidelity_default(self):
        """roundtrip_fidelity returns dict with expected keys."""
        from adp_sdk.adapters.base import AdapterBase

        class _Concrete(AdapterBase):
            framework_id = "_test_concrete_base"

            def export(self, manifest):
                super().export(manifest)  # covers abstract pass body
                return {}

            def import_from(self, config):
                super().import_from(config)  # covers abstract pass body
                return _make_adp()

        adapter = _Concrete()
        fidelity = adapter.roundtrip_fidelity()
        assert isinstance(fidelity, dict)
        assert "flow.graph" in fidelity
        assert "workspace" in fidelity
        # Exercise both abstract methods (covers pass bodies)
        adp = _make_adp()
        result = adapter.export(adp)
        assert result == {}
        imported = adapter.import_from({})
        assert isinstance(imported, ADP)


# ===========================================================================
# PydanticAI adapter tests
# ===========================================================================

class TestPydanticAIAdapter:
    def setup_method(self):
        import adp_sdk.adapters  # noqa: F401 — triggers registration
        from adp_sdk.adapters.pydantic_ai import PydanticAIAdapter
        self.adapter = PydanticAIAdapter()

    def test_export_llm_node(self):
        adp = _make_adp(nodes=[{"id": "chat", "kind": "llm", "model_ref": "gpt4"}])
        result = self.adapter.export(adp)
        assert "agents" in result
        assert "chat" in result["agents"]
        assert result["agents"]["chat"]["model"] == "gpt4"

    def test_export_tool_node(self):
        adp = _make_adp(nodes=[{"id": "fetch", "kind": "tool"}])
        result = self.adapter.export(adp)
        assert "fetch" in result["agents"]
        assert result["agents"]["fetch"]["deps"]["type"] == "Tool"

    def test_export_edges_become_adjacency(self):
        nodes = [{"id": "a", "kind": "llm"}, {"id": "b", "kind": "llm"}]
        edges = [{"from": "a", "to": "b"}]
        adp = _make_adp(nodes=nodes, edges=edges)
        result = self.adapter.export(adp)
        assert "adjacency" in result
        assert "b" in result["adjacency"]["a"]

    def test_export_tools_mcp(self):
        from adp_sdk.adp_model import MCPServerModel
        adp = _make_adp()
        adp = ADP(
            adp_version="0.3.0",
            id="test.adapter",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n1", "kind": "llm"}], edges=[], start_nodes=["n1"], end_nodes=[])),
            tools=ToolsModel(mcp_servers=[MCPServerModel(id="my-mcp", description="desc", transport="stdio", endpoint="http://mcp")]),
        )
        result = self.adapter.export(adp)
        assert any(t["name"] == "my-mcp" for t in result["tools"])

    def test_export_models(self):
        adp = _make_adp(models=[{"id": "gpt4", "provider": "openai", "model": "gpt-4o"}])
        result = self.adapter.export(adp)
        assert "gpt4" in result["models"]
        assert result["models"]["gpt4"]["provider"] == "openai"

    def test_export_embedder_adapter_hints(self):
        adp = _make_adp(adapter_hints={"pydantic_ai": {"embedder_config": {"provider": "openai"}}})
        result = self.adapter.export(adp)
        assert "embedder" in result
        assert result["embedder"]["provider"] == "openai"

    def test_import_from_basic(self):
        config = {
            "agents": {
                "chat": {"name": "chat", "model": "gpt-4o"},
            },
            "adjacency": {},
            "tools": [],
            "models": {},
        }
        adp = self.adapter.import_from(config)
        assert isinstance(adp, ADP)
        assert adp.adp_version == "0.3.0"

    def test_import_from_tool_agent(self):
        config = {
            "agents": {
                "fetch": {"name": "fetch", "model": "", "deps": {"type": "Tool"}},
            },
            "adjacency": {},
            "tools": [],
            "models": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "tool" for n in nodes)

    def test_import_from_with_adjacency(self):
        config = {
            "agents": {
                "a": {"name": "a", "model": "gpt-4"},
                "b": {"name": "b", "model": "gpt-4"},
            },
            "adjacency": {"a": ["b"]},
            "tools": [],
            "models": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        edges = data["flow"]["graph"]["edges"]
        assert any(e["from"] == "a" and e["to"] == "b" for e in edges)

    def test_import_from_with_tools(self):
        config = {
            "agents": {"a": {"name": "a", "model": "gpt-4"}},
            "adjacency": {},
            "tools": [{"name": "api1", "description": "desc", "type": "HttpApis"}],
            "models": {},
        }
        adp = self.adapter.import_from(config)
        assert adp.tools is not None

    def test_import_from_with_models(self):
        config = {
            "agents": {"a": {"name": "a", "model": "m1"}},
            "adjacency": {},
            "tools": [],
            "models": {"m1": {"provider": "openai", "model": "gpt-4o"}},
        }
        adp = self.adapter.import_from(config)
        assert adp.runtime.models is not None

    def test_import_from_with_embedder(self):
        config = {
            "agents": {},
            "adjacency": {},
            "tools": [],
            "models": {},
            "embedder": {"provider": "openai"},
        }
        adp = self.adapter.import_from(config)
        # embedder goes into adapter_hints
        assert adp.runtime is not None

    def test_import_from_no_nodes(self):
        config = {
            "agents": {},
            "adjacency": {},
            "tools": [],
            "models": {},
        }
        adp = self.adapter.import_from(config)
        assert isinstance(adp, ADP)

    def test_roundtrip_fidelity(self):
        fidelity = self.adapter.roundtrip_fidelity()
        assert fidelity["runtime.models"] == "faithful"


# ===========================================================================
# SemanticKernel adapter tests
# ===========================================================================

class TestSemanticKernelAdapter:
    def setup_method(self):
        import adp_sdk.adapters  # noqa: F401
        from adp_sdk.adapters.semantic_kernel import SemanticKernelAdapter
        self.adapter = SemanticKernelAdapter()

    def test_export_llm_node(self):
        adp = _make_adp(nodes=[{"id": "llm_node", "kind": "llm", "model_ref": "claude3"}])
        result = self.adapter.export(adp)
        assert "llm_node" in result["steps"]
        assert result["steps"]["llm_node"]["type"] == "LLMService"
        assert result["steps"]["llm_node"]["model"] == "claude3"

    def test_export_tool_node(self):
        adp = _make_adp(nodes=[{"id": "tool_node", "kind": "tool", "tool_ref": "my-tool"}])
        result = self.adapter.export(adp)
        assert "tool_node" in result["steps"]
        assert result["steps"]["tool_node"]["type"] == "Function"
        assert result["steps"]["tool_node"]["name"] == "my-tool"

    def test_export_retriever_node(self):
        adp = _make_adp(nodes=[{"id": "ret", "kind": "retriever", "memory_ref": "store1"}])
        result = self.adapter.export(adp)
        assert result["steps"]["ret"]["type"] == "Retriever"
        assert result["steps"]["ret"]["memory"] == "store1"

    def test_export_unknown_node_kind(self):
        adp = _make_adp(nodes=[{"id": "other", "kind": "router"}])
        result = self.adapter.export(adp)
        assert result["steps"]["other"]["type"] == "Node"

    def test_export_edges_become_workflow(self):
        nodes = [{"id": "a", "kind": "llm"}, {"id": "b", "kind": "llm"}]
        edges = [{"from": "a", "to": "b"}]
        adp = _make_adp(nodes=nodes, edges=edges)
        result = self.adapter.export(adp)
        assert any(w["from"] == "a" and w["to"] == "b" for w in result["workflow"])

    def test_export_plugins_from_http_apis(self):
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            tools=ToolsModel(http_apis=[HTTPAPIModel(id="billing", description="Billing API", base_url="https://api.example.com")]),
        )
        result = self.adapter.export(adp)
        assert "billing" in result["plugins"]

    def test_export_ai_services_from_models(self):
        adp = _make_adp(models=[{"id": "m1", "provider": "anthropic", "model": "claude-3-opus"}])
        result = self.adapter.export(adp)
        assert "m1" in result["ai_services"]
        assert result["ai_services"]["m1"]["provider"] == "anthropic"

    def test_export_adapter_hints(self):
        adp = _make_adp(adapter_hints={"semantic_kernel": {"planner": "sequential"}})
        result = self.adapter.export(adp)
        assert "hints" in result
        assert result["hints"]["planner"] == "sequential"

    def test_import_from_llmservice_step(self):
        config = {
            "steps": {"chat": {"type": "LLMService", "model": "gpt-4o"}},
            "workflow": [],
            "plugins": {},
            "ai_services": {},
        }
        adp = self.adapter.import_from(config)
        assert isinstance(adp, ADP)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["id"] == "chat" and n["kind"] == "llm" for n in nodes)

    def test_import_from_function_step(self):
        config = {
            "steps": {"fetch": {"type": "Function", "name": "my-fn"}},
            "workflow": [],
            "plugins": {},
            "ai_services": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "tool" for n in nodes)

    def test_import_from_retriever_step(self):
        config = {
            "steps": {"ret": {"type": "Retriever", "memory": "store1"}},
            "workflow": [],
            "plugins": {},
            "ai_services": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "retriever" for n in nodes)

    def test_import_from_router_step(self):
        config = {
            "steps": {"r": {"type": "Router"}},
            "workflow": [],
            "plugins": {},
            "ai_services": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "router" for n in nodes)

    def test_import_from_unknown_step_type(self):
        config = {
            "steps": {"weird": {"type": "SomethingNew"}},
            "workflow": [],
            "plugins": {},
            "ai_services": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["id"] == "weird" for n in nodes)

    def test_import_from_plugins_http_apis(self):
        config = {
            "steps": {},
            "workflow": [],
            "plugins": {"billing": {"type": "HttpApis", "description": "Billing", "endpoint": "https://api.example.com"}},
            "ai_services": {},
        }
        adp = self.adapter.import_from(config)
        assert adp.tools is not None

    def test_import_from_ai_services(self):
        config = {
            "steps": {"a": {"type": "LLMService", "model": "gpt-4o"}},
            "workflow": [],
            "plugins": {},
            "ai_services": {"svc1": {"provider": "openai", "model": "gpt-4o"}},
        }
        adp = self.adapter.import_from(config)
        assert adp.runtime.models is not None

    def test_import_from_workflow_edges(self):
        config = {
            "steps": {
                "a": {"type": "LLMService", "model": "gpt-4o"},
                "b": {"type": "LLMService", "model": "gpt-4o"},
            },
            "workflow": [{"from": "a", "to": "b"}],
            "plugins": {},
            "ai_services": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        edges = data["flow"]["graph"]["edges"]
        assert any(e["from"] == "a" and e["to"] == "b" for e in edges)

    def test_import_from_no_steps(self):
        config = {"steps": {}, "workflow": [], "plugins": {}, "ai_services": {}}
        adp = self.adapter.import_from(config)
        assert isinstance(adp, ADP)

    def test_roundtrip_fidelity(self):
        fidelity = self.adapter.roundtrip_fidelity()
        assert fidelity["runtime.models"] == "faithful"
        assert fidelity["tools"] == "faithful"


# ===========================================================================
# LangGraph adapter tests
# ===========================================================================

class TestLangGraphAdapter:
    def setup_method(self):
        import adp_sdk.adapters  # noqa: F401
        from adp_sdk.adapters.langgraph import LangGraphAdapter
        self.adapter = LangGraphAdapter()

    def test_export_llm_node(self):
        adp = _make_adp(nodes=[{"id": "chat", "kind": "llm", "model_ref": "gpt4"}])
        result = self.adapter.export(adp)
        assert result["nodes"]["chat"]["type"] == "ChatModel"
        assert result["nodes"]["chat"]["model"] == "gpt4"

    def test_export_tool_node(self):
        adp = _make_adp(nodes=[{"id": "fetch", "kind": "tool", "tool_ref": "my-api"}])
        result = self.adapter.export(adp)
        assert result["nodes"]["fetch"]["type"] == "ToolNode"
        assert result["nodes"]["fetch"]["tool"] == "my-api"

    def test_export_router_node(self):
        adp = _make_adp(nodes=[{"id": "route", "kind": "router", "strategy": "round_robin"}])
        result = self.adapter.export(adp)
        assert result["nodes"]["route"]["type"] == "Router"
        assert result["nodes"]["route"]["strategy"] == "round_robin"

    def test_export_retriever_node(self):
        adp = _make_adp(nodes=[{"id": "ret", "kind": "retriever", "memory_ref": "store1"}])
        result = self.adapter.export(adp)
        assert result["nodes"]["ret"]["type"] == "Retriever"
        assert result["nodes"]["ret"]["memory"] == "store1"

    def test_export_input_node(self):
        adp = _make_adp(nodes=[{"id": "in", "kind": "input"}])
        result = self.adapter.export(adp)
        assert result["nodes"]["in"]["type"] == "Start"

    def test_export_output_node(self):
        adp = _make_adp(nodes=[{"id": "out", "kind": "output"}])
        result = self.adapter.export(adp)
        assert result["nodes"]["out"]["type"] == "End"

    def test_export_unknown_kind(self):
        adp = _make_adp(nodes=[{"id": "x", "kind": "evaluator"}])
        result = self.adapter.export(adp)
        assert result["nodes"]["x"]["type"] == "Node"

    def test_export_node_with_params(self):
        adp = _make_adp(nodes=[{"id": "n", "kind": "llm", "params": {"temperature": 0.5}}])
        result = self.adapter.export(adp)
        assert "params" in result["nodes"]["n"]

    def test_export_edges(self):
        nodes = [{"id": "a", "kind": "llm"}, {"id": "b", "kind": "llm"}]
        edges = [{"from": "a", "to": "b"}]
        adp = _make_adp(nodes=nodes, edges=edges)
        result = self.adapter.export(adp)
        assert "b" in result["edges"]["a"]

    def test_export_adapter_hints(self):
        adp = _make_adp(adapter_hints={"langgraph": {"checkpointer": "sqlite", "memory_store": "redis", "recursion_limit": 25, "stream_mode": "values"}})
        result = self.adapter.export(adp)
        assert result["checkpointer"] == "sqlite"
        assert result["store"] == "redis"
        assert result["recursion_limit"] == 25
        assert result["stream_mode"] == "values"

    def test_import_from_nodes(self):
        config = {
            "nodes": {
                "chat": {"type": "ChatModel", "model": "gpt-4o"},
                "fetch": {"type": "ToolNode", "tool": "api"},
                "ret": {"type": "Retriever", "memory": "store1"},
                "start": {"type": "Start"},
                "end": {"type": "End"},
                "other": {"type": "UnknownType"},
            },
            "edges": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        kinds = {n["id"]: n["kind"] for n in nodes}
        assert kinds["chat"] == "llm"
        assert kinds["fetch"] == "tool"
        assert kinds["ret"] == "retriever"
        assert kinds["start"] == "input"
        assert kinds["end"] == "output"

    def test_import_from_edges(self):
        config = {
            "nodes": {"a": {"type": "ChatModel"}, "b": {"type": "ChatModel"}},
            "edges": {"a": ["b"]},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        edges = data["flow"]["graph"]["edges"]
        assert any(e["from"] == "a" and e["to"] == "b" for e in edges)

    def test_import_from_adapter_hints(self):
        config = {
            "nodes": {},
            "edges": {},
            "checkpointer": "sqlite",
            "store": "redis",
            "recursion_limit": 50,
            "stream_mode": "updates",
        }
        adp = self.adapter.import_from(config)
        assert adp.flow is not None

    def test_import_from_with_name_and_id(self):
        config = {
            "id": "my-graph",
            "name": "My Graph",
            "nodes": {},
            "edges": {},
        }
        adp = self.adapter.import_from(config)
        assert adp.id == "my-graph"
        assert adp.name == "My Graph"

    def test_roundtrip_fidelity(self):
        fidelity = self.adapter.roundtrip_fidelity()
        assert fidelity["flow.graph"] == "faithful"


# ===========================================================================
# AutoGen adapter tests
# ===========================================================================

class TestAutoGenAdapter:
    def setup_method(self):
        import adp_sdk.adapters  # noqa: F401
        from adp_sdk.adapters.autogen import AutoGenAdapter
        self.adapter = AutoGenAdapter()

    def test_export_llm_node(self):
        adp = _make_adp(nodes=[{"id": "chat", "kind": "llm", "model_ref": "gpt-4"}])
        result = self.adapter.export(adp)
        agents = result["agents"]
        assert any(a["type"] == "AssistantAgent" for a in agents)

    def test_export_tool_node(self):
        adp = _make_adp(nodes=[{"id": "fetch", "kind": "tool", "tool_ref": "billing-fn"}])
        result = self.adapter.export(adp)
        assert any(t["name"] == "fetch" for t in result["tools"])
        agents = result["agents"]
        assert any(a["type"] == "ToolAgent" for a in agents)

    def test_export_router_node(self):
        adp = _make_adp(nodes=[{"id": "decide", "kind": "router", "strategy": "round_robin"}])
        result = self.adapter.export(adp)
        agents = result["agents"]
        assert any(a["type"] == "RouterAgent" for a in agents)

    def test_export_loop_policy_max_turns(self):
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(
                id="f",
                graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[]),
                loop_policy={"default_max_iterations": 5},
            ),
        )
        result = self.adapter.export(adp)
        assert result.get("max_turns") == 5

    def test_export_adapter_hints(self):
        adp = _make_adp(adapter_hints={"autogen": {"human_input_mode": "NEVER", "max_turns": 10}})
        result = self.adapter.export(adp)
        assert result["human_input_mode"] == "NEVER"
        assert result["max_turns"] == 10

    def test_import_from_assistant_agent(self):
        config = {
            "agents": [{"type": "AssistantAgent", "name": "chat", "model": "gpt-4"}],
            "tools": [],
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "llm" for n in nodes)

    def test_import_from_tool_agent(self):
        config = {
            "agents": [{"type": "ToolAgent", "name": "fetch"}],
            "tools": [],
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "tool" for n in nodes)

    def test_import_from_other_agent_type(self):
        config = {
            "agents": [{"type": "SomeOther", "name": "x"}],
            "tools": [],
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "router" for n in nodes)

    def test_import_from_with_tools(self):
        config = {
            "agents": [{"type": "AssistantAgent", "name": "a", "model": "gpt-4"}],
            "tools": [{"name": "tool1", "function": "billing:fn"}],
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any("tool_tool1" in n.get("id", "") for n in nodes)

    def test_import_from_adapter_hints(self):
        config = {
            "agents": [],
            "tools": [],
            "human_input_mode": "ALWAYS",
            "max_turns": 20,
        }
        adp = self.adapter.import_from(config)
        assert adp.flow is not None

    def test_import_from_empty_agents(self):
        config = {"agents": [], "tools": []}
        adp = self.adapter.import_from(config)
        assert isinstance(adp, ADP)

    def test_roundtrip_fidelity(self):
        fidelity = self.adapter.roundtrip_fidelity()
        assert fidelity["loop.termination"] == "faithful"


# ===========================================================================
# CrewAI adapter tests
# ===========================================================================

class TestCrewAIAdapter:
    def setup_method(self):
        import adp_sdk.adapters  # noqa: F401
        from adp_sdk.adapters.crewai import CrewAIAdapter
        self.adapter = CrewAIAdapter()

    def test_export_llm_node(self):
        adp = _make_adp(nodes=[{"id": "chat", "kind": "llm", "model_ref": "gpt-4", "label": "Chatbot"}])
        result = self.adapter.export(adp)
        agents = result["agents"]
        assert any(a["role"] == "Chatbot" for a in agents)

    def test_export_tool_node_skipped(self):
        """Tool nodes are not exported as CrewAI agents (they become tools)."""
        adp = _make_adp(nodes=[{"id": "fetch", "kind": "tool"}])
        result = self.adapter.export(adp)
        # tool nodes produce no agents entry
        assert all(a.get("role") != "fetch" for a in result["agents"])

    def test_export_adapter_hints_process(self):
        adp = _make_adp(adapter_hints={"crewai": {"process": "hierarchical", "max_rpm": 30}})
        result = self.adapter.export(adp)
        assert result["process"] == "hierarchical"
        assert result["max_rpm"] == 30

    def test_import_from_agents(self):
        config = {
            "agents": [
                {"role": "Researcher", "llm": "gpt-4"},
                {"role": "Writer", "llm": "gpt-4"},
            ],
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert len(nodes) == 2
        assert all(n["kind"] == "llm" for n in nodes)

    def test_import_from_creates_sequential_edges(self):
        config = {
            "agents": [
                {"role": "A", "llm": "gpt-4"},
                {"role": "B", "llm": "gpt-4"},
                {"role": "C", "llm": "gpt-4"},
            ],
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        edges = data["flow"]["graph"]["edges"]
        assert len(edges) == 2

    def test_import_from_adapter_hints(self):
        config = {
            "agents": [{"role": "R", "llm": "gpt-4"}],
            "process": "sequential",
            "max_rpm": 10,
        }
        adp = self.adapter.import_from(config)
        assert adp.flow is not None

    def test_import_from_empty_agents(self):
        config = {"agents": []}
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert nodes == []

    def test_roundtrip_fidelity(self):
        fidelity = self.adapter.roundtrip_fidelity()
        assert fidelity["tools.policy"] == "faithful"


# ===========================================================================
# LlamaIndex adapter tests
# ===========================================================================

class TestLlamaIndexAdapter:
    def setup_method(self):
        import adp_sdk.adapters  # noqa: F401
        from adp_sdk.adapters.llamaindex import LlamaIndexAdapter
        self.adapter = LlamaIndexAdapter()

    def test_export_tools(self):
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            tools=ToolsModel(http_apis=[HTTPAPIModel(id="api1", description="API", base_url="https://api.example.com")]),
        )
        result = self.adapter.export(adp)
        assert any(t["name"] == "api1" for t in result["tools"])

    def test_export_semantic_memory(self):
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            memory={"stores": [{"id": "vs", "type": "semantic", "provider": "pinecone", "index": "my-index"}]},
        )
        result = self.adapter.export(adp)
        assert "vector_store" in result["memory"]

    def test_export_episodic_memory(self):
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            memory={"stores": [{"id": "ep", "type": "episodic", "provider": "redis"}]},
        )
        result = self.adapter.export(adp)
        assert "chat_memory" in result["memory"]

    def test_export_embedder_hint(self):
        adp = _make_adp(adapter_hints={"llamaindex": {"embedder_config": {"model": "text-embedding-3-small"}}})
        result = self.adapter.export(adp)
        assert "embedder" in result

    def test_import_from_tools(self):
        config = {
            "tools": [{"name": "t1", "description": "Tool 1", "base_url": ""}],
            "memory": {},
        }
        adp = self.adapter.import_from(config)
        assert adp.tools is not None

    def test_import_from_vector_store_memory(self):
        config = {
            "tools": [],
            "memory": {"vector_store": {"provider": "pinecone", "index": "idx"}},
        }
        adp = self.adapter.import_from(config)
        assert adp.memory is not None

    def test_import_from_chat_memory(self):
        config = {
            "tools": [],
            "memory": {"chat_memory": {"provider": "redis"}},
        }
        adp = self.adapter.import_from(config)
        assert adp.memory is not None

    def test_import_from_both_memories(self):
        config = {
            "tools": [],
            "memory": {
                "vector_store": {"provider": "pinecone", "index": "idx"},
                "chat_memory": {"provider": "redis"},
            },
        }
        adp = self.adapter.import_from(config)
        memory = adp.memory
        if isinstance(memory, dict):
            assert len(memory["stores"]) == 2

    def test_import_from_embedder(self):
        config = {
            "tools": [],
            "memory": {},
            "embedder": {"model": "text-embedding-3-small"},
        }
        adp = self.adapter.import_from(config)
        assert adp.flow is not None

    def test_import_from_no_tools_no_memory(self):
        config = {"tools": [], "memory": {}}
        adp = self.adapter.import_from(config)
        assert isinstance(adp, ADP)

    def test_roundtrip_fidelity(self):
        fidelity = self.adapter.roundtrip_fidelity()
        assert fidelity["memory.stores"] == "faithful"


# ===========================================================================
# GoogleADK adapter tests
# ===========================================================================

class TestGoogleADKAdapter:
    def setup_method(self):
        import adp_sdk.adapters  # noqa: F401
        from adp_sdk.adapters.google_adk import GoogleADKAdapter
        self.adapter = GoogleADKAdapter()

    def test_export_llm_node(self):
        adp = _make_adp(nodes=[{"id": "agent1", "kind": "llm", "model_ref": "gemini-1.5-pro"}])
        result = self.adapter.export(adp)
        agents = result["agents"]
        assert any(a["name"] == "agent1" and a["type"] == "LLMAgent" for a in agents)

    def test_export_tool_node(self):
        adp = _make_adp(nodes=[{"id": "fetch", "kind": "tool", "tool_ref": "search"}])
        result = self.adapter.export(adp)
        agents = result["agents"]
        assert any(a["type"] == "ToolAgent" for a in agents)

    def test_export_router_node(self):
        adp = _make_adp(nodes=[{"id": "route", "kind": "router", "strategy": "round_robin"}])
        result = self.adapter.export(adp)
        agents = result["agents"]
        assert any(a["type"] == "RouterAgent" for a in agents)

    def test_export_artifacts(self):
        from adp_sdk.adp_model import ArtifactsModel, ArtifactStore
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            artifacts=ArtifactsModel(stores=[ArtifactStore(id="s1", scope="session", provider="gcs", bucket="my-bucket")]),
        )
        result = self.adapter.export(adp)
        assert len(result["artifacts"]) == 1

    def test_export_session_memory(self):
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            memory={"stores": [{"id": "sess", "type": "episodic", "provider": "redis", "scope": "session", "endpoint": "redis://localhost"}]},
        )
        result = self.adapter.export(adp)
        assert result["session_service"].get("provider") == "redis"

    def test_export_adapter_hints(self):
        adp = _make_adp(adapter_hints={"google_adk": {"memory_store": "firestore"}})
        result = self.adapter.export(adp)
        assert result["memory_store"] == "firestore"

    def test_export_tools_mcp_and_http(self):
        """Export with mcp_servers and http_apis populates adk_tools list (covers line 60)."""
        from adp_sdk.adp_model import ToolsModel, MCPServerModel, HTTPAPIModel
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            tools=ToolsModel(
                mcp_servers=[MCPServerModel(
                    id="mcp-search",
                    description="MCP search tool",
                    transport="http",
                    endpoint="https://mcp.example/mcp",
                )],
                http_apis=[HTTPAPIModel(id="billing-api", description="Billing API", base_url="https://billing.example")],
            ),
        )
        result = self.adapter.export(adp)
        tool_names = [t["name"] for t in result.get("tools", [])]
        assert "mcp-search" in tool_names
        assert "billing-api" in tool_names

    def test_import_from_llm_agent(self):
        config = {
            "agents": [{"type": "LLMAgent", "name": "chat", "model": "gemini-1.5-pro"}],
            "tools": [],
            "artifacts": [],
            "session_service": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "llm" for n in nodes)

    def test_import_from_tool_agent(self):
        config = {
            "agents": [{"type": "ToolAgent", "name": "fetch", "tool": "search"}],
            "tools": [],
            "artifacts": [],
            "session_service": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "tool" for n in nodes)

    def test_import_from_router_agent(self):
        config = {
            "agents": [{"type": "RouterAgent", "name": "decide", "strategy": "round_robin"}],
            "tools": [],
            "artifacts": [],
            "session_service": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "router" for n in nodes)

    def test_import_from_with_tools(self):
        config = {
            "agents": [],
            "tools": [{"name": "t1", "description": "Tool", "function": "t1"}],
            "artifacts": [],
            "session_service": {},
        }
        adp = self.adapter.import_from(config)
        assert adp.tools is not None

    def test_import_from_with_artifacts(self):
        config = {
            "agents": [],
            "tools": [],
            "artifacts": [{"id": "s1", "provider": "gcs", "bucket": "b", "scope": "session"}],
            "session_service": {},
        }
        adp = self.adapter.import_from(config)
        assert adp.artifacts is not None

    def test_import_from_with_session_service(self):
        config = {
            "agents": [],
            "tools": [],
            "artifacts": [],
            "session_service": {"provider": "redis", "endpoint": "redis://localhost"},
        }
        adp = self.adapter.import_from(config)
        assert adp.memory is not None

    def test_import_from_memory_store_hint(self):
        config = {
            "agents": [],
            "tools": [],
            "artifacts": [],
            "session_service": {},
            "memory_store": "firestore",
        }
        adp = self.adapter.import_from(config)
        assert adp.flow is not None

    def test_roundtrip_fidelity(self):
        fidelity = self.adapter.roundtrip_fidelity()
        assert fidelity["artifacts"] == "faithful"


# ===========================================================================
# OpenAI Agents adapter tests
# ===========================================================================

class TestOpenAIAgentsAdapter:
    def setup_method(self):
        import adp_sdk.adapters  # noqa: F401
        from adp_sdk.adapters.openai_agents import OpenAIAgentsAdapter
        self.adapter = OpenAIAgentsAdapter()

    def test_export_llm_node(self):
        adp = _make_adp(nodes=[{"id": "chat", "kind": "llm", "model_ref": "gpt-4o"}])
        result = self.adapter.export(adp)
        assert any(a["name"] == "chat" for a in result["agents"])

    def test_export_tool_node(self):
        adp = _make_adp(nodes=[{"id": "fetch", "kind": "tool", "tool_ref": "search"}])
        result = self.adapter.export(adp)
        assert any(a["name"] == "fetch" for a in result["agents"])

    def test_export_edges_as_handoffs(self):
        nodes = [{"id": "a", "kind": "llm"}, {"id": "b", "kind": "llm"}]
        edges = [{"from": "a", "to": "b"}]
        adp = _make_adp(nodes=nodes, edges=edges)
        result = self.adapter.export(adp)
        assert any(h["from"] == "a" and h["to"] == "b" for h in result["handoffs"])

    def test_export_tools(self):
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            tools=ToolsModel(http_apis=[HTTPAPIModel(id="api1", description="API", base_url="https://api.example.com")]),
        )
        result = self.adapter.export(adp)
        assert any(t["name"] == "api1" for t in result["tools"])

    def test_export_guardrails_cost_threshold(self):
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            guardrails={"interrupts": [{"id": "cost-guard", "trigger": "cost_threshold", "mode": "block"}]},
        )
        result = self.adapter.export(adp)
        assert "cost_limit" in result["guardrails"]

    def test_export_observability_tracing(self):
        adp = ADP(
            adp_version="0.3.0",
            id="test",
            runtime=RuntimeModel(execution=[RuntimeEntry(backend="python", id="py", entrypoint="app:main")]),
            flow=FlowModel(id="f", graph=GraphModel(nodes=[{"id": "n", "kind": "llm"}], edges=[], start_nodes=["n"], end_nodes=[])),
            observability={"tracing": {"backend": "langfuse", "trace_events": ["model_request"]}},
        )
        result = self.adapter.export(adp)
        assert result["observability"]["tracing"]["backend"] == "langfuse"

    def test_import_from_agents(self):
        config = {
            "agents": [{"name": "chat", "model": "gpt-4o", "tools": []}],
            "handoffs": [],
            "tools": [],
            "observability": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any(n["kind"] == "llm" for n in nodes)

    def test_import_from_agent_with_tools(self):
        config = {
            "agents": [{"name": "agent1", "model": "gpt-4o", "tools": ["search"]}],
            "handoffs": [],
            "tools": [],
            "observability": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        nodes = data["flow"]["graph"]["nodes"]
        assert any("tool" in n.get("id", "") for n in nodes)

    def test_import_from_handoffs(self):
        config = {
            "agents": [{"name": "a", "model": "gpt-4o", "tools": []}, {"name": "b", "model": "gpt-4o", "tools": []}],
            "handoffs": [{"from": "a", "to": "b"}],
            "tools": [],
            "observability": {},
        }
        adp = self.adapter.import_from(config)
        data = adp.model_dump(by_alias=True, exclude_none=True)
        edges = data["flow"]["graph"]["edges"]
        assert any(e["from"] == "a" and e["to"] == "b" for e in edges)

    def test_import_from_with_tools(self):
        config = {
            "agents": [],
            "handoffs": [],
            "tools": [{"name": "t1", "description": "Tool 1"}],
            "observability": {},
        }
        adp = self.adapter.import_from(config)
        assert adp.tools is not None

    def test_import_from_with_observability(self):
        config = {
            "agents": [],
            "handoffs": [],
            "tools": [],
            "observability": {"tracing": {"backend": "langfuse", "events": ["model_request"]}},
        }
        adp = self.adapter.import_from(config)
        assert adp.observability is not None

    def test_import_from_empty(self):
        config = {"agents": [], "handoffs": [], "tools": [], "observability": {}}
        adp = self.adapter.import_from(config)
        assert isinstance(adp, ADP)

    def test_roundtrip_fidelity(self):
        fidelity = self.adapter.roundtrip_fidelity()
        assert fidelity["guardrails.interrupts"] == "faithful"


# ===========================================================================
# Adapters __init__.py — ensures all adapters are imported and registered
# ===========================================================================

def test_adapters_init_registers_all():
    """Importing adp_sdk.adapters registers all framework adapters."""
    import adp_sdk.adapters  # noqa: F401
    from adp_sdk.adapters.registry import AdapterRegistry
    frameworks = AdapterRegistry.available()
    expected = {"langgraph", "autogen", "crewai", "llamaindex", "google_adk",
                "openai_agents", "pydantic_ai", "semantic_kernel"}
    assert expected.issubset(set(frameworks))
