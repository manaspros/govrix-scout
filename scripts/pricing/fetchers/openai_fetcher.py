# open ai doesnt expose a pricing json api 
# i will  maintain a known good fall back table and try to scrape 
# the pricing page as a supplement the fallback is always used 

import logging 
from typing import Optional
import requests
from bs4 import BeautifulSoup
from .base_fetcher import BaseFetcher, ModelPricing

logger=logging.getLogger(__name__)

# Fall back table(update the mannually when openai annouces changes )
# pricing in usd 1m tokens. 
OPENAI_FALLBACK: list[dict] = [
    # Current flagship models
    {"model_id": "gpt-4.1",          "input": 2.00,  "output": 8.00,  "context": 1_000_000},
    {"model_id": "gpt-4.1-mini",     "input": 0.40,  "output": 1.60,  "context": 1_000_000},
    {"model_id": "gpt-4.1-nano",     "input": 0.10,  "output": 0.40,  "context": 1_000_000},
    {"model_id": "gpt-4o",           "input": 2.50,  "output": 10.00, "context": 128_000},
    {"model_id": "gpt-4o-mini",      "input": 0.15,  "output": 0.60,  "context": 128_000},
    # Legacy (still active for existing integrations)
    {"model_id": "gpt-4-turbo",      "input": 10.00, "output": 30.00, "context": 128_000,
     "notes": "legacy"},
    {"model_id": "gpt-4",            "input": 30.00, "output": 60.00, "context": 8_192,
     "notes": "legacy"},
    {"model_id": "gpt-3.5-turbo",    "input": 0.50,  "output": 1.50,  "context": 16_385,
     "notes": "legacy"},
    # Reasoning models
    {"model_id": "o3",               "input": 10.00, "output": 40.00, "context": 200_000},
    {"model_id": "o3-mini",          "input": 1.10,  "output": 4.40,  "context": 200_000},
    {"model_id": "o4-mini",          "input": 1.10,  "output": 4.40,  "context": 200_000},
]


class OpenAIFetcher(BaseFetcher):
    PRICING_URL = "https://openai.com/api/pricing/"

    @property
    def provider_name(self)->str:
        return "OpenAI"
    
    def fetch(self)->list[ModelPricing]:
        """ 
        Try live scrape first: fallback to hardcoded table on any faliure 
        This gurantees the script alwasys returns something valid.
        """
        try:
            scraped=self._scrape_pricing_page()
            if scraped:
                logger.info(f"[OpenAI]Live scrape succeeded: {len(scraped)} models foung")
                return scraped
        except Exception as e:
            logger.warning(f"[OpenAI] Live Scrape failed: ({e}), using fallback table ")
        
        logger.info(f"[OpenAI] Using fallback table with {len(OPENAI_FALLBACK)} models")
        return self._build_from_fallback()
    

    def _build_from_fallback(self)->list[ModelPricing]:
        result=[]
        for entry in OPENAI_FALLBACK:
            result.append(ModelPricing(
                model_id=entry["model_id"],
                provider="openai",
                input_per_1m=entry["input"],
                output_per_1m=entry["output"],
                context_window=entry.get("context"),
                notes=entry.get("notes")
            ))
        return result
    
    def _scrape_pricing_page(self)->Optional[list[ModelPricing]]:
        """
        Scrape OpenAI's pricing page. This is best effort - the page structure 
        changes frequently . Returns None if parsing fails.
        """
        resp=requests.get(
            self.PRICING_URL,
            headers=self.HEADERS,
            timeout=15
        )
        resp.raise_for_status()

        soup=BeautifulSoup(resp.text,"lxml")
        models=[]
        # OpenAI's pricing page uses a table with model name + input/output columns
        # This selector targets their current format (as of 2025)

        for row in soup.select("table tr"):
            cells=row.find_all("td")
            if len(cells)<3:
                continue
            
            model_name=cells[0].get_text(strip=True).lower()
            if not model_name.startswith("gpt") and not model_name.startswith("o"):
                continue

            try:
                input_price=self._parse_price(cells[1].get_text(strip=True))
                output_price=self._parse_price(cells[2].get_text(strip=True))
                if input_price is None or output_price is None:
                    continue
                
                models.append(ModelPricing(
                    model_id=model_name,
                    provider="openai",
                    input_per_1m=input_price,
                    output_per_1m=output_price
                    
                ))
            except (ValueError,IndexError):
                continue
        return models if len(models)>=3 else None
    
    @staticmethod
    def _parse_price(text:str)->Optional[float]:
        """ Parse '$2.50 / 1M tokens' -> 2.50 """
        text=text.replace(",","").replace("$", "").strip()
        try:
            return float(text.split()[0])
        
        except (ValueError,IndexError):
            return None

