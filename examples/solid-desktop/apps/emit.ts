// Emits the plan for one app: emit.ts APP.tsx OUT.plan.json (plan v2.3, async tier on). Run it from
// reactivescript/frontend; RS_FRONTEND overrides where that checkout is (default: beside this one).
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
const fe = process.env.RS_FRONTEND ?? resolve(import.meta.dirname, '../../../../reactivescript/frontend');
const { loadTsxModules } = await import(fe + '/tsx/modules.ts');
const { resolvePlanV2 } = await import(fe + '/plan/resolve-v2.ts');
const [input, out] = process.argv.slice(2);
const read = (p: string) => { try { return readFileSync(p, 'utf8'); } catch { return undefined; } };
const set = loadTsxModules(resolve(input), read, { async: true, compose: true });
const plan = resolvePlanV2(set, { async: true, compose: true });
writeFileSync(out, JSON.stringify(plan, null, 2) + '\n');
console.log(out, 'version', plan.version);
