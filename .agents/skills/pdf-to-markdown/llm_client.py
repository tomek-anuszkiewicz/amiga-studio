#!/usr/bin/env python3
"""
llm_client.py: Unified Gemini LLM client for pdf-to-markdown stages.
Uses google.genai SDK with gemini-3.8-flash and thinking_level='medium'.
"""

import atexit
import json
import os
import threading
from pathlib import Path
from typing import Optional
from dotenv import load_dotenv

# Try loading .env from repo root or working directory
for p in [Path.cwd() / ".env", Path(__file__).resolve().parents[3] / ".env"]:
    if p.exists():
        load_dotenv(p)
        break

_call_lock = threading.Lock()
_stage_call_count = 0


def _init_call_count():
    global _stage_call_count
    metrics_file = os.getenv("LLM_STAGE_METRICS_FILE")
    if metrics_file:
        try:
            p = Path(metrics_file)
            if p.exists():
                with open(p, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    _stage_call_count = int(data.get("llm_calls", 0))
        except Exception:
            pass


_init_call_count()


def _record_call():
    global _stage_call_count
    with _call_lock:
        _stage_call_count += 1
        metrics_file = os.getenv("LLM_STAGE_METRICS_FILE")
        if metrics_file:
            try:
                p = Path(metrics_file)
                p.parent.mkdir(parents=True, exist_ok=True)
                with open(p, "w", encoding="utf-8") as f:
                    json.dump({"llm_calls": _stage_call_count}, f)
            except Exception:
                pass


def _dump_metrics():
    metrics_file = os.getenv("LLM_STAGE_METRICS_FILE")
    if metrics_file:
        try:
            p = Path(metrics_file)
            p.parent.mkdir(parents=True, exist_ok=True)
            with _call_lock:
                count = _stage_call_count
            with open(p, "w", encoding="utf-8") as f:
                json.dump({"llm_calls": count}, f)
        except Exception:
            pass


atexit.register(_dump_metrics)


from datetime import datetime

_call_counter = 0
_call_counter_lock = threading.Lock()


def _next_call_id() -> int:
    global _call_counter
    with _call_counter_lock:
        _call_counter += 1
        return _call_counter


class GeminiClient:
    def __init__(self, config: dict):
        if not config or not isinstance(config, dict) or "llm" not in config:
            raise ValueError(
                "GeminiClient requires an explicit, non-empty configuration dictionary containing an 'llm' section."
            )
        self.config = config["llm"]
        self.api_key = os.getenv("GEMINI_API_KEY")
        self.default_model = self.config.get("model_prose", "gemini-3.8-flash")
        self.vision_model = self.config.get("model_vision", "gemini-3.8-flash")
        self.default_thinking_budget = self.config.get("default_thinking_budget", 0)
        self.stages_thinking_budget = self.config.get("stages_thinking_budget", {})
        self.temperature = self.config.get("temperature", 0.1)
        self.client = None
        self._init_client()

    @property
    def call_count(self) -> int:
        with _call_lock:
            return _stage_call_count

    @classmethod
    def get_total_calls(cls) -> int:
        with _call_lock:
            return _stage_call_count

    @classmethod
    def reset_call_count(cls):
        global _stage_call_count
        with _call_lock:
            _stage_call_count = 0

    def _init_client(self):
        if not self.api_key:
            raise RuntimeError("GEMINI_API_KEY environment variable is required for pdf-to-markdown pipeline.")
        from google import genai
        self.client = genai.Client(api_key=self.api_key)

    def is_available(self) -> bool:
        return True

    def resolve_thinking_budget(self, stage: Optional[str] = None, thinking_budget: Optional[int] = None) -> Optional[int]:
        if thinking_budget is not None:
            return thinking_budget
        if stage and stage in self.stages_thinking_budget:
            return self.stages_thinking_budget[stage]
        return self.default_thinking_budget

    def _build_config(self, stage: Optional[str] = None, thinking_budget: Optional[int] = None, response_mime_type: Optional[str] = None):
        from google.genai import types
        budget = self.resolve_thinking_budget(stage=stage, thinking_budget=thinking_budget)
        thinking_config = None
        if budget is not None:
            thinking_config = types.ThinkingConfig(thinking_budget=budget)

        cfg = types.GenerateContentConfig(
            temperature=self.temperature,
            thinking_config=thinking_config,
            response_mime_type=response_mime_type,
        )
        return cfg, budget

    def generate_text(self, prompt: str, model: str = None, stage: Optional[str] = None, thinking_budget: Optional[int] = None, response_mime_type: Optional[str] = None) -> str:
        import time
        import re

        models_to_try = [model or self.default_model]
        if "gemini-3.6-flash" not in models_to_try:
            models_to_try.append("gemini-3.6-flash")

        config, budget = self._build_config(stage=stage, thinking_budget=thinking_budget, response_mime_type=response_mime_type)
        cid = _next_call_id()

        last_error = None
        for m in models_to_try:
            for attempt in range(4):
                try:
                    _record_call()
                    ts_start = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                    b_str = f"thinking={budget}" if budget is not None else "thinking=auto"
                    s_str = f" stage={stage}" if stage else ""
                    print(f"[{ts_start}] [LLM START #{cid}] model={m}{s_str} {b_str}...")
                    t0 = time.perf_counter()

                    response = self.client.models.generate_content(
                        model=m,
                        contents=prompt,
                        config=config,
                    )
                    elapsed = time.perf_counter() - t0
                    ts_done = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                    print(f"[{ts_done}] [LLM DONE  #{cid}] elapsed={elapsed:.2f}s")

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

    def generate_vision(self, prompt: str, image_path, model: str = None, stage: Optional[str] = None, thinking_budget: Optional[int] = None, response_mime_type: Optional[str] = None) -> str:
        import time
        import re
        from PIL import Image

        if isinstance(image_path, (list, tuple)):
            paths = [Path(p) for p in image_path if Path(p).exists()]
            if not paths:
                raise FileNotFoundError(f"No valid images found in: {image_path}")
            images = [Image.open(p) for p in paths]
            img_desc = f"{len(images)} images: {[p.name for p in paths]}"
        else:
            p = Path(image_path)
            if not p.exists():
                raise FileNotFoundError(f"Image not found for vision generation: {image_path}")
            images = [Image.open(p)]
            img_desc = p.name

        models_to_try = [model or self.vision_model]
        if "gemini-3.6-flash" not in models_to_try:
            models_to_try.append("gemini-3.6-flash")

        config, budget = self._build_config(stage=stage, thinking_budget=thinking_budget, response_mime_type=response_mime_type)
        cid = _next_call_id()

        last_error = None
        for m in models_to_try:
            for attempt in range(4):
                try:
                    _record_call()
                    ts_start = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                    b_str = f"thinking={budget}" if budget is not None else "thinking=auto"
                    s_str = f" stage={stage}" if stage else ""
                    print(f"[{ts_start}] [LLM START #{cid}] model={m}{s_str} {b_str} img={img_desc}...")
                    t0 = time.perf_counter()

                    response = self.client.models.generate_content(
                        model=m,
                        contents=[*images, prompt],
                        config=config,
                    )
                    elapsed = time.perf_counter() - t0
                    ts_done = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                    print(f"[{ts_done}] [LLM DONE  #{cid}] elapsed={elapsed:.2f}s")

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

    def generate_json(self, prompt: str, image_path: Optional[Path] = None, model: str = None, stage: Optional[str] = None, thinking_budget: Optional[int] = None):
        import json
        import re

        def _clean_and_parse(raw_text: str):
            text = raw_text.strip()
            fence_match = re.search(r"```(?:json)?\s*([\s\S]*?)\s*```", text)
            if fence_match:
                text = fence_match.group(1).strip()

            # 1. Direct parse with strict=False (allows control chars)
            try:
                return json.loads(text, strict=False)
            except Exception:
                pass

            # 2. Repair naked unescaped backslashes (e.g. \a, \alpha, \ ) common in math/OCR
            repaired = re.sub(r'\\([^"\\/bfnrtu])', r'\\\\\1', text)
            try:
                return json.loads(repaired, strict=False)
            except Exception:
                pass

            # 3. Bracket extraction with repair
            bracket_match = re.search(r"(\{[\s\S]*\}|\[[\s\S]*\])", text)
            if bracket_match:
                cand = bracket_match.group(1)
                try:
                    return json.loads(cand, strict=False)
                except Exception:
                    repaired_cand = re.sub(r'\\([^"\\/bfnrtu])', r'\\\\\1', cand)
                    return json.loads(repaired_cand, strict=False)

            raise ValueError(f"No valid JSON found in response:\n{text[:400]}")

        # Try with response_mime_type="application/json", retrying up to 2 times on parsing failure
        last_parse_error = None
        for attempt in range(2):
            raw = (
                self.generate_vision(prompt, image_path, model=model, stage=stage, thinking_budget=thinking_budget, response_mime_type="application/json")
                if image_path else
                self.generate_text(prompt, model=model, stage=stage, thinking_budget=thinking_budget, response_mime_type="application/json")
            )
            if not raw:
                raise RuntimeError("Fatal: LLM returned empty response for generate_json.")

            try:
                return _clean_and_parse(raw)
            except Exception as e:
                last_parse_error = e
                print(f"[!] Warning: JSON parse failed (attempt {attempt+1}/2): {e}. Retrying with plain text mode...")
                # Second attempt fallback to unconstrained text mode
                try:
                    raw_fallback = (
                        self.generate_vision(prompt, image_path, model=model, stage=stage, thinking_budget=thinking_budget)
                        if image_path else
                        self.generate_text(prompt, model=model, stage=stage, thinking_budget=thinking_budget)
                    )
                    return _clean_and_parse(raw_fallback)
                except Exception as e2:
                    last_parse_error = e2

        raise RuntimeError(f"Fatal: JSON parse failed across all retry attempts: {last_parse_error}")
