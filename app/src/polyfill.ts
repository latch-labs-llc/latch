// Must be the FIRST import of the entry module: ES imports are hoisted, so
// assigning Buffer inside main.tsx runs only after every dependency has
// already evaluated — any module-scope Buffer use would crash the bundle.
// As its own module, this evaluates before the rest of the import graph.
import { Buffer } from "buffer";
(globalThis as any).Buffer = Buffer;
