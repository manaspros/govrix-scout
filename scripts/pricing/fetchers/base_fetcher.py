from abc import ABC,abstractmethod
from dataclasses import dataclass, field
from typing import Optional

@dataclass
class ModelPricing:
    """Pricing for a single model - all cost in USD per 1M tokens. """
    model_id:str
    provider:str
    input_per_1m:float
    output_per_1m:float
    context_window:Optional[int] = None
    notes:Optional[str]=None

class BaseFetcher(ABC):
    """All provider fetchers must implement this interface."""

    HEADERS = {
        "User-Agent": (
            "govrix-scout-pricing-updater/1.0 "
            "(https://github.com/manaspros/govrix-scout)"
        )
    }


    @abstractmethod
    def fetch(self)->list[ModelPricing]:
        """Fetch the pricing data and return a list of ModelPricing objects."""
        ...
    @property
    @abstractmethod
    def provider_name(self)->str:
        """Human - readable provider name e.g 'OpenAI'"""


        