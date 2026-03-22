import json
import pytest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).parent.parent))

from update_pricing import compute_diff, validate_pricing, models_to_dict
from fetchers.base_fetcher import ModelPricing
from fetchers.openai_fetcher import OpenAIFetcher
from fetchers.anthropic_fetcher import AnthropicFetcher


def make_entry(input_price,output_price):
    return{
        "provider":"openai",
        "input_per_1m_usd":input_price,
        "output_per_1m_usd":output_price,
        "context_window":128000,
        "notes":None,
    }

def test_diff_detects_price_increase():
    old={"gpt-4o":make_entry(2.50,10.00)}
    new={"gpt-4o":make_entry(5.00,15.00)}
    changes=compute_diff(old,new)
    assert len(changes)==1
    assert changes[0]["type"]=="changed"
    assert changes[0]["pct_input"]==100.0


def test_diff_detects_price_decrease():
    old = {"gpt-4o": make_entry(5.00, 20.00)}
    new = {"gpt-4o": make_entry(2.50, 10.00)}
    changes = compute_diff(old, new)
    assert changes[0]["pct_input"] == -50.0

def test_diff_detects_new_model():
    old={}
    new={"gpt-5":make_entry(10.00,30.00)}
    changes=compute_diff(old,new)
    assert len(changes)==1

def test_diff_detects_removed_model():
    old = {"gpt-3": make_entry(1.00, 2.00)}
    new = {}
    changes = compute_diff(old, new)
    assert changes[0]["type"] == "removed"

def test_diff_no_changes_when_identical():
    models = {"gpt-4o": make_entry(2.50, 10.00)}
    changes = compute_diff(models, models)
    assert changes == []

def test_diff_ignores_float_rounding_noise():
    old = {"gpt-4o": make_entry(2.50, 10.00)}
    # 0.0001 difference should NOT be flagged
    new = {"gpt-4o": make_entry(2.5001, 10.0001)}
    changes = compute_diff(old, new)
    assert changes == []

# validate pricing tests 
def test_validation_passes_for_good_data():
    models = {
        f"model-{i}": make_entry(1.0 + i, 3.0 + i)
        for i in range(6)
    }
    errors = validate_pricing(models)
    assert errors == []

def test_validation_fails_for_zero_price():
    models = {"bad-model": make_entry(0, 10.00)}
    errors = validate_pricing(models)
    assert any("input price is <= 0" in e for e in errors)


def test_validation_warn_when_output_cheaper_than_input():
    models = {f"m{i}": make_entry(1.0, 2.0) for i in range(6)}
    models["weird"] = make_entry(10.00, 5.00)  # output < input
    errors = validate_pricing(models)
    assert any("output price < input price" in e for e in errors)

def test_validation_fails_for_too_few_models():
    models={"only-one":make_entry(1.0,3.0)}
    errors=validate_pricing(models)
    assert any("Too few models" in e for e in errors)

# fetcher fallback tests 
def test_openai_fallback_returns_valid_models():
    fetcher=OpenAIFetcher()
    models=fetcher._build_from_fallback()
    assert len(models)>=5
    for m in models:
        assert m.provider =="openai"
        assert m.input_per_1m>0
        assert m.output_per_1m>0 

def test_anthropic_fallback_returns_valid_models():
    fetcher=AnthropicFetcher()
    models=fetcher._build_from_fallback()
    assert len(models)>=5
    for m in models:
        assert m.provider=="anthropic"
        assert "claude" in m.model_id

def test_models_to_dict_structure():
    models=[
        ModelPricing("gpt-4o","openai",2.50,10.00,128_000,None)
    ]
    result=models_to_dict(models)
    assert "gpt-4o" in result
    assert result["gpt-4o"]["input_per_1m_usd"]==2.50
    assert result["gpt-4o"]["provider"]=="openai"

