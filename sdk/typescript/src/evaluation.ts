import { execFileSync } from "child_process";

export interface EvaluationResult {
  passed: boolean;
  score: number | null;
  reason: string;
  metadata: Record<string, unknown>;
  evaluatorId: string;
  evaluatorType: string;
}

export interface Evaluator {
  evaluate(
    output: Record<string, unknown>,
    context: Record<string, unknown>
  ): Promise<EvaluationResult>;
}

export class UnsupportedEvaluatorTypeError extends Error {
  constructor(type: string) {
    super(`Unsupported evaluator type: '${type}'`);
    this.name = "UnsupportedEvaluatorTypeError";
  }
}

class ScriptEvaluator implements Evaluator {
  constructor(private config: Record<string, unknown>) {}

  async evaluate(
    output: Record<string, unknown>,
    context: Record<string, unknown>
  ): Promise<EvaluationResult> {
    const runtime = this.config.runtime as string;
    const inline = this.config.inline as string | undefined;
    const scriptRef = this.config.script_ref as string | undefined;
    const id = (this.config.id as string) ?? "";

    if (!inline && !scriptRef) {
      throw new Error(`ScriptEvaluator '${id}': requires inline or script_ref`);
    }

    const input = JSON.stringify({ output, context });

    if (runtime === "javascript") {
      const code = inline ?? "";
      const vm = await import("vm");
      const sandbox: Record<string, unknown> = {};
      vm.createContext(sandbox);
      const fn = vm.runInContext(`(function evaluate(output, context) { ${code} })`, sandbox);
      const raw = fn(output, context);
      return normalizeResult(raw, id, "script");
    }

    if (runtime === "python") {
      const scriptCode = inline
        ? `import sys, json\noutput = json.loads(sys.stdin.read())["output"]\ncontext = json.loads(sys.stdin.read())["context"]\n${inline}\nresult = evaluate(output, context)\nprint(json.dumps(result))`
        : undefined;
      /* c8 ignore next */
      const args = scriptRef ? [scriptRef] : ["-c", scriptCode ?? ""];
      /* c8 ignore next */
      const cmd = runtime === "python" ? "python3" : "/bin/bash";
      try {
        const stdout = execFileSync(cmd, args, { input, encoding: "utf8", timeout: 30000 });
        const raw = JSON.parse(stdout.trim());
        return normalizeResult(raw, id, "script");
      } catch (err: any) {
        return {
          passed: false,
          score: null,
          reason: `Script error: ${err.message}`,
          metadata: {},
          evaluatorId: id,
          evaluatorType: "script",
        };
      }
    }

    if (runtime === "bash") {
      try {
        const stdout = execFileSync("/bin/bash", ["-c", inline ?? ""], {
          input,
          encoding: "utf8",
          timeout: 30000,
        });
        const raw = JSON.parse(stdout.trim());
        return normalizeResult(raw, id, "script");
      } catch (err: any) {
        return {
          passed: false,
          score: null,
          reason: `Bash error: ${err.message}`,
          metadata: {},
          evaluatorId: id,
          evaluatorType: "script",
        };
      }
    }

    throw new UnsupportedEvaluatorTypeError(`script runtime '${runtime}'`);
  }
}

class DeterministicEvaluator implements Evaluator {
  constructor(private config: Record<string, unknown>) {}

  async evaluate(
    output: Record<string, unknown>,
    _context: Record<string, unknown>
  ): Promise<EvaluationResult> {
    const functionRef = this.config.function_ref as string;
    const id = (this.config.id as string) ?? "";
    const [modulePath, funcName] = functionRef.split(":");
    if (!modulePath || !funcName) {
      throw new Error(`DeterministicEvaluator: invalid function_ref '${functionRef}' — expected 'module:function'`);
    }
    const mod = await import(modulePath);
    /* c8 ignore next */
    const fn = mod[funcName] ?? mod.default?.[funcName];
    if (typeof fn !== "function") {
      throw new Error(`DeterministicEvaluator: '${funcName}' not found in '${modulePath}'`);
    }
    const raw = fn(output);
    return normalizeResult(raw, id, "deterministic");
  }
}

class LLMJudgeEvaluator implements Evaluator {
  constructor(private config: Record<string, unknown>) {}

