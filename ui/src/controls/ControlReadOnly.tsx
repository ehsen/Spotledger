import type { ControlProps } from "./registry";
export function ControlReadOnly({ value }: ControlProps) {
  const display = value === null || value === undefined ? "—" : String(value);
  return <span className="form-readonly">{display}</span>;
}
