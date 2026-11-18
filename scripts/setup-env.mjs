import { randomBytes } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const template = await readFile(new URL('../.env.example', import.meta.url), 'utf8');
const content = template.replace(/^(JWT_SECRET|SIGNING_KEY_KEK)=.*$/gm, (_, name) => `${name}=${randomBytes(48).toString('base64url')}`);
try {
    await writeFile(resolve('.env'), content, { mode: 0o600, flag: 'wx' });
    process.stdout.write('Created .env with unique development secrets. Review its connection settings before starting.\n');
} catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code === 'EEXIST') {
        process.stderr.write('.env already exists and has been preserved. Configure required secrets in that file.\n');
        process.exitCode = 1;
    } else {
        throw error;
    }
}
