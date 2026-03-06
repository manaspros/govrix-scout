#!/usr/bin/env python3
"""
govrix-scout pricing updater
============================
Fetches current LLM token pricing from OpenAI and Anthropic,
writes a validated pricing.json to config/pricing.json,
and prints a human-readable diff when prices change.

Usage:
    python scripts/pricing/update_pricing.py           # update
    python scripts/pricing/update_pricing.py --dry-run # preview only
    python scripts/pricing/update_pricing.py --check   # exit 1 if outdated
"""

import argparse
import json
import logging
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from fetchers.anthropic_fetcher import AnthropicFetcher
from fetchers.base_fetcher import ModelPricing
from fetchers.openai_fetcher import OpenAIFetcher

REPO_ROOT = Path(__file__).parent.parent.parent
PRICING_FILE = REPO_ROOT / "config" / "pricing.json"
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s  %(levelname)-8s  %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger(__name__)

# Helpers 
def load_existing_pricing()->dict:
    """Load existing pricing.json or return empty structure"""
    if PRICING_FILE.exists():
        with open(PRICING_FILE) as f:
            return json.load(f)
    return {"models":{},"last_updated":None,"schema_version":"1"}

def models_to_dict(models:list[ModelPricing])->dict:
    """Convert list of ModelPricing -> dict keyed by model_id"""
    result={}
    for m in models:
        result[m.model_id]={
            "provider":m.provider,
            "input_per_1m_usd":m.input_per_1m,
            "output_per_1m_usd":m.output_per_1m,
            "context_window":m.context_window,
            "notes":m.notes,
        }
    return result

def compute_diff(old:dict,new:dict)->str:
    """Compare old vs new model pricing dicts.
    Returns a human readable diff string."""
    changes=[]
    all_models=set(old.keys())|set(new.keys())

    for model_id in sorted(all_models):
        old_entry=old.get(model_id)
        new_entry=new.get(model_id)

        if old_entry is None:
            changes.append({
                "type":"added",
                "model":model_id,
                "new_input":new_entry["input_per_1m_usd"],
                "new_output":new_entry["output_per_1m_usd"],
            })
        elif new_entry is None:
            changes.append({
                "type":"removed",
                "model":model_id,
                "old_input":old_entry["input_per_1m_usd"],
                "old_output":old_entry["output_per_1m_usd"],
            })
        else:
            old_in=old_entry["input_per_1m_usd"]
            old_out=old_entry["output_per_1m_usd"]
            new_in=new_entry["input_per_1m_usd"]
            new_out=new_entry["output_per_1m_usd"]


            if abs(old_in-new_in)>0.001 or abs(old_out-new_out)>0.001:
                pct_in = ((new_in - old_in) / old_in * 100) if old_in else 0
                pct_out = ((new_out - old_out) / old_out * 100) if old_out else 0
                changes.append({
                    "type":"changed",
                    "model":model_id,
                    "old_input":old_in, 
                    "old_output":old_out,
                    "new_input":new_in,
                    "new_output":new_out,
                    "pct_input":round(pct_in,1),
                    "pct_output":round(pct_out,1)
                })

    return changes


def print_diff_report(changes:list[dict]):
    """ Print the pricing diff to stdout"""
    if not changes:
        print("\n✅  No pricing changes detected.\n")
        return

    print(f"\n{'-'*70}")
    print(f"  🔔  PRICING CHANGES DETECTED  ({len(changes)} model(s))")
    print(f"{'─' * 70}")

    for ch in changes:
        m = ch["model"]
        if ch["type"] == "added":
            print(
                f"  ➕  ADDED    {m:40s}"
                f"  in=${ch['new_input']:.2f}  out=${ch['new_output']:.2f}"
            )
        elif ch["type"] == "removed":
            print(
                f"  ➖  REMOVED  {m:40s}"
                f"  in=${ch['old_input']:.2f}  out=${ch['old_output']:.2f}"
            )
        elif ch["type"] == "changed":
            arrow_in = "↑" if ch["new_input"] > ch["old_input"] else "↓"
            arrow_out = "↑" if ch["new_output"] > ch["old_output"] else "↓"
            print(
                f"  📊  CHANGED  {m:40s}"
                f"  in: ${ch['old_input']:.2f} → ${ch['new_input']:.2f} "
                f"({arrow_in}{abs(ch['pct_input'])}%)  "
                f"out: ${ch['old_output']:.2f} → ${ch['new_output']:.2f} "
                f"({arrow_out}{abs(ch['pct_output'])}%)"
            )

    print(f"{'─' * 70}\n")


