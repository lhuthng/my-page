<script>
</script>

<svelte:head>
	<title>Home</title>
	<!-- <meta name="description" content="Svelte demo app" /> -->
</svelte:head>

<div class="flex flex-col gap-8 py-8 md:h-screen min-h-200">
	<div class="w-cap bg-yello-200">
		<div class="flex justify-between">
			<span>Software</span>
			<span>Game</span>
			<span>Music</span>
			<span>Art</span>
			<span>Maths</span>
			<span>All topics</span>
		</div>
	</div>
	<div class="w-cap grow">
		<div
			class="flex not-md:flex-col justify-between h-full bg-green-500 w-full"
		>
			<div
				class="flex flex-col not-md:h-160 w-full md:w-3/5 bg-green-600"
			>
				<span>Featured</span>
				<div class="full p-2">
					<div class="full bg-green-700 rounded-lg"></div>
				</div>
			</div>
			<div class="flex flex-col grow">
				<span class="text-right">Latest</span>
				<div
					class="grid grid-cols-1 not-md:h-200 md:grid-cols-2 gap-2 grow [&>span]:rounded-lg [&>span]:bg-green-600 p-2"
				>
					<span></span>
					<span></span>
					<span></span>
					<span></span>
				</div>
			</div>
		</div>
	</div>
	<div class="h-4 shrink-0"></div>
</div>
<div class="w-full h-120 bg-purple-300"></div>

<style lang="postcss">
	@reference "../app.css";
</style>
