// Attach native listeners (synchronously, in the user-gesture stack) that
// create or resume the shared `__cookitAudioCtx`. Without this, any
// AudioContext created later — e.g. when a timer expires several minutes after
// the most recent click — stays `suspended` under Firefox/Chrome's autoplay
// policy and the beep produces no sound.
//
// Listeners are registered in capture phase so they run before any Dioxus
// handler can cancel propagation. Re-runs of the resume() call are cheap and
// required: AudioContexts can drop back to suspended (tab backgrounded, OS
// audio reconfigured, etc.), so every gesture re-primes.
(function() {
    if (window.__cookitAudioPrimerAttached) return;
    window.__cookitAudioPrimerAttached = true;
    const prime = () => {
        try {
            if (!window.__cookitAudioCtx) {
                const AC = window.AudioContext || window.webkitAudioContext;
                if (!AC) return;
                window.__cookitAudioCtx = new AC();
            }
            if (window.__cookitAudioCtx.state === 'suspended') {
                window.__cookitAudioCtx.resume();
            }
        } catch (e) {}
    };
    ['pointerdown', 'click', 'keydown', 'touchstart'].forEach(ev => {
        document.addEventListener(ev, prime, { capture: true });
    });
})();
