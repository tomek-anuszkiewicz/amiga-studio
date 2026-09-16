#!/usr/bin/env python3
"""
llm_client.py: Unified Gemini LLM client for pdf-to-markdown stages.
Uses google.genai SDK with gemini-3.8-flash and thinking_level='medium'.
"""

import os
from pathlib import Path
from typing import Optional
from dotenv import load_dotenv

# Try loading .env from repo root or working directory
for p in [Path.cwd() / ".env", Path(__file__).resolve().parents[3] / ".env"]:
    if p.exists():
        load_dotenv(p)
        break


class GeminiClient:
    def __init__(self, config: dict = None):
        self.config = (config or {}).get("llm", {})
        self.api_key = os.getenv("GEMINI_API_KEY")
        self.default_model = self.config.get("model_prose", "gemini-3.8-flash")
        self.vision_model = self.config.get("model_vision", "gemini-3.8-flash")
        self.thinking_level = self.config.get("thinking_level", "medium")
        self.temperature = self.config.get("temperature", 0.1)
        self.client = None
        self._init_client()

    def _init_client(self):
        if not self.api_key:
            return
        try:
            from google import genai
            self.client = genai.Client(api_key=self.api_key)
        except Exception as e:
            print(f"[!] Warning: Failed to initialize google.genai Client: {e}")
            self.client = None

    def is_available(self) -> bool:
        return self.client is not None and bool(self.api_key)

    def generate_text(self, prompt: str, model: str = None) -> str:
        if not self.is_available():
            raise RuntimeError("GEMINI_API_KEY environment variable is required for pipeline inference.")
        import time
        import re
        from google.genai import types

        models_to_try = [model or self.default_model]
        if "gemini-3.6-flash" not in models_to_try:
            models_to_try.append("gemini-3.6-flash")

        last_error = None
        for m in models_to_try:
            for attempt in range(4):
                try:
                    config = types.GenerateContentConfig(
                        temperature=self.temperature,
                    )
                    response = self.client.models.generate_content(
                        model=m,
                        contents=prompt,
                        config=config,
                    )
                    if response and response.text:
                        return response.text
                except Exception as e:
                    last_error = e
                    err_str = str(e)
                    if "429" in err_str or "RESOURCE_EXHAUSTED" in err_str:
                        delay_match = re.search(r"retry in (\d+(?:\.\d+)?)s", err_str)
                        delay = float(delay_match.group(1)) + 1.0 if delay_match else (12.0 * (attempt + 1))
                        print(f"[*] Rate limit (429). Backing off for {delay:.1f}s (attempt {attempt+1}/4)...")
                        time.sleep(delay)
                        continue
                    print(f"[!] Warning: LLM generate_text on model {m} failed: {e}")
                    break

        raise RuntimeError(
            f"Fatal: LLM generate_text failed across all models ({models_to_try}) and retry attempts. "
            f"Last error: {last_error}"
        )

    def generate_vision(self, prompt: str, image_path: Path, model: str = None) -> str:
        if not self.is_available():
            raise RuntimeError("GEMINI_API_KEY environment variable is required for pipeline inference.")
        if not image_path.exists():
            raise FileNotFoundError(f"Image not found for vision generation: {image_path}")
        import time
        import re
        from google.genai import types
        from PIL import Image

        models_to_try = [model or self.vision_model]
        if "gemini-3.6-flash" not in models_to_try:
            models_to_try.append("gemini-3.6-flash")

        image = Image.open(image_path)
        last_error = None
        for m in models_to_try:
            for attempt in range(4):
                try:
                    config = types.GenerateContentConfig(
                        temperature=self.temperature,
                    )
                    response = self.client.models.generate_content(
                        model=m,
                        contents=[image, prompt],
                        config=config,
                    )
                    if response and response.text:
                        return response.text
                except Exception as e:
                    last_error = e
                    err_str = str(e)
                    if "429" in err_str or "RESOURCE_EXHAUSTED" in err_str:
                        delay_match = re.search(r"retry in (\d+(?:\.\d+)?)s", err_str)
                        delay = float(delay_match.group(1)) + 1.0 if delay_match else (12.0 * (attempt + 1))
                        print(f"[*] Rate limit (429). Backing off for {delay:.1f}s (attempt {attempt+1}/4)...")
                        time.sleep(delay)
                        continue
                    print(f"[!] Warning: LLM generate_vision on model {m} failed: {e}")
                    break

        raise RuntimeError(
            f"Fatal: LLM generate_vision failed across all models ({models_to_try}) and retry attempts for {image_path}. "
            f"Last error: {last_error}"
        )

    def generate_json(self, prompt: str, image_path: Optional[Path] = None, model: str = None):
        import json
        import re

        raw = self.generate_vision(prompt, image_path, model=model) if image_path else self.generate_text(prompt, model=model)
        if not raw:
            raise RuntimeError("Fatal: LLM returned empty response for generate_json.")

        # Clean markdown fences if present
        text = raw.strip()
        fence_match = re.search(r"```(?:json)?\s*([\s\S]*?)\s*```", text)
        if fence_match:
            text = fence_match.group(1).strip()

        # Try parsing full text
        try:
            return json.loads(text)
        except Exception:
            pass

        # Try finding JSON object or array
        bracket_match = re.search(r"(\{[\s\S]*\}|\[[\s\S]*\])", text)
        if bracket_match:
            try:
                return json.loads(bracket_match.group(1))
            except Exception as e:
                raise RuntimeError(f"Fatal: JSON parse failed: {e}\nRaw response:\n{text[:500]}")

        raise RuntimeError(f"Fatal: No valid JSON object or array found in LLM response:\n{text[:500]}")
