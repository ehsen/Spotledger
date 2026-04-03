import type { ControlProps } from "./registry";
export function ControlInt({ id, value, onChange, disabled, required }: ControlProps) {
  return (
    <input
      id={id}
      type="number"
      className="form-input"
      value={value === null || value === undefined ? "" : String(value)}
      onChange={(e) => onChange(e.target.value === "" ? null : parseInt(e.target.value, 10))}
      disabled={disabled}
      required={required}
      step={1}
    />
  );
}
