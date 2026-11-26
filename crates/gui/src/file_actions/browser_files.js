export function pick_file(kind, title, extensions, multiple) {
    if (typeof window.showOpenFilePicker === "function") {
        const options = {
            id: `amiga-${kind}`,
            multiple,
            types: [{ description: title, accept: { "application/octet-stream": extensions } }],
        };
        // Call before any await so the browser still has the user's activation.
        return window.showOpenFilePicker(options).then(async handles => {
            if (multiple) return handles;
            return [await handles[0].getFile()];
        }, error => {
            if (error.name === "AbortError") return null;
            throw error;
        });
    }

    return new Promise(resolve => {
        const input = document.createElement("input");
        input.type = "file";
        input.title = title;
        input.hidden = true;
        input.accept = extensions.join(",");
        input.multiple = multiple;
        const finish = result => {
            input.onchange = null;
            input.oncancel = null;
            input.remove();
            resolve(result);
        };
        input.onchange = () => {
            const files = Array.from(input.files);
            finish(files.length === 0 ? null : files);
        };
        input.oncancel = () => finish(null);
        document.body.appendChild(input);
        input.click();
    });
}
