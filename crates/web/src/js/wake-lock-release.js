const lock = window.__cookitWakeLock;
if (lock) {
    window.__cookitWakeLock = null;
    try { await lock.release(); } catch (e) {}
}
