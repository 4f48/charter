<script lang="ts">
	let { children, href, page } = $props();
</script>

<a
	{href}
	class={{
		'border-r-surface border-b-border border-l-surface bg-surface flex h-full w-32 items-center justify-center border-r border-b border-l p-3':
			page != href,
		'border-r-border border-l-border bg-bg flex h-full w-32 items-center justify-center border-r border-l p-3':
			page == href
	}}
	data-astro-prefetch="load"
>
	{@render children?.()}
</a>
