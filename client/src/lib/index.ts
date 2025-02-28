import { Feature, type Map } from 'ol';
import { Point } from 'ol/geom';
import VectorLayer from 'ol/layer/Vector';
import { fromLonLat } from 'ol/proj';
import VectorSource from 'ol/source/Vector';
import { writable } from 'svelte/store';

export const ws = writable<WebSocket>();
export const url = writable<string>();
export const markers = writable<[number, number][]>([]);

export function addMarker(map: Map, position: [number, number]) {
	const marker = new Feature({
		geometry: new Point(fromLonLat([position[0], position[1]]))
	});
	marker.setStyle();
	map.addLayer(
		new VectorLayer({
			source: new VectorSource({
				features: [marker]
			})
		})
	);
}
