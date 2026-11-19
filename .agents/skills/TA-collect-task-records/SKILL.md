---
name: TA-collect-task-records
description: Preserve selected completed files in the private execution directory, or run explicitly selected archive analysis.
---

# Preserve selected task records

Apply the [invocation contract](../../references/workflow-contract.md).
Executor: main session. Use the [ordinary copy procedure](../../../tools/workflow/capture/README.md#preserve-selected-files-at-task-closure).

For ordinary closure, identify the known selected files, the execution directory
and the surviving publication record after the final project graph refresh.
Run `records.py preserve` for those inputs. Keep active external producers and
required source paths available. Reuse files already present in the execution
directory. Report the observed outcome and any blocked path directly. Use the
existing publication control record for closure state. Stop after copying.

For separately selected archive analysis, use
[`records.py collect`](../../../tools/workflow/capture/README.md#collect-and-assess)
on the closed registration. Preserve the original command's revision, status,
input identities and recorded gaps. Return an `execution_archive` result and
use `manifest_ref` for its verified manifest locator and digest. If the manifest
lives in the primary private archive, preserve a byte-identical verified copy
beside the selected analysis result. Assessment is a separate operation.
