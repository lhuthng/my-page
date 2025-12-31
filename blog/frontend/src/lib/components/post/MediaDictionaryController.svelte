<script>
  import OfflineMediaCreator from "./OfflineMediaCreator.svelte";
  import OnlineMediaSearcher from "./OnlineMediaSearcher.svelte";

  let {
    updateMediaDictionary,
    registerSearch,
    registerMediaCheck,
    registerGetMedia,
  } = $props();

  let newMedia = $state({});

  let onlineMedia = $state({});
  let offlineMedia = $derived(
    Object.fromEntries(
      Object.entries(newMedia).map(([key, value]) => [key, value.url]),
    ),
  );

  $effect(() => {
    updateMediaDictionary({
      ...offlineMedia,
      ...Object.fromEntries(
        Object.entries(onlineMedia).filter(([_, v]) => v !== undefined),
      ),
    });
  });

  const searchOnlineMedia = async (keys) => {
    const dict = { ...onlineMedia };
    const missingKeys = keys.filter((key) => !(key in onlineMedia));

    missingKeys.forEach((key) => (dict[key] = null));
    onlineMedia = { ...dict };

    missingKeys.forEach(async (key) => {
      const res = await fetch("/api/media/s/" + key, { method: "GET" });
      if (res.ok) {
        onlineMedia[key] = (await res.json()).url;
      } else {
        onlineMedia[key] = undefined;
        setTimeout(() => delete onlineMedia[key], 5000);
      }
    });
  };

  registerSearch(searchOnlineMedia);
  registerMediaCheck({
    isOnline: (keyword) =>
      keyword in onlineMedia && onlineMedia[keyword] != null,
    isOffline: (keyword) =>
      keyword in offlineMedia && offlineMedia[keyword] != null,
  });
  registerGetMedia((keyword) => newMedia[keyword]);

  const updateNewMedia = (media) => {
    const temp = { ...newMedia };
    const names = [];
    media.forEach((medium) => {
      temp[medium.name] = medium;
      names.push(medium.name);
    });
    searchOnlineMedia(names);
    newMedia = { ...temp };
  };

  const changeName = (oldName, newName) => {
    if (newName in newMedia) return false;

    let temp = { ...newMedia };
    temp[newName] = temp[oldName];

    delete temp[oldName];

    newMedia = { ...temp };
    if (!(newName in onlineMedia)) {
      searchOnlineMedia([newName]);
    }
    console.log({ ...onlineMedia }, { ...newMedia });
    return true;
  };
</script>

<OnlineMediaSearcher
  class="flex flex-col w-1/2 lg:w-60 h-full text-dark bg-primary/40 rounded-xl"
/>

<OfflineMediaCreator
  {onlineMedia}
  {offlineMedia}
  {updateNewMedia}
  {changeName}
  class="flex flex-col w-1/2 lg:w-60 h-full text-dark bg-primary/40 rounded-xl"
/>
