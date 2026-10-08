"""HTML text requests with configuration and model validation."""

from conversion.client import CodexClient
from .config import validate_config, HTML_STAGES
from .transport import HtmlTransport


class HtmlClient(CodexClient):
    def __init__(self, config, *, stage="html_to_markdown", transport=None, cache=None):
        validate_config(config, HTML_STAGES, {stage})
        super().__init__(config, stage=stage, operation="html_to_markdown", transport=transport or HtmlTransport(),
                         cache=cache, timeout=config["llm"].get("timeout_seconds", 180))
        try:
            self.transport.validate(self.selected)
        except BaseException:
            self.close()
            raise

    def generate_text(self, prompt, *, validator=None):
        self.transport.validate(self.selected)
        return self._generate(prompt, validator=validator)
