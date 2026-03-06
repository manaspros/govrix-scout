import logging
from typing import Optional
import requests
from bs4 import BeautifulSoup
from .base_fetcher import BaseFetcher, ModelPricing
logger=logging.getLogger(__name__)

## Fallback Table 
# source https://platform.claude.com/docs/en/about-claude/pricing
ANTHROPIC_FALLBACK: list[dict] = [
    # Claude 4.6 family (current)
    {"model_id": "claude-opus-4-6",        "input": 5.00,  "output": 25.00, "context": 200_000},
    {"model_id": "claude-sonnet-4-6",      "input": 3.00,  "output": 15.00, "context": 200_000},
    # Claude 4.5 family
    {"model_id": "claude-opus-4-5",        "input": 5.00,  "output": 25.00, "context": 200_000},
    {"model_id": "claude-sonnet-4-5",      "input": 3.00,  "output": 15.00, "context": 200_000},
    {"model_id": "claude-haiku-4-5",       "input": 1.00,  "output": 5.00,  "context": 200_000},
    # Claude 4 family
    {"model_id": "claude-opus-4",          "input": 15.00, "output": 75.00, "context": 200_000,
     "notes": "legacy"},
    {"model_id": "claude-sonnet-4",        "input": 3.00,  "output": 15.00, "context": 200_000},
    # Claude 3.5 family (still widely used)
    {"model_id": "claude-sonnet-3-5",      "input": 3.00,  "output": 15.00, "context": 200_000,
     "notes": "legacy"},
    {"model_id": "claude-haiku-3-5",       "input": 0.80,  "output": 4.00,  "context": 200_000,
     "notes": "legacy"},
]

class AnthropicFetcher(BaseFetcher):
    PRICING_URL = "https://platform.claude.com/docs/en/about-claude/pricing"
    @property 
    def provider_name(self)->str:
        return "Anthropic"
    

    def fetch(self)->list[ModelPricing]:
        try:
            scraped=self._scrape_pricing_page()
            if scraped:
                logger.info(f"[Anthropic]Live scrape succeeded: {len(scraped)} models")
                return scraped
        except Exception as e:
            logger.warning(f"[Anthropic] Live Scrape failed: ({e}), using fallback table ")


        logger.info(f"[Anthropic] Using fallback table with {len(ANTHROPIC_FALLBACK)} models")
        return self._build_from_fallback()
    
    def _build_from_fallback(self)->list[ModelPricing]:
        return [
            ModelPricing(
                model_id=e["model_id"],
                provider="anthropic",
                input_per_1m=e["input"],
                output_per_1m=e["output"],
                context_window=e.get("context"),
                notes=e.get("notes"),
            )
            for e in ANTHROPIC_FALLBACK
        ]
    
    def _scrape_pricing_page(self)->Optional[list[ModelPricing]]:
        resp=requests.get(self.PRICING_URL,headers=self.HEADERS,timeout=15)
        resp.raise_for_status()
        soup=BeautifulSoup(resp.text,"lxml")
        models=[]

        for row in soup.select("table tr"):
            cells=row.select("td")
            if len(cells)<3:
                continue
                
            model_name=cells[0].get_text(strip=True).lower()
            if "claude" not in model_name:
                continue
            try:
                input_price=self._parse_price(cells[1].get_text(strip=True))
                output_price=self._parse_price(cells[2].get_text(strip=True))
                if input_price is None or output_price is None:
                    continue
                    
                models.append(ModelPricing(
                    model_id=model_name.replace(" ","-"),
                    provider="anthropic",
                    input_per_1m=input_price,
                    output_per_1m=output_price
                ))
            except (ValueError,IndexError):
                continue
            return models if len(models)>=3 else None
    @staticmethod
    def _parse_price(text:str)->Optional[float]:
        text=text.replace(",", "").replace("$", "").strip()
        try:
            return float(text.split()[0])
        except(ValueError,IndexError):
            return None
