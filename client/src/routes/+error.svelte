<script lang="ts">
	import { page } from '$app/state';
</script>

<div class="flex w-[25%] flex-col items-center gap-1">
	<h1 class="text-2xl font-bold text-[#9d0006]">Error {page.status}</h1>
	<p class="text-[#cc241d]">{page.error?.message}</p>
</div>
