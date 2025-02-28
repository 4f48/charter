<script lang="ts">
	export type Status = 'Connected' | 'Offline' | 'Error';
	let { status }: { status: Status } = $props();
</script>

<div class="flex items-center gap-2">
	<div class="relative flex h-2 w-2">
		<div
			class={{
				'absolute inline-flex h-full w-full animate-ping rounded-full bg-[#689d6a] opacity-75':
					status == 'Connected',
				hidden: status != 'Connected'
			}}
		></div>
		<div
			class={{
				'relative inline-flex h-2 w-2 rounded-full bg-[#689d6a]': status == 'Connected',
				'relative inline-flex h-2 w-2 rounded-full bg-[#cc241d]': status == 'Error',
				'relative inline-flex h-2 w-2 rounded-full bg-[#7c6f64]': status == 'Offline'
			}}
		></div>
	</div>
	<span>{status}</span>
</div>
