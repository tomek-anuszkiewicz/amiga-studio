#!/usr/bin/env python3
"""
Standalone Polish Language Detection & Quality Gate
Enforces .agents/rules/language-policy.md (Strict English mandate).
Detects Polish words and phrases even without diacritics ('ogonki').
Supports:
  1. CLI file scanning: python tools/harness/check_polish.py [file_path ...]
  2. Explicit staged-additions scanning: python tools/harness/check_polish.py --staged
"""

import sys
import argparse
import re
import subprocess
from pathlib import Path

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

try:
    from lingua import Language, LanguageDetectorBuilder
    from spellchecker import SpellChecker
except ImportError as e:
    print(f"Error importing dependencies: {e}", file=sys.stderr)
    print("Please install requirements: pip install lingua-language-detector pyspellchecker", file=sys.stderr)
    sys.exit(2)

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_TARGET_FILE = REPO_ROOT / ".agents" / "rules" / "language-policy.md"

# Build language detector with candidate languages
DETECTOR = LanguageDetectorBuilder.from_languages(
    Language.ENGLISH, Language.POLISH, Language.GERMAN, Language.FRENCH, Language.LATIN
).build()

# English dictionary validator to eliminate false positives on valid English words
ENGLISH_DICT = SpellChecker(language="en")

# Technical terms, language keywords, register symbols, and crate names
TECHNICAL_WHITELIST = {
    # Rust keywords & primitives
    "bool", "true", "false", "none", "some", "ok", "err", "struct", "impl",
    "enum", "pub", "fn", "let", "mut", "self", "super", "crate", "mod",
    "trait", "type", "where", "async", "await", "dyn", "move", "ref",
    "static", "const", "unsafe", "extern", "match", "if", "else", "loop",
    "while", "for", "in", "as", "break", "continue", "return", "yield",
    "vec", "usize", "isize", "u8", "u16", "u32", "u64", "u128",
    "i8", "i16", "i32", "i64", "i128", "f32", "f64", "str", "char",
    # Hardware & emulator domain terms
    "copper", "blitter", "denise", "paula", "agnus", "amiga", "motorola",
    "chipset", "copcon", "vposr", "vhposr", "dmacon", "intreq", "intena",
    "adkcon", "adkconr", "bplcon0", "bplcon1", "bplcon2", "color00",
    "ciaa", "ciab", "tod", "rtc", "uart", "fifo", "irq", "ipl", "nmi",
    "m68000", "m68k", "cck", "cck1", "cck2", "dma", "cpu", "bus", "alu",
    "bitplane", "bitplanes", "genlock", "ham", "ham6", "ham8", "lace",
    "lores", "hires", "shres", "pal", "ntsc", "kickstart", "workbench",
    "fastram", "chipram", "slowram", "autoconfig", "zorro", "rom", "ram",
    "byte", "word", "long", "prefetch", "substep", "microstep", "micro",
    # Common file extensions and format suffixes
    "py", "rs", "toml", "json", "yaml", "yml", "md", "txt", "html", "css", "js", "sh", "ps1", "bat", "lock",
    # Amiga custom chip registers (all chipsets)
    "bltcon0", "bltcon1", "bltafwm", "bltalwm", "bltcpt", "bltcptl", "bltcpth",
    "bltbpt", "bltbptl", "bltbpth", "bltapt", "bltaptl", "bltapth", "bltdpt", "bltdptl", "bltdpth",
    "bltsiz", "bltsizh", "bltsizv", "bltcon0l", "bltamod", "bltbmod", "bltcmod", "bltdmod",
    "bltadat", "bltbdat", "bltcdat", "dmaconr", "intenar", "intreqr",
    "cop1lch", "cop1lcl", "cop2lch", "cop2lcl", "copjmp1", "copjmp2", "copins",
    "diwstrt", "diwstop", "ddfstrt", "ddfstop", "vposw", "vhposw",
    "bplcon3", "bplcon4", "bpl1dat", "bpl2dat", "bpl3dat", "bpl4dat", "bpl5dat", "bpl6dat",
    "bpl1pt", "bpl1pth", "bpl1ptl", "bpl2pt", "bpl2pth", "bpl2ptl",
    "bpl3pt", "bpl3pth", "bpl3ptl", "bpl4pt", "bpl4pth", "bpl4ptl",
    "bpl5pt", "bpl5pth", "bpl5ptl", "bpl6pt", "bpl6pth", "bpl6ptl",
    "clxcon", "clxcon2", "clxdat",
    "dsksync", "dsksyn", "dskdatr", "dskdat", "dskpt", "dskpth", "dskptl", "dsklen", "dskbytr",
    "aud0lch", "aud0lcl", "aud0len", "aud0per", "aud0vol", "aud0dat",
    "aud1lch", "aud1lcl", "aud1len", "aud1per", "aud1vol", "aud1dat",
    "aud2lch", "aud2lcl", "aud2len", "aud2per", "aud2vol", "aud2dat",
    "aud3lch", "aud3lcl", "aud3len", "aud3per", "aud3vol", "aud3dat",
    "pot0dat", "pot1dat", "potinp", "potgo", "potgor",
    "serdat", "serdatr", "serper", "goto",
    # Common English words that could be flagged by sub-word heuristics
    "policy", "rules", "guidelines", "scratch", "tools", "crates", "tests",
    "buffer", "buffers", "cache", "offset", "pointer", "stack", "frame",
    "memory", "register", "instruction", "cycle", "cycles", "clock",
    "handle", "handler", "state", "target", "source", "dest", "entry",
    "cwik", "pasti", "winnicki", "kuba", "gurubook", "babel", "elowar", "aminet",
    "dpi", "gz", "zip", "tar", "lha", "adf", "rom", "json", "toml", "ps1", "".join(["test", "kit"])
}

