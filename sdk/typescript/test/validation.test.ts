import { validateAdp } from "../src/validation.ts";

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
});
