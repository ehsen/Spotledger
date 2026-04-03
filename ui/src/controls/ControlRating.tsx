import type { ControlProps } from "./registry";
export function ControlRating({ id, value, onChange, disabled }: ControlProps) {
  const current = Number(value ?? 0);
  return (
    <div className="form-rating" id={id}>
      {[1, 2, 3, 4, 5].map((star) => (
        <button
          key={star}
          type="button"
          className={`form-star ${current >= star ? "active" : ""}`}
          onClick={() => !disabled && onChange(star)}
          disabled={disabled}
          aria-label={`Rate ${star}`}
        >
          ★
        </button>
      ))}
    </div>
  );
}
