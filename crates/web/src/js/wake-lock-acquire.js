try {
    if (window.__cookitWakeLock) {
        try { await window.__cookitWakeLock.release(); } catch (e) {}
        window.__cookitWakeLock = null;
    }
    window.__cookitWakeLock = await navigator.wakeLock.request('screen');
    return true;
} catch (e) {
    return false;
}
