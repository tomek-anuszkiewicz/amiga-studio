"""Read optional Docker and Qdrant metadata from an already running daemon."""

from collections import defaultdict
import json
import platform
import re
import shutil
import sys

from .common import DATA_ERRORS, ProbeError, triplet, version_text


DOCKER_RELEASES = "https://docs.docker.com/desktop/release-notes.md"
QDRANT = "qdrant/qdrant"
RAG_SETUP = ".agents/references/developer-setup.md#domain-hardware-rag-setup-rag"
DIGEST = re.compile(r"sha256:[a-f0-9]{64}")


def canonical_qdrant(value):
    name = re.sub(r"^(?:(?:index\.)?docker\.io/)?", "", value, count=1)
    if re.fullmatch(r"qdrant/qdrant(?::[\w][\w.-]{0,127}|@sha256:[a-f0-9]{64})?", name):
        return name if name != QDRANT else f"{name}:latest"
    return None


def qdrant_targets(probe):
    seen = defaultdict(set)
    containers = probe.run(["docker", "ps", "--format", "{{json .}}"])
    for line in containers.splitlines():
        container = json.loads(line)
        image = canonical_qdrant(container["Image"])
        if image:
            container_id = container["ID"]
            if not re.fullmatch(r"[a-f0-9]{12,64}", container_id):
                raise ProbeError("Docker returned an invalid container identifier")
            image_id = probe.run(["docker", "container", "inspect", "--format", "{{.Image}}", container_id])
            seen[image_id].add((image, "running container"))
    images = probe.run(["docker", "image", "ls", "--format", "{{json .}}", QDRANT])
    for line in images.splitlines():
        row = json.loads(line)
        image = canonical_qdrant(f'{row["Repository"]}:{row["Tag"]}')
        if image:
            image_id = probe.run(["docker", "image", "inspect", "--format", "{{.Id}}", image])
            seen[image_id].add((image, "local tag"))
    return seen


def qdrant_upstream(probe, ref, info):
    def manifest(reference):
        return json.loads(probe.run(["docker", "manifest", "inspect", reference], timeout=16))

    remote = manifest(ref)
    if "manifests" in remote:
        candidates = [
            row["digest"] for row in remote["manifests"] if row.get("platform", {}).get("os") == info["Os"]
            and row["platform"].get("architecture") == info["Architecture"]
            and (not info.get("Variant") or row["platform"].get("variant") == info["Variant"])
        ]
        if len(candidates) != 1 or not DIGEST.fullmatch(candidates[0]):
            raise ProbeError("the official image tag does not identify one matching platform image")
        remote = manifest(f"{QDRANT}@{candidates[0]}")
    latest = remote["config"]["digest"]
    if not DIGEST.fullmatch(latest):
        raise ProbeError("the official image manifest did not provide a valid configuration digest")
    return latest


def check_qdrant(area, probe):
    source = "Docker Hub qdrant/qdrant image manifests"
    targets = qdrant_targets(probe)
    if not targets:
        area.record("optional missing", "Qdrant Docker image", observed="no official local tag or running image",
                    source=RAG_SETUP, detail="shared RAG service")
    cache = {}
    for image_id, refs in sorted(targets.items()):
        if not DIGEST.fullmatch(image_id):
            area.incomplete("Qdrant image", source, "Docker returned an invalid local image identifier")
            continue
        try:
            info = json.loads(probe.run(["docker", "image", "inspect", "--format", "{{json .}}", image_id]))
            if info["Id"] != image_id:
                raise ProbeError("Docker returned a different local image identifier")
        except DATA_ERRORS as error:
            area.incomplete("Qdrant image", source,
                            error if isinstance(error, ProbeError) else "invalid Docker image metadata",
                            observed=image_id)
            continue
        for ref, local in sorted(refs):
            label = ref.removeprefix("qdrant/qdrant")
            if label.startswith("@sha256:"):
                label = "@" + label[8:20]
            item = f"Qdrant{label} ({local})"
            try:
                key = ref, info["Os"], info["Architecture"], info.get("Variant")
                if key not in cache:
                    try:
                        cache[key] = qdrant_upstream(probe, ref, info)
                    except DATA_ERRORS as error:
                        cache[key] = (error if isinstance(error, ProbeError)
                                      else ProbeError("invalid Qdrant registry manifest"))
                latest = cache[key]
                if isinstance(latest, ProbeError):
                    raise latest
                if latest == image_id:
                    area.record("current", item)
                else:
                    area.record("drift" if "@" in ref else "update", item, observed=image_id, declared=ref,
                                candidate=latest, source=source)
            except DATA_ERRORS as error:
                area.incomplete(item, source,
                                error if isinstance(error, ProbeError) else "invalid image platform data",
                                observed=image_id, declared=ref)


def docker_desktop_releases(page, system, architecture):
    headings = re.split(r"(?m)^## (\d+\.\d+\.\d+)\s*$", page)
    download_prefix = f"https://desktop.docker.com/{system}/main/{architecture}/"
    candidates = [triplet(version) for version, body in zip(headings[1::2], headings[2::2])
                  if download_prefix in body.split("\n### ", 1)[0]]
    if not candidates:
        raise ProbeError("Docker Desktop release notes did not identify this platform and architecture")
    return max(candidates)


def check_docker(area, probe, _tracked):
    if shutil.which("docker") is None:
        area.record("optional missing", "Docker", observed="not on PATH", source=RAG_SETUP,
                    detail="shared RAG service")
        return
    try:
        probe.run(["docker", "--version"])
        data = json.loads(probe.run(["docker", "version", "--format", "{{json .}}"]))
        server = data["Server"]
        server_version = server["Version"]

        endpoint = probe.run(["docker", "context", "inspect", "--format", "{{.Endpoints.docker.Host}}"])
        if not endpoint.startswith(("npipe://", "unix://")):
            raise ProbeError("the selected Docker context does not identify a local daemon")
    except DATA_ERRORS as error:
        area.incomplete("Docker daemon and Qdrant", RAG_SETUP,
                        error if isinstance(error, ProbeError) else "Docker did not return usable daemon metadata")
        return
    installed = None
    try:
        distribution = server.get("Platform", {}).get("Name", "")
        if "Docker Desktop" not in distribution:
            raise ProbeError("the Docker engine is managed through an unidentified distribution channel")
        installed = triplet(distribution)
        system = {"win32": "win", "darwin": "mac", "linux": "linux"}.get(sys.platform)
        architecture = {"amd64": "amd64", "x86_64": "amd64", "arm64": "arm64",
                        "aarch64": "arm64"}.get(platform.machine().lower())
        if not system or not architecture:
            raise ProbeError("the Docker Desktop host platform or architecture could not be identified")
        page = probe.http(DOCKER_RELEASES, "text/plain")
        area.compare("Docker Desktop", installed, docker_desktop_releases(page, system, architecture), DOCKER_RELEASES)
    except DATA_ERRORS as error:
        area.incomplete("Docker distribution", DOCKER_RELEASES,
                        error if isinstance(error, ProbeError) else "invalid Docker release metadata",
                        observed=version_text(installed) if installed is not None else server_version)
    try:
        check_qdrant(area, probe)
    except DATA_ERRORS as error:
        area.incomplete("Qdrant Docker image", RAG_SETUP,
                        error if isinstance(error, ProbeError) else "invalid Docker container or image metadata")
