// Stand-in for any module the project does not have installed (plugins, presets…).
// Callable, chainable, and returns harmless values so configs keep loading.
function stub() { return proxy; }
const proxy = new Proxy(stub, {
  get(_t, key) {
    if (key === '__esModule') return false;
    if (key === 'default') return proxy;
    if (key === Symbol.toPrimitive) return () => '';
    if (key === 'handler') return () => {};
    if (key === 'config') return {};
    return proxy;
  },
  apply() { return proxy; },
});
module.exports = proxy;
