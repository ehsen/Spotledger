import type { ControlProps } from "./registry";
export function ControlCheck({ id, value, onChange, disabled, field }: ControlProps) {
  return (
    <label className="form-check-label" htmlFor={id}>
      <input
        id={id}
        type="checkbox"
        className="form-checkbox"
        checked={value === 1 || value === true}
        onChange={(e) => onChange(e.target.checked ? 1 : 0)}
        disabled={disabled}
      />
      <span className="form-check-text">{field.label}</span>
    </label>
  );
}
