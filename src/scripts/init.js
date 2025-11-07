(function () {
	if (window.__API__INVOKE) return;
	let nextId = 1;
	window.__API__INVOKE = function invoke(event, value) {
		return new Promise((resolve, reject) => {
			const id = nextId++;
			const message = JSON.stringify(
				{ payload: { id, event: { type: event, value } } },
				null,
				2,
			);
			window.ipc.postMessage(message);
		});
	};

	window.invoke = window.__API__INVOKE;
})();
