import type { ControlProps } from "./registry";
export function ControlData({ id, value, onChange, disabled, required, field }: ControlProps) {
  return (
    <input
      id={id}
      type="text"
      className="form-input"
      value={String(value ?? "")}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      required={required}
      maxLength={undefined}
      autoComplete="off"
    />
  );
}
