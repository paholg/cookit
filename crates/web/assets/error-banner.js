// Renders uncaught JS errors and unhandled promise rejections as a banner
// pinned to the bottom of the page. Without this, Dioxus interpreter errors
// (e.g. "node is null" during a bad diff) are invisible unless devtools are
// open and the site keeps "working" while the DOM is silently corrupted.
(function () {
  var STYLE =
    "position:fixed;bottom:0;left:0;right:0;background:#7f1d1d;color:#fff;" +
    "padding:0.75em 2.5em 0.75em 1em;font-family:ui-monospace,SFMono-Regular,Menlo,monospace;" +
    "font-size:13px;z-index:99999;white-space:pre-wrap;overflow:auto;max-height:50vh;" +
    "border-top:3px solid #fca5a5;";

  var BTN_STYLE =
    "position:absolute;top:4px;right:8px;background:transparent;color:#fff;" +
    "border:0;font-size:20px;line-height:1;cursor:pointer;padding:4px 8px;";

  var messages = [];

  function ensureBanner() {
    var el = document.getElementById("__cookit_error_banner__");
    if (el) return el;
    el = document.createElement("div");
    el.id = "__cookit_error_banner__";
    el.style.cssText = STYLE;

    var close = document.createElement("button");
    close.textContent = "×";
    close.setAttribute("aria-label", "Dismiss");
    close.style.cssText = BTN_STYLE;
    close.onclick = function () {
      el.remove();
      messages = [];
    };
    el.appendChild(close);

    var pre = document.createElement("pre");
    pre.id = "__cookit_error_banner_content__";
    pre.style.cssText = "margin:0;white-space:pre-wrap;";
    el.appendChild(pre);
    document.body.appendChild(el);
    return el;
  }

  function render() {
    if (!document.body) {
      document.addEventListener("DOMContentLoaded", render, { once: true });
      return;
    }
    ensureBanner();
    document.getElementById("__cookit_error_banner_content__").textContent =
      messages.join("\n\n");
  }

  function record(label, msg) {
    messages.push("[" + label + "] " + msg);
    // Cap so a tight error loop doesn't grow unbounded.
    if (messages.length > 10) messages.shift();
    render();
  }

  window.addEventListener("error", function (e) {
    var m = e.message || (e.error && e.error.message) || "error";
    var s = e.error && e.error.stack ? "\n" + e.error.stack : "";
    record("error", m + s);
  });

  window.addEventListener("unhandledrejection", function (e) {
    var r = e.reason;
    var msg =
      r && r.stack ? r.stack : r && r.message ? r.message : String(r);
    record("unhandledrejection", msg);
  });
})();
