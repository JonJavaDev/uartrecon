// app.js - script ringan untuk halaman starter.
(function () {
  "use strict";

  function pad(n) {
    return n < 10 ? "0" + n : "" + n;
  }

  function tick() {
    var el = document.getElementById("clock");
    if (!el) return;
    var d = new Date();
    el.textContent =
      pad(d.getHours()) + ":" + pad(d.getMinutes()) + ":" + pad(d.getSeconds());
  }

  function onLoad() {
    var el = document.getElementById("loaded");
    if (el) {
      el.textContent = new Date().toLocaleString();
    }
    tick();
    setInterval(tick, 1000);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", onLoad);
  } else {
    onLoad();
  }
})();
