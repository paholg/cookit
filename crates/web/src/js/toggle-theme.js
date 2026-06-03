const root = document.documentElement;
const next = root.dataset.theme === 'dark' ? 'light' : 'dark';
root.dataset.theme = next;
try { localStorage.setItem('theme', next); } catch (e) {}
