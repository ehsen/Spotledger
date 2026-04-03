import { useState, useEffect, useRef, useCallback, memo } from "react";
import { call } from "../../naja/call";
import type { ControlProps } from "../registry";

interface SuggestResult { name: string; title?: string }

export const ControlLink = memo(function ControlLink({
  id, field, value, onChange, disabled, required,
}: ControlProps) {
  const [inputVal, setInputVal] = useState(String(value ?? ""));
  const [suggestions, setSuggestions] = useState<SuggestResult[]>([]);
  const [open, setOpen] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();
  const containerRef = useRef<HTMLDivElement>(null);
  const linked_doctype = field.options ?? "";

  // Sync prop value → input when doc reloads
  useEffect(() => { setInputVal(String(value ?? "")); }, [value]);

  const search = useCallback((query: string) => {
    if (!linked_doctype || query.length < 1) { setSuggestions([]); return; }
    clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      try {
        const res = await call<{ results: SuggestResult[] }>(
          "frappe.desk.search.search_link",
          {
            txt: query,
            doctype: linked_doctype,
            ignore_user_permissions: 0,
            reference_doctype: field.options ?? "",
          },
          { background: true },
        );
        setSuggestions((res as { results: SuggestResult[] }).results ?? []);
        setOpen(true);
      } catch { setSuggestions([]); }
    }, 200);
  }, [linked_doctype, field.options]);

  function handleInput(e: React.ChangeEvent<HTMLInputElement>) {
    const v = e.target.value;
    setInputVal(v);
    if (v === "") { onChange(null); setSuggestions([]); setOpen(false); return; }
    search(v);
  }

  function handleSelect(result: SuggestResult) {
    setInputVal(result.name);
    onChange(result.name);
    setSuggestions([]);
    setOpen(false);
  }

  function handleBlur() {
    setTimeout(() => setOpen(false), 150);
  }

  return (
    <div ref={containerRef} className="form-link-wrap">
      <input
        id={id}
        type="text"
        className="form-input form-link"
        value={inputVal}
        onChange={handleInput}
        onBlur={handleBlur}
        onFocus={() => { if (inputVal) search(inputVal); }}
        disabled={disabled}
        required={required}
        autoComplete="off"
        placeholder={linked_doctype}
      />
      {open && suggestions.length > 0 && (
        <ul className="form-link-dropdown">
          {suggestions.slice(0, 15).map((s) => (
            <li
              key={s.name}
              className="form-link-option"
              onMouseDown={(e) => { e.preventDefault(); handleSelect(s); }}
            >
              <span className="link-name">{s.name}</span>
              {s.title && s.title !== s.name && (
                <span className="link-title">{s.title}</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
});
