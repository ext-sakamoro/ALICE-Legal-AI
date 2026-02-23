import { create } from "zustand";

interface LegalResult {
  type: "analyze" | "compile" | "risk";
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  data: any;
}

interface LegalState {
  // Inputs
  document: string;
  language: string;
  templateId: string;

  // Output
  result: LegalResult | null;
  loading: boolean;

  // Setters
  setDocument: (document: string) => void;
  setLanguage: (language: string) => void;
  setTemplateId: (templateId: string) => void;
  setResult: (result: LegalResult | null) => void;
  setLoading: (loading: boolean) => void;

  // Actions
  reset: () => void;
}

const initialState = {
  document: "",
  language: "en",
  templateId: "",
  result: null,
  loading: false,
};

export const useLegalStore = create<LegalState>((set) => ({
  ...initialState,

  setDocument: (document) => set({ document }),
  setLanguage: (language) => set({ language }),
  setTemplateId: (templateId) => set({ templateId }),
  setResult: (result) => set({ result }),
  setLoading: (loading) => set({ loading }),

  reset: () => set(initialState),
}));
