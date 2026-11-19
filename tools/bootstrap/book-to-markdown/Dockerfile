FROM node:22.22.0-bookworm-slim@sha256:dd9d21971ec4395903fa6143c2b9267d048ae01ca6d3ea96f16cb30df6187d94 AS math
WORKDIR /assets
COPY book-to-markdown/package.json book-to-markdown/package-lock.json ./
RUN npm ci --ignore-scripts

FROM python:3.13-slim-bookworm@sha256:a5a7017efc5969e00ae2b91c8eb0631049a569652aa4ffc4501769049ae45c72
ENV PYTHONUNBUFFERED=1 PYTHONDONTWRITEBYTECODE=1 PLAYWRIGHT_BROWSERS_PATH=/opt/playwright
WORKDIR /app
COPY book-to-markdown/requirements.txt tools/bootstrap/book-to-markdown/requirements.txt
COPY conversion/requirements.txt tools/bootstrap/conversion/requirements.txt
COPY pdf-to-markdown/requirements.txt tools/bootstrap/pdf-to-markdown/requirements.txt
RUN pip install --no-cache-dir -r tools/bootstrap/book-to-markdown/requirements.txt \
        -r tools/bootstrap/pdf-to-markdown/requirements.txt \
    && python -m playwright install --with-deps chromium \
    && rm -rf /var/lib/apt/lists/*
COPY book-to-markdown/book_review.py book-to-markdown/container-entrypoint.py tools/bootstrap/book-to-markdown/
COPY book-to-markdown/review/ tools/bootstrap/book-to-markdown/review/
COPY book-to-markdown/prompts/ tools/bootstrap/book-to-markdown/prompts/
COPY conversion/*.py tools/bootstrap/conversion/
COPY pdf-to-markdown/common/ tools/bootstrap/pdf-to-markdown/common/
COPY --from=math /assets/node_modules tools/bootstrap/book-to-markdown/node_modules
ENTRYPOINT ["python", "/app/tools/bootstrap/book-to-markdown/container-entrypoint.py"]
