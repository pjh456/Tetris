declare const LevelBrand: unique symbol;
export type Level = number & { readonly [LevelBrand]: typeof LevelBrand };

export function is_valid_level(n: unknown): n is Level {
  return typeof n === 'number' && Number.isInteger(n) && n >= 1;
}
