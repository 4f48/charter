<script lang="ts">
	import { onMount } from 'svelte';
	import { ws, url, markers } from '$lib';

	let connected = $state(false);
	let submitting = $state(false);

	onMount(() => {
		if ($ws?.readyState == WebSocket.OPEN) {
			connected = true;
		} else connected = false;
	});

	const handleConnect = (event: SubmitEvent) => {
		event.preventDefault();
		if (connected) {
			connected = false;
			$ws.close();
			return;
		}
		submitting = true;
		$ws = new WebSocket($url);
		$ws.onopen = () => {
			connected = true;
			submitting = false;
		};
		$ws.onclose = () => {
			connected = false;
			submitting = false;
		};
		$ws.onerror = () => {
			connected = false;
			submitting = false;
		};
	};
	const clearMarkers = () => {
		$markers = [];
	};
</script>

<svelte:head>
	<title>Mobilisat Panel &#x2022; Settings</title>
</svelte:head>

<div>
	<form class="flex w-[25vw] flex-col" onsubmit={handleConnect}>
		<span
			class={{
				'mb-5 flex h-12 items-center justify-center border border-[#cc241d] font-mono text-[#9d0006]':
					!connected,
				'mb-5 flex h-12 items-center justify-center border border-[#689d6a] font-mono text-[#427b58]':
					connected
			}}
		>
			{#if connected}
				Connected
			{:else}
				Not Connected
			{/if}
		</span>
		<label for="url" class="font-mono text-[#3c3836]"> WebSocket URL </label>
		<input
			class="mb-2 h-10 border border-[#bdae93] px-2 font-mono text-[#3c3836] placeholder:text-[#bdae93] focus:border-[#928374] focus:outline-none disabled:cursor-not-allowed disabled:bg-[#d5c4a1] disabled:text-[#928374]"
			id="url"
			placeholder="ws://..."
			bind:value={$url}
			type="url"
			required
			disabled={submitting || connected}
		/>
		<button
			type="submit"
			class="border-border bg-surface h-10 cursor-pointer border font-mono text-[#3c3836] hover:border-[#928374] disabled:cursor-wait disabled:bg-[#f2e5bc] disabled:hover:border-[#bdae93]"
			disabled={submitting}
		>
			{#if submitting}
				Connecting...
			{:else if !connected}
				Connect
			{:else}
				Disconnect
			{/if}
		</button>
	</form>
	<button
		class="bg-bg border-border active:bg-surface mt-3 h-10 w-full cursor-pointer border font-mono text-[#3c3836] hover:border-[#928374] disabled:cursor-wait disabled:bg-[#f2e5bc] disabled:hover:border-[#bdae93]"
		onclick={clearMarkers}>Clear markers</button
	>
</div>
