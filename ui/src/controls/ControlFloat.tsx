import type { ControlProps } from "./registry";
export function ControlFloat({ id, value, onChange, disabled, required, field }: ControlProps) {
  const precision = parseInt(field.precision ?? "2", 10) || 2;
  return (
    <input
      id={id}
      type="number"
      className="form-input"
      value={value === null || value === undefined ? "" : String(value)}
      onChange={(e) => onChange(e.target.value === "" ? null : parseFloat(e.target.value))}
      disabled={disabled}
      required={required}
      step={Math.pow(10, -precision)}
    />
  );
}
