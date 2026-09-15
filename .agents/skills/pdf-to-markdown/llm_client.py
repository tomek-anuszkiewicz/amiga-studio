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

    def generate_text(self, prompt: str, model: str = None) -> Optional[str]:
        if not self.is_available():
            return None
        try:
            from google.genai import types
            target_model = model or self.default_model
            config = types.GenerateContentConfig(
                temperature=self.temperature,
                thinking_config=types.ThinkingConfig(thinking_level=self.thinking_level)
            )
            response = self.client.models.generate_content(
                model=target_model,
                contents=prompt,
                config=config,
            )
            return response.text if response else None
        except Exception as e:
            print(f"[!] Warning: LLM generate_text failed: {e}")
            return None

    def generate_vision(self, prompt: str, image_path: Path, model: str = None) -> Optional[str]:
        if not self.is_available():
            return None
        if not image_path.exists():
            return None
        try:
            from google.genai import types
            from PIL import Image

            target_model = model or self.vision_model
            image = Image.open(image_path)
            config = types.GenerateContentConfig(
                temperature=self.temperature,
                thinking_config=types.ThinkingConfig(thinking_level=self.thinking_level)
            )
            response = self.client.models.generate_content(
                model=target_model,
                contents=[image, prompt],
                config=config,
            )
            return response.text if response else None
        except Exception as e:
            print(f"[!] Warning: LLM generate_vision failed: {e}")
            return None
