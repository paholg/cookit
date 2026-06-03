requestAnimationFrame(() => {
    const h = window.location.hash;
    if (!h) return;
    try {
        const el = document.querySelector(h);
        if (el) el.scrollIntoView({ block: 'start' });
    } catch (e) {}
});
