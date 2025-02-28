<script lang="ts">
	import '../app.css';
	import Navigation from '$lib/Navigation.svelte';
	let { children } = $props();
</script>

<Navigation />
<div class="flex h-[calc(100vh-3rem)] w-full items-center justify-center">
	{@render children()}
</div>
