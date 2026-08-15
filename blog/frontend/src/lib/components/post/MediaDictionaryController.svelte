<script>
	import OfflineMediaCreator from './OfflineMediaCreator.svelte';
	import OnlineMediaSearcher from './OnlineMediaSearcher.svelte';

	// `media` is created once by the view-model (`createMediaDictionary()`) and
	// handed down as a plain prop — this component only renders it. It used to
	// own this state itself and hand functions back *up* into `$state` slots
	// its parent declared for that purpose; now nothing needs to cross the
	// boundary in either direction beyond this one object.
	let { media, ...rest } = $props();
</script>

<div {...rest}>
	<OnlineMediaSearcher
		class="flex flex-col w-1/2 lg:w-60 h-full text-dark bg-background/30 border border-dark/10 rounded-xl"
	/>

	<OfflineMediaCreator
		onlineMedia={media.online}
		offlineMedia={media.offline}
		uploadNewMedia={media.uploadNew}
		changeName={media.changeName}
		class="flex flex-col w-1/2 lg:w-60 h-full text-dark bg-background/30 border border-dark/10 rounded-xl"
	/>
</div>
