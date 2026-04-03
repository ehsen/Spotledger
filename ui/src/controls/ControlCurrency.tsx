import type { ControlProps } from "./registry";
export function ControlCurrency({ id, value, onChange, disabled, required }: ControlProps) {
  return (
    <div className="form-currency-wrap">
      <input
        id={id}
        type="number"
        className="form-input form-currency"
        value={value === null || value === undefined ? "" : String(value)}
        onChange={(e) => onChange(e.target.value === "" ? null : parseFloat(e.target.value))}
        disabled={disabled}
        required={required}
        step="0.01"
      />
    </div>
  );
}
