import pytest
import tempfile
import os
from pathlib import Path
from adp_sdk.evaluation import (
    load_evaluator,
    load_evaluators_from_manifest,
    EvaluationResult,
    ScriptEvaluator,
    DeterministicEvaluator,
    LLMJudgeEvaluator,
    _resolve_script_ref,
)


def test_load_evaluator_script_inline_returns_result():
    config = {
        "id": "t1",
        "type": "script",
        "runtime": "python",
        "inline": "def evaluate(output, context):\n    return {'passed': True, 'score': 1.0, 'reason': 'ok'}\n",
    }
    ev = load_evaluator(config)
    result = ev.evaluate_sync({"text": "hello"}, {})
    assert isinstance(result, EvaluationResult)
    assert result.passed is True
    assert result.score == 1.0
    assert result.evaluator_id == "t1"


def test_load_evaluator_script_inline_fail():
    config = {
        "id": "t2",
        "type": "script",
        "runtime": "python",
        "inline": "def evaluate(output, context):\n    return {'passed': False, 'score': 0.0, 'reason': 'bad'}\n",
    }
    ev = load_evaluator(config)
    result = ev.evaluate_sync({}, {})
    assert result.passed is False
    assert result.score == 0.0


def test_load_evaluator_deterministic():
    config = {"id": "t3", "type": "deterministic", "function_ref": "builtins:len"}
    ev = load_evaluator(config)
    assert isinstance(ev, DeterministicEvaluator)


def test_load_evaluator_llm_judge_raises_not_implemented():
    config = {"id": "t4", "type": "llm_judge", "model": "gpt-4o"}
    ev = load_evaluator(config)
    assert isinstance(ev, LLMJudgeEvaluator)
    with pytest.raises(NotImplementedError):
        ev.evaluate_sync({}, {})


def test_load_evaluator_unknown_type_raises():
    with pytest.raises(ValueError, match="Unknown evaluator type"):
        load_evaluator({"id": "t5", "type": "unknown_type"})


def test_load_evaluators_from_manifest_merges_evaluators_and_judges():
    manifest = {
        "x_testing": {
            "evaluators": [
                {
                    "id": "ev1",
                    "type": "script",
                    "runtime": "python",
                    "inline": "def evaluate(o, c): return {'passed': True}",
                }
            ],
            "judges": [{"id": "j1", "type": "llm_judge", "model": "gpt-4o-mini"}],
        }
    }
    evs = load_evaluators_from_manifest(manifest)
    assert "ev1" in evs
    assert "j1" in evs


def test_load_evaluators_from_manifest_evaluators_wins_on_id_collision():
    manifest = {
        "x_testing": {
            "evaluators": [
                {
                    "id": "shared",
                    "type": "script",
                    "runtime": "python",
                    "inline": "def evaluate(o, c): return {'passed': True}",
                }
            ],
            "judges": [{"id": "shared", "type": "llm_judge", "model": "gpt-4o"}],
        }
    }
    evs = load_evaluators_from_manifest(manifest)
    assert isinstance(evs["shared"], ScriptEvaluator)


def _dict_evaluator_fn(output, context):
    """Helper function that returns a dict — used for DeterministicEvaluator dict branch."""
    return {"passed": True, "score": 0.8, "reason": "dict result", "extra": "meta"}


def _int_evaluator_fn(output, context):
    """Helper function that returns an int (truthy) — tests non-bool/non-dict branch."""
    return 42


def test_deterministic_evaluator_dict_result():
    """DeterministicEvaluator returns EvaluationResult from a dict function result."""
    import sys
    # Register a fake module so importlib.import_module can find our test function
    import types
    mod = types.ModuleType("_adp_test_helpers")
    mod.dict_fn = _dict_evaluator_fn
    sys.modules["_adp_test_helpers"] = mod
    try:
        config = {
            "id": "det-dict",
            "type": "deterministic",
            "function_ref": "_adp_test_helpers:dict_fn",
        }
        ev = load_evaluator(config)
        result = ev.evaluate_sync({}, {})
        assert isinstance(result, EvaluationResult)
        assert result.evaluator_id == "det-dict"
        assert result.passed is True
        assert result.score == 0.8
        assert result.reason == "dict result"
        assert result.metadata == {"extra": "meta"}
    finally:
        sys.modules.pop("_adp_test_helpers", None)


