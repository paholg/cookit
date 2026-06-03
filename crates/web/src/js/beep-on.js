// Beep keeps running until every expired timer is silenced (or removed).
// Reuses the shared `__cookitAudioCtx` that the Start-timer click primed (see
// audio-primer.js) — creating a fresh AudioContext here would be subject to
// autoplay policy and produce silence in Firefox.
try {
    const ctx = window.__cookitAudioCtx;
    if (ctx && !window.__cookitBeep) {
        if (ctx.state === 'suspended') { ctx.resume(); }
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.frequency.value = 400;
        gain.gain.value = 0.0;
        osc.connect(gain).connect(ctx.destination);
        osc.start();
        const tick = () => {
            const t = ctx.currentTime;
            gain.gain.cancelScheduledValues(t);
            gain.gain.setValueAtTime(0.25, t + 0.01);
            gain.gain.setValueAtTime(0.0, t + 0.26);
            window.__cookitBeepTimeout = setTimeout(tick, 500);
        };
        tick();
        window.__cookitBeep = { osc, gain };
    }
} catch (e) {}
