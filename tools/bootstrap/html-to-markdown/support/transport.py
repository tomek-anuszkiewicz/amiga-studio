"""HTML model catalog and input-capability validation."""

from conversion.transport import CodexTransport


class HtmlTransport(CodexTransport):
    def __init__(self):
        super().__init__()
        from openai_codex.generated.v2_all import ModelListResponse
        try:
            self.models = []
            cursor = None
            while True:
                response = self._client.request("model/list", {"cursor": cursor}, response_model=ModelListResponse)
                data = response.model_dump(mode="json", by_alias=True)
                self.models.extend(data["data"])
                cursor = data["nextCursor"]
                if not cursor:
                    break
        except BaseException:
            self.close()
            raise

    def validate(self, selected, *, images=False):
        model = next((m for m in self.models if m["model"] == selected.model), None)
        if model is None:
            raise ValueError(f"Model not in runtime catalog: {selected.model}")
        efforts = {e["reasoningEffort"] for e in model["supportedReasoningEfforts"]}
        if selected.reasoning_effort not in efforts:
            raise ValueError(f"Unsupported effort for {selected.model}: {selected.reasoning_effort}")
        required = {"text", "image"} if images else {"text"}
        if not required.issubset(model["inputModalities"]):
            raise ValueError(f"{selected.stage}: missing input capabilities {required}")

    def run(self, selected, prompt, *, images=(), timeout=180):
        self.validate(selected, images=bool(images))
        return super().run(selected, prompt, images=images, timeout=timeout)
