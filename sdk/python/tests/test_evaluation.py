import pytest
from adp_sdk.evaluation import (
    load_evaluator,
    load_evaluators_from_manifest,
    EvaluationResult,
    ScriptEvaluator,
    DeterministicEvaluator,
    LLMJudgeEvaluator,
    ScriptRefVerificationError,
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


@pytest.mark.skipif(
    not __import__("shutil").which("docker"),
    reason="Docker not available",
)
def test_container_evaluator_skipped_without_docker():
    pass  # pragma: no cover
