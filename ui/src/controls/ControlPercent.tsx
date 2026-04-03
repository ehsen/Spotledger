import type { ControlProps } from "./registry";
export function ControlPercent({ id, value, onChange, disabled, required }: ControlProps) {
  return (
    <div className="form-percent-wrap">
      <input
        id={id}
        type="number"
        className="form-input form-percent"
        value={value === null || value === undefined ? "" : String(value)}
        onChange={(e) => onChange(e.target.value === "" ? null : parseFloat(e.target.value))}
        disabled={disabled}
        required={required}
        min={0}
        max={100}
        step="0.01"
      />
      <span className="form-percent-suffix">%</span>
    </div>
  );
}
