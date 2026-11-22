"""Replace an installed host-side capture snapshot under its saved Compose project."""

import argparse
from contextlib import ExitStack
from datetime import UTC, datetime
import json
from pathlib import Path
import re
import sys

from compose_service import (
    CAPTURE_SERVICES, PREVIOUS_LAYOUT_CONFIG, ComposeService, Deadline, copy_runtime_file,
    rename_without_overwrite, verify_archive,
)
from confirmation import STARTUP_SECONDS
from control import mutation_claim, status
from registry import CaptureError, Project, read_json, write_json


def replace_services(service, stage):
    since = datetime.now(UTC).isoformat()
    service.compose(stage, "up", "--detach", "--no-deps", "--force-recreate", *CAPTURE_SERVICES)
    return service.wait_ready(service.containers(), status, since)


def require_running(service):
    health = service.healthy(service.containers(), status)
    if not health:
        raise CaptureError("Receiver and Collector readiness: both installed services must be ready before upgrading.")
    return health


def restore_installed(installed, selected, *, merge_output=True):
    installed.deadline = selected.deadline = Deadline.start(STARTUP_SECONDS)
    installed.compose("Capture rollback configuration", "config", "--quiet")
    installed.containers(transition_revision=selected.config_id)
    if installed.control != selected.control and merge_output:
        selected.compose("Capture migration rollback stop", "stop", *CAPTURE_SERVICES)
        copy_spool(selected, installed)
    return replace_services(installed, "Capture rollback")


def copy_spool(source, destination):
    for path in source.spool.iterdir():
        copy_runtime_file(source.project, path, destination.spool / path.name)


def copy_receiver_state(installed, selected):
    # Credentials and historical files stay in the host-only previous layout.
    for path in installed.control.glob("lineage*.json"):
        copy_runtime_file(installed.project, path, selected.control / path.name)
    copy_spool(installed, selected)


def previous_layout_destination(service, config_id):
    if not isinstance(config_id, str) or not re.fullmatch(r"[0-9a-f]{64}", config_id):
        raise CaptureError("The previous layout revision is invalid. Keep the runtime directories.")
    project = service.project
    destination = project.plain_path(service.root / "previous-layout" / config_id / "receiver")
    project.require_ignored(destination)
    return destination


def retain_previous_layout(service, settings_path, settings):
    config_id = settings[PREVIOUS_LAYOUT_CONFIG]
    project = service.project
    source = project.plain_path(project.archive / "_receiver")
    destination = previous_layout_destination(service, config_id)
    try:
        if source == service.control or config_id == service.config_id:
            raise CaptureError("The previous runtime layout is still selected.")
        if read_json(settings_path) != settings:
            raise CaptureError("The saved settings changed before retaining the previous layout.")
        if source.exists():
            if not source.is_dir():
                raise CaptureError("The previous runtime layout is not a directory.")
            destination.parent.mkdir(parents=True, exist_ok=True)
            if destination.exists():
                raise CaptureError("The previous-layout destination exists. Keep both runtime directories.")
            rename_without_overwrite(source, destination)
        elif not destination.is_dir():
            raise CaptureError("The previous runtime directory and its retained copy are missing.")
        write_json(settings_path, {key: value for key, value in settings.items() if key != PREVIOUS_LAYOUT_CONFIG})
    except (OSError, ValueError) as error:
        raise CaptureError(f"Previous layout retention is incomplete: {error}") from error
    return {"previous_config_id": config_id, "previous_layout": destination.relative_to(project.primary).as_posix()}


