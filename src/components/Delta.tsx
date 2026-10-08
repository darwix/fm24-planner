export default function Delta({ a, b }: { a: number; b: number }) {
  const x = a - b;
  if (x > 0) return <b className="up">+{x}</b>;
  if (x < 0) return <b className="dn">{x}</b>;
  return <b className="eq">·</b>;
}
