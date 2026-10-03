#!/usr/bin/env python3
"""
Environment and dependency validation for the PDF-to-Markdown conversion pipeline.
Checks Python version, required packages, and Tesseract OCR engine availability.
"""

import sys
import shutil
from pathlib import Path

REQUIRED_PACKAGES = [
    ("pymupdf", "PyMuPDF"),
    ("pytesseract", "pytesseract"),
    ("PIL", "Pillow"),
    ("pypdf", "pypdf"),
    ("tqdm", "tqdm"),
]

OPTIONAL_PACKAGES = [
    ("ocrmypdf", "ocrmypdf"),
]


def check_python_version() -> bool:
    print(f"Python Version: {sys.version.split()[0]} ({sys.executable})")
    if sys.version_info < (3, 10):
        print("  [FAIL] Python 3.10 or higher is required.")
        return False
    print("  [OK] Python version is supported.")
    return True


def check_packages() -> bool:
    all_ok = True
    print("\nChecking Python Dependencies:")
    for mod_name, disp_name in REQUIRED_PACKAGES:
        try:
            mod = __import__(mod_name)
            ver = getattr(mod, "__version__", "unknown")
            print(f"  [OK] {disp_name:<15} (version: {ver})")
        except ImportError as e:
            print(f"  [FAIL] {disp_name:<15} Missing ({e})")
            all_ok = False

    for mod_name, disp_name in OPTIONAL_PACKAGES:
        try:
            mod = __import__(mod_name)
            ver = getattr(mod, "__version__", "unknown")
            print(f"  [OK] {disp_name:<15} (version: {ver}) [optional]")
        except ImportError:
            print(f"  [WARN] {disp_name:<15} Not installed [optional]")

    return all_ok


def check_tesseract() -> bool:
    print("\nChecking Tesseract OCR Engine:")
    tess_path = shutil.which("tesseract")
    
    if not tess_path:
        common_paths = [
            Path(r"C:\Program Files\Tesseract-OCR\tesseract.exe"),
            Path(r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe"),
            Path.home() / "AppData" / "Local" / "Programs" / "Tesseract-OCR" / "tesseract.exe",
        ]
        for cp in common_paths:
            if cp.is_file():
                tess_path = str(cp)
                break

    if not tess_path:
        print("  [FAIL] Tesseract OCR executable not found on PATH or standard install locations.")
        print("         Please install Tesseract OCR or add it to PATH.")
        return False

    try:
        import pytesseract
        pytesseract.pytesseract.tesseract_cmd = tess_path
        ver = pytesseract.get_tesseract_version()
        print(f"  [OK] Tesseract binary: {tess_path}")
        print(f"  [OK] Tesseract version: {ver}")
        return True
    except Exception as e:
        print(f"  [FAIL] Tesseract verification error: {e}")
        return False


def main() -> int:
    print("=" * 60)
    print(" PDF Pipeline Environment Self-Check (Stage 1)")
    print("=" * 60)

    py_ok = check_python_version()
    pkg_ok = check_packages()
    tess_ok = check_tesseract()

    print("\n" + "=" * 60)
    if py_ok and pkg_ok and tess_ok:
        print("[STATUS] Environment is fully configured and ready for OCR processing!")
        print("=" * 60)
        return 0
    else:
        print("[STATUS] Environment validation failed. Please address the errors above.")
        print("=" * 60)
        return 1


if __name__ == "__main__":
    sys.exit(main())
