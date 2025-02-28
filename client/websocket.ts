function random(min: number, max: number) {
	return Math.random() * (max - min + 1) + min;
}

Bun.serve({
	fetch(req, server) {
		if (server.upgrade(req)) {
			return;
		}
		return new Response('Upgrade failed', { status: 500 });
	},
	websocket: {
		open(ws) {
			console.debug('WebSocket connection from ' + ws.remoteAddress);
			setInterval(() => {
				ws.send(
					JSON.stringify({
						Data: [random(-180, 180), random(-90, 90), random(100, 999)]
					})
				);
			}, 100);
		},
		close(ws) {
			console.debug('WebSocket connection with ' + ws.remoteAddress + ' closed');
		},
		message: () => {}
	}
});
