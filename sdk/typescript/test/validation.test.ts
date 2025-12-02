import { validateAdp } from "../src/validation.js";

const valid = {
  adp_version: "0.1.0",
  id: "agent.test",
  runtime: { execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }] },
  flow: {},
  evaluation: {},
};

test("validateAdp passes on valid manifest", () => {
  const errors = validateAdp(valid);
  expect(errors.length).toBe(0);
});

test("validateAdp fails on missing runtime", () => {
  const errors = validateAdp({ ...valid, runtime: {} } as any);
  expect(errors.length).toBeGreaterThan(0);
  expect(errors.some((e: string) => e.toLowerCase().includes("runtime") || e.toLowerCase().includes("execution"))).toBe(true);
});

test("validateAdp fails on missing id", () => {
  const errors = validateAdp({ ...valid, id: "" } as any);
  expect(errors.length).toBeGreaterThan(0);
  expect(errors.some((e: string) => e.toLowerCase().includes("id"))).toBe(true);
});

test("validateAdp fails on invalid adp_version", () => {
  const errors = validateAdp({ ...valid, adp_version: "0.3.0" } as any);
  expect(errors.length).toBeGreaterThan(0);
  expect(errors.some((e: string) => e.includes("0.1.0") || e.includes("0.2.0") || e.toLowerCase().includes("version") || e.toLowerCase().includes("enum"))).toBe(true);
});

test("validateAdp passes on v0.2.0", () => {
  const v0_2_0 = {
    adp_version: "0.2.0",
    id: "agent.v0.2.0",
    runtime: {
      execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }],
      models: [
        {
          id: "primary",
          provider: "openai",
          model: "gpt-4",
          api_key_env: "OPENAI_API_KEY"
        }
      ]
    },
    flow: {
      id: "test.flow",
      graph: {
        nodes: [
          { id: "input", kind: "input" },
          { id: "llm", kind: "llm", model_ref: "primary" },
          { id: "tool", kind: "tool", tool_ref: "api" },
          { id: "output", kind: "output" }
        ],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["output"]
      }
    },
    evaluation: {}
  };
  const errors = validateAdp(v0_2_0);
  expect(errors.length).toBe(0);
});

test("validateAdp fails on empty execution array", () => {
  const errors = validateAdp({ ...valid, runtime: { execution: [] } } as any);
  expect(errors.length).toBeGreaterThan(0);
});

test("validateAdp passes with multiple backends", () => {
  const multiBackend = {
    ...valid,
    runtime: {
      execution: [
        { backend: "docker", id: "docker", image: "acme/agent:1.0" },
        { backend: "python", id: "python", entrypoint: "main:app" },
        { backend: "wasm", id: "wasm", module: "agent.wasm" },
      ],
    },
  };
  const errors = validateAdp(multiBackend);
  expect(errors.length).toBe(0);
});

test("validateAdp passes with optional fields", () => {
  const full = {
    ...valid,
    name: "Test Agent",
    description: "A test agent",
    owner: "test-org",
    tags: ["test", "example"],
  };
  const errors = validateAdp(full);
  expect(errors.length).toBe(0);
  expect(full.name).toBe("Test Agent");
  expect(full.description).toBe("A test agent");
});

test("validateAdp handles different backend types", () => {
  const backends = ["docker", "wasm", "python", "typescript", "binary", "custom"];
  backends.forEach((backend) => {
    const adp = {
      ...valid,
      runtime: { execution: [{ backend, id: `${backend}-id` }] },
    };
    const errors = validateAdp(adp);
    // Should not crash, may or may not validate backend type
    expect(Array.isArray(errors)).toBe(true);
  });
});

test("validateAdp validates flow structure", () => {
  const withFlow = {
    ...valid,
    flow: {
      id: "test.flow",
      graph: {
        nodes: [{ id: "input", kind: "input" }],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["input"],
      },
    },
  };
  const errors = validateAdp(withFlow);
  expect(errors.length).toBe(0);
});

test("validateAdp validates evaluation structure", () => {
  const withEval = {
    ...valid,
    evaluation: {
      suites: [
        {
          id: "basic",
          metrics: [
            {
              id: "m1",
              type: "deterministic",
              function: "noop",
              scoring: "boolean",
              threshold: true,
            },
          ],
        },
      ],
    },
  };
  const errors = validateAdp(withEval);
  expect(errors.length).toBe(0);
});

test("validateAdp handles validation failure", () => {
  // Test the branch where validation fails (line 26: if (ok) return [])
  // This covers the else branch when ok is false
  const invalid = { ...valid, adp_version: "invalid" };
  const errors = validateAdp(invalid);
  // Should return array of errors when validation fails
  expect(Array.isArray(errors)).toBe(true);
  expect(errors.length).toBeGreaterThan(0);
  // Verify error messages are strings
  errors.forEach(err => {
    expect(typeof err).toBe("string");
  });
});

test("validateAdp returns empty array on success", () => {
  // Test the branch where validation succeeds (line 27: return [])
  const errors = validateAdp(valid);
  expect(Array.isArray(errors)).toBe(true);
  expect(errors.length).toBe(0);
});
