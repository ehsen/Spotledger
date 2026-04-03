import type { ControlProps } from "./registry";
export function ControlAttach({ id, value, onChange, disabled }: ControlProps) {
  const fileUrl = String(value ?? "");
  return (
    <div className="form-attach">
      {fileUrl && (
        <a
          href={fileUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="form-attach-preview"
        >
          {fileUrl.split("/").pop()}
        </a>
      )}
      {!disabled && (
        <input
          id={id}
          type="file"
          className="form-input form-file"
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) onChange(URL.createObjectURL(file));
          }}
        />
      )}
    </div>
  );
}
