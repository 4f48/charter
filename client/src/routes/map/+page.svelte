<script lang="ts">
	import Map from 'ol/Map';
	import View from 'ol/View';
	import OSM from 'ol/source/OSM';
	import TileLayer from 'ol/layer/Tile';
	import { fromLonLat } from 'ol/proj';
	import { onMount } from 'svelte';
	import StatBox from '$lib/StatBox.svelte';
	import ConnectionStatus, { type Status } from '$lib/ConnectionStatus.svelte';
	import { ws, markers, addMarker } from '$lib';

	let status = $state($ws ? $ws.readyState : WebSocket.CLOSED);

	let acceleration = $state<number>();
	let altitude = $state<number>();
	let distance = $state<number>();
	let speed = $state<number>();

	let connectionStatus: Status = $derived.by(() => {
		if (status == WebSocket.OPEN) {
			return 'Connected';
		} else if (status == WebSocket.CLOSED || $ws == undefined) {
			return 'Offline';
		} else {
			return 'Error';
		}
	});
	if ($ws) {
		try {
			$ws.onclose = () => (status = WebSocket.CLOSED);
			$ws.onopen = () => (status = WebSocket.OPEN);
			$ws.onerror = () => (status = -1);
		} catch (error) {
			if (error instanceof Error) console.error(error.message);
		}
	}

	onMount(() => {
		let map = new Map({
			target: 'map',
			layers: [
				new TileLayer({
					source: new OSM()
				})
			],
			view: new View({
				projection: 'EPSG:3857',
				center: fromLonLat([19, 47]),
				zoom: 1
			})
		});
		$markers.forEach((marker) => {
			addMarker(map, marker);
		});
		if ($ws) {
			try {
				$ws.onmessage = (msg) => {
					const data = JSON.parse(msg.data).Data;
					console.debug(JSON.parse(msg.data));

					altitude = Math.floor(data[2]);

					const position: [number, number] = [data[0], data[1]];
					$markers.push(position);
					addMarker(map, position);
				};
			} catch (error) {
				if (error instanceof Error) console.error(error);
			}
		}
	});
</script>

<svelte:head>
	<title>Mobilisat Panel &#x2022; Map</title>
</svelte:head>

<div class="m-0 flex h-full w-full flex-col items-center justify-center p-5">
	<div class="ml-1 w-full">
		<ConnectionStatus status={connectionStatus} />
	</div>
	<div
		id="map"
		class="border-border bg-surface mb-5 h-full w-full flex-[6] cursor-grab border active:cursor-grabbing"
	></div>
	<div class="flex w-full flex-[1] gap-3">
		<StatBox
			name="Acceleration"
			value={acceleration ? acceleration.toString() + 'm/s^2' : undefined}
		/>
		<StatBox name="Altitude" value={altitude ? altitude.toString() + 'm' : undefined} />
		<StatBox name="Distance" value={distance ? distance.toString() + 'm' : undefined} />
		<StatBox name="Speed" value={speed ? speed.toString() + 'm/s' : undefined} />
	</div>
</div>
