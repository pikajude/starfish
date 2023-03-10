export function ifn<T, Z>(x: T | null, cb: (y: T) => Z): Z | null {
  return x == null ? null : cb(x);
}