  async evaluate(
    _output: Record<string, unknown>,
    _context: Record<string, unknown>
  ): Promise<EvaluationResult> {
    throw new Error(
      "LLMJudgeEvaluator requires an LLM client. Inject one via config.llm_client or use a framework-specific adapter."
    );
  }
}

class ContainerEvaluator implements Evaluator {
  constructor(private config: Record<string, unknown>) {}

  /* c8 ignore start */
  async evaluate(
    output: Record<string, unknown>,
    context: Record<string, unknown>
  ): Promise<EvaluationResult> {
    const image = this.config.image as string;
    const imageDigest = this.config.image_digest as string;
    const id = (this.config.id as string) ?? "";
    const timeoutSeconds = (this.config.timeout_seconds as number) ?? 30;
    const command = (this.config.command as string[]) ?? [];
    const env = (this.config.env as Record<string, string>) ?? {};

    const envArgs: string[] = [];
    for (const [k, v] of Object.entries(env)) {
      envArgs.push("-e", `${k}=${v}`);
    }

    const imageRef = imageDigest ? `${image}@${imageDigest}` : image;
    const dockerArgs = ["run", "--rm", "-i", ...envArgs, ...(command.length ? [imageRef, ...command] : [imageRef])];
    const input = JSON.stringify({ output, context });

    try {
      const stdout = execFileSync("docker", dockerArgs, {
        input,
        encoding: "utf8",
        timeout: timeoutSeconds * 1000,
      });
      const raw = JSON.parse(stdout.trim());
      return normalizeResult(raw, id, "container");
    } catch (err: any) {
      const exitCode = err.status ?? "unknown";
      return {
        passed: false,
        score: null,
        reason: `container exited with code ${exitCode}: ${err.message}`,
        metadata: {},
        evaluatorId: id,
        evaluatorType: "container",
      };
    }
  }
  /* c8 ignore stop */
}

function normalizeResult(
  raw: unknown,
  id: string,
  type: string
): EvaluationResult {
  if (typeof raw === "boolean") {
    return { passed: raw, score: raw ? 1.0 : 0.0, reason: "", metadata: {}, evaluatorId: id, evaluatorType: type };
  }
  if (typeof raw === "object" && raw !== null) {
    const r = raw as Record<string, unknown>;
    return {
      passed: Boolean(r.passed ?? (typeof r.score === "number" ? r.score >= 0.5 : false)),
      score: typeof r.score === "number" ? r.score : null,
      reason: (r.reason as string) ?? "",
      metadata: (r.metadata as Record<string, unknown>) ?? {},
      evaluatorId: id,
      evaluatorType: type,
    };
  }
  return { passed: Boolean(raw), score: null, reason: "", metadata: {}, evaluatorId: id, evaluatorType: type };
}

export function loadEvaluator(config: Record<string, unknown>): Evaluator {
  const type = config.type as string;
  switch (type) {
    case "llm_judge":
      return new LLMJudgeEvaluator(config);
    case "script":
      return new ScriptEvaluator(config);
    case "container":
      return new ContainerEvaluator(config);
    case "deterministic":
      return new DeterministicEvaluator(config);
    default:
      throw new UnsupportedEvaluatorTypeError(type ?? "(missing)");
  }
}

export function loadEvaluatorsFromManifest(
  manifest: Record<string, unknown>
): Map<string, Evaluator> {
  const xTesting = (manifest.x_testing ?? {}) as Record<string, unknown>;
  const evaluatorsArr = (xTesting.evaluators ?? []) as Record<string, unknown>[];
  const judgesArr = (xTesting.judges ?? []) as Record<string, unknown>[];

  const result = new Map<string, Evaluator>();

  // Load judges[] first (lower priority on collision)
  for (const judge of judgesArr) {
    const id = judge.id as string;
    if (id && !result.has(id)) {
      result.set(id, loadEvaluator({ ...judge, type: judge.type ?? "llm_judge" }));
    }
  }

  // evaluators[] wins on id collision
  for (const ev of evaluatorsArr) {
    const id = ev.id as string;
    if (id) {
      result.set(id, loadEvaluator(ev));
    }
  }

  return result;
}
