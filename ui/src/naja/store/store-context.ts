// React context for per-tab form store access
// Avoids prop drilling — any form sub-component reads the store via useFormStore().

import { createContext, useContext } from "react";
import type { UseBoundStore, StoreApi } from "zustand";
import type { FormStore } from "./form-store";

export type FormStoreInstance = UseBoundStore<StoreApi<FormStore>>;

export const FormStoreContext = createContext<FormStoreInstance | null>(null);

/** Use the form store from any component inside a FormEngine tree. */
export function useFormStore(): FormStoreInstance {
  const store = useContext(FormStoreContext);
  if (!store) throw new Error("useFormStore must be used inside a FormEngine");
  return store;
}