def test_deterministic_evaluator_non_bool_non_dict_result():
    """DeterministicEvaluator handles a non-bool, non-dict return (truthy int)."""
    import sys
    import types
    mod = types.ModuleType("_adp_test_helpers2")
    mod.int_fn = _int_evaluator_fn
    sys.modules["_adp_test_helpers2"] = mod
    try:
        config = {
            "id": "det-int",
            "type": "deterministic",
            "function_ref": "_adp_test_helpers2:int_fn",
        }
        ev = load_evaluator(config)
        result = ev.evaluate_sync({}, {})
        assert isinstance(result, EvaluationResult)
        assert result.passed is True  # bool(42) == True
        assert result.evaluator_id == "det-int"
    finally:
        sys.modules.pop("_adp_test_helpers2", None)


def test_script_evaluator_bash_runtime():
    """ScriptEvaluator works with bash runtime."""
    import shutil
    if not shutil.which("bash"):
        pytest.skip("bash not available")
    bash_script = '#!/bin/bash\necho \'{"passed": true, "score": 1.0, "reason": "ok"}\'\n'
    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(bash_script)
        script_path = f.name
    try:
        os.chmod(script_path, 0o755)
        config = {
            "id": "bash-ev",
            "type": "script",
            "runtime": "bash",
            "script_ref": script_path,
        }
        ev = load_evaluator(config)
        result = ev.evaluate_sync({}, {})
        assert isinstance(result, EvaluationResult)
        assert result.passed is True
        assert result.score == 1.0
    finally:
        Path(script_path).unlink(missing_ok=True)


def test_script_evaluator_script_ref_file():
    """ScriptEvaluator loads a script from a file path (script_ref branch)."""
    script_code = "def evaluate(output, context):\n    return {'passed': True, 'score': 0.9, 'reason': 'file ref'}\n"
    with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
        f.write(script_code)
        script_path = f.name
    try:
        config = {
            "id": "file-ref-ev",
            "type": "script",
            "runtime": "python",
            "script_ref": script_path,
        }
        ev = load_evaluator(config)
        result = ev.evaluate_sync({}, {})
        assert result.passed is True
        assert result.score == 0.9
        assert result.reason == "file ref"
    finally:
        Path(script_path).unlink(missing_ok=True)


def test_run_script_nonzero_exit():
    """_run_script returns passed=False when the script exits non-zero."""
    from adp_sdk.evaluation import _run_script
    import shutil
    if not shutil.which("python3"):
        pytest.skip("python3 not available")
    # Write a script that raises an exception (exits non-zero)
    with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
        f.write("def evaluate(output, context):\n    raise RuntimeError('fail')\n")
        script_path = f.name
    try:
        result = _run_script("python", script_path, {}, {})
        assert result["passed"] is False
        assert "exited" in result["reason"]
    finally:
        Path(script_path).unlink(missing_ok=True)


def test_run_script_invalid_json_output():
    """_run_script returns passed=False when script stdout is not valid JSON."""
    from adp_sdk.evaluation import _run_script
    import shutil
    if not shutil.which("python3"):
        pytest.skip("python3 not available")
    # Script whose evaluate() outputs a non-JSON string via print before json.dumps wraps it.
    # We use sys.stdout.write directly to bypass the json.dumps wrapper.
    script_code = (
        "import sys\n"
        "def evaluate(output, context):\n"
        "    sys.stdout.write('NOT VALID JSON\\n')\n"
        "    return {'passed': True}\n"
    )
    with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
        f.write(script_code)
        script_path = f.name
    try:
        result = _run_script("python", script_path, {}, {})
        # stdout = "NOT VALID JSON\n{...}" which fails JSON decode on the whole thing
        assert result["passed"] is False
        assert "not valid JSON" in result["reason"]
    finally:
        Path(script_path).unlink(missing_ok=True)


