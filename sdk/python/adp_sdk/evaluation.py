from __future__ import annotations

import asyncio
import importlib
import subprocess
import json
import os
import re
import shutil
import tempfile
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from pathlib import Path



@dataclass
class EvaluationResult:
    passed: bool
    score: float | None = None
    reason: str = ""
    metadata: dict = field(default_factory=dict)
    evaluator_id: str = ""
    evaluator_type: str = ""


class BaseEvaluator(ABC):
    def __init__(self, config: dict) -> None:
        self._config = config
        self._id = config.get("id", "")
        self._type = config.get("type", "")

    @abstractmethod
    async def evaluate(self, output: dict, context: dict) -> EvaluationResult:
        ...

    def evaluate_sync(self, output: dict, context: dict) -> EvaluationResult:
        return asyncio.run(self.evaluate(output, context))


class DeterministicEvaluator(BaseEvaluator):
    async def evaluate(self, output: dict, context: dict) -> EvaluationResult:
        function_ref = self._config["function_ref"]
        module_name, fn_name = function_ref.rsplit(":", 1)
        mod = importlib.import_module(module_name)
        fn = getattr(mod, fn_name)
        result = fn(output, context)
        if isinstance(result, bool):
            return EvaluationResult(passed=result, evaluator_id=self._id, evaluator_type=self._type)
        if isinstance(result, dict):
            return EvaluationResult(
                passed=bool(result.get("passed", False)),
                score=result.get("score"),
                reason=result.get("reason", ""),
                metadata={k: v for k, v in result.items() if k not in ("passed", "score", "reason")},
                evaluator_id=self._id,
                evaluator_type=self._type,
            )
        return EvaluationResult(passed=bool(result), evaluator_id=self._id, evaluator_type=self._type)


class ScriptEvaluator(BaseEvaluator):
    async def evaluate(self, output: dict, context: dict) -> EvaluationResult:
        runtime = self._config["runtime"]
        if "inline" in self._config:
            script_code = self._config["inline"]
            with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
                f.write(script_code)
                script_path = f.name
        else:
            script_path = str(_resolve_script_ref(self._config["script_ref"]))

        try:
            result = _run_script(runtime, script_path, output, context)
        finally:
            if "inline" in self._config:
                Path(script_path).unlink(missing_ok=True)

        return EvaluationResult(
            passed=bool(result.get("passed", False)),
            score=result.get("score"),
            reason=result.get("reason", ""),
            metadata={k: v for k, v in result.items() if k not in ("passed", "score", "reason")},
            evaluator_id=self._id,
            evaluator_type=self._type,
        )


def _run_script(runtime: str, script_path: str, output: dict, context: dict) -> dict:
    payload = json.dumps({"output": output, "context": context})
    if runtime == "python":
        wrapper = (
            "import json, sys\n"
            "_payload = json.loads(sys.stdin.read())\n"
            "output = _payload['output']\n"
            "context = _payload['context']\n"
        )
        with open(script_path) as f:
            script_code = f.read()
        full_code = wrapper + script_code + "\n_result = evaluate(output, context)\nprint(json.dumps(_result))\n"
        proc = subprocess.run(
            ["python3", "-c", full_code],
            input=payload,
            capture_output=True,
            text=True,
            timeout=30,
        )
    elif runtime == "bash":
        proc = subprocess.run(
            ["bash", script_path],
            input=payload,
            capture_output=True,
            text=True,
            timeout=30,
        )
    else:
        raise ValueError(f"Unsupported script runtime: {runtime}")

    if proc.returncode != 0:
        return {"passed": False, "reason": f"script exited {proc.returncode}: {proc.stderr[:200]}"}
    try:
        return json.loads(proc.stdout.strip())
    except json.JSONDecodeError:
        return {"passed": False, "reason": f"script output is not valid JSON: {proc.stdout[:200]}"}


