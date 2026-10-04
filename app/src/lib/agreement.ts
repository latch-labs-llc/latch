/**
 * The deterministic agreement template lives in @latch-labs/checkout so every
 * integrator regenerates byte-identical text (template v1). Re-exported here
 * as the app's single source — do not fork the template.
 */
export { buildAgreement, sha256, hex } from "@latch-labs/checkout";
export type { AgreementParams } from "@latch-labs/checkout";
