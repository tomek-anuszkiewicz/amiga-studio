// Host file references stay in IndexedDB; file contents are never retained here.
const RECENT_FILE_LIMIT = 5;
let history = { binary: [], rom: [] };
let database;
let restoration;
let pendingWrite = Promise.resolve();

function openDatabase() {
    if (!database) {
        database = new Promise((resolve, reject) => {
            const request = indexedDB.open("amiga-file-history", 1);
            request.onupgradeneeded = () => request.result.createObjectStore("history");
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
            request.onblocked = () => reject(new Error("Recent file storage is blocked by another tab."));
        });
    }
    return database;
}

export function restore_history() {
    if (!restoration) {
        restoration = openDatabase().then(db => new Promise((resolve, reject) => {
            const transaction = db.transaction("history", "readonly");
            const request = transaction.objectStore("history").get("files");
            transaction.oncomplete = () => {
                if (request.result) history = request.result;
                resolve();
            };
            transaction.onabort = () => reject(transaction.error);
            transaction.onerror = () => reject(transaction.error);
        }));
    }
    return restoration;
}

export function recent_names(kind) {
    return history[kind].map(entry => entry.name);
}

export function remember_file(kind, name, handle) {
    return update_history(async () => {
        const entries = history[kind];
        const retained = [];
        for (const entry of entries) {
            const same = handle && entry.handle
                ? await handle.isSameEntry(entry.handle)
                : !handle && !entry.handle && entry.name === name;
            if (!same) retained.push(entry);
        }
        history[kind] = [{ name, handle }, ...retained].slice(0, RECENT_FILE_LIMIT);
    });
}

function update_history(update) {
    const write = pendingWrite.then(async () => {
        await restore_history();
        await update();
        const db = await openDatabase();
        await new Promise((resolve, reject) => {
            const transaction = db.transaction("history", "readwrite");
            transaction.objectStore("history").put(history, "files");
            transaction.oncomplete = resolve;
            transaction.onabort = () => reject(transaction.error);
            transaction.onerror = () => reject(transaction.error);
        });
    });
    // A failed write must not prevent later attempts; its caller still receives the error.
    pendingWrite = write.catch(() => {});
    return write;
}

export function pick_file(kind, title, extensions, recentIndex) {
    if (typeof window.showOpenFilePicker === "function") {
        const options = {
            id: `amiga-${kind}`,
            multiple: false,
            types: [{ description: title, accept: { "application/octet-stream": extensions } }],
        };
        const entry = history[kind]?.[recentIndex];
        if (entry?.handle) options.startIn = entry.handle;
        // Check independently: a picker may silently ignore a deleted starting folder.
        // Denied access does not prove that a file has been deleted.
        const missing = entry?.handle
            ? entry.handle.getFile().then(() => false, error => error.name === "NotFoundError")
            : Promise.resolve(false);
        // Call before any await so the browser still has the user's activation.
        let missingStart = false;
        const selection = window.showOpenFilePicker(options).catch(error => {
            if (entry?.handle && error.name === "NotFoundError") {
                missingStart = true;
                delete options.startIn;
                return window.showOpenFilePicker(options);
            }
            throw error;
        });
        return Promise.all([
            selection.then(handles => ({ handles }), error => ({ error })), missing,
        ]).then(async ([outcome, unavailable]) => {
            if (unavailable || missingStart) {
                await update_history(() => {
                    history[kind] = history[kind].filter(candidate => candidate !== entry);
                });
                const category = kind === "binary" ? "binaries" : "ROMs";
                window.alert(`"${entry.name}" or its folder no longer exists. Removed from recent ${category}. If the folder is unavailable, the file picker uses the browser's default location.`);
            }
            if (outcome.error) {
                if (outcome.error.name === "AbortError") return null;
                throw outcome.error;
            }
            const handles = outcome.handles;
            const handle = handles[0];
            return [await handle.getFile(), handle];
        });
    }

    // File inputs cannot accept a starting directory; keep selection available.
    return new Promise(resolve => {
        const input = document.createElement("input");
        input.type = "file";
        input.title = title;
        input.hidden = true;
        input.accept = extensions.join(",");
        const finish = result => {
            input.onchange = null;
            input.oncancel = null;
            input.remove();
            resolve(result);
        };
        input.onchange = () => finish(input.files[0] ? [input.files[0], null] : null);
        input.oncancel = () => finish(null);
        document.body.appendChild(input);
        input.click();
    });
}
