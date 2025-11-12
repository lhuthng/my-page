<script>
	import { saveLogin } from "$lib/client/user";
	import Footer from "$lib/components/Footer.svelte";
	import Header from "$lib/components/Header.svelte";
	import "../app.css";

	let { children } = $props();
</script>

<Header />
<main>
	{@render children()}
</main>
<Footer />
