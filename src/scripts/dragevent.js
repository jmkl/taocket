function dragging() {
	const drag = document.querySelector("[data-taoket-drag-region]");
	if (drag) {
		let isDragging = false;
		let startX = 0;
		let startY = 0;
		drag.addEventListener("mousedown", (e) => {
			if (e.button !== 0) return;
			isDragging = true;
			startX = e.clientX;
			startY = e.clientY;
			invoke("Move", true);
			e.preventDefault();
			e.stopPropagation();
		});

		window.addEventListener("mousemove", (e) => {
			if (!isDragging) return;
			const deltaX = e.clientX - startX;
			const deltaY = e.clientY - startY;
		});

		window.addEventListener("mouseup", () => {
			if (!isDragging) return;
			isDragging = false;
			invoke("Move", false);
		});
	}
}

(function () {
	let initialX = 0;
	let initialY = 0;
	document.addEventListener("mousedown", (e) => {
		const attr = e.target.getAttribute("data-taocket-drag-region");
		if (attr != null && (e.detail === 1 || e.detail === 2)) {
			e.preventDefault();
			e.stopImmediatePropagation();
			invoke("Move", true);
		}
	});
})();
