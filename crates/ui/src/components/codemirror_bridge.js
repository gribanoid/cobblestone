// SPDX-License-Identifier: MIT

const [id, options] = await dioxus.recv();
const root = document.getElementById(id);
if (!root) return null;

const channel = root.querySelector("textarea");
const surface = root.querySelector(".cb-codemirror-surface");
const editor = globalThis.CobblestoneCodeMirror.mountEditor(surface, channel.value, options, text => {
  channel.value = text;
  channel.dispatchEvent(new Event("input", { bubbles: true }));
});

try {
  while (root.isConnected) {
    const command = await dioxus.recv();
    if (command === "destroy" || !root.isConnected) break;
    if (command.configure) editor.configure(command.configure);
    if (command.insert) editor.insertLine(command.insert);
  }
} finally {
  editor.destroy();
}
return null;
