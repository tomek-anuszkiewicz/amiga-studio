This task describes only the element identified by `target_id`. The selected
element and the `context_ids` form the input group. Apply
the selected image or table prompt's description guidance to that group.
Store the result in the selected element's `description` field; the renderer
places it after the element in `<details>` only after a description has been
saved.
Do not infer that other adjacent captions, notes or legends belong to it.
If the selected group does not provide enough evidence, finish with that
limitation.

Read only the selected element and the listed context elements. For an image,
inspect its Working asset before describing it. You may inspect its PDF source
page when useful. Previously read elements remain available in `read_context`,
and the inspected Working image remains attached on later turns; do not request
either again when you have enough evidence. Then return one `patch` action that
adds or replaces only the selected element's `description` field at the element
path returned by `read`.
Put the complete fenced `text` block required by the image or table prompt in
that JSON patch value. The editor stores the text inside the fence and renders
the fence inside a collapsible `<details>` block below the selected element.
Do not include image/table markup in either description. Do not change
the original source content, asset, table, or another element. The editor
finishes this task automatically after the first successful description patch.
