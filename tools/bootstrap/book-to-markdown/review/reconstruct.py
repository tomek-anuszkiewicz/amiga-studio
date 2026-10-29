"""Rebuild local review inputs from an accepted book recipe and its source PDF."""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

from PIL import Image

from .model import elements, export_markdown, load_input
from common.pdf_config import load_config, preparation_settings


def file_sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def import_accepted_book(root):
    root = root.resolve()
    source = root / 'source.pdf'
    recipe = root / 'book.json'
    if not source.is_file() or not recipe.is_file():
        raise ValueError('Reconstruction requires source.pdf and book.json')
    for name in ('working', 'accepted', 'pdf/pages'):
        if (root / name).exists():
            raise ValueError(f'Review output already exists: {name}')
    metadata = root / 'source.json'
    if metadata.is_file():
        expected_source = json.loads(metadata.read_text(encoding='utf-8'))['sha256']
        if file_sha256(source) != expected_source:
            raise ValueError('source.pdf does not match source.json')
    book = json.loads(recipe.read_text(encoding='utf-8'))
    for element in elements(book):
        asset = element.get('asset')
        if asset and (asset != f"assets/{element['id']}.png" or len(element['pdf_regions']) != 1):
            raise ValueError(f"Cannot reconstruct edited image asset for {element['id']}")
    expected_ocr = root / 'source-ocr.json'
    previous_ocr = expected_ocr.read_bytes() if expected_ocr.is_file() else None
    if previous_ocr is not None:
        ocr_record = json.loads(previous_ocr)
        if ocr_record['source_sha256'] != file_sha256(source):
            raise ValueError('source-ocr.json belongs to a different source PDF')

    pipeline = Path(__file__).resolve().parents[2] / 'pdf-to-markdown/pipeline.py'
    try:
        subprocess.run([sys.executable, str(pipeline), '--pdf', str(source),
                        '--from-stage', '01', '--to-stage', '02'], check=True)
    finally:
        if previous_ocr is not None and expected_ocr.is_file():
            generated = json.loads(expected_ocr.read_text(encoding='utf-8'))
            expected_ocr.write_bytes(previous_ocr)
    if previous_ocr is not None:
        active_profile = preparation_settings(load_config(root / 'pdf-to-markdown/config.yaml'))
        if (active_profile != ocr_record['profile']
                or generated['profile'] != ocr_record['profile']
                or generated['ocr_pages'] != ocr_record['ocr_pages']):
            raise ValueError('OCR profile or page selection differs from source-ocr.json')
        if file_sha256(root / 'source-ocr.pdf') != ocr_record['prepared_sha256']:
            print('OCR PDF bytes differ from the authored preparation; checking book references')

    prepared = root / 'pdf-to-markdown/02_preprocess'
    working = root / 'working'
    pages = root / 'pdf/pages'
    working.mkdir()
    try:
        pages.mkdir(parents=True)
        for page in sorted(prepared.glob('page_*.png')):
            shutil.copy2(page, pages / page.name)
        for element in elements(book):
            asset = element.get('asset')
            if not asset:
                continue
            region = element['pdf_regions'][0]
            with Image.open(pages / f"page_{region['page']:04d}.png") as image:
                box = region['bbox']
                if len(box) != 4 or not (0 <= box[0] < box[2] <= image.width
                                         and 0 <= box[1] < box[3] <= image.height):
                    raise ValueError(f"Invalid PDF region for {element['id']}")
                destination = working / asset
                destination.parent.mkdir(parents=True, exist_ok=True)
                image.crop(tuple(box)).save(destination)
        shutil.copy2(recipe, working / 'book.json')
        loaded, _, _ = load_input(working)
        export_markdown(loaded, working)
    except BaseException:
        shutil.rmtree(working, ignore_errors=True)
        shutil.rmtree(pages, ignore_errors=True)
        raise
    return sum(len(chapter['elements']) for chapter in loaded['chapters'])
