// vault-style image paste + drag-and-drop into the markdown editor.
// JS → { filename, data_base64 } → Rust saves to attachments/ → appends ![[name]].
(async function cobblestoneAttachImage() {
  function pad2(n) {
    return String(n).padStart(2, "0");
  }

  function stamp() {
    const d = new Date();
    return (
      d.getFullYear() +
      pad2(d.getMonth() + 1) +
      pad2(d.getDate()) +
      pad2(d.getHours()) +
      pad2(d.getMinutes()) +
      pad2(d.getSeconds())
    );
  }

  function extFromMime(mime) {
    if (mime === "image/jpeg") return "jpg";
    if (mime === "image/svg+xml") return "svg";
    if (mime && mime.indexOf("image/") === 0) {
      return mime.slice(6).split("+")[0] || "png";
    }
    return "png";
  }

  function pastedImageFilename(mime) {
    return "Pasted image " + stamp() + "." + extFromMime(mime);
  }

  /** Keep original name for file drops (vault-style); sanitize lightly. */
  function droppedImageFilename(file) {
    let name = (file && file.name) || "";
    name = name.replace(/[\\/]/g, "-").trim();
    if (!name) {
      return pastedImageFilename(file.type || "image/png");
    }
    // Ensure it has an image extension.
    if (!/\.(png|jpe?g|gif|webp|svg|avif)$/i.test(name)) {
      name = name + "." + extFromMime(file.type || "image/png");
    }
    return name;
  }

  function isImageFile(file) {
    if (!file) return false;
    if (file.type && file.type.indexOf("image/") === 0) return true;
    return /\.(png|jpe?g|gif|webp|svg|avif)$/i.test(file.name || "");
  }

  function isFileDrag(e) {
    const dt = e.dataTransfer;
    if (!dt || !dt.types) return false;
    return Array.prototype.indexOf.call(dt.types, "Files") >= 0;
  }

  /** Editor surface: edit host or whole note pane (drops on the note pane). */
  function editorHostFromEvent(e) {
    const t = e.target;
    if (!t || !t.closest) return null;
    return (
      t.closest(".cb-md-host") ||
      t.closest(".editor-wrap") ||
      t.closest("#editor-area")
    );
  }

  /** Sidebar drop target: folder path, "" for the vault root, or null. */
  function treeFolderFromEvent(e) {
    const t = e.target;
    if (!t || !t.closest) return null;
    const wrap = t.closest("[data-drop-folder]");
    if (wrap) return { el: wrap.querySelector(".tree-folder") || wrap, folder: wrap.getAttribute("data-drop-folder") || "" };
    const tree = t.closest(".vault-tree");
    if (tree) return { el: tree, folder: "" };
    return null;
  }

  function markDragOver(el) {
    if (!el) return;
    el.classList.add("cb-md-drag-over");
    const host = el.classList.contains("cb-md-host")
      ? el
      : el.querySelector(".cb-md-host");
    if (host) host.classList.add("cb-md-drag-over");
  }

  function clearDragOverClasses() {
    document
      .querySelectorAll(".cb-md-drag-over, .cb-file-drop-over")
      .forEach(function (el) {
        el.classList.remove("cb-md-drag-over");
        el.classList.remove("cb-file-drop-over");
      });
  }

  async function blobToBase64(blob) {
    const buf = await blob.arrayBuffer();
    const bytes = new Uint8Array(buf);
    let binary = "";
    const chunk = 0x8000;
    for (let i = 0; i < bytes.length; i += chunk) {
      binary += String.fromCharCode.apply(null, bytes.subarray(i, i + chunk));
    }
    return btoa(binary);
  }

  /** `folder` (string) → save into that sidebar folder without embedding;
   *  omitted → save next to the open note and insert ![[…]] at the caret. */
  async function sendImageFile(file, filename, folder) {
    const data_base64 = await blobToBase64(file);
    const msg = { filename: filename, data_base64: data_base64 };
    if (typeof folder === "string") msg.folder = folder;
    dioxus.send(msg);
    await dioxus.recv();
  }

  async function attachFiles(fileList, folder) {
    const files = Array.prototype.slice.call(fileList || []).filter(isImageFile);
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const filename = droppedImageFilename(file);
      try {
        await sendImageFile(file, filename, folder);
      } catch (err) {
        console.error("cobblestone attach image failed", err);
        try {
          dioxus.send({ error: String(err) });
        } catch (_) {}
      }
    }
  }

  // ── Paste (clipboard image) ──────────────────────────────────────────────
  document.addEventListener(
    "paste",
    async function (e) {
      const inEditor = editorHostFromEvent(e);
      if (!inEditor) return;
      const items = e.clipboardData && e.clipboardData.items;
      if (!items) return;
      let imageItem = null;
      for (let i = 0; i < items.length; i++) {
        if (items[i].type && items[i].type.indexOf("image/") === 0) {
          imageItem = items[i];
          break;
        }
      }
      if (!imageItem) return;
      e.preventDefault();
      e.stopPropagation();
      const file = imageItem.getAsFile();
      if (!file) return;
      try {
        await sendImageFile(file, pastedImageFilename(file.type || "image/png"));
      } catch (err) {
        console.error("cobblestone paste image failed", err);
        try {
          dioxus.send({ error: String(err) });
        } catch (_) {}
      }
    },
    true,
  );

  // ── Drag & drop files onto the editor (Finder / Explorer / browser) ──────
  let dragDepth = 0;

  function clearDragOver() {
    dragDepth = 0;
    clearDragOverClasses();
  }

  // ── Drop files onto a sidebar folder → saved into that folder ────────────
  document.addEventListener(
    "dragover",
    function (e) {
      if (!isFileDrag(e)) return;
      const target = treeFolderFromEvent(e);
      if (!target) {
        document.querySelectorAll(".cb-file-drop-over").forEach(function (el) {
          el.classList.remove("cb-file-drop-over");
        });
        return;
      }
      e.preventDefault();
      e.stopPropagation();
      if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
      document.querySelectorAll(".cb-file-drop-over").forEach(function (el) {
        if (el !== target.el) el.classList.remove("cb-file-drop-over");
      });
      target.el.classList.add("cb-file-drop-over");
    },
    true,
  );

  document.addEventListener(
    "drop",
    async function (e) {
      if (!isFileDrag(e)) return;
      const target = treeFolderFromEvent(e);
      if (!target) return;
      e.preventDefault();
      e.stopPropagation();
      clearDragOver();
      const files = e.dataTransfer && e.dataTransfer.files;
      if (!files || !files.length) return;
      await attachFiles(files, target.folder);
    },
    true,
  );

  document.addEventListener(
    "dragenter",
    function (e) {
      if (!isFileDrag(e)) return;
      const host = editorHostFromEvent(e);
      if (!host) return;
      e.preventDefault();
      dragDepth += 1;
      markDragOver(host);
    },
    true,
  );

  document.addEventListener(
    "dragover",
    function (e) {
      if (!isFileDrag(e)) return;
      const host = editorHostFromEvent(e);
      if (!host) return;
      e.preventDefault();
      e.stopPropagation();
      if (e.dataTransfer) {
        e.dataTransfer.dropEffect = "copy";
      }
      markDragOver(host);
    },
    true,
  );

  document.addEventListener(
    "dragleave",
    function (e) {
      if (!isFileDrag(e)) return;
      const host = editorHostFromEvent(e);
      if (!host) return;
      dragDepth = Math.max(0, dragDepth - 1);
      const related = e.relatedTarget;
      if (related && host.contains(related)) return;
      if (dragDepth === 0) {
        clearDragOverClasses();
      }
    },
    true,
  );

  document.addEventListener(
    "drop",
    async function (e) {
      if (!isFileDrag(e)) return;
      const host = editorHostFromEvent(e);
      clearDragOver();
      if (!host) return;
      e.preventDefault();
      e.stopPropagation();
      const files = e.dataTransfer && e.dataTransfer.files;
      if (!files || !files.length) return;
      await attachFiles(files);
    },
    true,
  );

  document.addEventListener("dragend", clearDragOver, true);

  while (true) {
    await new Promise(function (r) {
      setTimeout(r, 60 * 60 * 1000);
    });
  }
})();
