import type { ControlProps } from "./registry";
export function ControlPassword({ id, value, onChange, disabled, required }: ControlProps) {
  return (
    <input
      id={id}
      type="password"
      className="form-input"
      value={String(value ?? "")}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      required={required}
      autoComplete="new-password"
    />
  );
}
