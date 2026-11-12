import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../index.css";
import { Bench } from "./Bench";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Bench />
  </StrictMode>
);
