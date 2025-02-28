<script lang="ts">
	import NavigationLink from '$lib/NavigationLink.svelte';
	import { page } from '$app/state';
</script>

<header class="flex">
	<nav class="flex h-12 w-full items-center">
		<div class="border-b-border bg-surface h-full w-12 border-b"></div>
		<NavigationLink href="/" page={page.url.pathname}>Settings</NavigationLink>
		<NavigationLink href="/map" page={page.url.pathname}>Map</NavigationLink>
		<div class="border-b-border bg-surface h-full w-full border-b"></div>
	</nav>
</header>
