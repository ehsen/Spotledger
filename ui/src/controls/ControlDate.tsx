import type { ControlProps } from "./registry";
export function ControlDate({ id, value, onChange, disabled, required }: ControlProps) {
  return (
    <input
      id={id}
      type="date"
      className="form-input form-date"
      value={String(value ?? "")}
      onChange={(e) => onChange(e.target.value || null)}
      disabled={disabled}
      required={required}
    />
  );
}