def validate_pricing(models:dict)-> list[str]:
    """
    Basic sanity checks on the pricing data. 
    return a list of error strings, (empty = all good )

    """
    errors=[]
    if len(models)<5:
        errors.append(f"Too few models in result: {len(models)}"
                      "(expected at least 5 , possible scrape failure)")
        
    for model_id, entry in models.items():
        if entry["input_per_1m_usd"] <= 0:
            errors.append(f"{model_id}: input price is <= 0")
        if entry["output_per_1m_usd"] <= 0:
            errors.append(f"{model_id}: output price is <= 0")
        if entry["output_per_1m_usd"] < entry["input_per_1m_usd"]:
            errors.append(
                f"{model_id}: output price < input price "
                f"({entry['output_per_1m_usd']} < {entry['input_per_1m_usd']}) "
                "— this is unusual, please verify"
            )

    return errors



## main 
def run(dry_run:bool=False,check_mode:bool=False)->int:
    """
    Return exit code:
    0=Success ( no changes or changes written )
    1=changes detected in -- checkmode 
    2=validation failure
    """
    logger.info("Starting govrix-scout pricing updater")

    ### Fetch 
    fetchers=[OpenAIFetcher(), AnthropicFetcher()]
    all_models=list[ModelPricing]()

    for fetcher in fetchers:
        logger.info(f"Fetching {fetcher.provider_name} pricing...")
        try:
            resutls=fetcher.fetch()
            all_models.extend(resutls)
            logger.info(f"  -> {len(resutls)} models from {fetcher.provider_name}")
        except Exception as e:
            logger.error(f" X Failed to fetch {fetcher.provider_name}:{e}")
    
    if not all_models:
        logger.error("No prcing data fetched from any provider . Aborting")
        return 2
    
    # convert + validate 
    new_models=models_to_dict(all_models)
    errors=validate_pricing(new_models)


    if errors:
        logger.error("Validation failded: ")
        for err in errors:
            logger.error(f"  . {err}")
        return 2
    
    # diff 
    existing=load_existing_pricing()
    old_models=existing.get("models",{})
    changes=compute_diff(old_models,new_models)
    print_diff_report(changes)


    ## --check mode: exit 1 if there are changes 
    if check_mode:
        if changes:
            print("⚠️  Pricing is outdated. Run `make update-pricing` to update.")
            return 1
        print("✅  Pricing is up-to-date.")
        return 0
    
    ## --dry_run: preview only dont write 
    if dry_run:
        print("🔍  Dry run — no file written.")
        print(f"    Would write {len(new_models)} models to {PRICING_FILE}")
        return 0
    
    ## Write 
    output={
        "schema_version":"1",
        "last_updated":datetime.now(timezone.utc).isoformat(),
        "models_count":len(new_models),
        "models":new_models,
    }

    PRICING_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(PRICING_FILE,"w") as f:
        json.dump(output,f,indent=2)
        f.write("\n")

    logger.info(f"✅ Wrote {len(new_models)} models -> {PRICING_FILE}")
    return 0

def main()->None:
    parser=argparse.ArgumentParser(description="Fetch and update LLM Pricing for govrix-scout")
    parser.add_argument("--dry-run",action="store_true",
    help="Preview changes without writing to pricing.json"
    )
    parser.add_argument("--check",action="store_true",
    help="Exit 1 if pricing.json is outdated(use in CI)"
    )
    args=parser.parse_args()
    sys.exit(run(dry_run=args.dry_run,check_mode=args.check))


if __name__=="__main__":
    main()





