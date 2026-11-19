"""Replace an installed host-side capture snapshot under its saved Compose project."""

import argparse
from datetime import UTC, datetime
import json
from pathlib import Path
import sys

from compose_service import CAPTURE_SERVICES, ComposeService, Deadline, verify_archive
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


def restore_installed(installed, selected):
    installed.deadline = Deadline.start(STARTUP_SECONDS)
    installed.compose("Capture rollback configuration", "config", "--quiet")
    installed.containers(transition_revision=selected.config_id)
    return replace_services(installed, "Capture rollback")


def upgrade(args, cwd):
    project = Project(args.project, cwd)
    project.prepare()
    verify_archive(project)
    with project.launcher_lock(), mutation_claim(project):
        if any(record["end_utc"] is None for _, record in project.registrations()):
            raise CaptureError("Close all capture registrations before changing the installed revision.")

        deadline = Deadline.start(STARTUP_SECONDS)
        installed = ComposeService(project, args, deadline, revision="saved")
        selected = ComposeService(project, args, deadline, revision="upgrade")
        if installed.settings != selected.settings:
            raise CaptureError("Compose identity: the saved settings changed while validating the revisions.")
        installed.wait_engine()
        if installed.config_id == selected.config_id:
            return {"status": "already_current", **require_running(installed)}

        if args.rollback:
            return {"status": "restored", **restore_installed(installed, selected)}

        require_running(installed)
        selected.persist_code()
        selected.compose("Capture upgrade configuration", "config", "--quiet")
        require_running(installed)
        settings_path = installed.root / "compose-settings.json"
        project.require_ignored(settings_path)
        updated = {**installed.settings, "config_id": selected.config_id}
        if read_json(settings_path) != installed.settings:
            raise CaptureError("Compose identity: the saved settings changed before the upgrade.")
        try:
            health = replace_services(selected, "Capture upgrade")
            write_json(settings_path, updated)
        except (Exception, KeyboardInterrupt) as error:
            reason = str(error) or "operator interruption"
            try:
                # The recovery window is independent of time spent attempting the upgrade.
                restore_installed(installed, selected)
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
        return {"status": "upgraded", "previous_config_id": installed.config_id, **health}


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
