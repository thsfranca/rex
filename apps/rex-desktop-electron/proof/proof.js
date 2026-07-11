(() => {
  const params = new URLSearchParams(window.location.search);
  const bury = params.get("bury") === "1";
  if (bury) {
    document.body.dataset.bury = "1";
  }

  const canvas = document.getElementById("webgl-layer");
  const gl = canvas.getContext("webgl", {
    alpha: false,
    antialias: false,
    preserveDrawingBuffer: true,
  });
  if (!gl) {
    document.body.dataset.webglReady = "0";
    document.body.dataset.webglError = "no-webgl";
    return;
  }

  // Distinct bright clear color so bury screenshots are background-only.
  const CLEAR = [0.02, 0.78, 0.88, 1];

  function resize() {
    const dpr = window.devicePixelRatio || 1;
    const w = Math.max(1, Math.floor(window.innerWidth * dpr));
    const h = Math.max(1, Math.floor(window.innerHeight * dpr));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
      gl.viewport(0, 0, w, h);
    }
  }

  let frames = 0;
  let start = performance.now();

  function frame(now) {
    resize();
    const t = (now - start) / 1000;
    // Slight pulse so the buffer is clearly "alive" over 5s.
    const pulse = 0.04 * Math.sin(t * 2.2);
    gl.clearColor(CLEAR[0], CLEAR[1] + pulse, CLEAR[2], CLEAR[3]);
    gl.clear(gl.COLOR_BUFFER_BIT);
    frames += 1;
    document.body.dataset.webglFrames = String(frames);
    requestAnimationFrame(frame);
  }

  document.body.dataset.webglReady = "1";
  document.body.dataset.webglClear = "0.02,0.78,0.88";
  requestAnimationFrame(frame);
})();
