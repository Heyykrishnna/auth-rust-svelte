export type WorkspaceView = 'applications' | 'team' | 'settings' | 'security';

export function workspaceHref(
	workspaceId?: string,
	view: WorkspaceView = 'applications',
	appId?: string,
	section?: string
): string {
	const params = new URLSearchParams();
	if (workspaceId) params.set('workspace', workspaceId);
	if (view !== 'applications') params.set('view', view);
	if (appId) params.set('app', appId);
	if (section && section !== 'overview') params.set('section', section);
	const query = params.toString();
	return query ? `/control?${query}` : '/control';
}
