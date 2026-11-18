import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
	canDeleteApplication,
	canEditEnvironmentSettings,
	canEditMemberRole,
	canManageOwners,
	canManageTeam,
	canMutateEnvironment,
	computeSuccessRate,
	exportAuditLogsToCsv,
	formatRelativeTimestamp,
	getInitials
} from './permissions';

describe('Control Plane RBAC permission helpers', () => {
	it('enforces viewer as strictly read-only', () => {
		expect(canMutateEnvironment('viewer')).toBe(false);
		expect(canEditEnvironmentSettings('viewer')).toBe(false);
		expect(canManageTeam('viewer')).toBe(false);
		expect(canDeleteApplication('viewer')).toBe(false);
		expect(canManageOwners('viewer')).toBe(false);
	});

	it('allows developer operational mutations but restricts admin and owner controls', () => {
		expect(canMutateEnvironment('developer')).toBe(true);
		expect(canEditEnvironmentSettings('developer')).toBe(false);
		expect(canManageTeam('developer')).toBe(false);
		expect(canDeleteApplication('developer')).toBe(false);
		expect(canManageOwners('developer')).toBe(false);
	});

	it('allows admin team management except owners and allows owner full control', () => {
		expect(canEditEnvironmentSettings('admin')).toBe(true);
		expect(canManageTeam('admin')).toBe(true);
		expect(canDeleteApplication('admin')).toBe(true);
		expect(canManageOwners('admin')).toBe(false);
		expect(canEditMemberRole('admin', 'developer')).toBe(true);
		expect(canEditMemberRole('admin', 'viewer')).toBe(true);
		expect(canEditMemberRole('admin', 'owner')).toBe(false);

		expect(canManageOwners('owner')).toBe(true);
		expect(canEditMemberRole('owner', 'owner')).toBe(true);
	});
});

describe('Audit CSV export and formatting utilities', () => {
	it('exports CSV and neutralizes spreadsheet formula injection', () => {
		const csv = exportAuditLogsToCsv([
			{
				id: 'log_1',
				environment_id: 'env_1',
				user_id: null,
				actor_type: 'developer',
				actor_id: 'dev_1',
				resource_type: 'application_user',
				resource_id: 'au_1',
				event: '=HYPERLINK("evil")',
				ip_address: '127.0.0.1',
				user_agent: null,
				metadata: { note: 'hello, world' },
				created_at: '2026-11-18T12:00:00Z'
			}
		]);
		const lines = csv.split('\n');
		expect(lines).toHaveLength(2);
		expect(lines[1]).toContain("'=HYPERLINK(\"\"evil\"\")");
		expect(lines[1]).toContain('hello, world');
	});

	it('computes success rates, initials, and relative timestamps cleanly', () => {
		expect(computeSuccessRate(0, 0)).toBe(100);
		expect(computeSuccessRate(95, 5)).toBe(95);
		expect(getInitials('Ada Lovelace', 'ada@example.test')).toBe('AL');
		expect(getInitials('', 'grace@example.test')).toBe('GR');

		const now = Date.parse('2026-11-18T12:10:00Z');
		expect(formatRelativeTimestamp('2026-11-18T12:09:30Z', now)).toBe('Just now');
		expect(formatRelativeTimestamp('2026-11-18T12:05:00Z', now)).toBe('5m ago');
		expect(formatRelativeTimestamp('2026-11-18T10:10:00Z', now)).toBe('2h ago');
		expect(formatRelativeTimestamp(null, now)).toBe('Never');
	});
});

describe('Design system invariants (zero gradients, zero emojis)', () => {
	it('contains zero CSS gradients and zero emojis across apps/web/src', () => {
		const srcRoot = join(process.cwd(), 'src');
		const files: string[] = [];
		function walk(dir: string) {
			for (const entry of readdirSync(dir, { withFileTypes: true })) {
				const full = join(dir, entry.name);
				if (entry.isDirectory()) walk(full);
				else if (/\.(svelte|css)$/.test(entry.name)) files.push(full);
			}
		}
		walk(srcRoot);

		const gradientRe = /(?:linear|radial|conic)-gradient\(/;
		const emojiRe = /[\u{1F300}-\u{1FAFF}]/u;
		for (const file of files) {
			const content = readFileSync(file, 'utf8');
			expect(gradientRe.test(content), `Gradient found in ${file}`).toBe(false);
			expect(emojiRe.test(content), `Emoji found in ${file}`).toBe(false);
		}
	});
});

