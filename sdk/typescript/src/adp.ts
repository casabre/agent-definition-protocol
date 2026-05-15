import fs from "fs";
import yaml from "js-yaml";
import { validateAdp } from "./validation";

export interface Runtime {
  execution: Array<{ id: string; backend: string; [key: string]: unknown }>;
  models?: Array<{ id: string; provider: string; model: string; [key: string]: unknown }>;
}

export interface FlowNode {
  id: string;
  kind: "input" | "output" | "llm" | "tool" | "router" | "retriever" | "evaluator" | "subflow";
  [key: string]: unknown;
}

export interface Flow {
  id: string;
  graph: {
    nodes: FlowNode[];
    edges: Array<{ from: string; to: string; condition?: string }>;
    start_nodes: string[];
    end_nodes: string[];
  };
}

export interface EvaluationMetric {
  id: string;
  type: string;
  function: string;
  scoring: string;
  threshold: unknown;
}

export interface EvaluationSuite {
  id: string;
  metrics: EvaluationMetric[];
}

export interface Evaluation {
  suites: EvaluationSuite[];
  promotion_policy?: { require_passing_suites?: string[] };
}

export interface ADP {
  adp_version: string;
  id: string;
  runtime: Runtime;
  flow: Flow;
  evaluation: Evaluation;
  [key: string]: unknown;
}

export function parseADP(path: string): ADP {
  const data = yaml.load(fs.readFileSync(path, "utf8")) as ADP;
  return data;
}

export function validateADPFile(path: string): string[] {
  const adp = parseADP(path);
  return validateAdp(adp);
}
