// Clickable [[wikilinks]] in Reading view → Rust opens the note.
(async function cobblestoneWikilinkNav() {
  let sending = Promise.resolve();

  document.addEventListener(
    "click",
    function (e) {
      const t = e.target;
      if (!t || !t.closest) return;
      const a = t.closest("a.cb-wikilink");
      if (!a) return;
      const slug = a.getAttribute("data-slug");
      if (!slug) return;
      e.preventDefault();
      e.stopPropagation();
      sending = sending.then(async function () {
        dioxus.send({ slug: slug });
        try {
          await dioxus.recv();
        } catch (_) {}
      });
    },
    true
  );

  // Keep the dioxus::document::eval channel alive.
  while (true) {
    await new Promise(function (r) {
      setTimeout(r, 60 * 60 * 1000);
    });
  }
})();
