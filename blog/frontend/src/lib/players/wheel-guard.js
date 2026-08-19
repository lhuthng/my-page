// v86 installs its own window-level "wheel" listener that forwards straight to
// the emulated PS/2 mouse. A plain PS/2 mouse has no wheel: the guest's driver
// has to knock first (sample rate 200, 100, 80) before the mouse ID upgrades
// and packets grow a wheel byte. Windows 9x's stock driver never does that —
// IntelliPoint is what would — so mouse_id stays 0 and every scroll tick
// arrives as a phantom mouse IRQ with no movement, which is enough to fault
// EXPLORER.EXE. v86 sends it regardless of mouse_id.
//
// Blocking the DOM event alone does not work: lock_mouse() locks
// document.body, so once the mouse is captured the wheel event stops targeting
// anything inside the player's own element and a `shell.contains(target)`
// guard silently opts out — exactly while a game is being played. So gate at
// the bus, which v86's listener and ours both funnel through.

export function installWheelGuard(emulator, isBlocked) {
	const bus = emulator?.bus;
	if (!bus || bus.v86WheelGuarded) return;
	const send = bus.send.bind(bus);
	// Rest args: send() is also called as send(name, data, transfer).
	bus.send = (...args) => {
		if (args[0] === 'mouse-wheel' && isBlocked()) return;
		return send(...args);
	};
	bus.v86WheelGuarded = true;
}

/** Whether a wheel event belongs to the emulator. Pointer capture is the
 *  subtle case — see above. */
export function wheelBelongsToEmulator(event, shell) {
	if (typeof document !== 'undefined' && document.pointerLockElement) return true;
	return Boolean(shell?.contains(event.target));
}
