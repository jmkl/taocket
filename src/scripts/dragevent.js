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
