if (window.__cookitBeepTimeout) {
    clearTimeout(window.__cookitBeepTimeout);
    window.__cookitBeepTimeout = null;
}
if (window.__cookitBeep) {
    try { window.__cookitBeep.osc.stop(); } catch (e) {}
    try { window.__cookitBeep.gain.disconnect(); } catch (e) {}
    window.__cookitBeep = null;
}
