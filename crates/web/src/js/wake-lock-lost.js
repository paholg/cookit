const lock = window.__cookitWakeLock;
if (!lock) return true;
await new Promise(resolve => lock.addEventListener('release', resolve));
if (window.__cookitWakeLock === lock) window.__cookitWakeLock = null;
return true;