# Curated set of Polish words (including ASCII-transliterated forms without ogonki)
# to catch short isolated Polish words that might otherwise be ambiguous
POLISH_VOCABULARY_ASCII = {
    # Verbs / Actions
    "czekaj", "przeczekac", "przeczekaj", "wywolaj", "pobierz", "ustaw", "zapisz",
    "odczytaj", "sprawdz", "dodaj", "usun", "znajdz", "zamknij", "otworz",
    "zresetuj", "uruchom", "zatrzymaj", "przetworz", "zmien", "wyczysc",
    # Substantives / Architecture
    "petla", "bufor", "pamiec", "rejestr", "przerwanie", "instrukcja", "wartosc",
    "adres", "odczyt", "zapis", "tablica", "modul", "klasa", "metoda", "funkcja",
    "zmienna", "kolejny", "krok", "szyna", "danych", "uklad", "uklady", "czesci",
    "urzadzenie", "urzadzenia", "wyspecjalizowane", "burza", "burze", "opoznienie",
    "opozniajaca", "plik", "pliku", "pliki", "plikow", "slowo", "slowa", "slow",
    "jezyk", "jezyka", "polski", "polskie", "polska", "polsku", "tlumaczenie",
    "komentarz", "kod", "kodzie", "zrodlo", "zrodla", "blad", "bledy", "wyjatek",
    # Conjunctions / Adverbs
    "oraz", "takze", "poniewaz", "dlatego", "zaraz", "wtedy", "kiedy", "gdzie",
    "ktory", "ktora", "ktore", "ktorych", "ktorym", "bardzo", "dobrze", "teraz",
    "zawsze", "nigdy", "jeszcze", "tylko", "tutaj", "tam"
}


def split_identifier(ident: str):
    """Split PascalCase and camelCase into constituent words, supporting Polish characters."""
    parts = re.findall(r"[A-ZĄĆĘŁŃÓŚŹŻ]?[a-ząćęłńóśźż]+|[A-ZĄĆĘŁŃÓŚŹŻ]+(?=[A-ZĄĆĘŁŃÓŚŹŻ][a-ząćęłńóśźż]|\b)", ident)
    return parts if parts else [ident]


def is_candidate_word(word: str) -> bool:
    """Check if a word should be evaluated for Polish language."""
    if len(word) < 3:
        return False
    # Ignore pure ALL_CAPS symbols (e.g. DMA, CIA, ADKCON, BFFFFF, CPU, DBNE)
    if word.isupper():
        return False
    # Ignore words with digits
    if re.search(r"\d", word):
        return False
    w_lower = word.lower()
    if w_lower in TECHNICAL_WHITELIST:
        return False
    return True


def detect_polish_in_text(text: str):
    """
    Detect Polish words or phrases in a given text.
    Returns list of tuples: (line_number, matched_item, reason, line_preview)
    """
    violations = []
    lines = text.splitlines()

    for line_no, line in enumerate(lines, 1):
        line_clean = line.strip()
        if not line_clean:
            continue

        # 1. Quoted phrase inspection (e.g. "przeczekać burzę", "pętla opóźniająca")
        quoted_matches = re.findall(r'["\']([^"\']{4,})["\']', line)
        for q in quoted_matches:
            # Skip if quoted string is all caps (assembly mnemonics like "DBNE") or file/glob paths
            if q.isupper() or q.startswith("*.") or re.match(r"^[\*a-zA-Z0-9_\-\./\\]+\.[a-zA-Z0-9]+$", q):
                continue
            tokens = re.findall(r"\b[a-zA-ZąćęłńóśźżĄĆĘŁŃÓŚŹŻ]+\b", q)
            non_en = [
                t for t in tokens
                if t.lower() not in ENGLISH_DICT and t.lower() not in TECHNICAL_WHITELIST and not t.isupper()
            ]
            if non_en:
                conf = DETECTOR.compute_language_confidence_values(q)
                pl = next((c.value for c in conf if c.language == Language.POLISH), 0)
                if pl >= 0.60:
                    violations.append((line_no, q, f"Polish phrase (confidence: {pl:.2f})", line_clean))
                    continue

        # 2. Token-level analysis (handles identifiers, camelCase, and words)
        raw_tokens = re.findall(r"\b[a-zA-ZąćęłńóśźżĄĆĘŁŃÓŚŹŻ]{3,}\b", line)
        sub_words = []
        for tok in raw_tokens:
            if "_" in tok:
                for part in tok.split("_"):
                    sub_words.extend(split_identifier(part))
            else:
                sub_words.extend(split_identifier(tok))

        for w in sub_words:
            if not is_candidate_word(w):
                continue
            w_lower = w.lower()

            # If it's a valid English word in the dictionary, skip it
            if w_lower in ENGLISH_DICT:
                continue

            # Check known unaccented Polish vocabulary
            if w_lower in POLISH_VOCABULARY_ASCII:
                violations.append((line_no, w, "Polish word (vocabulary match)", line_clean))
                continue

            # Run Lingua statistical model for words >= 4 letters
            if len(w) >= 4:
                conf = DETECTOR.compute_language_confidence_values(w)
                pl = next((c.value for c in conf if c.language == Language.POLISH), 0)
                top = conf[0]
                if (top.language == Language.POLISH and top.value >= 0.55) or pl >= 0.65:
                    violations.append((line_no, w, f"Polish word (lingua: {pl:.2f})", line_clean))

    # Deduplicate results per line & matched term
    seen = set()
    deduped = []
    for item in violations:
        key = (item[0], item[1])
        if key not in seen:
            seen.add(key)
            deduped.append(item)

    return deduped


