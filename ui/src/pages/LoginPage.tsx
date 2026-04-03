import { useState, type FormEvent } from "react";
import { call } from "../naja/call";
import { useShell } from "../store/shell-store";

export function LoginPage() {
  const setUser = useShell((s) => s.setUser);
  const [usr, setUsr] = useState("");
  const [pwd, setPwd] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const res = await call<{ full_name: string }>("login", { usr, pwd });
      setUser(usr, (res as { full_name: string }).full_name ?? usr);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="login-root">
      <div className="login-card">
        <div className="login-logo">⚡ Spotledger</div>
        <form className="login-form" onSubmit={submit}>
          <label className="login-label">
            Email / Username
            <input
              className="login-input"
              type="text"
              autoFocus
              autoComplete="username"
              value={usr}
              onChange={(e) => setUsr(e.target.value)}
              required
            />
          </label>
          <label className="login-label">
            Password
            <input
              className="login-input"
              type="password"
              autoComplete="current-password"
              value={pwd}
              onChange={(e) => setPwd(e.target.value)}
              required
            />
          </label>
          {error && <div className="login-error">{error}</div>}
          <button className="login-btn" type="submit" disabled={loading}>
            {loading ? "Signing in…" : "Sign in"}
          </button>
        </form>
      </div>
    </div>
  );
}
