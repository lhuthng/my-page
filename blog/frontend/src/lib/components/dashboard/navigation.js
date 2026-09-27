// Shared description of the dashboard's sections. The lg+ rail
// (DashboardSideBar) and the burger menu's dashboard card (Header) both render
// from this module so the two navigations can never drift apart. Keep in mind
// callers render this conditionally on the viewer being a mod — never render
// it unconditionally, or unauthorized visitors would receive the links.
import BlogButton from '$lib/components/shell/buttons/BlogButton.svelte';
import ProjectButton from '$lib/components/shell/buttons/ProjectButton.svelte';
import SeriesButton from '$lib/components/shell/buttons/SeriesButton.svelte';
import AudiobookButton from '$lib/components/shell/buttons/AudiobookButton.svelte';
import Heart from '$lib/components/svgs/Heart.svelte';
import Diamond from '$lib/components/svgs/Diamond.svelte';
import GamesIcon from './icons/Games.svelte';
import MediaIcon from './icons/Media.svelte';
import UsersIcon from './icons/Users.svelte';
import NewsletterIcon from './icons/Newsletter.svelte';
import TrashIcon from './icons/Trash.svelte';
import V86Icon from './icons/V86.svelte';
import DatabaseIcon from './icons/Database.svelte';
import BackupIcon from './icons/Backup.svelte';

const groups = [
	{
		label: 'Content',
		items: [
			{ icon: BlogButton, label: 'Posts', path: '/dashboard/posts' },
			{ icon: ProjectButton, label: 'Projects', path: '/dashboard/projects', exact: true },
			{ icon: GamesIcon, label: 'Games', path: '/dashboard/games', exact: true },
			{ icon: SeriesButton, label: 'Series', path: '/dashboard/series' },
			{ icon: AudiobookButton, label: 'Audiobooks', path: '/dashboard/audiobooks' },
			{ icon: MediaIcon, label: 'Media', path: '/dashboard/media/manager' }
		]
	},
	{
		label: 'Site',
		items: [
			{ icon: UsersIcon, label: 'Users', path: '/dashboard/users' },
			{ icon: NewsletterIcon, label: 'Newsletter', path: '/dashboard/newsletter' },
			{ icon: TrashIcon, label: 'Trash', path: '/dashboard/trash' }
		]
	}
];

const adminItems = [
	{ icon: Heart, label: 'Highlight Posts', path: '/dashboard/highlights' },
	{ icon: Diamond, label: 'Highlight Projects', path: '/dashboard/projects/highlights' },
	{ icon: V86Icon, label: 'v86 Systems', path: '/dashboard/v86-systems' },
	{ icon: DatabaseIcon, label: 'Database', path: '/dashboard/database' },
	{ icon: BackupIcon, label: 'Backup & Sync', path: '/dashboard/backup' }
];

/** Admin-only sections are appended only when the viewer is an admin. */
export function dashboardNavGroups(role) {
	return role === 'admin' ? [...groups, { label: 'Admin', items: adminItems }] : groups;
}

export function isDashboardTabActive(currentPath, tab) {
	return (
		currentPath === tab.path ||
		(!tab.exact && tab.path !== '/dashboard' && currentPath.startsWith(tab.path))
	);
}