class ContainerEvaluator(BaseEvaluator):
    async def evaluate(self, output: dict, context: dict) -> EvaluationResult:
        image = self._config["image"]
        image_digest = self._config["image_digest"]
        image_ref = f"{image}@{image_digest}"
        command = self._config.get("command", [])
        env = self._config.get("env", {})
        input_format = self._config.get("input_format", "json_stdin")
        output_format = self._config.get("output_format", "json_stdout")
        timeout = self._config.get("timeout_seconds", 30)

        payload = json.dumps({"output": output, "context": context})

        docker_cmd = ["docker", "run", "--rm"]
        for k, v in env.items():
            docker_cmd += ["-e", f"{k}={v}"]

        if input_format == "file" or output_format == "file":
            in_fd, in_path = tempfile.mkstemp(suffix=".json")
            out_fd, out_path = tempfile.mkstemp(suffix=".json")
            os.close(in_fd)
            os.close(out_fd)
            try:
                Path(in_path).write_text(payload)
                docker_cmd += [
                    "-v", f"{in_path}:/adp_input.json",
                    "-v", f"{out_path}:/adp_output.json",
                    "-e", "ADP_INPUT_FILE=/adp_input.json",
                    "-e", "ADP_OUTPUT_FILE=/adp_output.json",
                ]
                docker_cmd.append(image_ref)
                if command:
                    docker_cmd.extend(command)
                proc = subprocess.run(docker_cmd, capture_output=True, text=True, timeout=timeout)
                if proc.returncode != 0:
                    return EvaluationResult(
                        passed=False, reason=f"container exited with code {proc.returncode}",
                        evaluator_id=self._id, evaluator_type=self._type,
                    )
                result = json.loads(Path(out_path).read_text())
            finally:
                Path(in_path).unlink(missing_ok=True)
                Path(out_path).unlink(missing_ok=True)
        else:
            docker_cmd.append(image_ref)
            if command:
                docker_cmd.extend(command)
            proc = subprocess.run(
                docker_cmd, input=payload, capture_output=True, text=True, timeout=timeout
            )
            if proc.returncode != 0:
                return EvaluationResult(
                    passed=False, reason=f"container exited with code {proc.returncode}",
                    evaluator_id=self._id, evaluator_type=self._type,
                )
            result = json.loads(proc.stdout.strip())

        return EvaluationResult(
            passed=bool(result.get("passed", False)),
            score=result.get("score"),
            reason=result.get("reason", ""),
            metadata={k: v for k, v in result.items() if k not in ("passed", "score", "reason")},
            evaluator_id=self._id,
            evaluator_type=self._type,
        )


class LLMJudgeEvaluator(BaseEvaluator):
    async def evaluate(self, output: dict, context: dict) -> EvaluationResult:
        raise NotImplementedError(
            "LLMJudgeEvaluator requires an LLM client. "
            "Instantiate with load_evaluator(config, llm_client=...) to provide one."
        )


class ScriptRefVerificationError(Exception):
    pass


_GIT_REF_RE = re.compile(r"^git\+https://(.+?)(?:\.git)?/(.+?)@([a-f0-9]{7,40})$")
_CACHE_DIR = Path.home() / ".adp_sdk" / "eval_cache"


def _resolve_script_ref(ref: str) -> Path:
    m = _GIT_REF_RE.match(ref)
    if not m:
        return Path(ref)

    repo_host_path, file_path, sha = m.group(1), m.group(2), m.group(3)
    repo_url = f"https://{repo_host_path}"
    cache_dir = _CACHE_DIR / sha
    sentinel = cache_dir / "_verified"

    if sentinel.exists():
        return cache_dir / "repo" / file_path

    cache_dir.mkdir(parents=True, exist_ok=True)
    repo_dir = cache_dir / "repo"
    subprocess.run(
        ["git", "clone", "--depth", "1", "--no-checkout", repo_url, str(repo_dir)],
        check=True, capture_output=True,
    )
    subprocess.run(
        ["git", "-C", str(repo_dir), "checkout", sha, "--", file_path],
        check=True, capture_output=True,
    )

    result = subprocess.run(
        ["git", "-C", str(repo_dir), "rev-parse", "HEAD"],
        capture_output=True, text=True,
    )
    actual_sha = result.stdout.strip()
    if not actual_sha.startswith(sha):
        shutil.rmtree(str(cache_dir), ignore_errors=True)
        raise ScriptRefVerificationError(
            f"SHA verification failed for ref '{ref}': "
            f"expected '{sha}', got '{actual_sha}'"
        )

    sentinel.write_text("ok")
    return repo_dir / file_path


_EVALUATOR_TYPES = {
    "deterministic": DeterministicEvaluator,
    "script": ScriptEvaluator,
    "container": ContainerEvaluator,
    "llm_judge": LLMJudgeEvaluator,
}


def load_evaluator(config: dict) -> BaseEvaluator:
    ev_type = config.get("type", "")
    cls = _EVALUATOR_TYPES.get(ev_type)
    if cls is None:
        raise ValueError(f"Unknown evaluator type: '{ev_type}'. Supported: {list(_EVALUATOR_TYPES)}")
    return cls(config)


def load_evaluators_from_manifest(manifest: dict) -> dict[str, BaseEvaluator]:
    x_testing = manifest.get("x_testing", {})
    evaluators: dict[str, BaseEvaluator] = {}

    # Load deprecated judges[] first (lower priority)
    judges = x_testing.get("judges", [])
    if judges and not x_testing.get("evaluators"):
        import warnings
        warnings.warn(
            "x_testing.judges[] is deprecated; migrate to x_testing.evaluators[]",
            DeprecationWarning,
            stacklevel=2,
        )
    for judge in judges:
        jid = judge.get("id")
        if jid:
            # Coerce old-style judge to evaluator config
            ev_config = {**judge, "type": "llm_judge"}
            evaluators[jid] = load_evaluator(ev_config)

    # Load evaluators[] (higher priority — wins on id collision)
    for ev in x_testing.get("evaluators", []):
        eid = ev.get("id")
        if eid:
            if eid in evaluators:
                import logging
                logging.debug("evaluators[] id '%s' overrides judges[] entry with same id", eid)
            evaluators[eid] = load_evaluator(ev)

    return evaluators
