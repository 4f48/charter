<script lang="ts">
	let { name, value = '?' } = $props();
</script>

<div class="bg-surface border-border flex w-[25%] grow flex-col border p-1">
	<h3>{name}</h3>
	<span class="text-center text-4xl">{value}</span>
</div>
