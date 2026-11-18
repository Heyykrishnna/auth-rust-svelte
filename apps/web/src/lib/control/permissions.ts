import type { AuditLogEntryDto, WorkspaceRole } from '$lib/api/types';

export function roleRank(role: WorkspaceRole | string | undefined | null): number {
	switch (role) {
		case 'owner':
			return 4;
		case 'admin':
			return 3;
		case 'developer':
			return 2;
		case 'viewer':
			return 1;
		default:
			return 0;
	}
}

export function canMutateEnvironment(role: WorkspaceRole | string | undefined | null): boolean {
	return roleRank(role) >= 2;
}

export function canEditEnvironmentSettings(
	role: WorkspaceRole | string | undefined | null
): boolean {
	return roleRank(role) >= 3;
}

export function canManageTeam(role: WorkspaceRole | string | undefined | null): boolean {
	return roleRank(role) >= 3;
}

export function canDeleteApplication(role: WorkspaceRole | string | undefined | null): boolean {
	return roleRank(role) >= 3;
}

export function canManageOwners(role: WorkspaceRole | string | undefined | null): boolean {
	return roleRank(role) >= 4;
}

export function canEditMemberRole(
	actorRole: WorkspaceRole | string | undefined | null,
	targetRole: WorkspaceRole | string
): boolean {
	if (actorRole === 'owner') return roleRank(targetRole) > 0;
	if (actorRole === 'admin') {
		return targetRole === 'admin' || targetRole === 'developer' || targetRole === 'viewer';
	}
	return false;
}

function escapeCsvCell(value: string): string {
	const safe = /^[=+\-@\t\r]/.test(value) ? `'${value}` : value;
	if (/[",\n\r]/.test(safe)) {
		return `"${safe.replace(/"/g, '""')}"`;
	}
	return safe;
}

export function exportAuditLogsToCsv(logs: AuditLogEntryDto[]): string {
	const header = [
		'id',
		'created_at',
		'event',
		'actor_type',
		'actor_id',
		'resource_type',
		'resource_id',
		'ip_address',
		'metadata'
	];
	const rows = logs.map((log) => [
		log.id,
		log.created_at,
		log.event,
		log.actor_type,
		log.actor_id ?? '',
		log.resource_type ?? '',
		log.resource_id ?? '',
		log.ip_address ?? '',
		JSON.stringify(log.metadata ?? {})
	]);
	return [header, ...rows].map((row) => row.map(escapeCsvCell).join(',')).join('\n');
}

export function formatRelativeTimestamp(iso: string | null | undefined, nowMs = Date.now()): string {
	if (!iso) return 'Never';
	const ts = Date.parse(iso);
	if (Number.isNaN(ts)) return iso;
	const diffSecs = Math.max(0, Math.floor((nowMs - ts) / 1000));
	if (diffSecs < 60) return 'Just now';
	if (diffSecs < 3600) return `${Math.floor(diffSecs / 60)}m ago`;
	if (diffSecs < 86400) return `${Math.floor(diffSecs / 3600)}h ago`;
	const days = Math.floor(diffSecs / 86400);
	if (days < 30) return `${days}d ago`;
	return new Date(ts).toISOString().slice(0, 10);
}

export function computeSuccessRate(success: number, failure: number): number {
	const total = success + failure;
	if (total <= 0) return 100;
	return Math.round((success / total) * 1000) / 10;
}

export function getInitials(displayName: string | null | undefined, email: string): string {
	const clean = (displayName ?? '').trim();
	if (clean.length > 0) {
		const parts = clean.split(/\s+/).filter(Boolean);
		if (parts.length >= 2) {
			return `${parts[0]![0] ?? ''}${parts[1]![0] ?? ''}`.toUpperCase();
		}
		return clean.slice(0, 2).toUpperCase();
	}
	return email.slice(0, 2).toUpperCase();
}

