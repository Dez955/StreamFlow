import type { UserError } from './types';

/** Backend rejections are serialized UserError objects; anything else becomes "other". */
export function toUserError(e: unknown): UserError {
  if (e && typeof e === 'object' && 'kind' in e) return e as UserError;
  return { kind: 'other', detail: String(e) };
}