def upgrade(args, cwd):
    project = Project(args.project, cwd)
    project.prepare()
    verify_archive(project)
    with ExitStack() as selected_claims, ExitStack() as installed_locks:
        installed_locks.enter_context(project.launcher_lock())
        deadline = Deadline.start(STARTUP_SECONDS)
        installed = ComposeService(project, args, deadline, revision="saved")
        selected = ComposeService(project, args, deadline, revision="upgrade")
        if installed.control != project.control:
            raise CaptureError("The capture runtime layout changed while acquiring the launcher lock. Retry the upgrade.")
        if installed.settings != selected.settings:
            raise CaptureError("Compose identity: the saved settings changed while validating the revisions.")
        installed_locks.enter_context(mutation_claim(project))
        if any(record["end_utc"] is None for _, record in project.registrations()):
            raise CaptureError("Close all capture registrations before changing the installed revision.")

        settings_path = installed.root / "compose-settings.json"
        project.require_ignored(settings_path)
        legacy = project.plain_path(project.archive / "_receiver")
        pending_retention = PREVIOUS_LAYOUT_CONFIG in installed.settings
        if pending_retention and installed.config_id != selected.config_id:
            raise CaptureError("Previous layout retention is incomplete. Retry from the installed revision's checkout.")
        if installed.control != legacy and legacy.exists() and not pending_retention:
            raise CaptureError("Previous layout retention is incomplete and the previous revision is unknown. "
                               "Keep both runtime directories.")
        installed.wait_engine()
        if installed.config_id == selected.config_id:
            report = {"status": "already_current", **require_running(installed)}
            if pending_retention:
                report.update(retain_previous_layout(installed, settings_path, installed.settings))
            return report

        if args.rollback:
            return {"status": "restored", **restore_installed(installed, selected)}

        require_running(installed)
        selected.persist(cwd)
        relocating = installed.control != selected.control
        if relocating:
            if installed.control != legacy:
                raise CaptureError("Unexpected previous runtime layout. Keep the source directory.")
            previous = previous_layout_destination(selected, installed.config_id)
            if previous.exists():
                raise CaptureError("The previous-layout destination exists. Keep both runtime directories.")
            selected_claims.enter_context(mutation_claim(project, control=selected.control))
        selected.compose("Capture upgrade configuration", "config", "--quiet")
        require_running(installed)
        updated = {**installed.settings, "config_id": selected.config_id}
        if relocating:
            updated[PREVIOUS_LAYOUT_CONFIG] = installed.config_id
        if read_json(settings_path) != installed.settings:
            raise CaptureError("Compose identity: the saved settings changed before the upgrade.")
        selected_started = False
        try:
            if relocating:
                installed.compose("Capture migration stop", "stop", *CAPTURE_SERVICES)
                copy_receiver_state(installed, selected)
            selected_started = True
            health = replace_services(selected, "Capture upgrade")
            write_json(settings_path, updated)
        except (Exception, KeyboardInterrupt) as error:
            reason = str(error) or "operator interruption"
            try:
                # The recovery window is independent of time spent attempting the upgrade.
                restore_installed(installed, selected, merge_output=selected_started)
                settings = read_json(settings_path)
                if settings == updated:
                    write_json(settings_path, installed.settings)
                elif settings != installed.settings:
                    raise CaptureError("The saved settings changed during the upgrade.")
            except (Exception, KeyboardInterrupt) as rollback_error:
                detail = str(rollback_error) or "operator interruption"
                raise CaptureError(f"Capture upgrade failed ({reason}). Rollback failed ({detail}). "
                                   "Keep both snapshots and inspect the selected capture deployment.") from rollback_error
            raise CaptureError(f"Capture upgrade failed ({reason}). The installed revision was restored.") from error
        report = {"status": "upgraded", "previous_config_id": installed.config_id, **health}
        if relocating:
            # Windows will not rename a directory containing the open launcher
            # lock. Keep the selected runtime claimed across the move.
            installed_locks.close()
            report.update(retain_previous_layout(selected, settings_path, updated))
        return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", required=True, type=Path, help="Primary checkout root.")
    parser.add_argument("--rollback", action="store_true",
                        help="Restore the saved revision after an interrupted cutover from this source.")
    try:
        report = upgrade(parser.parse_args(), Path.cwd())
    except (OSError, KeyError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
