// Shared public navigation list. The lg+ rail (NavigationSideBar) and the
// header's burger menu both render from this module, so a route added to one
// can never stay unreachable on phones. `secret` items are mod-owned and are
// only rendered for viewers the client auth state marks as mods.
import AboutButton from './buttons/AboutButton.svelte';
import AudiobookButton from './buttons/AudiobookButton.svelte';
import BlogButton from './buttons/BlogButton.svelte';
import DashboardButton from './buttons/DashboardButton.svelte';
import HomeButton from './buttons/HomeButton.svelte';
import ProjectButton from './buttons/ProjectButton.svelte';
import SeriesButton from './buttons/SeriesButton.svelte';

export const publicNavItems = [
	{ icon: HomeButton, label: 'Home', path: '/', route: '' },
	{ icon: BlogButton, label: 'Posts', path: '/posts', route: 'posts' },
	{ icon: ProjectButton, label: 'Projects', path: '/projects', route: 'projects' },
	{ icon: SeriesButton, label: 'Series', path: '/series', route: 'series' },
	{ icon: AudiobookButton, label: 'Audiobooks', path: '/audiobooks', route: 'audiobooks' },
	{ icon: AboutButton, label: 'About', path: '/about', route: 'about' },
	{
		icon: DashboardButton,
		label: 'Dashboard',
		path: '/dashboard',
		route: 'dashboard',
		secret: true
	}
];
