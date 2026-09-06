import hashlib
from pathlib import Path
from typing import Optional, Dict
from PIL import Image

from .config import GEMINI_API_KEY, VISION_MODEL

class VisionAnalyzer:
    def __init__(self, cache: Dict[str, str]):
        self.cache = cache
        self.client = None
        self._init_client()

    def _init_client(self):
        if not GEMINI_API_KEY:
            return
        try:
            from google import genai
            self.client = genai.Client(api_key=GEMINI_API_KEY)
        except Exception as e:
            # Fallback attempt for google.generativeai if google-genai differs
            try:
                import google.generativeai as genai_legacy
                genai_legacy.configure(api_key=GEMINI_API_KEY)
                self.legacy_model = genai_legacy.GenerativeModel(VISION_MODEL)
            except Exception as e2:
                self.client = None

    def _file_hash(self, file_path: Path) -> str:
        hasher = hashlib.sha256()
        with open(file_path, "rb") as f:
            while chunk := f.read(65536):
                hasher.update(chunk)
        return hasher.hexdigest()

    def analyze_image(self, image_path_str: str) -> Optional[str]:
        image_path = Path(image_path_str)
        if not image_path.is_file():
            return None

        # Check cache by hash
        try:
            img_hash = self._file_hash(image_path)
            if img_hash in self.cache:
                return self.cache[img_hash]
        except Exception:
            return None

        if not self.client and not hasattr(self, 'legacy_model'):
            return None

        prompt = (
            "Analyze this technical image, diagram, or schematic. Provide a concise, highly factual "
            "technical description of what is depicted (architecture, register fields, circuit, timing, or block diagram) "
            "and extract all visible text, labels, values, and identifiers (OCR)."
        )

        description = ""
        try:
            pil_img = Image.open(image_path)
            if self.client:
                # google-genai SDK
                response = self.client.models.generate_content(
                    model=VISION_MODEL,
                    contents=[pil_img, prompt]
                )
                description = response.text or ""
            elif hasattr(self, 'legacy_model'):
                response = self.legacy_model.generate_content([prompt, pil_img])
                description = response.text or ""
        except Exception as e:
            # Silently skip if quota depleted or image unreadable
            return None

        description = description.strip()
        if description:
            self.cache[img_hash] = description
            return description
        return None