def scan_file(file_path: Path):
    """Scan a single file and return list of violations."""
    try:
        content = file_path.read_text(encoding="utf-8", errors="ignore")
    except Exception as e:
        print(f"Error reading {file_path}: {e}", file=sys.stderr)
        return []
    return detect_polish_in_text(content)


def scan_staged_additions():
    """Checks git staged diffs for Polish language violations in newly added lines."""
    cmd = ["git", "diff", "--cached", "-U0"]
    res = subprocess.run(cmd, cwd=REPO_ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
    if res.returncode != 0:
        print("Git diff check failed", file=sys.stderr)
        return 1

    excluded_files = {
        "tools/harness/check_polish.py",
        ".agents/rules/language-policy.md",
        "DIARY.md"
    }

    current_file = None
    file_additions = {}

    for line in res.stdout.splitlines():
        if line.startswith("+++ b/"):
            current_file = line[6:].strip()
            continue
        if not current_file:
            continue
        normalized = current_file.replace("\\", "/")
        if normalized in excluded_files:
            continue
        # Only check text/source files
        suffix = Path(current_file).suffix.lower()
        if suffix not in {".rs", ".md", ".toml", ".py", ".sh", ".json", ".txt"}:
            continue

        if line.startswith("+") and not line.startswith("+++"):
            added_line = line[1:]
            file_additions.setdefault(current_file, []).append(added_line)

    total_violations = 0
    for f_str, added_lines in file_additions.items():
        diff_text = "\n".join(added_lines)
        violations = detect_polish_in_text(diff_text)
        if violations:
            total_violations += len(violations)
            print(f"\n[FAIL] Polish language violations in staged diff of {f_str}:")
            for line_no, word, reason, line_text in violations:
                print(f"  Line {line_no:4d}: [{word}] -> {reason}")
                print(f"            {line_text}")

    if total_violations > 0:
        print(f"\nTotal: {total_violations} violation(s) found across staged additions.")
        print("Per .agents/rules/language-policy.md, please translate all Polish text to English.")
        return 1

    print("[PASS] All staged additions comply with language-policy.md.")
    return 0


def main():
    parser = argparse.ArgumentParser(description="Scan explicit files or staged additions for Polish text.")
    parser.add_argument("files", nargs="*", type=Path, help="Files to scan (default: language-policy.md)")
    parser.add_argument("--staged", action="store_true", help="Scan added lines in the Git index")
    args = parser.parse_args()
    if args.staged:
        if args.files:
            parser.error("--staged cannot be combined with file paths")
        return scan_staged_additions()

    # CLI mode: process target files or default to language-policy.md
    targets = args.files or [DEFAULT_TARGET_FILE]

    all_violations = 0
    for target in targets:
        if not target.exists():
            print(f"Error: Target file not found: {target}", file=sys.stderr)
            all_violations += 1
            continue

        print(f">> Scanning file for Polish words: {target}")
        violations = scan_file(target)

        if violations:
            all_violations += len(violations)
            print(f"\n[VIOLATIONS DETECTED] Found {len(violations)} Polish occurrence(s) in {target.name}:")
            for line_no, word, reason, line in violations:
                print(f"  Line {line_no:4d}: [{word}] ({reason})")
                print(f"            \"{line}\"")
            print(f"\nPlease refer to .agents/rules/language-policy.md for translation rules.\n")
        else:
            print(f"[CLEAN] No Polish language violations detected in {target.name}.\n")

    return 1 if all_violations > 0 else 0


if __name__ == "__main__":
    sys.exit(main())