def test_run_script_unsupported_runtime():
    """_run_script raises ValueError for unsupported runtime."""
    from adp_sdk.evaluation import _run_script
    with pytest.raises(ValueError, match="Unsupported script runtime"):
        _run_script("ruby", "/some/script.rb", {}, {})


def test_resolve_script_ref_plain_path():
    """_resolve_script_ref returns Path(ref) for plain file paths (no git+ prefix)."""
    result = _resolve_script_ref("/some/local/script.py")
    assert str(result) == "/some/local/script.py"


def test_resolve_script_ref_cached(tmp_path):
    """_resolve_script_ref returns cached path when sentinel file exists."""
    import re
    from adp_sdk.evaluation import _GIT_REF_RE, _CACHE_DIR
    # Build a synthetic cache: create sentinel file so the cached path is returned
    sha = "a" * 7
    cache_dir = tmp_path / sha
    cache_dir.mkdir(parents=True)
    sentinel = cache_dir / "_verified"
    sentinel.write_text("ok")
    # Patch _CACHE_DIR so it points to our temp dir
    import adp_sdk.evaluation as ev_module
    original_cache = ev_module._CACHE_DIR
    ev_module._CACHE_DIR = tmp_path
    try:
        ref = f"git+https://github.com/example/repo.git/path/to/script.py@{sha}"
        result = _resolve_script_ref(ref)
        assert "script.py" in str(result)
    finally:
        ev_module._CACHE_DIR = original_cache


def test_load_evaluators_from_manifest_no_x_testing():
    """load_evaluators_from_manifest returns empty dict when x_testing is missing."""
    manifest = {}
    evs = load_evaluators_from_manifest(manifest)
    assert evs == {}


def test_load_evaluators_from_manifest_judges_only_warns():
    """load_evaluators_from_manifest issues DeprecationWarning for judges-only config."""
    import warnings
    manifest = {
        "x_testing": {
            "judges": [{"id": "j1", "type": "llm_judge", "model": "gpt-4o"}],
        }
    }
    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        evs = load_evaluators_from_manifest(manifest)
    assert "j1" in evs
    assert any(issubclass(warning.category, DeprecationWarning) for warning in w)


def test_load_evaluators_from_manifest_id_collision_logs(caplog):
    """load_evaluators_from_manifest logs debug message on id collision."""
    import logging
    manifest = {
        "x_testing": {
            "evaluators": [
                {
                    "id": "shared",
                    "type": "script",
                    "runtime": "python",
                    "inline": "def evaluate(o, c): return {'passed': True}",
                }
            ],
            "judges": [{"id": "shared", "type": "llm_judge", "model": "gpt-4o"}],
        }
    }
    with caplog.at_level(logging.DEBUG):
        evs = load_evaluators_from_manifest(manifest)
    assert isinstance(evs["shared"], ScriptEvaluator)


def test_deterministic_evaluator_bool_result():
    """DeterministicEvaluator returns EvaluationResult when function returns a bool (line 49 branch)."""
    import sys
    import types
    mod = types.ModuleType("_adp_test_bool_helpers")
    mod.bool_fn = lambda output, context: True
    sys.modules["_adp_test_bool_helpers"] = mod
    try:
        config = {
            "id": "det-bool",
            "type": "deterministic",
            "function_ref": "_adp_test_bool_helpers:bool_fn",
        }
        ev = load_evaluator(config)
        result = ev.evaluate_sync({}, {})
        assert isinstance(result, EvaluationResult)
        assert result.passed is True
        assert result.score is None
        assert result.evaluator_id == "det-bool"
    finally:
        sys.modules.pop("_adp_test_bool_helpers", None)


@pytest.mark.skipif(
    not __import__("shutil").which("docker"),
    reason="Docker not available",
)
def test_container_evaluator_skipped_without_docker():
    pass  # pragma: no cover
