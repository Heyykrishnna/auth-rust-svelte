import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm, mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const root = fileURLToPath(new URL("../", import.meta.url));
const archive = join(root, "upstream/picomatch-4.0.7.tgz");
const expectedIntegrity =
  "qcJu88Q2IWqJsDD529JKMdwGm/dvInW4HvQnRwiH9JtihJvzGOscDtHE3x1pBKeUOTysQ8kVmLnJ2kJu7yhcGA==";
const digest = createHash("sha512")
  .update(await readFile(archive))
  .digest("base64");
if (digest !== expectedIntegrity)
  throw new Error("Picomatch upstream archive integrity mismatch");

const files = [
  "index.js",
  "posix.js",
  "lib/constants.js",
  "lib/parse.js",
  "lib/picomatch.js",
  "lib/scan.js",
  "lib/utils.js",
];
const temporary = await mkdtemp(join(tmpdir(), "picomatch-generate-"));
const check = process.argv.includes("--check");

try {
  for (const file of [...files, "LICENSE"]) {
    const path = join(temporary, file);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(
      path,
      execFileSync("tar", ["-xOf", archive, `package/${file}`]),
    );
  }
  execFileSync("patch", ["-s", "-p1", "-i", join(root, "security.patch")], {
    cwd: temporary,
  });

  for (const file of files) {
    const source = await readFile(join(temporary, file), "utf8");
    const output = ts.transpileModule(source, {
      compilerOptions: {
        target: ts.ScriptTarget.ES2022,
        module: ts.ModuleKind.CommonJS,
        removeComments: true,
        newLine: ts.NewLineKind.LineFeed,
      },
    }).outputText;
    const destination = join(root, file);
    if (check) {
      if ((await readFile(destination, "utf8")) !== output)
        throw new Error(`Stale vendored file: ${file}`);
    } else {
      await mkdir(dirname(destination), { recursive: true });
      await writeFile(destination, output);
    }
  }

  if (
    !Buffer.from(await readFile(join(root, "LICENSE"))).equals(
      await readFile(join(temporary, "LICENSE")),
    )
  ) {
    throw new Error("Picomatch upstream license differs");
  }
  process.stdout.write(
    `Picomatch integrity, patch, generated files, and license ${check ? "verified" : "generated"}\n`,
  );
} finally {
  await rm(temporary, { recursive: true, force: true });
}
