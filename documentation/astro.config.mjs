// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'Easymake',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/pchakour/easymake' }],
			sidebar: [
				{
					label: 'Start here',
					items: [
						// Each item here is one entry in the navigation menu.
						{ label: 'Getting started', slug: 'start_here/getting_started' },
						// { label: 'First steps', slug: 'start_here/first_steps' },
						// { label: 'Command line', slug: 'start_here/command_line' },
					],
				},
				{
					label: 'Guides',
					items: [
						// Each item here is one entry in the navigation menu.
						// { label: 'Example Guide', slug: 'guides/example' },
					],
				},
				{
					label: 'Reference',
					autogenerate: { directory: 'reference' },
				},
			],
		}),
	],
});
