// On-screen controls for phones. The game reads keyboard events from its canvas, so each control
// presses player one's keys (assets/game.yaml keyboard1) instead of teaching the engine about touch.
const coarse = matchMedia("(pointer: coarse)").matches;
if (coarse || new URLSearchParams(location.search).has("touch")) mount();

function mount() {
  const style = document.createElement("style");
  style.textContent = `
    .tc { position: fixed; inset: 0; pointer-events: none; z-index: 10; touch-action: none;
      -webkit-user-select: none; user-select: none; -webkit-touch-callout: none;
      font: 600 13px/1 -apple-system, system-ui, sans-serif; color: #fff; }
    .tc button { pointer-events: auto; touch-action: none; border: 2px solid rgba(255,255,255,.35);
      background: rgba(0,0,0,.28); color: inherit; font: inherit; border-radius: 999px; padding: 0; }
    .tc button.on { background: rgba(255,255,255,.35); }
    .tc-pad { position: absolute; left: calc(env(safe-area-inset-left) + 20px); bottom: calc(env(safe-area-inset-bottom) + 20px);
      width: 150px; height: 150px; border-radius: 50%; pointer-events: auto; touch-action: none;
      background: rgba(0,0,0,.22); border: 2px solid rgba(255,255,255,.3); }
    .tc-knob { position: absolute; left: 50%; top: 50%; width: 56px; height: 56px; margin: -28px 0 0 -28px;
      border-radius: 50%; background: rgba(255,255,255,.4); }
    .tc-actions { position: absolute; right: calc(env(safe-area-inset-right) + 20px); bottom: calc(env(safe-area-inset-bottom) + 20px);
      display: grid; grid-template-columns: 64px 64px; gap: 12px; }
    .tc-actions button { width: 64px; height: 64px; }
    .tc-actions .jump { grid-column: 2; grid-row: 2; width: 72px; height: 72px; }
    .tc-menu { position: absolute; top: calc(env(safe-area-inset-top) + 12px); right: calc(env(safe-area-inset-right) + 16px);
      display: flex; gap: 8px; }
    .tc-menu button { padding: 8px 14px; }
  `;
  document.head.append(style);

  const root = document.createElement("div");
  root.className = "tc";
  root.innerHTML = `
    <div class="tc-pad"><div class="tc-knob"></div></div>
    <div class="tc-actions">
      <button data-code="KeyK">Grab</button><button data-code="KeyJ">Shoot</button>
      <button data-code="KeyL">Slide</button><button class="jump" data-code="Space">Jump</button>
    </div>
    <div class="tc-menu"><button data-code="Enter">Start</button><button data-code="Escape">Menu</button></div>
  `;
  document.body.append(root);

  const send = (type, code) => {
    const canvas = document.querySelector("canvas");
    canvas?.dispatchEvent(new KeyboardEvent(type, { code, key: code, bubbles: true, cancelable: true }));
  };

  // The game samples held keys once per frame, so a tap shorter than a frame would never register.
  const minimumHoldMs = 80;
  for (const button of root.querySelectorAll("button")) {
    const code = button.dataset.code;
    let pressedAt = 0;
    const release = () => {
      if (!button.classList.contains("on")) return;
      button.classList.remove("on");
      setTimeout(() => send("keyup", code), Math.max(0, pressedAt + minimumHoldMs - performance.now()));
    };
    button.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      button.setPointerCapture(event.pointerId);
      button.classList.add("on");
      pressedAt = performance.now();
      send("keydown", code);
    });
    button.addEventListener("pointerup", release);
    button.addEventListener("pointercancel", release);
  }

  // The stick holds up to two of WASD, so diagonals aim and move like a gamepad would.
  const pad = root.querySelector(".tc-pad");
  const knob = root.querySelector(".tc-knob");
  const held = new Set();
  const hold = (next) => {
    for (const code of held) if (!next.has(code)) send("keyup", code);
    for (const code of next) if (!held.has(code)) send("keydown", code);
    held.clear();
    for (const code of next) held.add(code);
  };
  const steer = (event) => {
    const box = pad.getBoundingClientRect();
    const radius = box.width / 2;
    let dx = event.clientX - (box.left + radius);
    let dy = event.clientY - (box.top + radius);
    const length = Math.hypot(dx, dy);
    if (length > radius) {
      dx *= radius / length;
      dy *= radius / length;
    }
    knob.style.transform = `translate(${dx}px, ${dy}px)`;
    const dead = radius * 0.3;
    const next = new Set();
    if (dx < -dead) next.add("KeyA");
    if (dx > dead) next.add("KeyD");
    if (dy < -dead) next.add("KeyW");
    if (dy > dead) next.add("KeyS");
    hold(next);
  };
  const center = () => {
    knob.style.transform = "";
    hold(new Set());
  };
  pad.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    pad.setPointerCapture(event.pointerId);
    steer(event);
  });
  pad.addEventListener("pointermove", (event) => {
    if (pad.hasPointerCapture(event.pointerId)) steer(event);
  });
  pad.addEventListener("pointerup", center);
  pad.addEventListener("pointercancel", center);
}
