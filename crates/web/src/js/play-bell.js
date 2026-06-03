// Play the bell once through the shared `__cookitAudioCtx` that a user gesture
// primed (see audio-primer.js). Routing through that context — rather than a
// fresh `new Audio()` — keeps us on the audio path the browser already
// unlocked, so a bell fired minutes after the last click is still audible
// under Firefox/Chrome autoplay policy. The decoded buffer is cached on
// `window` so we only fetch/decode the mp3 once.
(async () => {
    try {
        const ctx = window.__cookitAudioCtx;
        if (!ctx) return;
        if (ctx.state === 'suspended') { await ctx.resume(); }
        if (!window.__cookitBellBuffer) {
            const resp = await fetch(BELL_URL);
            const data = await resp.arrayBuffer();
            window.__cookitBellBuffer = await ctx.decodeAudioData(data);
        }
        const src = ctx.createBufferSource();
        src.buffer = window.__cookitBellBuffer;
        src.connect(ctx.destination);
        src.start();
    } catch (e) {}
})();
