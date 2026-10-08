import { ReactNode } from "react";
import Header from "@/components/Header";

export default function App({ children }: { children: ReactNode }) {
  return (
    <div id="app-root">
      <Header />
      {children}
    </div>
  );
}
