(function () {
  const injectedAt = new Date().toISOString();

  window.startToolsPluginBridge = {
    injectedAt,
    ping(from) {
      return `pong from preload.js, called by ${from}, injected at ${injectedAt}`;
    },
  };

  console.log('[StartTools preload test] injected', {
    injectedAt,
    plugin: window.__STARTTOOLS_PLUGIN__,
  });
})();
