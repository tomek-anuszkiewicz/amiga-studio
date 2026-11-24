# Capture test inputs

Each capture test that needs filesystem inputs restores its own starting tree
from [capture-repositories.zip](capture-repositories.zip) using its unittest ID.
The archive also retains the four shared Git starting trees. Case descriptions
record which directories must exist even when empty, which values depend on the
temporary root, and which files share bytes. The test runner restores these
inputs in each worker. Setup that creates a scenario's starting state belongs
in the generator. Keep behavior that the test observes, including later file
writes and retries, in the test.

Run the generator from the repository root when changing a case's starting
state, a published capture service file, the service file list, or the root
ignore rules. Use an output path to write a separate archive for comparison.

~~~text
python tools/workflow/capture/tests/fixtures/build_capture_repositories.py
python tools/workflow/capture/tests/fixtures/build_capture_repositories.py <output.zip>
~~~

The generator uses disposable Git trees and synthetic credentials. To add a
fixture, identify the last filesystem state before the observed behavior and
update the recipe for that unittest method. Preserve binary files as bytes.
Declare the path values that need to follow the test directory, and update any
dependent digests only after those paths are known.
